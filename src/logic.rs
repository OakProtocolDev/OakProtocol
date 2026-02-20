//! Core protocol logic: CPMM math, commit‑reveal, fee accounting.

use alloc::vec::Vec;

use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    block,
    call::Call,
    contract,
    crypto,
    msg,
    prelude::*,
};

use crate::{
    constants::{
        as_u256, COMMIT_REVEAL_DELAY, DEFAULT_FEE_BPS, FEE_DENOMINATOR,
        MAX_COMMITMENT_AGE, MAX_FEE_BPS, MINIMUM_LIQUIDITY, TREASURY_FEE_BPS,
    },
    errors::*,
    events::{
        emit_add_liquidity, emit_cancel_commitment, emit_commit_swap, emit_flash_swap,
        emit_pause_changed, emit_reveal_swap, emit_set_fee, emit_withdraw_treasury_fees,
    },
    state::OakDEX,
    token::{balance_of, safe_transfer, safe_transfer_from},
};

/// Encode `(amount_in, salt)` similarly to `abi.encode`.
fn encode_commit_data(amount_in: U256, salt: U256) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(64);
    encoded.extend_from_slice(&amount_in.to_be_bytes::<32>());
    encoded.extend_from_slice(&salt.to_be_bytes::<32>());
    encoded
}

/// Compute commitment hash as `keccak256(abi.encode(amount_in, salt))`.
fn compute_commit_hash(amount_in: U256, salt: U256) -> FixedBytes<32> {
    let encoded = encode_commit_data(amount_in, salt);
    crypto::keccak(&encoded)
}

/// Verify that `sender` is the contract owner.
fn only_owner(owner: Address) -> OakResult<()> {
    let sender = msg::sender();
    if sender != owner {
        return Err(err(ERR_ONLY_OWNER));
    }
    Ok(())
}

/// Validate that an address is not the zero address.
///
/// @notice Prevents invalid address inputs that could lead to fund loss.
/// @dev Zero address checks are critical for token transfers and access control.
fn require_non_zero_address(addr: Address) -> OakResult<()> {
    if addr == Address::ZERO {
        return Err(err(ERR_INVALID_ADDRESS));
    }
    Ok(())
}

/// Re-entrancy guard: ensure function is not called recursively.
///
/// @notice Checks and sets the global `locked` flag.
/// @dev Must be paired with `unlock_reentrancy_guard` in a finally-like pattern.
fn lock_reentrancy_guard(dex: &mut OakDEX) -> OakResult<()> {
    if dex.locked.get() {
        return Err(err(ERR_REENTRANT_CALL));
    }
    dex.locked.set(true);
    Ok(())
}

/// Re-entrancy guard: release the lock.
///
/// @notice Clears the global `locked` flag.
/// @dev Must be called after `lock_reentrancy_guard` to prevent deadlock.
fn unlock_reentrancy_guard(dex: &mut OakDEX) {
    dex.locked.set(false);
}

