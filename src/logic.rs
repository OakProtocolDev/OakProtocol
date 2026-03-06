//! Core protocol logic: CPMM math, commit‑reveal, fee accounting.

use alloc::vec::Vec;

use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    block,
    call::{self, Call},
    contract,
    crypto,
    msg,
    prelude::public,
};

use crate::{
    access::{default_admin_role, pauser_role},
    constants::{
        as_u256, q112_u256, BPS, CIRCUIT_BREAKER_IMPACT_BPS, COMMIT_REVEAL_DELAY, DEFAULT_FEE_BPS,
        FEE_DENOMINATOR, GAS_REBATE_BPS, INITIAL_FEE, LP_FEE_PCT, MAX_COMMITMENT_AGE, MAX_FEE_BPS,
        MAX_PATH_LENGTH, MAX_TRADE_RESERVE_BPS, MINIMUM_LIQUIDITY, OWNER_TRANSFER_DELAY_BLOCKS,
        TREASURY_FEE_BPS, BUYBACK_FEE_PCT, TREASURY_FEE_PCT,
    },
    errors::*,
    events::{
        emit_add_liquidity, emit_buyback_wallet_set, emit_cancel_commitment, emit_circuit_breaker_cleared,
        emit_circuit_breaker_triggered, emit_close_position, emit_commit_swap, emit_flash_swap,
        emit_lp_transfer,         emit_open_position, emit_order_cancelled, emit_order_executed,
        emit_order_placed, emit_owner_changed, emit_pause_changed, emit_pending_owner_set,
        emit_pool_created, emit_reveal_swap, emit_set_fee, emit_set_position_tp_sl,
        emit_set_position_trailing, emit_trailing_stop_triggered, emit_withdraw_treasury_fees,
    },
    pausable::Pausable,
    state::OakDEX,
    token::{balance_of, safe_transfer, safe_transfer_from},
};

/// Encode `(amount_in, salt)` similarly to `abi.encode`.
fn encode_commit_data(amount_in: U256, salt: U256) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(64);
    encoded.extend_from_slice(&amount_in.to_be_bytes::<32>());
    encoded.extend_from_slice(&salt.to_be_bytes::<32>());
    encoded
}

/// Compute commitment hash as `keccak256(abi.encode(amount_in, salt))`.
/// Public for test and SDK use.
pub fn compute_commit_hash(amount_in: U256, salt: U256) -> FixedBytes<32> {
    let encoded = encode_commit_data(amount_in, salt);
    crypto::keccak(&encoded)
}

/// Verify that `sender` is the contract owner.
fn only_owner(owner: Address) -> OakResult<()> {
    let sender = msg::sender();
    if sender != owner {
        return Err(err(ERR_ONLY_OWNER));
    }
    Ok(())
}

/// Validate that an address is not the zero address.
///
/// @notice Prevents invalid address inputs that could lead to fund loss.
/// @dev Zero address checks are critical for token transfers and access control.
fn require_non_zero_address(addr: Address) -> OakResult<()> {
    if addr == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    Ok(())
}

/// Re-entrancy guard: ensure function is not called recursively.
///
/// @notice Checks and sets the global `locked` flag.
/// @dev Must be paired with `unlock_reentrancy_guard` in a finally-like pattern.
fn lock_reentrancy_guard(dex: &mut OakDEX) -> OakResult<()> {
    if dex.locked.get() {
        return Err(err(ERR_REENTRANT_CALL));
    }
    dex.locked.set(true);
    Ok(())
}

/// Re-entrancy guard: release the lock.
///
/// @notice Clears the global `locked` flag.
/// @dev Must be called after `lock_reentrancy_guard` to prevent deadlock.
fn unlock_reentrancy_guard(dex: &mut OakDEX) {
    dex.locked.set(false);
}

/// Emergency circuit breaker: revert if protocol is paused.
///
/// @notice Applied to commit_swap, reveal_swap, and flash_swap.
/// @dev Only owner can pause/unpause via pause() and unpause().
fn require_not_paused(dex: &OakDEX) -> OakResult<()> {
    if dex.paused.get() {
        return Err(err(ERR_PAUSED));
    }
    Ok(())
}

/// Map order ID (U256) to storage key (Address = last 20 bytes of BE encoding).
fn order_id_to_address(order_id: U256) -> Address {
    let b = order_id.to_be_bytes::<32>();
    Address::from_slice(&b[12..32])
}

/// Map position ID (U256) to storage key (same as order_id for consistency).
fn position_id_to_address(position_id: U256) -> Address {
    let b = position_id.to_be_bytes::<32>();
    Address::from_slice(&b[12..32])
}

/// Safety circuit breaker: when triggered, swaps and add_liquidity are disabled.
/// Only remove_liquidity and claim_fees allowed. Owner can clear.
fn require_not_circuit_breaker(dex: &OakDEX) -> OakResult<()> {
    if dex.circuit_breaker_triggered.get() {
        return Err(err(ERR_CIRCUIT_BREAKER));
    }
    Ok(())
}

/// Update TWAP oracle cumulative prices and last block.
///
/// @notice Must be called at the beginning of every swap (reveal_swap) and add_liquidity.
/// @dev Uses Q112.64 fixed-point: price0 = reserve1/reserve0, price1 = reserve0/reserve1.
///      On L2 we use block number as time index for gas efficiency.
///      cumulative += price * (current_block - block_last); all math checked.
fn update_oracle(dex: &mut OakDEX, reserve0: U256, reserve1: U256) -> OakResult<()> {
    let block_last = dex.block_timestamp_last.get();
    let current_block = U256::from(block::number());

    if reserve0.is_zero() || reserve1.is_zero() {
        dex.block_timestamp_last.set(current_block);
        return Ok(());
    }

    let time_elapsed = current_block.checked_sub(block_last).unwrap_or(U256::ZERO);
    if time_elapsed.is_zero() {
        return Ok(());
    }

    let q112 = q112_u256();
    // price0 = reserve1 / reserve0 in Q112.64
    let price0 = reserve1
        .checked_mul(q112)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(reserve0)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    // price1 = reserve0 / reserve1 in Q112.64
    let price1 = reserve0
        .checked_mul(q112)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(reserve1)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    let cum0_delta = price0
        .checked_mul(time_elapsed)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let cum1_delta = price1
        .checked_mul(time_elapsed)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let cum0 = dex.price0_cumulative_last.get();
    let cum1 = dex.price1_cumulative_last.get();

    let new_cum0 = cum0.checked_add(cum0_delta).ok_or_else(|| err(ERR_OVERFLOW))?;
    let new_cum1 = cum1.checked_add(cum1_delta).ok_or_else(|| err(ERR_OVERFLOW))?;

    dex.price0_cumulative_last.set(new_cum0);
    dex.price1_cumulative_last.set(new_cum1);
    dex.block_timestamp_last.set(current_block);

    Ok(())
}

