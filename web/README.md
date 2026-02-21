# Oak Protocol Web

Next.js 14 (App Router) frontend for Oak Protocol DEX on Arbitrum Sepolia.

## Setup

```bash
npm install
cp .env.example .env.local
# Set NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID in .env.local (get from https://cloud.walletconnect.com)
npm run dev
```

## Peer dependencies

All required peer dependencies are already in `package.json`:

- `@tanstack/react-query` (RainbowKit)
- `react` / `react-dom` (>=18)
- `viem` 2.x
- `wagmi` ^2.9.0

If you see peer warnings for **pino-pretty** or **@react-native-async-storage/async-storage**, they come from transitive dependencies and are optional for this app. To silence them (optional):

```bash
npm install pino-pretty @react-native-async-storage/async-storage --save-optional
```

Or ignore them; the app runs without them.

## Environment

- `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` – WalletConnect Cloud project ID (client-safe; no secrets).

Do **not** put private keys or API secrets in any `NEXT_PUBLIC_*` variable.
