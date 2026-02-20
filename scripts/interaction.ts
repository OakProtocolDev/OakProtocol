/**
 * Oak Protocol Interaction Examples
 * 
 * This script demonstrates how to interact with Oak Protocol's commit-reveal mechanism.
 * 
 * Usage:
 *   npx ts-node scripts/interaction.ts <command> [args...]
 * 
 * Commands:
 *   init <contract> <owner> <treasury>  - Initialize the contract
 *   commit <contract> <amount> <salt>   - Create a swap commitment
 *   reveal <contract> <token0> <token1> <amount> <salt> <minOut> - Execute swap
 *   addLiquidity <contract> <token0> <token1> <amount0> <amount1> - Add liquidity
 */

import { ethers } from "ethers";
import * as crypto from "crypto";

// Arbitrum Sepolia configuration
const ARBITRUM_SEPOLIA_RPC = "https://sepolia-rollup.arbitrum.io/rpc";
const CHAIN_ID = 421614; // Arbitrum Sepolia

// ABI for Oak Protocol (minimal interface)
const OAK_PROTOCOL_ABI = [
    "function init(address initialOwner, address treasury) external",
    "function commitSwap(bytes32 hash) external",
    "function revealSwap(address token0, address token1, uint256 amountIn, uint256 salt, uint256 minAmountOut) external",
    "function addLiquidity(address token0, address token1, uint256 amount0, uint256 amount1) external",
    "function paused() external view returns (bool)",
    "event CommitSwap(address indexed user, bytes32 hash, uint256 blockNumber)",
    "event RevealSwap(address indexed user, uint256 amountIn, uint256 amountOut, uint256 treasuryFee, uint256 lpFee)",
];

/**
 * Generate a commitment hash from amount and salt.
 * 
 * @param amountIn - Input token amount (as BigNumber or string)
 * @param salt - Random salt (as BigNumber or string)
 * @returns keccak256 hash of abi.encode(amountIn, salt)
 */
function generateCommitHash(amountIn: ethers.BigNumber, salt: ethers.BigNumber): string {
    // ABI encode: (uint256, uint256)
    const encoder = new ethers.utils.AbiCoder();
    const encoded = encoder.encode(
        ["uint256", "uint256"],
        [amountIn, salt]
    );
    
    // keccak256 hash
    return ethers.utils.keccak256(encoded);
}

/**
 * Generate a random salt for commitment.
 */
function generateSalt(): ethers.BigNumber {
    const randomBytes = crypto.randomBytes(32);
    return ethers.BigNumber.from(randomBytes);
}

/**
 * Wait for a specified number of blocks.
 */
async function waitForBlocks(provider: ethers.providers.Provider, blocks: number): Promise<void> {
    const currentBlock = await provider.getBlockNumber();
    const targetBlock = currentBlock + blocks;
    
    console.log(`⏳ Waiting for ${blocks} blocks (current: ${currentBlock}, target: ${targetBlock})...`);
    
    return new Promise((resolve) => {
        const checkBlock = async () => {
            const block = await provider.getBlockNumber();
            if (block >= targetBlock) {
                console.log(`✅ Block ${targetBlock} reached`);
                resolve();
            } else {
                setTimeout(checkBlock, 2000); // Check every 2 seconds
            }
        };
        checkBlock();
    });
}

/**
 * Initialize the Oak Protocol contract.
 */
async function initializeContract(
    contractAddress: string,
    ownerAddress: string,
    treasuryAddress: string,
    signer: ethers.Signer
): Promise<void> {
    console.log("\n⚙️  Initializing Oak Protocol contract...");
    console.log(`   Contract: ${contractAddress}`);
    console.log(`   Owner: ${ownerAddress}`);
    console.log(`   Treasury: ${treasuryAddress}`);
    
    const contract = new ethers.Contract(contractAddress, OAK_PROTOCOL_ABI, signer);
    
    try {
        const tx = await contract.init(ownerAddress, treasuryAddress);
        console.log(`📤 Transaction sent: ${tx.hash}`);
        
        const receipt = await tx.wait();
        console.log(`✅ Contract initialized in block ${receipt.blockNumber}`);
    } catch (error: any) {
        console.error(`❌ Initialization failed: ${error.message}`);
        throw error;
    }
}

