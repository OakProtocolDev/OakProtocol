#!/usr/bin/env python3
"""
Oak Protocol Deployment Script for Arbitrum Sepolia Testnet

This script:
1. Compiles the Rust contract to WASM
2. Deploys to Arbitrum Sepolia using cargo-stylus
3. Initializes the contract with owner and treasury addresses

Prerequisites:
- Rust and cargo-stylus installed
- Private key or mnemonic in environment variables
- Sufficient ETH on Arbitrum Sepolia for deployment
"""

import os
import subprocess
import sys
from pathlib import Path
from typing import Optional

# Arbitrum Sepolia RPC endpoint
ARBITRUM_SEPOLIA_RPC = "https://sepolia-rollup.arbitrum.io/rpc"

# Stylus program ID (will be set after deployment)
PROGRAM_ID_ENV = "STYLUS_PROGRAM_ID"


def check_prerequisites() -> bool:
    """Check if required tools are installed."""
    print("🔍 Checking prerequisites...")
    
    # Check Rust
    try:
        subprocess.run(["rustc", "--version"], check=True, capture_output=True)
        print("✅ Rust installed")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("❌ Rust not found. Install from https://rustup.rs/")
        return False
    
    # Check cargo-stylus
    try:
        result = subprocess.run(
            ["cargo", "stylus", "--version"],
            check=True,
            capture_output=True,
            text=True
        )
        print(f"✅ cargo-stylus installed: {result.stdout.strip()}")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("❌ cargo-stylus not found. Install with: cargo install --force cargo-stylus")
        return False
    
    # Check wasm32 target
    try:
        subprocess.run(
            ["rustup", "target", "list", "--installed"],
            check=True,
            capture_output=True
        )
        result = subprocess.run(
            ["rustup", "target", "list", "--installed"],
            check=True,
            capture_output=True,
            text=True
        )
        if "wasm32-unknown-unknown" not in result.stdout:
            print("⚠️  Installing wasm32-unknown-unknown target...")
            subprocess.run(
                ["rustup", "target", "add", "wasm32-unknown-unknown"],
                check=True
            )
        print("✅ wasm32-unknown-unknown target available")
    except subprocess.CalledProcessError:
        print("❌ Failed to check/install wasm32 target")
        return False
    
    return True


def compile_contract() -> bool:
    """Compile the Rust contract to WASM."""
    print("\n🔨 Compiling contract to WASM...")
    
    try:
        result = subprocess.run(
            ["cargo", "build", "--target", "wasm32-unknown-unknown", "--release"],
            check=True,
            capture_output=True,
            text=True
        )
        print("✅ Contract compiled successfully")
        
        # Check if WASM file exists
        wasm_path = Path("target/wasm32-unknown-unknown/release/oak_protocol.wasm")
        if wasm_path.exists():
            size_kb = wasm_path.stat().st_size / 1024
            print(f"📦 WASM size: {size_kb:.2f} KB")
            return True
        else:
            print("⚠️  WASM file not found at expected location")
            return False
            
    except subprocess.CalledProcessError as e:
        print(f"❌ Compilation failed: {e.stderr}")
        return False


def deploy_contract(
    private_key: Optional[str] = None,
    mnemonic: Optional[str] = None,
    rpc_url: str = ARBITRUM_SEPOLIA_RPC
) -> Optional[str]:
    """
    Deploy the contract to Arbitrum Sepolia.
    
    Returns the deployed contract address or None on failure.
    """
    print("\n🚀 Deploying contract to Arbitrum Sepolia...")
    
    if not private_key and not mnemonic:
        print("❌ Either PRIVATE_KEY or MNEMONIC environment variable must be set")
        return None
    
    # Build cargo-stylus deploy command
    cmd = [
        "cargo", "stylus", "deploy",
        "--wasm-file", "target/wasm32-unknown-unknown/release/oak_protocol.wasm",
        "--network", "sepolia",
        "--rpc-url", rpc_url,
    ]
    
    if private_key:
        cmd.extend(["--private-key", private_key])
    elif mnemonic:
        cmd.extend(["--mnemonic", mnemonic])
    
    try:
        print("⏳ Deployment in progress (this may take a few minutes)...")
        result = subprocess.run(
            cmd,
            check=True,
            capture_output=True,
            text=True
        )
        
        # Parse output for contract address
        output = result.stdout + result.stderr
        print(output)
        
        # Look for contract address in output
        # cargo-stylus typically outputs: "Contract deployed at: 0x..."
        for line in output.split("\n"):
            if "deployed" in line.lower() or "address" in line.lower():
                print(f"✅ {line}")
                # Try to extract address
                if "0x" in line:
                    parts = line.split("0x")
                    if len(parts) > 1:
                        address = "0x" + parts[1].split()[0][:40]
                        return address
        
        print("⚠️  Could not parse contract address from output")
        return None
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Deployment failed: {e.stderr}")
        return None


