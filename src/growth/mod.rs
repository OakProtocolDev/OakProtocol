//! Growth Engine: StakingRewards, Referral, Quest.
//!
//! Each module emits EmissionEvent for indexer (personal cabinet on bonus.oak.trade).

pub mod staking_rewards;
pub mod referral;
pub mod quest;

pub use staking_rewards::StakingRewards;
pub use referral::ReferralEngine;
pub use quest::QuestSystem;