/// Core swap processing with configurable from/to (for direct swaps and order execution).
///
/// @notice When `from` == contract, no transfer_in is performed (tokens already in contract).
/// @dev Used by process_swap (from=to=msg::sender) and execute_order (from=contract, to=order_owner).
fn process_swap_from_to(
    dex: &mut OakDEX,
    from: Address,
    to: Address,
    token0: Address,
    token1: Address,
    amount_in: U256,
    min_amount_out: U256,
) -> OakResult<U256> {
    require_non_zero_address(token0)?;
    require_non_zero_address(token1)?;
    if amount_in.is_zero() {
        return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
    }
    if min_amount_out.is_zero() {
        return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
    }
    require_not_paused(dex)?;
    require_not_circuit_breaker(dex)?;

    let contract_addr = contract::address();
    if from != contract_addr {
        let user_balance = balance_of(token0, from);
        if user_balance < amount_in {
            return Err(err(ERR_INSUFFICIENT_BALANCE));
        }
    }

    // Snapshot pool reserves and fee configuration.
    let (pool_token0, pool_token1) = if token0 < token1 {
        (token0, token1)
    } else {
        (token1, token0)
    };
    // First, read reserves via a short-lived mutable borrow.
    let (reserve0, reserve1) = {
        let mut outer = dex.pools.setter(pool_token0);
        let pool = outer.setter(pool_token1);
        if !pool.initialized.get() {
            return Err(err(ERR_INVALID_TOKEN));
        }
        (pool.reserve0.get(), pool.reserve1.get())
    };
    let fee_bps = dex.protocol_fee_bps.get();

    // TWAP oracle: update cumulative prices at the beginning of every swap.
    update_oracle(dex, reserve0, reserve1)?;

    // Determine direction within the pool and compute amount_out.
    let (reserve_in, reserve_out) = if token0 == pool_token0 {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    // Bank-style cap: single trade cannot exceed MAX_TRADE_RESERVE_BPS of reserve (e.g. 10%).
    let max_trade = reserve_in
        .checked_mul(as_u256(MAX_TRADE_RESERVE_BPS))
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(BPS))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    if amount_in > max_trade {
        return Err(err(ERR_TRADE_TOO_LARGE));
    }

    let amount_out = get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps)?;

    // Circuit breaker: auto-trigger on extreme price impact (e.g. 20%+). Audit trail event.
    let impact_num = amount_out
        .checked_mul(reserve_in)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_mul(as_u256(BPS))
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let impact_den = amount_in
        .checked_mul(reserve_out)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let impact_bps = if impact_den.is_zero() {
        U256::ZERO
    } else {
        impact_num.checked_div(impact_den).unwrap_or(U256::ZERO)
    };
    let price_impact_bps = as_u256(BPS).saturating_sub(impact_bps).min(U256::from(10000u64));
    if price_impact_bps >= as_u256(CIRCUIT_BREAKER_IMPACT_BPS) {
        dex.circuit_breaker_triggered.set(true);
        emit_circuit_breaker_triggered(price_impact_bps);
        return Err(err(ERR_CIRCUIT_BREAKER));
    }

    // Strict slippage protection: revert if actual output below minimum.
    if amount_out < min_amount_out {
        return Err(err(ERR_SLIPPAGE_EXCEEDED));
    }

    // Compute fee split: 60% LP, 20% Treasury, 20% Buyback.
    let (_effective_in, treasury_fee, lp_fee, buyback_fee) =
        compute_fee_split(amount_in, fee_bps)?;

    // Reserve invariant: only (amount_in - treasury - buyback) goes to pool; rest is claimable by owner.
    // This ensures withdraw_treasury_fees does not drain pool reserves (balance = pool_reserves + treasury + buyback).
    let to_pool_in = amount_in
        .checked_sub(treasury_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_sub(buyback_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let new_reserve_in = reserve_in
        .checked_add(to_pool_in)
        .ok_or_else(|| err(ERR_RESERVE0_OVERFLOW))?;

    let new_reserve_out = reserve_out
        .checked_sub(amount_out)
        .ok_or_else(|| err(ERR_INSUFFICIENT_LIQUIDITY))?;

    let min_liquidity = dex.min_liquidity.get();

    let (new_reserve0, new_reserve1) = if token0 == pool_token0 {
        (new_reserve_in, new_reserve_out)
    } else {
        (new_reserve_out, new_reserve_in)
    };

    if new_reserve0 < min_liquidity || new_reserve1 < min_liquidity {
        return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
    }

    {
        let mut outer = dex.pools.setter(pool_token0);
        let mut pool = outer.setter(pool_token1);
        pool.reserve0.set(new_reserve0);
        pool.reserve1.set(new_reserve1);
    }

    // Update analytics and accounting.
    let current_volume0 = dex.total_volume_token0.get();
    let current_volume1 = dex.total_volume_token1.get();

    let new_volume0 = current_volume0
        .checked_add(amount_in)
        .ok_or_else(|| err(ERR_VOLUME_OVERFLOW))?;

    let new_volume1 = current_volume1
        .checked_add(amount_out)
        .ok_or_else(|| err(ERR_VOLUME_OVERFLOW))?;

    dex.total_volume_token0.set(new_volume0);
    dex.total_volume_token1.set(new_volume1);

    // Per-token treasury and buyback (60/20/20 model).
    let token_in = token0;
    let prev_treasury = dex.treasury_balance.setter(token_in).get();
    let prev_buyback = dex.buyback_balance.setter(token_in).get();
    dex.treasury_balance.setter(token_in).set(
        prev_treasury
            .checked_add(treasury_fee)
            .ok_or_else(|| err(ERR_OVERFLOW))?,
    );
    dex.buyback_balance.setter(token_in).set(
        prev_buyback
            .checked_add(buyback_fee)
            .ok_or_else(|| err(ERR_OVERFLOW))?,
    );

    // Gas-rebate placeholder: track a small portion of protocol fee for future gas rebates.
    let total_fee = treasury_fee
        .checked_add(lp_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let gas_rebate = total_fee
        .checked_mul(as_u256(GAS_REBATE_BPS))
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    if !gas_rebate.is_zero() {
        let acc = dex.accrued_gas_rebate_token0.get();
        let new_acc = acc
            .checked_add(gas_rebate)
            .ok_or_else(|| err(ERR_OVERFLOW))?;
        dex.accrued_gas_rebate_token0.set(new_acc);
    }

    // Transfer in: from -> contract (skip if from == contract, tokens already there)
    if from != contract_addr {
        safe_transfer_from(token0, from, contract_addr, amount_in)?;
    }
    // Transfer out: contract -> to
    safe_transfer(token1, to, amount_out)?;

    Ok(amount_out)
}

/// Core swap processing: invariant math, slippage protection, fee accounting and transfers.
///
/// @notice Entrypoint path: from = to = msg::sender. Emits RevealSwap.
fn process_swap(
    dex: &mut OakDEX,
    token0: Address,
    token1: Address,
    amount_in: U256,
    min_amount_out: U256,
) -> OakResult<U256> {
    let sender = msg::sender();
    let amount_out = process_swap_from_to(dex, sender, sender, token0, token1, amount_in, min_amount_out)?;
    let (_effective_in, treasury_fee, lp_fee, _buyback_fee) =
        compute_fee_split(amount_in, dex.protocol_fee_bps.get())?;
    emit_reveal_swap(sender, amount_in, amount_out, treasury_fee, lp_fee);
    Ok(amount_out)
}

// ---------- EIP-712 Gasless Permit Swap ----------

/// EIP-712 domain name and version for PermitSwap.
const EIP712_NAME: &[u8] = b"Oak Protocol";
const EIP712_VERSION: &[u8] = b"1";
/// Chain ID for EIP-712 domain (Arbitrum One). Use same chain as deployment.
const CHAIN_ID_ARBITRUM_ONE: u64 = 42161;

fn ecrecover_precompile() -> Address {
    Address::from_word(U256::from(1u64).to_be_bytes::<32>().into())
}

/// keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
fn eip712_domain_type_hash() -> FixedBytes<32> {
    crypto::keccak(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
}

/// keccak256("PermitSwap(address owner,address tokenIn,address tokenOut,uint256 amountIn,uint256 minAmountOut,uint256 deadline,uint256 nonce)")
fn permit_swap_type_hash() -> FixedBytes<32> {
    crypto::keccak(b"PermitSwap(address owner,address tokenIn,address tokenOut,uint256 amountIn,uint256 minAmountOut,uint256 deadline,uint256 nonce)")
}

/// Encode 32-byte value for ABI (left-pad to 32 bytes).
fn enc_u256(x: U256) -> [u8; 32] {
    x.to_be_bytes::<32>()
}
fn enc_addr(a: Address) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..32].copy_from_slice(a.as_slice());
    out
}

/// Compute EIP-712 domain separator: hash of encoded domain.
fn compute_domain_separator(verifying_contract: Address, chain_id: u64) -> FixedBytes<32> {
    let name_hash = crypto::keccak(EIP712_NAME);
    let version_hash = crypto::keccak(EIP712_VERSION);
    let mut enc = Vec::with_capacity(128);
    enc.extend_from_slice(eip712_domain_type_hash().as_slice());
    enc.extend_from_slice(name_hash.as_slice());
    enc.extend_from_slice(version_hash.as_slice());
    enc.extend_from_slice(&enc_u256(U256::from(chain_id)));
    enc.extend_from_slice(&enc_addr(verifying_contract));
    crypto::keccak(&enc)
}

/// Compute EIP-712 digest for PermitSwap: "\x19\x01" || domainSeparator || structHash.
fn compute_permit_swap_digest(
    owner: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    min_amount_out: U256,
    deadline: U256,
    nonce: U256,
    domain_separator: &FixedBytes<32>,
) -> FixedBytes<32> {
    let type_hash = permit_swap_type_hash();
    let mut enc = Vec::with_capacity(256);
    enc.extend_from_slice(type_hash.as_slice());
    enc.extend_from_slice(&enc_addr(owner));
    enc.extend_from_slice(&enc_addr(token_in));
    enc.extend_from_slice(&enc_addr(token_out));
    enc.extend_from_slice(&enc_u256(amount_in));
    enc.extend_from_slice(&enc_u256(min_amount_out));
    enc.extend_from_slice(&enc_u256(deadline));
    enc.extend_from_slice(&enc_u256(nonce));
    let struct_hash = crypto::keccak(&enc);
    let mut prefix = Vec::with_capacity(66);
    prefix.extend_from_slice(b"\x19\x01");
    prefix.extend_from_slice(domain_separator.as_slice());
    prefix.extend_from_slice(struct_hash.as_slice());
    crypto::keccak(&prefix)
}

/// Recover signer from EIP-712 digest and (v, r, s). Returns zero address on failure.
fn ecrecover_recover(digest: FixedBytes<32>, v: u8, r: [u8; 32], s: [u8; 32]) -> Address {
    let v_normalized = if v <= 1 { v + 27 } else { v };
    let mut calldata = Vec::with_capacity(128);
    calldata.extend_from_slice(digest.as_slice());
    calldata.extend_from_slice(&enc_u256(U256::from(v_normalized)));
    calldata.extend_from_slice(&r);
    calldata.extend_from_slice(&s);
    let precompile = ecrecover_precompile();
    match call::static_call(Call::new(), precompile, &calldata) {
        Ok(ret) if ret.len() >= 32 => {
            let out: [u8; 32] = ret[0..32].try_into().unwrap_or([0; 32]);
            Address::from_slice(&out[12..32])
        }
        _ => Address::ZERO,
    }
}

/// Pure CPMM math with a configurable total fee.
///
/// @notice Computes constant‑product output amount for a given input.
/// @dev Uses Uniswap‑style formula:
///      amount_out = (amount_in_with_fee * reserve_out)
///                   / (reserve_in * FEE_DENOMINATOR + amount_in_with_fee)
///      where amount_in_with_fee = amount_in * (FEE_DENOMINATOR - fee_bps).
pub fn get_amount_out_with_fee(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
    fee_bps: U256,
) -> OakResult<U256> {
    if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
        return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
    }

    // If the effective fee rounds down to zero for this trade size,
    // treat it as "dust": the input is too small to produce a meaningful
    // output under the configured fee. In this case we return 0 instead
    // of reverting, so callers can decide whether to proceed.
    let total_fee = amount_in
        .checked_mul(fee_bps)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    if !fee_bps.is_zero() && total_fee.is_zero() {
        return Ok(U256::ZERO);
    }

    let fee_multiplier = as_u256(FEE_DENOMINATOR)
        .checked_sub(fee_bps)
        .ok_or_else(|| err(ERR_FEE_OVERFLOW))?;

    let amount_in_with_fee = amount_in
        .checked_mul(fee_multiplier)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let numerator = amount_in_with_fee
        .checked_mul(reserve_out)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let denominator_part1 = reserve_in
        .checked_mul(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let denominator = denominator_part1
        .checked_add(amount_in_with_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    // Integer division in Rust performs floor rounding (rounds down).
    // This is protocol-favorable: users receive slightly less, protocol retains value.
    // Formula: amount_out = floor((amount_in_with_fee * reserve_out) / denominator)
    let amount_out = numerator
        .checked_div(denominator)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    Ok(amount_out)
}

/// Inverse of get_amount_out: amount_in needed to receive at least amount_out (single hop). Rounds up (protocol-safe).
pub fn get_amount_in_with_fee(
    amount_out: U256,
    reserve_in: U256,
    reserve_out: U256,
    fee_bps: U256,
) -> OakResult<U256> {
    if amount_out.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
        return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
    }
    let reserve_out_sub = reserve_out.checked_sub(amount_out).ok_or_else(|| err(ERR_INSUFFICIENT_LIQUIDITY))?;
    let fee_mult = as_u256(FEE_DENOMINATOR).checked_sub(fee_bps).ok_or_else(|| err(ERR_FEE_OVERFLOW))?;
    let numerator = amount_out
        .checked_mul(reserve_in)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_mul(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let denominator = reserve_out_sub
        .checked_mul(fee_mult)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let amount_in = numerator
        .checked_div(denominator)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    let remainder = numerator % denominator;
    let amount_in_ceil = if remainder.is_zero() {
        amount_in
    } else {
        amount_in.checked_add(U256::from(1u64)).ok_or_else(|| err(ERR_OVERFLOW))?
    };
    Ok(amount_in_ceil)
}

/// Compute the total fee and its split: 60% LP, 20% Treasury, 20% Buyback.
///
/// @notice World-class fee model: LPs get majority, treasury and buyback fund get equal shares.
/// @dev All math checked; remainder goes to LP to avoid dust.
pub fn compute_fee_split(
    amount_in: U256,
    fee_bps: U256,
) -> OakResult<(U256, U256, U256, U256)> {
    if amount_in.is_zero() {
        return Ok((U256::ZERO, U256::ZERO, U256::ZERO, U256::ZERO));
    }

    let total_fee = amount_in
        .checked_mul(fee_bps)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    if total_fee.is_zero() {
        return Ok((amount_in, U256::ZERO, U256::ZERO, U256::ZERO));
    }

    // 20% Treasury
    let treasury_fee = total_fee
        .checked_mul(as_u256(TREASURY_FEE_PCT))
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(U256::from(100u64))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    // 20% Buyback
    let buyback_fee = total_fee
        .checked_mul(as_u256(BUYBACK_FEE_PCT))
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(U256::from(100u64))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    // 60% LP (remainder to avoid rounding dust)
    let lp_fee = total_fee
        .checked_sub(treasury_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_sub(buyback_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let effective_in = amount_in
        .checked_sub(total_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    Ok((effective_in, treasury_fee, lp_fee, buyback_fee))
}

/// Integer square root for `U256` (floor).
///
/// @notice Returns `floor(sqrt(x))` using a Babylonian-style iteration.
/// @dev This is used for initial LP token minting: sqrt(amount0 * amount1).
fn u256_sqrt(x: U256) -> U256 {
    if x.is_zero() {
        return U256::ZERO;
    }

    // Initial approximation: x/2 + 1
    let mut z = x;
    let mut y = (x >> 1) + U256::from(1u64);

    while y < z {
        z = y;
        y = (x.checked_div(y).unwrap_or(U256::ZERO) + y) >> 1;
    }

    z
}

/// Public contract functions implementation.
///
/// @notice Core entrypoints exposed to external callers.
/// @dev These methods operate on Stylus storage types defined in `state`.
///      This block is only compiled for on-chain (wasm32) builds; host
///      tests use the pure helper functions above instead.
#[cfg(all(not(test), target_arch = "wasm32"))]
#[public]
impl OakDEX {
    /// Create a new pool for a token pair.
    ///
    /// @notice Anyone can create a pool, but each canonical pair (token0, token1)
    ///         can only be initialized once.
    pub fn create_pool(&mut self, token_a: Address, token_b: Address) -> OakResult<()> {
        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        require_non_zero_address(token_a)?;
        require_non_zero_address(token_b)?;
        if token_a == token_b {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }

        // Canonical ordering
        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        // Access pool storage
        let mut outer = self.pools.setter(token0);
        let mut pool = outer.setter(token1);

        if pool.initialized.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POOL_EXISTS));
        }

        // Initialize empty pool
        pool.reserve0.set(U256::ZERO);
        pool.reserve1.set(U256::ZERO);
        pool.lp_total_supply.set(U256::ZERO);
        pool.initialized.set(true);

        emit_pool_created(token0, token1);

        unlock_reentrancy_guard(self);

        Ok(())
    }
    /// Initialize the contract.
    ///
    /// @notice One‑time initializer setting owner, treasury, and default fee.
    /// @dev Reverts if called more than once or if owner/treasury are zero.
    pub fn init(&mut self, initial_owner: Address, treasury: Address) -> OakResult<()> {
        let current_owner = self.owner.get();
        if current_owner != Address::ZERO {
            return Err(err(ERR_ALREADY_INITIALIZED));
        }

        if initial_owner == Address::ZERO {
            return Err(err(ERR_INVALID_OWNER));
        }
        if treasury == Address::ZERO {
            return Err(err(ERR_INVALID_OWNER));
        }
        let contract_addr = contract::address();
        if treasury == contract_addr {
            return Err(err(ERR_TREASURY_IS_CONTRACT));
        }

        self.owner.set(initial_owner);
        self.treasury.set(treasury);

        // Set initial total fee (0.5%) for the first month after launch.
        // Governance can later reduce this to `DEFAULT_FEE_BPS` via `set_fee`.
        self.protocol_fee_bps.set(as_u256(INITIAL_FEE));

        // Initialize analytics and fee accounting.
        self.total_volume_token0.set(U256::ZERO);
        self.total_volume_token1.set(U256::ZERO);
        self.accrued_treasury_fees_token0.set(U256::ZERO);
        self.accrued_lp_fees_token0.set(U256::ZERO);

        // TWAP oracle and gas-rebate placeholder.
        self.price0_cumulative_last.set(U256::ZERO);
        self.price1_cumulative_last.set(U256::ZERO);
        self.block_timestamp_last.set(U256::ZERO);
        self.accrued_gas_rebate_token0.set(U256::ZERO);

        // Contract starts active, unlocked, circuit breaker off.
        self.paused.set(false);
        self.locked.set(false);
        self.circuit_breaker_triggered.set(false);
        self.buyback_wallet.set(Address::ZERO);
        self.pending_owner.set(Address::ZERO);
        self.owner_transfer_after_block.set(U256::ZERO);
        self.next_position_id.set(U256::ZERO);

        // Access Control: grant DEFAULT_ADMIN_ROLE and PAUSER_ROLE to initial_owner (multisig).
        self.roles.setter(default_admin_role()).setter(initial_owner).set(true);
        self.roles.setter(pauser_role()).setter(initial_owner).set(true);

        Ok(())
    }

    /// Update the total protocol fee.
    ///
    /// @notice Owner‑only function to adjust the global fee (in basis points).
    /// @dev Upper bound protects users from excessive fees.
    pub fn set_fee(&mut self, new_fee_bps: u16) -> OakResult<()> {
        let owner = self.owner.get();
        only_owner(owner)?;

        if new_fee_bps as u64 > MAX_FEE_BPS {
            return Err(err(ERR_FEE_TOO_HIGH));
        }

        self.protocol_fee_bps.set(U256::from(new_fee_bps));

        emit_set_fee(new_fee_bps);

        Ok(())
    }

    /// Pause trading in case of emergency.
    ///
    /// @notice Caller must have PAUSER_ROLE (e.g. multisig). Disables swaps and commits.
    /// @dev Uses Pausable trait; CEI: state update before any external.
    pub fn pause(&mut self) -> OakResult<()> {
        Pausable::pause(self).map_err(|e| e)
    }

    /// Resume trading after an incident is resolved.
    ///
    /// @notice Caller must have PAUSER_ROLE.
    pub fn unpause(&mut self) -> OakResult<()> {
        Pausable::unpause(self).map_err(|e| e)
    }

    /// Create a swap commitment.
    ///
    /// @notice Stores a commitment hash and the current block number.
    /// @dev Part 1 of the commit‑reveal flow used for MEV resistance.
    pub fn commit_swap(&mut self, hash: FixedBytes<32>) -> OakResult<()> {
        require_not_paused(self)?;

        let sender = msg::sender();

        if hash == FixedBytes::ZERO {
            return Err(err(ERR_INVALID_HASH));
        }

        let current_block = U256::from(block::number());

        let hash_u256 = U256::from_be_bytes::<32>(hash.into());
        self.commitment_hashes.setter(sender).set(hash_u256);
        self.commitment_timestamps.setter(sender).set(current_block);
        self.commitment_activated.setter(sender).set(true);

        emit_commit_swap(sender, hash, current_block);

        Ok(())
    }

    /// Reveal a previously committed swap and execute it.
    ///
    /// @notice Performs hash verification, time‑lock enforcement, fee
    ///         accounting, CPMM pricing, strict slippage and deadline checks, and token transfers.
    /// @dev Part 2 of commit‑reveal flow, providing strong MEV protection.
    ///      Reverts with DeadlineExpired if block number > deadline, SlippageExceeded if output < min_amount_out.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0 (input token)
    /// * `token1` - Address of token1 (output token)
    /// * `amount_in` - Input token amount
    /// * `salt` - Random salt used in commitment
    /// * `min_amount_out` - Minimum output tokens (strict slippage protection)
    /// * `deadline` - Block number after which the transaction must revert (deadline protection)
    pub fn reveal_swap(
        &mut self,
        token0: Address,
        token1: Address,
        amount_in: U256,
        salt: U256,
        min_amount_out: U256,
        deadline: U256,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
        lock_reentrancy_guard(self)?;

        // Input sanitization: validate addresses
        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        // Input sanitization: validate amounts
        if amount_in.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        if min_amount_out.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }

        require_not_paused(self)?;
        require_not_circuit_breaker(self)?;

        // Deadline protection: revert if transaction is included after deadline (block number).
        let current_block = U256::from(block::number());
        if current_block > deadline {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_DEADLINE_EXPIRED));
        }

        let sender = msg::sender();

        // Reentrancy protection: check activation, then clear commitment
        // before performing any external‑effectful logic.
        let is_activated = self.commitment_activated.setter(sender).get();
        if !is_activated {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_COMMIT_NOT_FOUND));
        }

        let stored_hash_u256 = self.commitment_hashes.setter(sender).get();
        if stored_hash_u256.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_COMMIT_NOT_FOUND));
        }

        let computed_hash = compute_commit_hash(amount_in, salt);
        let computed_hash_u256 = U256::from_be_bytes::<32>(computed_hash.into());

        if stored_hash_u256 != computed_hash_u256 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_HASH));
        }

        let commit_block = self.commitment_timestamps.setter(sender).get();
        // current_block already set above for deadline check

        // Check commitment expiration (prevent storage bloat)
        let max_block = commit_block
            .checked_add(as_u256(MAX_COMMITMENT_AGE))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        if current_block > max_block {
            // Commitment expired, clear it and return error
            self.commitment_activated.setter(sender).set(false);
            self.commitment_hashes.setter(sender).set(U256::ZERO);
            unlock_reentrancy_guard(self);
            return Err(err(ERR_COMMITMENT_EXPIRED));
        }

        // Check minimum delay (MEV protection)
        let min_block = commit_block
            .checked_add(as_u256(COMMIT_REVEAL_DELAY))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        if current_block < min_block {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_TOO_EARLY));
        }

        // Clear commitment state prior to swap execution.
        self.commitment_activated.setter(sender).set(false);
        self.commitment_hashes.setter(sender).set(U256::ZERO);

        // Execute the actual swap with invariant checks, slippage protection,
        // and fee accounting. All math and external calls are performed inside
        // `process_swap`, which uses fully checked arithmetic and accrues
        // treasury fees for the admin wallet.
        let result = process_swap(self, token0, token1, amount_in, min_amount_out);
        let amount_out = match result {
            Ok(v) => v,
            Err(e) => {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        };

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Execute a swap on behalf of `owner` using EIP-712 permit (gasless flow).
    ///
    /// @notice Relayer calls this paying gas; contract verifies ECDSA signature then runs swap.
    /// @dev Checks deadline (block number), nonce, recovers signer; increments nonce; executes process_swap_from_to(owner, owner, ...).
    pub fn execute_swap_with_permit(
        &mut self,
        owner: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        min_amount_out: U256,
        deadline: U256,
        nonce: U256,
        v: u8,
        r: FixedBytes<32>,
        s: FixedBytes<32>,
    ) -> OakResult<()> {
        lock_reentrancy_guard(self)?;

        require_non_zero_address(owner)?;
        require_non_zero_address(token_in)?;
        require_non_zero_address(token_out)?;
        if amount_in.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        if min_amount_out.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }

        require_not_paused(self)?;
        require_not_circuit_breaker(self)?;

        let current_block = U256::from(block::number());
        if current_block > deadline {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PERMIT_EXPIRED));
        }

        let current_nonce = self.permit_swap_nonce.setter(owner).get();
        if nonce != current_nonce {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PERMIT_NONCE));
        }
        self.permit_swap_nonce.setter(owner).set(
            current_nonce.checked_add(U256::from(1u64)).ok_or_else(|| err(ERR_OVERFLOW))?,
        );

        let contract_addr = contract::address();
        let domain_separator = compute_domain_separator(contract_addr, CHAIN_ID_ARBITRUM_ONE);
        let digest = compute_permit_swap_digest(
            owner,
            token_in,
            token_out,
            amount_in,
            min_amount_out,
            deadline,
            nonce,
            &domain_separator,
        );
        let recovered = ecrecover_recover(digest, v, r.0, s.0);
        if recovered != owner {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PERMIT_INVALID_SIGNATURE));
        }

        let amount_out = process_swap_from_to(
            self,
            owner,
            owner,
            token_in,
            token_out,
            amount_in,
            min_amount_out,
        )?;
        let (_effective_in, treasury_fee, lp_fee, _buyback_fee) =
            compute_fee_split(amount_in, self.protocol_fee_bps.get())?;
        emit_reveal_swap(owner, amount_in, amount_out, treasury_fee, lp_fee);

        unlock_reentrancy_guard(self);
        Ok(())
    }

    /// Returns the current permit-swap nonce for `owner` (for EIP-712 gasless flow).
    pub fn get_permit_swap_nonce(&mut self, owner: Address) -> U256 {
        self.permit_swap_nonce.setter(owner).get()
    }

    /// Add liquidity to the pool.
    ///
    /// @notice Adds token0 and token1 to the reserves, enforcing minimum liquidity.
    /// @dev In a full implementation, this would also mint LP tokens.
    ///      Transfers tokens from caller to contract before updating reserves.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0
    /// * `token1` - Address of token1
    /// * `amount0` - Amount of token0 to add
    /// * `amount1` - Amount of token1 to add
    /// * `amount0_min` - Minimum amount0 to accept (LP slippage protection)
    /// * `amount1_min` - Minimum amount1 to accept (LP slippage protection)
    pub fn add_liquidity(
        &mut self,
        token0: Address,
        token1: Address,
        amount0: U256,
        amount1: U256,
        amount0_min: U256,
        amount1_min: U256,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
        // This must be the first state-modifying operation
        lock_reentrancy_guard(self)?;

        // Input sanitization: validate addresses
        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        // Input sanitization: validate amounts
        if amount0.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_AMOUNT0_ZERO));
        }
        if amount1.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_AMOUNT1_ZERO));
        }

        require_not_paused(self)?;
        require_not_circuit_breaker(self)?;

        // Canonicalize token ordering for pool key.
        let (pool_token0, pool_token1) = if token0 < token1 {
            (token0, token1)
        } else {
            (token1, token0)
        };
        let mut outer = self.pools.setter(pool_token0);
        let mut pool = outer.setter(pool_token1);
        if !pool.initialized.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }

        // Map provided amounts into canonical order.
        let (amount0_c, amount1_c) = if token0 == pool_token0 {
            (amount0, amount1)
        } else {
            (amount1, amount0)
        };

        // LP slippage protection (bank-grade: never accept below user minimum).
        if amount0_c < amount0_min || amount1_c < amount1_min {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_LP_SLIPPAGE));
        }

        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();
        let total_supply = pool.lp_total_supply.get();

        // Compute LP tokens to mint, following Uniswap V2 semantics.
        // First liquidity: liquidity = sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
        // Subsequent: min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1)
        let liquidity = if total_supply.is_zero() {
            let product = amount0_c
                .checked_mul(amount1_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_LIQUIDITY_OVERFLOW)
                })?;
            let sqrt = u256_sqrt(product);
            let min_lp = as_u256(MINIMUM_LIQUIDITY);

            if sqrt <= min_lp {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }

            // Lock MINIMUM_LIQUIDITY LP tokens forever to the zero address.
            pool.lp_total_supply.set(min_lp);
            pool.lp_balances.setter(Address::ZERO).set(min_lp);

            sqrt.checked_sub(min_lp).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?
        } else {
            // amount0 * totalSupply / reserve0
            let liquidity0 = amount0
                .checked_mul(total_supply)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_LIQUIDITY_OVERFLOW)
                })?
                .checked_div(reserve0)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?;

            let liquidity1 = amount1
                .checked_mul(total_supply)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_LIQUIDITY_OVERFLOW)
                })?
                .checked_div(reserve1)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?;

            let liq = if liquidity0 < liquidity1 {
                liquidity0
            } else {
                liquidity1
            };

            if liq.is_zero() {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }

            liq
        };

        // Transfer tokens from caller to contract before updating state.
        let provider = msg::sender();
        let contract_addr = contract::address();
        if let Err(e) = safe_transfer_from(token0, provider, contract_addr, amount0) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }
        if let Err(e) = safe_transfer_from(token1, provider, contract_addr, amount1) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        // Update reserves after successful transfer (canonical order).
        let new_reserve0 = reserve0
            .checked_add(amount0_c)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE0_OVERFLOW)
            })?;
        let new_reserve1 = reserve1
            .checked_add(amount1_c)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE1_OVERFLOW)
            })?;

        pool.reserve0.set(new_reserve0);
        pool.reserve1.set(new_reserve1);

        // Mint LP tokens to provider (pool-specific).
        let current_total = pool.lp_total_supply.get();
        let new_total = current_total
            .checked_add(liquidity)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_total_supply.set(new_total);

        let current_balance = pool.lp_balances.setter(provider).get();
        let new_balance = current_balance
            .checked_add(liquidity)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_balances.setter(provider).set(new_balance);

        // LP token Transfer event (mint from zero).
        emit_lp_transfer(Address::ZERO, provider, liquidity);

        emit_add_liquidity(provider, amount0, amount1);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Remove liquidity from the pool.
    ///
    /// @notice Burns LP tokens and returns the underlying token0 and token1
    ///         to the provider in proportion to their share of total supply.
    /// @dev Uses the standard Uniswap V2 pro‑rata formula:
    ///      amount0 = lp_amount * reserve0 / totalSupply
    ///      amount1 = lp_amount * reserve1 / totalSupply
    pub fn remove_liquidity(
        &mut self,
        token0: Address,
        token1: Address,
        lp_amount: U256,
        amount0_min: U256,
        amount1_min: U256,
    ) -> OakResult<()> {
        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        if lp_amount.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ZERO_AMOUNT));
        }

        require_not_paused(self)?;

        // Canonical pool key
        let (pool_token0, pool_token1) = if token0 < token1 {
            (token0, token1)
        } else {
            (token1, token0)
        };
        let mut outer = self.pools.setter(pool_token0);
        let mut pool = outer.setter(pool_token1);
        if !pool.initialized.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }

        let provider = msg::sender();
        let total_supply = pool.lp_total_supply.get();
        if total_supply.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Check provider balance
        let balance = pool.lp_balances.getter(provider).get();
        if lp_amount > balance {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();

        // Pro-rata amounts to withdraw (canonical)
        let amount0_c = reserve0
            .checked_mul(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?
            .checked_div(total_supply)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;
        let amount1_c = reserve1
            .checked_mul(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?
            .checked_div(total_supply)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;
        if amount0_c.is_zero() || amount1_c.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }
        if amount0_c < amount0_min || amount1_c < amount1_min {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_LP_SLIPPAGE));
        }

        // Map canonical amounts back to user token order
        let (amount0, amount1) = if token0 == pool_token0 {
            (amount0_c, amount1_c)
        } else {
            (amount1_c, amount0_c)
        };

        // Update LP supply and balances
        let new_total = total_supply
            .checked_sub(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_total_supply.set(new_total);

        let new_balance = balance
            .checked_sub(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_balances.setter(provider).set(new_balance);

        // Update reserves after withdrawal (canonical)
        let (new_reserve0, new_reserve1) = if token0 == pool_token0 {
            let new_r0 = reserve0
                .checked_sub(amount0_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            let new_r1 = reserve1
                .checked_sub(amount1_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            (new_r0, new_r1)
        } else {
            let new_r0 = reserve0
                .checked_sub(amount1_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            let new_r1 = reserve1
                .checked_sub(amount0_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            (new_r0, new_r1)
        };

        pool.reserve0.set(new_reserve0);
        pool.reserve1.set(new_reserve1);

        // Transfer underlying tokens back to the provider
        if let Err(e) = safe_transfer(token0, provider, amount0) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }
        if let Err(e) = safe_transfer(token1, provider, amount1) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        // LP token Transfer event (burn to zero).
        emit_lp_transfer(provider, Address::ZERO, lp_amount);

        // Re-entrancy guard release
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Compute expected output amounts along a multi-hop path.
    ///
    /// @notice Pure view helper used by router/frontends to estimate
    ///         final amount_out for a given path, taking per-pool fees
    ///         into account.
    pub fn get_amounts_out(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> OakResult<Vec<U256>> {
        if path.len() < 2 {
            return Err(err(ERR_INVALID_PATH));
        }
        if path.len() as u64 > MAX_PATH_LENGTH {
            return Err(err(ERR_PATH_TOO_LONG));
        }
        if amount_in.is_zero() {
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }

        let mut amounts = Vec::with_capacity(path.len());
        amounts.push(amount_in);
        let mut current_in = amount_in;

        // Single global fee setting for now.
        let fee_bps = self.protocol_fee_bps.get();

        for i in 0..(path.len() - 1) {
            let input = path[i];
            let output = path[i + 1];

            if input == output {
                return Err(err(ERR_INVALID_PATH));
            }

            // Canonical pair ordering.
            let (token0, token1) = if input < output {
                (input, output)
            } else {
                (output, input)
            };

            let outer = self.pools.getter(token0);
            let pool = outer.getter(token1);
            if !pool.initialized.get() {
                return Err(err(ERR_INVALID_TOKEN));
            }

            let reserve0 = pool.reserve0.get();
            let reserve1 = pool.reserve1.get();
            if reserve0.is_zero() || reserve1.is_zero() {
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }

            // Direction within this pool.
            let (reserve_in, reserve_out) = if input == token0 {
                (reserve0, reserve1)
            } else {
                (reserve1, reserve0)
            };

            let out = get_amount_out_with_fee(current_in, reserve_in, reserve_out, fee_bps)?;
            if out.is_zero() {
                return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
            }

            amounts.push(out);
            current_in = out;
        }

        Ok(amounts)
    }

    /// Get current reserves for a given token pair.
    ///
    /// @notice Returns reserves in the same order as the input tokens.
    pub fn get_reserves(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> OakResult<(U256, U256)> {
        require_non_zero_address(token_a)?;
        require_non_zero_address(token_b)?;
        if token_a == token_b {
            return Err(err(ERR_INVALID_TOKEN));
        }

        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        let outer = self.pools.getter(token0);
        let pool = outer.getter(token1);
        if !pool.initialized.get() {
            return Err(err(ERR_INVALID_TOKEN));
        }

        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();

        // Map back to caller's token order
        let (out0, out1) = if token_a == token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };

        Ok((out0, out1))
    }

    /// Router-style multi-hop swap: exact input, minimum output.
    ///
    /// @notice Swaps an exact amount of the first token in `path` for as much
    ///         as possible of the last token, going through intermediate pools.
    /// @dev For now the recipient `to` must be the caller (`msg::sender`),
    ///      since `process_swap` always transfers to `sender`.
    pub fn swap_exact_tokens_for_tokens(
        &mut self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> OakResult<Vec<U256>> {
        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        // Basic input validation
        if amount_in.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        if amount_out_min.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }
        if path.len() < 2 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_PATH));
        }
        if path.len() as u64 > MAX_PATH_LENGTH {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PATH_TOO_LONG));
        }

        // Recipient must be non-zero and, в текущей версии, совпадать с sender.
        let sender = msg::sender();
        if to == Address::ZERO || to != sender {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_ADDRESS));
        }

        require_not_paused(self)?;
        require_not_circuit_breaker(self)?;

        // Deadline based on block timestamp
        let now = U256::from(block::timestamp());
        if now > deadline {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_EXPIRED));
        }

        // Compute expected amounts along the path
        let amounts = match self.get_amounts_out(amount_in, path.clone()) {
            Ok(v) => v,
            Err(e) => {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        };

        let final_out = *amounts.last().unwrap_or(&U256::ZERO);
        if final_out < amount_out_min {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }

        // Execute the multi-hop swaps sequentially.
        // At каждом шаге process_swap:
        // - списывает amount_in хопа с sender в контракт
        // - отправляет amount_out хопа обратно sender'у
        // - обновляет резервы пула (через PoolData)
        for i in 0..(path.len() - 1) {
            let token_in = path[i];
            let token_out = path[i + 1];
            let hop_in = amounts[i];
            let hop_min_out = amounts[i + 1]; // строгое ожидание по расчёту get_amounts_out

            if let Err(e) = process_swap(self, token_in, token_out, hop_in, hop_min_out) {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        }

        // Release re-entrancy guard
        unlock_reentrancy_guard(self);

        Ok(amounts)
    }

    // ---------- TP/SL/Limit orders (pro exchange features) ----------

    /// Place a TP, SL or Limit order. Tokens to sell are escrowed in the contract.
    ///
    /// @param token_in Token to receive when order executes.
    /// @param token_out Token to sell (transferred from caller to contract).
    /// @param amount_out Amount of token_out to sell.
    /// @param trigger_price For TP/Limit: execute when price >= this; for SL: when price <= this (price = reserve_in/reserve_out).
    /// @param order_type 0 = Limit, 1 = TP, 2 = SL.
    /// @param oco_with_order_id If non-zero, link this order with another (OCO). When either executes, the other is cancelled.
    pub fn place_order(
        &mut self,
        token_in: Address,
        token_out: Address,
        amount_out: U256,
        trigger_price: U256,
        order_type: U256,
        oco_with_order_id: U256,
    ) -> OakResult<U256> {
        lock_reentrancy_guard(self)?;
        require_non_zero_address(token_in)?;
        require_non_zero_address(token_out)?;
        if token_in == token_out {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }
        if amount_out.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        let order_type_u = order_type.as_limbs()[0];
        if order_type_u > 2 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_ORDER_TYPE));
        }
        require_not_paused(self)?;
        require_not_circuit_breaker(self)?;

        let sender = msg::sender();
        let contract_addr = contract::address();
        let balance = balance_of(token_out, sender);
        if balance < amount_out {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_BALANCE));
        }

        safe_transfer_from(token_out, sender, contract_addr, amount_out)?;

        let next_id = self.next_order_id.get();
        let new_id = next_id.checked_add(U256::from(1u64)).ok_or_else(|| {
            unlock_reentrancy_guard(self);
            err(ERR_OVERFLOW)
        })?;
        self.next_order_id.set(new_id);

        let key = order_id_to_address(new_id);
        self.order_owner.setter(key).set(sender);
        self.order_token_in.setter(key).set(token_in);
        self.order_token_out.setter(key).set(token_out);
        self.order_amount_out.setter(key).set(amount_out);
        self.order_trigger_price.setter(key).set(trigger_price);
        self.order_type.setter(key).set(order_type);
        self.order_status.setter(key).set(U256::ZERO); // Open
        self.order_created_at.setter(key).set(U256::from(block::number()));

        if !oco_with_order_id.is_zero() {
            let oco_key = order_id_to_address(oco_with_order_id);
            let oco_owner = self.order_owner.setter(oco_key).get();
            if oco_owner == Address::ZERO {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_OCO_PAIR_INVALID));
            }
            if oco_owner != sender {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_ORDER_NOT_OWNER));
            }
            let oco_status = self.order_status.setter(oco_key).get();
            if oco_status != U256::ZERO {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_OCO_PAIR_INVALID));
            }
            self.order_oco_pair.setter(key).set(oco_with_order_id);
            self.order_oco_pair.setter(oco_key).set(new_id);
        }

        emit_order_placed(new_id, sender, token_in, token_out, amount_out, trigger_price, order_type);
        unlock_reentrancy_guard(self);
        Ok(new_id)
    }

    /// Cancel an open order; returns escrowed tokens to the owner.
    pub fn cancel_order(&mut self, order_id: U256) -> OakResult<()> {
        lock_reentrancy_guard(self)?;
        let sender = msg::sender();
        let key = order_id_to_address(order_id);
        let owner = self.order_owner.setter(key).get();
        if owner == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ORDER_NOT_FOUND));
        }
        if owner != sender {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ORDER_NOT_OWNER));
        }
        let status = self.order_status.setter(key).get();
        if status != U256::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ORDER_NOT_OPEN));
        }
        let token_out = self.order_token_out.setter(key).get();
        let amount_out = self.order_amount_out.setter(key).get();
        self.order_status.setter(key).set(U256::from(2u64)); // Cancelled
        safe_transfer(token_out, sender, amount_out)?;
        let oco_pair = self.order_oco_pair.setter(key).get();
        if !oco_pair.is_zero() {
            let oco_key = order_id_to_address(oco_pair);
            self.order_oco_pair.setter(key).set(U256::ZERO);
            self.order_oco_pair.setter(oco_key).set(U256::ZERO);
        }
        emit_order_cancelled(order_id, sender);
        unlock_reentrancy_guard(self);
        Ok(())
    }

    /// Execute an open order when price condition is met. Anyone may call.
    ///
    /// @param order_id Order to execute.
    /// @param min_amount_out Minimum token_in to send to order owner (slippage).
    pub fn execute_order(&mut self, order_id: U256, min_amount_out: U256) -> OakResult<U256> {
        lock_reentrancy_guard(self)?;
        let key = order_id_to_address(order_id);
        let owner = self.order_owner.setter(key).get();
        if owner == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ORDER_NOT_FOUND));
        }
        let status = self.order_status.setter(key).get();
        if status != U256::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ORDER_NOT_OPEN));
        }
        let token_in = self.order_token_in.setter(key).get();
        let token_out = self.order_token_out.setter(key).get();
        let amount_out = self.order_amount_out.setter(key).get();
        let trigger_price = self.order_trigger_price.setter(key).get();
        let order_type = self.order_type.setter(key).get();

        let current_price = self.get_current_price(token_in, token_out)?;
        let order_type_u = order_type.as_limbs()[0];
        let condition_met = if order_type_u == 2 {
            current_price <= trigger_price
        } else {
            current_price >= trigger_price
        };
        if !condition_met {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ORDER_CONDITION_NOT_MET));
        }

        let contract_addr = contract::address();
        let amount_in_received = process_swap_from_to(
            self,
            contract_addr,
            owner,
            token_out,
            token_in,
            amount_out,
            min_amount_out,
        )?;
        self.order_status.setter(key).set(U256::from(1u64)); // Executed
        emit_order_executed(order_id, owner, amount_in_received);

        let oco_pair = self.order_oco_pair.setter(key).get();
        if !oco_pair.is_zero() {
            let oco_key = order_id_to_address(oco_pair);
            let oco_owner = self.order_owner.setter(oco_key).get();
            let oco_status = self.order_status.setter(oco_key).get();
            if oco_owner != Address::ZERO && oco_status == U256::ZERO {
                let oco_token_out = self.order_token_out.setter(oco_key).get();
                let oco_amount_out = self.order_amount_out.setter(oco_key).get();
                self.order_status.setter(oco_key).set(U256::from(2u64)); // Cancelled
                safe_transfer(oco_token_out, oco_owner, oco_amount_out)?;
                emit_order_cancelled(oco_pair, oco_owner);
            }
            self.order_oco_pair.setter(key).set(U256::ZERO);
            self.order_oco_pair.setter(oco_key).set(U256::ZERO);
        }

        unlock_reentrancy_guard(self);
        Ok(amount_in_received)
    }

    /// View: get order details by ID.
    pub fn get_order(
        &self,
        order_id: U256,
    ) -> OakResult<(Address, Address, Address, U256, U256, U256, U256, U256, U256)> {
        let key = order_id_to_address(order_id);
        let owner = self.order_owner.getter(key).get();
        if owner == Address::ZERO {
            return Err(err(ERR_ORDER_NOT_FOUND));
        }
        Ok((
            owner,
            self.order_token_in.getter(key).get(),
            self.order_token_out.getter(key).get(),
            self.order_amount_out.getter(key).get(),
            self.order_trigger_price.getter(key).get(),
            self.order_type.getter(key).get(),
            self.order_status.getter(key).get(),
            self.order_created_at.getter(key).get(),
            self.order_oco_pair.getter(key).get(),
        ))
    }

    /// View: current price (reserve_in / reserve_out) for token_in/token_out pair.
    pub fn get_current_price(&self, token_in: Address, token_out: Address) -> OakResult<U256> {
        let (r0, r1) = self.get_reserves(token_in, token_out)?;
        let (reserve_in, reserve_out) = if token_in < token_out {
            (r0, r1)
        } else {
            (r1, r0)
        };
        if reserve_out.is_zero() {
            return Err(err(ERR_DIVISION_BY_ZERO));
        }
        Ok(reserve_in.checked_div(reserve_out).unwrap_or(U256::ZERO))
    }

    // ---------- Tracked positions (pro terminal: PnL, TP/SL, close) ----------

    /// Open a tracked position after a swap (record size + entry price for PnL and TP/SL).
    ///
    /// @param base_token Token held (e.g. ETH); sold on close.
    /// @param quote_token Token to receive on close (e.g. USDC).
    /// @param size Amount of base token (18 decimals).
    /// @param entry_price Quote per base (18 decimals; from get_current_price at open).
    /// @param initial_collateral Optional margin in quote (18 decimals). If > 0, transferred from caller; used for liquidation price: liq_price = (collateral + margin_added) / size.
    pub fn open_position(
        &mut self,
        base_token: Address,
        quote_token: Address,
        size: U256,
        entry_price: U256,
        initial_collateral: U256,
    ) -> OakResult<U256> {
        lock_reentrancy_guard(self)?;
        require_non_zero_address(base_token)?;
        require_non_zero_address(quote_token)?;
        if base_token == quote_token {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }
        if size.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        require_not_paused(self)?;

        let sender = msg::sender();
        let contract_addr = contract::address();
        if !initial_collateral.is_zero() {
            let bal = balance_of(quote_token, sender);
            if bal < initial_collateral {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_MARGIN_ZERO_OR_INSUFFICIENT));
            }
            safe_transfer_from(quote_token, sender, contract_addr, initial_collateral)?;
            let prev = self.position_margin_balance.setter(quote_token).get();
            self.position_margin_balance
                .setter(quote_token)
                .set(prev.checked_add(initial_collateral).ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?);
        }

        let next_id = self.next_position_id.get();
        let new_id = next_id.checked_add(U256::from(1u64)).ok_or_else(|| {
            unlock_reentrancy_guard(self);
            err(ERR_OVERFLOW)
        })?;
        self.next_position_id.set(new_id);

        let key = position_id_to_address(new_id);
        self.position_owner.setter(key).set(sender);
        self.position_base.setter(key).set(base_token);
        self.position_quote.setter(key).set(quote_token);
        self.position_size.setter(key).set(size);
        self.position_entry_price.setter(key).set(entry_price);
        self.position_tp_price.setter(key).set(U256::ZERO);
        self.position_sl_price.setter(key).set(U256::ZERO);
        self.position_trailing_delta_bps.setter(key).set(U256::ZERO);
        self.position_trailing_peak_price.setter(key).set(U256::ZERO);
        self.position_initial_collateral.setter(key).set(initial_collateral);
        self.position_margin_added.setter(key).set(U256::ZERO);
        self.position_opened_at.setter(key).set(U256::from(block::number()));
        self.position_status.setter(key).set(U256::ZERO); // Open

        emit_open_position(new_id, sender, base_token, quote_token, size, entry_price);
        unlock_reentrancy_guard(self);
        Ok(new_id)
    }

    /// Add margin to an open position (increases collateral, does not change entry_price or size).
    ///
    /// @param amount Amount of quote token to add (18 decimals). Transferred from owner to contract.
    /// Liquidation price becomes (initial_collateral + margin_added + amount) / size.
    pub fn add_margin(&mut self, position_id: U256, amount: U256) -> OakResult<()> {
        lock_reentrancy_guard(self)?;
        if amount.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_MARGIN_ZERO_OR_INSUFFICIENT));
        }
        let sender = msg::sender();
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.setter(key).get();
        if owner == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        if owner != sender {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_OWNER));
        }
        let status = self.position_status.setter(key).get();
        if status != U256::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_OPEN));
        }
        let quote_token = self.position_quote.setter(key).get();
        let bal = balance_of(quote_token, sender);
        if bal < amount {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_MARGIN_ZERO_OR_INSUFFICIENT));
        }
        let contract_addr = contract::address();
        safe_transfer_from(quote_token, sender, contract_addr, amount)?;
        let prev_added = self.position_margin_added.setter(key).get();
        self.position_margin_added
            .setter(key)
            .set(prev_added.checked_add(amount).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?);
        let prev_balance = self.position_margin_balance.setter(quote_token).get();
        self.position_margin_balance
            .setter(quote_token)
            .set(prev_balance.checked_add(amount).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?);
        unlock_reentrancy_guard(self);
        Ok(())
    }

    /// Set or update Take-Profit and Stop-Loss prices for an open position.
    ///
    /// @param tp_price Execute when market price >= this (0 = clear TP).
    /// @param sl_price Execute when market price <= this (0 = clear SL).
    pub fn set_position_tp_sl(
        &mut self,
        position_id: U256,
        tp_price: U256,
        sl_price: U256,
    ) -> OakResult<()> {
        let sender = msg::sender();
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.setter(key).get();
        if owner == Address::ZERO {
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        if owner != sender {
            return Err(err(ERR_POSITION_NOT_OWNER));
        }
        let status = self.position_status.setter(key).get();
        if status != U256::ZERO {
            return Err(err(ERR_POSITION_NOT_OPEN));
        }
        self.position_tp_price.setter(key).set(tp_price);
        self.position_sl_price.setter(key).set(sl_price);
        emit_set_position_tp_sl(position_id, sender, tp_price, sl_price);
        Ok(())
    }

    /// Set trailing stop-loss for an open position (owner only).
    ///
    /// @param trailing_delta_bps Delta in basis points (e.g. 100 = 1%). Trigger when oracle price <= peak * (10000 - delta_bps) / 10000. Max 10000.
    /// @dev Initial peak is set to entry_price; off-chain bot updates peak via update_trailing_stop when price rises.
    pub fn set_position_trailing_stop(
        &mut self,
        position_id: U256,
        trailing_delta_bps: U256,
    ) -> OakResult<()> {
        let sender = msg::sender();
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.setter(key).get();
        if owner == Address::ZERO {
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        if owner != sender {
            return Err(err(ERR_POSITION_NOT_OWNER));
        }
        let status = self.position_status.setter(key).get();
        if status != U256::ZERO {
            return Err(err(ERR_POSITION_NOT_OPEN));
        }
        let delta = trailing_delta_bps.as_limbs()[0];
        if delta == 0 || delta > 10_000 {
            return Err(err(ERR_INVALID_ORDER_TYPE)); // reuse or add ERR_INVALID_DELTA
        }
        self.position_trailing_delta_bps.setter(key).set(trailing_delta_bps);
        let entry = self.position_entry_price.setter(key).get();
        self.position_trailing_peak_price.setter(key).set(entry);
        emit_set_position_trailing(position_id, sender, trailing_delta_bps, entry);
        Ok(())
    }

    /// Update trailing stop (call by off-chain bot on each oracle price tick).
    ///
    /// If new_price > peak, peak is updated. If new_price <= peak * (10000 - delta_bps) / 10000, position is closed
    /// (base transferred from owner to contract, swapped to quote, sent to owner). Owner must have approved the contract.
    ///
    /// @param new_price Current oracle price (quote per base, 18 decimals).
    /// @param min_amount_out Minimum quote to receive (slippage).
    pub fn update_trailing_stop(
        &mut self,
        position_id: U256,
        new_price: U256,
        min_amount_out: U256,
    ) -> OakResult<U256> {
        lock_reentrancy_guard(self)?;
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.setter(key).get();
        if owner == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        let status = self.position_status.setter(key).get();
        if status != U256::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_OPEN));
        }
        let delta_bps = self.position_trailing_delta_bps.setter(key).get();
        if delta_bps.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_TRAILING_DISABLED));
        }
        let mut peak = self.position_trailing_peak_price.setter(key).get();
        if new_price > peak {
            peak = new_price;
            self.position_trailing_peak_price.setter(key).set(peak);
        }
        let bps_u = as_u256(10_000u64);
        let trigger_num = peak
            .checked_mul(bps_u.saturating_sub(delta_bps))
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;
        let trigger_price = trigger_num
            .checked_div(bps_u)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;
        if new_price > trigger_price {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_TRAILING_NOT_TRIGGERED));
        }
        let base_token = self.position_base.setter(key).get();
        let quote_token = self.position_quote.setter(key).get();
        let size = self.position_size.setter(key).get();
        let initial_collateral = self.position_initial_collateral.setter(key).get();
        let margin_added = self.position_margin_added.setter(key).get();
        let margin_total = initial_collateral
            .checked_add(margin_added)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;
        if !margin_total.is_zero() {
            let prev = self.position_margin_balance.setter(quote_token).get();
            self.position_margin_balance
                .setter(quote_token)
                .set(prev.checked_sub(margin_total).ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?);
            self.position_initial_collateral.setter(key).set(U256::ZERO);
            self.position_margin_added.setter(key).set(U256::ZERO);
            safe_transfer(quote_token, owner, margin_total)?;
        }
        let amount_out = process_swap_from_to(
            self,
            owner,
            owner,
            base_token,
            quote_token,
            size,
            min_amount_out,
        )?;
        self.position_status.setter(key).set(U256::from(1u64)); // Closed
        emit_close_position(position_id, owner, amount_out);
        emit_trailing_stop_triggered(position_id, owner, peak, trigger_price, amount_out);
        unlock_reentrancy_guard(self);
        Ok(amount_out)
    }

    /// Close an open position: return margin to owner, market-sell base for quote, mark closed.
    ///
    /// @param min_amount_out Slippage protection (minimum quote to receive from swap).
    pub fn close_position(&mut self, position_id: U256, min_amount_out: U256) -> OakResult<U256> {
        lock_reentrancy_guard(self)?;
        require_not_paused(self)?;
        let sender = msg::sender();
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.setter(key).get();
        if owner == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        if owner != sender {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_OWNER));
        }
        let status = self.position_status.setter(key).get();
        if status != U256::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POSITION_NOT_OPEN));
        }

        let base_token = self.position_base.setter(key).get();
        let quote_token = self.position_quote.setter(key).get();
        let size = self.position_size.setter(key).get();
        let initial_collateral = self.position_initial_collateral.setter(key).get();
        let margin_added = self.position_margin_added.setter(key).get();
        let margin_total = initial_collateral
            .checked_add(margin_added)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        if !margin_total.is_zero() {
            let prev = self.position_margin_balance.setter(quote_token).get();
            self.position_margin_balance
                .setter(quote_token)
                .set(prev.checked_sub(margin_total).ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?);
            self.position_initial_collateral.setter(key).set(U256::ZERO);
            self.position_margin_added.setter(key).set(U256::ZERO);
            safe_transfer(quote_token, owner, margin_total)?;
        }

        let amount_out = process_swap_from_to(
            self,
            sender,
            sender,
            base_token,
            quote_token,
            size,
            min_amount_out,
        )?;
        self.position_status.setter(key).set(U256::from(1u64)); // Closed
        emit_close_position(position_id, sender, amount_out);
        unlock_reentrancy_guard(self);
        Ok(amount_out)
    }

    /// Execute TP/SL for a position if condition met (anyone may call; keeper-friendly).
    ///
    /// @param min_amount_out Slippage when closing.
    pub fn execute_position_tp_sl(
        &mut self,
        position_id: U256,
        min_amount_out: U256,
    ) -> OakResult<U256> {
        require_not_paused(self)?;
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.setter(key).get();
        if owner == Address::ZERO {
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        let status = self.position_status.setter(key).get();
        if status != U256::ZERO {
            return Err(err(ERR_POSITION_NOT_OPEN));
        }
        let base_token = self.position_base.setter(key).get();
        let quote_token = self.position_quote.setter(key).get();
        let tp_price = self.position_tp_price.setter(key).get();
        let sl_price = self.position_sl_price.setter(key).get();
        if tp_price.is_zero() && sl_price.is_zero() {
            return Err(err(ERR_POSITION_TP_SL_NOT_MET));
        }
        let current_price = self.get_current_price(base_token, quote_token)?;
        let tp_met = !tp_price.is_zero() && current_price >= tp_price;
        let sl_met = !sl_price.is_zero() && current_price <= sl_price;
        if !tp_met && !sl_met {
            return Err(err(ERR_POSITION_TP_SL_NOT_MET));
        }
        self.close_position(position_id, min_amount_out)
    }

    /// View: get position by ID.
    pub fn get_position(
        &self,
        position_id: U256,
    ) -> OakResult<(
        Address,
        Address,
        Address,
        U256,
        U256,
        U256,
        U256,
        U256,
        U256,
        U256,
        U256,
        U256,
        U256,
    )> {
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.getter(key).get();
        if owner == Address::ZERO {
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        Ok((
            owner,
            self.position_base.getter(key).get(),
            self.position_quote.getter(key).get(),
            self.position_size.getter(key).get(),
            self.position_entry_price.getter(key).get(),
            self.position_tp_price.getter(key).get(),
            self.position_sl_price.getter(key).get(),
            self.position_trailing_delta_bps.getter(key).get(),
            self.position_trailing_peak_price.getter(key).get(),
            self.position_initial_collateral.getter(key).get(),
            self.position_margin_added.getter(key).get(),
            self.position_opened_at.getter(key).get(),
            self.position_status.getter(key).get(),
        ))
    }

    /// View: liquidation price and health factor for a position (for frontend / liquidator).
    ///
    /// Liquidation price (long) = (initial_collateral + margin_added) / size (quote per base, 18 decimals).
    /// When mark_price <= liquidation_price, position is undercollateralized.
    /// Health factor (basis points) = current_price * 10_000 / liquidation_price. HF > 10_000 = healthy; HF <= 10_000 = at or below liquidation.
    ///
    /// @return (liquidation_price, health_factor_bps). If size is 0 or total margin is 0, liquidation_price is 0 and health_factor_bps is 0.
    pub fn get_position_health(
        &self,
        position_id: U256,
    ) -> OakResult<(U256, U256)> {
        let key = position_id_to_address(position_id);
        let owner = self.position_owner.getter(key).get();
        if owner == Address::ZERO {
            return Err(err(ERR_POSITION_NOT_FOUND));
        }
        let base_token = self.position_base.getter(key).get();
        let quote_token = self.position_quote.getter(key).get();
        let size = self.position_size.getter(key).get();
        let initial_collateral = self.position_initial_collateral.getter(key).get();
        let margin_added = self.position_margin_added.getter(key).get();
        let total_margin = initial_collateral
            .checked_add(margin_added)
            .unwrap_or(U256::ZERO);
        let liquidation_price = if size.is_zero() {
            U256::ZERO
        } else {
            total_margin.checked_div(size).unwrap_or(U256::ZERO)
        };
        let health_factor_bps = if liquidation_price.is_zero() {
            U256::ZERO
        } else {
            let current_price = self.get_current_price(base_token, quote_token).unwrap_or(U256::ZERO);
            current_price
                .checked_mul(as_u256(10_000u64))
                .and_then(|n| n.checked_div(liquidation_price))
                .unwrap_or(U256::ZERO)
        };
        Ok((liquidation_price, health_factor_bps))
    }

    /// View: next position ID (so indexers/frontends can scan 1..next_position_id - 1 for open positions).
    pub fn get_next_position_id(&self) -> U256 {
        self.next_position_id.get()
    }

    /// One-click close position: swap full amount of token_from for token_to (single hop).
    ///
    /// @param amount_in Amount of token_from to sell (e.g. user's balance; frontend passes it).
    pub fn close_position_market(
        &mut self,
        amount_in: U256,
        token_from: Address,
        token_to: Address,
        min_amount_out: U256,
    ) -> OakResult<U256> {
        if amount_in.is_zero() {
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        lock_reentrancy_guard(self)?;
        let out = process_swap(self, token_from, token_to, amount_in, min_amount_out)?;
        unlock_reentrancy_guard(self);
        Ok(out)
    }

    /// Cancel an expired or unwanted commitment.
    ///
    /// @notice Allows users to clear their commitment if it has expired or they no longer
    ///         wish to execute the swap. Prevents storage bloat from abandoned commitments.
    /// @dev Can only cancel own commitment, and only if expired or minimum delay has passed.
    ///
    /// # Returns
    /// `Ok(())` on successful cancellation
    pub fn cancel_commitment(&mut self) -> OakResult<()> {
        let sender = msg::sender();

        // Check if commitment exists
        let is_activated = self.commitment_activated.setter(sender).get();
        if !is_activated {
            return Err(err(ERR_COMMIT_NOT_FOUND));
        }

        let commit_block = self.commitment_timestamps.setter(sender).get();
        let current_block = U256::from(block::number());

        // Allow cancellation if:
        // 1. Commitment has expired (older than MAX_COMMITMENT_AGE blocks), OR
        // 2. Minimum delay has passed (user can cancel after reveal window)
        let max_block = commit_block
            .checked_add(as_u256(MAX_COMMITMENT_AGE))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        let min_block = commit_block
            .checked_add(as_u256(COMMIT_REVEAL_DELAY))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        // Can cancel if expired OR if minimum delay has passed
        if current_block <= max_block && current_block < min_block {
            // Cannot cancel: commitment is still valid and within reveal window
            return Err(err(ERR_TOO_EARLY));
        }

        // Clear commitment state
        self.commitment_activated.setter(sender).set(false);
        self.commitment_hashes.setter(sender).set(U256::ZERO);
        self.commitment_timestamps.setter(sender).set(U256::ZERO);

        emit_cancel_commitment(sender, current_block);

        Ok(())
    }

    /// Withdraw (claim) accrued treasury fees for a given token.
    ///
    /// @notice Owner-only. Transfers per-token treasury balance (20% of fees) to treasury address.
    /// @dev 60/20/20 model: 20% Treasury, 20% Buyback, 60% LP. Resets balance after transfer.
    pub fn withdraw_treasury_fees(&mut self, token: Address) -> OakResult<()> {
        let owner = self.owner.get();
        only_owner(owner)?;
        require_non_zero_address(token)?;
        lock_reentrancy_guard(self)?;

        let treasury = self.treasury.get();
        if treasury == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_OWNER));
        }
        let contract_addr = contract::address();
        if treasury == contract_addr {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_TREASURY_IS_CONTRACT));
        }

        let accrued = self.treasury_balance.setter(token).get();
        if accrued.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_NO_TREASURY_FEES));
        }
        let contract_balance = balance_of(token, contract_addr);
        if contract_balance < accrued {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        self.treasury_balance.setter(token).set(U256::ZERO);
        safe_transfer(token, treasury, accrued)?;
        emit_withdraw_treasury_fees(treasury, token, accrued);
        unlock_reentrancy_guard(self);
        Ok(())
    }

    /// Protocol analytics: total trading volume (global).
    ///
    /// @notice For dashboards and grant reviewers. Use get_treasury_balance(token) for per-token fees.
    pub fn get_protocol_analytics(&self) -> OakResult<(U256, U256)> {
        Ok((
            self.total_volume_token0.get(),
            self.total_volume_token1.get(),
        ))
    }

    /// Treasury balance for a token (claimable by owner via withdraw_treasury_fees).
    pub fn get_treasury_balance(&self, token: Address) -> OakResult<U256> {
        Ok(self.treasury_balance.getter(token).get())
    }

    /// Buyback fund balance for a token (20% of fees; for OAK buyback).
    pub fn get_buyback_balance(&self, token: Address) -> OakResult<U256> {
        Ok(self.buyback_balance.getter(token).get())
    }

    /// Trade impact for frontends: expected amounts, price impact per hop, fee per hop.
    ///
    /// @notice CEX-grade view: amount_out, price_impact_bps, fee in input token per hop.
    pub fn calculate_trade_impact(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> OakResult<(Vec<U256>, Vec<U256>, Vec<U256>)> {
        if path.len() as u64 > MAX_PATH_LENGTH {
            return Err(err(ERR_PATH_TOO_LONG));
        }
        let amounts = self.get_amounts_out(amount_in, path.clone())?;
        let fee_bps = self.protocol_fee_bps.get();
        let mut impacts = Vec::with_capacity(amounts.len().saturating_sub(1));
        let mut fees = Vec::with_capacity(amounts.len().saturating_sub(1));

        for i in 0..(path.len().saturating_sub(1)) {
            let input = path[i];
            let output = path[i + 1];
            let (token0, token1) = if input < output {
                (input, output)
            } else {
                (output, input)
            };
            let outer = self.pools.getter(token0);
            let pool = outer.getter(token1);
            if !pool.initialized.get() {
                return Err(err(ERR_INVALID_TOKEN));
            }
            let (reserve_in, reserve_out) = if input == token0 {
                (pool.reserve0.get(), pool.reserve1.get())
            } else {
                (pool.reserve1.get(), pool.reserve0.get())
            };
            let amt_in = amounts[i];
            let amt_out = amounts[i + 1];

            let fee_hop = amt_in
                .checked_mul(fee_bps)
                .ok_or_else(|| err(ERR_OVERFLOW))?
                .checked_div(as_u256(FEE_DENOMINATOR))
                .unwrap_or(U256::ZERO);
            fees.push(fee_hop);

            let impact_num = amt_out
                .checked_mul(reserve_in)
                .ok_or_else(|| err(ERR_OVERFLOW))?
                .checked_mul(as_u256(BPS))
                .ok_or_else(|| err(ERR_OVERFLOW))?;
            let impact_den = amt_in.checked_mul(reserve_out).ok_or_else(|| err(ERR_OVERFLOW))?;
            let impact_bps = if impact_den.is_zero() {
                U256::ZERO
            } else {
                impact_num.checked_div(impact_den).unwrap_or(U256::ZERO)
            };
            let price_impact_bps = as_u256(BPS)
                .checked_sub(impact_bps)
                .unwrap_or(U256::ZERO)
                .min(U256::from(10000u64));
            impacts.push(price_impact_bps);
        }

        Ok((amounts, impacts, fees))
    }

    /// LP position: balance and pool share in basis points.
    pub fn get_lp_position(
        &self,
        user: Address,
        token_a: Address,
        token_b: Address,
    ) -> OakResult<(U256, U256)> {
        require_non_zero_address(token_a)?;
        require_non_zero_address(token_b)?;
        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        let outer = self.pools.getter(token0);
        let pool = outer.getter(token1);
        if !pool.initialized.get() {
            return Err(err(ERR_INVALID_TOKEN));
        }
        let balance = pool.lp_balances.getter(user).get();
        let total = pool.lp_total_supply.get();
        let share_bps = if total.is_zero() {
            U256::ZERO
        } else {
            balance
                .checked_mul(U256::from(10000u64))
                .ok_or_else(|| err(ERR_OVERFLOW))?
                .checked_div(total)
                .unwrap_or(U256::ZERO)
        };
        Ok((balance, share_bps))
    }

    /// Amounts in required along path to get exact amount_out (last element). Rounds up per hop (protocol-safe).
    pub fn get_amounts_in(
        &self,
        amount_out: U256,
        path: Vec<Address>,
    ) -> OakResult<Vec<U256>> {
        if path.len() < 2 {
            return Err(err(ERR_INVALID_PATH));
        }
        if path.len() as u64 > MAX_PATH_LENGTH {
            return Err(err(ERR_PATH_TOO_LONG));
        }
        if amount_out.is_zero() {
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }
        let fee_bps = self.protocol_fee_bps.get();
        let mut amounts = Vec::with_capacity(path.len());
        let mut current_out = amount_out;
        for i in (0..path.len()).rev() {
            amounts.push(current_out);
            if i == 0 {
                break;
            }
            let output = path[i];
            let input = path[i - 1];
            if input == output {
                return Err(err(ERR_INVALID_PATH));
            }
            let (token0, token1) = if input < output {
                (input, output)
            } else {
                (output, input)
            };
            let outer = self.pools.getter(token0);
            let pool = outer.getter(token1);
            if !pool.initialized.get() {
                return Err(err(ERR_INVALID_TOKEN));
            }
            let (reserve_in, reserve_out) = if input == token0 {
                (pool.reserve0.get(), pool.reserve1.get())
            } else {
                (pool.reserve1.get(), pool.reserve0.get())
            };
            current_out = get_amount_in_with_fee(current_out, reserve_in, reserve_out, fee_bps)?;
        }
        amounts.reverse();
        Ok(amounts)
    }

    /// Quote: same as calculate_trade_impact (amounts, price_impact_bps per hop, fee per hop).
    pub fn get_quote(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> OakResult<(Vec<U256>, Vec<U256>, Vec<U256>)> {
        self.calculate_trade_impact(amount_in, path)
    }

    /// Impermanent loss estimate in basis points (pool-level). IL = 2*sqrt(r)/(1+r) - 1 where r = reserve1/reserve0.
    /// Returns approximate IL in bps (negative = loss). Uses scaled math to avoid overflow.
    pub fn get_impermanent_loss_bps(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> OakResult<U256> {
        require_non_zero_address(token_a)?;
        require_non_zero_address(token_b)?;
        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        let outer = self.pools.getter(token0);
        let pool = outer.getter(token1);
        if !pool.initialized.get() {
            return Err(err(ERR_INVALID_TOKEN));
        }
        let r0 = pool.reserve0.get();
        let r1 = pool.reserve1.get();
        if r0.is_zero() {
            return Ok(U256::ZERO);
        }
        let ratio_bps = r1
            .checked_mul(as_u256(BPS))
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_div(r0)
            .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
        let ratio_scaled = ratio_bps.checked_mul(as_u256(BPS)).ok_or_else(|| err(ERR_OVERFLOW))?;
        let sqrt_r = u256_sqrt(ratio_scaled);
        let two_sqrt = sqrt_r.checked_mul(U256::from(2u64)).ok_or_else(|| err(ERR_OVERFLOW))?;
        let denom = as_u256(BPS).checked_add(ratio_bps).ok_or_else(|| err(ERR_OVERFLOW))?;
        let value_lp_bps = two_sqrt
            .checked_mul(as_u256(BPS))
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_div(denom)
            .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
        let il_bps = value_lp_bps.saturating_sub(as_u256(BPS));
        Ok(il_bps)
    }

    /// Dynamic fee hook: currently returns base protocol fee. Future: volatility-based adjustment.
    pub fn get_dynamic_fee_bps(&self, _token_a: Address, _token_b: Address) -> OakResult<U256> {
        Ok(self.protocol_fee_bps.get())
    }

    /// Manually trigger circuit breaker (owner only). Stops swaps until cleared. Audit event.
    pub fn trigger_circuit_breaker(&mut self) -> OakResult<()> {
        only_owner(self.owner.get())?;
        self.circuit_breaker_triggered.set(true);
        emit_circuit_breaker_triggered(U256::ZERO); // 0 = manual trigger
        Ok(())
    }

    /// Clear circuit breaker (owner only). Re-enables swaps. Audit event.
    pub fn clear_circuit_breaker(&mut self) -> OakResult<()> {
        only_owner(self.owner.get())?;
        self.circuit_breaker_triggered.set(false);
        emit_circuit_breaker_cleared();
        Ok(())
    }

    /// Set buyback wallet (owner only). Can set to zero to disable.
    pub fn set_buyback_wallet(&mut self, wallet: Address) -> OakResult<()> {
        only_owner(self.owner.get())?;
        self.buyback_wallet.set(wallet);
        emit_buyback_wallet_set(wallet);
        Ok(())
    }

    /// Two-step ownership transfer (DoD-style). Pending owner must call accept_owner() after delay.
    pub fn set_pending_owner(&mut self, pending: Address) -> OakResult<()> {
        only_owner(self.owner.get())?;
        let after_block = U256::from(block::number())
            .checked_add(as_u256(OWNER_TRANSFER_DELAY_BLOCKS))
            .ok_or_else(|| err(ERR_OVERFLOW))?;
        self.pending_owner.set(pending);
        self.owner_transfer_after_block.set(after_block);
        emit_pending_owner_set(pending, after_block);
        Ok(())
    }

    /// Accept ownership (callable only by pending owner after delay).
    pub fn accept_owner(&mut self) -> OakResult<()> {
        let pending = self.pending_owner.get();
        if pending == Address::ZERO {
            return Err(err(ERR_NO_PENDING_OWNER));
        }
        if msg::sender() != pending {
            return Err(err(ERR_PENDING_OWNER_ONLY));
        }
        let after_block = self.owner_transfer_after_block.get();
        if U256::from(block::number()) < after_block {
            return Err(err(ERR_OWNER_TRANSFER_TOO_EARLY));
        }
        let old = self.owner.get();
        self.owner.set(pending);
        self.pending_owner.set(Address::ZERO);
        self.owner_transfer_after_block.set(U256::ZERO);
        emit_owner_changed(old, pending);
        Ok(())
    }

    /// Execute a flash swap (uncollateralized loan).
    ///
    /// @notice Allows borrowing tokens without upfront collateral, provided the borrower
    ///         returns the borrowed amount plus fees within the same transaction.
    /// @dev Uses a callback pattern via `IOakCallee` to notify the borrower.
    ///      After the callback, the new product of reserves (k = reserve0 * reserve1)
    ///      must be greater than or equal to the product before the swap, including fees.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0 (can be borrowed if amount0_out > 0)
    /// * `token1` - Address of token1 (can be borrowed if amount1_out > 0)
    /// * `amount0_out` - Amount of token0 to borrow (0 if not borrowing token0)
    /// * `amount1_out` - Amount of token1 to borrow (0 if not borrowing token1)
    /// * `data` - Optional calldata to pass to the callback
    ///
    /// # Safety
    /// - Re-entrancy guard is active during the entire flash swap
    /// - Verifies k' >= k * (1 + fee) after callback
    /// - Reverts if insufficient liquidity or repayment fails
    #[cfg(all(not(test), target_arch = "wasm32"))]
    pub fn flash_swap(
        &mut self,
        token0: Address,
        token1: Address,
        amount0_out: U256,
        amount1_out: U256,
        data: Vec<u8>,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
        // This must be the first state-modifying operation
        lock_reentrancy_guard(self)?;

        // Input sanitization: validate addresses
        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        // Input sanitization: at least one amount must be non-zero
        if amount0_out.is_zero() && amount1_out.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }

        require_not_paused(self)?;

        // Snapshot reserves and fee configuration before the swap
        let reserve0_before = self.reserves0.get();
        let reserve1_before = self.reserves1.get();
        let fee_bps = self.protocol_fee_bps.get();

        // Calculate initial k (constant product before swap)
        let k_before = reserve0_before
            .checked_mul(reserve1_before)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        // Verify sufficient liquidity for the requested amounts
        if amount0_out > reserve0_before || amount1_out > reserve1_before {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Calculate new reserves after lending (before callback)
        let reserve0_after_lend = reserve0_before
            .checked_sub(amount0_out)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        let reserve1_after_lend = reserve1_before
            .checked_sub(amount1_out)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        // Ensure minimum liquidity is maintained
        let min_liquidity = self.min_liquidity.get();
        if reserve0_after_lend < min_liquidity || reserve1_after_lend < min_liquidity {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Transfer tokens to borrower (INTERACTION: external call)
        let borrower = msg::sender();
        let contract_addr = contract::address();

        if !amount0_out.is_zero() {
            if let Err(e) = safe_transfer(token0, borrower, amount0_out) {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        }

        if !amount1_out.is_zero() {
            if let Err(e) = safe_transfer(token1, borrower, amount1_out) {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        }

        // Calculate fees owed (0.3% of borrowed amounts)
        // Fee calculation: fee = amount * fee_bps / FEE_DENOMINATOR
        let fee0 = if !amount0_out.is_zero() {
            amount0_out
                .checked_mul(fee_bps)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?
                .checked_div(as_u256(FEE_DENOMINATOR))
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?
        } else {
            U256::ZERO
        };

        let fee1 = if !amount1_out.is_zero() {
            amount1_out
                .checked_mul(fee_bps)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?
                .checked_div(as_u256(FEE_DENOMINATOR))
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?
        } else {
            U256::ZERO
        };

        // Calculate total repayment amounts (borrowed + fees)
        let amount0_owed = amount0_out
            .checked_add(fee0)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        let amount1_owed = amount1_out
            .checked_add(fee1)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        // Call callback (INTERACTION: external call to borrower's contract)
        // The borrower must implement: oakFlashSwapCallback(uint256,uint256,bytes)
        // We use ABI encoding to call the callback function
        let selector = crypto::keccak(b"oakFlashSwapCallback(uint256,uint256,bytes)");
        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector[0..4]); // Function selector (first 4 bytes)
        
        // ABI encode parameters: (uint256, uint256, bytes)
        // For uint256: pad to 32 bytes, big-endian
        call_data.extend_from_slice(&amount0_owed.to_be_bytes::<32>());
        call_data.extend_from_slice(&amount1_owed.to_be_bytes::<32>());
        
        // For bytes: offset (32 bytes) + length (32 bytes) + data (padded to 32-byte boundary)
        let data_offset = U256::from(96u64); // offset to data: 32 (amount0) + 32 (amount1) + 32 (offset)
        call_data.extend_from_slice(&data_offset.to_be_bytes::<32>());
        let data_len = U256::from(data.len());
        call_data.extend_from_slice(&data_len.to_be_bytes::<32>());
        call_data.extend_from_slice(&data);
        // Pad data to 32-byte boundary
        let padding = (32 - (data.len() % 32)) % 32;
        for _ in 0..padding {
            call_data.push(0u8);
        }
        
        // Make the external call - this will revert if callback fails.
        // The callback must transfer the repayment tokens back to this contract.
        // Stylus call API: call::call(context, to, data).
        if let Err(e) = call::call(Call::new(), borrower, &call_data) {
            unlock_reentrancy_guard(self);
            return Err(e.into());
        }

        // Verify repayment: check contract balances after callback
        let balance0_after = balance_of(token0, contract_addr);
        let balance1_after = balance_of(token1, contract_addr);

        // Calculate what the balances should be after repayment
        // We need: balance0_after >= reserve0_after_lend + amount0_owed
        //         balance1_after >= reserve1_after_lend + amount1_owed
        let expected_balance0 = reserve0_after_lend
            .checked_add(amount0_owed)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        let expected_balance1 = reserve1_after_lend
            .checked_add(amount1_owed)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        if balance0_after < expected_balance0 || balance1_after < expected_balance1 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Calculate actual repayment amounts (may be more than required)
        let actual_repayment0 = balance0_after
            .checked_sub(reserve0_after_lend)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        let actual_repayment1 = balance1_after
            .checked_sub(reserve1_after_lend)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        // Update reserves to reflect the repayment
        // New reserves = reserves_after_lend + actual_repayment
        let reserve0_after = reserve0_after_lend
            .checked_add(actual_repayment0)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE0_OVERFLOW)
            })?;

        let reserve1_after = reserve1_after_lend
            .checked_add(actual_repayment1)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE1_OVERFLOW)
            })?;

        // CRITICAL: Verify k' >= k * (1 + fee_rate)
        // This ensures the protocol doesn't lose value and collects fees
        // k_after = reserve0_after * reserve1_after
        let k_after = reserve0_after
            .checked_mul(reserve1_after)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        // Calculate minimum k required: k_min = k_before * (FEE_DENOMINATOR + fee_bps) / FEE_DENOMINATOR
        // This ensures the new product includes the 0.3% fee as required
        // Example: if fee_bps = 30 (0.3%), then k_min = k_before * 10030 / 10000
        let fee_multiplier = as_u256(FEE_DENOMINATOR)
            .checked_add(fee_bps)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        let k_min = k_before
            .checked_mul(fee_multiplier)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?
            .checked_div(as_u256(FEE_DENOMINATOR))
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;

        // Verify k_after >= k_min (protocol must not lose value, fees must be paid)
        // This is the core requirement: new product must be >= old product * (1 + fee)
        if k_after < k_min {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Update reserves (EFFECT: state change)
        self.reserves0.set(reserve0_after);
        self.reserves1.set(reserve1_after);

        // Update analytics: track flash swap volume
        let current_volume0 = self.total_volume_token0.get();
        let current_volume1 = self.total_volume_token1.get();

        if !amount0_out.is_zero() {
            let new_volume0 = current_volume0
                .checked_add(amount0_out)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_VOLUME_OVERFLOW)
                })?;
            self.total_volume_token0.set(new_volume0);
        }

        if !amount1_out.is_zero() {
            let new_volume1 = current_volume1
                .checked_add(amount1_out)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_VOLUME_OVERFLOW)
                })?;
            self.total_volume_token1.set(new_volume1);
        }

        // Update fee accounting (60/20/20: per-token treasury and buyback)
        if !fee0.is_zero() {
            let (_e, treasury_fee0, _lp0, buyback_fee0) =
                match compute_fee_split(amount0_out, fee_bps) {
                    Ok(s) => s,
                    Err(e) => {
                        unlock_reentrancy_guard(self);
                        return Err(e);
                    }
                };
            let pt = self.treasury_balance.setter(token0);
            let pb = self.buyback_balance.setter(token0);
            pt.set(pt.get().checked_add(treasury_fee0).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?);
            pb.set(pb.get().checked_add(buyback_fee0).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?);
        }
        if !fee1.is_zero() {
            let (_e, treasury_fee1, _lp1, buyback_fee1) =
                match compute_fee_split(amount1_out, fee_bps) {
                    Ok(s) => s,
                    Err(e) => {
                        unlock_reentrancy_guard(self);
                        return Err(e);
                    }
                };
            let pt = self.treasury_balance.setter(token1);
            let pb = self.buyback_balance.setter(token1);
            pt.set(pt.get().checked_add(treasury_fee1).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?);
            pb.set(pb.get().checked_add(buyback_fee1).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?);
        }

        // Emit FlashSwap event
        emit_flash_swap(borrower, token0, token1, amount0_out, amount1_out, fee0, fee1);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }
}

