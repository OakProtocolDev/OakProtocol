//! Core protocol logic: CPMM math, commit‑reveal, fee accounting.

use alloc::vec::Vec;

use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    block,
    contract,
    crypto,
    msg,
};

use crate::{
    constants::{
        as_u256, q112_u256, COMMIT_REVEAL_DELAY, DEFAULT_FEE_BPS, FEE_DENOMINATOR, GAS_REBATE_BPS,
        INITIAL_FEE, MAX_COMMITMENT_AGE, MAX_FEE_BPS, MINIMUM_LIQUIDITY, TREASURY_FEE_BPS,
    },
    errors::*,
    events::{
        emit_add_liquidity, emit_cancel_commitment, emit_commit_swap, emit_flash_swap,
        emit_lp_transfer, emit_pause_changed, emit_reveal_swap, emit_set_fee,
        emit_withdraw_treasury_fees,
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

/// Emergency circuit breaker: revert if protocol is paused.
///
/// @notice Applied to commit_swap, reveal_swap, and flash_swap.
/// @dev Only owner can pause/unpause via pause() and unpause().
fn require_not_paused(dex: &OakDEX) -> OakResult<()> {
    if dex.paused.get() {
        return Err(err(ERR_PAUSED));
    }
    Ok(())
}

/// Update TWAP oracle cumulative prices and last block.
///
/// @notice Must be called at the beginning of every swap (reveal_swap) and add_liquidity.
/// @dev Uses Q112.64 fixed-point: price0 = reserve1/reserve0, price1 = reserve0/reserve1.
///      On L2 we use block number as time index for gas efficiency.
///      cumulative += price * (current_block - block_last); all math checked.
fn update_oracle(dex: &mut OakDEX, reserve0: U256, reserve1: U256) -> OakResult<()> {
    let block_last = dex.block_timestamp_last.get();
    let current_block = U256::from(block::number());

    if reserve0.is_zero() || reserve1.is_zero() {
        dex.block_timestamp_last.set(current_block);
        return Ok(());
    }

    let time_elapsed = current_block.checked_sub(block_last).unwrap_or(U256::ZERO);
    if time_elapsed.is_zero() {
        return Ok(());
    }

    let q112 = q112_u256();
    // price0 = reserve1 / reserve0 in Q112.64
    let price0 = reserve1
        .checked_mul(q112)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(reserve0)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    // price1 = reserve0 / reserve1 in Q112.64
    let price1 = reserve0
        .checked_mul(q112)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(reserve1)
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;

    let cum0_delta = price0
        .checked_mul(time_elapsed)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let cum1_delta = price1
        .checked_mul(time_elapsed)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    let cum0 = dex.price0_cumulative_last.get();
    let cum1 = dex.price1_cumulative_last.get();

    let new_cum0 = cum0.checked_add(cum0_delta).ok_or_else(|| err(ERR_OVERFLOW))?;
    let new_cum1 = cum1.checked_add(cum1_delta).ok_or_else(|| err(ERR_OVERFLOW))?;

    dex.price0_cumulative_last.set(new_cum0);
    dex.price1_cumulative_last.set(new_cum1);
    dex.block_timestamp_last.set(current_block);

    Ok(())
}

/// Core swap processing: invariant math, slippage protection, fee accounting and transfers.
///
/// @notice Internal helper used by entrypoints that perform a swap.
/// @dev Applies strict slippage checks, uses fully checked arithmetic, and
///      accrues protocol fees to the admin (treasury) accounting bucket.
fn process_swap(
    dex: &mut OakDEX,
    token0: Address,
    token1: Address,
    amount_in: U256,
    min_amount_out: U256,
) -> OakResult<U256> {
    // Input sanitization: validate addresses
    require_non_zero_address(token0)?;
    require_non_zero_address(token1)?;

    // Input sanitization: validate amounts
    if amount_in.is_zero() {
        return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
    }
    if min_amount_out.is_zero() {
        return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
    }

    // Emergency circuit breaker
    require_not_paused(dex)?;

    let sender = msg::sender();

    // Balance check: ensure user has enough tokens before attempting transfer.
    let user_balance = balance_of(token0, sender);
    if user_balance < amount_in {
        return Err(err(ERR_INSUFFICIENT_BALANCE));
    }

    // Snapshot pool reserves and fee configuration.
    let (pool_token0, pool_token1) = if token0 < token1 {
        (token0, token1)
    } else {
        (token1, token0)
    };
    // First, read reserves via a short-lived mutable borrow.
    let (reserve0, reserve1) = {
        let mut outer = dex.pools.setter(pool_token0);
        let pool = outer.setter(pool_token1);
        if !pool.initialized.get() {
            return Err(err(ERR_INVALID_TOKEN));
        }
        (pool.reserve0.get(), pool.reserve1.get())
    };
    let fee_bps = dex.protocol_fee_bps.get();

    // TWAP oracle: update cumulative prices at the beginning of every swap.
    update_oracle(dex, reserve0, reserve1)?;

    // Determine direction within the pool and compute amount_out.
    let (reserve_in, reserve_out) = if token0 == pool_token0 {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    let amount_out = get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps)?;

    // Strict slippage protection: revert if actual output below minimum.
    if amount_out < min_amount_out {
        return Err(err(ERR_SLIPPAGE_EXCEEDED));
    }

    // Compute fee split for analytics and treasury accounting.
    let (_effective_in, treasury_fee, lp_fee) = compute_fee_split(amount_in, fee_bps)?;

    // Update reserves under the standard CPMM assumption.
    let new_reserve_in = reserve_in
        .checked_add(amount_in)
        .ok_or_else(|| err(ERR_RESERVE0_OVERFLOW))?;

    let new_reserve_out = reserve_out
        .checked_sub(amount_out)
        .ok_or_else(|| err(ERR_INSUFFICIENT_LIQUIDITY))?;

    let min_liquidity = dex.min_liquidity.get();

    let (new_reserve0, new_reserve1) = if token0 == pool_token0 {
        (new_reserve_in, new_reserve_out)
    } else {
        (new_reserve_out, new_reserve_in)
    };

    if new_reserve0 < min_liquidity || new_reserve1 < min_liquidity {
        return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
    }

    {
        let mut outer = dex.pools.setter(pool_token0);
        let mut pool = outer.setter(pool_token1);
        pool.reserve0.set(new_reserve0);
        pool.reserve1.set(new_reserve1);
    }

    // Update analytics and accounting.
    let current_volume0 = dex.total_volume_token0.get();
    let current_volume1 = dex.total_volume_token1.get();

    let new_volume0 = current_volume0
        .checked_add(amount_in)
        .ok_or_else(|| err(ERR_VOLUME_OVERFLOW))?;

    let new_volume1 = current_volume1
        .checked_add(amount_out)
        .ok_or_else(|| err(ERR_VOLUME_OVERFLOW))?;

    dex.total_volume_token0.set(new_volume0);
    dex.total_volume_token1.set(new_volume1);

    let current_treasury_fees = dex.accrued_treasury_fees_token0.get();
    let current_lp_fees = dex.accrued_lp_fees_token0.get();

    let new_treasury_fees = current_treasury_fees
        .checked_add(treasury_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let new_lp_fees = current_lp_fees
        .checked_add(lp_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;

    dex.accrued_treasury_fees_token0.set(new_treasury_fees);
    dex.accrued_lp_fees_token0.set(new_lp_fees);

    // Gas-rebate placeholder: track a small portion of protocol fee for future gas rebates.
    let total_fee = treasury_fee
        .checked_add(lp_fee)
        .ok_or_else(|| err(ERR_OVERFLOW))?;
    let gas_rebate = total_fee
        .checked_mul(as_u256(GAS_REBATE_BPS))
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    if !gas_rebate.is_zero() {
        let acc = dex.accrued_gas_rebate_token0.get();
        let new_acc = acc
            .checked_add(gas_rebate)
            .ok_or_else(|| err(ERR_OVERFLOW))?;
        dex.accrued_gas_rebate_token0.set(new_acc);
    }

    // Transfer tokens: user -> contract (token0)
    let contract_addr = contract::address();
    safe_transfer_from(token0, sender, contract_addr, amount_in)?;

    // Transfer tokens: contract -> user (token1)
    safe_transfer(token1, sender, amount_out)?;

    // Emit swap event with fee split; admin wallet (treasury) can later withdraw
    // its share via `withdraw_treasury_fees`, which transfers directly to the
    // configured treasury address.
    emit_reveal_swap(sender, amount_in, amount_out, treasury_fee, lp_fee);

    Ok(amount_out)
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

    // If the effective fee rounds down to zero for this trade size,
    // treat it as "dust": the input is too small to produce a meaningful
    // output under the configured fee. In this case we return 0 instead
    // of reverting, so callers can decide whether to proceed.
    let total_fee = amount_in
        .checked_mul(fee_bps)
        .ok_or_else(|| err(ERR_OVERFLOW))?
        .checked_div(as_u256(FEE_DENOMINATOR))
        .ok_or_else(|| err(ERR_DIVISION_BY_ZERO))?;
    if !fee_bps.is_zero() && total_fee.is_zero() {
        return Ok(U256::ZERO);
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

/// Integer square root for `U256` (floor).
///
/// @notice Returns `floor(sqrt(x))` using a Babylonian-style iteration.
/// @dev This is used for initial LP token minting: sqrt(amount0 * amount1).
fn u256_sqrt(x: U256) -> U256 {
    if x.is_zero() {
        return U256::ZERO;
    }

    // Initial approximation: x/2 + 1
    let mut z = x;
    let mut y = (x >> 1) + U256::from(1u64);

    while y < z {
        z = y;
        y = (x.checked_div(y).unwrap_or(U256::ZERO) + y) >> 1;
    }

    z
}

/// Public contract functions implementation.
///
/// @notice Core entrypoints exposed to external callers.
/// @dev These methods operate on Stylus storage types defined in `state`.
///      This block is only compiled for on-chain (wasm32) builds; host
///      tests use the pure helper functions above instead.
#[cfg(all(not(test), target_arch = "wasm32"))]
#[public]
impl OakDEX {
    /// Create a new pool for a token pair.
    ///
    /// @notice Anyone can create a pool, but each canonical pair (token0, token1)
    ///         can only be initialized once.
    pub fn create_pool(&mut self, token_a: Address, token_b: Address) -> OakResult<()> {
        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        require_non_zero_address(token_a)?;
        require_non_zero_address(token_b)?;
        if token_a == token_b {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }

        // Canonical ordering
        let (token0, token1) = if token_a < token_b {
            token_a
        } else {
            token_b
        };
        let token1 = if token_a < token_b { token_b } else { token_a };

        // Access pool storage
        let mut outer = self.pools.setter(token0);
        let mut pool = outer.setter(token1);

        if pool.initialized.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_POOL_EXISTS));
        }

        // Initialize empty pool
        pool.reserve0.set(U256::ZERO);
        pool.reserve1.set(U256::ZERO);
        pool.lp_total_supply.set(U256::ZERO);
        pool.initialized.set(true);

        // Release guard
        unlock_reentrancy_guard(self);

        Ok(())
    }
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

        // Set initial total fee (0.5%) for the first month after launch.
        // Governance can later reduce this to `DEFAULT_FEE_BPS` via `set_fee`.
        self.protocol_fee_bps.set(as_u256(INITIAL_FEE));

        // Initialize analytics and fee accounting.
        self.total_volume_token0.set(U256::ZERO);
        self.total_volume_token1.set(U256::ZERO);
        self.accrued_treasury_fees_token0.set(U256::ZERO);
        self.accrued_lp_fees_token0.set(U256::ZERO);

        // TWAP oracle and gas-rebate placeholder.
        self.price0_cumulative_last.set(U256::ZERO);
        self.price1_cumulative_last.set(U256::ZERO);
        self.block_timestamp_last.set(U256::ZERO);
        self.accrued_gas_rebate_token0.set(U256::ZERO);

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
        require_not_paused(self)?;

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
    ///         accounting, CPMM pricing, strict slippage and deadline checks, and token transfers.
    /// @dev Part 2 of commit‑reveal flow, providing strong MEV protection.
    ///      Reverts with DeadlineExpired if block number > deadline, SlippageExceeded if output < min_amount_out.
    ///      Strict CEI: Lock acquired at start, released at end.
    ///
    /// # Arguments
    /// * `token0` - Address of token0 (input token)
    /// * `token1` - Address of token1 (output token)
    /// * `amount_in` - Input token amount
    /// * `salt` - Random salt used in commitment
    /// * `min_amount_out` - Minimum output tokens (strict slippage protection)
    /// * `deadline` - Block number after which the transaction must revert (deadline protection)
    pub fn reveal_swap(
        &mut self,
        token0: Address,
        token1: Address,
        amount_in: U256,
        salt: U256,
        min_amount_out: U256,
        deadline: U256,
    ) -> OakResult<()> {
        // CRITICAL: Re-entrancy guard acquired at the VERY BEGINNING
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

        // Emergency circuit breaker
        require_not_paused(self)?;

        // Deadline protection: revert if transaction is included after deadline (block number).
        let current_block = U256::from(block::number());
        if current_block > deadline {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_DEADLINE_EXPIRED));
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
        // current_block already set above for deadline check

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

        // Execute the actual swap with invariant checks, slippage protection,
        // and fee accounting. All math and external calls are performed inside
        // `process_swap`, which uses fully checked arithmetic and accrues
        // treasury fees for the admin wallet.
        let result = process_swap(self, token0, token1, amount_in, min_amount_out);
        let amount_out = match result {
            Ok(v) => v,
            Err(e) => {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        };

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

        require_not_paused(self)?;

        // Canonicalize token ordering for pool key.
        let (pool_token0, pool_token1) = if token0 < token1 {
            (token0, token1)
        } else {
            (token1, token0)
        };
        let outer = self.pools.setter(pool_token0);
        let pool = outer.setter(pool_token1);
        if !pool.initialized.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }

        // Map provided amounts into canonical order.
        let (amount0_c, amount1_c) = if token0 == pool_token0 {
            (amount0, amount1)
        } else {
            (amount1, amount0)
        };

        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();
        let total_supply = pool.lp_total_supply.get();

        // Compute LP tokens to mint, following Uniswap V2 semantics.
        // First liquidity: liquidity = sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
        // Subsequent: min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1)
        let liquidity = if total_supply.is_zero() {
            let product = amount0_c
                .checked_mul(amount1_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_LIQUIDITY_OVERFLOW)
                })?;
            let sqrt = u256_sqrt(product);
            let min_lp = as_u256(MINIMUM_LIQUIDITY);

            if sqrt <= min_lp {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }

            // Lock MINIMUM_LIQUIDITY LP tokens forever to the zero address.
            pool.lp_total_supply.set(min_lp);
            pool.lp_balances.setter(Address::ZERO).set(min_lp);

            sqrt.checked_sub(min_lp).ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?
        } else {
            // amount0 * totalSupply / reserve0
            let liquidity0 = amount0
                .checked_mul(total_supply)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_LIQUIDITY_OVERFLOW)
                })?
                .checked_div(reserve0)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?;

            let liquidity1 = amount1
                .checked_mul(total_supply)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_LIQUIDITY_OVERFLOW)
                })?
                .checked_div(reserve1)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?;

            let liq = if liquidity0 < liquidity1 {
                liquidity0
            } else {
                liquidity1
            };

            if liq.is_zero() {
                unlock_reentrancy_guard(self);
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }

            liq
        };

        // Transfer tokens from caller to contract before updating state.
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

        // Update reserves after successful transfer (canonical order).
        let new_reserve0 = reserve0
            .checked_add(amount0_c)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE0_OVERFLOW)
            })?;
        let new_reserve1 = reserve1
            .checked_add(amount1_c)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_RESERVE1_OVERFLOW)
            })?;

        pool.reserve0.set(new_reserve0);
        pool.reserve1.set(new_reserve1);

        // Mint LP tokens to provider (pool-specific).
        let current_total = pool.lp_total_supply.get();
        let new_total = current_total
            .checked_add(liquidity)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_total_supply.set(new_total);

        let current_balance = pool.lp_balances.setter(provider).get();
        let new_balance = current_balance
            .checked_add(liquidity)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_balances.setter(provider).set(new_balance);

        // LP token Transfer event (mint from zero).
        emit_lp_transfer(Address::ZERO, provider, liquidity);

        emit_add_liquidity(provider, amount0, amount1);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Remove liquidity from the pool.
    ///
    /// @notice Burns LP tokens and returns the underlying token0 and token1
    ///         to the provider in proportion to their share of total supply.
    /// @dev Uses the standard Uniswap V2 pro‑rata formula:
    ///      amount0 = lp_amount * reserve0 / totalSupply
    ///      amount1 = lp_amount * reserve1 / totalSupply
    pub fn remove_liquidity(
        &mut self,
        token0: Address,
        token1: Address,
        lp_amount: U256,
    ) -> OakResult<()> {
        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        require_non_zero_address(token0)?;
        require_non_zero_address(token1)?;

        if lp_amount.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_ZERO_AMOUNT));
        }

        require_not_paused(self)?;

        // Canonical pool key
        let (pool_token0, pool_token1) = if token0 < token1 {
            (token0, token1)
        } else {
            (token1, token0)
        };
        let outer = self.pools.setter(pool_token0);
        let pool = outer.setter(pool_token1);
        if !pool.initialized.get() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_TOKEN));
        }

        let provider = msg::sender();
        let total_supply = pool.lp_total_supply.get();
        if total_supply.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Check provider balance
        let balance = pool.lp_balances.setter(provider).get();
        if lp_amount > balance {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();

        // Pro-rata amounts to withdraw (canonical)
        let amount0_c = reserve0
            .checked_mul(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?
            .checked_div(total_supply)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;
        let amount1_c = reserve1
            .checked_mul(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_OVERFLOW)
            })?
            .checked_div(total_supply)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_DIVISION_BY_ZERO)
            })?;
        if amount0_c.is_zero() || amount1_c.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
        }

        // Map canonical amounts back to user token order
        let (amount0, amount1) = if token0 == pool_token0 {
            (amount0_c, amount1_c)
        } else {
            (amount1_c, amount0_c)
        };

        // Update LP supply and balances
        let new_total = total_supply
            .checked_sub(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_total_supply.set(new_total);

        let new_balance = balance
            .checked_sub(lp_amount)
            .ok_or_else(|| {
                unlock_reentrancy_guard(self);
                err(ERR_LIQUIDITY_OVERFLOW)
            })?;
        pool.lp_balances.setter(provider).set(new_balance);

        // Update reserves after withdrawal (canonical)
        let (new_reserve0, new_reserve1) = if token0 == pool_token0 {
            let new_r0 = reserve0
                .checked_sub(amount0_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            let new_r1 = reserve1
                .checked_sub(amount1_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            (new_r0, new_r1)
        } else {
            let new_r0 = reserve0
                .checked_sub(amount1_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            let new_r1 = reserve1
                .checked_sub(amount0_c)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_INSUFFICIENT_LIQUIDITY)
                })?;
            (new_r0, new_r1)
        };

        pool.reserve0.set(new_reserve0);
        pool.reserve1.set(new_reserve1);

        // Transfer underlying tokens back to the provider
        if let Err(e) = safe_transfer(token0, provider, amount0) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }
        if let Err(e) = safe_transfer(token1, provider, amount1) {
            unlock_reentrancy_guard(self);
            return Err(e);
        }

        // LP token Transfer event (burn to zero).
        emit_lp_transfer(provider, Address::ZERO, lp_amount);

        // Re-entrancy guard release
        unlock_reentrancy_guard(self);

        Ok(())
    }

    /// Compute expected output amounts along a multi-hop path.
    ///
    /// @notice Pure view helper used by router/frontends to estimate
    ///         final amount_out for a given path, taking per-pool fees
    ///         into account.
    pub fn get_amounts_out(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> OakResult<Vec<U256>> {
        if path.len() < 2 {
            return Err(err(ERR_INVALID_PATH));
        }
        if amount_in.is_zero() {
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }

        let mut amounts = Vec::with_capacity(path.len());
        amounts.push(amount_in);
        let mut current_in = amount_in;

        // Single global fee setting for now.
        let fee_bps = self.protocol_fee_bps.get();

        for i in 0..(path.len() - 1) {
            let input = path[i];
            let output = path[i + 1];

            if input == output {
                return Err(err(ERR_INVALID_PATH));
            }

            // Canonical pair ordering.
            let (token0, token1) = if input < output {
                (input, output)
            } else {
                (output, input)
            };

            let outer = self.pools.getter(token0);
            let pool = outer.getter(token1);
            if !pool.initialized.get() {
                return Err(err(ERR_INVALID_TOKEN));
            }

            let reserve0 = pool.reserve0.get();
            let reserve1 = pool.reserve1.get();
            if reserve0.is_zero() || reserve1.is_zero() {
                return Err(err(ERR_INSUFFICIENT_LIQUIDITY));
            }

            // Direction within this pool.
            let (reserve_in, reserve_out) = if input == token0 {
                (reserve0, reserve1)
            } else {
                (reserve1, reserve0)
            };

            let out = get_amount_out_with_fee(current_in, reserve_in, reserve_out, fee_bps)?;
            if out.is_zero() {
                return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
            }

            amounts.push(out);
            current_in = out;
        }

        Ok(amounts)
    }

    /// Get current reserves for a given token pair.
    ///
    /// @notice Returns reserves in the same order as the input tokens.
    pub fn get_reserves(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> OakResult<(U256, U256)> {
        require_non_zero_address(token_a)?;
        require_non_zero_address(token_b)?;
        if token_a == token_b {
            return Err(err(ERR_INVALID_TOKEN));
        }

        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        let outer = self.pools.getter(token0);
        let pool = outer.getter(token1);
        if !pool.initialized.get() {
            return Err(err(ERR_INVALID_TOKEN));
        }

        let reserve0 = pool.reserve0.get();
        let reserve1 = pool.reserve1.get();

        // Map back to caller's token order
        let (out0, out1) = if token_a == token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };

        Ok((out0, out1))
    }

    /// Router-style multi-hop swap: exact input, minimum output.
    ///
    /// @notice Swaps an exact amount of the first token in `path` for as much
    ///         as possible of the last token, going through intermediate pools.
    /// @dev For now the recipient `to` must be the caller (`msg::sender`),
    ///      since `process_swap` always transfers to `sender`.
    pub fn swap_exact_tokens_for_tokens(
        &mut self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> OakResult<Vec<U256>> {
        // Re-entrancy guard
        lock_reentrancy_guard(self)?;

        // Basic input validation
        if amount_in.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_INPUT_AMOUNT));
        }
        if amount_out_min.is_zero() {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }
        if path.len() < 2 {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_PATH));
        }

        // Recipient must be non-zero and, в текущей версии, совпадать с sender.
        let sender = msg::sender();
        if to == Address::ZERO || to != sender {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INVALID_ADDRESS));
        }

        require_not_paused(self)?;

        // Deadline based on block timestamp
        let now = U256::from(block::timestamp());
        if now > deadline {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_EXPIRED));
        }

        // Compute expected amounts along the path
        let amounts = match self.get_amounts_out(amount_in, path.clone()) {
            Ok(v) => v,
            Err(e) => {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        };

        let final_out = *amounts.last().unwrap_or(&U256::ZERO);
        if final_out < amount_out_min {
            unlock_reentrancy_guard(self);
            return Err(err(ERR_INSUFFICIENT_OUTPUT_AMOUNT));
        }

        // Execute the multi-hop swaps sequentially.
        // At каждом шаге process_swap:
        // - списывает amount_in хопа с sender в контракт
        // - отправляет amount_out хопа обратно sender'у
        // - обновляет резервы пула (через PoolData)
        for i in 0..(path.len() - 1) {
            let token_in = path[i];
            let token_out = path[i + 1];
            let hop_in = amounts[i];
            let hop_min_out = amounts[i + 1]; // строгое ожидание по расчёту get_amounts_out

            if let Err(e) = process_swap(self, token_in, token_out, hop_in, hop_min_out) {
                unlock_reentrancy_guard(self);
                return Err(e);
            }
        }

        // Release re-entrancy guard
        unlock_reentrancy_guard(self);

        Ok(amounts)
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
    #[cfg(all(not(test), target_arch = "wasm32"))]
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

        require_not_paused(self)?;

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

            // Gas-rebate placeholder: track a small portion of protocol fee (token0 side).
            let total_fee0 = treasury_fee0
                .checked_add(lp_fee0)
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?;
            let gas_rebate = total_fee0
                .checked_mul(as_u256(GAS_REBATE_BPS))
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?
                .checked_div(as_u256(FEE_DENOMINATOR))
                .ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_DIVISION_BY_ZERO)
                })?;
            if !gas_rebate.is_zero() {
                let acc = self.accrued_gas_rebate_token0.get();
                let new_acc = acc.checked_add(gas_rebate).ok_or_else(|| {
                    unlock_reentrancy_guard(self);
                    err(ERR_OVERFLOW)
                })?;
                self.accrued_gas_rebate_token0.set(new_acc);
            }
        }

        // Emit FlashSwap event
        emit_flash_swap(borrower, token0, token1, amount0_out, amount1_out, fee0, fee1);

        // CRITICAL: Release re-entrancy guard at the VERY END
        // This must be the last operation before return
        unlock_reentrancy_guard(self);

        Ok(())
    }
}

/// Host/test stub for `flash_swap`.
///
/// Compiled for non-wasm32 targets (including `cargo test`) to keep
/// the public interface of `OakDEX` intact without pulling in Stylus
/// call machinery. The real implementation above is only enabled for
/// on-chain (wasm32) builds.
#[cfg(any(test, not(target_arch = "wasm32")))]
impl OakDEX {
    pub fn flash_swap(
        &mut self,
        _token0: Address,
        _token1: Address,
        _amount0_out: U256,
        _amount1_out: U256,
        _data: Vec<u8>,
    ) -> OakResult<()> {
        Err(err(ERR_PAUSED))
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

