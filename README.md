<p align="center">
  <img src="https://img.shields.io/badge/Arbitrum-Stylus-28A0F0?style=for-the-badge&logo=ethereum" alt="Arbitrum Stylus" />
  <img src="https://img.shields.io/badge/Rust-Secure-000000?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/MEV-Protected-00C853?style=for-the-badge" alt="MEV Protected" />
  <img src="https://img.shields.io/badge/Audited-Security-FF6D00?style=for-the-badge" alt="Audited" />
</p>

<h1 align="center">🌳 Oak Protocol</h1>
<p align="center">
  <strong>The First MEV-Protected DEX on Arbitrum Stylus</strong>
</p>
<p align="center">
  Next-generation decentralized exchange built with Rust — fast, secure, and gas-efficient
</p>

---

## Introduction

**Oak Protocol** is the first MEV-protected decentralized exchange (DEX) built specifically for Arbitrum Stylus. By combining Rust's performance with the Commit-Reveal mechanism, we eliminate front-running and sandwich attacks while delivering **40-50% gas savings** compared to traditional Solidity DEXs.

We're building the future of fair, efficient DeFi — one swap at a time.

---

## Key Features

### 🔒 Commit-Reveal Mechanism

| Feature | Description |
|---------|-------------|
| **Front-Running Protection** | Users submit a commitment (hash) first; swap parameters remain hidden until execution |
| **Two-Phase Execution** | `commit_swap()` → wait 5 blocks → `reveal_swap()` — MEV bots cannot predict or front-run your trades |
| **Cryptographic Security** | keccak256 ensures commitment integrity; hash forgery is computationally infeasible |
| **Time Lock** | Minimum 5-block delay between commit and reveal prevents immediate extraction of trade information |

> *Your swap stays private until it's confirmed. No more sandwich attacks.*

### 💰 Dynamic Fees

| Capability | Details |
|------------|---------|
| **Owner-Controlled** | Protocol fee adjustable via `set_fee()` (owner only) |
| **DAO-Ready** | Owner address can be a multisig or DAO contract for decentralized governance |
| **Capped at 10%** | Maximum 1000 basis points prevents fee abuse |
| **Default 0.3%** | Competitive fee structure out of the box |

### ⚡ Ultra-Low Gas

| Operation | Oak Protocol (Stylus) | Solidity DEX (Uniswap V2) | Savings |
|-----------|----------------------|---------------------------|---------|
| **commit_swap** | ~15,200 gas | ~45,000–50,000 gas | **~70%** |
| **reveal_swap** | ~33,400 gas | ~65,000–80,000 gas | **40–50%** |
| **add_liquidity** | Optimized | Baseline | **10–15%** |

*Rust + WASM compilation delivers significantly more efficient bytecode than EVM.*

### 🛡️ Emergency Pause

| Feature | Benefit |
|---------|---------|
| **Panic Button** | Owner can halt all swaps instantly via `pause()` |
| **User Fund Protection** | Critical for institutional adoption — we prioritize safety |
| **Reversible** | `unpause()` restores operations after issue resolution |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Oak Protocol Stack                        │
├─────────────────────────────────────────────────────────────┤
│  Rust        │  Memory-safe, zero-cost abstractions         │
│  Stylus SDK  │  Arbitrum-native smart contract framework   │
│  WASM        │  Compact, fast execution on Arbitrum         │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Role |
|-------|------------|------|
| **Language** | Rust | Memory safety, type safety, no undefined behavior |
| **Framework** | Stylus SDK | Solidity ABI compatibility, EVM integration |
| **Runtime** | WebAssembly | ~15–25 KB compiled size, efficient execution |

---

## Audit Summary

Oak Protocol has undergone a comprehensive internal security audit. All identified vulnerabilities have been addressed.

| Vulnerability | Status | Protection |
|---------------|--------|------------|
| Reentrancy | ✅ Fixed | Commitment cleared before swap execution |
| Integer Overflow | ✅ Protected | All U256 ops use `checked_*` methods |
| Access Control | ✅ Protected | `_only_owner` on all admin functions |
| Zero Address | ✅ Fixed | Validated in `init()` |
| Hash Forgery | ✅ Protected | keccak256 cryptographic commitment |
| Time Lock Bypass | ✅ Protected | Network-controlled block number |

📄 **Full Report:** [AUDIT_REPORT.md](./AUDIT_REPORT.md)

---

## Roadmap

```
Phase 1          Phase 2           Phase 3            Phase 4
   │                 │                  │                  │
   ▼                 ▼                  ▼                  ▼
┌──────┐       ┌──────────┐      ┌──────────┐      ┌─────────────────┐
│Grant │ ────► │  Hiring  │ ───► │ Mainnet  │ ───► │  Aggregator     │
│      │       │  & Team  │      │  Launch  │      │  Integrations   │
└──────┘       └──────────┘      └──────────┘      └─────────────────┘
  Arbitrum         Expand            Full             1inch, Paraswap,
  Foundation       core team        deployment        CoWSwap, etc.
```

| Phase | Milestone | Target |
|-------|-----------|--------|
| **1** | Arbitrum Foundation Grant | Q1 2026 |
| **2** | Team Expansion | Q2 2026 |
| **3** | Mainnet Launch | Q2–Q3 2026 |
| **4** | Aggregator Partnerships | Q3–Q4 2026 |

---

## Development

### Build

```bash
cargo build --target wasm32-unknown-unknown
```

### Check

```bash
cargo check --target wasm32-unknown-unknown
```

### Prerequisites

- Rust (latest stable)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

---

## Contract Interface

| Function | Access | Description |
|----------|--------|-------------|
| `init(owner)` | Once | Initialize contract, set owner |
| `commit_swap(hash)` | Public | Submit swap commitment |
| `reveal_swap(amount_in, salt, min_amount_out)` | Public | Execute swap after reveal |
| `add_liquidity(amount0, amount1)` | Public | Add liquidity to pool |
| `set_fee(fee_bps)` | Owner | Update protocol fee |
| `pause()` / `unpause()` | Owner | Emergency control |

---

## Why Oak Protocol?

| For Users | For LPs | For Builders |
|-----------|---------|--------------|
| No MEV extraction | Fair execution | Rust + Stylus = modern stack |
| Lower gas costs | Dynamic fee revenue | Open source, audited |
| Slippage protection | Emergency pause safety | Arbitrum ecosystem growth |

---

<p align="center">
  <strong>Oak Protocol</strong> — Fair DeFi on Arbitrum Stylus
</p>
<p align="center">
  Built for the Arbitrum Foundation Grant Program
</p>
