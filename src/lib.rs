//! Oak Protocol - Next-generation DEX for Arbitrum Stylus
//! 
//! This DEX implements a Commit-Reveal system to protect users from
//! front-running and sandwich attacks (MEV protection).

#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    block,
    crypto,
    msg,
    prelude::*,
    storage::{StorageAddress, StorageBool, StorageMap, StorageU256},
};

/// Commitment structure for Commit-Reveal mechanism
/// 
/// This structure stores the commitment data for each user:
/// - hash: The hash of the commitment (keccak256 of the reveal data)
/// - timestamp: When the commitment was created
/// - activated: Whether the commitment has been activated/revealed
#[derive(Clone, Copy)]
pub struct Commitment {
    /// Hash of the commitment (keccak256 of reveal data)
    pub hash: U256,
    /// Timestamp when commitment was created
    pub timestamp: U256,
    /// Whether the commitment has been activated
    pub activated: bool,
}

/// Main storage structure for OakDEX
/// 
/// This structure holds all the state for the DEX:
/// - reserves0: Reserve of token0 in the liquidity pool
/// - reserves1: Reserve of token1 in the liquidity pool
/// - commitment_hashes: Mapping from user address to commitment hash
/// - commitment_timestamps: Mapping from user address to commitment timestamp
/// - commitment_activated: Mapping from user address to commitment activation status
sol_storage! {
    pub struct OakDEX {
        /// Reserve of token0 in the liquidity pool
        StorageU256 reserves0;
        /// Reserve of token1 in the liquidity pool
        StorageU256 reserves1;
        /// Minimum liquidity that must remain in the pool (to prevent draining)
        StorageU256 min_liquidity;
        /// Protocol fee in basis points (e.g., 30 = 0.3%)
        StorageU256 protocol_fee_bps;
        /// Owner address (can change protocol settings)
        StorageAddress owner;
        /// Total trading volume for token0 (for analytics)
        StorageU256 total_volume_token0;
        /// Total trading volume for token1 (for analytics)
        StorageU256 total_volume_token1;
        /// Emergency pause switch (if true, swaps are frozen)
        StorageBool paused;
        /// Mapping from user address to commitment hash
        StorageMap<Address, StorageU256> commitment_hashes;
        /// Mapping from user address to commitment timestamp
        StorageMap<Address, StorageU256> commitment_timestamps;
        /// Mapping from user address to commitment activation status
        StorageMap<Address, StorageBool> commitment_activated;
    }
}

// Protocol constants
const DEFAULT_FEE_BPS: u64 = 30; // Default fee 0.3% (30 basis points)
const FEE_DENOMINATOR: u64 = 10000; // Denominator for basis points (10000 = 100%)
const MINIMUM_LIQUIDITY: u64 = 1000; // Minimum liquidity to prevent pool draining
const COMMIT_REVEAL_DELAY: u64 = 5; // Minimum blocks between commit and reveal (~1 minute)
const MAX_FEE_BPS: u64 = 1000; // Maximum fee 10% (1000 basis points) to prevent abuse

/// Private function to calculate output token amount with dynamic fee
///
/// Uses formula: amount_out = (amount_in_with_fee * reserve_out) / (reserve_in * FEE_DENOMINATOR + amount_in_with_fee)
/// where amount_in_with_fee = amount_in * (FEE_DENOMINATOR - fee_bps)
///
/// # Arguments
/// * `amount_in` - input token amount
/// * `reserve_in` - reserve of input token
/// * `reserve_out` - reserve of output token
/// * `fee_bps` - fee in basis points (e.g., 30 = 0.3%)
///
/// # Returns
/// Output token amount after applying fee
fn _get_amount_out_with_fee(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
    fee_bps: U256,
) -> Result<U256, Vec<u8>> {
    // Check for zero values
    if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
        return Err(b"INSUFFICIENT_INPUT_AMOUNT".to_vec());
    }

    // Calculate fee multiplier for amount_in: (10000 - fee_bps)
    // E.g., with 30 bps fee: 10000 - 30 = 9970 (meaning 99.7% remains after fee)
    let fee_multiplier = U256::from(FEE_DENOMINATOR)
        .checked_sub(fee_bps)
        .ok_or_else(|| b"FEE_OVERFLOW".to_vec())?;

    // Calculate amount_in with fee applied: amount_in * fee_multiplier
    let amount_in_with_fee = amount_in
        .checked_mul(fee_multiplier)
        .ok_or_else(|| b"OVERFLOW".to_vec())?;

    // Calculate numerator: amount_in_with_fee * reserve_out
    let numerator = amount_in_with_fee
        .checked_mul(reserve_out)
        .ok_or_else(|| b"OVERFLOW".to_vec())?;

    // Calculate denominator: reserve_in * FEE_DENOMINATOR + amount_in_with_fee
    let denominator_part1 = reserve_in
        .checked_mul(U256::from(FEE_DENOMINATOR))
        .ok_or_else(|| b"OVERFLOW".to_vec())?;
    
    let denominator = denominator_part1
        .checked_add(amount_in_with_fee)
        .ok_or_else(|| b"OVERFLOW".to_vec())?;

    // Calculate result: numerator / denominator
    let amount_out = numerator
        .checked_div(denominator)
        .ok_or_else(|| b"DIVISION_BY_ZERO".to_vec())?;

    Ok(amount_out)
}

