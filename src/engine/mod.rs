//! Core Engine: modular separation of swap logic and order execution.
//!
//! - **Swap core**: Uniswap-style/CPMM single-swap math and execution (storage-minimal).
//! - **Execution strategy**: Trait for Atomic vs Commit-Reveal; chosen per-call or via storage.
//! - **Order execution**: Uses swap core + strategy; supports batching.
//! - **Emergency**: TWAP deviation circuit breaker (check_price_deviation).

pub mod strategy;
pub mod swap_core;
pub mod execution;
pub mod emergency;

pub use strategy::{ExecutionMode, ExecutionStrategy, Atomic, CommitReveal};
pub use swap_core::SwapCore;
pub use execution::OrderExecution;
pub use emergency::check_price_deviation;