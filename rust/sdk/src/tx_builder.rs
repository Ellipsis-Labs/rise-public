//! Transaction builder for Phoenix perpetuals exchange.
//!
//! This module provides `PhoenixTxBuilder`, which builds Solana instructions
//! from exchange metadata without requiring network access or keypairs.

use std::str::FromStr;

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use thiserror::Error;

use crate::PhoenixMetadata;
use crate::order_tickets::{
    BracketLeg, BracketLegExecution, BracketLegOrders, BracketLegSize, BracketLegTicket,
    DEFAULT_BRACKET_LEG_SLIPPAGE_BPS, LimitOrderTicket, MarketOrderTicket, OrderTicketMetadata,
};
use crate::phoenix_rise_ix::{
    CancelId, CancelOrdersByIdParams, CancelStopLossParams, CondensedOrder,
    CreateConditionalOrdersAccountParams, DepositFundsParams, Direction, EmberDepositParams,
    EmberWithdrawParams, HawkeyeBboViewAccounts, HawkeyeTraderViewAccounts, IsolatedCollateralFlow,
    IsolatedLimitOrderParams, IsolatedMarketOrderParams, LimitOrderParams,
    MarketOrderDelegatedParams, MarketOrderParams, MultiLimitOrderParams, OrderPacket,
    PHOENIX_PROGRAM_ID, PlaceLimitOrderWithConditionalsParams, PlacePositionConditionalOrderParams,
    PlaceStopLossParams, RegisterTraderParams, Side, SplApproveParams, StopLossOrderKind,
    SyncParentToChildParams, TransferCollateralChildToParentParams, TransferCollateralParams,
    TriggerOrderParams, USDC_MINT, WithdrawFundsParams, client_order_id_to_bytes,
    create_associated_token_account_idempotent_ix, create_cancel_orders_by_id_ix,
    create_cancel_stop_loss_ix, create_create_conditional_orders_account_ix,
    create_deposit_funds_ix, create_ember_deposit_ix, create_ember_withdraw_ix,
    create_hawkeye_view_bbo_ix, create_hawkeye_view_funding_ix,
    create_hawkeye_view_liquidation_price_ix, create_hawkeye_view_margin_for_asset_ix,
    create_hawkeye_view_margin_ix, create_place_limit_order_ix,
    create_place_limit_order_with_conditionals_ix, create_place_market_order_delegated_ix,
    create_place_market_order_ix, create_place_multi_limit_order_ix,
    create_place_position_conditional_order_ix, create_place_stop_loss_ix,
    create_register_trader_ix, create_spl_approve_ix, create_sync_parent_to_child_ix,
    create_transfer_collateral_child_to_parent_ix, create_transfer_collateral_ix,
    create_withdraw_funds_ix, get_associated_token_address, get_conditional_orders_address,
    get_ember_state_address, get_stop_loss_address,
};
use crate::phoenix_rise_math::{MathError, WrapperNum};
use crate::phoenix_rise_types::accounts::StopLosses;
use crate::phoenix_rise_types::{
    CROSS_MARGIN_SUBACCOUNT_IDX, ExchangeMarketConfig, Trader, TraderKey,
};

const USDC_NATIVE_DECIMALS: f64 = 1_000_000.0;
const DEFAULT_CONDITIONAL_ORDERS_CAPACITY: u8 = 8;
const BPS_DENOMINATOR: u128 = 10_000;

