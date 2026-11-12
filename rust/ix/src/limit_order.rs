//! Place limit order instruction construction.

use borsh::to_vec;
use solana_pubkey::Pubkey;

use crate::constants::{PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID};
use crate::error::PhoenixIxError;
use crate::order_packet::{OrderPacket, client_order_id_to_bytes};
use crate::types::{
    AccountMeta, Instruction, IsolatedCollateralFlow, OrderFlags, SelfTradeBehavior, Side,
    push_trader_index_accounts,
};

/// How a limit order interacts with the book when placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LimitOrderKind {
    /// Regular limit order: may match resting liquidity, then rests.
    #[default]
    Limit,
    /// Post-only order: never takes resting liquidity.
    ///
    /// `slide` is sent as the PostOnly packet's `slide` flag, which lets the
    /// program reprice a crossing order to the best non-crossing tick.
    PostOnly { slide: bool },
}

/// Parameters for placing a limit order.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LimitOrderParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    orderbook: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    spline_collection: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    side: Side,
    price_in_ticks: u64,
    num_base_lots: u64,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: Option<u64>,
    client_order_id: u128,
    last_valid_slot: Option<u64>,
    order_flags: OrderFlags,
    cancel_existing: bool,
    /// Market symbol (e.g. "SOL"). Not serialized into the instruction.
    symbol: String,
    /// Subaccount index (0 = cross-margin, 1+ = isolated). Not serialized.
    subaccount_index: u8,
    /// Limit or post-only. Defaults to [`LimitOrderKind::Limit`].
    #[cfg_attr(feature = "serde", serde(default))]
    kind: LimitOrderKind,
}

impl LimitOrderParams {
    /// Start building with the builder pattern.
    pub fn builder() -> LimitOrderParamsBuilder {
        LimitOrderParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn orderbook(&self) -> Pubkey {
        self.orderbook
    }

    pub fn spline_collection(&self) -> Pubkey {
        self.spline_collection
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn price_in_ticks(&self) -> u64 {
        self.price_in_ticks
    }

    pub fn num_base_lots(&self) -> u64 {
        self.num_base_lots
    }

    pub fn self_trade_behavior(&self) -> SelfTradeBehavior {
        self.self_trade_behavior
    }

    pub fn match_limit(&self) -> Option<u64> {
        self.match_limit
    }

    pub fn client_order_id(&self) -> u128 {
        self.client_order_id
    }

    pub fn last_valid_slot(&self) -> Option<u64> {
        self.last_valid_slot
    }

    pub fn order_flags(&self) -> OrderFlags {
        self.order_flags
    }

    pub fn cancel_existing(&self) -> bool {
        self.cancel_existing
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn subaccount_index(&self) -> u8 {
        self.subaccount_index
    }

    pub fn kind(&self) -> LimitOrderKind {
        self.kind
    }

    /// The order packet placed by this order, as encoded after the
    /// instruction discriminant.
    ///
    /// A [`LimitOrderKind::PostOnly`] order yields a PostOnly packet, which
    /// carries neither `self_trade_behavior` nor `match_limit`.
    pub fn order_packet(&self) -> OrderPacket {
        let client_order_id = client_order_id_to_bytes(self.client_order_id);
        match self.kind {
            LimitOrderKind::Limit => OrderPacket::limit(
                self.side,
                self.price_in_ticks,
                self.num_base_lots,
                self.self_trade_behavior,
                self.match_limit,
                client_order_id,
                self.last_valid_slot,
                self.order_flags,
                self.cancel_existing,
            ),
            LimitOrderKind::PostOnly { slide } => OrderPacket::post_only(
                self.side,
                self.price_in_ticks,
                self.num_base_lots,
                client_order_id,
                slide,
                self.last_valid_slot,
                self.order_flags,
                self.cancel_existing,
            ),
        }
    }
}

/// Builder for `LimitOrderParams`.
#[derive(Default)]
pub struct LimitOrderParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    side: Option<Side>,
    price_in_ticks: Option<u64>,
    num_base_lots: Option<u64>,
    self_trade_behavior: Option<SelfTradeBehavior>,
    match_limit: Option<u64>,
    client_order_id: Option<u128>,
    last_valid_slot: Option<u64>,
    order_flags: Option<OrderFlags>,
    cancel_existing: Option<bool>,
    symbol: Option<String>,
    subaccount_index: Option<u8>,
    kind: LimitOrderKind,
}

impl LimitOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
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

    pub fn orderbook(mut self, orderbook: Pubkey) -> Self {
        self.orderbook = Some(orderbook);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
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

    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    pub fn price_in_ticks(mut self, price_in_ticks: u64) -> Self {
        self.price_in_ticks = Some(price_in_ticks);
        self
    }

    pub fn num_base_lots(mut self, num_base_lots: u64) -> Self {
        self.num_base_lots = Some(num_base_lots);
        self
    }

    pub fn self_trade_behavior(mut self, self_trade_behavior: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = Some(self_trade_behavior);
        self
    }

    pub fn match_limit(mut self, match_limit: u64) -> Self {
        self.match_limit = Some(match_limit);
        self
    }

    pub fn client_order_id(mut self, client_order_id: u128) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn last_valid_slot(mut self, last_valid_slot: u64) -> Self {
        self.last_valid_slot = Some(last_valid_slot);
        self
    }

    pub fn order_flags(mut self, order_flags: OrderFlags) -> Self {
        self.order_flags = Some(order_flags);
        self
    }

    pub fn cancel_existing(mut self, cancel_existing: bool) -> Self {
        self.cancel_existing = Some(cancel_existing);
        self
    }

    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn subaccount_index(mut self, subaccount_index: u8) -> Self {
        self.subaccount_index = Some(subaccount_index);
        self
    }

    /// Place the order as post-only (see [`LimitOrderKind::PostOnly`]).
    ///
    /// Post-only orders cannot be combined with
    /// [`Self::self_trade_behavior`] or [`Self::match_limit`]; `build()`
    /// rejects that combination with
    /// [`PhoenixIxError::PostOnlyIncompatible`].
    pub fn post_only(mut self, slide: bool) -> Self {
        self.kind = LimitOrderKind::PostOnly { slide };
        self
    }

    pub fn build(self) -> Result<LimitOrderParams, PhoenixIxError> {
        if matches!(self.kind, LimitOrderKind::PostOnly { .. }) {
            if self.self_trade_behavior.is_some() {
                return Err(PhoenixIxError::PostOnlyIncompatible("self_trade_behavior"));
            }
            if self.match_limit.is_some() {
                return Err(PhoenixIxError::PostOnlyIncompatible("match_limit"));
            }
        }
        Ok(LimitOrderParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            orderbook: self
                .orderbook
                .ok_or(PhoenixIxError::MissingField("orderbook"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
            global_trader_index: self
                .global_trader_index
                .ok_or(PhoenixIxError::MissingField("global_trader_index"))?,
            active_trader_buffer: self
                .active_trader_buffer
                .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?,
            side: self.side.ok_or(PhoenixIxError::MissingField("side"))?,
            price_in_ticks: self
                .price_in_ticks
                .ok_or(PhoenixIxError::MissingField("price_in_ticks"))?,
            num_base_lots: self
                .num_base_lots
                .ok_or(PhoenixIxError::MissingField("num_base_lots"))?,
            self_trade_behavior: self.self_trade_behavior.unwrap_or(SelfTradeBehavior::Abort),
            match_limit: self.match_limit,
            client_order_id: self.client_order_id.unwrap_or(0),
            last_valid_slot: self.last_valid_slot,
            order_flags: self.order_flags.unwrap_or(OrderFlags::None),
            cancel_existing: self.cancel_existing.unwrap_or(false),
            symbol: self.symbol.unwrap_or_default(),
            subaccount_index: self.subaccount_index.unwrap_or(0),
            kind: self.kind,
        })
    }
}

/// Create a place limit order instruction.
///
/// # Arguments
///
/// * `params` - The limit order parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
///
/// # Errors
///
/// Returns an error if required parameters are missing.
pub fn create_place_limit_order_ix(
    params: LimitOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    let data = encode_limit_order(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate(params: &LimitOrderParams) -> Result<(), PhoenixIxError> {
    if params.global_trader_index().is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if params.active_trader_buffer().is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    Ok(())
}

fn encode_limit_order(params: &LimitOrderParams) -> Vec<u8> {
    let mut data = Vec::new();

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&crate::PhoenixInstruction::PlaceLimitOrder.discriminant());

    data.extend_from_slice(
        &to_vec(&params.order_packet().kind).expect("serialization should not fail"),
    );

    data
}

fn build_accounts(params: &LimitOrderParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // LogAccountGroupAccounts (2 accounts)
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));

    // MarketActionInstructionGroupAccounts
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    accounts.push(AccountMeta::writable(params.trader_account()));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));

    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    accounts.push(AccountMeta::writable(params.orderbook()));
    accounts.push(AccountMeta::writable(params.spline_collection()));

