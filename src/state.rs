//! Storage layout and core data structures for Oak Protocol.
//!
//! This module defines the on-chain storage for both the core DEX (`OakDEX`)
//! and the GMX-style vault/guardian (`OakSentinel`). The layout is intentionally
//! flat and Stylus-friendly, and includes reserved space for future extensions
//! such as the Oak Bet casino module without requiring a storage migration.

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

sol_storage! {
    /// Per‑pair pool data for multi‑pool support.
    pub struct PoolData {
        /// Reserve of token0 in the pool (canonical ordering).
        StorageU256 reserve0;
        /// Reserve of token1 in the pool (canonical ordering).
        StorageU256 reserve1;
        /// Total LP token supply for this pool.
        StorageU256 lp_total_supply;
        /// Per‑address LP balances for this pool.
        StorageMap<Address, StorageU256> lp_balances;
        /// Initialization flag to distinguish configured pools.
        StorageBool initialized;
    }

    #[cfg_attr(any(test, not(target_arch = "wasm32")), allow(unused_doc_comments))]
    /// Main storage structure for Oak Protocol.
    ///
    /// @notice Holds all on-chain state for the DEX.
    /// @dev This layout is intentionally flat and Stylus‑friendly. Higher‑level
    ///      abstractions live in `logic`. Reserved fields at the end allow
    ///      backwards‑compatible extension for future features (e.g. Oak Bet).
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

        /// Treasury address receiving a share of fees (admin wallet).
        StorageAddress treasury;

        /// Accrued fees owed to the treasury in token0 units.
        StorageU256 accrued_treasury_fees_token0;

        /// Accrued fees owed to LPs in token0 units (accounting only).
        StorageU256 accrued_lp_fees_token0;

        /// Total trading volume for token0 (for analytics).
        StorageU256 total_volume_token0;
        /// Total trading volume for token1 (for analytics).
        StorageU256 total_volume_token1;

        /// TWAP Oracle: cumulative price of token0 in Q112.64 format (price0 = reserve1/reserve0).
        StorageU256 price0_cumulative_last;
        /// TWAP Oracle: cumulative price of token1 in Q112.64 format (price1 = reserve0/reserve1).
        StorageU256 price1_cumulative_last;
        /// TWAP Oracle: block number (or timestamp) of last oracle update.
        /// @dev On L2 we use block number as time index for gas efficiency.
        StorageU256 block_timestamp_last;

        /// Gas-rebate reserve: portion of protocol fee tracked for future gas rebates (placeholder).
        StorageU256 accrued_gas_rebate_token0;

        /// Emergency pause switch (if true, swaps are frozen).
        StorageBool paused;

        /// Mapping from user address to commitment hash (U256-encoded bytes32).
        StorageMap<Address, StorageU256> commitment_hashes;
        /// Mapping from user address to commitment block timestamp.
        StorageMap<Address, StorageU256> commitment_timestamps;
        /// Mapping from user address to commitment activation status.
        StorageMap<Address, StorageBool> commitment_activated;

        /// Global re-entrancy guard (true = locked, false = unlocked).
        /// @dev Prevents recursive calls to critical functions.
        StorageBool locked;

        /// Mapping from canonical token0 to inner map of token1 -> pool data.
        /// @dev token0 and token1 are always sorted (token0 < token1) to avoid duplicates.
        StorageMap<Address, StorageMap<Address, PoolData>> pools;
        /// Reserved space for future protocol extensions (e.g. Oak Bet).
        /// @dev These fields deliberately sit at the end of the layout so
        ///      that new features can reuse them without shifting storage.
        StorageU256 reserved2;
        StorageU256 reserved3;
    }

    /// Guardian / vault state used by the GMX-style leverage module.
    ///
    /// @notice This struct backs the internal `vault` module and is intended
    ///         to be driven by a higher-level controller (OakSentinel).
    /// @dev Similar to `OakDEX`, we keep the layout flat and leave reserved
    ///      slots at the end for future leverage / casino extensions.
    pub struct OakSentinel {
        /// Owner address for vault/admin operations.
        StorageAddress owner;
        /// Emergency pause for vault operations.
        StorageBool paused;

        /// Total pool amount per token (GMX: poolAmounts).
        StorageMap<Address, StorageU256> vault_pool_amount;
        /// Fee reserves per token (GMX: feeReserves).
        StorageMap<Address, StorageU256> vault_fee_reserves;
        /// Reserved token amounts backing open leverage (GMX: reservedAmounts).
        StorageMap<Address, StorageU256> vault_reserved_amount;
        /// Guaranteed USD exposure per collateral token (GMX: guaranteedUsd).
        StorageMap<Address, StorageU256> vault_guaranteed_usd;
        /// Buffer amounts per token to protect against pool underflow.
        StorageMap<Address, StorageU256> vault_buffer_amount;

        /// Global short and long exposure in USD.
        StorageU256 vault_global_short_size_usd;
        StorageU256 vault_global_long_size_usd;

        /// Reserved space for Oak Bet / future vault features.
        StorageU256 sentinel_reserved0;
        StorageU256 sentinel_reserved1;
        StorageU256 sentinel_reserved2;
        StorageU256 sentinel_reserved3;
    }
}

