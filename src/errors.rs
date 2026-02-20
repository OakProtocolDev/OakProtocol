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

