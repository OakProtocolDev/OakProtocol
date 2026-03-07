//! Shared error helpers and result type for Oak Protocol.

use alloc::vec::Vec;

/// Canonical result type used across the protocol.
pub type OakResult<T> = Result<T, Vec<u8>>;

/// Helper to build a `Vec<u8>` from a string literal at call site.
#[inline]
pub fn err(msg: &'static [u8]) -> Vec<u8> {
    msg.to_vec()
}

// Core error codes (Solidity-style short strings for tooling friendliness).
pub const ERR_ALREADY_INITIALIZED: &[u8] = b"ALREADY_INITIALIZED";
pub const ERR_INVALID_OWNER: &[u8] = b"INVALID_OWNER";
pub const ERR_ONLY_OWNER: &[u8] = b"ONLY_OWNER";
pub const ERR_FEE_TOO_HIGH: &[u8] = b"FEE_TOO_HIGH";
pub const ERR_PAUSED: &[u8] = b"PAUSED";
pub const ERR_AMOUNT0_ZERO: &[u8] = b"AMOUNT0_ZERO";
pub const ERR_AMOUNT1_ZERO: &[u8] = b"AMOUNT1_ZERO";
pub const ERR_LIQUIDITY_OVERFLOW: &[u8] = b"LIQUIDITY_OVERFLOW";
pub const ERR_RESERVE0_OVERFLOW: &[u8] = b"RESERVE0_OVERFLOW";
pub const ERR_RESERVE1_OVERFLOW: &[u8] = b"RESERVE1_OVERFLOW";
pub const ERR_INSUFFICIENT_LIQUIDITY: &[u8] = b"INSUFFICIENT_LIQUIDITY";
pub const ERR_VOLUME_OVERFLOW: &[u8] = b"VOLUME_OVERFLOW";

pub const ERR_INSUFFICIENT_INPUT_AMOUNT: &[u8] = b"INSUFFICIENT_INPUT_AMOUNT";
pub const ERR_INSUFFICIENT_OUTPUT_AMOUNT: &[u8] = b"INSUFFICIENT_OUTPUT_AMOUNT";
/// Strict slippage protection: actual output below user minimum.
pub const ERR_SLIPPAGE_EXCEEDED: &[u8] = b"SLIPPAGE_EXCEEDED";
/// Deadline protection: transaction included after user-specified deadline.
pub const ERR_DEADLINE_EXPIRED: &[u8] = b"DEADLINE_EXPIRED";
pub const ERR_OVERFLOW: &[u8] = b"OVERFLOW";
pub const ERR_FEE_OVERFLOW: &[u8] = b"FEE_OVERFLOW";
pub const ERR_DIVISION_BY_ZERO: &[u8] = b"DIVISION_BY_ZERO";

pub const ERR_INVALID_HASH: &[u8] = b"INVALID_HASH";
pub const ERR_COMMIT_NOT_FOUND: &[u8] = b"COMMIT_NOT_FOUND";
pub const ERR_BLOCK_OVERFLOW: &[u8] = b"BLOCK_OVERFLOW";
pub const ERR_TOO_EARLY: &[u8] = b"TOO_EARLY";
pub const ERR_COMMITMENT_EXPIRED: &[u8] = b"COMMITMENT_EXPIRED";
pub const ERR_INVALID_ADDRESS: &[u8] = b"INVALID_ADDRESS";

// Token transfer errors
pub const ERR_TOKEN_TRANSFER_FAILED: &[u8] = b"TOKEN_TRANSFER_FAILED";
pub const ERR_INSUFFICIENT_BALANCE: &[u8] = b"INSUFFICIENT_BALANCE";
pub const ERR_ZERO_AMOUNT: &[u8] = b"ZERO_AMOUNT";

// Re-entrancy guard errors
pub const ERR_REENTRANT_CALL: &[u8] = b"REENTRANT_CALL";