def initialize_contract(
    contract_address: str,
    owner_address: str,
    treasury_address: str,
    private_key: Optional[str] = None,
    mnemonic: Optional[str] = None,
    rpc_url: str = ARBITRUM_SEPOLIA_RPC
) -> bool:
    """
    Initialize the deployed contract with owner and treasury addresses.
    """
    print(f"\n⚙️  Initializing contract at {contract_address}...")
    print(f"   Owner: {owner_address}")
    print(f"   Treasury: {treasury_address}")
    
    # Use cargo-stylus to call init function
    cmd = [
        "cargo", "stylus", "call",
        "--address", contract_address,
        "--function", "init",
        "--args", f"{owner_address},{treasury_address}",
        "--network", "sepolia",
        "--rpc-url", rpc_url,
    ]
    
    if private_key:
        cmd.extend(["--private-key", private_key])
    elif mnemonic:
        cmd.extend(["--mnemonic", mnemonic])
    
    try:
        result = subprocess.run(
            cmd,
            check=True,
            capture_output=True,
            text=True
        )
        print("✅ Contract initialized successfully")
        print(result.stdout)
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Initialization failed: {e.stderr}")
        return False


def main():
    """Main deployment flow."""
    print("=" * 60)
    print("🌳 Oak Protocol Deployment Script")
    print("=" * 60)
    
    # Check prerequisites
    if not check_prerequisites():
        sys.exit(1)
    
    # Compile contract
    if not compile_contract():
        sys.exit(1)
    
    # Get deployment credentials
    private_key = os.getenv("PRIVATE_KEY")
    mnemonic = os.getenv("MNEMONIC")
    
    if not private_key and not mnemonic:
        print("\n❌ Error: Set PRIVATE_KEY or MNEMONIC environment variable")
        print("   Example: export PRIVATE_KEY=0x...")
        sys.exit(1)
    
    # Get addresses for initialization
    owner_address = os.getenv("OWNER_ADDRESS")
    treasury_address = os.getenv("TREASURY_ADDRESS")
    
    if not owner_address or not treasury_address:
        print("\n⚠️  Warning: OWNER_ADDRESS and TREASURY_ADDRESS not set")
        print("   Contract will be deployed but not initialized")
        print("   Set them to auto-initialize:")
        print("   export OWNER_ADDRESS=0x...")
        print("   export TREASURY_ADDRESS=0x...")
    
    # Deploy contract
    contract_address = deploy_contract(private_key=private_key, mnemonic=mnemonic)
    
    if not contract_address:
        print("\n❌ Deployment failed")
        sys.exit(1)
    
    print(f"\n✅ Contract deployed at: {contract_address}")
    
    # Initialize if addresses provided
    if owner_address and treasury_address:
        if initialize_contract(contract_address, owner_address, treasury_address,
                              private_key=private_key, mnemonic=mnemonic):
            print("\n" + "=" * 60)
            print("🎉 Deployment Complete!")
            print("=" * 60)
            print(f"Contract Address: {contract_address}")
            print(f"Owner: {owner_address}")
            print(f"Treasury: {treasury_address}")
            print("\nNext steps:")
            print("1. Verify contract on Arbiscan")
            print("2. Add liquidity to the pool")
            print("3. Test commit-reveal swaps")
        else:
            print("\n⚠️  Contract deployed but initialization failed")
            print("   You can initialize manually using the interaction scripts")
    else:
        print("\n⚠️  Contract deployed but not initialized")
        print("   Set OWNER_ADDRESS and TREASURY_ADDRESS and run:")
        print(f"   python3 scripts/interaction.py init {contract_address} {owner_address} {treasury_address}")


if __name__ == "__main__":
    main()