/**
 * Create a swap commitment.
 */
async function commitSwap(
    contractAddress: string,
    amountIn: string,
    salt: string,
    signer: ethers.Signer
): Promise<{ hash: string; blockNumber: number }> {
    console.log("\n🔒 Creating swap commitment...");
    console.log(`   Amount In: ${amountIn}`);
    console.log(`   Salt: ${salt}`);
    
    const amountBn = ethers.BigNumber.from(amountIn);
    const saltBn = ethers.BigNumber.from(salt);
    
    // Generate commitment hash
    const commitHash = generateCommitHash(amountBn, saltBn);
    console.log(`   Commitment Hash: ${commitHash}`);
    
    const contract = new ethers.Contract(contractAddress, OAK_PROTOCOL_ABI, signer);
    
    try {
        const tx = await contract.commitSwap(commitHash);
        console.log(`📤 Transaction sent: ${tx.hash}`);
        
        const receipt = await tx.wait();
        console.log(`✅ Commitment created in block ${receipt.blockNumber}`);
        
        // Parse event
        const event = receipt.events?.find((e: any) => e.event === "CommitSwap");
        if (event) {
            console.log(`   Event: User ${event.args.user}, Block ${event.args.blockNumber}`);
        }
        
        return {
            hash: commitHash,
            blockNumber: receipt.blockNumber
        };
    } catch (error: any) {
        console.error(`❌ Commit failed: ${error.message}`);
        throw error;
    }
}

/**
 * Reveal and execute a swap.
 */
async function revealSwap(
    contractAddress: string,
    token0: string,
    token1: string,
    amountIn: string,
    salt: string,
    minAmountOut: string,
    signer: ethers.Signer
): Promise<void> {
    console.log("\n🔓 Revealing and executing swap...");
    console.log(`   Token0: ${token0}`);
    console.log(`   Token1: ${token1}`);
    console.log(`   Amount In: ${amountIn}`);
    console.log(`   Salt: ${salt}`);
    console.log(`   Min Amount Out: ${minAmountOut}`);
    
    const contract = new ethers.Contract(contractAddress, OAK_PROTOCOL_ABI, signer);
    
    try {
        const tx = await contract.revealSwap(
            token0,
            token1,
            amountIn,
            salt,
            minAmountOut
        );
        console.log(`📤 Transaction sent: ${tx.hash}`);
        
        const receipt = await tx.wait();
        console.log(`✅ Swap executed in block ${receipt.blockNumber}`);
        
        // Parse event
        const event = receipt.events?.find((e: any) => e.event === "RevealSwap");
        if (event) {
            console.log(`   Amount Out: ${event.args.amountOut.toString()}`);
            console.log(`   Treasury Fee: ${event.args.treasuryFee.toString()}`);
            console.log(`   LP Fee: ${event.args.lpFee.toString()}`);
        }
    } catch (error: any) {
        console.error(`❌ Reveal failed: ${error.message}`);
        throw error;
    }
}

/**
 * Add liquidity to the pool.
 */
async function addLiquidity(
    contractAddress: string,
    token0: string,
    token1: string,
    amount0: string,
    amount1: string,
    signer: ethers.Signer
): Promise<void> {
    console.log("\n💧 Adding liquidity...");
    console.log(`   Token0: ${token0}, Amount: ${amount0}`);
    console.log(`   Token1: ${token1}, Amount: ${amount1}`);
    
    const contract = new ethers.Contract(contractAddress, OAK_PROTOCOL_ABI, signer);
    
    try {
        const tx = await contract.addLiquidity(token0, token1, amount0, amount1);
        console.log(`📤 Transaction sent: ${tx.hash}`);
        
        const receipt = await tx.wait();
        console.log(`✅ Liquidity added in block ${receipt.blockNumber}`);
    } catch (error: any) {
        console.error(`❌ Add liquidity failed: ${error.message}`);
        throw error;
    }
}

/**
 * Complete commit-reveal swap flow.
 */
