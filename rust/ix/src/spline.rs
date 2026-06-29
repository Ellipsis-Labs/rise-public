//! Spline instruction construction.

use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
};
use crate::discriminants::PhoenixInstruction;
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction};

pub const MAX_SPLINE_REGIONS: usize = 10;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RegisterSplineParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    payer: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_option"))]
    permission_account: Option<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    market_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    spline_collection: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
}

impl RegisterSplineParams {
    pub fn builder() -> RegisterSplineParamsBuilder {
        RegisterSplineParamsBuilder::new()
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn permission_account(&self) -> Option<Pubkey> {
        self.permission_account
    }

    pub fn market_account(&self) -> Pubkey {
        self.market_account
    }

    pub fn spline_collection(&self) -> Pubkey {
        self.spline_collection
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }
}

#[derive(Default)]
pub struct RegisterSplineParamsBuilder {
    payer: Option<Pubkey>,
    authority: Option<Pubkey>,
    permission_account: Option<Pubkey>,
    market_account: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl RegisterSplineParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn market_account(mut self, market_account: Pubkey) -> Self {
        self.market_account = Some(market_account);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn build(self) -> Result<RegisterSplineParams, PhoenixIxError> {
        let global_trader_index = self
            .global_trader_index
            .ok_or(PhoenixIxError::MissingField("global_trader_index"))?;
        if global_trader_index.is_empty() {
            return Err(PhoenixIxError::EmptyGlobalTraderIndex);
        }
        let active_trader_buffer = self
            .active_trader_buffer
            .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?;
        if active_trader_buffer.is_empty() {
            return Err(PhoenixIxError::EmptyActiveTraderBuffer);
        }

        Ok(RegisterSplineParams {
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            authority: self
                .authority
                .ok_or(PhoenixIxError::MissingField("authority"))?,
            permission_account: self.permission_account,
            market_account: self
                .market_account
                .ok_or(PhoenixIxError::MissingField("market_account"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeactivateSplineParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_option"))]
    permission_account: Option<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    market_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    spline_collection: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
}

impl DeactivateSplineParams {
    pub fn builder() -> DeactivateSplineParamsBuilder {
        DeactivateSplineParamsBuilder::new()
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn permission_account(&self) -> Option<Pubkey> {
        self.permission_account
    }

    pub fn market_account(&self) -> Pubkey {
        self.market_account
    }

    pub fn spline_collection(&self) -> Pubkey {
        self.spline_collection
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }
}

#[derive(Default)]
pub struct DeactivateSplineParamsBuilder {
    authority: Option<Pubkey>,
    permission_account: Option<Pubkey>,
    market_account: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl DeactivateSplineParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn market_account(mut self, market_account: Pubkey) -> Self {
        self.market_account = Some(market_account);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn build(self) -> Result<DeactivateSplineParams, PhoenixIxError> {
        let global_trader_index = self
            .global_trader_index
            .ok_or(PhoenixIxError::MissingField("global_trader_index"))?;
        if global_trader_index.is_empty() {
            return Err(PhoenixIxError::EmptyGlobalTraderIndex);
        }
        let active_trader_buffer = self
            .active_trader_buffer
            .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?;
        if active_trader_buffer.is_empty() {
            return Err(PhoenixIxError::EmptyActiveTraderBuffer);
        }

        Ok(DeactivateSplineParams {
            authority: self
                .authority
                .ok_or(PhoenixIxError::MissingField("authority"))?,
            permission_account: self.permission_account,
            market_account: self
                .market_account
                .ok_or(PhoenixIxError::MissingField("market_account"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TickRegionParams {
    pub start_offset: u64,
    pub end_offset: u64,
    pub density: u32,
    pub top_level_hidden_take_size: u32,
    pub lifespan: u64,
}

impl TickRegionParams {
    pub const fn new(start_offset: u64, end_offset: u64, density: u32, lifespan: u64) -> Self {
        Self {
            start_offset,
            end_offset,
            density,
            top_level_hidden_take_size: 0,
            lifespan,
        }
    }

    pub const fn new_with_hidden_take_size(
        start_offset: u64,
        end_offset: u64,
        density: u32,
        top_level_hidden_take_size: u32,
        lifespan: u64,
    ) -> Self {
        Self {
            start_offset,
            end_offset,
            density,
            top_level_hidden_take_size,
            lifespan,
        }
    }

    pub fn try_from_raw(
        start_offset: u64,
        end_offset: u64,
        density: u64,
        lifespan: u64,
    ) -> Result<Self, PhoenixIxError> {
        Self::try_from_raw_with_hidden_take_size(start_offset, end_offset, density, 0, lifespan)
    }

    pub fn try_from_raw_with_hidden_take_size(
        start_offset: u64,
        end_offset: u64,
        density: u64,
        top_level_hidden_take_size: u64,
        lifespan: u64,
    ) -> Result<Self, PhoenixIxError> {
        if start_offset >= end_offset {
            return Err(PhoenixIxError::InvalidSplineTickRegion);
        }
        let density =
            u32::try_from(density).map_err(|_| PhoenixIxError::InvalidSplineTickRegion)?;
        let top_level_hidden_take_size = u32::try_from(top_level_hidden_take_size)
            .map_err(|_| PhoenixIxError::InvalidSplineTickRegion)?;
        Ok(Self::new_with_hidden_take_size(
            start_offset,
            end_offset,
            density,
            top_level_hidden_take_size,
            lifespan,
        ))
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateSplinePriceParams {
    pub new_mid_price: u64,
    pub user_update_slot: Option<u64>,
    pub refresh_regions: bool,
}

impl UpdateSplinePriceParams {
    pub fn builder() -> UpdateSplinePriceParamsBuilder {
        UpdateSplinePriceParamsBuilder::new()
    }
}

#[derive(Default)]
pub struct UpdateSplinePriceParamsBuilder {
    new_mid_price: Option<u64>,
    user_update_slot: Option<u64>,
    refresh_regions: Option<bool>,
}

impl UpdateSplinePriceParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_mid_price(mut self, new_mid_price: u64) -> Self {
        self.new_mid_price = Some(new_mid_price);
        self
    }

    pub fn user_update_slot(mut self, user_update_slot: u64) -> Self {
        self.user_update_slot = Some(user_update_slot);
        self
    }

    pub fn refresh_regions(mut self, refresh_regions: bool) -> Self {
        self.refresh_regions = Some(refresh_regions);
        self
    }

    pub fn build(self) -> Result<UpdateSplinePriceParams, PhoenixIxError> {
        Ok(UpdateSplinePriceParams {
            new_mid_price: self
                .new_mid_price
                .ok_or(PhoenixIxError::MissingField("new_mid_price"))?,
            user_update_slot: self.user_update_slot,
            refresh_regions: self.refresh_regions.unwrap_or(true),
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateSplinePriceParamsWithOrdering {
    pub new_mid_price: u64,
    pub user_update_slot: Option<u64>,
    pub refresh_regions: bool,
    pub user_sequence_number: u64,
    pub client_order_id: [u8; 16],
    pub override_sequence_number: bool,
}

impl UpdateSplinePriceParamsWithOrdering {
    pub fn builder() -> UpdateSplinePriceParamsWithOrderingBuilder {
        UpdateSplinePriceParamsWithOrderingBuilder::new()
    }
}

#[derive(Default)]
pub struct UpdateSplinePriceParamsWithOrderingBuilder {
    new_mid_price: Option<u64>,
    user_update_slot: Option<u64>,
    refresh_regions: Option<bool>,
    user_sequence_number: Option<u64>,
    client_order_id: Option<[u8; 16]>,
    override_sequence_number: Option<bool>,
}

impl UpdateSplinePriceParamsWithOrderingBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_mid_price(mut self, new_mid_price: u64) -> Self {
        self.new_mid_price = Some(new_mid_price);
        self
    }

    pub fn user_update_slot(mut self, user_update_slot: u64) -> Self {
        self.user_update_slot = Some(user_update_slot);
        self
    }

    pub fn refresh_regions(mut self, refresh_regions: bool) -> Self {
        self.refresh_regions = Some(refresh_regions);
        self
    }

    pub fn user_sequence_number(mut self, user_sequence_number: u64) -> Self {
        self.user_sequence_number = Some(user_sequence_number);
        self
    }

    pub fn client_order_id(mut self, client_order_id: [u8; 16]) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn override_sequence_number(mut self, override_sequence_number: bool) -> Self {
        self.override_sequence_number = Some(override_sequence_number);
        self
    }

    pub fn build(self) -> Result<UpdateSplinePriceParamsWithOrdering, PhoenixIxError> {
        Ok(UpdateSplinePriceParamsWithOrdering {
            new_mid_price: self
                .new_mid_price
                .ok_or(PhoenixIxError::MissingField("new_mid_price"))?,
            user_update_slot: self.user_update_slot,
            refresh_regions: self.refresh_regions.unwrap_or(true),
            user_sequence_number: self
                .user_sequence_number
                .ok_or(PhoenixIxError::MissingField("user_sequence_number"))?,
            client_order_id: self.client_order_id.unwrap_or([0u8; 16]),
            override_sequence_number: self.override_sequence_number.unwrap_or(false),
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateSplineParametersParamsWithOrdering {
    pub bid_regions: Vec<TickRegionParams>,
    pub ask_regions: Vec<TickRegionParams>,
    pub refresh_regions: bool,
    pub user_sequence_number: u64,
    pub client_order_id: [u8; 16],
    pub override_sequence_number: bool,
}

impl UpdateSplineParametersParamsWithOrdering {
    pub fn builder() -> UpdateSplineParametersParamsWithOrderingBuilder {
        UpdateSplineParametersParamsWithOrderingBuilder::new()
    }
}

#[derive(Default)]
pub struct UpdateSplineParametersParamsWithOrderingBuilder {
    bid_regions: Vec<TickRegionParams>,
    ask_regions: Vec<TickRegionParams>,
    refresh_regions: Option<bool>,
    user_sequence_number: Option<u64>,
    client_order_id: Option<[u8; 16]>,
    override_sequence_number: Option<bool>,
}

impl UpdateSplineParametersParamsWithOrderingBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bid_regions(mut self, bid_regions: Vec<TickRegionParams>) -> Self {
        self.bid_regions = bid_regions;
        self
    }

    pub fn ask_regions(mut self, ask_regions: Vec<TickRegionParams>) -> Self {
        self.ask_regions = ask_regions;
        self
    }

    pub fn bid_regions_raw(
        mut self,
        regions: impl IntoIterator<Item = (u64, u64, u64, u64)>,
    ) -> Result<Self, PhoenixIxError> {
        self.bid_regions = regions
            .into_iter()
            .map(|(start, end, density, lifespan)| {
                TickRegionParams::try_from_raw(start, end, density, lifespan)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self)
    }

    pub fn ask_regions_raw(
        mut self,
        regions: impl IntoIterator<Item = (u64, u64, u64, u64)>,
    ) -> Result<Self, PhoenixIxError> {
        self.ask_regions = regions
            .into_iter()
            .map(|(start, end, density, lifespan)| {
                TickRegionParams::try_from_raw(start, end, density, lifespan)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self)
    }

    pub fn refresh_regions(mut self, refresh_regions: bool) -> Self {
        self.refresh_regions = Some(refresh_regions);
        self
    }

    pub fn user_sequence_number(mut self, user_sequence_number: u64) -> Self {
        self.user_sequence_number = Some(user_sequence_number);
        self
    }

    pub fn client_order_id(mut self, client_order_id: [u8; 16]) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn override_sequence_number(mut self, override_sequence_number: bool) -> Self {
        self.override_sequence_number = Some(override_sequence_number);
        self
    }

    pub fn build(self) -> Result<UpdateSplineParametersParamsWithOrdering, PhoenixIxError> {
        if self.bid_regions.is_empty() && self.ask_regions.is_empty() {
            return Err(PhoenixIxError::InvalidSplineTickRegion);
        }
        if self.bid_regions.len() > MAX_SPLINE_REGIONS
            || self.ask_regions.len() > MAX_SPLINE_REGIONS
        {
            return Err(PhoenixIxError::InvalidSplineTickRegion);
        }

        Ok(UpdateSplineParametersParamsWithOrdering {
            bid_regions: self.bid_regions,
            ask_regions: self.ask_regions,
            refresh_regions: self.refresh_regions.unwrap_or(true),
            user_sequence_number: self
                .user_sequence_number
                .ok_or(PhoenixIxError::MissingField("user_sequence_number"))?,
            client_order_id: self.client_order_id.unwrap_or([0u8; 16]),
            override_sequence_number: self.override_sequence_number.unwrap_or(false),
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PositionSizeLimits {
    pub long: u32,
    pub short: u32,
}

impl PositionSizeLimits {
    pub const fn symmetric(size: u32) -> Self {
        Self {
            long: size,
            short: size,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PositionSizeLimit {
    Disabled,
    Limit(PositionSizeLimits),
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateSplinePositionLimitsConfigParams {
    pub max_position_size: Option<PositionSizeLimit>,
    pub leverage_decrease_in_bps: Option<u32>,
}

impl UpdateSplinePositionLimitsConfigParams {
    pub fn builder() -> UpdateSplinePositionLimitsConfigParamsBuilder {
        UpdateSplinePositionLimitsConfigParamsBuilder::new()
    }
}

#[derive(Default)]
pub struct UpdateSplinePositionLimitsConfigParamsBuilder {
    max_position_size: Option<PositionSizeLimit>,
    leverage_decrease_in_bps: Option<u32>,
}

impl UpdateSplinePositionLimitsConfigParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_position_size(mut self, max_position_size: PositionSizeLimit) -> Self {
        self.max_position_size = Some(max_position_size);
        self
    }

    pub fn symmetric_max_position_size(mut self, size: u32) -> Self {
        self.max_position_size = Some(PositionSizeLimit::Limit(PositionSizeLimits::symmetric(
            size,
        )));
        self
    }

    pub fn disable_max_position_size(mut self) -> Self {
        self.max_position_size = Some(PositionSizeLimit::Disabled);
        self
    }

    pub fn leverage_decrease_in_bps(mut self, leverage_decrease_in_bps: u32) -> Self {
        self.leverage_decrease_in_bps = Some(leverage_decrease_in_bps);
        self
    }

    pub fn build(self) -> Result<UpdateSplinePositionLimitsConfigParams, PhoenixIxError> {
        if self.max_position_size.is_none() && self.leverage_decrease_in_bps.is_none() {
            return Err(PhoenixIxError::MissingSplineUpdate);
        }
        if self
            .leverage_decrease_in_bps
            .is_some_and(|bps| bps > 10_000)
        {
            return Err(PhoenixIxError::InvalidLeverageDecreaseBps);
        }

        Ok(UpdateSplinePositionLimitsConfigParams {
            max_position_size: self.max_position_size,
            leverage_decrease_in_bps: self.leverage_decrease_in_bps,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineUpdateAccounts {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    signer: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    spline_collection: Pubkey,
}

impl SplineUpdateAccounts {
    pub fn builder() -> SplineUpdateAccountsBuilder {
        SplineUpdateAccountsBuilder::new()
    }

    pub fn signer(&self) -> Pubkey {
        self.signer
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn spline_collection(&self) -> Pubkey {
        self.spline_collection
    }
}

#[derive(Default)]
pub struct SplineUpdateAccountsBuilder {
    signer: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
}

impl SplineUpdateAccountsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
        self
    }

    pub fn build(self) -> Result<SplineUpdateAccounts, PhoenixIxError> {
        Ok(SplineUpdateAccounts {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
        })
    }
}

pub fn create_register_spline_ix(
    params: RegisterSplineParams,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_register_spline_accounts(&params),
        data: PhoenixInstruction::RegisterSpline.discriminant().to_vec(),
    })
}

pub fn create_deactivate_spline_ix(
    params: DeactivateSplineParams,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_deactivate_spline_accounts(&params),
        data: PhoenixInstruction::DeactivateSpline.discriminant().to_vec(),
    })
}

pub fn create_update_spline_price_ix(
    accounts: SplineUpdateAccounts,
    params: UpdateSplinePriceParams,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_spline_update_accounts(&accounts),
        data: encode_with_instruction(PhoenixInstruction::UpdateSplinePrice, &params),
    })
}

pub fn create_update_spline_price_with_ordering_ix(
    accounts: SplineUpdateAccounts,
    params: UpdateSplinePriceParamsWithOrdering,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_spline_update_accounts(&accounts),
        data: encode_with_instruction(PhoenixInstruction::UpdateSplinePrice, &params),
    })
}

pub fn create_update_spline_parameters_with_ordering_ix(
    accounts: SplineUpdateAccounts,
    params: UpdateSplineParametersParamsWithOrdering,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_spline_update_accounts(&accounts),
        data: encode_with_instruction(PhoenixInstruction::UpdateSplineParameters, &params),
    })
}

pub fn create_update_spline_position_limits_config_ix(
    accounts: SplineUpdateAccounts,
    params: UpdateSplinePositionLimitsConfigParams,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_spline_update_accounts(&accounts),
        data: encode_with_instruction(
            PhoenixInstruction::UpdateSplinePositionLimitsConfig,
            &params,
        ),
    })
}

fn encode_with_instruction<T: BorshSerialize>(
    instruction: PhoenixInstruction,
    params: &T,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 64);
    data.extend_from_slice(&instruction.discriminant());
    data.extend_from_slice(&to_vec(params).expect("serializing spline params should not fail"));
    data
}

fn build_register_spline_accounts(params: &RegisterSplineParams) -> Vec<AccountMeta> {
    let mut accounts = log_accounts();
    accounts.extend(authority_instruction_accounts(
        params.authority(),
        params.permission_account(),
    ));
    accounts.extend([
        AccountMeta::writable_signer(params.payer()),
        AccountMeta::writable(params.spline_collection()),
        AccountMeta::readonly(params.market_account()),
    ]);
    extend_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );
    accounts.extend([
        AccountMeta::writable(params.trader_account()),
        AccountMeta::readonly(params.perp_asset_map()),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
    ]);
    accounts
}

fn build_deactivate_spline_accounts(params: &DeactivateSplineParams) -> Vec<AccountMeta> {
    let mut accounts = log_accounts();
    accounts.extend(authority_instruction_accounts(
        params.authority(),
        params.permission_account(),
    ));
    accounts.extend([
        AccountMeta::writable(params.spline_collection()),
        AccountMeta::readonly(params.market_account()),
    ]);
    extend_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );
    accounts.extend([
        AccountMeta::writable(params.trader_account()),
        AccountMeta::readonly(params.perp_asset_map()),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
    ]);
    accounts
}

fn build_spline_update_accounts(accounts: &SplineUpdateAccounts) -> Vec<AccountMeta> {
    let mut account_metas = log_accounts();
    account_metas.extend([
        AccountMeta::readonly_signer(accounts.signer()),
        AccountMeta::readonly(accounts.trader_account()),
        AccountMeta::writable(accounts.spline_collection()),
    ]);
    account_metas
}

fn log_accounts() -> Vec<AccountMeta> {
    vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
    ]
}

fn authority_instruction_accounts(
    authority: Pubkey,
    permission_account: Option<Pubkey>,
) -> [AccountMeta; 3] {
    [
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::readonly_signer(authority),
        AccountMeta::writable(permission_account.unwrap_or(authority)),
    ]
}

fn extend_index_accounts(
    accounts: &mut Vec<AccountMeta>,
    global_trader_index: &[Pubkey],
    active_trader_buffer: &[Pubkey],
) {
    accounts.extend(
        global_trader_index
            .iter()
            .copied()
            .map(AccountMeta::writable),
    );
    accounts.extend(
        active_trader_buffer
            .iter()
            .copied()
            .map(AccountMeta::writable),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update_accounts() -> SplineUpdateAccounts {
        SplineUpdateAccounts::builder()
            .signer(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .build()
            .unwrap()
    }

    #[test]
    fn tick_region_serializes_current_program_shape() {
        let region = TickRegionParams::new_with_hidden_take_size(1, 5, 10, 2, 99);
        let bytes = to_vec(&region).unwrap();

        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[0..8], &1u64.to_le_bytes());
        assert_eq!(&bytes[8..16], &5u64.to_le_bytes());
        assert_eq!(&bytes[16..20], &10u32.to_le_bytes());
        assert_eq!(&bytes[20..24], &2u32.to_le_bytes());
        assert_eq!(&bytes[24..32], &99u64.to_le_bytes());
    }

    #[test]
    fn update_spline_price_uses_legacy_payload_shape() {
        let params = UpdateSplinePriceParams::builder()
            .new_mid_price(42)
            .refresh_regions(true)
            .build()
            .unwrap();
        let ix = create_update_spline_price_ix(update_accounts(), params).unwrap();

        assert_eq!(
            &ix.data[..8],
            &PhoenixInstruction::UpdateSplinePrice.discriminant()
        );
        assert_eq!(ix.data.len(), 18);
        assert_eq!(&ix.data[8..16], &42u64.to_le_bytes());
        assert_eq!(ix.data[16], 0);
        assert_eq!(ix.data[17], 1);
    }

    #[test]
    fn update_spline_price_with_ordering_appends_sequence_fields() {
        let params = UpdateSplinePriceParamsWithOrdering::builder()
            .new_mid_price(42)
            .refresh_regions(true)
            .user_sequence_number(7)
            .client_order_id([3; 16])
            .build()
            .unwrap();
        let ix = create_update_spline_price_with_ordering_ix(update_accounts(), params).unwrap();

        assert_eq!(
            PhoenixInstruction::UpdateSplinePrice.discriminant(),
            crate::constants::compute_discriminant("global:update_spline_price")
        );
        assert_eq!(
            &ix.data[..8],
            &PhoenixInstruction::UpdateSplinePrice.discriminant()
        );
        assert_eq!(ix.data.len(), 43);
        assert_eq!(&ix.data[18..26], &7u64.to_le_bytes());
        assert_eq!(&ix.data[26..42], &[3; 16]);
        assert_eq!(ix.data[42], 0);
    }

    #[test]
    fn update_spline_parameters_requires_regions() {
        let result = UpdateSplineParametersParamsWithOrdering::builder()
            .user_sequence_number(1)
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::InvalidSplineTickRegion)
        ));
    }

    #[test]
    fn register_spline_account_order() {
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let permission = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let spline_collection = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let perp_asset_map = Pubkey::new_unique();
        let gti = Pubkey::new_unique();
        let atb = Pubkey::new_unique();
        let params = RegisterSplineParams::builder()
            .payer(payer)
            .authority(authority)
            .permission_account(permission)
            .market_account(market)
            .spline_collection(spline_collection)
            .trader_account(trader_account)
            .perp_asset_map(perp_asset_map)
            .global_trader_index(vec![gti])
            .active_trader_buffer(vec![atb])
            .build()
            .unwrap();

        let ix = create_register_spline_ix(params).unwrap();

        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert_eq!(ix.accounts[3].pubkey, authority);
        assert!(ix.accounts[3].is_signer);
        assert_eq!(ix.accounts[4].pubkey, permission);
        assert!(ix.accounts[4].is_writable);
        assert_eq!(ix.accounts[5].pubkey, payer);
        assert!(ix.accounts[5].is_signer);
        assert!(ix.accounts[5].is_writable);
        assert_eq!(ix.accounts[6].pubkey, spline_collection);
        assert_eq!(ix.accounts[7].pubkey, market);
        assert_eq!(ix.accounts[8].pubkey, gti);
        assert_eq!(ix.accounts[9].pubkey, atb);
        assert_eq!(ix.accounts[10].pubkey, trader_account);
        assert_eq!(ix.accounts[11].pubkey, perp_asset_map);
        assert_eq!(ix.accounts[12].pubkey, SYSTEM_PROGRAM_ID);
        assert_eq!(
            &ix.data[..8],
            &PhoenixInstruction::RegisterSpline.discriminant()
        );
    }
}
