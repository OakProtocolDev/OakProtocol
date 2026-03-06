//! Solidity-compatible events for Oak Protocol.
//!
//! @notice Event helper functions for logging Solidity-compatible events.
//! @dev Uses evm::raw_log for maximum compatibility with Stylus SDK 0.6.

use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    evm,
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

/// Emit LP token Transfer-like event for LP balances.
///
/// @notice Mimics ERC-20 `Transfer` for LP tokens so that wallets
///         and indexers can track LP positions.
pub fn emit_lp_transfer(from: Address, to: Address, value: U256) {
    let topics = &[from.into_word(), to.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&value.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when circuit breaker auto-triggers (price impact exceeded). Audit trail.
pub fn emit_circuit_breaker_triggered(price_impact_bps: U256) {
    let topics = &[];
    let mut data = Vec::new();
    data.extend_from_slice(&price_impact_bps.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when owner clears circuit breaker.
pub fn emit_circuit_breaker_cleared() {
    let topics = &[];
    let data: &[u8] = &[];
    let _ = evm::raw_log(topics, data);
}

/// Emit when a new pool is created. Indexers use this to enumerate pairs.
pub fn emit_pool_created(token0: Address, token1: Address) {
    let topics = &[token0.into_word(), token1.into_word()];
    let data: &[u8] = &[];
    let _ = evm::raw_log(topics, data);
}

/// Emit when buyback wallet is set (owner-only).
pub fn emit_buyback_wallet_set(wallet: Address) {
    let topics = &[wallet.into_word()];
    let data: &[u8] = &[];
    let _ = evm::raw_log(topics, data);
}

/// Emit when pending owner is set (two-step transfer).
pub fn emit_pending_owner_set(pending: Address, transfer_after_block: U256) {
    let topics = &[pending.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&transfer_after_block.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when ownership is transferred (after accept_owner).
pub fn emit_owner_changed(old_owner: Address, new_owner: Address) {
    let topics = &[old_owner.into_word(), new_owner.into_word()];
    let data: &[u8] = &[];
    let _ = evm::raw_log(topics, data);
}

/// Emit when a TP/SL/Limit order is placed.
pub fn emit_order_placed(
    order_id: U256,
    owner: Address,
    token_in: Address,
    token_out: Address,
    amount_out: U256,
    trigger_price: U256,
    order_type: U256,
) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&order_id.to_be_bytes::<32>());
    data.extend_from_slice(token_in.as_slice());
    data.extend_from_slice(token_out.as_slice());
    data.extend_from_slice(&amount_out.to_be_bytes::<32>());
    data.extend_from_slice(&trigger_price.to_be_bytes::<32>());
    data.extend_from_slice(&order_type.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when an order is cancelled (tokens returned to owner).
pub fn emit_order_cancelled(order_id: U256, owner: Address) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&order_id.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when an order is executed (TP/SL/Limit filled).
pub fn emit_order_executed(order_id: U256, owner: Address, amount_in_received: U256) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&order_id.to_be_bytes::<32>());
    data.extend_from_slice(&amount_in_received.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

// ---------- Position events (pro terminal) ----------

/// Emit when a position is opened (tracked for PnL/TP/SL).
pub fn emit_open_position(
    position_id: U256,
    owner: Address,
    base_token: Address,
    quote_token: Address,
    size: U256,
    entry_price: U256,
) {
    let topics = &[owner.into_word(), base_token.into_word(), quote_token.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&position_id.to_be_bytes::<32>());
    data.extend_from_slice(&size.to_be_bytes::<32>());
    data.extend_from_slice(&entry_price.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when a position is closed (market sell).
pub fn emit_close_position(position_id: U256, owner: Address, amount_out: U256) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&position_id.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when TP/SL is set or updated on a position.
pub fn emit_set_position_tp_sl(position_id: U256, owner: Address, tp_price: U256, sl_price: U256) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&position_id.to_be_bytes::<32>());
    data.extend_from_slice(&tp_price.to_be_bytes::<32>());
    data.extend_from_slice(&sl_price.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when trailing stop is set on a position.
pub fn emit_set_position_trailing(position_id: U256, owner: Address, trailing_delta_bps: U256, initial_peak: U256) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&position_id.to_be_bytes::<32>());
    data.extend_from_slice(&trailing_delta_bps.to_be_bytes::<32>());
    data.extend_from_slice(&initial_peak.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

/// Emit when trailing stop triggers a close (oracle price dropped below trigger level).
pub fn emit_trailing_stop_triggered(position_id: U256, owner: Address, peak_price: U256, trigger_price: U256, amount_out: U256) {
    let topics = &[owner.into_word()];
    let mut data = Vec::new();
    data.extend_from_slice(&position_id.to_be_bytes::<32>());
    data.extend_from_slice(&peak_price.to_be_bytes::<32>());
    data.extend_from_slice(&trigger_price.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out.to_be_bytes::<32>());
    let _ = evm::raw_log(topics, &data);
}

