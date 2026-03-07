//! Emergency circuit breaker: TWAP price deviation check.
//!
//! If TWAP-observable price changes more than TWAP_DEVIATION_BPS_MAX (15%) in a single block,
//! the contract is put into Paused state and circuit breaker is triggered (audit trail).

use stylus_sdk::{alloy_primitives::U256, crypto};

use crate::constants::{q112_u256, BPS, TWAP_DEVIATION_BPS_MAX};
use crate::errors::{err, OakResult, ERR_CIRCUIT_BREAKER};
use crate::events::emit_emergency_triggered;
use crate::state::OakDEX;

/// Reason identifier for EmergencyTriggered (indexed for The Graph): keccak256("TWAP_DEVIATION").
pub fn emergency_reason_twap_deviation() -> stylus_sdk::alloy_primitives::FixedBytes<32> {
    crypto::keccak(b"TWAP_DEVIATION")
}

/// Check TWAP price deviation vs previous block. If deviation > 15%, set paused + circuit breaker and emit.
/// Call after updating cumulative TWAP (e.g. at start of swap). Reserves must be non-zero.
pub fn check_price_deviation(dex: &mut OakDEX, reserve0: U256, reserve1: U256) -> OakResult<()> {
    if reserve0.is_zero() || reserve1.is_zero() {
        return Ok(());
    }
    let q112 = q112_u256();
    let price0 = reserve1
        .checked_mul(q112)
        .ok_or_else(|| err(crate::errors::ERR_OVERFLOW))?
        .checked_div(reserve0)
        .ok_or_else(|| err(crate::errors::ERR_DIVISION_BY_ZERO))?;
    let price1 = reserve0
        .checked_mul(q112)
        .ok_or_else(|| err(crate::errors::ERR_OVERFLOW))?
        .checked_div(reserve1)
        .ok_or_else(|| err(crate::errors::ERR_DIVISION_BY_ZERO))?;

    let last0 = dex.last_twap_price0.get();
    let last1 = dex.last_twap_price1.get();

    let deviation_bps = |current: U256, last: U256| -> Option<U256> {
        if last.is_zero() {
            return Some(U256::ZERO);
        }
        let (diff, overflow) = if current > last {
            current.overflowing_sub(last)
        } else {
            last.overflowing_sub(current)
        };
        if overflow {
            return Some(U256::from(BPS)); // force trigger on overflow
        }
        diff.checked_mul(U256::from(BPS))?.checked_div(last)
    };

    if deviation_bps(price0, last0).map_or(true, |b| b > U256::from(TWAP_DEVIATION_BPS_MAX)) {
        dex.circuit_breaker_triggered.set(true);
        dex.paused.set(true);
        emit_emergency_triggered(emergency_reason_twap_deviation());
        return Err(err(ERR_CIRCUIT_BREAKER));
    }
    if deviation_bps(price1, last1).map_or(true, |b| b > U256::from(TWAP_DEVIATION_BPS_MAX)) {
        dex.circuit_breaker_triggered.set(true);
        dex.paused.set(true);
        emit_emergency_triggered(emergency_reason_twap_deviation());
        return Err(err(ERR_CIRCUIT_BREAKER));
    }

    dex.last_twap_price0.set(price0);
    dex.last_twap_price1.set(price1);
    Ok(())
}
