//! StakingRewards: rewards for LP tokens (ERC-20 or ERC-1155).
//!
//! Users stake LP tokens; contract distributes reward token per block.
//! Emits EmissionEvent for indexer (Staked, Unstaked, RewardClaimed).

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    block,
};

use crate::errors::{err, OakResult, ERR_DIVISION_BY_ZERO, ERR_OVERFLOW, ERR_STAKING_NOT_INIT, ERR_STAKING_ZERO_AMOUNT};
use crate::events::{emit_emission_event, emission_module_staking};
use crate::state::OakDEX;
use crate::token::{safe_transfer, safe_transfer_from};

/// Event types for EmissionEvent (Staking module).
pub const STAKING_EVENT_REWARD_CLAIMED: u64 = 0;
pub const STAKING_EVENT_STAKED: u64 = 1;
pub const STAKING_EVENT_UNSTAKED: u64 = 2;

/// 1e18 for reward scaling.
fn precision() -> U256 {
    U256::from(1_000_000_000_000_000_000u64)
}

/// Update reward_per_token and last_update; then update user's pending rewards.
fn _update_rewards(dex: &mut OakDEX, user: Address) -> OakResult<()> {
    let reward_token = dex.staking_reward_token.get();
    let staking_token = dex.staking_token.get();
    if reward_token == Address::ZERO || staking_token == Address::ZERO {
        return Err(err(ERR_STAKING_NOT_INIT));
    }
    let total = dex.staking_total_staked.get();
    let last = dex.staking_last_update_block.get();
    let block_num = U256::from(block::number());
    let rate = dex.staking_reward_rate_per_block.get();
    let mut reward_per_token = dex.staking_reward_per_token_stored.get();
    if !total.is_zero() && block_num > last {
        let blocks = block_num.checked_sub(last).ok_or_else(|| err(ERR_OVERFLOW))?;
        let reward_delta = blocks
            .checked_mul(rate)
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_mul(precision())
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_div(total)
            .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
        reward_per_token = reward_per_token.checked_add(reward_delta).ok_or_else(|| err(ERR_OVERFLOW))?;
        dex.staking_reward_per_token_stored.set(reward_per_token);
    }
    dex.staking_last_update_block.set(block_num);
    let user_balance = dex.staking_user_balance.setter(user).get();
    let user_paid = dex.staking_user_reward_per_token_paid.setter(user).get();
    let pending = if user_balance.is_zero() {
        U256::ZERO
    } else {
        reward_per_token
            .checked_sub(user_paid)
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_mul(user_balance)
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_div(precision())
            .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?
    };
    dex.staking_user_reward_per_token_paid.setter(user).set(reward_per_token);
    let current = dex.staking_user_rewards.setter(user).get();
    dex.staking_user_rewards.setter(user).set(current.checked_add(pending).ok_or_else(|| err(ERR_OVERFLOW))?);
    Ok(())
}

/// StakingRewards logic (uses OakDEX growth storage).
pub struct StakingRewards;

impl StakingRewards {
    /// Initialize staking (owner only). LP token can be ERC-20 or ERC-1155 (use token_id 0 for ERC-20).
    pub fn init(
        dex: &mut OakDEX,
        reward_token: Address,
        staking_token: Address,
        reward_rate_per_block: U256,
    ) -> OakResult<()> {
        if stylus_sdk::msg::sender() != dex.owner.get() {
            return Err(err(crate::errors::ERR_ONLY_OWNER));
        }
        dex.staking_reward_token.set(reward_token);
        dex.staking_token.set(staking_token);
        dex.staking_reward_rate_per_block.set(reward_rate_per_block);
        dex.staking_last_update_block.set(U256::from(block::number()));
        Ok(())
    }

