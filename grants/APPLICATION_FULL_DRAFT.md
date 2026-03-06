# Oak Protocol — Full Grant Application ($35,000)

---

## 1. Project name and tagline

**Oak Protocol** — The first MEV-protected DEX on Arbitrum Stylus. Fair DeFi, zero front-running, built in Rust.

---

## 2. Team

Solo founder with 4+ years in systems programming and smart contracts. Background in Rust and low-level systems; prior work includes DeFi tooling and on-chain protocol logic. Shipped Oak Protocol from design to full codebase: Rust/Stylus core (CPMM, multi-pool, router, commit–reveal, flash swaps), internal security review, test suite (stress, re-entrancy, dust, router), and GMX-style dashboard (Next.js 14, Wagmi, RainbowKit). Focus is on security-first mainnet launch: grant funds will go to an external audit and deployment, not marketing.

---

## 3. Project description (what you’re building)

Oak Protocol is a **decentralized exchange** on **Arbitrum Stylus** that removes MEV (front-running and sandwich attacks) via a **commit–reveal** mechanism while staying gas-efficient.

**Technical summary:**

- **Smart contracts**: Rust, compiled to WASM; Stylus `sol_storage!` layout; CPMM with configurable fee (0.3%); multi-pool support and router (multi-hop swaps).
- **MEV resistance**: User commits a hash `H = keccak256(amount_in, salt)` in tx 1; after ≥5 blocks, user reveals and executes the swap in tx 2 with slippage protection. Parameters are hidden until reveal.
- **Security**: Re-entrancy guard, 100% checked math, CEI pattern, owner-only admin, internal security review (AUDIT.md) and automated tests (stress, router, integration).
- **Frontend**: Next.js 14 dashboard (GMX-inspired), Wagmi v2, RainbowKit, real-time charts; ready for commit–reveal and multi-hop swaps.

**Differentiators:**

- First MEV-resistant DEX on Stylus.
- Significant gas savings (40–50% vs typical Solidity DEX) from WASM.
- Production-oriented: full core and dashboard in repo; clear path to mainnet post-audit.

---

## 4. Why this grant / why Arbitrum Stylus

We build **natively on Arbitrum Stylus** to prove that production DeFi (DEX, MEV resistance, flash swaps) can run on Rust/WASM with better gas and security. Funding us supports the Stylus ecosystem and positions Arbitrum as the chain where MEV-resistant retail trading is viable. MEV is a major barrier to fair DeFi; Oak delivers a working, auditable design and implementation that can serve as a reference for the ecosystem.

---

## 5. Milestones and use of funds ($35,000)

| Milestone | Deliverable | Budget |
|-----------|-------------|--------|
| **M1: External audit** | Engagement with audit firm; remediation; public report or summary | $25,000 |
| **M2: Mainnet deployment** | Deploy to Arbitrum One; verification; fee/treasury/pause config | $3,000 |
| **M3: Launch & bootstrap** | Documentation, 1–2 initial pools, minimal LP seeding | $5,000 |
| **Reserve** | Post-audit fixes or unexpected costs | $2,000 |

**Total ask:** $35,000.

Funds are used for **audit** (required for mainnet) and **launch** (deployment + bootstrap). No speculative marketing; focus is security and adoption.

---

## 6. Traction and proof of work

- **Code**: [https://github.com/OakProtocolDev/OakProtocol](https://github.com/OakProtocolDev/OakProtocol). Rust contracts + Next.js app; stress tests (fees, re-entrancy, dust, limits), router integration tests, and general integration tests.
- **Security**: Internal review in `AUDIT.md`; testnet deployment and public dashboard URL will be live upon grant acceptance.
- **Docs**: README with architecture, security, roadmap; `grants/` folder with one-pager and this application.

---

## 7. Roadmap (short)

- **April–May 2026**: External audit; remediation if needed.
- **May 2026**: Mainnet deployment (Arbitrum One); initial liquidity (1–2 pairs).
- **Q3 2026**: Aggregator integrations (1inch, Paraswap, etc.); oracle price feeds; additional pairs and router usage.
- **Q4 2026**: Treasury and fee parameterization; path to governance/DAO; scale fee revenue and protocol sustainability.

---

## 8. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Audit finds critical issues | Budget and timeline include remediation; we prioritize security over speed. |
| Low initial liquidity | Bootstrap with 1–2 pairs; partner with protocols or LPs; clear value prop (MEV resistance). |
| Stylus adoption curve | We document and open-source; aim to be the reference MEV-resistant DEX on Stylus. |

---

## 9. Links and contact

- **Repository:** https://github.com/OakProtocolDev/OakProtocol  
- **Contact:** oak.protocol.2025@gmail.com  
- **Discord (grant inquiries):** Oak.node  

*(Testnet app URL will be added once deployment is public.)*

---

## 10. One-sentence pitch

Oak Protocol is the first MEV-protected DEX on Arbitrum Stylus; we need $35,000 to audit and launch on mainnet so traders get fair execution and the ecosystem gets a production Stylus DeFi showcase.
