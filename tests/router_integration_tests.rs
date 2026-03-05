//! Integration-style tests for Oak Shield router math (multi-hop swaps).
//!
//! NOTE: These tests run on the host and focus on the CPMM math and
//! reserve updates for a 2-hop path USDC -> OAK -> SOL. We model pools
//! in-memory and use `get_amount_out_with_fee` to ensure the router
//! logic can safely chain multiple pools.

use oak_protocol::{
    constants::{as_u256, DEFAULT_FEE_BPS, FEE_DENOMINATOR},
    logic::get_amount_out_with_fee,
};

use stylus_sdk::alloy_primitives::{Address, U256};

/// Helper to build a deterministic mock address for testing.
fn addr(id: u8) -> Address {
    Address::from([id; 20])
}

#[test]
fn two_hop_swap_usdc_to_sol_math_consistent() {
    // Fee configuration (use DEFAULT_FEE_BPS for model)
    let fee_bps = as_u256(DEFAULT_FEE_BPS);

    // Pool 1: USDC/OAK
    // 100_000 USDC / 10_000 OAK (достаточно большие резервы, чтобы fee не обнулился)
    let reserve_usdc_1 = U256::from(100_000u64);
    let mut reserve_oak_1 = U256::from(10_000u64);

    // Pool 2: OAK/SOL
    // 10_000 OAK / 500 SOL
    let mut reserve_oak_2 = U256::from(10_000u64);
    let reserve_sol_2_initial = U256::from(500u64);
    let mut reserve_sol_2 = reserve_sol_2_initial;

    // User balances (simplified model)
    let user_usdc_initial = U256::from(20_000u64);
    let mut user_usdc = user_usdc_initial;
    let mut user_oak = U256::ZERO;
    let mut user_sol = U256::ZERO;

    // User wants to swap 10_000 USDC -> SOL via path [USDC, OAK, SOL]
    let amount_in_usdc = U256::from(10_000u64);

    // --- First hop: USDC -> OAK in pool 1 ---
    let amount_oak_out = get_amount_out_with_fee(
        amount_in_usdc,
        reserve_usdc_1,
        reserve_oak_1,
        fee_bps,
    )
    .expect("first hop amount_out must succeed");

    assert!(
        amount_oak_out > U256::ZERO,
        "first hop must produce non-zero OAK"
    );

    // Update pool 1 reserves (CPMM semantics) – we track только OAK для проверок
    reserve_oak_1 = reserve_oak_1 - amount_oak_out;

    // Update user balances
    user_usdc = user_usdc - amount_in_usdc;
    user_oak = user_oak + amount_oak_out;

    // --- Second hop: OAK -> SOL in pool 2 ---
    let amount_sol_out =
        get_amount_out_with_fee(amount_oak_out, reserve_oak_2, reserve_sol_2, fee_bps)
            .expect("second hop amount_out must succeed");

    assert!(
        amount_sol_out > U256::ZERO,
        "second hop must produce non-zero SOL"
    );

    // Update pool 2 reserves
    reserve_oak_2 = reserve_oak_2 + amount_oak_out;
    reserve_sol_2 = reserve_sol_2 - amount_sol_out;

    // Update user balances
    user_oak = user_oak - amount_oak_out;
    user_sol = user_sol + amount_sol_out;

    // --- Assertions ---

    // 1) User USDC decreased by exactly 100
    assert_eq!(
        user_usdc,
        user_usdc_initial - amount_in_usdc,
        "user USDC balance must decrease by exact input amount"
    );

    // 2) User SOL increased by the final routed amount
    assert_eq!(
        user_sol, amount_sol_out,
        "user SOL balance must equal final routed amount"
    );

    // 3) Intermediate token OAK moved from pool1 -> pool2 via user
    //    - pool1: OAK reserve decreased
    //    - pool2: OAK reserve increased
    assert!(
        reserve_oak_1 < U256::from(10_000u64),
        "pool1 OAK reserve must decrease after USDC->OAK swap"
    );
    assert!(
        reserve_oak_2 > U256::from(10_000u64),
        "pool2 OAK reserve must increase after OAK->SOL swap"
    );

    // 4) No "revert": both hops produced positive output and math did not overflow.
    //    As an extra sanity check, recompute total fee fraction for the first hop.
    let total_fee_first = amount_in_usdc * fee_bps / as_u256(FEE_DENOMINATOR);
    assert!(
        total_fee_first <= amount_in_usdc,
        "total fee must not exceed input amount"
    );
}

