//! GMX-style Vault logic for Oak Protocol.
//!
//! Internal (non-public) helpers for swap and leverage. All state-changing
//! entrypoints are intended to be invoked by the OakSentinel after the
//! Commit–Reveal step. Math is 100% checked; uses alloy_primitives (U256).

use stylus_sdk::alloy_primitives::{Address, U256};

use crate::errors::{err, OakResult, ERR_DIVISION_BY_ZERO, ERR_OVERFLOW};
use crate::errors::{
    ERR_INSUFFICIENT_INPUT_AMOUNT,
    ERR_INSUFFICIENT_OUTPUT_AMOUNT,
    ERR_VAULT_BUFFER,
    ERR_VAULT_POOL_EXCEEDED,
    ERR_VAULT_RESERVE_EXCEEDS_POOL,
    ERR_VAULT_INSUFFICIENT_RESERVE,
};
use crate::state::OakSentinel;

/// Basis points divisor (10_000 = 100%).
const BASIS_POINTS_DIVISOR: u64 = 10_000;
/// Price precision (GMX uses 10^30; we use 10^18 for smaller constants).
const PRICE_PRECISION: u64 = 1_000_000_000_000_000_000;

fn as_u256(x: u64) -> U256 {
    U256::from(x)
}

// -----------------------------------------------------------------------------
// Pool amount helpers (GMX: _increasePoolAmount / _decreasePoolAmount)
// -----------------------------------------------------------------------------

#[inline]
fn get_pool_amount(state: &mut OakSentinel, token: Address) -> U256 {
    state.vault_pool_amount.setter(token).get()
}

#[inline]
fn set_pool_amount(state: &mut OakSentinel, token: Address, value: U256) {
    state.vault_pool_amount.setter(token).set(value);
}

