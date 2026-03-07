# Security Audit (Audit-Ready Summary)

This document is the **audit-ready** summary for Oak Stylus Trading Engine: threat model, in-house testing references, and formal verification plan. Full technical details are in the root [SECURITY_AUDIT.md](../SECURITY_AUDIT.md).

---

## 1. Threat Model

### 1.1 Attack Vectors and Mitigations

| Vector | Description | Mitigation |
|--------|-------------|------------|
| **Price Manipulation** | Attacker moves spot or TWAP price to extract value (e.g. same-block dump, oracle manipulation). | (1) TWAP updated at **start** of swap (before reserve change); (2) **Single-trade cap** (`MAX_TRADE_RESERVE_BPS`, e.g. 10% of reserve); (3) **Circuit breaker** on price impact ≥20%; (4) **TWAP deviation circuit breaker**: if TWAP price changes >15% in one block, contract is **paused** and circuit breaker triggered (`src/engine/emergency.rs` `check_price_deviation`). |
| **Flash Loan Attack** | Large uncollateralized borrow used to drain pool or distort oracle. | (1) Single-trade cap limits size per swap; (2) Circuit breaker (impact + TWAP deviation) stops extreme moves; (3) Flash **swap** (borrow from pool) enforces `k' >= k * (1 + fee)` after callback. No external flash-loan oracle dependency. |
| **Governance Hijack** | Attacker gains owner or admin rights and changes fee, withdraws treasury, or pauses maliciously. | (1) **Two-step ownership** with delay (`OWNER_TRANSFER_DELAY_BLOCKS`); (2) **TimelockController**: critical admin actions (set_fee, withdraw_treasury_fees, set_buyback_wallet, etc.) should be executed via **queue → 24h delay → execute** (`src/timelock.rs`). Only addresses with `TIMELOCK_ADMIN_ROLE` or `DEFAULT_ADMIN_ROLE` can queue; anyone can execute after delay. (3) Events for all admin actions (SetFee, WithdrawTreasuryFees, PendingOwnerSet, OwnerChanged, etc.). |

### 1.2 Governance: Timelock (24h) for Admin-Only Functions

All sensitive admin functions (conceptually `#[admin_only]`) are protected by:

- **Immediate owner checks**: `only_owner(owner)` for `set_fee`, `withdraw_treasury_fees`, `set_buyback_wallet`, `trigger_circuit_breaker`, `clear_circuit_breaker`, `pause`, `unpause`, `set_pending_owner`, and related setters.
- **Recommended execution path**: For parameter changes (fee, treasury withdrawal, buyback wallet), the **TimelockController** should be used: queue an operation (target = contract, calldata = set_fee/withdraw/…) with **delay ≥ 24h** (`TIMELOCK_MIN_DELAY_BLOCKS = 86400`). After the delay, anyone can call `execute_operation`. This mirrors OpenZeppelin-style TimelockController and prevents a single compromised key from acting in one block.

Implementation: `src/timelock.rs` — `queue_operation`, `get_operation_ready_block`, `execute_operation`; state: `timelock_ready_block: StorageMap<operation_id, ready_block>`.

---

## 2. In-house Testing

- **Test suite location**: Rust unit and integration tests in the repo (e.g. `tests/integration_tests.rs`, and any `#[cfg(test)]` modules in `src/`).
- **Running tests**: From repo root, `cargo test` (optionally `cargo test --no-default-features` for no_std where applicable). CI or local runs produce test logs; link your CI artifact or a recent run summary here, e.g.:
  - **Example**: “Test run: `cargo test 2>&1` — see [GitHub Actions / CI link] or local `cargo test` output.”
- **Scope**: CPMM math, commit-reveal, reentrancy guard, circuit breaker, TWAP update, access control, order/position lifecycle. See [IN_HOUSE_TESTING.md](IN_HOUSE_TESTING.md) for scope and [SECURITY_AUDIT.md](../SECURITY_AUDIT.md) for threat model details.

**Link to test logs**: Replace with your actual CI URL or document: “In-house test logs are available from the latest `cargo test` run in the repository (see `tests/` and CI workflow).”

---

## 3. Formal Verification (Plan)

We plan to formally verify the following in a future phase:

| Component | Scope | Notes |
|-----------|--------|--------|
| **CPMM / math_core** | `get_amount_out_with_fee`, `get_amount_in_with_fee` (and equivalent used in router) | Invariants: output bounds, monotonicity in amount_in, fee deduction consistency. |
| **Reserve and fee accounting** | Reserve updates in `process_swap_from_to_with_fee`; `compute_fee_split`; invariant `balance(contract, token) >= sum(pool reserves) + treasury_balance[token] + buyback_balance[token]`. | Prove no reserve inflation and no over-withdrawal. |
| **TWAP and emergency** | `update_oracle` cumulative price math; `check_price_deviation` (deviation threshold and pause/circuit breaker side effects). | Show that deviation >15% implies trigger and that oracle math is consistent. |

Tools and approach (to be chosen): Kani, Creusot, or external (e.g. Certora) for specified invariants; focus on math and accounting first, then access control and timelock guarantees.

---

## 4. Events for The Graph

Indexed events for subgraph and monitoring:

- **SwapExecuted(sender indexed, tokenIn indexed, tokenOut indexed, amountIn, amountOut)** — `emit_swap_executed` in `src/events.rs`.
- **PositionOpened(positionId, owner indexed, baseToken indexed, quoteToken indexed, size, entryPrice)** — `emit_open_position` in `src/events.rs`.
- **EmergencyTriggered(reason indexed)** — `emit_emergency_triggered` in `src/events.rs` (e.g. reason = keccak256("TWAP_DEVIATION")).

Additional events (CircuitBreakerTriggered/Cleared, PoolCreated, PendingOwnerSet, OwnerChanged, WithdrawTreasuryFees, etc.) are documented in the root SECURITY_AUDIT.md.
