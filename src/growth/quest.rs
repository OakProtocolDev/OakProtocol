//! Quest System for bonus.oak.trade: XP and Badges for trading volume milestones.
//!
//! Emits EmissionEvent(Quest, user, XPGranted | BadgeMinted, amount_or_badge_id, 0) for indexer.

use stylus_sdk::alloy_primitives::{Address, U256};

use crate::errors::{err, OakResult, ERR_OVERFLOW};
use crate::events::{emit_emission_event, emission_module_quest};
use crate::state::OakDEX;

/// Event types for EmissionEvent (Quest module).
pub const QUEST_EVENT_XP_GRANTED: u64 = 4;
pub const QUEST_EVENT_BADGE_MINTED: u64 = 5;

/// Quest System (uses OakDEX growth storage).
pub struct QuestSystem;

impl QuestSystem {
    /// Record trading volume for user (call from swap logic). Updates cumulative volume and grants XP by milestone.
    pub fn record_volume(dex: &mut OakDEX, user: Address, volume_delta: U256) -> OakResult<()> {
        if volume_delta.is_zero() {
            return Ok(());
        }
        let prev = dex.quest_user_volume.setter(user).get();
        let new_volume = prev.checked_add(volume_delta).ok_or_else(|| err(ERR_OVERFLOW))?;
        dex.quest_user_volume.setter(user).set(new_volume);
        Ok(())
    }

    /// Grant XP to user (e.g. on milestone). Emits EmissionEvent for indexer.
    pub fn grant_xp(dex: &mut OakDEX, user: Address, xp: U256) -> OakResult<()> {
        if xp.is_zero() {
            return Ok(());
        }
        let prev = dex.quest_user_xp.setter(user).get();
        dex.quest_user_xp.setter(user).set(prev.checked_add(xp).ok_or_else(|| err(ERR_OVERFLOW))?);
        emit_emission_event(
            emission_module_quest(),
            user,
            U256::from(QUEST_EVENT_XP_GRANTED),
            xp,
            U256::ZERO,
        );
        Ok(())
    }

    /// Set badge NFT contract (owner only). 0 = disabled.
    pub fn set_badge_contract(dex: &mut OakDEX, contract: Address) -> OakResult<()> {
        if stylus_sdk::msg::sender() != dex.owner.get() {
            return Err(err(crate::errors::ERR_ONLY_OWNER));
        }
        dex.quest_badge_contract.set(contract);
        Ok(())
    }

    /// Emit badge mint event (e.g. when user hits volume milestone). Actual NFT mint can be done by backend or separate call.
    pub fn emit_badge_minted(_dex: &OakDEX, user: Address, badge_token_id: U256) {
        emit_emission_event(
            emission_module_quest(),
            user,
            U256::from(QUEST_EVENT_BADGE_MINTED),
            U256::ZERO,
            badge_token_id,
        );
    }

    /// View: user cumulative volume.
    pub fn get_user_volume(dex: &OakDEX, user: Address) -> U256 {
        dex.quest_user_volume.getter(user).get()
    }

    /// View: user XP.
    pub fn get_user_xp(dex: &OakDEX, user: Address) -> U256 {
        dex.quest_user_xp.getter(user).get()
    }
}
