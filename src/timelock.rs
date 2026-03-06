//! TimelockController: queue -> delay -> execute for critical parameter changes.
//!
//! Operation id = keccak256(abi.encode(target, value, data, predecessor, salt)).
//! State in sol_storage!: `timelock_ready_block: StorageMap<FixedBytes<32>, StorageU256>`.
//! CEI: state updates (clear ready_block) before external execute call.

use alloc::vec::Vec;

use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    block,
    call::{self, Call},
    crypto,
    msg,
};

use crate::{
    access::{default_admin_role, require_role},
    constants::TIMELOCK_MIN_DELAY_BLOCKS,
    errors::*,
    state::OakDEX,
};

/// Role that can queue timelock operations (e.g. multisig).
pub fn timelock_admin_role() -> FixedBytes<32> {
    crypto::keccak(b"TIMELOCK_ADMIN_ROLE")
}

/// Compute operation id = keccak256(abi.encode(target, value, data, predecessor, salt)).
fn operation_id(
    target: Address,
    value: U256,
    data: &[u8],
    predecessor: Address,
    salt: FixedBytes<32>,
) -> FixedBytes<32> {
    let mut enc = Vec::new();
    enc.extend_from_slice(&encode_addr(target));
    enc.extend_from_slice(&encode_u256(value));
    enc.extend_from_slice(&encode_bytes(data));
    enc.extend_from_slice(&encode_addr(predecessor));
    enc.extend_from_slice(salt.as_slice());
    crypto::keccak(&enc)
}

fn encode_addr(a: Address) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..32].copy_from_slice(a.as_slice());
    out
}
fn encode_u256(x: U256) -> [u8; 32] {
    x.to_be_bytes::<32>()
}
fn encode_bytes(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + b.len());
    let len_u32 = (b.len() as u32).to_be_bytes();
    let mut len_padded = [0u8; 32];
    len_padded[28..32].copy_from_slice(&len_u32);
    out.extend_from_slice(&len_padded);
    out.extend_from_slice(b);
    out
}

/// Queue a timelock operation. Caller must have TIMELOCK_ADMIN_ROLE or DEFAULT_ADMIN_ROLE.
/// Sets `timelock_ready_block[id] = block.number + delay_blocks` (delay_blocks >= TIMELOCK_MIN_DELAY_BLOCKS).
pub fn queue_operation(
    dex: &mut OakDEX,
    target: Address,
    value: U256,
    data: &[u8],
    predecessor: Address,
    salt: FixedBytes<32>,
    delay_blocks: u64,
) -> Result<FixedBytes<32>, Vec<u8>> {
    if delay_blocks < TIMELOCK_MIN_DELAY_BLOCKS {
        return Err(err(ERR_TIMELOCK_NOT_READY));
    }
    if require_role(dex, timelock_admin_role()).is_err() && require_role(dex, default_admin_role()).is_err() {
        return Err(err(ERR_MISSING_ROLE));
    }
    let id = operation_id(target, value, data, predecessor, salt);
    let ready_at = U256::from(block::number())
        .checked_add(U256::from(delay_blocks))
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    dex.timelock_ready_block.setter(id).set(ready_at);
    Ok(id)
}

/// Returns the block number after which the operation can be executed (0 if not queued).
pub fn get_operation_ready_block(dex: &OakDEX, operation_id: FixedBytes<32>) -> U256 {
    dex.timelock_ready_block.getter(operation_id).get()
}

/// Execute a queued operation: checks delay elapsed, clears state, then calls target with value and data.
/// Caller can be anyone; predecessor is checked if non-zero (optional guard).
pub fn execute_operation(
    dex: &mut OakDEX,
    target: Address,
    value: U256,
    data: &[u8],
    predecessor: Address,
    salt: FixedBytes<32>,
) -> Result<(), Vec<u8>> {
    let id = operation_id(target, value, data, predecessor, salt);
    let ready_at = dex.timelock_ready_block.setter(id).get();
    if ready_at.is_zero() {
        return Err(err(ERR_TIMELOCK_UNKNOWN_OPERATION));
    }
    let block_num = U256::from(block::number());
    if block_num < ready_at {
        return Err(err(ERR_TIMELOCK_NOT_READY));
    }
    if predecessor != Address::ZERO && msg::sender() != predecessor {
        return Err(err(ERR_MISSING_ROLE));
    }
    // CEI: clear state before external call
    dex.timelock_ready_block.setter(id).set(U256::ZERO);
    // Stylus call::call(context, to, data) — value not forwarded; use target contract's payable if needed.
    let calldata = Vec::from(data);
    if call::call(Call::new(), target, &calldata).is_err() {
        return Err(err(ERR_TIMELOCK_NOT_READY));
    }
    Ok(())
}
