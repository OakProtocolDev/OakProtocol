//! Swap core: single-swap logic only (no order/position state).
//!
//! This module defines the interface and types for the Uniswap-style pool swap.
//! Actual implementation lives in `crate::logic` (process_swap_from_to_with_fee)
//! to avoid circular deps; this layer is the abstraction for the engine.

use stylus_sdk::alloy_primitives::{Address, U256};

use crate::errors::OakResult;
use crate::state::OakDEX;

/// Parameters for a single pool swap (minimal; one logical slot when passed).
#[derive(Clone, Copy, Debug)]
pub struct SwapParams {
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub min_amount_out: U256,
    /// Fee in basis points; None = use protocol default.
    pub fee_bps_override: Option<U256>,
}

/// Result of a single swap.
#[derive(Clone, Copy, Debug)]
pub struct SwapResult {
    pub amount_out: U256,
}

/// Swap core interface: one swap in the pool.
///
/// Implementations are in `logic.rs` (process_swap_from_to / process_swap_from_to_with_fee).
/// This trait allows the execution layer to call swap without depending on order/position logic.
pub trait SwapCore {
    /// Execute one swap: from -> to (contract or user). Returns amount_out.
    fn execute_swap(
        dex: &mut OakDEX,
        from: Address,
        to: Address,
        params: SwapParams,
    ) -> OakResult<SwapResult>;
}

/// Default swap core delegates to logic module.
pub struct DexSwapCore;

impl SwapCore for DexSwapCore {
    fn execute_swap(
        dex: &mut OakDEX,
        from: Address,
        to: Address,
        params: SwapParams,
    ) -> OakResult<SwapResult> {
        let fee_bps = params.fee_bps_override.unwrap_or_else(|| dex.protocol_fee_bps.get());
        let amount_out = crate::logic::process_swap_from_to_with_fee(
            dex,
            from,
            to,
            params.token_in,
            params.token_out,
            params.amount_in,
            params.min_amount_out,
            fee_bps,
        )?;
        Ok(SwapResult { amount_out })
    }
}