/// Private function to encode data in abi.encode format
///
/// Encodes amount_in and salt into bytes for subsequent hashing
/// Format: 32 bytes for amount_in + 32 bytes for salt
fn _encode_commit_data(amount_in: U256, salt: U256) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(64);
    // Append amount_in as 32 bytes (big-endian)
    encoded.extend_from_slice(&amount_in.to_be_bytes::<32>());
    // Append salt as 32 bytes (big-endian)
    encoded.extend_from_slice(&salt.to_be_bytes::<32>());
    encoded
}

/// Private function to compute commitment hash
///
/// Computes keccak256(abi.encode(amount_in, salt))
fn _compute_commit_hash(amount_in: U256, salt: U256) -> FixedBytes<32> {
    let encoded = _encode_commit_data(amount_in, salt);
    crypto::keccak(&encoded)
}

/// Private function to verify owner rights
///
/// Ensures the caller is the contract owner
///
/// # Arguments
/// * `owner` - owner address from storage
///
/// # Returns
/// Ok(()) if caller is owner, otherwise error
fn _only_owner(owner: Address) -> Result<(), Vec<u8>> {
    let sender = msg::sender();
    if sender != owner {
        return Err(b"ONLY_OWNER".to_vec());
    }
    Ok(())
}

// Public contract functions implementation
#[public]
impl OakDEX {
    /// Contract initialization function (must be called on deploy)
    ///
    /// Sets the owner and initial protocol parameters
    ///
    /// # Arguments
    /// * `initial_owner` - contract owner address
    ///
    /// # Returns
    /// Ok(()) on successful initialization
    pub fn init(&mut self, initial_owner: Address) -> Result<(), Vec<u8>> {
        // Ensure contract is not already initialized (owner not set)
        let current_owner = self.owner.get();
        if current_owner != Address::ZERO {
            return Err(b"ALREADY_INITIALIZED".to_vec());
        }

        // CRITICAL CHECK: Owner cannot be zero address
        if initial_owner == Address::ZERO {
            return Err(b"INVALID_OWNER".to_vec());
        }

        // Set owner
        self.owner.set(initial_owner);

        // Set default fee (0.3% = 30 basis points)
        self.protocol_fee_bps.set(U256::from(DEFAULT_FEE_BPS));

        // Initialize analytics to zero
        self.total_volume_token0.set(U256::ZERO);
        self.total_volume_token1.set(U256::ZERO);

        // Set pause status to false (contract active)
        self.paused.set(false);

        Ok(())
    }

    /// Function to set new protocol fee (owner only)
    ///
    /// Allows owner to change protocol fee within reasonable limits
    ///
    /// # Arguments
    /// * `new_fee_bps` - new fee in basis points (max 1000 = 10%)
    ///
    /// # Returns
    /// Ok(()) on successful fee change
    pub fn set_fee(&mut self, new_fee_bps: u16) -> Result<(), Vec<u8>> {
        // Owner rights check
        let owner = self.owner.get();
        _only_owner(owner)?;

        // Ensure fee does not exceed maximum (10%)
        if new_fee_bps > MAX_FEE_BPS as u16 {
            return Err(b"FEE_TOO_HIGH".to_vec());
        }

        // Optional: fee cannot be set to 0 (protocol must earn revenue)
        // if new_fee_bps == 0 {
        //     return Err(b"FEE_ZERO".to_vec());
        // }

        // Set new fee
        self.protocol_fee_bps.set(U256::from(new_fee_bps));

        Ok(())
    }

    /// Emergency pause function in case of vulnerability (owner only)
    ///
    /// "Panic button" to halt all swaps immediately
    /// Funds appreciate this for demonstrating security focus
    ///
    /// # Returns
    /// Ok(()) on successful pause activation
    pub fn pause(&mut self) -> Result<(), Vec<u8>> {
        // Owner rights check
        let owner = self.owner.get();
        _only_owner(owner)?;

        // Activate pause
        self.paused.set(true);

        Ok(())
    }

