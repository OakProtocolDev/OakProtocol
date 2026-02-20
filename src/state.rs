//! Storage layout and core data structures for Oak Protocol.

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    prelude::*,
    storage::{StorageAddress, StorageBool, StorageMap, StorageU256},
};

/// Commitment structure for the commit‑reveal mechanism.
///
/// @notice Describes a user's pending swap commitment.
/// @dev The live storage representation is split across several `StorageMap`s
///      for gas efficiency on Stylus. This plain struct is used for in-memory
///      reasoning and documentation.
#[derive(Clone, Copy)]
pub struct Commitment {
    /// Hash of the commitment (keccak256 of reveal data).
    pub hash: U256,
    /// Block number when the commitment was created.
    pub timestamp: U256,
    /// Whether the commitment has been activated and not yet revealed.
    pub activated: bool,
}

/// Main storage structure for Oak Protocol.
///
/// @notice Holds all on-chain state for the DEX.
/// @dev This layout is intentionally flat and Stylus‑friendly. Higher‑level
///      abstractions live in `logic`.
sol_storage! {
    pub struct OakDEX {
        /// Reserve of token0 in the liquidity pool.
        StorageU256 reserves0;
        /// Reserve of token1 in the liquidity pool.
        StorageU256 reserves1;

        /// Minimum liquidity that must remain in the pool (to prevent draining).
        StorageU256 min_liquidity;

        /// Total protocol fee in basis points (e.g., 30 = 0.3%).
        StorageU256 protocol_fee_bps;

        /// Owner address (can change protocol settings).
        StorageAddress owner;

        /// Treasury address receiving a share of fees.
        StorageAddress treasury;

        /// Accrued fees owed to the treasury in token0 units.
        StorageU256 accrued_treasury_fees_token0;

        /// Accrued fees owed to LPs in token0 units (accounting only).
        StorageU256 accrued_lp_fees_token0;

        /// Total trading volume for token0 (for analytics).
        StorageU256 total_volume_token0;
        /// Total trading volume for token1 (for analytics).
        StorageU256 total_volume_token1;

        /// Emergency pause switch (if true, swaps are frozen).
        StorageBool paused;

        /// Mapping from user address to commitment hash (U256-encoded bytes32).
        StorageMap<Address, StorageU256> commitment_hashes;
        /// Mapping from user address to commitment block timestamp.
        StorageMap<Address, StorageU256> commitment_timestamps;
        /// Mapping from user address to commitment activation status.
        StorageMap<Address, StorageBool> commitment_activated;

        /// Global re-entrancy guard (1 = locked, 0 = unlocked).
        /// @dev Prevents recursive calls to critical functions.
        StorageBool locked;
    }
}

