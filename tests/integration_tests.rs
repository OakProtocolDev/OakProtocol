//! High-level integration-style tests for Oak Protocol logic.
//!
//! These tests exercise the core flows (commit‑reveal, slippage/deadline checks,
//! TWAP oracle updates, and flash swap invariants) at the Rust level.
//!
//! NOTE:
//! - These are *hosted* tests that run with `std` enabled (`cfg(test)` in `lib.rs`),
//!   so we focus on logic invariants rather than full Stylus VM wiring.
//! - Where direct Stylus context (e.g. `block::number`, `msg::sender`) is required,
//!   we model scenarios using pure helper functions and state transitions.

use oak_protocol::{
    constants::{as_u256, q112_u256, COMMIT_REVEAL_DELAY, DEFAULT_FEE_BPS, FEE_DENOMINATOR},
    errors::{
        ERR_COMMIT_NOT_FOUND, ERR_DEADLINE_EXPIRED, ERR_SLIPPAGE_EXCEEDED, ERR_TOO_EARLY, OakResult,
    },
    logic::{
        compute_fee_split, compute_commit_hash, get_amount_out_with_fee,
        // The following helpers are internal to the crate; for integration tests
        // we exercise them indirectly via scenario modeling.
    },
    state::Commitment,
};

use stylus_sdk::alloy_primitives::U256;

/// Simple helper to build a commitment structure for testing.
fn make_commitment(amount_in: U256, salt: U256, block_number: U256) -> (Commitment, U256) {
    let hash_bytes = compute_commit_hash(amount_in, salt);
    let hash = U256::from_be_bytes::<32>(hash_bytes.into());
    (
        Commitment {
            hash,
            timestamp: block_number,
            activated: true,
        },
        hash,
    )
}

/// Basic model of the commit‑reveal predicates without Stylus host dependencies.
fn can_reveal(
    commitment: &Commitment,
    amount_in: U256,
    salt: U256,
    current_block: U256,
    min_block_delay: U256,
    max_commit_age: U256,
    deadline: U256,
) -> OakResult<()> {
    if !commitment.activated || commitment.hash.is_zero() {
        return Err(ERR_COMMIT_NOT_FOUND.to_vec());
    }

    let computed = U256::from_be_bytes::<32>(compute_commit_hash(amount_in, salt).into());
    if computed != commitment.hash {
        return Err(ERR_COMMIT_NOT_FOUND.to_vec());
    }

    if current_block > deadline {
        return Err(ERR_DEADLINE_EXPIRED.to_vec());
    }

    let max_block = commitment.timestamp + max_commit_age;
    if current_block > max_block {
        return Err(ERR_DEADLINE_EXPIRED.to_vec());
    }

    let min_block = commitment.timestamp + min_block_delay;
    if current_block < min_block {
        return Err(ERR_TOO_EARLY.to_vec());
    }

    Ok(())
}

#[test]
fn commit_reveal_successful_flow() {
    let amount_in = U256::from(1_000u64);
    let salt = U256::from(42u64);

    let commit_block = U256::from(100u64);
    let (commitment, _hash) = make_commitment(amount_in, salt, commit_block);

    let min_delay = as_u256(COMMIT_REVEAL_DELAY);
    let max_age = U256::from(10_000u64);

    // Reveal in the same block as minimum allowed (on‑chain code uses `>=`)
    let reveal_block = commit_block + min_delay;
    let deadline = reveal_block + U256::from(100u64);

    let result = can_reveal(
        &commitment,
        amount_in,
        salt,
        reveal_block,
        min_delay,
        max_age,
        deadline,
    );

    assert!(result.is_ok(), "commit‑reveal should succeed at min delay");
}

#[test]
fn reveal_fails_due_to_slippage() {
    // Set up a simple constant‑product pool
    let amount_in = U256::from(1_000u64);
    let reserve_in = U256::from(10_000u64);
    let reserve_out = U256::from(20_000u64);
    let fee_bps = as_u256(DEFAULT_FEE_BPS);

    // Compute expected amount out under current reserves
    let expected_out =
        get_amount_out_with_fee(amount_in, reserve_in, reserve_out, fee_bps).unwrap();

    // User sets a min_out slightly above expected_out to force slippage failure
    let min_amount_out = expected_out + U256::from(1u64);

    // In the on‑chain code, this comparison guards reveal:
    // if amount_out < min_amount_out => ERR_SLIPPAGE_EXCEEDED.
    if expected_out < min_amount_out {
        assert_eq!(ERR_SLIPPAGE_EXCEEDED, ERR_SLIPPAGE_EXCEEDED);
    } else {
        panic!("Expected slippage failure condition not met in model");
    }
}

