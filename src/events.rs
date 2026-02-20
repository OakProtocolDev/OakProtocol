//! Solidity-compatible events for Oak Protocol.
//!
//! @notice Event helper functions for logging Solidity-compatible events.
//! @dev Uses evm::raw_log for maximum compatibility with Stylus SDK 0.6.

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    evm,
    prelude::*,
};

/// Emit CommitSwap event.
pub fn emit_commit_swap(user: Address, hash: FixedBytes<32>, block_number: U256) {
    let topics = &[user.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&hash.0);
    data.extend_from_slice(&block_number.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit RevealSwap event.
pub fn emit_reveal_swap(
    user: Address,
    amount_in: U256,
    amount_out: U256,
    treasury_fee: U256,
    lp_fee: U256,
) {
    let topics = &[user.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out.to_be_bytes::<32>());
    data.extend_from_slice(&treasury_fee.to_be_bytes::<32>());
    data.extend_from_slice(&lp_fee.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit AddLiquidity event.
pub fn emit_add_liquidity(provider: Address, amount0: U256, amount1: U256) {
    let topics = &[provider.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&amount0.to_be_bytes::<32>());
    data.extend_from_slice(&amount1.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit SetFee event.
pub fn emit_set_fee(new_fee_bps: u16) {
    let topics = &[];
    let mut data = Vec::new();
    data.extend_from_slice(&U256::from(new_fee_bps).to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit PauseChanged event.
pub fn emit_pause_changed(paused: bool) {
    let topics = &[];
    let mut data = Vec::new();
    data.extend_from_slice(&U256::from(paused as u8).to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit WithdrawTreasuryFees event.
pub fn emit_withdraw_treasury_fees(treasury: Address, token: Address, amount: U256) {
    let topics = &[treasury.into_word(), token.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit CancelCommitment event.
pub fn emit_cancel_commitment(user: Address, block_number: U256) {
    let topics = &[user.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&block_number.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit FlashSwap event.
///
/// @notice Emitted when a flash swap is initiated and completed.
/// @dev Includes borrower address, token addresses, borrowed amounts, and fees paid.
pub fn emit_flash_swap(
    borrower: Address,
    token0: Address,
    token1: Address,
    amount0_out: U256,
    amount1_out: U256,
    fee0: U256,
    fee1: U256,
) {
    let topics = &[borrower.into_word(), token0.into_word(), token1.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&amount0_out.to_be_bytes::<32>());
    data.extend_from_slice(&amount1_out.to_be_bytes::<32>());
    data.extend_from_slice(&fee0.to_be_bytes::<32>());
    data.extend_from_slice(&fee1.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