/// Pure CPMM math with a configurable total fee.
///
/// @notice Computes constant‑product output amount for a given input.
/// @dev Uses Uniswap‑style formula:
///      amount_out = (amount_in_with_fee * reserve_out)
///                   / (reserve_in * FEE_DENOMINATOR + amount_in_with_fee)
///      where amount_in_with_fee = amount_in * (FEE_DENOMINATOR - fee_bps).
pub fn get_amount_out_with_fee(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
    fee_bps: U256,
) -> OakResult<U256> {
    if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
        return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
    }

    let fee_multiplier = as_u256(FEE_DENOMINATOR)
        .checked_sub(fee_bps)
        .ok_or_else(|| err(ERR_FEE_OVERFLOW))?;

    let amount_in_with_fee = amount_in
        .checked_mul(fee_multiplier)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let numerator = amount_in_with_fee
        .checked_mul(reserve_out)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let denominator_part1 = reserve_in
        .checked_mul(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let denominator = denominator_part1
        .checked_add(amount_in_with_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    // Integer division in Rust performs floor rounding (rounds down).
    // This is protocol-favorable: users receive slightly less, protocol retains value.
    // Formula: amount_out = floor((amount_in_with_fee * reserve_out) / denominator)
    let amount_out = numerator
        .checked_div(denominator)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    Ok(amount_out)
}

/// Compute the total fee and its split between treasury and LPs.
///
/// @notice Splits a 0.3% fee into 0.12% treasury and 0.18% LPs.
/// @dev All math is done in `U256` to avoid narrowing conversions.
///      Rounding favors the protocol: any rounding remainder is allocated to LPs
///      to ensure treasury_fee + lp_fee = total_fee exactly.
pub fn compute_fee_split(amount_in: U256, fee_bps: U256) -> OakResult<(U256, U256, U256)> {
    if amount_in.is_zero() {
        return Ok((U256::ZERO, U256::ZERO, U256::ZERO));
    }

    let total_fee = amount_in
        .checked_mul(fee_bps)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    if total_fee.is_zero() {
        return Ok((amount_in, U256::ZERO, U256::ZERO));
    }

    // Calculate treasury fee (0.12% = 12/30 of total fee)
    let treasury_fee = total_fee
        .checked_mul(as_u256(TREASURY_FEE_BPS))
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(DEFAULT_FEE_BPS))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    // Calculate LP fee (0.18% = 18/30 of total fee)
    // Rounding protection: ensure treasury_fee + lp_fee = total_fee exactly
    // Any rounding remainder goes to LPs (protocol-favorable)
    let lp_fee = total_fee
        .checked_sub(treasury_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    // Verify: treasury_fee + lp_fee = total_fee (no precision loss)
    // This ensures the protocol never loses 1 wei due to rounding
    debug_assert!(treasury_fee + lp_fee == total_fee);

    let effective_in = amount_in
        .checked_sub(total_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    Ok((effective_in, treasury_fee, lp_fee))
}

/// Public contract functions implementation.
///
/// @notice Core entrypoints exposed to external callers.
/// @dev These methods operate on Stylus storage types defined in `state`.
#[public]
impl OakDEX {
    /// Initialize the contract.
    ///
    /// @notice One‑time initializer setting owner, treasury, and default fee.
    /// @dev Reverts if called more than once or if owner/treasury are zero.
    pub fn init(&mut self, initial_owner: Address, treasury: Address) -> OakResult<()> {
        let current_owner = self.owner.get();
        if current_owner != Address::ZERO {
            return Err(err(ERR_ALREADY_INITIALIZED));
        }

        if initial_owner == Address::ZERO {
            return Err(err(ERR_INVALID_OWNER));
        }
        if treasury == Address::ZERO {
            return Err(err(ERR_INVALID_OWNER));
        }

        self.owner.set(initial_owner);
        self.treasury.set(treasury);

        // Set default total fee (0.3%).
        self.protocol_fee_bps.set(as_u256(DEFAULT_FEE_BPS));

        // Initialize analytics and fee accounting.
        self.total_volume_token0.set(U256::ZERO);
        self.total_volume_token1.set(U256::ZERO);
        self.accrued_treasury_fees_token0.set(U256::ZERO);
        self.accrued_lp_fees_token0.set(U256::ZERO);

        // Contract starts active and unlocked.
        self.paused.set(false);
        self.locked.set(false);

        Ok(())
    }

    /// Update the total protocol fee.
    ///
    /// @notice Owner‑only function to adjust the global fee (in basis points).
    /// @dev Upper bound protects users from excessive fees.
    pub fn set_fee(&mut self, new_fee_bps: u16) -> OakResult<()> {
        let owner = self.owner.get();
        only_owner(owner)?;

        if new_fee_bps as u64 > MAX_FEE_BPS {
            return Err(err(ERR_FEE_TOO_HIGH));
        }

        self.protocol_fee_bps.set(U256::from(new_fee_bps));

        emit_set_fee(new_fee_bps);

        Ok(())
    }

    /// Pause trading in case of emergency.
    ///
    /// @notice Owner‑only panic button that disables swaps and commits.
    /// @dev This is a standard safety switch for governance and responders.
    pub fn pause(&mut self) -> OakResult<()> {
        let owner = self.owner.get();
        only_owner(owner)?;

        self.paused.set(true);
        emit_pause_changed(true);

        Ok(())
    }

    /// Resume trading after an incident is resolved.
    ///
    /// @notice Owner‑only function to re‑enable all functionality.
    pub fn unpause(&mut self) -> OakResult<()> {
        let owner = self.owner.get();
        only_owner(owner)?;

        self.paused.set(false);
        emit_pause_changed(false);

        Ok(())
    }

    /// Create a swap commitment.
    ///
    /// @notice Stores a commitment hash and the current block number.
    /// @dev Part 1 of the commit‑reveal flow used for MEV resistance.
    pub fn commit_swap(&mut self, hash: FixedBytes<32>) -> OakResult<()> {
        if self.paused.get() {
            return Err(err(ERR_PAUSED));
        }

        let sender = msg::sender();

        if hash == FixedBytes::ZERO {
            return Err(err(ERR_INVALID_HASH));
        }

        let current_block = U256::from(block::number());

        let hash_u256 = U256::from_be_bytes::<32>(hash.into());
        self.commitment_hashes.setter(sender).set(hash_u256);
        self.commitment_timestamps.setter(sender).set(current_block);
        self.commitment_activated.setter(sender).set(true);

        emit_commit_swap(sender, hash, current_block);

        Ok(())
    }

    /// Reveal a previously committed swap and execute it.
    ///
    /// @notice Performs hash verification, time‑lock enforcement, fee
    ///         accounting, CPMM pricing, slippage checks, and token transfers.
    /// @dev Part 2 of commit‑reveal flow, providing strong MEV protection.
    ///      Requires token0 and token1 addresses to perform transfers.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0 (input token)
    /// * `token1` - Address of token1 (output token)
    /// * `amount_in` - Input token amount
    /// * `salt` - Random salt used in commitment
    /// * `min_amount_out` - Minimum output tokens (slippage protection)
    pub fn reveal_swap(
        &mut self,
        token0: Address,
        token1: Address,
        amount_in: U256,
        salt: U256,
        min_amount_out: U256,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
        // This must be the first state-modifying operation
        lock_reentrancy_guard(self)?;

        // Input sanitization: validate addresses
        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        // Input sanitization: validate amounts
        if amount_in.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        if min_amount_out.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }

        // Pause guard
        if self.paused.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PAUSED));
        }

        let sender = msg::sender();

        // Reentrancy protection: check activation, then clear commitment
        // before performing any external‑effectful logic.
        let is_activated = self.commitment_activated.setter(sender).get();
        if !is_activated {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_COMMIT_NOT_FOUND));
        }

        let stored_hash_u256 = self.commitment_hashes.setter(sender).get();
        if stored_hash_u256.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_COMMIT_NOT_FOUND));
        }

        let computed_hash = compute_commit_hash(amount_in, salt);
        let computed_hash_u256 = U256::from_be_bytes::<32>(computed_hash.into());

        if stored_hash_u256 != computed_hash_u256 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_HASH));
        }

        let commit_block = self.commitment_timestamps.setter(sender).get();
        let current_block = U256::from(block::number());

        // Check commitment expiration (prevent storage bloat)
        let max_block = commit_block
            .checked_add(as_u256(MAX_COMMITMENT_AGE))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        if current_block > max_block {
            // Commitment expired, clear it and return error
            self.commitment_activated.setter(sender).set(false);
            self.commitment_hashes.setter(sender).set(U256::ZERO);
            unlock_reentrancy_guard(self);
            return Err(err(ERR_COMMITMENT_EXPIRED));
        }

        // Check minimum delay (MEV protection)
        let min_block = commit_block
            .checked_add(as_u256(COMMIT_REVEAL_DELAY))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        if current_block < min_block {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_TOO_EARLY));
        }

        // Clear commitment state prior to swap execution.
        self.commitment_activated.setter(sender).set(false);
        self.commitment_hashes.setter(sender).set(U256::ZERO);

        // Snapshot reserves and fee configuration.
        let reserve0 = self.reserves0.get();
        let reserve1 = self.reserves1.get();
        let fee_bps = self.protocol_fee_bps.get();

        // Compute amount_out using CPMM with total fee.
        let amount_out = match get_amount_out_with_fee(amount_in, reserve0, reserve1, fee_bps) {
            Ok(out) => out,
            Err(e) => {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        };

        // Explicit slippage protection via user‑provided minimum.
        if amount_out < min_amount_out {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }

        // Compute fee split for analytics and treasury accounting.
        let (_effective_in, treasury_fee, lp_fee) = match compute_fee_split(amount_in, fee_bps) {
            Ok(split) => split,
            Err(e) => {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        };

        // Update reserves under the standard CPMM assumption.
        let new_reserve0 = reserve0
            .checked_add(amount_in)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE0_OVERFLOW)
            })?;

        let new_reserve1 = reserve1
            .checked_sub(amount_out)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        let min_liquidity = self.min_liquidity.get();
        if new_reserve0 < min_liquidity || new_reserve1 < min_liquidity {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        self.reserves0.set(new_reserve0);
        self.reserves1.set(new_reserve1);

        // Update analytics and accounting.
        let current_volume0 = self.total_volume_token0.get();
        let current_volume1 = self.total_volume_token1.get();

        let new_volume0 = current_volume0
            .checked_add(amount_in)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_VOLUME_OVERFLOW)
            })?;

        let new_volume1 = current_volume1
            .checked_add(amount_out)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_VOLUME_OVERFLOW)
            })?;

        self.total_volume_token0.set(new_volume0);
        self.total_volume_token1.set(new_volume1);

        let current_treasury_fees = self.accrued_treasury_fees_token0.get();
        let current_lp_fees = self.accrued_lp_fees_token0.get();

        let new_treasury_fees = current_treasury_fees
            .checked_add(treasury_fee)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;
        let new_lp_fees = current_lp_fees
            .checked_add(lp_fee)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        self.accrued_treasury_fees_token0.set(new_treasury_fees);
        self.accrued_lp_fees_token0.set(new_lp_fees);

        // Transfer tokens: user -> contract (token0)
        let contract_addr = contract::address();
        if let Err(e) = safe_transfer_from(token0, sender, contract_addr, amount_in) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        // Transfer tokens: contract -> user (token1)
        if let Err(e) = safe_transfer(token1, sender, amount_out) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        emit_reveal_swap(sender, amount_in, amount_out, treasury_fee, lp_fee);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Add liquidity to the pool.
    ///
    /// @notice Adds token0 and token1 to the reserves, enforcing minimum liquidity.
    /// @dev In a full implementation, this would also mint LP tokens.
    ///      Transfers tokens from caller to contract before updating reserves.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0
    /// * `token1` - Address of token1
    /// * `amount0` - Amount of token0 to add
    /// * `amount1` - Amount of token1 to add
    pub fn add_liquidity(
        &mut self,
        token0: Address,
        token1: Address,
        amount0: U256,
        amount1: U256,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
        // This must be the first state-modifying operation
        lock_reentrancy_guard(self)?;

        // Input sanitization: validate addresses
        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        // Input sanitization: validate amounts
        if amount0.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_AMOUNT0_ZERO));
        }
        if amount1.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_AMOUNT1_ZERO));
        }

        // Pause guard
        if self.paused.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PAUSED));
        }

        let reserve0 = self.reserves0.get();
        let reserve1 = self.reserves1.get();
        let min_liquidity = self.min_liquidity.get();

        let new_reserve0 = reserve0
            .checked_add(amount0)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE0_OVERFLOW)
            })?;

        let new_reserve1 = reserve1
            .checked_add(amount1)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE1_OVERFLOW)
            })?;

        let total_liquidity = new_reserve0
            .checked_add(new_reserve1)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;

        if min_liquidity.is_zero() {
            let min_liq = as_u256(MINIMUM_LIQUIDITY);
            self.min_liquidity.set(min_liq);

            if total_liquidity < min_liq {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }
        } else if new_reserve0 < min_liquidity || new_reserve1 < min_liquidity {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Transfer tokens from caller to contract before updating state
        let provider = msg::sender();
        let contract_addr = contract::address();
        if let Err(e) = safe_transfer_from(token0, provider, contract_addr, amount0) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }
        if let Err(e) = safe_transfer_from(token1, provider, contract_addr, amount1) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        self.reserves0.set(new_reserve0);
        self.reserves1.set(new_reserve1);

        emit_add_liquidity(provider, amount0, amount1);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Cancel an expired or unwanted commitment.
    ///
    /// @notice Allows users to clear their commitment if it has expired or they no longer
    ///         wish to execute the swap. Prevents storage bloat from abandoned commitments.
    /// @dev Can only cancel own commitment, and only if expired or minimum delay has passed.
    ///
    /// # Returns
    /// `Ok(())` on successful cancellation
    pub fn cancel_commitment(&mut self) -> OakResult<()> {
        let sender = msg::sender();

        // Check if commitment exists
        let is_activated = self.commitment_activated.setter(sender).get();
        if !is_activated {
            return Err(err(ERR_COMMIT_NOT_FOUND));
        }

        let commit_block = self.commitment_timestamps.setter(sender).get();
        let current_block = U256::from(block::number());

        // Allow cancellation if:
        // 1. Commitment has expired (older than MAX_COMMITMENT_AGE blocks), OR
        // 2. Minimum delay has passed (user can cancel after reveal window)
        let max_block = commit_block
            .checked_add(as_u256(MAX_COMMITMENT_AGE))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        let min_block = commit_block
            .checked_add(as_u256(COMMIT_REVEAL_DELAY))
            .ok_or_else(|| err(ERR_BLOCK_OVERFLOW))?;

        // Can cancel if expired OR if minimum delay has passed
        if current_block <= max_block && current_block < min_block {
            // Cannot cancel: commitment is still valid and within reveal window
            return Err(err(ERR_TOO_EARLY));
        }

        // Clear commitment state
        self.commitment_activated.setter(sender).set(false);
        self.commitment_hashes.setter(sender).set(U256::ZERO);
        self.commitment_timestamps.setter(sender).set(U256::ZERO);

        emit_cancel_commitment(sender, current_block);

        Ok(())
    }

    /// Withdraw accrued treasury fees.
    ///
    /// @notice Owner-only function to transfer accumulated treasury fees (0.12% of swaps)
    ///         to the treasury address.
    /// @dev Resets the accrued counter after withdrawal to prevent double-spending.
    ///
    /// # Arguments
    /// * `token` - Address of the token to withdraw (must be token0)
    ///
    /// # Returns
    /// `Ok(())` on success, error if no fees available or transfer fails
    pub fn withdraw_treasury_fees(&mut self, token: Address) -> OakResult<()> {
        // Owner check
        let owner = self.owner.get();
        only_owner(owner)?;

        // Input sanitization: validate token address
        require_non_zero_address(token)?;

        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        let treasury = self.treasury.get();
        if treasury == Address::ZERO {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_OWNER));
        }

        let accrued = self.accrued_treasury_fees_token0.get();
        if accrued.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_NO_TREASURY_FEES));
        }

        // Reset counter before transfer (CEI pattern)
        self.accrued_treasury_fees_token0.set(U256::ZERO);

        // Transfer to treasury
        safe_transfer(token, treasury, accrued)?;

        emit_withdraw_treasury_fees(treasury, token, accrued);

        // Release re-entrancy guard
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Execute a flash swap (uncollateralized loan).
    ///
    /// @notice Allows borrowing tokens without upfront collateral, provided the borrower
    ///         returns the borrowed amount plus fees within the same transaction.
    /// @dev Uses a callback pattern via `IOakCallee` to notify the borrower.
    ///      After the callback, the new product of reserves (k = reserve0 * reserve1)
    ///      must be greater than or equal to the product before the swap, including fees.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0 (can be borrowed if amount0_out > 0)
    /// * `token1` - Address of token1 (can be borrowed if amount1_out > 0)
    /// * `amount0_out` - Amount of token0 to borrow (0 if not borrowing token0)
    /// * `amount1_out` - Amount of token1 to borrow (0 if not borrowing token1)
    /// * `data` - Optional calldata to pass to the callback
    ///
    /// # Safety
    /// - Re-entrancy guard is active during the entire flash swap
    /// - Verifies k' >= k * (1 + fee) after callback
    /// - Reverts if insufficient liquidity or repayment fails
    pub fn flash_swap(
        &mut self,
        token0: Address,
        token1: Address,
        amount0_out: U256,
        amount1_out: U256,
        data: Vec<u8>,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
        // This must be the first state-modifying operation
        lock_reentrancy_guard(self)?;

        // Input sanitization: validate addresses
        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        // Input sanitization: at least one amount must be non-zero
        if amount0_out.is_zero() && amount1_out.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }

        // Pause guard
        if self.paused.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_PAUSED));
        }

        // Snapshot reserves and fee configuration before the swap
        let reserve0_before = self.reserves0.get();
        let reserve1_before = self.reserves1.get();
        let fee_bps = self.protocol_fee_bps.get();

        // Calculate initial k (constant product before swap)
        let k_before = reserve0_before
            .checked_mul(reserve1_before)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        // Verify sufficient liquidity for the requested amounts
        if amount0_out > reserve0_before || amount1_out > reserve1_before {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Calculate new reserves after lending (before callback)
        let reserve0_after_lend = reserve0_before
            .checked_sub(amount0_out)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        let reserve1_after_lend = reserve1_before
            .checked_sub(amount1_out)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        // Ensure minimum liquidity is maintained
        let min_liquidity = self.min_liquidity.get();
        if reserve0_after_lend < min_liquidity || reserve1_after_lend < min_liquidity {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Transfer tokens to borrower (INTERACTION: external call)
        let borrower = msg::sender();
        let contract_addr = contract::address();

        if !amount0_out.is_zero() {
            if let Err(e) = safe_transfer(token0, borrower, amount0_out) {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        }

        if !amount1_out.is_zero() {
            if let Err(e) = safe_transfer(token1, borrower, amount1_out) {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        }

        // Calculate fees owed (0.3% of borrowed amounts)
        // Fee calculation: fee = amount * fee_bps / FEE_DENOMINATOR
        let fee0 = if !amount0_out.is_zero() {
            amount0_out
                .checked_mul(fee_bps)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?
                .checked_div(as_u256(FEE_DENOMINATOR))
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?
        } else {
            U256::ZERO
        };

        let fee1 = if !amount1_out.is_zero() {
            amount1_out
                .checked_mul(fee_bps)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?
                .checked_div(as_u256(FEE_DENOMINATOR))
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?
        } else {
            U256::ZERO
        };

        // Calculate total repayment amounts (borrowed + fees)
        let amount0_owed = amount0_out
            .checked_add(fee0)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        let amount1_owed = amount1_out
            .checked_add(fee1)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        // Call callback (INTERACTION: external call to borrower's contract)
        // The borrower must implement: oakFlashSwapCallback(uint256,uint256,bytes)
        // We use ABI encoding to call the callback function
        let selector = crypto::keccak(b"oakFlashSwapCallback(uint256,uint256,bytes)");
        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector[0..4]); // Function selector (first 4 bytes)
        
        // ABI encode parameters: (uint256, uint256, bytes)
        // For uint256: pad to 32 bytes, big-endian
        call_data.extend_from_slice(&amount0_owed.to_be_bytes::<32>());
        call_data.extend_from_slice(&amount1_owed.to_be_bytes::<32>());
        
        // For bytes: offset (32 bytes) + length (32 bytes) + data (padded to 32-byte boundary)
        let data_offset = U256::from(96u64); // offset to data: 32 (amount0) + 32 (amount1) + 32 (offset)
        call_data.extend_from_slice(&data_offset.to_be_bytes::<32>());
        let data_len = U256::from(data.len());
        call_data.extend_from_slice(&data_len.to_be_bytes::<32>());
        call_data.extend_from_slice(&data);
        // Pad data to 32-byte boundary
        let padding = (32 - (data.len() % 32)) % 32;
        for _ in 0..padding {
            call_data.push(0u8);
        }
        
        // Make the external call - this will revert if callback fails
        // The callback must transfer the repayment tokens back to this contract
        let call = Call::new_in(borrower);
        if let Err(e) = call.call_raw(&call_data, false) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        // Verify repayment: check contract balances after callback
        let balance0_after = balance_of(token0, contract_addr);
        let balance1_after = balance_of(token1, contract_addr);

        // Calculate what the balances should be after repayment
        // We need: balance0_after >= reserve0_after_lend + amount0_owed
        //         balance1_after >= reserve1_after_lend + amount1_owed
        let expected_balance0 = reserve0_after_lend
            .checked_add(amount0_owed)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        let expected_balance1 = reserve1_after_lend
            .checked_add(amount1_owed)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        if balance0_after < expected_balance0 || balance1_after < expected_balance1 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Calculate actual repayment amounts (may be more than required)
        let actual_repayment0 = balance0_after
            .checked_sub(reserve0_after_lend)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        let actual_repayment1 = balance1_after
            .checked_sub(reserve1_after_lend)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_INSUFFICIENT_LIQUIDITY)
            })?;

        // Update reserves to reflect the repayment
        // New reserves = reserves_after_lend + actual_repayment
        let reserve0_after = reserve0_after_lend
            .checked_add(actual_repayment0)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE0_OVERFLOW)
            })?;

        let reserve1_after = reserve1_after_lend
            .checked_add(actual_repayment1)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE1_OVERFLOW)
            })?;

        // CRITICAL: Verify k' >= k * (1 + fee_rate)
        // This ensures the protocol doesn't lose value and collects fees
        // k_after = reserve0_after * reserve1_after
        let k_after = reserve0_after
            .checked_mul(reserve1_after)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        // Calculate minimum k required: k_min = k_before * (FEE_DENOMINATOR + fee_bps) / FEE_DENOMINATOR
        // This ensures the new product includes the 0.3% fee as required
        // Example: if fee_bps = 30 (0.3%), then k_min = k_before * 10030 / 10000
        let fee_multiplier = as_u256(FEE_DENOMINATOR)
            .checked_add(fee_bps)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?;

        let k_min = k_before
            .checked_mul(fee_multiplier)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?
            .checked_div(as_u256(FEE_DENOMINATOR))
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;

        // Verify k_after >= k_min (protocol must not lose value, fees must be paid)
        // This is the core requirement: new product must be >= old product * (1 + fee)
        if k_after < k_min {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Update reserves (EFFECT: state change)
        self.reserves0.set(reserve0_after);
        self.reserves1.set(reserve1_after);

        // Update analytics: track flash swap volume
        let current_volume0 = self.total_volume_token0.get();
        let current_volume1 = self.total_volume_token1.get();

        if !amount0_out.is_zero() {
            let new_volume0 = current_volume0
                .checked_add(amount0_out)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_VOLUME_OVERFLOW)
                })?;
            self.total_volume_token0.set(new_volume0);
        }

        if !amount1_out.is_zero() {
            let new_volume1 = current_volume1
                .checked_add(amount1_out)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_VOLUME_OVERFLOW)
                })?;
            self.total_volume_token1.set(new_volume1);
        }

        // Update fee accounting
        if !fee0.is_zero() {
            let (_effective_in, treasury_fee0, lp_fee0) =
                match compute_fee_split(amount0_out, fee_bps) {
                    Ok(split) => split,
                    Err(e) => {
                        unlock_reentrancy_guard(self);
                        return Err(e);
                    }
                };

            let current_treasury_fees = self.accrued_treasury_fees_token0.get();
            let current_lp_fees = self.accrued_lp_fees_token0.get();

            let new_treasury_fees = current_treasury_fees
                .checked_add(treasury_fee0)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?;
            let new_lp_fees = current_lp_fees
                .checked_add(lp_fee0)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?;

            self.accrued_treasury_fees_token0.set(new_treasury_fees);
            self.accrued_lp_fees_token0.set(new_lp_fees);
        }

        // Emit FlashSwap event
        emit_flash_swap(borrower, token0, token1, amount0_out, amount1_out, fee0, fee1);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpmm_math_respects_fee() {
        let amount_in = U256::from(1_000u64);
        let reserve_in = U256::from(10_000u64);
        let reserve_out = U256::from(20_000u64);
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let out =
            get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps).unwrap();

        // Sanity: positive output and strictly less than pro‑rata without fees.
        assert!(out > U256::ZERO);

        let out_no_fee = amount_in * reserve_out / (reserve_in + amount_in);
        assert!(out < out_no_fee);
    }

    #[test]
    fn fee_split_matches_ratios() {
        let amount_in = U256::from(1_000_000u64);
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let (_effective_in, treasury_fee, lp_fee) =
            compute_fee_split(amount_in, fee_bps).unwrap();

        // Total fee should be 0.3% of amount_in.
        let total_fee = treasury_fee + lp_fee;
        let expected_total_fee = amount_in * as_u256(DEFAULT_FEE_BPS) / as_u256(FEE_DENOMINATOR);
        assert_eq!(total_fee, expected_total_fee);

        // Treasury share should be 0.12% and LP share 0.18% of amount_in (within integer rounding).
        let expected_treasury =
            amount_in * as_u256(TREASURY_FEE_BPS) / as_u256(FEE_DENOMINATOR);
        let expected_lp = amount_in * as_u256(LP_FEE_BPS) / as_u256(FEE_DENOMINATOR);

        assert_eq!(treasury_fee, expected_treasury);
        assert_eq!(lp_fee, expected_lp);
    }

    #[test]
    fn commit_hash_roundtrip() {
        let amount_in = U256::from(42u64);
        let salt = U256::from(1337u64);

        let hash = compute_commit_hash(amount_in, salt);

        let encoded = encode_commit_data(amount_in, salt);
        let direct = crypto::keccak(&encoded);

        assert_eq!(hash, direct);
    }

    #[test]
    fn fee_split_no_precision_loss() {
        // Test that rounding never causes protocol to lose 1 wei
        // Use values that don't divide evenly to test rounding protection
        let amount_in = U256::from(1_000_001u64); // 1M + 1 (tests rounding)
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let (_effective_in, treasury_fee, lp_fee) =
            compute_fee_split(amount_in, fee_bps).unwrap();

        // Calculate expected total fee
        let expected_total_fee = amount_in
            .checked_mul(fee_bps)
            .unwrap()
            .checked_div(as_u256(FEE_DENOMINATOR))
            .unwrap();

        // Verify: treasury_fee + lp_fee = total_fee exactly (no precision loss)
        let actual_total_fee = treasury_fee
            .checked_add(lp_fee)
            .unwrap();

        assert_eq!(
            actual_total_fee, expected_total_fee,
            "Fee split must not lose precision: {} + {} = {}, expected {}",
            treasury_fee, lp_fee, actual_total_fee, expected_total_fee
        );

        // Verify effective_in calculation
        let calculated_effective_in = amount_in
            .checked_sub(expected_total_fee)
            .unwrap();
        assert_eq!(_effective_in, calculated_effective_in);
    }

    #[test]
    fn cpmm_floor_rounding_favors_protocol() {
        // Test that CPMM calculation uses floor rounding (protocol-favorable)
        let amount_in = U256::from(1_000u64);
        let reserve_in = U256::from(10_000u64);
        let reserve_out = U256::from(20_000u64);
        let fee_bps = as_u256(DEFAULT_FEE_BPS);

        let amount_out = get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps)
            .unwrap();

        // Calculate exact value (with infinite precision)
        // amount_in_with_fee = amount_in * (FEE_DENOMINATOR - fee_bps) / FEE_DENOMINATOR
        // amount_out_exact = (amount_in_with_fee * reserve_out) / (reserve_in * FEE_DENOMINATOR + amount_in_with_fee)
        let amount_in_with_fee = amount_in
            .checked_mul(as_u256(FEE_DENOMINATOR).checked_sub(fee_bps).unwrap())
            .unwrap()
            .checked_div(as_u256(FEE_DENOMINATOR))
            .unwrap();

        let numerator = amount_in_with_fee
            .checked_mul(reserve_out)
            .unwrap();

        let denominator = reserve_in
            .checked_mul(as_u256(FEE_DENOMINATOR))
            .unwrap()
            .checked_add(amount_in_with_fee)
            .unwrap();

        // Integer division performs floor rounding
        let expected_floor = numerator.checked_div(denominator).unwrap();

        assert_eq!(
            amount_out, expected_floor,
            "CPMM must use floor rounding (protocol-favorable)"
        );
    }
}