    /// Function to unpause contract (owner only)
    ///
    /// Resumes contract operation after issue resolution
    ///
    /// # Returns
    /// Ok(()) on successful unpause
    pub fn unpause(&mut self) -> Result<(), Vec<u8>> {
        // Owner rights check
        let owner = self.owner.get();
        _only_owner(owner)?;

        // Deactivate pause
        self.paused.set(false);

        Ok(())
    }
    /// Public function to create swap commitment
    ///
    /// User submits hash of their secret (amount_in and salt encoded in secret).
    /// Stores hash, current block number, and sets activation status to true.
    ///
    /// # Arguments
    /// * `hash` - commitment hash (bytes32), computed as keccak256(abi.encode(amount_in, salt))
    ///
    /// # Returns
    /// Ok(()) on successful commitment creation
    pub fn commit_swap(&mut self, hash: FixedBytes<32>) -> Result<(), Vec<u8>> {
        // Pause check: commitment creation forbidden when contract is paused
        if self.paused.get() {
            return Err(b"PAUSED".to_vec());
        }

        // Get sender address
        let sender = msg::sender();
        
        // Ensure hash is non-zero
        if hash == FixedBytes::ZERO {
            return Err(b"INVALID_HASH".to_vec());
        }
        
        // Get current block number
        let current_block = U256::from(block::number());
        
        // Store commitment hash
        // Convert FixedBytes<32> to U256 for storage
        let hash_u256 = U256::from_be_bytes::<32>(hash.into());
        self.commitment_hashes.setter(sender).set(hash_u256);
        
        // Store commitment block number
        self.commitment_timestamps.setter(sender).set(current_block);
        
        // Set activation status to true
        self.commitment_activated.setter(sender).set(true);
        
        Ok(())
    }

    /// Public function to reveal commitment and execute swap
    ///
    /// Reveals secret data (amount_in and salt), validates it,
    /// and executes token exchange with MEV protection.
    ///
    /// # Arguments
    /// * `amount_in` - input token amount
    /// * `salt` - random number for brute-force protection
    /// * `min_amount_out` - minimum output tokens (slippage protection)
    ///
    /// # Returns
    /// Ok(()) on successful swap execution
    pub fn reveal_swap(
        &mut self,
        amount_in: U256,
        salt: U256,
        min_amount_out: U256,
    ) -> Result<(), Vec<u8>> {
        // Pause check: swaps forbidden when contract is paused
        if self.paused.get() {
            return Err(b"PAUSED".to_vec());
        }

        // Get sender address
        let sender = msg::sender();
        
        // CRITICAL REENTRANCY PROTECTION: Reset commitment status BEFORE executing swap
        // Prevents commitment reuse in case of reentrancy attack
        let is_activated = self.commitment_activated.setter(sender).get();
        if !is_activated {
            return Err(b"COMMIT_NOT_FOUND".to_vec());
        }
        
        // Get stored commitment hash
        let stored_hash_u256 = self.commitment_hashes.setter(sender).get();
        if stored_hash_u256.is_zero() {
            return Err(b"COMMIT_NOT_FOUND".to_vec());
        }
        
        // Check 2: Verify hash - keccak256(abi.encode(amount_in, salt)) must match
        let computed_hash = _compute_commit_hash(amount_in, salt);
        let computed_hash_u256 = U256::from_be_bytes::<32>(computed_hash.into());
        
        if stored_hash_u256 != computed_hash_u256 {
            return Err(b"INVALID_HASH".to_vec());
        }
        
        // Check 3: Time lock! Reveal only allowed after minimum 5 blocks
        let commit_block = self.commitment_timestamps.setter(sender).get();
        let current_block = U256::from(block::number());
        
        // Ensure enough blocks have passed
        let min_block = commit_block
            .checked_add(U256::from(COMMIT_REVEAL_DELAY))
            .ok_or_else(|| b"BLOCK_OVERFLOW".to_vec())?;
        
        if current_block < min_block {
            return Err(b"TOO_EARLY".to_vec());
        }
        
        // CRITICAL PROTECTION: Reset commitment status immediately after all checks, BEFORE swap execution
        // Prevents commitment reuse even in reentrancy scenario
        self.commitment_activated.setter(sender).set(false);
        self.commitment_hashes.setter(sender).set(U256::ZERO);
        
        // Get current reserves (optimization: read once)
        let reserve0 = self.reserves0.get();
        let reserve1 = self.reserves1.get();
        
        // Get current protocol fee
        let fee_bps = self.protocol_fee_bps.get();
        
        // Swap direction (token0 to token1 or vice versa)
        // For simplicity assume swap from token0 to token1
        // Add direction parameter in full implementation
        
        // Calculate output amount with dynamic fee
        let amount_out = _get_amount_out_with_fee(amount_in, reserve0, reserve1, fee_bps)?;
        
        // Ensure result is not less than min_amount_out (slippage protection)
        if amount_out < min_amount_out {
            return Err(b"INSUFFICIENT_OUTPUT_AMOUNT".to_vec());
        }
        
        // Update reserves after swap
        let new_reserve0 = reserve0
            .checked_add(amount_in)
            .ok_or_else(|| b"RESERVE0_OVERFLOW".to_vec())?;
        
        let new_reserve1 = reserve1
            .checked_sub(amount_out)
            .ok_or_else(|| b"INSUFFICIENT_LIQUIDITY".to_vec())?;
        
        // Check minimum liquidity
        let min_liquidity = self.min_liquidity.get();
        if new_reserve0 < min_liquidity || new_reserve1 < min_liquidity {
            return Err(b"INSUFFICIENT_LIQUIDITY".to_vec());
        }
        
        // Update reserves in storage
        self.reserves0.set(new_reserve0);
        self.reserves1.set(new_reserve1);
        
        // Update trading volume analytics (for reporting and fund compliance)
        let current_volume0 = self.total_volume_token0.get();
        let current_volume1 = self.total_volume_token1.get();
        
        let new_volume0 = current_volume0
            .checked_add(amount_in)
            .ok_or_else(|| b"VOLUME_OVERFLOW".to_vec())?;
        
        let new_volume1 = current_volume1
            .checked_add(amount_out)
            .ok_or_else(|| b"VOLUME_OVERFLOW".to_vec())?;
        
        self.total_volume_token0.set(new_volume0);
        self.total_volume_token1.set(new_volume1);
        
        // Commitment status already reset above for reentrancy protection
        // Guarantees commitment cannot be used twice
        
        Ok(())
    }

