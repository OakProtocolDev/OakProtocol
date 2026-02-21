# Oak Protocol — Internal Security Audit Report

**Audit Date:** February 2025  
**Scope:** Smart contracts (Arbitrum Stylus, Rust/WASM), Frontend (Next.js 14, Wagmi, Viem)  
**Auditor:** Internal Security Review  

---

## Executive Summary

This audit examined the Oak Protocol codebase, encompassing the commit-reveal MEV-protected DEX smart contracts (Rust/Arbitrum Stylus) and the Next.js frontend. The protocol implements a two-phase commit-reveal flow to mitigate front-running and sandwich attacks.

**Overall assessment:** The codebase demonstrates solid security foundations—reentrancy guards, checked arithmetic, CEI patterns, and input validation—but **one CRITICAL** vulnerability and several **MEDIUM** and **LOW** issues were identified that must be addressed before production deployment and external audit.

---

## Risk Summary

| Level | Count |
|-------|-------|
| **Critical** | 1 |
| **High** | 2 |
| **Medium** | 5 |
| **Low** | 4 |

---

## 1. Smart Contract Findings

### 1.1 [CRITICAL] Treasury Withdrawal Does Not Update Reserves

**File:** `src/logic.rs` — `withdraw_treasury_fees`

**Description:** When the owner withdraws accrued treasury fees, the contract transfers `token0` to the treasury address but **does not decrease `reserves0`**. The reserves represent the pool's token balances. After withdrawal:

- Actual `token0` balance = `reserves0 - accrued`
- Stored `reserves0` = unchanged (still reflects pre-withdrawal)

This breaks the invariant `balance_of(token0, contract) == reserves0`. Subsequent swaps will use inflated reserves for CPMM math, leading to incorrect `amount_out` calculations and potential **underpayment to users** or **insolvency** if the contract attempts to send more `token1` than it holds.

**Recommended Fix:**

```rust
// Before transfer, decrease reserves to maintain invariant
let reserve0 = self.reserves0.get();
let new_reserve0 = reserve0.checked_sub(accrued)
    .ok_or_else(|| { unlock_reentrancy_guard(self); err(ERR_INSUFFICIENT_LIQUIDITY) })?;
self.reserves0.set(new_reserve0);

// Then reset accrued and transfer
self.accrued_treasury_fees_token0.set(U256::ZERO);
safe_transfer(token, treasury, accrued)?;
```

---

### 1.2 [HIGH] Salt Predictability / Replay from Commitment Event

**File:** `src/logic.rs` — `commit_swap`, `reveal_swap`; `src/events.rs`

**Description:** The `CommitSwap` event emits the commitment hash and block number. Although the hash binds `(amount_in, salt)`, the **salt is chosen client-side**. If the frontend uses weak randomness (e.g., `Math.random()`, low-entropy `Date.now()`), an attacker could:

1. Observe the commitment event
2. Bruteforce or predict the salt (if entropy is low)
3. Frontrun the reveal with the user's own parameters

**Recommended Fix:**

- Frontend: Use `crypto.getRandomValues()` for salt generation (or equivalent CSPRNG)
- Consider adding `msg.sender` to the commitment hash to prevent cross-user replay if commitment structure is ever reused
- Document the requirement for cryptographically secure salt in integration guides

---

### 1.3 [HIGH] Reveal Transaction Frontrunning Window

**File:** `src/logic.rs` — `reveal_swap`; `src/constants.rs`

**Description:** The commit-reveal delay is **5 blocks** (`COMMIT_REVEAL_DELAY = 5`). Once the delay has passed, the reveal transaction enters the public mempool. A searcher can observe the reveal transaction (which exposes `amount_in`, `salt`, `min_amount_out`, `deadline`, `token0`, `token1`) and **sandwich or frontrun it** before it is mined, as the reveal itself is not commitment-hidden.

The design protects the **commit** phase (intent is hidden). The **reveal** phase is inherently transparent. On L2 (Arbitrum), the mempool dynamics differ from Ethereum L1, but private orderflow and inclusion strategies can still affect fairness.

**Recommended Fix:**

- Document this as an accepted design tradeoff; commit-reveal mitigates frontrunning of the *commit*, not the reveal
- Consider integration with private RPCs or inclusion services for power users
- Evaluate increasing the delay if block times and mempool structure allow

---

### 1.4 [MEDIUM] Flash Swap Callback ABI Encoding May Be Incorrect

**File:** `src/logic.rs` — `flash_swap` (lines 681–713)

**Description:** The raw ABI encoding for `oakFlashSwapCallback(uint256,uint256,bytes)` is constructed manually. The `bytes` parameter encoding (offset, length, data) may not conform to the Solidity ABI spec in edge cases:

- The offset value `96` assumes a specific layout
- Dynamic `bytes` padding could be incorrect for certain `data.length` values

If the borrower contract uses a strict ABI decoder, the callback may revert or read wrong data.

**Recommended Fix:**

- Use `alloy_sol_types` or Stylus SDK ABI encoding utilities if available
- Add integration tests with a Solidity borrower contract that implements the callback
- Validate against `cast abi-encode` or equivalent

