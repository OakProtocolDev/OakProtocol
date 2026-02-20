# Oak Protocol Interaction Scripts

This directory contains scripts for interacting with Oak Protocol on Arbitrum Sepolia.

## Setup

### TypeScript Scripts

```bash
# Install dependencies
npm install

# Or using yarn
yarn install
```

### Environment Variables

Create a `.env` file in the project root (see `.env.example`):

```bash
PRIVATE_KEY=0x...  # Your wallet private key
RPC_URL=https://sepolia-rollup.arbitrum.io/rpc  # Optional
```

## Usage

### Initialize Contract

```bash
npx ts-node scripts/interaction.ts init \
  <CONTRACT_ADDRESS> \
  <OWNER_ADDRESS> \
  <TREASURY_ADDRESS>
```

### Complete Commit-Reveal Swap

This command automatically generates a salt and performs the full commit-reveal flow:

```bash
npx ts-node scripts/interaction.ts swap \
  <CONTRACT_ADDRESS> \
  <TOKEN0_ADDRESS> \
  <TOKEN1_ADDRESS> \
  <AMOUNT_IN> \
  <MIN_AMOUNT_OUT>
```

Example:
```bash
npx ts-node scripts/interaction.ts swap \
  0x1234... \
  0xToken0... \
  0xToken1... \
  1000000000000000000 \
  950000000000000000
```

### Manual Commit-Reveal Flow

If you want to control each step:

1. **Generate salt and commit hash** (in your code):
   ```typescript
   const salt = ethers.BigNumber.from(ethers.utils.randomBytes(32));
   const amountIn = ethers.utils.parseEther("1.0");
   const commitHash = ethers.utils.keccak256(
     ethers.utils.defaultAbiCoder.encode(
       ["uint256", "uint256"],
       [amountIn, salt]
     )
   );
   ```

2. **Commit swap**:
   ```bash
   npx ts-node scripts/interaction.ts commit \
     <CONTRACT_ADDRESS> \
     <AMOUNT_IN> \
     <SALT>
   ```

3. **Wait for 5 blocks** (manually or use a block explorer)

4. **Reveal swap**:
   ```bash
   npx ts-node scripts/interaction.ts reveal \
     <CONTRACT_ADDRESS> \
     <TOKEN0_ADDRESS> \
     <TOKEN1_ADDRESS> \
     <AMOUNT_IN> \
     <SALT> \
     <MIN_AMOUNT_OUT>
   ```

### Add Liquidity

```bash
npx ts-node scripts/interaction.ts addLiquidity \
  <CONTRACT_ADDRESS> \
  <TOKEN0_ADDRESS> \
  <TOKEN1_ADDRESS> \
  <AMOUNT0> \
  <AMOUNT1>
```

## Example Workflow

```bash
# 1. Set environment variables
export PRIVATE_KEY=0x...
export CONTRACT=0x...  # Deployed contract address
export TOKEN0=0x...    # Token0 address
export TOKEN1=0x...    # Token1 address

# 2. Add liquidity (first time setup)
npx ts-node scripts/interaction.ts addLiquidity \
  $CONTRACT $TOKEN0 $TOKEN1 \
  100000000000000000000 \
  200000000000000000000

# 3. Perform a swap (complete flow)
npx ts-node scripts/interaction.ts swap \
  $CONTRACT $TOKEN0 $TOKEN1 \
  1000000000000000000 \
  950000000000000000
```

## Notes

- All amounts should be in wei (18 decimals for most tokens)
- The script automatically waits for 5 blocks between commit and reveal
- Make sure you have approved the contract to spend your tokens before swapping
- Ensure you have sufficient ETH for gas fees
