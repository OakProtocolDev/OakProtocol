#!/bin/bash
#
# Oak Protocol Deployment Script for Arbitrum Sepolia
#
# This script compiles and deploys Oak Protocol to Arbitrum Sepolia testnet.
#
# Usage:
#   ./deploy.sh
#
# Environment Variables:
#   PRIVATE_KEY - Your wallet private key (required)
#   OWNER_ADDRESS - Owner address for initialization (optional)
#   TREASURY_ADDRESS - Treasury address for initialization (optional)
#   RPC_URL - RPC endpoint (default: Arbitrum Sepolia)

set -e  # Exit on error

ARBITRUM_SEPOLIA_RPC="${RPC_URL:-https://sepolia-rollup.arbitrum.io/rpc}"

echo "============================================================"
echo "🌳 Oak Protocol Deployment Script"
echo "============================================================"

# Check prerequisites
echo ""
echo "🔍 Checking prerequisites..."

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not found. Install from https://rustup.rs/"
    exit 1
fi
echo "✅ Rust installed: $(rustc --version)"

# Check cargo-stylus
if ! command -v cargo-stylus &> /dev/null; then
    echo "❌ cargo-stylus not found. Installing..."
    cargo install --force cargo-stylus || {
        echo "❌ Failed to install cargo-stylus"
        exit 1
    }
fi
echo "✅ cargo-stylus installed: $(cargo stylus --version 2>&1)"

# Check wasm32 target
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "⚠️  Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi
echo "✅ wasm32-unknown-unknown target available"

# Check private key
if [ -z "$PRIVATE_KEY" ]; then
    echo ""
    echo "❌ Error: PRIVATE_KEY environment variable not set"
    echo "   Example: export PRIVATE_KEY=0x..."
    exit 1
fi

# Compile contract
echo ""
echo "🔨 Compiling contract to WASM..."
cargo build --target wasm32-unknown-unknown --release

WASM_FILE="target/wasm32-unknown-unknown/release/oak_protocol.wasm"
if [ ! -f "$WASM_FILE" ]; then
    echo "❌ WASM file not found at $WASM_FILE"
    exit 1
fi

SIZE_KB=$(du -k "$WASM_FILE" | cut -f1)
echo "✅ Contract compiled successfully"
echo "📦 WASM size: ${SIZE_KB} KB"

# Deploy contract
echo ""
echo "🚀 Deploying contract to Arbitrum Sepolia..."
echo "⏳ This may take a few minutes..."

DEPLOY_OUTPUT=$(cargo stylus deploy \
    --wasm-file "$WASM_FILE" \
    --network sepolia \
    --rpc-url "$ARBITRUM_SEPOLIA_RPC" \
    --private-key "$PRIVATE_KEY" 2>&1)

echo "$DEPLOY_OUTPUT"

# Extract contract address (basic parsing)
CONTRACT_ADDRESS=$(echo "$DEPLOY_OUTPUT" | grep -oE "0x[a-fA-F0-9]{40}" | head -1)

if [ -z "$CONTRACT_ADDRESS" ]; then
    echo ""
    echo "⚠️  Could not parse contract address from output"
    echo "   Please check the output above for the deployed address"
    exit 1
fi

echo ""
echo "✅ Contract deployed at: $CONTRACT_ADDRESS"

# Initialize if addresses provided
if [ -n "$OWNER_ADDRESS" ] && [ -n "$TREASURY_ADDRESS" ]; then
    echo ""
    echo "⚙️  Initializing contract..."
    echo "   Owner: $OWNER_ADDRESS"
    echo "   Treasury: $TREASURY_ADDRESS"
    
    cargo stylus call \
        --address "$CONTRACT_ADDRESS" \
        --function init \
        --args "$OWNER_ADDRESS,$TREASURY_ADDRESS" \
        --network sepolia \
        --rpc-url "$ARBITRUM_SEPOLIA_RPC" \
        --private-key "$PRIVATE_KEY" || {
        echo ""
        echo "⚠️  Initialization failed"
        echo "   You can initialize manually later"
        exit 1
    }
    
    echo ""
    echo "✅ Contract initialized successfully"
else
    echo ""
    echo "⚠️  Contract deployed but not initialized"
    echo "   Set OWNER_ADDRESS and TREASURY_ADDRESS to auto-initialize:"
    echo "   export OWNER_ADDRESS=0x..."
    echo "   export TREASURY_ADDRESS=0x..."
    echo ""
    echo "   Or initialize manually:"
    echo "   cargo stylus call --address $CONTRACT_ADDRESS --function init --args <owner>,<treasury> --network sepolia --private-key \$PRIVATE_KEY"
fi

echo ""
echo "============================================================"
echo "🎉 Deployment Complete!"
echo "============================================================"
echo "Contract Address: $CONTRACT_ADDRESS"
[ -n "$OWNER_ADDRESS" ] && echo "Owner: $OWNER_ADDRESS"
[ -n "$TREASURY_ADDRESS" ] && echo "Treasury: $TREASURY_ADDRESS"
echo ""
echo "Next steps:"
echo "1. Verify contract on Arbiscan: https://sepolia.arbiscan.io/address/$CONTRACT_ADDRESS"
echo "2. Add liquidity to the pool"
echo "3. Test commit-reveal swaps using scripts/interaction.ts"
echo "============================================================"
