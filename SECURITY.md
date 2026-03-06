# Security Policy & Responsible Disclosure

## Reporting a Vulnerability

If you believe you have found a security vulnerability in Oak Protocol (contracts, SDK, or related tooling), please report it responsibly.

**Do not** open a public GitHub issue for security-sensitive findings.

### How to Report

1. **Email:** [oak.protocol.2025@gmail.com](mailto:oak.protocol.2025@gmail.com)  
   Subject: `[Oak Security] Brief description`

2. **Include:**
   - Description of the vulnerability and impact
   - Steps to reproduce (or proof-of-concept if possible)
   - Suggested fix (optional)
   - Your contact details if you want to be credited

### What to Expect

- We will acknowledge receipt within **48 hours** and aim to provide an initial assessment within **7 days**.
- We will keep you updated on remediation and, with your permission, credit you in our advisory and/or hall of fame after the fix is deployed.

### Scope

- **In scope:** Oak Protocol smart contracts (Rust/Stylus), commit-reveal logic, fee and reserve accounting, access control, flash swaps, and any deployed instances we maintain.
- **Out of scope:** Third-party frontends we do not control, general Ethereum/Arbitrum network issues, social engineering, and issues requiring physical access or compromise of private keys.

### Bug Bounty

A formal bug bounty program is planned **after mainnet launch**. Until then, we appreciate responsible disclosure and will recognize researchers in our security acknowledgments.

---

**Security documentation:** See [SECURITY_AUDIT.md](./SECURITY_AUDIT.md) for threat model, mitigations, and audit checklist.