// Treasury withdrawal errors
pub const ERR_NO_TREASURY_FEES: &[u8] = b"NO_TREASURY_FEES";
pub const ERR_INVALID_TOKEN: &[u8] = b"INVALID_TOKEN";
pub const ERR_POOL_EXISTS: &[u8] = b"POOL_EXISTS";
pub const ERR_INVALID_PATH: &[u8] = b"INVALID_PATH";
pub const ERR_EXPIRED: &[u8] = b"EXPIRED";

// Vault (GMX-style) errors
pub const ERR_VAULT_POOL_EXCEEDED: &[u8] = b"VAULT_POOL_EXCEEDED";
pub const ERR_VAULT_INSUFFICIENT_RESERVE: &[u8] = b"VAULT_INSUFFICIENT_RESERVE";
pub const ERR_VAULT_RESERVE_EXCEEDS_POOL: &[u8] = b"VAULT_RESERVE_EXCEEDS_POOL";
pub const ERR_VAULT_BUFFER: &[u8] = b"VAULT_BUFFER";

/// Circuit breaker triggered; swaps disabled until owner clears.
pub const ERR_CIRCUIT_BREAKER: &[u8] = b"CIRCUIT_BREAKER";

/// Path length exceeds MAX_PATH_LENGTH.
pub const ERR_PATH_TOO_LONG: &[u8] = b"PATH_TOO_LONG";
/// LP add liquidity: received below minimum (slippage).
pub const ERR_LP_SLIPPAGE: &[u8] = b"LP_SLIPPAGE";
/// Single trade size exceeds MAX_TRADE_RESERVE_BPS of reserve (bank cap).
pub const ERR_TRADE_TOO_LARGE: &[u8] = b"TRADE_TOO_LARGE";
/// Caller is not the pending owner.
pub const ERR_PENDING_OWNER_ONLY: &[u8] = b"PENDING_OWNER_ONLY";
/// No pending owner transfer.
pub const ERR_NO_PENDING_OWNER: &[u8] = b"NO_PENDING_OWNER";
/// Owner transfer delay not yet elapsed.
pub const ERR_OWNER_TRANSFER_TOO_EARLY: &[u8] = b"OWNER_TRANSFER_TOO_EARLY";
/// Treasury cannot be the contract itself (would lock funds).
pub const ERR_TREASURY_IS_CONTRACT: &[u8] = b"TREASURY_IS_CONTRACT";

// TP/SL/Limit order errors
/// Order not found or invalid order ID.
pub const ERR_ORDER_NOT_FOUND: &[u8] = b"ORDER_NOT_FOUND";
/// Order is not in Open status (already executed or cancelled).
pub const ERR_ORDER_NOT_OPEN: &[u8] = b"ORDER_NOT_OPEN";
/// Caller is not the order owner.
pub const ERR_ORDER_NOT_OWNER: &[u8] = b"ORDER_NOT_OWNER";
/// Invalid order type (must be 0 Limit, 1 TP, 2 SL).
pub const ERR_INVALID_ORDER_TYPE: &[u8] = b"INVALID_ORDER_TYPE";
/// Price condition not met for execution.
pub const ERR_ORDER_CONDITION_NOT_MET: &[u8] = b"ORDER_CONDITION_NOT_MET";

// Position errors (pro terminal)
/// Position not found or invalid position ID.
pub const ERR_POSITION_NOT_FOUND: &[u8] = b"POSITION_NOT_FOUND";
/// Caller is not the position owner.
pub const ERR_POSITION_NOT_OWNER: &[u8] = b"POSITION_NOT_OWNER";
/// Position is not open (already closed).
pub const ERR_POSITION_NOT_OPEN: &[u8] = b"POSITION_NOT_OPEN";
/// TP/SL condition not met (for execute_position_tp_sl).
pub const ERR_POSITION_TP_SL_NOT_MET: &[u8] = b"POSITION_TP_SL_NOT_MET";
/// Trailing stop not enabled (trailing_delta_bps == 0).
pub const ERR_TRAILING_DISABLED: &[u8] = b"TRAILING_DISABLED";
/// Trailing stop condition not met (price above trigger level).
pub const ERR_TRAILING_NOT_TRIGGERED: &[u8] = b"TRAILING_NOT_TRIGGERED";