---

### 1.5 [MEDIUM] `withdraw_treasury_fees` Token Parameter Not Validated Against Pool

**File:** `src/logic.rs` — `withdraw_treasury_fees`

**Description:** The function accepts an arbitrary `token` address. The accounting (`accrued_treasury_fees_token0`) is tracked only for `token0`. If the owner passes `token1` or a malicious token address, the transfer could:

- Succeed but withdraw the wrong asset
- Interact with a token that has unexpected behavior (e.g., fee-on-transfer, rebasing)

**Recommended Fix:**

- Add a storage slot for the canonical `token0` address used by the pool, or
- Validate `token == self.token0.get()` (if token0 is stored), or at minimum document that `token` must be the pool's token0

---

### 1.6 [MEDIUM] `balance_of` Returns Zero on Failure

**File:** `src/token.rs` — `balance_of`

**Description:** When `IERC20::balanceOf` fails (e.g., non-contract address, revert), `balance_of` returns `U256::ZERO`. In `flash_swap`, this is used to verify repayment. If the token call fails, returning zero could cause the contract to incorrectly reject a valid repayment or, in a different code path, accept insufficient repayment.

**Recommended Fix:**

- Propagate the error instead of returning zero, or
- Use `try_balance_of` and explicitly handle failures in callers

---

### 1.7 [MEDIUM] No Token0/Token1 Validation in Swap Functions

**File:** `src/logic.rs` — `reveal_swap`, `add_liquidity`, `flash_swap`

**Description:** The contract does not enforce that `token0` and `token1` match the pool's canonical token pair. A malicious or mistaken caller could specify arbitrary token addresses, potentially interacting with tokens that have non-standard behavior (fee-on-transfer, rebasing, pausable) and breaking CPMM assumptions.

**Recommended Fix:**

- Store `token0` and `token1` in state during `init` or first `add_liquidity`
- Require `token0 == self.token0.get()` and `token1 == self.token1.get()` in swap and liquidity functions

---

### 1.8 [MEDIUM] `cancel_commitment` Allows Canceling Within Reveal Window

**File:** `src/logic.rs` — `cancel_commitment`

**Description:** Users can cancel a commitment once `current_block >= min_block` (i.e., after the delay). This includes the entire reveal window. Allowing cancel within the reveal window is by design (user may change their mind), but it creates a minor griefing vector: a user could repeatedly commit and cancel to increase state writes and marginally affect gas costs for the protocol.

**Risk:** Low impact; document as acceptable.

---

### 1.9 [LOW] `debug_assert!` in Fee Split

**File:** `src/logic.rs` — `compute_fee_split` (line 232)

**Description:** `debug_assert!(treasury_fee + lp_fee == total_fee)` is compiled out in release builds. If a logic error ever breaks this invariant, it would go unnoticed in production.

**Recommended Fix:** Replace with `assert!` or return an error if the invariant fails.

---

### 1.10 [LOW] Entrypoint Returns Empty Vec

**File:** `src/lib.rs` — `main`

**Description:** The `#[entrypoint]` returns `Ok(Vec::new())` for all inputs. The contract is not wired to dispatch to `OakDEX` methods. This appears to be a placeholder; the actual entrypoint must be generated (e.g., by `cargo stylus`) and must correctly route to the public methods.

**Recommended Fix:** Ensure the deployment pipeline uses the correct generated entrypoint and ABI.

---

## 2. Stylus/WASM-Specific Checks

### 2.1 Memory Safety

- **Rust `no_std`:** The contract uses `#![cfg_attr(not(test), no_std)]` and `alloc`, which reduces surface area for heap-related issues.
- **No unsafe blocks:** No `unsafe` code was found in the contracts; the Stylus SDK handles FFI.
- **Vectors:** All `Vec` usage appears bounds-safe; no raw pointer arithmetic.

### 2.2 Reentrancy

- **Global lock:** `lock_reentrancy_guard` / `unlock_reentrancy_guard` are used consistently in `reveal_swap`, `add_liquidity`, `withdraw_treasury_fees`, and `flash_swap`.
- **CEI pattern:** State updates occur before external calls; lock is released at the very end.
- **Flash swap callback:** The callback is invoked while the lock is held, preventing reentrancy into the DEX.

### 2.3 Integer Overflow

- **Checked math:** All arithmetic uses `checked_add`, `checked_sub`, `checked_mul`, `checked_div` with explicit error handling.
- **U256:** Uses `alloy_primitives::U256`; no unchecked conversions.

---

## 3. Frontend Vulnerabilities

### 3.1 [MEDIUM] Transaction Hash Displayed Before On-Chain Confirmation

**File:** `web/app/trading/page.tsx`, `web/components/SuccessModal.tsx`

**Description:** The UI shows a “Transaction Successful” modal with a transaction hash and Arbiscan link **immediately** after the placeholder swap handler resolves. The hash is generated client-side via `crypto.getRandomValues` and is not a real on-chain transaction. When wired to real contracts:

- If the modal is shown before the transaction is confirmed, users may be misled
- A failed or reverted transaction could still show “success” if the error handling is wrong

