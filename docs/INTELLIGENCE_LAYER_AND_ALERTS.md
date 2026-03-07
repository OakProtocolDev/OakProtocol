# Intelligence Layer & Alerts (alerts.oak.trade)

## Overview

The Intelligence Layer adds **Copy Trading** and **Signal Marketplace** to the Oak Protocol contract. A backend service monitors contract logs and pushes real-time notifications to **alerts.oak.trade** (Telegram/Discord bot).

---

## 1. Copy Trading (contract)

### Storage (keyed by follower)

- `copy_trading_leader`: follower → leader address (0 = no subscription).
- `copy_trading_slippage_bps`: max slippage in basis points (e.g. 50 = 0.5%).
- `copy_trading_amount_ratio_bps`: fraction of leader amount to copy (e.g. 5000 = 50%).

### Entrypoints

- **subscribe(leader, slippage_bps, amount_ratio_bps)**  
  - Caller = follower. Sets subscription; overwrites any existing one. One leader per follower.
- **unsubscribe()**  
  - Caller = follower. Clears subscription. **Revocable at any time** (security requirement).
- **execute_copy_trade(follower, leader, token_in, token_out, leader_amount_in, deadline)**  
  - Callable by anyone (relayer/backend). Checks subscription, computes follower amount and min_out from slippage, executes swap from follower’s balance, emits event.

### Events (for indexer / alerts)

- **CopySubscription(follower, leader, slippage_bps, amount_ratio_bps)**  
  - Subscription created or updated.
- **CopySubscriptionRevoked(follower, leader)**  
  - Follower revoked subscription.
- **CopyTradeExecuted(follower, leader, amount_in, amount_out)**  
  - Copy trade executed; backend uses this for push notifications.

### Safety

- Only the **follower** can revoke their own subscription (unsubscribe). No one else can clear it.

---

## 2. Signal Marketplace (contract)

### Model

- **Signal** = encrypted content; identified by `signal_id_hash` (bytes32). Content key is delivered **off-chain** after payment.
- **Listing**: seller sets price (in protocol token) per `signal_id_hash`.
- **Purchase**: buyer pays with EIP-712 signed listing; contract transfers protocol tokens to seller and marks listing as purchased; backend delivers content key.

### Storage

- `signal_price`: seller → (signal_id_hash → price). 0 = delisted.
- `signal_purchased`: buyer → (listing_hash → true). listing_hash = EIP-712 struct hash of the listing.
- `signal_nonce`: per-seller nonce for EIP-712 replay protection.

### EIP-712

- **Type**: `SignalListing(address seller, bytes32 signalIdHash, uint256 price, uint256 nonce, uint256 deadline)`.
- **Domain**: same as Oak Protocol (name "Oak Protocol", version "1", chainId, verifyingContract).
- **listing_hash** = struct hash of the above (used as key in `signal_purchased` and in events).

### Entrypoints

- **list_signal(signal_id_hash, price)** — caller = seller.
- **delist_signal(signal_id_hash)** — caller = seller; sets price to 0.
- **purchase_signal(buyer, seller, signal_id_hash, price, nonce, deadline, protocol_token, v, r, s)** — verifies EIP-712 signature, transfers protocol token from buyer to seller, sets purchased, bumps seller nonce, emits.

### Event

- **SignalPurchased(buyer, seller, listing_hash, price)**  
  - Backend uses this to deliver the encrypted content key to the buyer (e.g. via alerts.oak.trade or a dedicated API).

---

## 3. WebSocket / Backend architecture (alerts.oak.trade)

### Components

1. **Chain log subscriber (Node.js or Go)**  
   - Connects to Arbitrum RPC (e.g. wss or polling).  
   - Subscribes to contract logs for:  
     - Copy: `CopySubscription`, `CopySubscriptionRevoked`, `CopyTradeExecuted`.  
     - Signal: `SignalPurchased`.  
     - Optional: `RevealSwap`, `EmissionEvent` for broader alerts.

2. **Event normalizer**  
   - Parses log topics and data into a canonical event shape (e.g. JSON: `{ type, chainId, contract, block, tx, payload }`).

3. **Event queue**  
   - In-memory or Redis queue so that burst of events don’t block the notifier.

4. **WebSocket server**  
   - Serves real-time event stream to connected clients (e.g. alerts.oak.trade frontend or bot backend).

5. **alerts.oak.trade (Telegram/Discord bot)**  
   - Consumes events (from queue or WebSocket).  
   - Sends push notifications, e.g.:  
     - “Copy trade executed: follower X copied leader Y, amount_in / amount_out.”  
     - “Subscription revoked: follower X stopped copying leader Y.”  
     - “Signal purchased: buyer X bought signal from seller Y; content key delivered via DM.”

### Data flow (high level)

```
Arbitrum RPC (logs)
    → Log subscriber
    → Event normalizer
    → Event queue
    → WebSocket server  ←→  alerts.oak.trade (Telegram/Discord bot)
    → Push notifications
```

### Implementation options

- **Node.js**: `ethers` / `viem` for log subscription; `ws` for WebSocket server; Telegram/Discord bots via their official APIs.
- **Go**: `go-ethereum` for logs; `gorilla/websocket` or similar; Telegram/Discord SDKs.

### Security

- Backend should validate event source (contract address, chain id).  
- Content key delivery for Signal Marketplace must be restricted to the buyer (e.g. encrypted to buyer’s key or delivered over authenticated channel).

---

## 4. Summary

| Feature            | Contract side                          | Backend / alerts.oak.trade                    |
|--------------------|----------------------------------------|-----------------------------------------------|
| Copy Trading       | subscribe / unsubscribe / execute_copy | Push on CopyTradeExecuted, CopySubscriptionRevoked |
| Signal Marketplace | list / delist / purchase (EIP-712)     | Push on SignalPurchased; deliver content key  |
| Safety             | Subscriptions revocable by follower   | N/A                                           |
