//! Oak Bet RNG module (Provably Fair).
//!
//! This module provides a lightweight, gas‑optimized random generator
//! based on keccak256. It is designed for the Oak Bet casino module:
//! users supply a client seed, while the contract contributes block
//! metadata as server entropy. The resulting hash is fully reproducible
//! off‑chain, enabling Provably Fair verification.

use stylus_sdk::{
    alloy_primitives::{FixedBytes, U256},
    block,
    crypto,
};

/// Simple RNG facade for Oak Bet.
pub struct OakRng;

impl OakRng {
    /// Generate a pseudo‑random `U256` value from block data and a client seed.
    ///
    /// Entropy sources:
    /// - `block::number()`      – public L2 block height
    /// - `block::timestamp()`   – public L2 block timestamp
    /// - `seed` (client seed)   – 32‑байтовый хэш, выбранный пользователем
    ///
    /// Provably Fair:
    /// - На входе известны три значения: `(block_number, block_timestamp, seed)`.
    /// - Контракт считает `h = keccak256(bn || ts || seed)`.
    /// - Пользователь может самостоятельно пересчитать `h` off‑chain и убедиться,
    ///   что результат совпадает с тем, что использовал контракт.
    ///
    /// Возвращаемое значение:
    /// - Число `U256`, полученное интерпретацией `h` как big‑endian целого.
    #[inline]
    pub fn generate_random_u256(seed: FixedBytes<32>) -> U256 {
        // Encode block number and timestamp as big‑endian U256 bytes.
        let bn = U256::from(block::number());
        let ts = U256::from(block::timestamp());

        // 32 bytes (bn) || 32 bytes (ts) || 32 bytes (user seed) = 96 bytes total.
        // Используем статический массив вместо Vec для минимизации аллокаций.
        let mut input = [0u8; 96];

        input[0..32].copy_from_slice(&bn.to_be_bytes::<32>());
        input[32..64].copy_from_slice(&ts.to_be_bytes::<32>());
        input[64..96].copy_from_slice(&<FixedBytes<32> as Into<[u8; 32]>>::into(seed));

        // keccak256(input)
        let hash = crypto::keccak(input);

        // Интерпретируем hash как U256 (big‑endian).
        U256::from_be_bytes::<32>(hash.into())
    }
}

