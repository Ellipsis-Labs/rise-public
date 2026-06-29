use phoenix_rise::accounts::trader::capabilities::TRADER_CAPABILITY_HOT;
use phoenix_rise::ix::constants::compute_discriminant;

/// Permission bit used by delegated trader onboarding flows.
pub const TRADER_ONBOARDING_PERMISSION: u64 = 1 << 4;

#[derive(Debug, Clone, Copy)]
pub struct GlobalConfigView {
    account_key: [u8; 32],
    risk_authority: [u8; 32],
    global_trader_index_header_key: [u8; 32],
    active_trader_buffer_header_key: [u8; 32],
}

impl GlobalConfigView {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, &'static str> {
        verify_discriminant(data, "account:global_configuration", 776)?;
        Ok(Self {
            account_key: read_pubkey(data, 8)?,
            risk_authority: read_pubkey(data, 72)?,
            global_trader_index_header_key: read_pubkey(data, 392)?,
            active_trader_buffer_header_key: read_pubkey(data, 424)?,
        })
    }

    pub const fn account_key(&self) -> [u8; 32] {
        self.account_key
    }

    pub const fn risk_authority(&self) -> [u8; 32] {
        self.risk_authority
    }

    pub const fn global_trader_index_header_key(&self) -> [u8; 32] {
        self.global_trader_index_header_key
    }

    pub const fn active_trader_buffer_header_key(&self) -> [u8; 32] {
        self.active_trader_buffer_header_key
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PermissionAccountView {
    authority: [u8; 32],
    user: [u8; 32],
    permission: u64,
    num_signer_actions_remaining: i64,
}

impl PermissionAccountView {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, &'static str> {
        verify_discriminant(data, "account:permission_account", 168)?;
        Ok(Self {
            authority: read_pubkey(data, 8)?,
            user: read_pubkey(data, 40)?,
            permission: read_u64(data, 80)?,
            num_signer_actions_remaining: read_i64(data, 96)?,
        })
    }

    pub const fn num_signer_actions_remaining(&self) -> i64 {
        self.num_signer_actions_remaining
    }

    pub const fn has_remaining_uses(&self) -> bool {
        self.num_signer_actions_remaining != 0
    }

    pub const fn has_permission(&self, permission: u64) -> bool {
        self.permission & permission == permission
    }

    pub fn is_set_for(&self, authority: &[u8; 32], user: &[u8; 32], permission: u64) -> bool {
        self.authority == *authority && self.user == *user && self.has_permission(permission)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TraderHeaderView {
    key: [u8; 32],
    authority: [u8; 32],
    flags: u32,
    max_positions: u32,
    position_authority: [u8; 32],
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    funding_key: [u8; 32],
    position_count: u64,
    position_capacity: u64,
}

impl TraderHeaderView {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, &'static str> {
        verify_discriminant(data, "account:trader", 240)?;
        let position_count = read_u64(data, 224)?;
        let position_capacity = read_u64(data, 232)?;
        if position_count > position_capacity {
            return Err("position count exceeds capacity");
        }
        Ok(Self {
            key: read_pubkey(data, 24)?,
            authority: read_pubkey(data, 56)?,
            flags: read_u32(data, 96)?,
            max_positions: read_u32(data, 112)?,
            position_authority: read_pubkey(data, 120)?,
            trader_pda_index: read_u8(data, 154)?,
            trader_subaccount_index: read_u8(data, 155)?,
            funding_key: read_pubkey(data, 156)?,
            position_count,
            position_capacity,
        })
    }

    pub const fn key(&self) -> [u8; 32] {
        self.key
    }

    pub const fn authority(&self) -> [u8; 32] {
        self.authority
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn max_positions(&self) -> u32 {
        self.max_positions
    }

    pub const fn position_authority(&self) -> [u8; 32] {
        self.position_authority
    }

    pub const fn trader_pda_index(&self) -> u8 {
        self.trader_pda_index
    }

    pub const fn trader_subaccount_index(&self) -> u8 {
        self.trader_subaccount_index
    }

    pub const fn funding_key(&self) -> [u8; 32] {
        self.funding_key
    }

    pub const fn position_count(&self) -> u64 {
        self.position_count
    }

    pub const fn position_capacity(&self) -> u64 {
        self.position_capacity
    }

    pub const fn is_hot(&self) -> bool {
        self.flags & TRADER_CAPABILITY_HOT != 0
    }

    pub const fn is_cold(&self) -> bool {
        self.flags != 0 && !self.is_hot()
    }
}

pub fn global_trader_index_account_count(data: &[u8]) -> Result<usize, &'static str> {
    multi_arena_account_count(data, "account:global_trader_index")
}

pub fn active_trader_buffer_account_count(data: &[u8]) -> Result<usize, &'static str> {
    multi_arena_account_count(data, "account:active_trader_buffer")
}

fn multi_arena_account_count(
    data: &[u8],
    discriminant_seed: &'static str,
) -> Result<usize, &'static str> {
    verify_discriminant(data, discriminant_seed, 80)?;
    let num_arenas = read_u16(data, 52)?;
    if num_arenas == 0 {
        return Err("multi-arena account has no arenas");
    }
    Ok(num_arenas as usize)
}

fn verify_discriminant(
    data: &[u8],
    discriminant_seed: &'static str,
    min_len: usize,
) -> Result<(), &'static str> {
    if data.len() < min_len {
        return Err("account data too short");
    }
    let discriminant = data.get(..8).ok_or("account data too short")?;
    let expected = compute_discriminant(discriminant_seed);
    if discriminant != expected.as_ref() {
        return Err("account discriminant mismatch");
    }
    Ok(())
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8, &'static str> {
    data.get(offset).copied().ok_or("account data too short")
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, &'static str> {
    let mut bytes = [0u8; 2];
    bytes.copy_from_slice(
        data.get(offset..offset + 2)
            .ok_or("account data too short")?,
    );
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, &'static str> {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(
        data.get(offset..offset + 4)
            .ok_or("account data too short")?,
    );
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, &'static str> {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(
        data.get(offset..offset + 8)
            .ok_or("account data too short")?,
    );
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64(data: &[u8], offset: usize) -> Result<i64, &'static str> {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(
        data.get(offset..offset + 8)
            .ok_or("account data too short")?,
    );
    Ok(i64::from_le_bytes(bytes))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<[u8; 32], &'static str> {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(
        data.get(offset..offset + 32)
            .ok_or("account data too short")?,
    );
    Ok(bytes)
}
