//! Intelligence Layer: Copy Trading and Signal Marketplace.
//!
//! - Copy Trading: follower subscribes to leader with slippage and amount_ratio; revocable anytime.
//! - Signal Marketplace: EIP-712 signed listings; payment in protocol tokens; encrypted content key off-chain.

pub mod copy_trading;
pub mod signal_marketplace;

pub use copy_trading::CopyTrading;
pub use signal_marketplace::SignalMarketplace;