    /// Public function to add liquidity to the pool
    ///
    /// Updates token reserves in storage and validates input.
    /// Enforces minimum liquidity to prevent pool draining.
    ///
    /// # Arguments
    /// * `amount0` - token0 amount to add
    /// * `amount1` - token1 amount to add
    ///
    /// # Returns
    /// Ok(()) on successful liquidity addition
    pub fn add_liquidity(&mut self, amount0: U256, amount1: U256) -> Result<(), Vec<u8>> {
        // Ensure amounts are greater than zero
        if amount0.is_zero() {
            return Err(b"AMOUNT0_ZERO".to_vec());
        }
        if amount1.is_zero() {
            return Err(b"AMOUNT1_ZERO".to_vec());
        }

        // Get current reserves
        let reserve0 = self.reserves0.get();
        let reserve1 = self.reserves1.get();
        let min_liquidity = self.min_liquidity.get();

        // Calculate new reserves
        let new_reserve0 = reserve0
            .checked_add(amount0)
            .ok_or_else(|| b"RESERVE0_OVERFLOW".to_vec())?;
        
        let new_reserve1 = reserve1
            .checked_add(amount1)
            .ok_or_else(|| b"RESERVE1_OVERFLOW".to_vec())?;

        // Calculate total liquidity after addition
        let total_liquidity = new_reserve0
            .checked_add(new_reserve1)
            .ok_or_else(|| b"LIQUIDITY_OVERFLOW".to_vec())?;

        // Minimum liquidity check
        // On first addition, set minimum liquidity
        if min_liquidity.is_zero() {
            let min_liq = U256::from(MINIMUM_LIQUIDITY);
            self.min_liquidity.set(min_liq);
            
            // Ensure we have sufficient liquidity after setting minimum
            if total_liquidity < min_liq {
                return Err(b"INSUFFICIENT_LIQUIDITY".to_vec());
            }
        } else {
            // Ensure sufficient funds after liquidity addition
            // (new reserves must exceed minimum liquidity)
            if new_reserve0 < min_liquidity || new_reserve1 < min_liquidity {
                return Err(b"INSUFFICIENT_LIQUIDITY".to_vec());
            }
        }

        // Update reserves in storage
        self.reserves0.set(new_reserve0);
        self.reserves1.set(new_reserve1);

        Ok(())
    }
}


/// Entry point for the Oak Protocol contract
/// 
/// This is the main entry point that will be called by the Stylus runtime.
/// All contract interactions will go through this entry point.
#[entrypoint]
pub fn main(_input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    // TODO: Implement contract logic
    Ok(vec![])
}
