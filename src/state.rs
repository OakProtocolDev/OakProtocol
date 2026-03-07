//! Storage layout and core data structures for Oak Protocol.
//!
//! This module defines the on-chain storage for both the core DEX (`OakDEX`)
//! and the GMX-style vault/guardian (`OakSentinel`). The layout is intentionally
//! flat and Stylus-friendly, and includes reserved space for future extensions
//! such as the Oak Bet casino module without requiring a storage migration.

use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
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
        /// Buyback wallet (20% of fees); optional, can be zero.
        StorageAddress buyback_wallet;
        /// Pending owner (two-step transfer, DoD-style).
        StorageAddress pending_owner;
        /// Block number after which pending_owner can accept ownership.
        StorageU256 owner_transfer_after_block;

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
        /// TWAP deviation circuit breaker: last observed price0 (Q112.64) for per-block deviation check.
        StorageU256 last_twap_price0;
        /// TWAP deviation circuit breaker: last observed price1 (Q112.64) for per-block deviation check.
        StorageU256 last_twap_price1;

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

        /// Per-token treasury balance (claimable by owner).
        StorageMap<Address, StorageU256> treasury_balance;
        /// Per-token buyback fund balance (20% of fees; OAK buyback).
        StorageMap<Address, StorageU256> buyback_balance;
        /// Circuit breaker: when true, swaps/commits/add_liquidity disabled; only remove_liquidity and claim_fees allowed.
        StorageBool circuit_breaker_triggered;

        /// --- TP/SL/Limit orders (pro exchange features) ---
        /// Next order ID (incremented on place_order).
        StorageU256 next_order_id;
        /// Order owner (key = order_id_as_address).
        StorageMap<Address, StorageAddress> order_owner;
        /// Token to receive when order executes (key = order_id_as_address).
        StorageMap<Address, StorageAddress> order_token_in;
        /// Token to sell (escrowed in contract until execute/cancel).
        StorageMap<Address, StorageAddress> order_token_out;
        /// Amount of token_out to sell.
        StorageMap<Address, StorageU256> order_amount_out;
        /// Trigger price: for TP/Limit execute when price >= this; for SL when price <= this (reserve_in/reserve_out, same units).
        StorageMap<Address, StorageU256> order_trigger_price;
        /// Order type: 0 = Limit, 1 = TP, 2 = SL.
        StorageMap<Address, StorageU256> order_type;
        /// Status: 0 = Open, 1 = Executed, 2 = Cancelled.
        StorageMap<Address, StorageU256> order_status;
        /// Block number when order was placed.
        StorageMap<Address, StorageU256> order_created_at;

        /// --- Tracked positions (pro terminal: PnL, TP/SL, close) ---
        /// Next position ID (incremented on open_position).
        StorageU256 next_position_id;
        /// Position owner (key = position_id_as_address).
        StorageMap<Address, StorageAddress> position_owner;
        /// Base token (e.g. ETH; sold on close).
        StorageMap<Address, StorageAddress> position_base;
        /// Quote token (e.g. USDC; received on close).
        StorageMap<Address, StorageAddress> position_quote;
        /// Size in base token units (18 decimals).
        StorageMap<Address, StorageU256> position_size;
        /// Entry price: quote per base (18 decimals; reserve_quote/reserve_base at open).
        StorageMap<Address, StorageU256> position_entry_price;
        /// Take-profit price (0 = not set). Execute when market price >= this.
        StorageMap<Address, StorageU256> position_tp_price;
        /// Stop-loss price (0 = not set). Execute when market price <= this.
        StorageMap<Address, StorageU256> position_sl_price;
        /// Trailing stop: delta in basis points (0 = disabled). E.g. 100 = 1%; trigger when price <= peak * (10000 - delta_bps) / 10000.
        StorageMap<Address, StorageU256> position_trailing_delta_bps;
        /// Trailing stop: peak price (quote per base, 18 decimals). Updated by update_trailing_stop when new_price > peak.
        StorageMap<Address, StorageU256> position_trailing_peak_price;
        /// Initial collateral (quote token, 18 decimals) at open. Part of total margin for liquidation price.
        StorageMap<Address, StorageU256> position_initial_collateral;
        /// Additional margin (quote token) added via add_margin. Total margin = initial_collateral + margin_added.
        StorageMap<Address, StorageU256> position_margin_added;
        /// Block when position was opened.
        StorageMap<Address, StorageU256> position_opened_at;
        /// Status: 0 = Open, 1 = Closed.
        StorageMap<Address, StorageU256> position_status;

        /// Total margin held in contract per quote token (sum of initial_collateral + margin_added for open positions).
        StorageMap<Address, StorageU256> position_margin_balance;

        /// OCO: other order ID (key = order_id_as_address). When this order executes, the paired order is cancelled.
        StorageMap<Address, StorageU256> order_oco_pair;

        /// Gasless trading: per-user nonce for EIP-712 PermitSwap (replay protection).
        StorageMap<Address, StorageU256> permit_swap_nonce;

        /// Access Control: role (bytes32) -> account -> has role.
        StorageMap<FixedBytes<32>, StorageMap<Address, StorageBool>> roles;

        /// Timelock: operation_id (keccak256(target,value,data,predecessor,salt)) -> block number after which execute is allowed.
        StorageMap<FixedBytes<32>, StorageU256> timelock_ready_block;

        /// --- Growth Engine: Referral ---
        /// referee => referrer (who referred this address).
        StorageMap<Address, StorageAddress> referral_referrer;
        /// Referral fee in basis points (e.g. 500 = 5% of protocol fee to referrer).
        StorageU256 referral_fee_bps;

        /// --- Growth Engine: StakingRewards (LP tokens ERC-20 / ERC-1155) ---
        /// Reward token address.
        StorageAddress staking_reward_token;
        /// Staking token (LP token address; ERC-20 or ERC-1155).
        StorageAddress staking_token;
        /// Reward per token scaled (accumulated).
        StorageU256 staking_reward_per_token_stored;
        /// Last block number when rewards were updated.
        StorageU256 staking_last_update_block;
        /// Reward rate per block (reward tokens per block).
        StorageU256 staking_reward_rate_per_block;
        /// User: reward per token paid (snapshot at last action).
        StorageMap<Address, StorageU256> staking_user_reward_per_token_paid;
        /// User: pending rewards to claim.
        StorageMap<Address, StorageU256> staking_user_rewards;
        /// User: staked balance (LP tokens).
        StorageMap<Address, StorageU256> staking_user_balance;
        /// Total staked supply (for reward math).
        StorageU256 staking_total_staked;

        /// --- Growth Engine: Quest (XP / Badges for volume) ---
        /// User cumulative trading volume (for milestones).
        StorageMap<Address, StorageU256> quest_user_volume;
        /// User XP (experience points).
        StorageMap<Address, StorageU256> quest_user_xp;
        /// Badge NFT contract (optional; 0 = no NFT).
        StorageAddress quest_badge_contract;

        /// --- Intelligence Layer: Copy Trading ---
        /// Follower => leader they copy (0 = no subscription).
        StorageMap<Address, StorageAddress> copy_trading_leader;
        /// Follower => max slippage in bps (e.g. 50 = 0.5%).
        StorageMap<Address, StorageU256> copy_trading_slippage_bps;
        /// Follower => amount ratio in bps (e.g. 5000 = 50% of leader amount).
        StorageMap<Address, StorageU256> copy_trading_amount_ratio_bps;

        /// --- Intelligence Layer: Signal Marketplace ---
        /// seller => (signal_id_bytes32 => price in protocol tokens). 0 = delisted.
        StorageMap<Address, StorageMap<FixedBytes<32>, StorageU256>> signal_price;
        /// buyer => (listing_hash => true if purchased). Content key delivered off-chain after payment.
        StorageMap<Address, StorageMap<FixedBytes<32>, StorageBool>> signal_purchased;
        /// Per-seller nonce for EIP-712 SignalListing replay protection.
        StorageMap<Address, StorageU256> signal_nonce;

        /// Reserved space for future protocol extensions (e.g. Oak Bet).
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

        /// Last batch: packed (block_number << 128) | total_amount_in for analytics / gas-rebate rating.
        /// @dev Single slot to minimize storage; decode with (val >> 128) and (val & ((1<<128)-1)).
        StorageU256 vault_last_batch_packed;

        /// Reserved space for Oak Bet / future vault features.
        StorageU256 sentinel_reserved0;
        StorageU256 sentinel_reserved1;
        StorageU256 sentinel_reserved2;
        StorageU256 sentinel_reserved3;
    }
}

