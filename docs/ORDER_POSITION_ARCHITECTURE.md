# Order & Position Architecture — Pro Trading Terminal

**Oak Protocol** supports Limit Orders, Stop-Loss (SL), Take-Profit (TP), and tracked **Positions** with PnL, entry price, and one-click close. This document describes data structures and the choice between on-chain and off-chain order storage.

---

## 1. Data Structures (On-Chain)

### 1.1 Order (TP/SL/Limit)

Orders are stored on-chain; tokens are **escrowed** in the contract until execution or cancel.

| Field           | Type    | Description |
|----------------|---------|-------------|
| `order_id`     | U256    | Unique ID (incrementing). |
| `owner`        | Address | Creator; receives `token_in` on execution. |
| `token_in`     | Address | Token to **receive** when order fills. |
| `token_out`    | Address | Token to **sell** (escrowed in contract). |
| `amount_out`   | U256    | Amount of `token_out` to sell. |
| `trigger_price`| U256    | Price (reserve_in/reserve_out, 18 decimals). Limit/TP: fill when price ≥ trigger; SL: when price ≤ trigger. |
| `order_type`   | U256    | 0 = Limit, 1 = TP, 2 = SL. |
| `status`       | U256    | 0 = Open, 1 = Executed, 2 = Cancelled. |
| `created_at`   | U256    | Block number when placed. |

| `oco_pair`       | U256    | If non-zero, the other order ID in an OCO pair. When this order executes, the paired order is cancelled (tokens returned to owner). |

**Storage key:** `order_id` is mapped to an `Address` (last 20 bytes of U256 BE) for Stylus `StorageMap` keys.

**Flow:** User calls `place_order(token_in, token_out, amount_out, trigger_price, order_type, oco_with_order_id)`. If `oco_with_order_id != 0`, the two orders are linked (OCO: One-Cancels-Other). When either order is executed, the other is automatically cancelled and its escrowed tokens returned to the owner. Anyone can call `execute_order(order_id, min_amount_out)` when price condition is met. User can `cancel_order(order_id)` to get escrowed tokens back (OCO link is cleared for both orders).

---

### 1.2 Position (Tracked for PnL / TP/SL / Close)

Positions are **tracking records**; the user’s tokens remain in their wallet until they close.

| Field            | Type    | Description |
|------------------|---------|-------------|
| `position_id`    | U256    | Unique ID (incrementing). |
| `owner`          | Address | Position owner. |
| `base_token`     | Address | Token held (e.g. ETH); sold on close. |
| `quote_token`    | Address | Token received on close (e.g. USDC). |
| `size`           | U256    | Amount of base (18 decimals). |
| `entry_price`    | U256    | Quote per base at open (18 decimals). |
| `tp_price`       | U256    | Take-profit price (0 = not set). |
| `sl_price`       | U256    | Stop-loss price (0 = not set). |
| `opened_at`      | U256    | Block number when opened. |
| `status`         | U256    | 0 = Open, 1 = Closed. |
| `trailing_delta_bps` | U256 | Trailing stop delta in basis points (0 = disabled). E.g. 100 = 1%. |
| `trailing_peak_price` | U256 | Peak price for trailing; updated by `update_trailing_stop` when oracle price rises. |

**Flow:** After a swap, user (or frontend) calls `open_position(base_token, quote_token, size, entry_price)` to register a position. Optionally `set_position_tp_sl(position_id, tp_price, sl_price)` and/or `set_position_trailing_stop(position_id, trailing_delta_bps)`. To close: `close_position(position_id, min_amount_out)`. Anyone can call `execute_position_tp_sl(position_id, min_amount_out)` when on-chain price ≥ TP or ≤ SL. **Trailing stop:** An off-chain bot (or keeper) calls `update_trailing_stop(position_id, new_price, min_amount_out)` on each oracle price update. If `new_price > peak`, `peak` is updated. If `new_price <= peak * (10000 - trailing_delta_bps) / 10000`, the position is closed (base transferred from owner, swapped to quote, sent to owner). Owner must have approved the contract to spend base tokens for trailing close.

**Margin & liquidation:**  
- `initial_collateral` (quote, 18 decimals) — optional margin at open; transferred to contract.  
- `margin_added` (quote) — increased by `add_margin(position_id, amount)`; does not change `entry_price` or `size`.  
- **Liquidation price (long)** = (initial_collateral + margin_added) / size (quote per base). When mark_price ≤ this, position is undercollateralized.  
- **Health factor (view)** = `get_position_health(position_id)` returns `(liquidation_price, health_factor_bps)`. `health_factor_bps = current_price * 10_000 / liquidation_price`; > 10_000 = healthy, ≤ 10_000 = at or below liquidation.

**PnL (off-chain / frontend):**  
`current_price = get_current_price(base_token, quote_token)`.  
Unrealized PnL (quote units): `size * (current_price - entry_price)` (long base). Frontend uses `get_position` + `get_current_price` + `get_position_health` to display.

