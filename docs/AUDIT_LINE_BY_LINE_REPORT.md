# Line-by-Line Security & Performance Audit Report

**Scope:** Oak Stylus Trading Engine (Arbitrum Stylus, Rust/WASM)  
**Focus:** Access control, reentrancy, arithmetic, inputs, storage, performance.

---

## 1. Security Audit Summary

### 1.1 Access Control ✅

| Function | Protection | Notes |
|----------|------------|--------|
| `set_fee` | `only_owner(owner)` | Bounds check `new_fee_bps <= MAX_FEE_BPS`. |
| `pause` / `unpause` | `require_role(pauser_role())` via Pausable | Role-based. |
| `withdraw_treasury_fees` | `only_owner(owner)` | Reentrancy guard; treasury != contract; balance check. |
| `trigger_circuit_breaker` / `clear_circuit_breaker` | `only_owner(self.owner.get())` | Event emitted. |
| `set_buyback_wallet` | `only_owner` | Can set to zero to disable. |
| `set_pending_owner` / `accept_owner` | Owner / pending only; delay check | Two-step transfer. |
| `set_referral_fee_bps` (growth) | `msg::sender() == dex.owner.get()` | Owner-only. |
| `set_badge_contract` (quest) | Same | Owner-only. |
| `init` (staking) | Owner-only | StakingRewards. |

**Recommendation:** Critical parameter changes (set_fee, withdraw_treasury_fees, set_buyback_wallet) should be executed via **TimelockController** (queue → 24h → execute). See `docs/SECURITY_AUDIT.md`.

---

### 1.2 Reentrancy ✅ (Fixes Applied)

| Area | Status | Change |
|------|--------|--------|
| **logic.rs** | ✅ | All entrypoints that perform token transfers already use `lock_reentrancy_guard` / `unlock_reentrancy_guard` (reveal_swap, add_liquidity, remove_liquidity, withdraw_treasury_fees, swap router, order/position execution, flash_swap). |
| **intelligence/copy_trading.rs** | ✅ **Fixed** | `execute_copy_trade` calls `process_swap_from_to_with_fee` (which does token transfers) but did **not** acquire the guard. **Fix:** Acquire `lock_reentrancy_guard(dex)` at entry; `unlock_reentrancy_guard(dex)` on every exit path (all error returns and success). |
| **intelligence/signal_marketplace.rs** | ✅ **Fixed** | `purchase_signal` performs `safe_transfer_from` without guard. **Fix:** Acquire guard at start; CEI: set `signal_purchased` and bump `signal_nonce` **before** transfer; on transfer failure, rollback state and unlock. |
| **growth/staking_rewards.rs** | ✅ **Fixed** | `stake`, `unstake`, `claim_rewards` perform external transfers without guard. **Fix:** Guard in all three; CEI in stake (state update then transfer; rollback on transfer failure); same in unstake/claim with rollback on transfer failure. |

**Guard visibility:** `lock_reentrancy_guard` and `unlock_reentrancy_guard` in `logic.rs` are now **`pub(crate)`** so that `intelligence` and `growth` can use them.

---

### 1.3 Arithmetic ✅

- **Checked ops:** All user- and reserve-derived math uses `checked_add`, `checked_sub`, `checked_mul`, `checked_div` (or explicit overflow handling). No raw `+`/`-`/`*`/`/` on `U256` in critical paths.
- **Division by zero:** Guards with `is_zero()` or `ok_or_else(|| err(ERR_DIVISION_BY_ZERO))` where applicable.
- **Copy trading / signal / staking:** Same pattern (checked ops, rollback on failure where state was updated before external call).

---

### 1.4 Input Validation ✅

- **Addresses:** `require_non_zero_address(token0/token1)` (or equivalent) on swap, liquidity, router, and withdraw paths. Owner/treasury zero checks in init and withdraw.
- **Amounts:** Zero amount checks (e.g. `amount_in.is_zero()`, `amount.is_zero()`) before processing. Slippage and deadline enforced.
- **Referral:** `referrer != referee`; zero referrer allowed (to clear).

