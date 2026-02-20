//! ERC-20 token interface and safe transfer utilities for Oak Protocol.
//!
//! @notice Provides type-safe wrappers around ERC-20 token operations.
//! @dev Uses stylus_sdk's call interface to interact with standard ERC-20 contracts.

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    call::Call,
    prelude::*,
};

use crate::errors::{err, OakResult, ERR_INVALID_ADDRESS, ERR_TOKEN_TRANSFER_FAILED};

/// Standard ERC-20 interface definition.
///
/// @notice Minimal interface for token transfers required by Oak Protocol.
/// @dev Only includes methods we actually use: transfer, transferFrom, balanceOf.
sol_interface! {
    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }
}

// Note: Flash swap callback interface
// Contracts implementing flash swaps must have a function:
// function oakFlashSwapCallback(uint256 amount0_owed, uint256 amount1_owed, bytes calldata data) external;
// This is called via raw ABI encoding in the flash_swap function.

/// Safely transfer ERC-20 tokens from `from` to `to`.
///
/// @notice Uses `transferFrom` to move tokens on behalf of a user.
/// @dev This function assumes the contract has been approved by `from`
///      to spend `amount` tokens. Reverts if transfer fails.
///      Includes input sanitization for addresses and amounts.
///
/// # Arguments
/// * `token` - Address of the ERC-20 token contract
/// * `from` - Address to transfer tokens from
/// * `to` - Address to transfer tokens to
/// * `amount` - Amount of tokens to transfer
///
/// # Returns
/// `Ok(())` on success, error on failure
pub fn safe_transfer_from(
    token: Address,
    from: Address,
    to: Address,
    amount: U256,
) -> OakResult<()> {
    // Input sanitization: validate addresses
    if token == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    if from == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    if to == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }

    // Input sanitization: validate amount
    if amount.is_zero() {
        return Ok(());
    }

    let call = Call::new_in(token);
    match IERC20::transferFrom(call, from, to, amount) {
        Ok(success) => {
            if success {
                Ok(())
            } else {
                Err(err(ERR_TOKEN_TRANSFER_FAILED))
            }
        }
        Err(e) => Err(e),
    }
}

/// Safely transfer ERC-20 tokens from this contract to `to`.
///
/// @notice Uses `transfer` to move tokens held by this contract.
/// @dev Reverts if transfer fails or contract has insufficient balance.
///      Includes input sanitization for addresses and amounts.
///
/// # Arguments
/// * `token` - Address of the ERC-20 token contract
/// * `to` - Address to transfer tokens to
/// * `amount` - Amount of tokens to transfer
///
/// # Returns
/// `Ok(())` on success, error on failure
pub fn safe_transfer(token: Address, to: Address, amount: U256) -> OakResult<()> {
    // Input sanitization: validate addresses
    if token == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    if to == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }

    // Input sanitization: validate amount
    if amount.is_zero() {
        return Ok(());
    }

    let call = Call::new_in(token);
    match IERC20::transfer(call, to, amount) {
        Ok(success) => {
            if success {
                Ok(())
            } else {
                Err(err(ERR_TOKEN_TRANSFER_FAILED))
            }
        }
        Err(e) => Err(e),
    }
}

/// Get the balance of an ERC-20 token for a given address.
///
/// @notice Queries token balance without modifying state.
/// @dev Returns zero if token contract doesn't exist or call fails.
///
/// # Arguments
/// * `token` - Address of the ERC-20 token contract
/// * `account` - Address to query balance for
///
/// # Returns
/// Token balance as `U256`, or zero on error
pub fn balance_of(token: Address, account: Address) -> U256 {
    let call = Call::new_in(token);
    match IERC20::balanceOf(call, account) {
        Ok(balance) => balance,
        Err(_) => U256::ZERO,
    }
}

/// Transfer native ETH from this contract to `to`.
///
/// @notice Uses Stylus's `transfer_eth` for native token transfers.
/// @dev Unlike Solidity's `transfer`, this forwards all gas.
///      Use with caution when interacting with untrusted contracts.
///
/// # Arguments
/// * `to` - Address to send ETH to
/// * `amount` - Amount of ETH to transfer (in wei)
///
/// # Returns
/// `Ok(())` on success, error on failure
pub fn safe_transfer_eth(to: Address, amount: U256) -> OakResult<()> {
    if amount.is_zero() {
        return Ok(());
    }

    stylus_sdk::call::transfer_eth(to, amount).map_err(|_| err(ERR_TOKEN_TRANSFER_FAILED))
}
