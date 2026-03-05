//! ERC-20 token interface and safe transfer utilities for Oak Protocol.
//!
//! For now this module provides **host-test-friendly stubs** for token
//! operations so that we can exercise the DEX math and state logic
//! off-chain. On-chain Stylus integration (via `sol_interface!` and
//! `Call`) can be reintroduced on top of these signatures.

use stylus_sdk::alloy_primitives::{Address, U256};

use crate::errors::{err, OakResult, ERR_INVALID_ADDRESS, ERR_TOKEN_TRANSFER_FAILED};

/// Safely transfer ERC-20 tokens from `from` to `to`.
///
/// Host-side implementation performs only input validation and assumes
/// success. On-chain, this should perform a real `transferFrom` call.
pub fn safe_transfer_from(
    token: Address,
    from: Address,
    to: Address,
    amount: U256,
) -> OakResult<()> {
    if token == Address::ZERO || from == Address::ZERO || to == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    if amount.is_zero() {
        return Ok(());
    }
    Ok(())
}

/// Safely transfer ERC-20 tokens from this contract to `to`.
///
/// Host-side implementation performs only input validation and assumes
/// success. On-chain, this should perform a real `transfer` call.
pub fn safe_transfer(token: Address, to: Address, amount: U256) -> OakResult<()> {
    if token == Address::ZERO || to == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    if amount.is_zero() {
        return Ok(());
    }
    Ok(())
}

/// Get the balance of an ERC-20 token for a given address.
///
/// Host-side implementation always returns zero; this is sufficient for
/// our pure-math tests that do not rely on actual balances.
pub fn balance_of(_token: Address, _account: Address) -> U256 {
    U256::ZERO
}

/// Transfer native ETH from this contract to `to`.
///
/// Host-side stub: returns `Ok(())` for zero amount and a generic error
/// otherwise. On-chain, this should call `stylus_sdk::call::transfer_eth`.
pub fn safe_transfer_eth(_to: Address, amount: U256) -> OakResult<()> {
    if amount.is_zero() {
        return Ok(());
    }
    Err(err(ERR_TOKEN_TRANSFER_FAILED))
}

