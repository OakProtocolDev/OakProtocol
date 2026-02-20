//! Core protocol constants for Oak Protocol.

use stylus_sdk::alloy_primitives::U256;

/// Default total trading fee in basis points (0.3%).
pub const DEFAULT_FEE_BPS: u64 = 30;

/// Basis points denominator (10000 = 100%).
pub const FEE_DENOMINATOR: u64 = 10_000;

/// Minimum total liquidity to keep the pool from being drained.
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

/// Minimum number of L1/L2 blocks between commit and reveal.
pub const COMMIT_REVEAL_DELAY: u64 = 5;

/// Maximum number of blocks a commitment can remain un-revealed before expiration.
/// @dev Prevents storage bloat from abandoned commitments.
pub const MAX_COMMITMENT_AGE: u64 = 1_000_000; // ~277 hours at 1 block/second

/// Maximum configurable fee in basis points (10%).
pub const MAX_FEE_BPS: u64 = 1_000;

/// Treasury share of the total fee in basis points (0.12%).
pub const TREASURY_FEE_BPS: u64 = 12;

/// LP share of the total fee in basis points (0.18%).
pub const LP_FEE_BPS: u64 = DEFAULT_FEE_BPS - TREASURY_FEE_BPS;

/// Gas-rebate share of total fee in basis points (placeholder for future gas rebates).
/// @dev A small portion of protocol fee is tracked in accrued_gas_rebate_token0.
pub const GAS_REBATE_BPS: u64 = 5;

/// Q112.64 fixed-point multiplier for TWAP cumulative prices (2^112).
pub const Q112: u128 = 1u128 << 112;

/// Returns 2^112 as U256 for TWAP cumulative price math.
#[inline]
pub fn q112_u256() -> U256 {
    U256::from(1u64).wrapping_shl(112)
}

/// Convenience helpers for working with `U256`-based math.
pub fn as_u256(value: u64) -> U256 {
    U256::from(value)
}