/// Host/test stub for `flash_swap`.
///
/// Compiled for non-wasm32 targets (including `cargo test`) to keep
/// the public interface of `OakDEX` intact without pulling in Stylus
/// call machinery. The real implementation above is only enabled for
/// on-chain (wasm32) builds.
#[cfg(any(test, not(target_arch = "wasm32")))]
impl OakDEX {
    pub fn flash_swap(
        &mut self,
        _token0: Address,
        _token1: Address,
        _amount0_out: U256,
        _amount1_out: U256,
        _data: Vec<u8>,
    ) -> OakResult<()> {
        Err(err(ERR_PAUSED))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpmm_math_respects_fee() {
        let amount_in = U256::from(1_000u64);
        let reserve_in = U256::from(10_000u64);
        let reserve_out = U256::from(20_000u64);
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let out =
            get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps).unwrap();

        // Sanity: positive output and strictly less than pro‑rata without fees.
        assert!(out > U256::ZERO);

        let out_no_fee = amount_in * reserve_out / (reserve_in + amount_in);
        assert!(out < out_no_fee);
    }

    #[test]
    fn fee_split_matches_ratios() {
        let amount_in = U256::from(1_000_000u64);
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let (_effective_in, treasury_fee, lp_fee, buyback_fee) =
            compute_fee_split(amount_in, fee_bps).unwrap();

        // Total fee should be 0.3% of amount_in.
        let total_fee = treasury_fee + lp_fee + buyback_fee;
        let expected_total_fee = amount_in * as_u256(DEFAULT_FEE_BPS) / as_u256(FEE_DENOMINATOR);
        assert_eq!(total_fee, expected_total_fee);

        // 60% LP, 20% Treasury, 20% Buyback of total fee.
        let expected_treasury = total_fee * as_u256(TREASURY_FEE_PCT) / U256::from(100u64);
        let expected_lp = total_fee * as_u256(LP_FEE_PCT) / U256::from(100u64);
        let expected_buyback = total_fee * as_u256(BUYBACK_FEE_PCT) / U256::from(100u64);

        assert_eq!(treasury_fee, expected_treasury);
        assert_eq!(lp_fee, expected_lp);
        assert_eq!(buyback_fee, expected_buyback);
    }

