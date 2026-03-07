# Oak Protocol — Full Grant Application (Arbitrum Grant)

---

## 1. Project name and tagline

**Oak Protocol** — The first MEV-protected DEX on Arbitrum Stylus. Fair DeFi, zero front-running, built in Rust.

---

## 2. Team

**Experience (Rust / Stylus):**

- **Rust & systems**: 4+ years systems programming; Rust used for performance-critical and safety-critical code (no_std, WASM, Stylus). Oak Protocol core is 100% Rust: Stylus `sol_storage!`, CPMM math, multi-pool router, commit–reveal, flash swaps, TimelockController, circuit breaker, and TWAP deviation emergency logic.
- **Stylus / Arbitrum**: Native Stylus development: contract layout, EVM interop (raw_log events, precompiles), gas-conscious patterns. Oak is designed for Arbitrum One deployment and Stylus toolchain (cargo-stylus, WASM).
- **DeFi / smart contracts**: Prior DeFi tooling and on-chain protocol logic; internal security review (threat model, in-house tests, audit-ready docs). Shipped full codebase: core engine, growth layer, intelligence layer (copy trading, signal marketplace), and ecosystem UI (profile, leaderboard, bridge apps).

Solo founder; grant funds will go to **Testnet completion** and **external security audit**, not marketing.

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

## 5. Milestones and use of funds

**Budget split (transparent):**

| Item | Amount | Purpose |
|------|--------|---------|
| **Testnet completion** | $10,000 | Finalize Testnet MVP: deployment, integration tests on testnet, documentation, and any remaining engineering to reach “audit-ready” state. |
| **Security audit** | $10,000 | External audit by a recognized firm; remediation of findings; public report or summary. We plan to engage **2–3 top-tier auditors** (e.g. **Spearbit**, **OpenZeppelin**, **Code4rena** or similar) for quotes and select one within the grant timeline. |
| **Reserve / deployment** | As needed | Post-audit fixes, mainnet deployment, and minimal bootstrap (e.g. 1–2 pools). |

**Milestones (clear and verifiable):**

| Milestone | Deliverable | Success criteria |
|-----------|-------------|------------------|
| **Milestone 1: Testnet MVP** | Deploy Oak Stylus contract to Arbitrum testnet; run integration tests on-chain; document deployment and test logs. | Contract live on testnet; test suite green; audit-ready code and docs (SECURITY_AUDIT.md, threat model, events for The Graph). |
| **Milestone 2: Security audit completion** | Sign engagement with chosen auditor (Spearbit / OpenZeppelin / other); complete audit; remediate critical/high findings; receive and publish report or summary. | Signed audit report or summary; no open critical/high issues (or documented acceptance of risk). |

**Total ask:** $20,000 ($10k Testnet + $10k audit). Any remainder can go to reserve or mainnet deployment.

---

## 6. Traction and proof of work

- **Code**: [https://github.com/OakProtocolDev/OakProtocol](https://github.com/OakProtocolDev/OakProtocol). Rust contracts + Next.js app; stress tests (fees, re-entrancy, dust, limits), router integration tests, and general integration tests.
- **Security**: Internal review in `SECURITY_AUDIT.md`; audit-ready summary in `docs/SECURITY_AUDIT.md` (threat model, governance/Timelock, formal verification plan). Testnet deployment and public dashboard URL will be live upon grant acceptance.
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

Oak Protocol is the first MEV-protected DEX on Arbitrum Stylus; we need grant support to complete the Testnet MVP and fund a professional security audit so we can launch on mainnet with confidence and give the ecosystem a production Stylus DeFi reference.

---

## 11. Response to potential rejection / FAQ

**“Why fund a solo founder?”**  
The codebase is production-sized (Rust/Stylus core, tests, docs, ecosystem UI). Delivery is evidenced by the repo and internal security review. Grant funds are allocated to **audit and Testnet completion**, not salary; we are seeking execution capital, not team build-out.

**“Why $10k for audit?”**  
We are requesting a **focused audit budget** ($10k) and will obtain quotes from **Spearbit, OpenZeppelin, and one other top auditor**. If the chosen firm’s quote exceeds $10k, we will use reserve or phase the scope (e.g. core + critical paths first). The amount is stated transparently so the committee can assess feasibility.

**“What if the fund previously declined?”**  
We have refined the application: **clear milestones** (Testnet MVP → Audit completion), **explicit budget split** ($10k Testnet, $10k audit), **named auditors** we plan to contact, and **stronger team section** (Rust/Stylus experience). We are open to feedback (e.g. different milestone order or audit scope) and can resubmit with adjustments.
