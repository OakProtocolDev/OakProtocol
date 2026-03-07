# Core Engine Architecture & Storage Minimization

## Core Engine (modular layout)

- **Swap logic** is separated from **order execution**:
  - `src/engine/swap_core.rs`: interface and types for a single pool swap (`SwapParams`, `SwapResult`, `SwapCore`). Implementation delegates to `logic::process_swap_from_to_with_fee`.
  - `src/engine/execution.rs`: order execution layer that uses `SwapCore` and `ExecutionStrategy` (no direct dependency on order/position storage layout).
  - `src/logic.rs`: CPMM math, fee accounting, reentrancy guard, and public entrypoints (swap, orders, positions). Called by the engine for the actual swap.

- **ExecutionStrategy** (`src/engine/strategy.rs`):
  - Trait `ExecutionStrategy` with `mode()` and `requires_commit()`.
  - `Atomic`: single-tx swap (default).
  - `CommitReveal`: two-step MEV protection. Strategy can be read from one storage slot (e.g. `execution_mode`: 0 = Atomic, 1 = CommitReveal) to avoid extra reads.

- **Shared Execution Batching** (`src/vault.rs`):
  - `batch_swap()`: aggregates contributions `(owner, amount_in)` in **one transaction** (same block), performs **one** Uniswap-style `_swap()` in the pool, then distributes output proportionally.
  - No cross-tx batch accumulator: aggregation is done in a single call to minimize storage.
  - One **packed slot** `vault_last_batch_packed`: `(block_number << 128) | total_amount_in` for analytics and Gas-Rebate rating (decode with `val >> 128` and `val & ((1<<128)-1)`).

## Storage minimization

- **Grouping in a single slot**:
  - Vault: `vault_last_batch_packed` stores last batch block and total in one `StorageU256`.
  - Execution mode: one slot (0 = Atomic, 1 = CommitReveal) if made configurable at runtime.

- **Avoid redundant writes**:
  - Batch execution does not store per-participant list on-chain; the list is passed in the same tx and processed in memory.
  - Order/position state uses one map per field (Stylus-friendly); packing multiple small fields into one slot can be added later (e.g. status + type in one word).

- **Reads**:
  - Execution strategy: at most one SLOAD per swap when using a global `execution_mode` slot.
  - Swap core uses existing pool and fee slots; no extra storage for the engine layer.

## OpenAPI (api.oak.trade)

- Spec: `docs/openapi-api.oak.trade.yaml`.
- Endpoints:
  - **GET /pools** — liquidity state (reserves, TVL, fee) in JSON.
  - **GET /gas-rebate/rating** — current Gas-Rebate rating (batch rebate bps, accrued rebate, last batch from packed slot).
  - **GET /transactions**, **GET /history** — transaction history (swaps, batch, orders) in JSON with optional pagination and filters.