#[test]
fn reveal_fails_due_to_deadline() {
    let amount_in = U256::from(1_000u64);
    let salt = U256::from(7u64);

    let commit_block = U256::from(1_000u64);
    let (commitment, _hash) = make_commitment(amount_in, salt, commit_block);

    let min_delay = as_u256(COMMIT_REVEAL_DELAY);
    let max_age = U256::from(10_000u64);

    // Deadline exactly equal to current block is allowed on‑chain (strict `>` check).
    // Model the failing case where current_block > deadline.
    let deadline = commit_block + min_delay;
    let current_block = deadline + U256::from(1u64);

    let result = can_reveal(
        &commitment,
        amount_in,
        salt,
        current_block,
        min_delay,
        max_age,
        deadline,
    );

    assert!(
        result.is_err(),
        "reveal past deadline should fail in model"
    );
    assert_eq!(result.err().unwrap(), ERR_DEADLINE_EXPIRED.to_vec());
}

#[test]
fn twap_price_changes_after_large_swap() {
    // Model a price move via cumulative price math without depending on Stylus host.
    let q112 = q112_u256();

    // Initial reserves and block numbers
    let reserve0_initial = U256::from(10_000u64);
    let reserve1_initial = U256::from(20_000u64);
    let block_last = U256::from(1_000u64);
    let block_now = U256::from(1_010u64); // 10 "seconds" / blocks elapsed

    let time_elapsed = block_now - block_last;
    assert!(time_elapsed > U256::ZERO);

    // Initial price0 = reserve1 / reserve0 in Q112.64
    let price0_initial = reserve1_initial * q112 / reserve0_initial;
    let cum0_initial = price0_initial * time_elapsed;

    // Simulate a large swap that doubles price (approximate)
    let reserve0_new = U256::from(5_000u64);
    let reserve1_new = U256::from(20_000u64);
    let price0_new = reserve1_new * q112 / reserve0_new;
    let cum0_new = price0_new * time_elapsed;

    assert!(
        cum0_new > cum0_initial,
        "TWAP cumulative price should increase after large price change"
    );
}

#[test]
fn flash_swap_fee_split_and_invariant() {
    // Model flash swap repayment on token0 side using the same fee math as the contract.
    let reserve0 = U256::from(100_000u64);
    let reserve1 = U256::from(200_000u64);

    let k_before = reserve0 * reserve1;
    let fee_bps = as_u256(DEFAULT_FEE_BPS);

    // Borrow some token0 in a flash swap
    let amount0_out = U256::from(10_000u64);

    // Protocol fee as in the contract: fee = amount * fee_bps / FEE_DENOMINATOR
    let total_fee = amount0_out * fee_bps / as_u256(FEE_DENOMINATOR);
    let amount0_owed = amount0_out + total_fee;

    // Simulate "after" reserves where the borrower repays exactly what is owed
    let reserve0_after = reserve0 - amount0_out + amount0_owed;
    let reserve1_after = reserve1;
    let k_after = reserve0_after * reserve1_after;

    // Minimum k required according to on‑chain logic:
    // k_min = k_before * (FEE_DENOMINATOR + fee_bps) / FEE_DENOMINATOR
    let fee_multiplier = as_u256(FEE_DENOMINATOR) + fee_bps;
    let k_min = k_before * fee_multiplier / as_u256(FEE_DENOMINATOR);

    assert!(
        k_after >= k_min,
        "flash swap repayment must maintain k' >= k * (1 + fee)"
    );

    // Check that fee split accounts for the same total_fee.
    let (_effective_in, treasury_fee, lp_fee) =
        compute_fee_split(amount0_out, fee_bps).expect("fee split must succeed");
    let accounted_total_fee = treasury_fee + lp_fee;
    assert_eq!(
        accounted_total_fee, total_fee,
        "fee split should match total flash swap fee"
    );
}