/// Errors that can occur when building Phoenix transactions.
#[derive(Debug, Error)]
pub enum PhoenixTxBuilderError {
    /// Instruction construction error.
    #[error("Instruction error: {0}")]
    Instruction(#[from] crate::phoenix_rise_ix::PhoenixIxError),

    /// Failed to parse pubkey.
    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(#[from] solana_pubkey::ParsePubkeyError),

    /// Unknown market symbol.
    #[error("Unknown symbol: {0}")]
    UnknownSymbol(String),

    /// Math conversion error (e.g., price to ticks).
    #[error("Math error: {0}")]
    Math(#[from] MathError),

    /// Insufficient collateral in parent (cross-margin) subaccount.
    #[error("Insufficient parent collateral: need {need} but have {have} quote lots")]
    InsufficientParentCollateral { need: u64, have: u64 },

    /// All isolated subaccount slots are occupied.
    #[error("No available isolated subaccount slot")]
    NoAvailableSubaccount,

    /// Cross-margin subaccount already has a position in this market.
    #[error("Cross-margin subaccount already has a position in {0}")]
    CrossMarginPositionExists(String),

    /// Attempted to place an order on an isolated-only market using the
    /// cross-margin subaccount.
    #[error("{0} is isolated-only and cannot be traded on the cross-margin subaccount")]
    IsolatedOnlyMarket(String),

    /// RPC fetch failed while resolving bracket-order prerequisites.
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Invalid bracket leg execution price configuration.
    #[error("Invalid bracket leg execution price: {0}")]
    InvalidBracketLegExecutionPrice(String),

    /// Attached limit-order conditionals do not yet support explicit per-leg
    /// sizing in the local tx builder.
    #[error(
        "Explicit TP/SL sizing for attached limit-order brackets is not supported yet; omit the \
         leg sizes or use position conditionals after fill"
    )]
    UnsupportedLimitBracketLegSizing,

    /// Delegated market-order builder only constructs the delegated market
    /// order instruction, not follow-on conditional-order legs.
    #[error("Delegated market orders do not support bracket legs in the local tx builder")]
    UnsupportedDelegatedMarketOrderBrackets,
}

pub(crate) struct ParsedAddresses {
    pub(crate) perp_asset_map: Pubkey,
    pub(crate) global_trader_index: Vec<Pubkey>,
    pub(crate) active_trader_buffer: Vec<Pubkey>,
    pub(crate) orderbook: Pubkey,
    pub(crate) spline_collection: Pubkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolvedBracketLeg {
    size: BracketLegSize,
    trigger: TriggerOrderParams,
}

/// Transaction builder for Phoenix perpetuals exchange.
///
/// Builds Solana instructions from exchange metadata without requiring
/// network access. Use this when you need fine-grained control over
/// transaction construction or want to batch instructions.
///
/// # Example
///
/// ```no_run
/// use phoenix_rise::{
///     MarketOrderTicket, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, Side,
/// };
/// use solana_pubkey::Pubkey;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let http = PhoenixHttpClient::new_from_env()?;
/// let exchange = http.get_exchange().await?.into();
/// let metadata = PhoenixMetadata::new(exchange);
/// let builder = PhoenixTxBuilder::new(&metadata);
///
/// let authority = Pubkey::new_unique();
/// let trader_pda = Pubkey::new_unique();
/// let ticket = MarketOrderTicket::builder()
///     .authority(authority)
///     .trader_account(trader_pda)
///     .symbol("SOL")
///     .side(Side::Bid)
///     .num_base_lots(100)
///     .build()?;
///
/// // Build instructions without sending
/// let ixs = builder.place_market_order(ticket).await?;
/// # Ok(())
/// # }
/// ```
pub struct PhoenixTxBuilder<'a> {
    metadata: &'a PhoenixMetadata,
}

impl std::fmt::Debug for PhoenixTxBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhoenixTxBuilder")
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl<'a> PhoenixTxBuilder<'a> {
    /// Creates a new transaction builder from exchange metadata.
    pub fn new(metadata: &'a PhoenixMetadata) -> Self {
        Self { metadata }
    }

    fn trader_pda(authority: &Pubkey, pda_index: u8, subaccount_index: u8) -> Pubkey {
        let pda_schema = [pda_index, subaccount_index];
        Pubkey::find_program_address(
            &[b"trader", authority.as_ref(), pda_schema.as_ref()],
            &*PHOENIX_PROGRAM_ID,
        )
        .0
    }

    /// Build an instruction to create a trader conditional-orders account.
    pub fn build_create_conditional_orders_account(
        &self,
        payer: Pubkey,
        trader_wallet: Pubkey,
        trader_account: Pubkey,
        capacity: u8,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let params = CreateConditionalOrdersAccountParams::builder()
            .payer(payer)
            .trader_wallet(trader_wallet)
            .trader_account(trader_account)
            .capacity(capacity)
            .build()?;

        let ix = create_create_conditional_orders_account_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Resolve the metadata required to convert an order ticket into ix params.
    pub fn order_ticket_metadata(
        &self,
        symbol: &str,
    ) -> Result<OrderTicketMetadata<'_>, PhoenixTxBuilderError> {
        let market_config = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let market_calc = self
            .metadata
            .get_market_calculator(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        Ok(OrderTicketMetadata {
            market_calc,
            market_config,
            exchange_keys: self.metadata.keys(),
        })
    }

    /// Build a market order from a trader-facing ticket, optionally appending
    /// bracket legs from `ticket.bracket_leg_ticket`.
    pub async fn place_market_order(
        &self,
        ticket: MarketOrderTicket,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let bracket = ticket.bracket_leg_ticket().cloned();
        let params = ticket.to_params(self.order_ticket_metadata(ticket.symbol())?)?;
        if let Some(bracket) = bracket.filter(|bracket| !bracket.is_empty()) {
            return self.create_bracket_market_order_ixs(params, &bracket).await;
        }
        self.create_market_order_ixs(params)
    }

    /// Build a delegated market order from a trader-facing ticket.
    ///
    /// `ticket.authority()` identifies the trader account authority for
    /// PDA/metadata resolution. `trader_wallet` is the signing wallet. Pass
    /// `None` for `permission_account` only when `trader_wallet` is the trader
    /// account authority or primary position authority; this puts
    /// `trader_wallet` itself in the permission-account slot. A
    /// secondary/delegated wallet needs an actual
    /// permission account bridging it to the trader account authority.
    pub fn place_market_order_delegated(
        &self,
        ticket: MarketOrderTicket,
        trader_wallet: Pubkey,
        permission_account: Option<Pubkey>,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        if ticket
            .bracket_leg_ticket()
            .is_some_and(|bracket| !bracket.is_empty())
        {
            return Err(PhoenixTxBuilderError::UnsupportedDelegatedMarketOrderBrackets);
        }

        let market_order = ticket.to_params(self.order_ticket_metadata(ticket.symbol())?)?;
        let params = MarketOrderDelegatedParams::builder()
            .market_order(market_order)
            .trader_wallet(trader_wallet)
            .permission_account(permission_account.unwrap_or(trader_wallet))
            .build()?;
        self.build_market_order_delegated_with_params(params)
    }

    /// Build a delegated market-order instruction with pre-built params.
    pub fn build_market_order_delegated_with_params(
        &self,
        params: MarketOrderDelegatedParams,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        if params.market_order().subaccount_index() == CROSS_MARGIN_SUBACCOUNT_IDX {
            self.reject_isolated_only(params.market_order().symbol())?;
        }

        let ix = create_place_market_order_delegated_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Build a limit order from a trader-facing ticket, optionally attaching
    /// bracket legs from `ticket.bracket_leg_ticket`.
    pub async fn place_limit_order(
        &self,
        ticket: LimitOrderTicket,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let bracket = ticket.bracket_leg_ticket().cloned();
        let params = ticket.to_params(self.order_ticket_metadata(ticket.symbol())?)?;
        if let Some(bracket) = bracket.filter(|bracket| !bracket.is_empty()) {
            return self.create_bracket_limit_order_ixs(params, &bracket).await;
        }
        self.create_limit_order_ixs(params)
    }

    fn create_market_order_ixs(
        &self,
        params: MarketOrderParams,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        if params.subaccount_index() == CROSS_MARGIN_SUBACCOUNT_IDX {
            self.reject_isolated_only(params.symbol())?;
        }

        let ix = create_place_market_order_ix(params)?;
        Ok(vec![ix.into()])
    }

    async fn create_bracket_market_order_ixs(
        &self,
        params: MarketOrderParams,
        bracket: &BracketLegTicket,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let authority = params.trader();
        let trader_account = params.trader_account();
        let symbol = params.symbol().to_string();
        let side = params.side();

        let mut ixs = self
            .maybe_create_conditional_orders_account_ixs(
                authority,
                trader_account,
                bracket.rpc_client(),
            )
            .await?;
        ixs.extend(self.create_market_order_ixs(params)?);
        ixs.extend(self.build_bracket_leg_orders(
            authority,
            trader_account,
            &symbol,
            side,
            bracket.bracket_legs(),
        )?);
        Ok(ixs)
    }

    fn create_limit_order_ixs(
        &self,
        params: LimitOrderParams,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        if params.subaccount_index() == CROSS_MARGIN_SUBACCOUNT_IDX {
            self.reject_isolated_only(params.symbol())?;
        }

        let ix = create_place_limit_order_ix(params)?;
        Ok(vec![ix.into()])
    }

    async fn create_bracket_limit_order_ixs(
        &self,
        params: LimitOrderParams,
        bracket: &BracketLegTicket,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        if params.subaccount_index() == CROSS_MARGIN_SUBACCOUNT_IDX {
            self.reject_isolated_only(params.symbol())?;
        }

        if bracket.bracket_legs().has_explicit_sizes() {
            return Err(PhoenixTxBuilderError::UnsupportedLimitBracketLegSizing);
        }

        let mut ixs = self
            .maybe_create_conditional_orders_account_ixs(
                params.trader(),
                params.trader_account(),
                bracket.rpc_client(),
            )
            .await?;
        ixs.push(self.create_limit_order_with_conditionals_ix(&params, bracket.bracket_legs())?);
        Ok(ixs)
    }

    /// Build a multi-limit-order instruction with pre-built params.
    pub fn build_multi_limit_order_with_params(
        &self,
        params: MultiLimitOrderParams,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let ix = create_place_multi_limit_order_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Build a multi-limit-order instruction.
    ///
    /// Places multiple post-only limit orders (bids and asks) in a single
    /// instruction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `trader_pda` - The trader's PDA account
    /// * `symbol` - Market symbol
    /// * `bids` - Bid orders as (price_usd, num_base_lots) tuples
    /// * `asks` - Ask orders as (price_usd, num_base_lots) tuples
    /// * `slide` - Whether orders should slide to top of book if they would
    ///   cross
    pub fn build_multi_limit_order(
        &self,
        authority: Pubkey,
        trader_pda: Pubkey,
        symbol: &str,
        bids: &[(f64, u64)],
        asks: &[(f64, u64)],
        slide: bool,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        let calc = self
            .metadata
            .get_market_calculator(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        let addrs = self.parse_addresses(market)?;

        let bid_orders: Vec<CondensedOrder> = bids
            .iter()
            .map(|(price, size)| {
                Ok(CondensedOrder {
                    price_in_ticks: calc.price_to_ticks(*price)?.as_inner(),
                    size_in_base_lots: *size,
                    last_valid_slot: None,
                })
            })
            .collect::<Result<_, PhoenixTxBuilderError>>()?;

        let ask_orders: Vec<CondensedOrder> = asks
            .iter()
            .map(|(price, size)| {
                Ok(CondensedOrder {
                    price_in_ticks: calc.price_to_ticks(*price)?.as_inner(),
                    size_in_base_lots: *size,
                    last_valid_slot: None,
                })
            })
            .collect::<Result<_, PhoenixTxBuilderError>>()?;

        let params = MultiLimitOrderParams::builder()
            .trader(authority)
            .trader_account(trader_pda)
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index)
            .active_trader_buffer(addrs.active_trader_buffer)
            .bids(bid_orders)
            .asks(ask_orders)
            .slide(slide)
            .symbol(symbol)
            .build()?;

        self.build_multi_limit_order_with_params(params)
    }

    /// Build cancel orders instruction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `trader_pda` - The trader's PDA account
    /// * `symbol` - Market symbol
    /// * `order_ids` - List of order IDs to cancel
    ///
    /// # Returns
    ///
    /// A vector containing the cancel orders instruction.
    pub fn build_cancel_orders(
        &self,
        authority: Pubkey,
        trader_pda: Pubkey,
        symbol: &str,
        order_ids: Vec<CancelId>,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        let addrs = self.parse_addresses(market)?;

        let params = CancelOrdersByIdParams::builder()
            .trader(authority)
            .trader_account(trader_pda)
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index)
            .active_trader_buffer(addrs.active_trader_buffer)
            .order_ids(order_ids)
            .build()?;

        let ix = create_cancel_orders_by_id_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Build a cancel stop loss instruction.
    ///
    /// Cancels an active stop-loss or take-profit order for a given market
    /// and execution direction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `trader_pda` - The trader's PDA account
    /// * `symbol` - Market symbol ("SOL", "BTC", "ETH")
    /// * `execution_direction` - Which leg to cancel (`LessThan` for SL on
    ///   longs, `GreaterThan` for TP on longs; reversed for shorts)
    pub fn build_cancel_bracket_leg(
        &self,
        authority: Pubkey,
        trader_pda: Pubkey,
        symbol: &str,
        execution_direction: Direction,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        self.build_cancel_bracket_leg_with_funder(
            authority,
            authority,
            trader_pda,
            symbol,
            execution_direction,
        )
    }

    /// Build a cancel stop loss instruction using the funding key stored on
    /// the stop-loss account.
    ///
    /// Legacy stop-loss account validation requires the cancel `funder`
    /// account to match the account that originally funded the stop-loss PDA.
    /// This mirrors the API ix route behavior while keeping instruction
    /// construction local to the Rust SDK.
    pub async fn build_cancel_bracket_leg_from_account(
        &self,
        authority: Pubkey,
        trader_pda: Pubkey,
        symbol: &str,
        execution_direction: Direction,
        rpc: &RpcClient,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let asset_id = market.asset_id as u64;
        let stop_loss = get_stop_loss_address(&trader_pda, asset_id);
        let account = rpc
            .get_account_with_commitment(&stop_loss, rpc.commitment())
            .await
            .map_err(|err| {
                PhoenixTxBuilderError::Rpc(format!(
                    "failed to fetch stop loss account {stop_loss}: {err}"
                ))
            })?
            .value
            .ok_or_else(|| {
                PhoenixTxBuilderError::Rpc(format!(
                    "stop loss account {stop_loss} not found for trader {trader_pda} on {symbol}"
                ))
            })?;

        if account.owner != *PHOENIX_PROGRAM_ID {
            let program_id = *PHOENIX_PROGRAM_ID;
            return Err(PhoenixTxBuilderError::Rpc(format!(
                "stop loss account {stop_loss} has owner {}, expected {program_id}",
                account.owner
            )));
        }

        let stop_losses = StopLosses::try_from_account_bytes(&account.data).map_err(|err| {
            PhoenixTxBuilderError::Rpc(format!(
                "failed to decode stop loss account {stop_loss}: {err}"
            ))
        })?;
        if stop_losses.trader_key != trader_pda {
            return Err(PhoenixTxBuilderError::Rpc(format!(
                "stop loss account {stop_loss} belongs to trader {}, expected {trader_pda}",
                stop_losses.trader_key
            )));
        }
        if stop_losses.asset_id != asset_id {
            return Err(PhoenixTxBuilderError::Rpc(format!(
                "stop loss account {stop_loss} has asset id {}, expected {asset_id}",
                stop_losses.asset_id
            )));
        }

        self.build_cancel_bracket_leg_with_funder(
            stop_losses.funding_key,
            authority,
            trader_pda,
            symbol,
            execution_direction,
        )
    }

    fn build_cancel_bracket_leg_with_funder(
        &self,
        funder: Pubkey,
        authority: Pubkey,
        trader_pda: Pubkey,
        symbol: &str,
        execution_direction: Direction,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let asset_id = market.asset_id as u64;

        let params = CancelStopLossParams::builder()
            .funder(funder)
            .trader_account(trader_pda)
            .position_authority(authority)
            .asset_id(asset_id)
            .execution_direction(execution_direction)
            .build()?;

        let ix = create_cancel_stop_loss_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Build position-conditional TP/SL instructions from a bracket ticket.
    pub async fn place_position_bracket_order(
        &self,
        authority: Pubkey,
        trader_account: Pubkey,
        symbol: &str,
        position_side: Side,
        bracket: BracketLegTicket,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        if bracket.is_empty() {
            return Ok(Vec::new());
        }

        let mut ixs = self
            .maybe_create_conditional_orders_account_ixs(
                authority,
                trader_account,
                bracket.rpc_client(),
            )
            .await?;
        ixs.extend(self.build_bracket_leg_orders(
            authority,
            trader_account,
            symbol,
            position_side,
            bracket.bracket_legs(),
        )?);
        Ok(ixs)
    }

    /// Build legacy stop-loss/take-profit trigger instructions.
    ///
    /// These mutate the legacy stop-loss account and emit trader-state
    /// position trigger deltas. Use `place_position_bracket_order` for the
    /// newer conditional-order account flow.
    pub fn build_stop_loss_orders(
        &self,
        authority: Pubkey,
        trader_account: Pubkey,
        symbol: &str,
        position_side: Side,
        bracket: &BracketLegOrders,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let addrs = self.parse_addresses(market)?;
        let asset_id = market.asset_id as u64;
        let stop_loss_account = get_stop_loss_address(&trader_account, asset_id);
        let resolved_legs = self.build_resolved_bracket_legs(symbol, position_side, bracket)?;

        resolved_legs
            .into_iter()
            .map(|leg| {
                let params = PlaceStopLossParams::builder()
                    .funder(authority)
                    .trader_account(trader_account)
                    .position_authority(authority)
                    .perp_asset_map(addrs.perp_asset_map)
                    .orderbook(addrs.orderbook)
                    .spline_collection(addrs.spline_collection)
                    .global_trader_index(addrs.global_trader_index.clone())
                    .active_trader_buffer(addrs.active_trader_buffer.clone())
                    .stop_loss_account(stop_loss_account)
                    .asset_id(asset_id)
                    .trigger_price(leg.trigger.trigger_price())
                    .execution_price(leg.trigger.execution_price())
                    .trade_side(leg.trigger.trade_side())
                    .execution_direction(leg.trigger.trigger_direction())
                    .order_kind(leg.trigger.order_kind())
                    .build()?;
                Ok(create_place_stop_loss_ix(params)?.into())
            })
            .collect()
    }

    /// Build deposit funds instructions.
    ///
    /// This method builds the full deposit flow:
    /// 1. Creates ATA for Phoenix tokens if needed (idempotent)
    /// 2. Deposits USDC via Ember to receive Phoenix tokens
    /// 3. Deposits Phoenix tokens into the Phoenix protocol
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `trader_pda` - The trader's PDA account
    /// * `usdc_amount` - Amount of USDC to deposit (e.g., 100.0 for $100)
    ///
    /// # Returns
    ///
    /// A vector containing 3 instructions that should be sent in a single
    /// transaction.
    pub fn build_deposit_funds(
        &self,
        authority: Pubkey,
        trader_pda: Pubkey,
        usdc_amount: f64,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        // Convert USDC amount to base units (6 decimals)
        let amount = (usdc_amount * 1_000_000.0) as u64;

        // Get exchange keys from metadata
        let keys = self.metadata.keys();
        let canonical_mint = Pubkey::from_str(&keys.canonical_mint)?;
        let global_vault = Pubkey::from_str(&keys.global_vault)?;
        let global_trader_index = parse_pubkey_vec(&keys.global_trader_index)?;
        let active_trader_buffer = parse_pubkey_vec(&keys.active_trader_buffer)?;

        // Derive addresses
        let trader_usdc_ata = get_associated_token_address(&authority, &USDC_MINT);
        let trader_phoenix_ata = get_associated_token_address(&authority, &canonical_mint);

        // 1. Create ATA instruction (idempotent)
        let create_ata_ix =
            create_associated_token_account_idempotent_ix(authority, authority, canonical_mint);

        // 2. Ember deposit instruction (USDC -> Phoenix tokens)
        let ember_params = EmberDepositParams::builder()
            .trader(authority)
            .usdc_mint(USDC_MINT)
            .canonical_mint(canonical_mint)
            .trader_usdc_account(trader_usdc_ata)
            .trader_phoenix_account(trader_phoenix_ata)
            .amount(amount)
            .build()?;
        let ember_ix = create_ember_deposit_ix(ember_params)?;

        // 3. Deposit funds instruction (Phoenix tokens -> protocol)
        let deposit_params = DepositFundsParams::builder()
            .trader(authority)
            .trader_account(trader_pda)
            .canonical_mint(canonical_mint)
            .global_vault(global_vault)
            .trader_token_account(trader_phoenix_ata)
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .amount(amount)
            .build()?;
        let deposit_ix = create_deposit_funds_ix(deposit_params)?;

        Ok(vec![
            create_ata_ix.into(),
            ember_ix.into(),
            deposit_ix.into(),
        ])
    }

    /// Build withdraw funds instructions.
    ///
    /// This method builds the full withdrawal flow:
    /// 1. Creates ATA for Phoenix tokens if needed (idempotent)
    /// 2. Approves Ember state to spend Phoenix tokens
    /// 3. Creates ATA for USDC if needed (idempotent)
    /// 4. Withdraws Phoenix tokens from Phoenix protocol
    /// 5. Converts Phoenix tokens to USDC via Ember
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `trader_pda` - The trader's PDA account
    /// * `usdc_amount` - Amount of USDC to withdraw (e.g., 100.0 for $100)
    ///
    /// # Returns
    ///
    /// A vector containing 5 instructions that should be sent in a single
    /// transaction.
    pub fn build_withdraw_funds(
        &self,
        authority: Pubkey,
        trader_pda: Pubkey,
        usdc_amount: f64,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        // Convert USDC amount to base units (6 decimals)
        let amount = (usdc_amount * 1_000_000.0) as u64;

        // Get exchange keys from metadata
        let keys = self.metadata.keys();
        let canonical_mint = Pubkey::from_str(&keys.canonical_mint)?;
        let global_vault = Pubkey::from_str(&keys.global_vault)?;
        let perp_asset_map = Pubkey::from_str(&keys.perp_asset_map)?;
        let withdraw_queue = Pubkey::from_str(&keys.withdraw_queue)?;
        let global_trader_index = parse_pubkey_vec(&keys.global_trader_index)?;
        let active_trader_buffer = parse_pubkey_vec(&keys.active_trader_buffer)?;

        // Derive addresses
        let trader_usdc_ata = get_associated_token_address(&authority, &USDC_MINT);
        let trader_phoenix_ata = get_associated_token_address(&authority, &canonical_mint);

        // 1. Create Phoenix token ATA instruction (idempotent)
        let create_phoenix_ata_ix =
            create_associated_token_account_idempotent_ix(authority, authority, canonical_mint);

        // 2. SPL Token Approve instruction (delegate Ember state to spend Phoenix
        //    tokens)
        let approve_params = SplApproveParams::builder()
            .source(trader_phoenix_ata)
            .delegate(get_ember_state_address())
            .owner(authority)
            .amount(amount)
            .build()?;
        let approve_ix = create_spl_approve_ix(approve_params)?;

        // 3. Create USDC ATA instruction (idempotent)
        let create_usdc_ata_ix =
            create_associated_token_account_idempotent_ix(authority, authority, USDC_MINT);

        // 4. Withdraw funds instruction (Phoenix protocol -> Phoenix token ATA)
        let withdraw_params = WithdrawFundsParams::builder()
            .trader(authority)
            .trader_account(trader_pda)
            .perp_asset_map(perp_asset_map)
            .global_vault(global_vault)
            .trader_token_account(trader_phoenix_ata)
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .withdraw_queue(withdraw_queue)
            .amount(amount)
            .build()?;
        let withdraw_ix = create_withdraw_funds_ix(withdraw_params)?;

        // 5. Ember withdraw instruction (Phoenix tokens -> USDC)
        let ember_params = EmberWithdrawParams::builder()
            .trader(authority)
            .usdc_mint(USDC_MINT)
            .canonical_mint(canonical_mint)
            .trader_usdc_account(trader_usdc_ata)
            .trader_phoenix_account(trader_phoenix_ata)
            .amount(Some(amount))
            .build()?;
        let ember_ix = create_ember_withdraw_ix(ember_params)?;

        Ok(vec![
            create_phoenix_ata_ix.into(),
            approve_ix.into(),
            create_usdc_ata_ix.into(),
            withdraw_ix.into(),
            ember_ix.into(),
        ])
    }

    /// Build a register trader instruction.
    ///
    /// Registers a new trader account. The authority pays for account creation.
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (also pays for account
    ///   creation)
    /// * `pda_index` - The PDA index for trader derivation
    /// * `subaccount_index` - 0 for cross-margin, 1-100 for isolated margin
    ///
    /// # Returns
    ///
    /// A vector containing the register trader instruction.
    pub fn build_register_trader(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let max_positions: u64 = if subaccount_index == CROSS_MARGIN_SUBACCOUNT_IDX {
            128
        } else {
            1
        };
        let trader_pda = Self::trader_pda(&authority, pda_index, subaccount_index);

        let params = RegisterTraderParams::builder()
            .payer(authority)
            .trader(authority)
            .trader_account(trader_pda)
            .max_positions(max_positions)
            .trader_pda_index(pda_index)
            .subaccount_index(subaccount_index)
            .build()?;
        let ix = create_register_trader_ix(params)?;

        Ok(vec![ix.into()])
    }

    /// Build a Hawkeye `view_margin` instruction for a derived trader account.
    pub fn build_hawkeye_view_margin(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let trader = Self::trader_pda(&authority, pda_index, subaccount_index);
        self.build_hawkeye_view_margin_for_trader(trader)
    }

    /// Build a Hawkeye `view_margin` instruction for an explicit trader
    /// account.
    pub fn build_hawkeye_view_margin_for_trader(
        &self,
        trader: Pubkey,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        Ok(vec![
            create_hawkeye_view_margin_ix(self.hawkeye_trader_accounts(trader)?).into(),
        ])
    }

    /// Build a Hawkeye `view_margin_for_asset` instruction for a derived trader
    /// account.
    pub fn build_hawkeye_view_margin_for_asset(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
        asset_id: u32,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let trader = Self::trader_pda(&authority, pda_index, subaccount_index);
        self.build_hawkeye_view_margin_for_asset_for_trader(trader, asset_id)
    }

    /// Build a Hawkeye `view_margin_for_asset` instruction for an explicit
    /// trader account.
    pub fn build_hawkeye_view_margin_for_asset_for_trader(
        &self,
        trader: Pubkey,
        asset_id: u32,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        Ok(vec![
            create_hawkeye_view_margin_for_asset_ix(
                self.hawkeye_trader_accounts(trader)?,
                asset_id,
            )
            .into(),
        ])
    }

    /// Build a Hawkeye `view_liquidation_price` instruction for a derived
    /// trader account.
    pub fn build_hawkeye_view_liquidation_price(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
        asset_id: u32,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let trader = Self::trader_pda(&authority, pda_index, subaccount_index);
        self.build_hawkeye_view_liquidation_price_for_trader(trader, asset_id)
    }

    /// Build a Hawkeye `view_liquidation_price` instruction for an explicit
    /// trader account.
    pub fn build_hawkeye_view_liquidation_price_for_trader(
        &self,
        trader: Pubkey,
        asset_id: u32,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        Ok(vec![
            create_hawkeye_view_liquidation_price_ix(
                self.hawkeye_trader_accounts(trader)?,
                asset_id,
            )
            .into(),
        ])
    }

    /// Build a Hawkeye `view_funding` instruction for a derived trader account
    /// and asset.
    pub fn build_hawkeye_view_funding(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
        asset_id: u32,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let trader = Self::trader_pda(&authority, pda_index, subaccount_index);
        self.build_hawkeye_view_funding_for_trader(trader, asset_id)
    }

    /// Build a Hawkeye `view_funding` instruction for an explicit trader
    /// account and asset.
    pub fn build_hawkeye_view_funding_for_trader(
        &self,
        trader: Pubkey,
        asset_id: u32,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        Ok(vec![
            create_hawkeye_view_funding_ix(self.hawkeye_trader_accounts(trader)?, asset_id).into(),
        ])
    }

    /// Build a Hawkeye `view_bbo` instruction for a market symbol.
    pub fn build_hawkeye_view_bbo(
        &self,
        symbol: &str,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        Ok(vec![
            create_hawkeye_view_bbo_ix(self.hawkeye_bbo_accounts(symbol)?).into(),
        ])
    }

    /// Build a transfer collateral instruction.
    ///
    /// Transfers collateral between two subaccounts (e.g., from cross-margin
    /// subaccount 0 to an isolated margin subaccount).
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `src_trader_pda` - The source trader PDA account
    /// * `dst_trader_pda` - The destination trader PDA account
    /// * `usdc_amount` - Amount of USDC to transfer (e.g., 100.0 for $100)
    ///
    /// # Returns
    ///
    /// A vector containing the transfer collateral instruction.
    pub fn build_transfer_collateral(
        &self,
        authority: Pubkey,
        src_trader_pda: Pubkey,
        dst_trader_pda: Pubkey,
        usdc_amount: f64,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let amount = (usdc_amount * 1_000_000.0) as u64;

        let keys = self.metadata.keys();
        let perp_asset_map = Pubkey::from_str(&keys.perp_asset_map)?;
        let global_trader_index = parse_pubkey_vec(&keys.global_trader_index)?;
        let active_trader_buffer = parse_pubkey_vec(&keys.active_trader_buffer)?;

        let params = TransferCollateralParams::builder()
            .trader(authority)
            .src_trader_account(src_trader_pda)
            .dst_trader_account(dst_trader_pda)
            .perp_asset_map(perp_asset_map)
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .amount(amount)
            .build()?;

        let ix = create_transfer_collateral_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Build a transfer collateral child-to-parent instruction.
    ///
    /// Transfers **all** collateral from a child subaccount back to the parent
    /// (subaccount 0). No-ops on-chain if the child has open positions, open
    /// orders, or zero collateral.
    ///
    /// # Arguments
    ///
    /// * `authority` - The trader's wallet address (signer)
    /// * `child_trader_pda` - The child trader PDA account
    /// * `parent_trader_pda` - The parent trader PDA account
    pub fn build_transfer_collateral_child_to_parent(
        &self,
        authority: Pubkey,
        child_trader_pda: Pubkey,
        parent_trader_pda: Pubkey,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let keys = self.metadata.keys();
        let perp_asset_map = Pubkey::from_str(&keys.perp_asset_map)?;
        let global_trader_index = parse_pubkey_vec(&keys.global_trader_index)?;
        let active_trader_buffer = parse_pubkey_vec(&keys.active_trader_buffer)?;

        let params = TransferCollateralChildToParentParams::builder()
            .trader(authority)
            .child_trader_account(child_trader_pda)
            .parent_trader_account(parent_trader_pda)
            .perp_asset_map(perp_asset_map)
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .build()?;

        let ix = create_transfer_collateral_child_to_parent_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Build a sync parent-to-child instruction.
    ///
    /// Syncs a parent trader account's state to a child (isolated) subaccount,
    /// including global trader index updates.
    ///
    /// # Arguments
    ///
    /// * `trader_wallet` - The trader wallet authority
    /// * `parent_trader_pda` - The parent trader PDA (subaccount 0)
    /// * `child_trader_pda` - The child trader PDA (subaccount > 0)
    pub fn build_sync_parent_to_child(
        &self,
        trader_wallet: Pubkey,
        parent_trader_pda: Pubkey,
        child_trader_pda: Pubkey,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let keys = self.metadata.keys();
        let global_trader_index = parse_pubkey_vec(&keys.global_trader_index)?;

        let params = SyncParentToChildParams::builder()
            .trader_wallet(trader_wallet)
            .parent_trader_account(parent_trader_pda)
            .child_trader_account(child_trader_pda)
            .global_trader_index(global_trader_index)
            .build()?;

        let ix = create_sync_parent_to_child_ix(params)?;
        Ok(vec![ix.into()])
    }

    /// Register a new isolated subaccount and sync parent capabilities to it.
    fn register_and_sync_subaccount(
        &self,
        parent_key: &TraderKey,
        child_key: &TraderKey,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let mut ixs = self.build_register_trader(
            child_key.authority(),
            child_key.pda_index,
            child_key.subaccount_index,
        )?;
        ixs.extend(self.build_sync_parent_to_child(
            child_key.authority(),
            parent_key.pda(),
            child_key.pda(),
        )?);
        Ok(ixs)
    }

    /// Resolve the isolated subaccount key, optionally registering it, then
    /// apply collateral flow instructions. Returns the subaccount key and
    /// accumulated instructions.
    fn prepare_isolated_subaccount(
        &self,
        trader: &Trader,
        symbol: &str,
        allow_cross_and_isolated: bool,
        collateral: &Option<IsolatedCollateralFlow>,
    ) -> Result<(TraderKey, Vec<Instruction>), PhoenixTxBuilderError> {
        if !allow_cross_and_isolated {
            if let Some(primary) = trader.primary_subaccount() {
                if primary.positions.contains_key(symbol) {
                    return Err(PhoenixTxBuilderError::CrossMarginPositionExists(
                        symbol.to_string(),
                    ));
                }
            }
        }

        let mut ixs = Vec::new();

        let sub_key = trader
            .get_or_create_isolated_subaccount_key(symbol)
            .ok_or(PhoenixTxBuilderError::NoAvailableSubaccount)?;

        if !trader.subaccount_exists(sub_key.subaccount_index) {
            ixs.extend(self.register_and_sync_subaccount(&trader.key, &sub_key)?);
        }

        match collateral {
            Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }) => {
                let existing = trader
                    .get_collateral_for_subaccount(sub_key.subaccount_index)
                    .as_inner()
                    .max(0) as u64;
                if *collateral > existing {
                    let transfer_amount = *collateral - existing;

                    let parent_collateral = trader
                        .primary_subaccount()
                        .map(|s| s.collateral.as_inner().max(0) as u64)
                        .unwrap_or(0);

                    if parent_collateral < transfer_amount {
                        return Err(PhoenixTxBuilderError::InsufficientParentCollateral {
                            need: transfer_amount,
                            have: parent_collateral,
                        });
                    }

                    let usdc_amount = transfer_amount as f64 / USDC_NATIVE_DECIMALS;
                    ixs.extend(self.build_transfer_collateral(
                        sub_key.authority(),
                        trader.key.pda(),
                        sub_key.pda(),
                        usdc_amount,
                    )?);
                }
            }
            Some(IsolatedCollateralFlow::Deposit { usdc_amount }) => {
                let usdc = *usdc_amount as f64 / USDC_NATIVE_DECIMALS;
                ixs.extend(self.build_deposit_funds(sub_key.authority(), sub_key.pda(), usdc)?);
            }
            None => {}
        }

        Ok((sub_key, ixs))
    }

    /// Build an isolated margin market order (convenience method).
    ///
    /// Encapsulates the full isolated margin trading flow:
    /// 1. Selects (or registers) an isolated subaccount for the asset
    /// 2. Funds the subaccount based on `collateral`
    /// 3. Places the market order
    /// 4. Optionally places bracket leg (SL/TP) orders
    /// 5. Sweeps remaining collateral back to parent if subaccount existed
    ///
    /// # Returns
    ///
    /// 1+ instructions depending on subaccount state.
    pub fn build_isolated_market_order(
        &self,
        trader: &Trader,
        symbol: &str,
        side: Side,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let params = IsolatedMarketOrderParams {
            side,
            price_in_ticks: None,
            num_base_lots,
            num_quote_lots: None,
            min_base_lots_to_fill: 0,
            min_quote_lots_to_fill: 0,
            self_trade_behavior: crate::phoenix_rise_ix::SelfTradeBehavior::Abort,
            match_limit: None,
            client_order_id: 0,
            last_valid_slot: None,
            order_flags: crate::phoenix_rise_ix::OrderFlags::None,
            cancel_existing: false,
            allow_cross_and_isolated,
            collateral,
        };
        self.build_isolated_market_order_with_params(trader, symbol, params, bracket)
    }

    /// Build an isolated margin market order with pre-built params.
    ///
    /// Same flow as `build_isolated_market_order` but accepts full
    /// `IsolatedMarketOrderParams` for advanced configuration.
    pub fn build_isolated_market_order_with_params(
        &self,
        trader: &Trader,
        symbol: &str,
        params: IsolatedMarketOrderParams,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let (sub_key, mut ixs) = self.prepare_isolated_subaccount(
            trader,
            symbol,
            params.allow_cross_and_isolated,
            &params.collateral,
        )?;

        let side = params.side;

        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let addrs = self.parse_addresses(market)?;

        let mut builder = MarketOrderParams::builder()
            .trader(sub_key.authority())
            .trader_account(sub_key.pda())
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index)
            .active_trader_buffer(addrs.active_trader_buffer)
            .side(params.side)
            .num_base_lots(params.num_base_lots)
            .symbol(symbol)
            .subaccount_index(sub_key.subaccount_index)
            .self_trade_behavior(params.self_trade_behavior)
            .order_flags(params.order_flags)
            .cancel_existing(params.cancel_existing)
            .client_order_id(params.client_order_id)
            .min_base_lots_to_fill(params.min_base_lots_to_fill)
            .min_quote_lots_to_fill(params.min_quote_lots_to_fill);

        if let Some(v) = params.price_in_ticks {
            builder = builder.price_in_ticks(v);
        }
        if let Some(v) = params.num_quote_lots {
            builder = builder.num_quote_lots(v);
        }
        if let Some(v) = params.match_limit {
            builder = builder.match_limit(v);
        }
        if let Some(v) = params.last_valid_slot {
            builder = builder.last_valid_slot(v);
        }

        ixs.extend(self.create_market_order_ixs(builder.build()?)?);

        // Bracket legs before child-to-parent sweep
        if let Some(bracket) = bracket {
            ixs.extend(self.build_bracket_leg_orders(
                sub_key.authority(),
                sub_key.pda(),
                symbol,
                side,
                bracket,
            )?);
        }

        if trader.subaccount_exists(sub_key.subaccount_index) {
            ixs.extend(self.build_transfer_collateral_child_to_parent(
                sub_key.authority(),
                sub_key.pda(),
                trader.key.pda(),
            )?);
        }

        Ok(ixs)
    }

    /// Build an isolated margin limit order (convenience method).
    ///
    /// Same flow as `build_isolated_market_order` but places a limit order.
    /// Takes `price` as a USD float and converts to ticks internally.
    pub fn build_isolated_limit_order(
        &self,
        trader: &Trader,
        symbol: &str,
        side: Side,
        price: f64,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let calc = self
            .metadata
            .get_market_calculator(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        let price_in_ticks = calc.price_to_ticks(price)?.as_inner();

        let params = IsolatedLimitOrderParams {
            side,
            price_in_ticks,
            num_base_lots,
            self_trade_behavior: crate::phoenix_rise_ix::SelfTradeBehavior::Abort,
            match_limit: None,
            client_order_id: 0,
            last_valid_slot: None,
            order_flags: crate::phoenix_rise_ix::OrderFlags::None,
            cancel_existing: false,
            allow_cross_and_isolated,
            collateral,
        };
        self.build_isolated_limit_order_with_params(trader, symbol, params)
    }

    /// Build an isolated margin limit order with pre-built params.
    ///
    /// Same flow as `build_isolated_limit_order` but accepts full
    /// `IsolatedLimitOrderParams` for advanced configuration.
    pub fn build_isolated_limit_order_with_params(
        &self,
        trader: &Trader,
        symbol: &str,
        params: IsolatedLimitOrderParams,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let (sub_key, mut ixs) = self.prepare_isolated_subaccount(
            trader,
            symbol,
            params.allow_cross_and_isolated,
            &params.collateral,
        )?;

        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let addrs = self.parse_addresses(market)?;

        let mut builder = LimitOrderParams::builder()
            .trader(sub_key.authority())
            .trader_account(sub_key.pda())
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index)
            .active_trader_buffer(addrs.active_trader_buffer)
            .side(params.side)
            .price_in_ticks(params.price_in_ticks)
            .num_base_lots(params.num_base_lots)
            .symbol(symbol)
            .subaccount_index(sub_key.subaccount_index)
            .self_trade_behavior(params.self_trade_behavior)
            .order_flags(params.order_flags)
            .cancel_existing(params.cancel_existing)
            .client_order_id(params.client_order_id);

        if let Some(v) = params.match_limit {
            builder = builder.match_limit(v);
        }
        if let Some(v) = params.last_valid_slot {
            builder = builder.last_valid_slot(v);
        }

        ixs.extend(self.create_limit_order_ixs(builder.build()?)?);

        if trader.subaccount_exists(sub_key.subaccount_index) {
            ixs.extend(self.build_transfer_collateral_child_to_parent(
                sub_key.authority(),
                sub_key.pda(),
                trader.key.pda(),
            )?);
        }

        Ok(ixs)
    }

    /// Build stop-loss and/or take-profit bracket leg instructions.
    ///
    /// Bracket legs are represented with a single position conditional order.
    /// Direction logic:
    /// - Primary Bid (long): SL triggers LessThan, TP triggers GreaterThan,
    ///   bracket trade side = Ask
    /// - Primary Ask (short): SL triggers GreaterThan, TP triggers LessThan,
    ///   bracket trade side = Bid
    pub fn build_bracket_leg_orders(
        &self,
        authority: Pubkey,
        trader_account: Pubkey,
        symbol: &str,
        primary_side: Side,
        bracket: &BracketLegOrders,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let addrs = self.parse_addresses(market)?;
        let asset_id = market.asset_id as u32;
        let resolved_legs = self.build_resolved_bracket_legs(symbol, primary_side, bracket)?;

        if resolved_legs.is_empty() {
            return Ok(Vec::new());
        }

        if resolved_legs.len() == 2 && resolved_legs[0].size == resolved_legs[1].size {
            let (greater_trigger_order, less_trigger_order) =
                self.bracket_trigger_pair(resolved_legs.iter().copied());
            return Ok(vec![self.create_position_conditional_order_ix(
                authority,
                trader_account,
                &addrs,
                asset_id,
                greater_trigger_order,
                less_trigger_order,
                resolved_legs[0].size,
            )?]);
        }

        let mut ixs = Vec::with_capacity(resolved_legs.len());
        for leg in resolved_legs {
            let (greater_trigger_order, less_trigger_order) =
                self.bracket_trigger_pair(std::iter::once(leg));
            ixs.push(self.create_position_conditional_order_ix(
                authority,
                trader_account,
                &addrs,
                asset_id,
                greater_trigger_order,
                less_trigger_order,
                leg.size,
            )?);
        }
        Ok(ixs)
    }

    fn build_bracket_trigger_orders(
        &self,
        symbol: &str,
        primary_side: Side,
        bracket: &BracketLegOrders,
    ) -> Result<(Option<TriggerOrderParams>, Option<TriggerOrderParams>), PhoenixTxBuilderError>
    {
        let resolved_legs = self.build_resolved_bracket_legs(symbol, primary_side, bracket)?;
        Ok(self.bracket_trigger_pair(resolved_legs.into_iter()))
    }

    fn build_resolved_bracket_legs(
        &self,
        symbol: &str,
        primary_side: Side,
        bracket: &BracketLegOrders,
    ) -> Result<Vec<ResolvedBracketLeg>, PhoenixTxBuilderError> {
        let calc = self
            .metadata
            .get_market_calculator(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        let (bracket_trade_side, sl_direction, tp_direction) = match primary_side {
            Side::Bid => (Side::Ask, Direction::LessThan, Direction::GreaterThan),
            Side::Ask => (Side::Bid, Direction::GreaterThan, Direction::LessThan),
        };

        let mut resolved_legs = Vec::new();

        if let Some(stop_loss) = &bracket.stop_loss {
            let trigger_ticks = calc.price_to_ticks(stop_loss.price)?.as_inner();
            let order_kind = stop_loss.resolved_order_kind(StopLossOrderKind::IOC);
            let execution_ticks = bracket_leg_execution_ticks(
                &calc,
                stop_loss,
                trigger_ticks,
                bracket_trade_side,
                order_kind,
            )?;
            resolved_legs.push(ResolvedBracketLeg {
                size: stop_loss.resolved_size(),
                trigger: TriggerOrderParams::new(
                    sl_direction,
                    bracket_trade_side,
                    order_kind,
                    trigger_ticks,
                    execution_ticks,
                ),
            });
        }

        if let Some(take_profit) = &bracket.take_profit {
            let trigger_ticks = calc.price_to_ticks(take_profit.price)?.as_inner();
            let order_kind = take_profit.resolved_order_kind(StopLossOrderKind::Limit);
            let execution_ticks = bracket_leg_execution_ticks(
                &calc,
                take_profit,
                trigger_ticks,
                bracket_trade_side,
                order_kind,
            )?;
            resolved_legs.push(ResolvedBracketLeg {
                size: take_profit.resolved_size(),
                trigger: TriggerOrderParams::new(
                    tp_direction,
                    bracket_trade_side,
                    order_kind,
                    trigger_ticks,
                    execution_ticks,
                ),
            });
        }

        Ok(resolved_legs)
    }

    fn bracket_trigger_pair(
        &self,
        resolved_legs: impl IntoIterator<Item = ResolvedBracketLeg>,
    ) -> (Option<TriggerOrderParams>, Option<TriggerOrderParams>) {
        let mut greater_trigger_order = None;
        let mut less_trigger_order = None;

        for leg in resolved_legs {
            match leg.trigger.trigger_direction() {
                Direction::GreaterThan => greater_trigger_order = Some(leg.trigger),
                Direction::LessThan => less_trigger_order = Some(leg.trigger),
            }
        }

        (greater_trigger_order, less_trigger_order)
    }

    fn create_position_conditional_order_ix(
        &self,
        authority: Pubkey,
        trader_account: Pubkey,
        addrs: &ParsedAddresses,
        asset_id: u32,
        greater_trigger_order: Option<TriggerOrderParams>,
        less_trigger_order: Option<TriggerOrderParams>,
        size: BracketLegSize,
    ) -> Result<Instruction, PhoenixTxBuilderError> {
        let mut builder = PlacePositionConditionalOrderParams::builder()
            .payer(authority)
            .trader_account(trader_account)
            .position_authority(authority)
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index.clone())
            .active_trader_buffer(addrs.active_trader_buffer.clone())
            .trader_conditional_orders(get_conditional_orders_address(&trader_account))
            .asset_id(asset_id);

        match size {
            BracketLegSize::PositionPercent(percent) => {
                builder = builder.size_percent(percent);
            }
            BracketLegSize::BaseLots(base_lots) => {
                builder = builder.size_base_lots(base_lots);
            }
        }

        if let Some(trigger) = greater_trigger_order {
            builder = builder.greater_trigger_order(trigger);
        }
        if let Some(trigger) = less_trigger_order {
            builder = builder.less_trigger_order(trigger);
        }

        Ok(create_place_position_conditional_order_ix(builder.build()?)?.into())
    }

    fn create_limit_order_with_conditionals_ix(
        &self,
        params: &LimitOrderParams,
        bracket: &BracketLegOrders,
    ) -> Result<Instruction, PhoenixTxBuilderError> {
        let (greater_trigger_order, less_trigger_order) =
            self.build_bracket_trigger_orders(params.symbol(), params.side(), bracket)?;

        let mut builder = PlaceLimitOrderWithConditionalsParams::builder()
            .trader_wallet(params.trader())
            .trader_account(params.trader_account())
            .perp_asset_map(params.perp_asset_map())
            .orderbook(params.orderbook())
            .spline_collection(params.spline_collection())
            .global_trader_index(params.global_trader_index().to_vec())
            .active_trader_buffer(params.active_trader_buffer().to_vec())
            .payer(params.trader())
            .trader_conditional_orders(get_conditional_orders_address(&params.trader_account()))
            .order_packet(limit_order_packet(params));

        if let Some(trigger) = greater_trigger_order {
            builder = builder.greater_trigger_order(trigger);
        }
        if let Some(trigger) = less_trigger_order {
            builder = builder.less_trigger_order(trigger);
        }

        Ok(create_place_limit_order_with_conditionals_ix(builder.build()?)?.into())
    }

    async fn maybe_create_conditional_orders_account_ixs(
        &self,
        authority: Pubkey,
        trader_account: Pubkey,
        rpc: &RpcClient,
    ) -> Result<Vec<Instruction>, PhoenixTxBuilderError> {
        let conditional_orders = get_conditional_orders_address(&trader_account);
        let account = rpc
            .get_account_with_commitment(&conditional_orders, rpc.commitment())
            .await
            .map_err(|error| PhoenixTxBuilderError::Rpc(error.to_string()))?;

        if account.value.is_some() {
            return Ok(Vec::new());
        }

        self.build_create_conditional_orders_account(
            authority,
            authority,
            trader_account,
            DEFAULT_CONDITIONAL_ORDERS_CAPACITY,
        )
    }

    /// Return an error if `symbol` is an isolated-only market.
    fn reject_isolated_only(&self, symbol: &str) -> Result<(), PhoenixTxBuilderError> {
        if self.metadata.is_isolated_only(symbol) {
            return Err(PhoenixTxBuilderError::IsolatedOnlyMarket(
                symbol.to_ascii_uppercase(),
            ));
        }
        Ok(())
    }

    fn hawkeye_trader_accounts(
        &self,
        trader: Pubkey,
    ) -> Result<HawkeyeTraderViewAccounts, PhoenixTxBuilderError> {
        let keys = self.metadata.keys();
        Ok(HawkeyeTraderViewAccounts {
            phoenix_program_id: keys
                .program_id
                .as_deref()
                .map(Pubkey::from_str)
                .transpose()?
                .unwrap_or(*PHOENIX_PROGRAM_ID),
            global_config: Pubkey::from_str(&keys.global_config)?,
            global_trader_index: parse_pubkey_vec(&keys.global_trader_index)?,
            active_trader_buffer: parse_pubkey_vec(&keys.active_trader_buffer)?,
            perp_asset_map: Pubkey::from_str(&keys.perp_asset_map)?,
            trader,
        })
    }

    fn hawkeye_bbo_accounts(
        &self,
        symbol: &str,
    ) -> Result<HawkeyeBboViewAccounts, PhoenixTxBuilderError> {
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;
        let keys = self.metadata.keys();
        let addrs = self.parse_addresses(market)?;

        Ok(HawkeyeBboViewAccounts {
            phoenix_program_id: keys
                .program_id
                .as_deref()
                .map(Pubkey::from_str)
                .transpose()?
                .unwrap_or(*PHOENIX_PROGRAM_ID),
            global_config: Pubkey::from_str(&keys.global_config)?,
            global_trader_index: addrs.global_trader_index,
            active_trader_buffer: addrs.active_trader_buffer,
            perp_asset_map: addrs.perp_asset_map,
            orderbook: addrs.orderbook,
            spline_collection: addrs.spline_collection,
        })
    }

    /// Parse all required addresses from the exchange metadata for a given
    /// market.
    fn parse_addresses(
        &self,
        market: &ExchangeMarketConfig,
    ) -> Result<ParsedAddresses, PhoenixTxBuilderError> {
        let keys = self.metadata.keys();
        let perp_asset_map = Pubkey::from_str(&keys.perp_asset_map)?;
        let global_trader_index = parse_pubkey_vec(&keys.global_trader_index)?;
        let active_trader_buffer = parse_pubkey_vec(&keys.active_trader_buffer)?;
        let orderbook = Pubkey::from_str(&market.market_pubkey)?;
        let spline_collection = Pubkey::from_str(&market.spline_pubkey)?;

        Ok(ParsedAddresses {
            perp_asset_map,
            global_trader_index,
            active_trader_buffer,
            orderbook,
            spline_collection,
        })
    }
}

fn bracket_leg_execution_ticks(
    calc: &crate::phoenix_rise_math::MarketCalculator,
    leg: &BracketLeg,
    trigger_ticks: u64,
    trade_side: Side,
    order_kind: StopLossOrderKind,
) -> Result<u64, PhoenixTxBuilderError> {
    match leg.execution {
        Some(BracketLegExecution::Price(price)) => Ok(calc.price_to_ticks(price)?.as_inner()),
        Some(BracketLegExecution::SlippageBps(slippage_bps)) => {
            execution_ticks_with_slippage(trigger_ticks, trade_side, slippage_bps)
        }
        None if order_kind == StopLossOrderKind::Limit => Ok(trigger_ticks),
        None => execution_ticks_with_slippage(
            trigger_ticks,
            trade_side,
            DEFAULT_BRACKET_LEG_SLIPPAGE_BPS,
        ),
    }
}

fn execution_ticks_with_slippage(
    trigger_ticks: u64,
    trade_side: Side,
    slippage_bps: u32,
) -> Result<u64, PhoenixTxBuilderError> {
    match trade_side {
        Side::Ask => sell_execution_ticks(trigger_ticks, slippage_bps),
        Side::Bid => buy_execution_ticks(trigger_ticks, slippage_bps),
    }
}

fn sell_execution_ticks(
    trigger_ticks: u64,
    slippage_bps: u32,
) -> Result<u64, PhoenixTxBuilderError> {
    if slippage_bps >= 10_000 {
        return Err(PhoenixTxBuilderError::InvalidBracketLegExecutionPrice(
            "sell-side slippage must be less than 10000 bps".to_string(),
        ));
    }

    let numerator = (trigger_ticks as u128)
        .checked_mul(BPS_DENOMINATOR - slippage_bps as u128)
        .ok_or_else(slippage_overflow_error)?;
    let mut ticks = (numerator / BPS_DENOMINATOR) as u64;

    if slippage_bps > 0 && ticks == trigger_ticks {
        ticks = trigger_ticks.checked_sub(1).ok_or_else(|| {
            PhoenixTxBuilderError::InvalidBracketLegExecutionPrice(
                "sell-side slippage resolves execution price to 0 ticks".to_string(),
            )
        })?;
    }

    if ticks == 0 {
        return Err(PhoenixTxBuilderError::InvalidBracketLegExecutionPrice(
            "sell-side slippage resolves execution price to 0 ticks".to_string(),
        ));
    }

    Ok(ticks)
}

fn buy_execution_ticks(
    trigger_ticks: u64,
    slippage_bps: u32,
) -> Result<u64, PhoenixTxBuilderError> {
    let numerator = (trigger_ticks as u128)
        .checked_mul(BPS_DENOMINATOR + slippage_bps as u128)
        .ok_or_else(slippage_overflow_error)?;
    let ticks = numerator
        .checked_add(BPS_DENOMINATOR - 1)
        .ok_or_else(slippage_overflow_error)?
        / BPS_DENOMINATOR;

    if ticks > u64::MAX as u128 {
        return Err(slippage_overflow_error());
    }

    let mut ticks = ticks as u64;
    if slippage_bps > 0 && ticks == trigger_ticks {
        ticks = trigger_ticks
            .checked_add(1)
            .ok_or_else(slippage_overflow_error)?;
    }

    Ok(ticks)
}

fn slippage_overflow_error() -> PhoenixTxBuilderError {
    PhoenixTxBuilderError::InvalidBracketLegExecutionPrice(
        "slippage overflows execution ticks".to_string(),
    )
}

/// Parse a vector of base58-encoded pubkeys.
fn parse_pubkey_vec(strings: &[String]) -> Result<Vec<Pubkey>, PhoenixTxBuilderError> {
    strings
        .iter()
        .map(|s| Pubkey::from_str(s).map_err(PhoenixTxBuilderError::from))
        .collect()
}

fn limit_order_packet(params: &LimitOrderParams) -> OrderPacket {
    OrderPacket::limit(
        params.side(),
        params.price_in_ticks(),
        params.num_base_lots(),
        params.self_trade_behavior(),
        params.match_limit(),
        client_order_id_to_bytes(params.client_order_id()),
        params.last_valid_slot(),
        params.order_flags(),
        params.cancel_existing(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::order_tickets::BracketLeg;
    use crate::phoenix_rise_ix::{
        OrderFlags, PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, get_ember_vault_address,
        place_market_order_delegated_discriminant,
    };
    use crate::phoenix_rise_math::{MarketCalculator, QuoteLotsPerBaseLotPerTick};
    use crate::phoenix_rise_types::{
        AuthoritySetView, ExchangeKeysView, ExchangeRiskFactors, ExchangeView, MarketStatus,
    };

    fn mock_exchange_keys() -> ExchangeKeysView {
        let authorities = || AuthoritySetView {
            root_authority: Pubkey::new_unique().to_string(),
            risk_authority: Pubkey::new_unique().to_string(),
            market_authority: Pubkey::new_unique().to_string(),
            oracle_authority: Pubkey::new_unique().to_string(),
        };

        ExchangeKeysView {
            program_id: None,
            global_config: Pubkey::new_unique().to_string(),
            current_authorities: authorities(),
            pending_authorities: authorities(),
            canonical_mint: Pubkey::new_unique().to_string(),
            global_vault: Pubkey::new_unique().to_string(),
            perp_asset_map: Pubkey::new_unique().to_string(),
            global_trader_index: vec![Pubkey::new_unique().to_string()],
            active_trader_buffer: vec![Pubkey::new_unique().to_string()],
            withdraw_queue: Pubkey::new_unique().to_string(),
        }
    }

    fn mock_market(symbol: &str) -> ExchangeMarketConfig {
        ExchangeMarketConfig {
            symbol: symbol.to_string(),
            asset_id: 1,
            market_status: MarketStatus::Active,
            metadata: None,
            market_pubkey: Pubkey::new_unique().to_string(),
            spline_pubkey: Pubkey::new_unique().to_string(),
            tick_size: 1_000_000,
            base_lots_decimals: 0,
            taker_fee: 0.0,
            maker_fee: 0.0,
            leverage_tiers: Vec::new(),
            risk_factors: ExchangeRiskFactors::default(),
            funding_interval_seconds: 1,
            funding_period_seconds: 1,
            max_funding_rate_per_interval: 0.0,
            open_interest_cap_base_lots: 0u64.into(),
            max_liquidation_size_base_lots: 0u64.into(),
            isolated_only: false,
            stats_snapshot: None,
        }
    }

    fn mock_metadata(symbol: &str) -> PhoenixMetadata {
        mock_metadata_with_keys(symbol, mock_exchange_keys())
    }

    fn mock_metadata_with_keys(symbol: &str, keys: ExchangeKeysView) -> PhoenixMetadata {
        let market = mock_market(symbol);
        let mut markets = HashMap::new();
        markets.insert(symbol.to_string(), market);
        PhoenixMetadata::new(ExchangeView { keys, markets })
    }

    #[test]
    fn test_market_order_ticket_defaults_to_expected_params() {
        let keys = mock_exchange_keys();
        let market = mock_market("SOL");
        let calc = MarketCalculator::new(
            market.base_lots_decimals,
            QuoteLotsPerBaseLotPerTick::new(market.tick_size),
        );
        let metadata = OrderTicketMetadata {
            market_calc: &calc,
            market_config: &market,
            exchange_keys: &keys,
        };

        let authority = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let ticket = MarketOrderTicket::builder()
            .authority(authority)
            .trader_account(trader_account)
            .symbol("SOL")
            .side(Side::Bid)
            .num_base_lots(25)
            .build()
            .unwrap();

        let params = ticket.to_params(metadata).unwrap();
        assert_eq!(params.trader(), authority);
        assert_eq!(params.trader_account(), trader_account);
        assert_eq!(params.side(), Side::Bid);
        assert_eq!(params.num_base_lots(), 25);
        assert_eq!(params.price_in_ticks(), None);
        assert_eq!(params.order_flags(), OrderFlags::None);
        assert_eq!(params.subaccount_index(), CROSS_MARGIN_SUBACCOUNT_IDX);
    }

    #[test]
    fn test_place_market_order_delegated_builds_distinct_ix() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let authority = Pubkey::new_unique();
        let trader_wallet = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let ticket = MarketOrderTicket::builder()
            .authority(authority)
            .trader_account(trader_account)
            .symbol("SOL")
            .side(Side::Bid)
            .num_base_lots(25)
            .build()
            .unwrap();

        let ix = builder
            .place_market_order_delegated(ticket, trader_wallet, None)
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(&ix.data[..8], &place_market_order_delegated_discriminant());
        assert_eq!(ix.accounts.len(), 11);
        assert_eq!(ix.accounts[3].pubkey, trader_wallet);
        assert!(ix.accounts[3].is_signer);
        assert_eq!(ix.accounts[4].pubkey, trader_wallet);
        assert_eq!(ix.accounts[5].pubkey, trader_account);
        assert_ne!(ix.accounts[3].pubkey, authority);
    }

    #[test]
    fn test_limit_order_ticket_converts_price_to_ticks() {
        let keys = mock_exchange_keys();
        let market = mock_market("SOL");
        let calc = MarketCalculator::new(
            market.base_lots_decimals,
            QuoteLotsPerBaseLotPerTick::new(market.tick_size),
        );
        let metadata = OrderTicketMetadata {
            market_calc: &calc,
            market_config: &market,
            exchange_keys: &keys,
        };

        let ticket = LimitOrderTicket::builder()
            .authority(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .symbol("SOL")
            .side(Side::Ask)
            .price(150.0)
            .num_base_lots(10)
            .order_flags(OrderFlags::ReduceOnly)
            .build()
            .unwrap();

        let params = ticket.to_params(metadata).unwrap();
        assert_eq!(params.side(), Side::Ask);
        assert_eq!(params.price_in_ticks(), 150);
        assert_eq!(params.num_base_lots(), 10);
        assert_eq!(params.order_flags(), OrderFlags::ReduceOnly);
    }

    #[test]
    fn test_parse_pubkey_vec() {
        // Valid Solana pubkeys (32 bytes, base58 encoded)
        let pubkeys = vec![
            "11111111111111111111111111111112".to_string(), // System program
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(), // SPL Token
        ];
        let result = parse_pubkey_vec(&pubkeys).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_pubkey_vec_invalid() {
        let pubkeys = vec!["invalid".to_string()];
        let result = parse_pubkey_vec(&pubkeys);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_ignores_metadata_program_id_for_register_trader() {
        let mut keys = mock_exchange_keys();
        let program_id = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        keys.program_id = Some(program_id.to_string());
        keys.global_config = global_config.to_string();
        let metadata = mock_metadata_with_keys("SOL", keys);
        let builder = PhoenixTxBuilder::new(&metadata);
        let authority = Pubkey::new_unique();
        let pda_index = 7;
        let subaccount_index = 2;
        let expected_trader_pda = Pubkey::find_program_address(
            &[
                b"trader",
                authority.as_ref(),
                &[pda_index, subaccount_index],
            ],
            &*PHOENIX_PROGRAM_ID,
        )
        .0;

        let ix = builder
            .build_register_trader(authority, pda_index, subaccount_index)
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert_ne!(ix.accounts[2].pubkey, global_config);
        assert_eq!(ix.accounts[5].pubkey, expected_trader_pda);
    }

    #[test]
    fn test_builder_ignores_metadata_program_id_for_ember_accounts() {
        let mut keys = mock_exchange_keys();
        let program_id = Pubkey::new_unique();
        keys.program_id = Some(program_id.to_string());
        let metadata = mock_metadata_with_keys("SOL", keys);
        let builder = PhoenixTxBuilder::new(&metadata);
        let authority = Pubkey::new_unique();
        let trader_pda = Pubkey::new_unique();

        let ixs = builder
            .build_deposit_funds(authority, trader_pda, 1.0)
            .unwrap();

        assert_eq!(ixs[1].accounts[1].pubkey, get_ember_state_address());
        assert_eq!(ixs[1].accounts[6].pubkey, get_ember_vault_address());
        assert_eq!(ixs[2].program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ixs[2].accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert_ne!(ixs[2].program_id, program_id);
    }

    #[test]
    fn test_builder_ignores_metadata_program_id_for_legacy_stop_loss() {
        let mut keys = mock_exchange_keys();
        let program_id = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        keys.program_id = Some(program_id.to_string());
        keys.global_config = global_config.to_string();
        let metadata = mock_metadata_with_keys("SOL", keys);
        let builder = PhoenixTxBuilder::new(&metadata);
        let authority = Pubkey::new_unique();
        let trader_pda = Pubkey::new_unique();

        let ixs = builder
            .build_stop_loss_orders(
                authority,
                trader_pda,
                "SOL",
                Side::Bid,
                &BracketLegOrders {
                    stop_loss: Some(BracketLeg::new(100.0)),
                    take_profit: Some(BracketLeg::new(200.0)),
                },
            )
            .unwrap();

        assert_eq!(ixs.len(), 2);
        for ix in ixs {
            assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
            assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
            assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
            assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
            assert_ne!(ix.accounts[2].pubkey, global_config);
            assert_eq!(
                ix.accounts[11].pubkey,
                get_stop_loss_address(&trader_pda, 1)
            );
        }
    }

    #[test]
    fn test_builder_cancel_stop_loss_uses_explicit_funder() {
        let mut keys = mock_exchange_keys();
        let program_id = Pubkey::new_unique();
        keys.program_id = Some(program_id.to_string());
        let metadata = mock_metadata_with_keys("SOL", keys);
        let builder = PhoenixTxBuilder::new(&metadata);
        let funder = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let trader_pda = Pubkey::new_unique();

        let ix = builder
            .build_cancel_bracket_leg_with_funder(
                funder,
                authority,
                trader_pda,
                "SOL",
                Direction::LessThan,
            )
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_ne!(ix.program_id, program_id);
        assert_eq!(ix.accounts[3].pubkey, funder);
        assert!(!ix.accounts[3].is_signer);
        assert!(ix.accounts[3].is_writable);
        assert_eq!(ix.accounts[5].pubkey, authority);
        assert!(ix.accounts[5].is_signer);
        assert_eq!(ix.accounts[6].pubkey, get_stop_loss_address(&trader_pda, 1));
    }

    #[test]
    fn test_try_to_tp_sl_config_rejects_explicit_leg_sizes() {
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0).with_size(BracketLegSize::BaseLots(5))),
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let err = bracket.try_to_tp_sl_config().unwrap_err();
        assert!(err.contains("custom TP/SL leg sizing"));
    }

    #[test]
    fn test_try_to_tp_sl_config_for_side_derives_slippage_execution_price() {
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0).with_slippage_bps(100)),
            take_profit: None,
        };

        let config = bracket.try_to_tp_sl_config_for_side(Side::Bid).unwrap();

        assert_eq!(config.stop_loss_trigger_price, Some(120.0));
        assert!((config.stop_loss_execution_price.unwrap() - 118.8).abs() < f64::EPSILON);
        assert_eq!(config.order_kind.as_deref(), Some("ioc"));
    }

    #[test]
    fn test_try_to_tp_sl_config_for_side_defaults_to_ten_percent_slippage() {
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: None,
        };

        let config = bracket.try_to_tp_sl_config_for_side(Side::Bid).unwrap();

        assert_eq!(config.stop_loss_trigger_price, Some(120.0));
        assert_eq!(config.stop_loss_execution_price, Some(108.0));
        assert_eq!(config.order_kind.as_deref(), Some("ioc"));
    }

    #[test]
    fn test_try_to_tp_sl_config_defaults_take_profit_to_limit_trigger() {
        let bracket = BracketLegOrders {
            stop_loss: None,
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let config = bracket.try_to_tp_sl_config().unwrap();

        assert_eq!(config.take_profit_trigger_price, Some(150.0));
        assert_eq!(config.take_profit_execution_price, Some(150.0));
        assert_eq!(config.order_kind.as_deref(), Some("limit"));
    }

    #[test]
    fn test_try_to_tp_sl_config_rejects_default_slippage_without_side() {
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: None,
        };

        let err = bracket.try_to_tp_sl_config().unwrap_err();
        assert!(err.contains("primary order side"));
    }

    #[test]
    fn test_try_to_tp_sl_config_for_side_rejects_mixed_default_order_kinds() {
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let err = bracket.try_to_tp_sl_config_for_side(Side::Bid).unwrap_err();
        assert!(err.contains("mixed TP/SL order kinds"));
    }

    #[test]
    fn test_bracket_leg_execution_price_uses_explicit_price() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0).with_execution_price(118.0)),
            take_profit: None,
        };

        let (_, less_trigger_order) = builder
            .build_bracket_trigger_orders("SOL", Side::Bid, &bracket)
            .unwrap();
        let less_trigger_order = less_trigger_order.unwrap();

        assert_eq!(less_trigger_order.trigger_price(), 120);
        assert_eq!(less_trigger_order.execution_price(), 118);
    }

