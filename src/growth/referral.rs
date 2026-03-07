//! Referral Engine: referrer => referee mapping, % of fees to referrer on each swap.
//! Emits EmissionEvent(Referral, referrer, ReferralFee, amount, 0) for indexer.

use stylus_sdk::alloy_primitives::{Address, U256};

use crate::constants::REFERRAL_FEE_BPS_MAX;
use crate::errors::{err, OakResult, ERR_DIVISION_BY_ZERO, ERR_OVERFLOW, ERR_REFERRAL_FEE_TOO_HIGH, ERR_REFERRAL_SELF};
use crate::events::{emit_emission_event, emission_module_referral};
use crate::state::OakDEX;
use crate::token::safe_transfer;

/// Event type: referral fee paid.
pub const REFERRAL_EVENT_FEE: u64 = 3;

/// Referral Engine (uses OakDEX growth storage).
pub struct ReferralEngine;

impl ReferralEngine {
    /// Set referrer for msg.sender (referee). Cannot self-refer.
    pub fn set_referrer(dex: &mut OakDEX, referrer: Address) -> OakResult<()> {
        let referee = stylus_sdk::msg::sender();
        if referrer == referee {
            return Err(err(ERR_REFERRAL_SELF));
        }
        if referrer == Address::ZERO {
            dex.referral_referrer.setter(referee).set(Address::ZERO);
            return Ok(());
        }
        dex.referral_referrer.setter(referee).set(referrer);
        Ok(())
    }

    /// Get referrer for a referee.
    pub fn get_referrer(dex: &OakDEX, referee: Address) -> Address {
        dex.referral_referrer.getter(referee).get()
    }

    /// Owner sets referral fee in basis points (e.g. 500 = 5% of protocol fee).
    pub fn set_referral_fee_bps(dex: &mut OakDEX, bps: U256) -> OakResult<()> {
        let owner = dex.owner.get();
        if stylus_sdk::msg::sender() != owner {
            return Err(err(crate::errors::ERR_ONLY_OWNER));
        }
        if bps > U256::from(REFERRAL_FEE_BPS_MAX) {
            return Err(err(ERR_REFERRAL_FEE_TOO_HIGH));
        }
        dex.referral_fee_bps.set(bps);
        Ok(())
    }

    /// Distribute referral share of fee to referrer (call from swap logic).
    pub fn distribute_referral_fee(
        dex: &mut OakDEX,
        token: Address,
        fee_amount: U256,
        referee: Address,
    ) -> OakResult<U256> {
        let referrer = dex.referral_referrer.getter(referee).get();
        if referrer == Address::ZERO || fee_amount.is_zero() {
            return Ok(U256::ZERO);
        }
        let bps = dex.referral_fee_bps.get();
        if bps.is_zero() {
            return Ok(U256::ZERO);
        }
        let referral_amount = fee_amount
            .checked_mul(bps)
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_div(U256::from(10_000u64))
            .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
        if referral_amount.is_zero() {
            return Ok(U256::ZERO);
        }
        safe_transfer(token, referrer, referral_amount)?;
        emit_emission_event(
            emission_module_referral(),
            referrer,
            U256::from(REFERRAL_EVENT_FEE),
            referral_amount,
            U256::ZERO,
        );
        Ok(referral_amount)
    }
}