fn increase_pool_amount(state: &mut OakSentinel, token: Address, amount: U256) -> OakResult<()> {
    let pool = get_pool_amount(state, token);
    let new_pool = pool.checked_add(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
    set_pool_amount(state, token, new_pool);
    Ok(())
}

fn decrease_pool_amount(
    state: &mut OakSentinel,
    token: Address,
    amount: U256,
) -> OakResult<()> {
    let pool = get_pool_amount(state, token);
    let reserved = state.vault_reserved_amount.setter(token).get();
    let new_pool = pool.checked_sub(amount).ok_or_else(|| err(ERR_VAULT_POOL_EXCEEDED))?;
    if reserved > new_pool {
        return Err(err(ERR_VAULT_RESERVE_EXCEEDS_POOL));
    }
    set_pool_amount(state, token, new_pool);
    Ok(())
}

// -----------------------------------------------------------------------------
// Fee reserves (GMX: feeReserves)
// -----------------------------------------------------------------------------

fn add_fee_reserves(state: &mut OakSentinel, token: Address, amount: U256) -> OakResult<()> {
    let current = state.vault_fee_reserves.setter(token).get();
    let new_val = current.checked_add(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
    state.vault_fee_reserves.setter(token).set(new_val);
    Ok(())
}

// -----------------------------------------------------------------------------
// Reserved amount (GMX: reservedAmounts) – for open leverage positions
// -----------------------------------------------------------------------------

fn get_reserved_amount(state: &mut OakSentinel, token: Address) -> U256 {
    state.vault_reserved_amount.setter(token).get()
}

fn increase_reserved_amount(
    state: &mut OakSentinel,
    token: Address,
    amount: U256,
) -> OakResult<()> {
    let pool = get_pool_amount(state, token);
    let reserved = get_reserved_amount(state, token);
    let new_reserved = reserved.checked_add(amount).ok_or_else(|| err(ERR_OVERFLOW))?;
    if new_reserved > pool {
        return Err(err(ERR_VAULT_RESERVE_EXCEEDS_POOL));
    }
    state.vault_reserved_amount.setter(token).set(new_reserved);
    Ok(())
}

fn decrease_reserved_amount(
    state: &mut OakSentinel,
    token: Address,
    amount: U256,
) -> OakResult<()> {
    let reserved = get_reserved_amount(state, token);
    let new_reserved = reserved.checked_sub(amount).ok_or_else(|| err(ERR_VAULT_INSUFFICIENT_RESERVE))?;
    state.vault_reserved_amount.setter(token).set(new_reserved);
    Ok(())
}

// -----------------------------------------------------------------------------
// Guaranteed USD (GMX: guaranteedUsd) – long position exposure
// -----------------------------------------------------------------------------

fn get_guaranteed_usd(state: &mut OakSentinel, token: Address) -> U256 {
    state.vault_guaranteed_usd.setter(token).get()
}

fn increase_guaranteed_usd(
    state: &mut OakSentinel,
    token: Address,
    usd_amount: U256,
) -> OakResult<()> {
    let current = get_guaranteed_usd(state, token);
    let new_val = current.checked_add(usd_amount).ok_or_else(|| err(ERR_OVERFLOW))?;
    state.vault_guaranteed_usd.setter(token).set(new_val);
    Ok(())
}

fn decrease_guaranteed_usd(
    state: &mut OakSentinel,
    token: Address,
    usd_amount: U256,
) -> OakResult<()> {
    let current = get_guaranteed_usd(state, token);
    let new_val = current.checked_sub(usd_amount).ok_or_else(|| err(ERR_OVERFLOW))?;
    state.vault_guaranteed_usd.setter(token).set(new_val);
    Ok(())
}

// -----------------------------------------------------------------------------
// Global short size (single counter in our layout; GMX has per-token)
// -----------------------------------------------------------------------------

fn increase_global_short_size(state: &mut OakSentinel, usd_delta: U256) -> OakResult<()> {
    let current = state.vault_global_short_size_usd.get();
    let new_val = current.checked_add(usd_delta).ok_or_else(|| err(ERR_OVERFLOW))?;
    state.vault_global_short_size_usd.set(new_val);
    Ok(())
}

fn decrease_global_short_size(state: &mut OakSentinel, usd_delta: U256) -> OakResult<()> {
    let current = state.vault_global_short_size_usd.get();
    let new_val = if usd_delta >= current {
        U256::ZERO
    } else {
        current.checked_sub(usd_delta).ok_or_else(|| err(ERR_OVERFLOW))?
    };
    state.vault_global_short_size_usd.set(new_val);
    Ok(())
}

// -----------------------------------------------------------------------------
// Buffer check (GMX: _validateBufferAmount)
// -----------------------------------------------------------------------------

fn validate_buffer_amount(state: &mut OakSentinel, token: Address) -> OakResult<()> {
    let pool = get_pool_amount(state, token);
    let buffer = state.vault_buffer_amount.setter(token).get();
    if pool < buffer {
        return Err(err(ERR_VAULT_BUFFER));
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// _swap(token_in, token_out, receiver)
// -----------------------------------------------------------------------------

/// Core swap logic using GMX-style formulas: price-based amount_out, fee in
/// basis points, then update pool amounts and fee reserves.
///
/// Caller must have already transferred `amount_in` of `token_in` into the
/// vault. `receiver` is unused here (caller performs transfer out of
/// `amount_out_after_fees` to `receiver`).
///
/// Uses: amount_out = amount_in * price_in / price_out, then fee on
/// amount_out, then pool_in += amount_in, pool_out -= amount_out,
/// fee_reserves(token_out) += fee.
pub fn _swap(
    state: &mut OakSentinel,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    price_in: U256,
    price_out: U256,
    fee_bps: u64,
) -> OakResult<U256> {
    if amount_in.is_zero() {
        return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
    }
    if price_out.is_zero() {
        return Err(err(ERR_DIVISION_BY_ZERO));
    }

    // amount_out_raw = amount_in * price_in / price_out (same decimals assumed)
    let amount_out_raw = amount_in
        .checked_mul(price_in)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(price_out)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    if amount_out_raw.is_zero() {
        return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
    }

    let fee_bps_u = as_u256(fee_bps.min(BASIS_POINTS_DIVISOR));
    let divisor = as_u256(BASIS_POINTS_DIVISOR);
    let fee_amount = amount_out_raw
        .checked_mul(fee_bps_u)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(divisor)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    let amount_out_after_fees = amount_out_raw
        .checked_sub(fee_amount)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let pool_out = get_pool_amount(state, token_out);
    if pool_out < amount_out_raw {
        return Err(err(ERR_VAULT_POOL_EXCEEDED));
    }

    increase_pool_amount(state, token_in, amount_in)?;
    decrease_pool_amount(state, token_out, amount_out_raw)?;
    add_fee_reserves(state, token_out, fee_amount)?;

    validate_buffer_amount(state, token_out)?;

    Ok(amount_out_after_fees)
}

// -----------------------------------------------------------------------------
// _increase_position
// -----------------------------------------------------------------------------

/// Increase a leverage position: adjust reserved amount and guaranteed USD
/// (or global short size). All deltas are precomputed by the caller (e.g.
/// from oracle prices and size/collateral deltas).
///
/// - Long: increase `reserved_amount` by `reserved_delta` (tokens reserved
///   to pay profits), increase `guaranteed_usd` by `guaranteed_delta`.
/// - Short: increase `reserved_amount` by `reserved_delta`, increase
///   `vault_global_short_size_usd` by `size_delta_usd`.
pub fn _increase_position(
    state: &mut OakSentinel,
    collateral_token: Address,
    _index_token: Address,
    size_delta_usd: U256,
    reserved_delta: U256,
    guaranteed_delta: U256,
    is_long: bool,
) -> OakResult<()> {
    increase_reserved_amount(state, collateral_token, reserved_delta)?;
    if is_long {
        increase_guaranteed_usd(state, collateral_token, guaranteed_delta)?;
        let global_long = state.vault_global_long_size_usd.get();
        let new_long = global_long
            .checked_add(size_delta_usd)
            .ok_or_else(|| err(ERR_OVERFLOW))?;
        state.vault_global_long_size_usd.set(new_long);
    } else {
        increase_global_short_size(state, size_delta_usd)?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// _decrease_position
// -----------------------------------------------------------------------------

/// Decrease a leverage position: decrease reserved amount and guaranteed USD
/// (or global short size). Deltas are precomputed by the caller.
///
/// - Long: decrease `reserved_amount` by `reserved_delta`, decrease
///   `guaranteed_usd` by `guaranteed_delta`, decrease global long size.
/// - Short: decrease `reserved_amount`, decrease global short size.
pub fn _decrease_position(
    state: &mut OakSentinel,
    collateral_token: Address,
    _index_token: Address,
    size_delta_usd: U256,
    reserved_delta: U256,
    guaranteed_delta: U256,
    is_long: bool,
) -> OakResult<()> {
    decrease_reserved_amount(state, collateral_token, reserved_delta)?;
    if is_long {
        decrease_guaranteed_usd(state, collateral_token, guaranteed_delta)?;
        let global_long = state.vault_global_long_size_usd.get();
        let new_long = if size_delta_usd >= global_long {
            U256::ZERO
        } else {
            global_long
                .checked_sub(size_delta_usd)
                .ok_or_else(|| err(ERR_OVERFLOW))?
        };
        state.vault_global_long_size_usd.set(new_long);
    } else {
        decrease_global_short_size(state, size_delta_usd)?;
    }
    Ok(())
}