    #[test]
    fn commit_hash_roundtrip() {
        let amount_in = U256::from(42u64);
        let salt = U256::from(1337u64);

        let hash = compute_commit_hash(amount_in, salt);

        let encoded = encode_commit_data(amount_in, salt);
        let direct = crypto::keccak(&encoded);

        assert_eq!(hash, direct);
    }

    #[test]
    fn fee_split_no_precision_loss() {
        // Test that rounding never causes protocol to lose 1 wei
        // Use values that don't divide evenly to test rounding protection
        let amount_in = U256::from(1_000_001u64); // 1M + 1 (tests rounding)
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let (_effective_in, treasury_fee, lp_fee, buyback_fee) =
            compute_fee_split(amount_in, fee_bps).unwrap();

        // Calculate expected total fee
        let expected_total_fee = amount_in
            .checked_mul(fee_bps)
            .unwrap()
            .checked_div(as_u256(FEE_DENOMINATOR))
            .unwrap();

        // Verify: treasury_fee + lp_fee + buyback_fee = total_fee exactly (no precision loss)
        let actual_total_fee = treasury_fee
            .checked_add(lp_fee)
            .unwrap()
            .checked_add(buyback_fee)
            .unwrap();

        assert_eq!(
            actual_total_fee, expected_total_fee,
            "Fee split must not lose precision: treasury+lp+buyback = {}, expected {}",
            actual_total_fee, expected_total_fee
        );

        // Verify effective_in calculation
        let calculated_effective_in = amount_in
            .checked_sub(expected_total_fee)
            .unwrap();
        assert_eq!(_effective_in, calculated_effective_in);
    }

