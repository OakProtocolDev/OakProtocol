//! Pausable trait: global `paused` flag and checks for critical operations.
//!
//! No_std compatible. Use `require_not_paused` at the start of critical paths
//! (swaps, close position, etc.). Only accounts with PAUSER_ROLE can pause/unpause.

use alloc::vec::Vec;

use crate::{
    access::{pauser_role, require_role},
    errors::*,
    events::emit_pause_changed,
    state::OakDEX,
};

/// Contract that can be paused. Critical operations must call `require_not_paused` first.
pub trait Pausable {
    /// Returns true if the contract is paused.
    fn is_paused(&self) -> bool;

    /// Reverts with `ERR_PAUSED` if the contract is paused (use at entry of critical functions).
    fn require_not_paused(&self) -> Result<(), Vec<u8>>;

    /// Pauses the contract. Caller must have PAUSER_ROLE. CEI: state update before any external.
    fn pause(&mut self) -> Result<(), Vec<u8>>;

    /// Unpauses the contract. Caller must have PAUSER_ROLE.
    fn unpause(&mut self) -> Result<(), Vec<u8>>;
}

impl Pausable for OakDEX {
    fn is_paused(&self) -> bool {
        self.paused.get()
    }

    fn require_not_paused(&self) -> Result<(), Vec<u8>> {
        if self.paused.get() {
            Err(err(ERR_PAUSED))
        } else {
            Ok(())
        }
    }

    fn pause(&mut self) -> Result<(), Vec<u8>> {
        require_role(self, pauser_role())?;
        self.paused.set(true);
        emit_pause_changed(true);
        Ok(())
    }

    fn unpause(&mut self) -> Result<(), Vec<u8>> {
        require_role(self, pauser_role())?;
        self.paused.set(false);
        emit_pause_changed(false);
        Ok(())
    }
}
