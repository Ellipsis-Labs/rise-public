//! Borrowed views for Phoenix permission accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::common::{
    PhoenixAccountDecodeError, borrow_prefix, read_prefix, require_len, verify_discriminant,
};
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const PERMISSION_ACCOUNT: &str = "PermissionAccount";
const PERMISSION_ACCOUNT_LEN: usize = core::mem::size_of::<PermissionAccount>();

const_assert_eq!(core::mem::size_of::<PermissionAccount>(), 168);

/// Permission bit used by delegated trader onboarding flows.
pub const TRADER_ONBOARDING_PERMISSION: u64 = 1 << 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PermissionAccount {
    discriminant: u64,
    authority: [u8; 32],
    user: [u8; 32],
    bump: u8,
    _padding1: [u8; 7],
    permission: u64,
    expires_at_timestamp: i64,
    num_signer_actions_remaining: i64,
    _padding2: [u8; 64],
}

impl PermissionAccount {
    /// Decode a permission account from an arbitrary byte slice.
    ///
    /// This copies the fixed-layout account with an unaligned read, so it is
    /// appropriate for off-chain callers that do not control account-data
    /// alignment. Use [`Self::try_from_buffer`] or
    /// [`Self::try_borrow_from_account_bytes`] in programs when the account
    /// data is naturally aligned and a borrowed zero-copy view is preferred.
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(PERMISSION_ACCOUNT, data, PERMISSION_ACCOUNT_LEN)?;
        verify_discriminant(
            PERMISSION_ACCOUNT,
            data,
            PhoenixAccount::PermissionAccount.discriminant(),
        )?;
        read_prefix(PERMISSION_ACCOUNT, data)
    }

    /// Try to borrow a zero-copy permission account view from a raw account
    /// buffer.
    ///
    /// This validates the fixed account length, POD alignment, and permission
    /// account discriminant before returning a borrowed account reference. It
    /// never panics on short or misaligned input.
    pub fn try_from_buffer(data: &[u8]) -> Result<&Self, PhoenixAccountDecodeError> {
        let account = borrow_prefix::<Self>(PERMISSION_ACCOUNT, data)?;
        if account.discriminant.to_le_bytes() != PhoenixAccount::PermissionAccount.discriminant() {
            let mut actual = [0u8; 8];
            actual.copy_from_slice(&data[..8]);
            return Err(PhoenixAccountDecodeError::InvalidDiscriminant {
                account: PERMISSION_ACCOUNT,
                expected: PhoenixAccount::PermissionAccount.discriminant(),
                actual,
            });
        }
        Ok(account)
    }

    /// Alias for [`Self::try_from_buffer`] that matches the naming used by the
    /// other Rise borrowed account views.
    pub fn try_borrow_from_account_bytes(data: &[u8]) -> Result<&Self, PhoenixAccountDecodeError> {
        Self::try_from_buffer(data)
    }

    #[inline(always)]
    pub const fn discriminant(&self) -> u64 {
        self.discriminant
    }

    #[inline(always)]
    pub const fn authority(&self) -> [u8; 32] {
        self.authority
    }

    #[inline(always)]
    pub const fn user(&self) -> [u8; 32] {
        self.user
    }

    #[inline(always)]
    pub const fn bump(&self) -> u8 {
        self.bump
    }

    #[inline(always)]
    pub const fn permission_bits(&self) -> u64 {
        self.permission
    }

    #[inline(always)]
    pub const fn expires_at_timestamp(&self) -> i64 {
        self.expires_at_timestamp
    }

    #[inline(always)]
    pub const fn num_signer_actions_remaining(&self) -> i64 {
        self.num_signer_actions_remaining
    }

    #[inline(always)]
    pub const fn has_permission(&self, permission: u64) -> bool {
        self.permission & permission == permission
    }

    #[inline(always)]
    pub const fn has_trader_onboarding_permission(&self) -> bool {
        self.has_permission(TRADER_ONBOARDING_PERMISSION)
    }

    #[inline(always)]
    pub const fn has_remaining_uses(&self) -> bool {
        self.num_signer_actions_remaining != 0
    }

    #[inline(always)]
    pub const fn is_expired_at(&self, unix_timestamp: i64) -> bool {
        self.expires_at_timestamp != 0 && unix_timestamp > self.expires_at_timestamp
    }

    #[inline(always)]
    pub fn is_set_for(&self, authority: &[u8; 32], user: &[u8; 32], permission: u64) -> bool {
        self.authority() == *authority && self.user() == *user && self.has_permission(permission)
    }
}

