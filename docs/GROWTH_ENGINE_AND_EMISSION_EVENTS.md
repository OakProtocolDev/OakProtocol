# Growth Engine & EmissionEvent (Indexer)

## Modules

1. **StakingRewards** (`src/growth/staking_rewards.rs`)  
   - Rewards for LP tokens (ERC-20 or ERC-1155).  
   - `init(reward_token, staking_token, reward_rate_per_block)` (owner), `stake(amount)`, `unstake(amount)`, `claim_rewards()`, `pending_rewards(user)`.

2. **Referral Engine** (`src/growth/referral.rs`)  
   - Mapping referee => referrer; % of protocol fee sent to referrer on each swap.  
   - `set_referrer(referrer)`, `get_referrer(referee)`, `set_referral_fee_bps(bps)` (owner).  
   - On swap: `distribute_referral_fee` is called from logic; referral share is sent to referrer and the rest goes to treasury.

3. **Quest System** (`src/growth/quest.rs`)  
   - For bonus.oak.trade: XP and Badges for trading volume.  
   - `record_volume(user, delta)` called from swap; `grant_xp(user, xp)`, `set_badge_contract(addr)` (owner), `emit_badge_minted(user, token_id)`.  
   - Views: `get_user_volume(user)`, `get_user_xp(user)`.

## EmissionEvent (for indexer / personal cabinet)

All three modules emit a **single event shape** so the indexer can subscribe once and route by `module_id`:

- **Signature**: `EmissionEvent(module_id, user, event_type, amount, token_id)`  
- **Topics**: `[user]` (indexed).  
- **Data**: 4 × 32 bytes: `module_id`, `event_type`, `amount`, `token_id`.

### module_id (data)

- `1` = Staking  
- `2` = Referral  
- `3` = Quest  

### event_type (data)

- Staking: `0` = RewardClaimed, `1` = Staked, `2` = Unstaked.  
- Referral: `3` = ReferralFee.  
- Quest: `4` = XPGranted, `5` = BadgeMinted.  

### amount / token_id

- Staking: amount = staked/claimed amount; token_id = 0.  
- Referral: amount = fee sent to referrer; token_id = 0.  
- Quest: amount = XP or 0 for badge; token_id = badge NFT id for BadgeMinted.  

Indexer should listen for this event and push to the personal cabinet (e.g. bonus.oak.trade) by `user` and `module_id`.