---

### 1.5 Storage ✅

- **Layout:** `sol_storage!` defines a flat layout; no manual packing. StorageMap keys are well-defined (address, FixedBytes<32>). No evidence of slot collision.
- **Reads:** Consistent use of `.getter(key).get()` and `.setter(key).set(value)`.

---

## 2. Performance (Stylus-Oriented)

### 2.1 Gas / Storage

- **Duplicate reads:** In hot paths (e.g. swap), `protocol_fee_bps` and pool reserves are read once per logical operation; acceptable. Optional future improvement: cache `fee_bps` in a local when used multiple times in the same function.
- **Packed storage:** Current layout is already slot-efficient; packing multiple small values into one slot would require a redesign and is not recommended without a clear size win.

### 2.2 Computational Complexity

- **Loops:** Bounded by `path.len() <= MAX_PATH_LENGTH` (e.g. 10) in router and `get_amounts_out`. Batch size capped by `MAX_BATCH_POSITIONS`. No unbounded loops on user input.
- **Heavy work:** TWAP and circuit breaker checks are O(1) per swap.

### 2.3 WASM / Memory

- **no_std:** Crate is `#![cfg_attr(not(test), no_std)]`; `alloc` used for `Vec` where needed. No `std` in contract code.
- **Allocations:** `Vec` used for path, encoding, and event data; sizes are bounded. Optional: reuse buffers where possible to reduce allocator churn (e.g. in event encoding).

---

## 3. Events for Indexers ✅

Critical actions already emit events:

- Swap: `RevealSwap`, `SwapExecuted` (sender, tokenIn, tokenOut, amountIn, amountOut).
- Liquidity: `AddLiquidity`, `RemoveLiquidity`, `PoolCreated`, `LP Transfer`.
- Admin: `SetFee`, `PauseChanged`, `WithdrawTreasuryFees`, `CircuitBreakerTriggered/Cleared`, `EmergencyTriggered`, `BuybackWalletSet`, `PendingOwnerSet`, `OwnerChanged`.
- Orders/positions: `OrderPlaced`, `OrderExecuted`, `OrderCancelled`, `OpenPosition`, `ClosePosition`, etc.
- Growth: `EmissionEvent` (module_id, user, event_type, amount, token_id).
- Intelligence: `CopySubscription`, `CopySubscriptionRevoked`, `CopyTradeExecuted`, `SignalPurchased`.

No additional critical events were missing for the audited paths.

---

## 4. Fixes Applied (Code Changes)

1. **logic.rs:** `lock_reentrancy_guard` and `unlock_reentrancy_guard` made **`pub(crate)`**.
2. **intelligence/copy_trading.rs:** Reentrancy guard around full `execute_copy_trade`; unlock on all error paths (subscription checks, deadline, amount_in zero, pool invalid, get_amount_out_with_fee failure, min_out overflow, process_swap failure).
3. **intelligence/signal_marketplace.rs:** Reentrancy guard in `purchase_signal`; CEI (set purchased + bump nonce before transfer); rollback state and unlock on transfer failure.
4. **growth/staking_rewards.rs:** Reentrancy guard in `stake`, `unstake`, `claim_rewards`; CEI and rollback on transfer failure in all three; unlock on `_update_rewards` failure.

---

## 5. Conclusion

- **Access control:** Admin functions are correctly restricted; Timelock is recommended for critical parameter changes.
- **Reentrancy:** All entrypoints that perform external calls (ERC20 transfer in/out) now use the global reentrancy guard; CEI and rollback are applied where state is updated before transfer.
- **Arithmetic and inputs:** Checked math and input validation are in place.
- **Storage:** No collision or misuse found.
- **Performance:** Bounded loops and no_std; optional micro-optimizations (caching, buffer reuse) can be considered later.
- **Events:** Sufficient for indexers and monitoring.

These changes are backward-compatible and do not alter the public API or storage layout.