async function completeSwapFlow(
    contractAddress: string,
    token0: string,
    token1: string,
    amountIn: string,
    minAmountOut: string,
    signer: ethers.Signer,
    provider: ethers.providers.Provider
): Promise<void> {
    console.log("\n" + "=".repeat(60));
    console.log("🌳 Oak Protocol: Complete Commit-Reveal Swap Flow");
    console.log("=".repeat(60));
    
    // Step 1: Generate salt and commitment hash
    const salt = generateSalt();
    console.log(`\n1️⃣  Generated salt: ${salt.toString()}`);
    
    // Step 2: Commit swap
    const { blockNumber: commitBlock } = await commitSwap(
        contractAddress,
        amountIn,
        salt.toString(),
        signer
    );
    
    // Step 3: Wait for 5 blocks (commit-reveal delay)
    await waitForBlocks(provider, 5);
    
    // Step 4: Reveal and execute swap
    await revealSwap(
        contractAddress,
        token0,
        token1,
        amountIn,
        salt.toString(),
        minAmountOut,
        signer
    );
    
    console.log("\n" + "=".repeat(60));
    console.log("✅ Swap completed successfully!");
    console.log("=".repeat(60));
}

// Main CLI handler
async function main() {
    const args = process.argv.slice(2);
    
    if (args.length === 0) {
        console.log(`
Usage: npx ts-node scripts/interaction.ts <command> [args...]

Commands:
  init <contract> <owner> <treasury>
    Initialize the contract
    
  commit <contract> <amount> <salt>
    Create a swap commitment
    
  reveal <contract> <token0> <token1> <amount> <salt> <minOut>
    Execute a swap after revealing
    
  swap <contract> <token0> <token1> <amount> <minOut>
    Complete commit-reveal flow (generates salt automatically)
    
  addLiquidity <contract> <token0> <token1> <amount0> <amount1>
    Add liquidity to the pool

Environment Variables:
  PRIVATE_KEY - Your wallet private key
  RPC_URL - RPC endpoint (default: Arbitrum Sepolia)
        `);
        process.exit(0);
    }
    
    const command = args[0];
    const privateKey = process.env.PRIVATE_KEY;
    
    if (!privateKey) {
        console.error("❌ Error: PRIVATE_KEY environment variable not set");
        process.exit(1);
    }
    
    const rpcUrl = process.env.RPC_URL || ARBITRUM_SEPOLIA_RPC;
    const provider = new ethers.providers.JsonRpcProvider(rpcUrl);
    const signer = new ethers.Wallet(privateKey, provider);
    
    console.log(`🔗 Connected to: ${rpcUrl}`);
    console.log(`👤 Signer: ${await signer.getAddress()}`);
    
    try {
        switch (command) {
            case "init":
                if (args.length !== 4) {
                    console.error("Usage: init <contract> <owner> <treasury>");
                    process.exit(1);
                }
                await initializeContract(args[1], args[2], args[3], signer);
                break;
                
            case "commit":
                if (args.length !== 4) {
                    console.error("Usage: commit <contract> <amount> <salt>");
                    process.exit(1);
                }
                await commitSwap(args[1], args[2], args[3], signer);
                break;
                
            case "reveal":
                if (args.length !== 7) {
                    console.error("Usage: reveal <contract> <token0> <token1> <amount> <salt> <minOut>");
                    process.exit(1);
                }
                await revealSwap(args[1], args[2], args[3], args[4], args[5], args[6], signer);
                break;
                
            case "swap":
                if (args.length !== 6) {
                    console.error("Usage: swap <contract> <token0> <token1> <amount> <minOut>");
                    process.exit(1);
                }
                await completeSwapFlow(args[1], args[2], args[3], args[4], args[5], signer, provider);
                break;
                
            case "addLiquidity":
                if (args.length !== 6) {
                    console.error("Usage: addLiquidity <contract> <token0> <token1> <amount0> <amount1>");
                    process.exit(1);
                }
                await addLiquidity(args[1], args[2], args[3], args[4], args[5], signer);
                break;
                
            default:
                console.error(`Unknown command: ${command}`);
                process.exit(1);
        }
    } catch (error: any) {
        console.error(`\n❌ Error: ${error.message}`);
        process.exit(1);
    }
}

if (require.main === module) {
    main();
}