**Recommended Fix:**

- Only show success modal after `waitForTransactionReceipt` (or equivalent) confirms success
- Display pending state clearly (“Confirming transaction…”)
- Never generate fake hashes for real flows

---

### 3.2 [LOW] XSS in Transaction Logging

**File:** `web/app/trading/page.tsx`, `web/components/LiveLogsPanel.tsx`

**Description:** Log messages are static strings; no user-controlled input is rendered as HTML. Trade history displays `amountIn`, `amountOut`, `txHash` from state. These are set from swap parameters and `crypto.randomUUID`/`getRandomValues`. Risk is low, but if future versions accept user input (e.g., labels, notes) and render them without sanitization, XSS could occur.

**Recommended Fix:** Continue using React's default escaping; avoid `dangerouslySetInnerHTML` for any user-controlled content.

---

### 3.3 [LOW] Session Trade History in Memory Only

**File:** `web/app/trading/page.tsx`

**Description:** Trade history is stored in React state (`useState`). It is lost on refresh and is not persisted. This is acceptable for session-only UX but means:

- No audit trail for users
- No protection against accidental tab close

**Recommended Fix:** If persistence is desired, use `sessionStorage` with explicit opt-in; avoid storing sensitive data. Do not use `localStorage` for sensitive financial data without encryption.

---

### 3.4 [LOW] WalletConnect Project ID Fallback

**File:** `web/config/wagmi.ts`

**Description:** If `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` is missing, the config uses `"YOUR_PROJECT_ID"`. This will cause WalletConnect to fail or behave unexpectedly in production.

**Recommended Fix:** Validate the project ID at build time or runtime and fail fast with a clear error if missing.

---

### 3.5 [LOW] External Link to Arbiscan Without Verification

**File:** `web/components/SuccessModal.tsx`, `web/app/trading/page.tsx`

**Description:** Links like `https://arbiscan.io/tx/${txHash}` assume mainnet. The app is configured for Arbitrum Sepolia; Arbiscan Sepolia uses `sepolia.arbiscan.io`. Wrong links could confuse users or point to non-existent transactions.

**Recommended Fix:** Use chain-aware explorer URLs (e.g., from wagmi’s chain config or `getChainBlockExplorerUrl`).

---

## 4. Commit-Reveal MEV Protection Analysis

### 4.1 Flow Summary

1. **Commit:** User sends `keccak256(abi.encode(amount_in, salt))` in `commit_swap`.
2. **Delay:** At least `COMMIT_REVEAL_DELAY` (5) blocks must pass.
3. **Reveal:** User sends `(token0, token1, amount_in, salt, min_amount_out, deadline)` in `reveal_swap`.
4. **Verification:** Contract recomputes the hash, checks delay, deadline, slippage, then executes the swap.

### 4.2 Strengths

- Commitment hides swap parameters until the reveal transaction is broadcast.
- Salt prevents cross-commitment replay.
- Delay reduces block-based predictability.
- Deadline and slippage checks protect users from stale executions.

### 4.3 Weaknesses

- **Reveal frontrunning:** Once revealed, the transaction is visible in the mempool; MEV can still extract value from the reveal itself.
- **Salt entropy:** Depends on client-side randomness; weak RNG weakens the design.
- **No commitment to token pair:** The hash only binds `(amount_in, salt)`. The same commitment could theoretically be revealed with different `token0`/`token1` if the contract allowed it; current logic does not validate token pair against a stored canonical pair.

---

## 5. Security Recommendations for Next Phase

1. **Fix Critical:** Implement reserve decrement in `withdraw_treasury_fees` before any external audit.
2. **Add token validation:** Store and validate `token0`/`token1` for all swap and liquidity operations.
3. **Harden salt generation:** Document and enforce CSPRNG for salt in the frontend; consider bundling a helper.
4. **Flash swap encoding:** Replace manual ABI encoding with library-based encoding and test against a Solidity borrower.
5. **Error propagation:** Change `balance_of` to propagate errors where used for critical checks.
6. **Frontend:** Integrate real contract calls with proper pending/success/error UX; remove fake transaction hashes for production.
7. **Chain-aware explorers:** Use explorer URLs from the configured chain.
8. **Formalize entrypoint:** Ensure the Stylus toolchain generates and wires the entrypoint correctly.

---

## Appendix: Files Audited

**Contracts:**
- `src/lib.rs`
- `src/logic.rs`
- `src/state.rs`
- `src/constants.rs`
- `src/errors.rs`
- `src/events.rs`
- `src/token.rs`
- `tests/integration_tests.rs`

**Frontend:**
- `web/app/page.tsx`
- `web/app/trading/page.tsx`
- `web/app/layout.tsx`
- `web/components/SwapWidget.tsx`
- `web/components/SuccessModal.tsx`
- `web/components/LiveLogsPanel.tsx`
- `web/config/wagmi.ts`
- `web/lib/placeholders.ts`
- `web/hooks/useBinanceData.ts`

---

*This report is intended for internal use to strengthen the protocol before external audit and mainnet deployment.*
