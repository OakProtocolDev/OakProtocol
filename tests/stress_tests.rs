//! Stress and adversarial scenario tests for Oak Protocol (Oak Shield).
//!
//!"The Greedy Trader", "The Flash-Loan Attack", "The Re-entrancy Trap",
//!and "Dust and Limits" are modeled at the math/accounting level,
//!reusing the same fee and invariant logic as the on-chain contract.

use oak_protocol::{
    constants::{as_u256, FEE_DENOMINATOR, INITIAL_FEE},
    errors::ERR_REENTRANT_CALL,
    logic::{compute_fee_split, get_amount_out_with_fee},
};

use stylus_sdk::alloy_primitives::U256;

// -----------------------------------------------------------------------------
// 1. "The Greedy Trader" – 100 sequential swaps, 0.5% fee, no rounding loss
// -----------------------------------------------------------------------------

#[test]
fn greedy_trader_fee_accounting_is_exact() {
    let fee_bps = as_u256(INITIAL_FEE); // 0.5% in basis points

    // Start with a symmetric pool for simplicity.
    let mut reserve_in = U256::from(1_000_000u64);
    let mut reserve_out = U256::from(1_000_000u64);

    // Simulate 100 sequential swaps of 10_000 units each.
    let amount_per_swap = U256::from(10_000u64);
    let swaps = 100u64;

    let mut total_input = U256::ZERO;
    let mut total_treasury_fees = U256::ZERO;
    let mut total_lp_fees = U256::ZERO;
    let mut total_buyback_fees = U256::ZERO;

    for _ in 0..swaps {
        // Compute swap output under current reserves
        let amount_out =
            get_amount_out_with_fee(amount_per_swap, reserve_in, reserve_out, fee_bps).unwrap();

        // Compute fee split for this swap (60/20/20: LP, Treasury, Buyback)
        let (_effective_in, treasury_fee, lp_fee, buyback_fee) =
            compute_fee_split(amount_per_swap, fee_bps).unwrap();

        // Update cumulative accounting
        total_input = total_input + amount_per_swap;
        total_treasury_fees = total_treasury_fees + treasury_fee;
        total_lp_fees = total_lp_fees + lp_fee;
        total_buyback_fees = total_buyback_fees + buyback_fee;

        // Update reserves as in the contract: reserve_in += amount_in, reserve_out -= amount_out
        reserve_in = reserve_in + amount_per_swap;
        reserve_out = reserve_out - amount_out;
    }

    // Total fee should be exactly 0.5% of aggregate input (floor-div).
    let expected_total_fee = total_input * fee_bps / as_u256(FEE_DENOMINATOR);
    let accounted_total_fee = total_treasury_fees + total_lp_fees + total_buyback_fees;

    assert_eq!(
        accounted_total_fee, expected_total_fee,
        "Total accounted fees (treasury + LP + buyback) must equal 0.5% of total input with no rounding loss"
    );
}

// -----------------------------------------------------------------------------
// 2. "The Flash-Loan Attack" – returning less than required must violate k-invariant
// -----------------------------------------------------------------------------

#[test]
fn flash_loan_attack_violates_k_invariant() {
    let fee_bps = as_u256(INITIAL_FEE); // use the same 0.5% launch fee

    // Initial reserves
    let reserve0 = U256::from(1_000_000u64);
    let reserve1 = U256::from(2_000_000u64);
    let k_before = reserve0 * reserve1;

    // Borrow 100_000 of token0 via flash swap
    let amount0_out = U256::from(100_000u64);

    // Contract-side required repayment (borrowed + fee)
    let required_fee = amount0_out * fee_bps / as_u256(FEE_DENOMINATOR);
    let required_repayment = amount0_out + required_fee;

    // Attacker tries to repay slightly less than required (1 wei short)
    let malicious_repayment = required_repayment - U256::from(1u64);

    // New reserves if the attack "succeeded"
    let reserve0_after = reserve0 - amount0_out + malicious_repayment;
    let reserve1_after = reserve1;
    let k_after = reserve0_after * reserve1_after;

    // On-chain logic enforces: k_after >= k_before * (1 + fee_bps/denom)
    let fee_multiplier = as_u256(FEE_DENOMINATOR) + fee_bps;
    let k_min = k_before * fee_multiplier / as_u256(FEE_DENOMINATOR);

    assert!(
        k_after < k_min,
        "Underpaying flash loan must drive k' below required minimum, triggering ERR_INSUFFICIENT_LIQUIDITY on-chain"
    );
}