---

## 2. On-Chain vs Off-Chain Orders (Gas Efficiency)

### Option A: On-Chain Orders (Current)

- **Place:** User transfers tokens to contract + SSTORE for order fields. Gas: ~1–2x swap (storage + transfer).
- **Cancel:** Return tokens + SSTORE.  
- **Execute:** Swap from contract to owner; anyone (or keeper) can call.

**Pros:** Simple, trustless, no relayers; fits existing MEV protection (commit-reveal applies to swap execution).  
**Cons:** Every place/cancel costs gas; order book size limited by block space.

### Option B: Off-Chain Signed Orders (GMX-Style)

- User signs order (params + nonce); no token transfer on place. Keeper/relayer submits when condition met; user’s tokens pulled at execution (allowance or vault).
- **Place:** 0 gas (signature only). **Execute:** Keeper pays; user pays approval once or vault deposit.

**Pros:** Very low gas for placing; large number of orders possible.  
**Cons:** Requires keeper infrastructure, indexing, and approval/vault flow; more complex; MEV on execution unless combined with private mempool/commit-reveal.

### Recommendation (Arbitrum Stylus)

- **MVP / v1:** Use **on-chain orders** (current design). Gas on Arbitrum is cheap; simplicity and MEV compatibility (commit-reveal) are more important. Optimize by packing order fields if needed (e.g. `order_type` + `status` in one slot).
- **Future:** Add **signed orders + relayer** as an optional path for power users who want to place many orders with minimal gas; keep on-chain orders for users who prefer full trustlessness.

---

## 3. MEV Protection

- **Swap execution:** Existing commit-reveal protects swap parameters until reveal; TP/SL/limit **execution** is a swap from contract (or user in `close_position`), so execution can be front-run unless executed via a private RPC or the same commit-reveal flow for the execution tx. Document that TP/SL execution is public once the execution tx is in the mempool.
- **Order placement:** Order parameters are public on place; only execution condition (price) is time-based. For limit orders, consider optional “post-only” or delay to reduce sandwich risk on execution.

---

## 4. Summary

| Feature        | Storage     | Tokens    | Who executes      |
|----------------|------------|-----------|-------------------|
| Limit/TP/SL    | On-chain   | Escrowed  | Anyone (keeper)   |
| Position       | On-chain   | In wallet | Owner (close) or anyone (TP/SL) |
| close_position | —          | User → swap → user | Owner only |

Order and Position structs are defined above; implementation lives in `state.rs` (storage), `logic.rs` (place/cancel/execute, open/set_tp_sl/close/execute_tp_sl, set_position_trailing_stop, update_trailing_stop), and `events.rs` (OrderPlaced, OrderCancelled, OrderExecuted, OpenPosition, ClosePosition, SetPositionTPSL, SetPositionTrailing, TrailingStopTriggered).

---

## 5. Trailing Stop-Loss

- **Storage:** `position_trailing_delta_bps` (0 = off), `position_trailing_peak_price` (quote per base, 18 decimals).
- **add_margin(position_id, amount):** Owner-only. Transfers `amount` of quote token from owner to contract; increases `margin_added` and global `position_margin_balance[quote]`. Does not change `entry_price` or `size`. Liquidation price becomes (initial_collateral + margin_added) / size.
- **get_position_health(position_id):** View. Returns `(liquidation_price, health_factor_bps)`. `liquidation_price = (initial_collateral + margin_added) / size`. `health_factor_bps = current_price * 10_000 / liquidation_price` (10000 = 1.0; > 10000 = healthy).
- **set_position_trailing_stop(position_id, trailing_delta_bps):** Owner-only. Sets delta (e.g. 100 = 1%) and initializes peak to entry price. Max delta 10000 (100%).
- **update_trailing_stop(position_id, new_price, min_amount_out):** Callable by anyone (off-chain bot). Uses oracle `new_price`: if `new_price > peak`, updates `peak`. If `new_price <= peak * (10000 - delta_bps) / 10000`, closes the position (return margin to owner, then transfer base from owner → contract, swap → quote to owner). Emits `TrailingStopTriggered`. Owner must have approved the contract to spend base tokens for this path.

---

## 6. OCO (One-Cancels-Other) Orders

- **Storage:** `order_oco_pair`: order_id → paired order_id.
- **place_order(..., oco_with_order_id):** If `oco_with_order_id != 0`, the new order is linked with the existing order (same owner, other order must be open). Both orders store each other’s ID.
- **execute_order(order_id):** After execution, if `order_oco_pair[order_id] != 0`, the paired order is cancelled (escrowed tokens returned to owner, status set to Cancelled, OCO links cleared).
- **cancel_order(order_id):** OCO links are cleared for both orders so the other order remains open but unlinked.
