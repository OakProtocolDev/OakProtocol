//! Copy Trading: wallet A subscribes to trades of wallet B with slippage and amount_ratio.
//! Subscriptions are revocable at any time by the follower.

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    block,
};

use crate::constants::{COPY_TRADING_AMOUNT_RATIO_BPS_MAX, COPY_TRADING_SLIPPAGE_BPS_MAX};
use crate::errors::{err, OakResult, ERR_COPY_LEADER_MISMATCH, ERR_COPY_NOT_SUBSCRIBED, ERR_COPY_SLIPPAGE, ERR_DIVISION_BY_ZERO, ERR_OVERFLOW};
use crate::events::{emit_copy_subscription, emit_copy_subscription_revoked, emit_copy_trade_executed};
use crate::state::OakDEX;

const BPS: u64 = 10_000;

/// Copy Trading: subscribe, revoke, execute copy (for backend/relayer).
pub struct CopyTrading;

impl CopyTrading {
    /// Subscribe msg.sender (follower) to copy `leader` with max slippage and amount ratio.
    /// amount_ratio_bps: e.g. 5000 = 50% of leader's amount per trade.
    /// Overwrites any existing subscription (one leader per follower).
    pub fn subscribe(
        dex: &mut OakDEX,
        leader: Address,
        slippage_bps: U256,
        amount_ratio_bps: U256,
    ) -> OakResult<()> {
        let follower = stylus_sdk::msg::sender();
        if leader == Address::ZERO || leader == follower {
            return Err(err(crate::errors::ERR_INVALID_ADDRESS));
        }
        if slippage_bps > U256::from(COPY_TRADING_SLIPPAGE_BPS_MAX) {
            return Err(err(ERR_COPY_SLIPPAGE));
        }
        if amount_ratio_bps.is_zero() || amount_ratio_bps > U256::from(COPY_TRADING_AMOUNT_RATIO_BPS_MAX) {
            return Err(err(crate::errors::ERR_INVALID_ORDER_TYPE));
        }
        dex.copy_trading_leader.setter(follower).set(leader);
        dex.copy_trading_slippage_bps.setter(follower).set(slippage_bps);
        dex.copy_trading_amount_ratio_bps.setter(follower).set(amount_ratio_bps);
        emit_copy_subscription(follower, leader, slippage_bps, amount_ratio_bps);
        Ok(())
    }

    /// Revoke subscription (follower = msg.sender). Safe to call anytime; no-op if not subscribed.
    pub fn unsubscribe(dex: &mut OakDEX) -> OakResult<()> {
        let follower = stylus_sdk::msg::sender();
        let leader = dex.copy_trading_leader.getter(follower).get();
        if leader == Address::ZERO {
            return Ok(());
        }
        dex.copy_trading_leader.setter(follower).set(Address::ZERO);
        dex.copy_trading_slippage_bps.setter(follower).set(U256::ZERO);
        dex.copy_trading_amount_ratio_bps.setter(follower).set(U256::ZERO);
        emit_copy_subscription_revoked(follower, leader);
        Ok(())
    }

    /// Execute a copy trade for `follower` (same trade as leader: token_in -> token_out, leader_amount_in).
    /// Callable by anyone (backend/relayer). Uses follower's balance and subscription params.
    /// Reentrancy: guard held for full swap (external token transfers).
    pub fn execute_copy_trade(
        dex: &mut OakDEX,
        follower: Address,
        leader: Address,
        token_in: Address,
        token_out: Address,
        leader_amount_in: U256,
        deadline: U256,
    ) -> OakResult<U256> {
        crate::logic::lock_reentrancy_guard(dex)?;
        let stored_leader = dex.copy_trading_leader.getter(follower).get();
        if stored_leader == Address::ZERO {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(err(ERR_COPY_NOT_SUBSCRIBED));
        }
        if stored_leader != leader {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(err(ERR_COPY_LEADER_MISMATCH));
        }
        if U256::from(block::number()) > deadline {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(err(crate::errors::ERR_EXPIRED));
        }
        let amount_ratio_bps = dex.copy_trading_amount_ratio_bps.getter(follower).get();
        let slippage_bps = dex.copy_trading_slippage_bps.getter(follower).get();
        let amount_in = leader_amount_in
            .checked_mul(amount_ratio_bps)
            .ok_or_else(|| err(ERR_OVERFLOW))?
            .checked_div(U256::from(BPS))
            .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
        if amount_in.is_zero() {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(err(crate::errors::ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        let (token0, token1) = if token_in < token_out {
            (token_in, token_out)
        } else {
            (token_out, token_in)
        };
        let outer = dex.pools.getter(token0);
        let pool = outer.getter(token1);
        if !pool.initialized.get() {
            crate::logic::unlock_reentrancy_guard(dex);
            return Err(err(crate::errors::ERR_INVALID_TOKEN));
        }
        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();
        let (reserve_in, reserve_out) = if token_in == token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };
        let fee_bps = dex.protocol_fee_bps.get(); // single storage read for amount_out and process_swap
        let expected_out = match crate::logic::get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps) {
            Ok(x) => x,
            Err(e) => {
                crate::logic::unlock_reentrancy_guard(dex);
                return Err(e);
            }
        };
        let slippage_deduction = match U256::from(BPS).checked_sub(slippage_bps) {
            Some(x) => x,
            None => {
                crate::logic::unlock_reentrancy_guard(dex);
                return Err(err(ERR_OVERFLOW));
            }
        };
        let min_out = match expected_out.checked_mul(slippage_deduction).and_then(|n| n.checked_div(U256::from(BPS))) {
            Some(x) => x,
            None => {
                crate::logic::unlock_reentrancy_guard(dex);
                return Err(err(ERR_OVERFLOW));
            }
        };
        let result = crate::logic::process_swap_from_to_with_fee(
            dex,
            follower,
            follower,
            token_in,
            token_out,
            amount_in,
            min_out,
            fee_bps,
        );
        match result {
            Ok(amount_out) => {
                emit_copy_trade_executed(follower, leader, amount_in, amount_out);
                crate::logic::unlock_reentrancy_guard(dex);
                Ok(amount_out)
            }
            Err(e) => {
                crate::logic::unlock_reentrancy_guard(dex);
                Err(e)
            }
        }
    }

    /// View: get subscription for follower (leader, slippage_bps, amount_ratio_bps). 0 leader = none.
    pub fn get_subscription(dex: &OakDEX, follower: Address) -> (Address, U256, U256) {
        let leader = dex.copy_trading_leader.getter(follower).get();
        let slippage_bps = dex.copy_trading_slippage_bps.getter(follower).get();
        let amount_ratio_bps = dex.copy_trading_amount_ratio_bps.getter(follower).get();
        (leader, slippage_bps, amount_ratio_bps)
    }
}