// -----------------------------------------------------------------------------
// 3. "The Re-entrancy Trap" – lock must block recursive entry
// -----------------------------------------------------------------------------

#[test]
fn reentrancy_trap_lock_blocks_second_entry() {
    // Model of the lock flag used in OakDEX.
    let mut locked = false;

    // First call acquires the lock successfully.
    assert!(
        acquire_lock(&mut locked).is_ok(),
        "first entry must acquire lock"
    );
    assert!(locked, "lock flag should be set after first entry");

    // Second (re-entrant) call should fail with ERR_REENTRANT_CALL semantics.
    let second = acquire_lock(&mut locked);
    assert!(
        second.is_err(),
        "second (re-entrant) entry must be rejected"
    );
    assert_eq!(
        second.err().unwrap(),
        ERR_REENTRANT_CALL.to_vec(),
        "error must match re-entrancy guard"
    );

    // Release lock to restore normal state.
    release_lock(&mut locked);
    assert!(!locked, "lock must be cleared after release");
}

fn acquire_lock(locked: &mut bool) -> Result<(), Vec<u8>> {
    if *locked {
        return Err(ERR_REENTRANT_CALL.to_vec());
    }
    *locked = true;
    Ok(())
}

fn release_lock(locked: &mut bool) {
    *locked = false;
}

// -----------------------------------------------------------------------------
// 4. "Dust and Limits" – extremal amounts for checked math
// -----------------------------------------------------------------------------

#[test]
fn dust_and_limits_are_safely_handled() {
    let fee_bps = as_u256(INITIAL_FEE);

    // Extremely small "lampart"-like amount.
    let dust_amount = U256::from(1u64);
    let reserve_in = U256::from(1_000_000u64);
    let reserve_out = U256::from(2_000_000u64);

    let out_dust =
        get_amount_out_with_fee(dust_amount, reserve_in, reserve_out, fee_bps).unwrap();
    // With floor rounding and fee, we either get 0 or a very small positive amount; both must be safe.
    assert!(
        out_dust == U256::ZERO || out_dust < dust_amount,
        "dust swap must not overflow and must respect fee/rounding semantics"
    );

    // Now test an upper-bound scenario using large values that still keep
    // *all intermediate products* safely within U256. We avoid U256::MAX or
    // u128::MAX here because the CPMM formulas multiply reserves and inputs,
    // which would overflow any fixed-size integer type.
    let big_reserve_in = U256::from(1_000_000_000_000_000_000u64); // 1e18
    let big_reserve_out = U256::from(2_000_000_000_000_000_000u64); // 2e18
    let big_amount_in = U256::from(500_000_000_000_000_000u64); // 5e17

    let out_big =
        get_amount_out_with_fee(big_amount_in, big_reserve_in, big_reserve_out, fee_bps)
            .expect("checked math must not overflow on upper-bound scenario");

    // Invariant sanity: output must be non-zero and strictly less than reserve_out.
    assert!(out_big > U256::ZERO, "large swap must produce non-zero output");
    assert!(
        out_big < big_reserve_out,
        "output must not exceed available reserves"
    );

    // Total fee for big_amount_in must be consistent with fee_bps (no overflow).
    let (_effective_in, treasury_fee, lp_fee, buyback_fee) =
        compute_fee_split(big_amount_in, fee_bps).expect("fee split must not overflow");
    let total_fee = treasury_fee + lp_fee + buyback_fee;
    let expected_fee = big_amount_in * fee_bps / as_u256(FEE_DENOMINATOR);
    assert_eq!(
        total_fee, expected_fee,
        "fee split must remain exact even at large scales"
    );
}