    accounts
}

/// Parameters for an isolated margin limit order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsolatedLimitOrderParams {
    pub side: Side,
    pub price_in_ticks: u64,
    pub num_base_lots: u64,
    pub self_trade_behavior: SelfTradeBehavior,
    pub match_limit: Option<u64>,
    pub client_order_id: u128,
    pub last_valid_slot: Option<u64>,
    pub order_flags: OrderFlags,
    pub cancel_existing: bool,
    pub allow_cross_and_isolated: bool,
    pub collateral: Option<IsolatedCollateralFlow>,
    pub target_leverage_bps: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_limit_order_ix() {
        let params = LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(Side::Bid)
            .price_in_ticks(50000)
            .num_base_lots(1000)
            .self_trade_behavior(SelfTradeBehavior::CancelProvide)
            .client_order_id(123)
            .build()
            .unwrap();

        let ix = create_place_limit_order_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 2 log accounts + 4 base accounts + 1 global trader index + 1 active
        // trader buffer + 2 market accounts = 10
        assert_eq!(ix.accounts.len(), 10);
        // Data should start with discriminant
        assert_eq!(
            &ix.data[..8],
            &crate::PhoenixInstruction::PlaceLimitOrder.discriminant()
        );
    }

    #[test]
    fn test_empty_global_trader_index_fails() {
        let params = LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(Side::Bid)
            .price_in_ticks(50000)
            .num_base_lots(1000)
            .build()
            .unwrap();

        let result = create_place_limit_order_ix(params);
        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            // Missing other required fields
            .build();

        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }

    fn post_only_builder() -> LimitOrderParamsBuilder {
        LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(Side::Ask)
            .price_in_ticks(50000)
            .num_base_lots(1000)
            .post_only(true)
    }

    #[test]
    fn test_post_only_rejects_explicit_match_limit() {
        let result = post_only_builder().match_limit(10).build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::PostOnlyIncompatible("match_limit"))
        ));
    }

    #[test]
    fn test_post_only_rejects_explicit_self_trade_behavior() {
        let result = post_only_builder()
            .self_trade_behavior(SelfTradeBehavior::Abort)
            .build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::PostOnlyIncompatible("self_trade_behavior"))
        ));
    }
}