    #[test]
    fn cpmm_floor_rounding_favors_protocol() {
        // Test that CPMM calculation uses floor rounding (protocol-favorable)
        let amount_in = U256::from(1_000u64);
        let reserve_in = U256::from(10_000u64);
        let reserve_out = U256::from(20_000u64);
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let amount_out = get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps)
            .unwrap();

        // Calculate exact value (with infinite precision)
        // amount_in_with_fee = amount_in * (FEE_DENOMINATOR - fee_bps) / FEE_DENOMINATOR
        // amount_out_exact = (amount_in_with_fee * reserve_out) / (reserve_in * FEE_DENOMINATOR + amount_in_with_fee)
        let amount_in_with_fee = amount_in
            .checked_mul(as_u256(FEE_DENOMINATOR).checked_sub(fee_bps).unwrap())
            .unwrap()
            .checked_div(as_u256(FEE_DENOMINATOR))
            .unwrap();

        let numerator = amount_in_with_fee
            .checked_mul(reserve_out)
            .unwrap();

        let denominator = reserve_in
            .checked_mul(as_u256(FEE_DENOMINATOR))
            .unwrap()
            .checked_add(amount_in_with_fee)
            .unwrap();

        // Integer division performs floor rounding
        let expected_floor = numerator.checked_div(denominator).unwrap();

        assert_eq!(
            amount_out, expected_floor,
            "CPMM must use floor rounding (protocol-favorable)"
        );
    }
}