    /// Stake LP tokens (ERC-20). For ERC-1155, use token_id 0 or extend with token_id map.
    /// Reentrancy: guard held; CEI: state updates before transfer.
    pub fn stake(dex: &mut OakDEX, amount: U256) -> OakResult<()> {
        if amount.is_zero() {
            return Err(err(ERR_STAKING_ZERO_AMOUNT));
        }
        let sender = stylus_sdk::msg::sender();
        let staking_token = dex.staking_token.get();
        if staking_token == Address::ZERO {
            return Err(err(ERR_STAKING_NOT_INIT));
        }
        crate::logic::lock_reentrancy_guard(dex)?;
        if let Err(e) = _update_rewards(dex, sender) {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(e);
        }
        let prev = dex.staking_user_balance.setter(sender).get();
        let new_balance = prev.checked_add(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
        let total = dex.staking_total_staked.get();
        let new_total = total.checked_add(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
        dex.staking_user_balance.setter(sender).set(new_balance);
        dex.staking_total_staked.set(new_total);
        let contract = stylus_sdk::contract::address();
        if let Err(e) = safe_transfer_from(staking_token, sender, contract, amount) {
            dex.staking_user_balance.setter(sender).set(prev);
            dex.staking_total_staked.set(total);
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(e);
        }
        emit_emission_event(
            emission_module_staking(),
            sender,
            U256::from(STAKING_EVENT_STAKED),
            amount,
            U256::ZERO,
        );
        crate::logic::unlock_reentrancy_guard(dex);
        Ok(())
    }

    /// Unstake LP tokens. Reentrancy: guard held; CEI: state then transfer.
    pub fn unstake(dex: &mut OakDEX, amount: U256) -> OakResult<()> {
        if amount.is_zero() {
            return Err(err(ERR_STAKING_ZERO_AMOUNT));
        }
        let sender = stylus_sdk::msg::sender();
        crate::logic::lock_reentrancy_guard(dex)?;
        if let Err(e) = _update_rewards(dex, sender) {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(e);
        }
        let balance = dex.staking_user_balance.setter(sender).get();
        if balance < amount {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(err(crate::errors::ERR_INSUFFICIENT_BALANCE));
        }
        let new_balance = balance.checked_sub(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
        let total = dex.staking_total_staked.get();
        let new_total = total.checked_sub(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
        dex.staking_user_balance.setter(sender).set(new_balance);
        dex.staking_total_staked.set(new_total);
        let staking_token = dex.staking_token.get();
        if let Err(e) = safe_transfer(staking_token, sender, amount) {
            dex.staking_user_balance.setter(sender).set(balance);
            dex.staking_total_staked.set(total);
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(e);
        }
        emit_emission_event(
            emission_module_staking(),
            sender,
            U256::from(STAKING_EVENT_UNSTAKED),
            amount,
            U256::ZERO,
        );
        crate::logic::unlock_reentrancy_guard(dex);
        Ok(())
    }

    /// Claim pending reward tokens. Reentrancy: guard held; CEI: state then transfer.
    pub fn claim_rewards(dex: &mut OakDEX) -> OakResult<U256> {
        let sender = stylus_sdk::msg::sender();
        crate::logic::lock_reentrancy_guard(dex)?;
        if let Err(e) = _update_rewards(dex, sender) {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(e);
        }
        let amount = dex.staking_user_rewards.setter(sender).get();
        dex.staking_user_rewards.setter(sender).set(U256::ZERO);
        if !amount.is_zero() {
            let reward_token = dex.staking_reward_token.get();
            if let Err(e) = safe_transfer(reward_token, sender, amount) {
                dex.staking_user_rewards.setter(sender).set(amount);
                crate::logic::unlock_reentrancy_guard(dex);
                return Err(e);
            }
            emit_emission_event(
                emission_module_staking(),
                sender,
                U256::from(STAKING_EVENT_REWARD_CLAIMED),
                amount,
                U256::ZERO,
            );
        }
        crate::logic::unlock_reentrancy_guard(dex);
        Ok(amount)
    }

    /// View: pending rewards for user.
    pub fn pending_rewards(dex: &mut OakDEX, user: Address) -> OakResult<U256> {
        let reward_token = dex.staking_reward_token.get();
        if reward_token == Address::ZERO {
            return Ok(U256::ZERO);
        }
        let total = dex.staking_total_staked.get();
        let last = dex.staking_last_update_block.get();
        let block_num = U256::from(block::number());
        let rate = dex.staking_reward_rate_per_block.get();
        let mut reward_per_token = dex.staking_reward_per_token_stored.get();
        if !total.is_zero() && block_num > last {
            let blocks = block_num.checked_sub(last).ok_or_else(|| err(ERR_OVERFLOW))?;
            let reward_delta = blocks
                .checked_mul(rate)
                .ok_or_else(|| err(ERR_OVERFLOW))?
                .checked_mul(precision())
                .ok_or_else(|| err(ERR_OVERFLOW))?
                .checked_div(total)
                .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
            reward_per_token = reward_per_token.checked_add(reward_delta).ok_or_else(|| err(ERR_OVERFLOW))?;
        }
        let user_balance = dex.staking_user_balance.setter(user).get();
        let user_paid = dex.staking_user_reward_per_token_paid.setter(user).get();
        let pending = if user_balance.is_zero() {
            U256::ZERO
        } else {
            reward_per_token
                .checked_sub(user_paid)
                .unwrap_or(U256::ZERO)
                .checked_mul(user_balance)
                .unwrap_or(U256::ZERO)
                .checked_div(precision())
                .unwrap_or(U256::ZERO)
        };
        let stored = dex.staking_user_rewards.setter(user).get();
        Ok(stored.checked_add(pending).unwrap_or(stored))
    }
}