    #[test]
    fn test_bracket_leg_slippage_moves_execution_price_away_from_trigger() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0).with_slippage_bps(100)),
            take_profit: None,
        };

        let (_, long_stop_loss) = builder
            .build_bracket_trigger_orders("SOL", Side::Bid, &bracket)
            .unwrap();
        assert_eq!(long_stop_loss.unwrap().execution_price(), 118);

        let (short_stop_loss, _) = builder
            .build_bracket_trigger_orders("SOL", Side::Ask, &bracket)
            .unwrap();
        assert_eq!(short_stop_loss.unwrap().execution_price(), 122);
    }

    #[test]
    fn test_bracket_leg_defaults_to_ten_percent_slippage() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: None,
        };

        let (_, long_stop_loss) = builder
            .build_bracket_trigger_orders("SOL", Side::Bid, &bracket)
            .unwrap();
        let long_stop_loss = long_stop_loss.unwrap();
        assert_eq!(long_stop_loss.order_kind(), StopLossOrderKind::IOC);
        assert_eq!(long_stop_loss.execution_price(), 108);

        let (short_stop_loss, _) = builder
            .build_bracket_trigger_orders("SOL", Side::Ask, &bracket)
            .unwrap();
        let short_stop_loss = short_stop_loss.unwrap();
        assert_eq!(short_stop_loss.order_kind(), StopLossOrderKind::IOC);
        assert_eq!(short_stop_loss.execution_price(), 132);
    }

    #[test]
    fn test_take_profit_defaults_to_limit_at_trigger_price() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let long_bracket = BracketLegOrders {
            stop_loss: None,
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let (long_take_profit, _) = builder
            .build_bracket_trigger_orders("SOL", Side::Bid, &long_bracket)
            .unwrap();
        let long_take_profit = long_take_profit.unwrap();
        assert_eq!(long_take_profit.order_kind(), StopLossOrderKind::Limit);
        assert_eq!(long_take_profit.trade_side(), Side::Ask);
        assert_eq!(long_take_profit.trigger_price(), 150);
        assert_eq!(long_take_profit.execution_price(), 150);

        let short_bracket = BracketLegOrders {
            stop_loss: None,
            take_profit: Some(BracketLeg::new(90.0)),
        };
        let (_, short_take_profit) = builder
            .build_bracket_trigger_orders("SOL", Side::Ask, &short_bracket)
            .unwrap();
        let short_take_profit = short_take_profit.unwrap();
        assert_eq!(short_take_profit.order_kind(), StopLossOrderKind::Limit);
        assert_eq!(short_take_profit.trade_side(), Side::Bid);
        assert_eq!(short_take_profit.trigger_price(), 90);
        assert_eq!(short_take_profit.execution_price(), 90);
    }

    #[test]
    fn test_ioc_take_profit_defaults_to_ten_percent_slippage_when_requested() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let bracket = BracketLegOrders {
            stop_loss: None,
            take_profit: Some(BracketLeg::new(150.0).with_ioc_order()),
        };

        let (take_profit, _) = builder
            .build_bracket_trigger_orders("SOL", Side::Bid, &bracket)
            .unwrap();
        let take_profit = take_profit.unwrap();
        assert_eq!(take_profit.order_kind(), StopLossOrderKind::IOC);
        assert_eq!(take_profit.execution_price(), 135);
    }

    #[test]
    fn test_bracket_leg_slots_match_frontend_defaults() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let long_bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let (greater, less) = builder
            .build_bracket_trigger_orders("SOL", Side::Bid, &long_bracket)
            .unwrap();
        let greater = greater.unwrap();
        let less = less.unwrap();
        assert_eq!(greater.order_kind(), StopLossOrderKind::Limit);
        assert_eq!(greater.trigger_price(), 150);
        assert_eq!(greater.execution_price(), 150);
        assert_eq!(less.order_kind(), StopLossOrderKind::IOC);
        assert_eq!(less.trigger_price(), 120);
        assert_eq!(less.execution_price(), 108);

        let short_bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: Some(BracketLeg::new(90.0)),
        };
        let (greater, less) = builder
            .build_bracket_trigger_orders("SOL", Side::Ask, &short_bracket)
            .unwrap();
        let greater = greater.unwrap();
        let less = less.unwrap();
        assert_eq!(greater.order_kind(), StopLossOrderKind::IOC);
        assert_eq!(greater.trigger_price(), 120);
        assert_eq!(greater.execution_price(), 132);
        assert_eq!(less.order_kind(), StopLossOrderKind::Limit);
        assert_eq!(less.trigger_price(), 90);
        assert_eq!(less.execution_price(), 90);
    }

    #[test]
    fn test_build_bracket_leg_orders_combines_matching_sizes() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0)),
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let ixs = builder
            .build_bracket_leg_orders(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                "SOL",
                Side::Bid,
                &bracket,
            )
            .unwrap();

        assert_eq!(ixs.len(), 1);
    }

    #[test]
    fn test_build_bracket_leg_orders_splits_mismatched_sizes() {
        let metadata = mock_metadata("SOL");
        let builder = PhoenixTxBuilder::new(&metadata);
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0).with_size(BracketLegSize::BaseLots(5))),
            take_profit: Some(
                BracketLeg::new(150.0).with_size(BracketLegSize::PositionPercent(50)),
            ),
        };

        let ixs = builder
            .build_bracket_leg_orders(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                "SOL",
                Side::Bid,
                &bracket,
            )
            .unwrap();

        assert_eq!(ixs.len(), 2);
    }
}
