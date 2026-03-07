//! Signal Marketplace: EIP-712 signed listings; payment in protocol tokens.
//! Encrypted content key is delivered off-chain after payment (alerts.oak.trade).

use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    block, contract,
};

use crate::errors::{err, OakResult, ERR_SIGNAL_ALREADY_PURCHASED, ERR_SIGNAL_INVALID_SIGNATURE, ERR_SIGNAL_NOT_LISTED};
use crate::events::emit_signal_purchased;
use crate::logic::{compute_domain_separator, compute_signal_listing_digest, compute_signal_listing_struct_hash, ecrecover_recover};
use crate::state::OakDEX;
use crate::token::safe_transfer_from;

const CHAIN_ID_ARBITRUM_ONE: u64 = 42161;

/// Signal Marketplace: list, delist, purchase with EIP-712.
pub struct SignalMarketplace;

impl SignalMarketplace {
    /// List a signal (seller = msg.sender). Price in protocol token wei.
    pub fn list_signal(dex: &mut OakDEX, signal_id_hash: FixedBytes<32>, price: U256) -> OakResult<()> {
        let seller = stylus_sdk::msg::sender();
        if seller == Address::ZERO {
            return Err(err(crate::errors::ERR_INVALID_ADDRESS));
        }
        dex.signal_price.setter(seller).setter(signal_id_hash).set(price);
        Ok(())
    }

    /// Delist: set price to 0 (seller = msg.sender).
    pub fn delist_signal(dex: &mut OakDEX, signal_id_hash: FixedBytes<32>) -> OakResult<()> {
        let seller = stylus_sdk::msg::sender();
        dex.signal_price.setter(seller).setter(signal_id_hash).set(U256::ZERO);
        Ok(())
    }

    /// Purchase signal: verify seller's EIP-712 signature, transfer protocol tokens to seller, mark purchased.
    /// Callable by anyone; buyer pays. Backend uses event to deliver encrypted content key.
    pub fn purchase_signal(
        dex: &mut OakDEX,
        buyer: Address,
        seller: Address,
        signal_id_hash: FixedBytes<32>,
        price: U256,
        nonce: U256,
        deadline: U256,
        protocol_token: Address,
        v: u8,
        r: [u8; 32],
        s: [u8; 32],
    ) -> OakResult<()> {
        if U256::from(block::number()) > deadline {
            return Err(err(crate::errors::ERR_EXPIRED));
        }
        let listed_price = dex.signal_price.getter(seller).getter(signal_id_hash).get();
        if listed_price != price || listed_price.is_zero() {
            return Err(err(ERR_SIGNAL_NOT_LISTED));
        }
        let current_nonce = dex.signal_nonce.getter(seller).get();
        if nonce != current_nonce {
            return Err(err(crate::errors::ERR_PERMIT_NONCE));
        }
        let listing_hash = compute_signal_listing_struct_hash(seller, signal_id_hash, price, nonce, deadline);
        if dex.signal_purchased.getter(buyer).getter(listing_hash).get() {
            return Err(err(ERR_SIGNAL_ALREADY_PURCHASED));
        }
        let contract_addr = contract::address();
        let domain_separator = compute_domain_separator(contract_addr, CHAIN_ID_ARBITRUM_ONE);
        let digest = compute_signal_listing_digest(
            seller,
            signal_id_hash,
            price,
            nonce,
            deadline,
            &domain_separator,
        );
        let recovered = ecrecover_recover(digest, v, r, s);
        if recovered != seller {
            return Err(err(ERR_SIGNAL_INVALID_SIGNATURE));
        }
        crate::logic::lock_reentrancy_guard(dex)?;
        // CEI: update state before external call (transfer); rollback on transfer failure.
        let new_nonce = current_nonce.checked_add(U256::from(1u64)).ok_or_else(|| err(crate::errors::ERR_OVERFLOW))?;
        dex.signal_purchased.setter(buyer).setter(listing_hash).set(true);
        dex.signal_nonce.setter(seller).set(new_nonce);
        match safe_transfer_from(protocol_token, buyer, seller, price) {
            Ok(()) => {
                emit_signal_purchased(buyer, seller, U256::from_be_bytes::<32>(listing_hash.into()), price);
                crate::logic::unlock_reentrancy_guard(dex);
                Ok(())
            }
            Err(e) => {
                dex.signal_purchased.setter(buyer).setter(listing_hash).set(false);
                dex.signal_nonce.setter(seller).set(current_nonce);
                crate::logic::unlock_reentrancy_guard(dex);
                Err(e)
            }
        }
    }

    /// View: listed price for (seller, signal_id_hash). 0 = not listed.
    pub fn get_signal_price(dex: &OakDEX, seller: Address, signal_id_hash: FixedBytes<32>) -> U256 {
        dex.signal_price.getter(seller).getter(signal_id_hash).get()
    }

    /// View: whether buyer has purchased the listing (by listing_hash).
    pub fn has_purchased(dex: &OakDEX, buyer: Address, listing_hash: FixedBytes<32>) -> bool {
        dex.signal_purchased.getter(buyer).getter(listing_hash).get()
    }

    /// View: current EIP-712 nonce for seller (replay protection).
    pub fn get_signal_nonce(dex: &OakDEX, seller: Address) -> U256 {
        dex.signal_nonce.getter(seller).get()
    }
}
