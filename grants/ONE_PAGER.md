# Oak Protocol — Grant One-Pager

**Project:** MEV-Protected DEX on Arbitrum Stylus  
**Ask:** $35,000 (single grant or milestone-based)  
**Stage:** Core complete, testnet-ready; mainnet target May 2026 post-audit  

---

## Problem

MEV bots extract billions from retail traders via front-running and sandwich attacks. Standard DEXs expose swap parameters in the mempool, making every trade exploitable.

## Solution

**Oak Protocol** is a production-ready DEX on **Arbitrum Stylus** (Rust/WASM) that **cryptographically hides** swap intent until execution:

- **Commit–reveal**: User commits `keccak256(amount_in, salt)`; reveal happens ≥5 blocks later with slippage protection.
- **40–50% gas savings** vs. Solidity DEXs (Stylus WASM).
- **Flash swaps** for arbitrage/liquidations without upfront capital.

## What’s Built

| Component | Status |
|-----------|--------|
| Rust/Stylus core (CPMM, fees, multi-pool, router) | ✅ Complete |
| Commit–reveal MEV resistance | ✅ Implemented |
| Flash swaps (callback, K verification) | ✅ Implemented |
| GMX-style dashboard (Next.js 14, Wagmi, RainbowKit) | ✅ In repo |
| Real-time charts, swap UI, liquidity UI | ✅ |
| Internal security review (AUDIT.md), stress + router tests | ✅ |
| External audit | 🔜 Planned (grant-funded) |

## Why Arbitrum Stylus

- **Rust**: memory safety, type safety, easier audits.
- **WASM**: 40–50% gas savings; critical for MEV-resistant flows (two tx: commit + reveal).
- **Ecosystem fit**: First MEV-resistant DEX on Stylus; showcases Stylus for DeFi.

## Use of Grant ($35,000)

1. **External security audit** ($25,000): Required for mainnet; engagement, remediation, report.
2. **Mainnet deployment** ($3,000): Arbitrum One deploy, verification, treasury/config setup.
3. **Launch & bootstrap** ($5,000): Documentation, 1–2 initial pools, minimal LP seeding.
4. **Reserve** ($2,000): Post-audit remediation or unexpected costs.

## Traction & Roadmap

- **Codebase**: Public — [https://github.com/OakProtocolDev/OakProtocol](https://github.com/OakProtocolDev/OakProtocol). Rust contracts, Next.js app, 10+ integration/stress/router tests.
- **Testnet**: Contract and dashboard code ready; deployment and public app URL upon grant acceptance.
- **May 2026**: Audit completion → mainnet (Arbitrum One) → initial liquidity.
- **Q3–Q4 2026**: Aggregator integrations, oracles, fee revenue scaling.

## Ask

We are applying for **$35,000** to fund an external audit and mainnet launch. Oak Protocol is the first MEV-resistant DEX on Arbitrum Stylus and is built to be the fairest, most gas-efficient DEX on Arbitrum.

**Contact:** oak.protocol.2025@gmail.com  
**Repo:** https://github.com/OakProtocolDev/OakProtocol  
**Discord (grant inquiries):** Oak.node
