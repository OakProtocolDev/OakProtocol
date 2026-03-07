//! Order execution layer: uses SwapCore and ExecutionStrategy.
//!
//! Decouples "how to execute" (Atomic vs Commit-Reveal) from "what to execute"
//! (single swap vs batch). Storage reads for strategy mode kept to a minimum.

use stylus_sdk::alloy_primitives::Address;

use crate::errors::OakResult;
use crate::state::OakDEX;

use super::strategy::{ExecutionMode, ExecutionStrategy};
use super::swap_core::{SwapCore, SwapParams, SwapResult};

/// Execution mode read from storage (one slot). 0 = Atomic, 1 = CommitReveal.
/// Caller must ensure dex has a slot for execution_mode if configurable at runtime.
pub fn execution_mode_from_storage(_dex: &OakDEX) -> ExecutionMode {
    // If we had a slot: _dex.execution_mode.get() -> ExecutionMode::from_u256(val)
    // Default: Atomic (no extra storage read).
    ExecutionMode::Atomic
}

/// Execute a single swap using the chosen strategy.
///
/// - Atomic: call SwapCore once.
/// - CommitReveal: caller must have already committed; this performs reveal (swap).
///   (Full commit/reveal flow remains in logic.rs; this is the execution abstraction.)
pub fn execute_swap_with_strategy<S: ExecutionStrategy>(
    dex: &mut OakDEX,
    from: Address,
    to: Address,
    params: SwapParams,
) -> OakResult<SwapResult> {
    // Atomic: direct swap. CommitReveal: reveal step (commit is separate entrypoint in logic).
    DexSwapCore::execute_swap(dex, from, to, params)
}

/// Type alias for the concrete swap core used by the DEX.
pub type DexSwapCore = super::swap_core::DexSwapCore;

/// OrderExecution: high-level interface for executing one or many orders/positions.
/// Batch execution aggregates and calls SwapCore once (see vault batch).
pub struct OrderExecution;