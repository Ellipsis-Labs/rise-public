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
    BracketLegOrders, BracketLegSize, BracketLegTicket, LimitOrderTicket, MarketOrderTicket,
    OrderTicketMetadata,
};
use crate::phoenix_rise_ix::{
    CancelId, CancelOrdersByIdParams, CancelStopLossParams, CondensedOrder,
    CreateConditionalOrdersAccountParams, DepositFundsParams, Direction, EmberDepositParams,
    EmberWithdrawParams, IsolatedCollateralFlow, IsolatedLimitOrderParams,
    IsolatedMarketOrderParams, LimitOrderParams, MarketOrderParams, MultiLimitOrderParams,
    OrderPacket, PlaceLimitOrderWithConditionalsParams, PlacePositionConditionalOrderParams,
    RegisterTraderParams, Side, SplApproveParams, StopLossOrderKind, SyncParentToChildParams,
    TransferCollateralChildToParentParams, TransferCollateralParams, TriggerOrderParams, USDC_MINT,
    WithdrawFundsParams, client_order_id_to_bytes, create_associated_token_account_idempotent_ix,
    create_cancel_orders_by_id_ix, create_cancel_stop_loss_ix,
    create_create_conditional_orders_account_ix, create_deposit_funds_ix, create_ember_deposit_ix,
    create_ember_withdraw_ix, create_place_limit_order_ix,
    create_place_limit_order_with_conditionals_ix, create_place_market_order_ix,
    create_place_multi_limit_order_ix, create_place_position_conditional_order_ix,
    create_register_trader_ix, create_spl_approve_ix, create_sync_parent_to_child_ix,
    create_transfer_collateral_child_to_parent_ix, create_transfer_collateral_ix,
    create_withdraw_funds_ix, get_associated_token_address, get_conditional_orders_address,
    get_ember_state_address,
};
use crate::phoenix_rise_math::{MathError, WrapperNum};
use crate::phoenix_rise_types::{
    CROSS_MARGIN_SUBACCOUNT_IDX, ExchangeMarketConfig, Trader, TraderKey,
};

const USDC_NATIVE_DECIMALS: f64 = 1_000_000.0;
const DEFAULT_CONDITIONAL_ORDERS_CAPACITY: u8 = 8;

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

    /// Attached limit-order conditionals do not yet support explicit per-leg
    /// sizing in the local tx builder.
    #[error(
        "Explicit TP/SL sizing for attached limit-order brackets is not supported yet; omit the \
         leg sizes or use position conditionals after fill"
    )]
    UnsupportedLimitBracketLegSizing,
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
    /// * `authority` - The trader's wallet address (signer / funder)
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
        let market = self
            .metadata
            .get_market(symbol)
            .ok_or_else(|| PhoenixTxBuilderError::UnknownSymbol(symbol.to_string()))?;

        let asset_id = market.asset_id as u64;

        let params = CancelStopLossParams::builder()
            .funder(authority)
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
        let trader_pda = crate::phoenix_rise_types::TraderKey::derive_pda(
            &authority,
            pda_index,
            subaccount_index,
        );

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
            let price_in_ticks = calc.price_to_ticks(stop_loss.price)?.as_inner();
            resolved_legs.push(ResolvedBracketLeg {
                size: stop_loss.resolved_size(),
                trigger: TriggerOrderParams::new(
                    sl_direction,
                    bracket_trade_side,
                    StopLossOrderKind::IOC,
                    price_in_ticks,
                    price_in_ticks,
                ),
            });
        }

        if let Some(take_profit) = &bracket.take_profit {
            let price_in_ticks = calc.price_to_ticks(take_profit.price)?.as_inner();
            resolved_legs.push(ResolvedBracketLeg {
                size: take_profit.resolved_size(),
                trigger: TriggerOrderParams::new(
                    tp_direction,
                    bracket_trade_side,
                    StopLossOrderKind::IOC,
                    price_in_ticks,
                    price_in_ticks,
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
    use crate::phoenix_rise_ix::OrderFlags;
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
        }
    }

    fn mock_metadata(symbol: &str) -> PhoenixMetadata {
        let market = mock_market(symbol);
        let mut markets = HashMap::new();
        markets.insert(symbol.to_string(), market);
        PhoenixMetadata::new(ExchangeView {
            keys: mock_exchange_keys(),
            markets,
        })
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
    fn test_try_to_tp_sl_config_rejects_explicit_leg_sizes() {
        let bracket = BracketLegOrders {
            stop_loss: Some(BracketLeg::new(120.0).with_size(BracketLegSize::BaseLots(5))),
            take_profit: Some(BracketLeg::new(150.0)),
        };

        let err = bracket.try_to_tp_sl_config().unwrap_err();
        assert!(err.contains("custom TP/SL leg sizing"));
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
