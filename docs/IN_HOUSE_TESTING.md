# In-house Unit & Integration Testing

This document describes the **in-house testing** performed on the Oak Stylus Trading Engine. There is **no external security audit report**; claims are limited to unit and integration tests and internal review.

## Scope

- **Unit tests**: CPMM math, commit hash roundtrip, fee accounting, access control helpers.
- **Integration-style tests**: Swap flow, add/remove liquidity, reentrancy guard, circuit breaker, order/position lifecycle.
- **No external audit**: No third-party security audit has been conducted. Mainnet deployment should follow an external audit and a clear threat model.

## Test Areas

| Area | What is tested |
|------|----------------|
| **Reentrancy** | Global lock on swap, add_liquidity, withdraw_treasury_fees, flash_swap; CEI (effects before external calls). |
| **Overflow / math** | Checked arithmetic in `get_amount_out_with_fee`, reserve updates, fee accrual. |
| **Access control** | PAUSER_ROLE for pause/unpause; owner-only for set_fee, withdraw_treasury_fees, set_buyback_wallet; two-step ownership. |
| **Commit-reveal** | Hash derivation, delay and expiration checks, state cleared before execution. |
| **Orders / positions** | place_order, execute_order, cancel_order; open_position, close_position, set_position_tp_sl, execute_position_tp_sl, trailing stop. |

## Public Analytics (for reporting)

- **Volume**: On-chain via `get_protocol_analytics()` (total_volume_token0, total_volume_token1).
- **Latency / revert rate**: Intended to be derived off-chain (indexer/backend) from events and tx outcomes; not stored on-chain.

## Threat model

See [SECURITY_AUDIT.md](../SECURITY_AUDIT.md) for threat model and mitigations. Focus: feasibility and a **credible threat model** (no overclaimed “internal audit”); transparent testing log only.
