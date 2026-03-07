//! Execution strategy: Atomic (one tx) vs Commit-Reveal (two-step MEV protection).
//!
//! Used by the execution layer to decide whether to perform a direct swap
//! or require commit then reveal. Storage slot can hold ExecutionMode (0 = Atomic, 1 = CommitReveal).

use stylus_sdk::alloy_primitives::U256;

/// Execution mode identifier (stored in one slot when configurable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ExecutionMode {
    /// Single-tx swap with slippage/deadline; no commit step.
    Atomic = 0,
    /// Two-step: commit hash, then reveal after delay.
    CommitReveal = 1,
}

impl ExecutionMode {
    pub fn from_u8(x: u8) -> Self {
        match x {
            1 => ExecutionMode::CommitReveal,
            _ => ExecutionMode::Atomic,
        }
    }
    pub fn from_u256(x: U256) -> Self {
        if x == U256::from(1u64) {
            ExecutionMode::CommitReveal
        } else {
            ExecutionMode::Atomic
        }
    }
    pub fn as_u256(self) -> U256 {
        U256::from(self as u8)
    }
}

/// Strategy for executing a swap: Atomic or Commit-Reveal.
pub trait ExecutionStrategy {
    fn mode() -> ExecutionMode;
    /// True if user must call commit before reveal.
    fn requires_commit() -> bool;
}

/// Default: one transaction, direct swap (EVM-style).
pub struct Atomic;

impl ExecutionStrategy for Atomic {
    fn mode() -> ExecutionMode {
        ExecutionMode::Atomic
    }
    fn requires_commit() -> bool {
        false
    }
}

/// MEV protection: commit hash first, reveal after block delay.
pub struct CommitReveal;

impl ExecutionStrategy for CommitReveal {
    fn mode() -> ExecutionMode {
        ExecutionMode::CommitReveal
    }
    fn requires_commit() -> bool {
        true
    }
}