// Batch execution (Shared Execution Gas-Rebate)
/// Batch must contain at least 2 positions.
pub const ERR_BATCH_TOO_FEW: &[u8] = b"BATCH_TOO_FEW";
/// Batch exceeds MAX_BATCH_POSITIONS.
pub const ERR_BATCH_TOO_MANY: &[u8] = b"BATCH_TOO_MANY";
/// All positions in batch must share the same (base, quote) pair.
pub const ERR_BATCH_NOT_SAME_PAIR: &[u8] = b"BATCH_NOT_SAME_PAIR";
/// Invalid OCO pair (order not found or not open).
pub const ERR_OCO_PAIR_INVALID: &[u8] = b"OCO_PAIR_INVALID";
/// Margin: zero amount or insufficient balance.
pub const ERR_MARGIN_ZERO_OR_INSUFFICIENT: &[u8] = b"MARGIN_ZERO_OR_INSUFFICIENT";

// Access Control
/// Caller does not have the required role.
pub const ERR_MISSING_ROLE: &[u8] = b"MISSING_ROLE";
/// Cannot grant role to zero address.
pub const ERR_GRANT_ZERO: &[u8] = b"GRANT_ZERO";

// Timelock
/// Operation not queued or id mismatch.
pub const ERR_TIMELOCK_UNKNOWN_OPERATION: &[u8] = b"TIMELOCK_UNKNOWN_OPERATION";
/// Delay not yet elapsed (execute before ready block).
pub const ERR_TIMELOCK_NOT_READY: &[u8] = b"TIMELOCK_NOT_READY";
/// Operation already executed or cancelled.
pub const ERR_TIMELOCK_ALREADY_EXECUTED: &[u8] = b"TIMELOCK_ALREADY_EXECUTED";

// Growth Engine
pub const ERR_REFERRAL_SELF: &[u8] = b"REFERRAL_SELF";
pub const ERR_REFERRAL_FEE_TOO_HIGH: &[u8] = b"REFERRAL_FEE_TOO_HIGH";
pub const ERR_STAKING_NOT_INIT: &[u8] = b"STAKING_NOT_INIT";
pub const ERR_STAKING_ZERO_AMOUNT: &[u8] = b"STAKING_ZERO_AMOUNT";
pub const ERR_QUEST_ALREADY_CLAIMED: &[u8] = b"QUEST_ALREADY_CLAIMED";
pub const ERR_QUEST_MILESTONE_NOT_MET: &[u8] = b"QUEST_MILESTONE_NOT_MET";

// Intelligence Layer: Copy Trading
pub const ERR_COPY_NOT_SUBSCRIBED: &[u8] = b"COPY_NOT_SUBSCRIBED";
pub const ERR_COPY_LEADER_MISMATCH: &[u8] = b"COPY_LEADER_MISMATCH";
pub const ERR_COPY_SLIPPAGE: &[u8] = b"COPY_SLIPPAGE";
// Signal Marketplace
pub const ERR_SIGNAL_NOT_LISTED: &[u8] = b"SIGNAL_NOT_LISTED";
pub const ERR_SIGNAL_ALREADY_PURCHASED: &[u8] = b"SIGNAL_ALREADY_PURCHASED";
pub const ERR_SIGNAL_INVALID_SIGNATURE: &[u8] = b"SIGNAL_INVALID_SIGNATURE";

// Gasless / EIP-712 permit swap
/// Invalid EIP-712 signature or recovered signer mismatch.
pub const ERR_PERMIT_INVALID_SIGNATURE: &[u8] = b"PERMIT_INVALID_SIGNATURE";
/// Permit swap deadline expired.
pub const ERR_PERMIT_EXPIRED: &[u8] = b"PERMIT_EXPIRED";
/// Permit nonce already used (replay).
pub const ERR_PERMIT_NONCE: &[u8] = b"PERMIT_NONCE";