/// Compatibility wrapper over a borrowed Phoenix permission account.
///
/// New program integrations should prefer
/// [`PermissionAccount::try_from_buffer`], which returns
/// `&PermissionAccount` directly and matches the zero-copy account pattern used
/// in `phoenix-eternal-types`.
#[derive(Clone, Copy, Debug)]
pub struct PermissionAccountRef<'a>(&'a PermissionAccount);

impl<'a> PermissionAccountRef<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        Ok(Self(PermissionAccount::try_from_buffer(data)?))
    }

    #[inline(always)]
    pub const fn as_account(&self) -> &'a PermissionAccount {
        self.0
    }

    #[inline(always)]
    pub const fn authority(&self) -> [u8; 32] {
        self.0.authority()
    }

    #[inline(always)]
    pub const fn user(&self) -> [u8; 32] {
        self.0.user()
    }

    #[inline(always)]
    pub const fn bump(&self) -> u8 {
        self.0.bump()
    }

    #[inline(always)]
    pub const fn permission_bits(&self) -> u64 {
        self.0.permission_bits()
    }

    #[inline(always)]
    pub const fn expires_at_timestamp(&self) -> i64 {
        self.0.expires_at_timestamp()
    }

    #[inline(always)]
    pub const fn num_signer_actions_remaining(&self) -> i64 {
        self.0.num_signer_actions_remaining()
    }

    #[inline(always)]
    pub const fn has_permission(&self, permission: u64) -> bool {
        self.0.has_permission(permission)
    }

    #[inline(always)]
    pub const fn has_trader_onboarding_permission(&self) -> bool {
        self.0.has_trader_onboarding_permission()
    }

    #[inline(always)]
    pub const fn has_remaining_uses(&self) -> bool {
        self.0.has_remaining_uses()
    }

    #[inline(always)]
    pub const fn is_expired_at(&self, unix_timestamp: i64) -> bool {
        self.0.is_expired_at(unix_timestamp)
    }

    #[inline(always)]
    pub fn is_set_for(&self, authority: &[u8; 32], user: &[u8; 32], permission: u64) -> bool {
        self.0.is_set_for(authority, user, permission)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PermissionAccount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PermissionAccount", 8)?;
        state.serialize_field("authority", &pubkey_string(&self.authority()))?;
        state.serialize_field("user", &pubkey_string(&self.user()))?;
        state.serialize_field("bump", &self.bump())?;
        state.serialize_field("permission_bits", &self.permission_bits())?;
        state.serialize_field("expires_at_timestamp", &self.expires_at_timestamp())?;
        state.serialize_field(
            "num_signer_actions_remaining",
            &self.num_signer_actions_remaining(),
        )?;
        state.serialize_field(
            "has_trader_onboarding_permission",
            &self.has_trader_onboarding_permission(),
        )?;
        state.serialize_field("has_remaining_uses", &self.has_remaining_uses())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PermissionAccountRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhoenixAccount;

    fn aligned_permission_data() -> [u64; PERMISSION_ACCOUNT_LEN / 8] {
        let authority = [7; 32];
        let user = [9; 32];
        let mut words = [0u64; PERMISSION_ACCOUNT_LEN / 8];
        let data = bytemuck::cast_slice_mut(&mut words);
        data[..8].copy_from_slice(&PhoenixAccount::PermissionAccount.discriminant());
        data[8..40].copy_from_slice(&authority);
        data[40..72].copy_from_slice(&user);
        data[72] = 254;
        data[80..88].copy_from_slice(&TRADER_ONBOARDING_PERMISSION.to_le_bytes());
        data[88..96].copy_from_slice(&123_i64.to_le_bytes());
        data[96..104].copy_from_slice(&7_i64.to_le_bytes());
        words
    }

    #[test]
    fn permission_account_view_reads_core_fields() {
        let authority = [7; 32];
        let user = [9; 32];
        let mut data = vec![0u8; PERMISSION_ACCOUNT_LEN];
        data[..8].copy_from_slice(&PhoenixAccount::PermissionAccount.discriminant());
        data[8..40].copy_from_slice(&authority);
        data[40..72].copy_from_slice(&user);
        data[72] = 254;
        data[80..88].copy_from_slice(&TRADER_ONBOARDING_PERMISSION.to_le_bytes());
        data[88..96].copy_from_slice(&123_i64.to_le_bytes());
        data[96..104].copy_from_slice(&7_i64.to_le_bytes());

        let permission = PermissionAccount::try_from_account_bytes(&data).unwrap();
        assert_eq!(permission.authority(), authority);
        assert_eq!(permission.user(), user);
        assert_eq!(permission.bump(), 254);
        assert!(permission.has_trader_onboarding_permission());
        assert!(permission.has_remaining_uses());
        assert_eq!(permission.num_signer_actions_remaining(), 7);
        assert!(permission.is_set_for(&authority, &user, TRADER_ONBOARDING_PERMISSION));
    }

    #[test]
    fn permission_account_decodes_from_unaligned_bytes() {
        let authority = [7; 32];
        let user = [9; 32];
        let mut aligned_data = vec![0u8; PERMISSION_ACCOUNT_LEN];
        aligned_data[..8].copy_from_slice(&PhoenixAccount::PermissionAccount.discriminant());
        aligned_data[8..40].copy_from_slice(&authority);
        aligned_data[40..72].copy_from_slice(&user);
        aligned_data[80..88].copy_from_slice(&TRADER_ONBOARDING_PERMISSION.to_le_bytes());

        let mut unaligned_data = vec![255u8];
        unaligned_data.extend_from_slice(&aligned_data);
        let permission = PermissionAccount::try_from_account_bytes(&unaligned_data[1..]).unwrap();

        assert_eq!(permission.authority(), authority);
        assert_eq!(permission.user(), user);
        assert!(permission.has_trader_onboarding_permission());
    }

    #[test]
    fn permission_account_borrows_aligned_account_bytes() {
        let words = aligned_permission_data();
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let permission = PermissionAccount::try_from_buffer(data).unwrap();

        assert_eq!(
            permission as *const PermissionAccount as *const u8,
            data.as_ptr()
        );
        assert_eq!(
            PermissionAccount::try_borrow_from_account_bytes(data).unwrap()
                as *const PermissionAccount,
            permission as *const PermissionAccount
        );
        assert_eq!(permission.authority(), [7; 32]);
        assert_eq!(permission.user(), [9; 32]);
        assert_eq!(permission.bump(), 254);
        assert!(permission.has_trader_onboarding_permission());
        assert_eq!(permission.num_signer_actions_remaining(), 7);
    }

    #[test]
    fn permission_account_ref_rejects_short_input() {
        let words = aligned_permission_data();
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let err =
            PermissionAccount::try_from_buffer(&data[..PERMISSION_ACCOUNT_LEN - 1]).unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::AccountTooSmall {
                account: PERMISSION_ACCOUNT,
                expected: PERMISSION_ACCOUNT_LEN,
                actual: PERMISSION_ACCOUNT_LEN - 1,
            }
        );
    }

    #[test]
    fn permission_account_ref_rejects_misaligned_input_without_panicking() {
        let mut words = [0u64; (PERMISSION_ACCOUNT_LEN / 8) + 1];
        let buffer = bytemuck::cast_slice_mut::<u64, u8>(&mut words);
        let data = &mut buffer[1..1 + PERMISSION_ACCOUNT_LEN];
        data[..8].copy_from_slice(&PhoenixAccount::PermissionAccount.discriminant());

        let err = PermissionAccount::try_from_buffer(data).unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::InvalidLayout {
                account: PERMISSION_ACCOUNT,
            }
        );
    }

    #[test]
    fn permission_account_ref_compatibility_wrapper_uses_borrowed_account() {
        let words = aligned_permission_data();
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let permission = PermissionAccountRef::try_from_account_bytes(data).unwrap();

        assert_eq!(
            permission.as_account() as *const PermissionAccount as *const u8,
            data.as_ptr()
        );
        assert_eq!(permission.authority(), [7; 32]);
        assert!(permission.has_trader_onboarding_permission());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn permission_account_json_encodes_pubkeys_as_base58_strings() {
        let authority = [7; 32];
        let user = [9; 32];
        let mut data = vec![0u8; PERMISSION_ACCOUNT_LEN];
        data[..8].copy_from_slice(&PhoenixAccount::PermissionAccount.discriminant());
        data[8..40].copy_from_slice(&authority);
        data[40..72].copy_from_slice(&user);

        let permission = PermissionAccount::try_from_account_bytes(&data).unwrap();
        let value = serde_json::to_value(permission).unwrap();

        assert_eq!(
            value["authority"],
            serde_json::Value::String(bs58::encode(authority).into_string())
        );
        assert_eq!(
            value["user"],
            serde_json::Value::String(bs58::encode(user).into_string())
        );
    }
}
