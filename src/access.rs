//! Access Control by roles (DEFAULT_ADMIN_ROLE, PAUSER_ROLE, UPGRADER_ROLE).
//!
//! No_std compatible. Caller is identified via `msg::sender()` (EVM predecessor).
//! Roles stored in `sol_storage!` as role_hash -> account -> bool.

use alloc::vec::Vec;
use stylus_sdk::{alloy_primitives::{Address, FixedBytes}, crypto, msg};

use crate::{errors::*, state::OakDEX};

/// Role identifiers (keccak256 of role name), as bytes32.
pub fn default_admin_role() -> FixedBytes<32> {
    crypto::keccak(b"DEFAULT_ADMIN_ROLE")
}
pub fn pauser_role() -> FixedBytes<32> {
    crypto::keccak(b"PAUSER_ROLE")
}
pub fn upgrader_role() -> FixedBytes<32> {
    crypto::keccak(b"UPGRADER_ROLE")
}

/// Returns true if `account` has `role`. Uses getter for read-only access.
#[inline]
pub fn has_role(dex: &OakDEX, role: FixedBytes<32>, account: Address) -> bool {
    dex.roles.getter(role).getter(account).get()
}

/// Requires that `msg::sender()` has `role`; otherwise returns `ERR_MISSING_ROLE`.
pub fn require_role(dex: &OakDEX, role: FixedBytes<32>) -> Result<(), Vec<u8>> {
    let caller = msg::sender();
    if has_role(dex, role, caller) {
        Ok(())
    } else {
        Err(err(ERR_MISSING_ROLE))
    }
}

/// Grants `role` to `account`. Caller must have DEFAULT_ADMIN_ROLE (or same role for renounce).
/// CEI: effects (storage) before no external calls.
pub fn grant_role(dex: &mut OakDEX, role: FixedBytes<32>, account: Address) -> Result<(), Vec<u8>> {
    if account == Address::ZERO {
        return Err(err(ERR_GRANT_ZERO));
    }
    require_role(dex, default_admin_role())?;
    dex.roles.setter(role).setter(account).set(true);
    Ok(())
}

/// Revokes `role` from `account`. Caller must have DEFAULT_ADMIN_ROLE.
pub fn revoke_role(dex: &mut OakDEX, role: FixedBytes<32>, account: Address) -> Result<(), Vec<u8>> {
    require_role(dex, default_admin_role())?;
    dex.roles.setter(role).setter(account).set(false);
    Ok(())
}
