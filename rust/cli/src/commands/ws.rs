use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

use clap::{Args, Subcommand};
use phoenix_rise::api::{
    LimitOrder, PhoenixWSClient, Position, Spline, SubaccountState, Trader, TraderKey,
};
use phoenix_rise::types::prelude::{
    CooldownStatus, ServerMessage, Timeframe, TraderStateCapabilities, TraderStatePayload,
    TraderStateServerMessage, TraderStateStopLossTrigger, TraderStateTakeProfitTrigger,
};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout};

use crate::context::CommandCtx;
use crate::output::print_json;
use crate::util::parse_pubkey;

#[derive(Debug, Clone, Args)]
pub struct WsListenArgs {
    /// Maximum seconds to wait for each websocket message when
    /// --duration-secs is not set.
    #[arg(long, default_value_t = 10)]
    pub timeout_secs: u64,
    /// Maximum messages to consume before printing the last one. Defaults to 1
    /// unless --duration-secs is provided.
    #[arg(long)]
    pub messages: Option<usize>,
    /// Total seconds to keep the subscription open before printing the last
    /// observed message.
    #[arg(long)]
    pub duration_secs: Option<u64>,
}

impl WsListenArgs {
    fn max_messages(&self) -> usize {
        self.messages.unwrap_or_else(|| {
            if self.duration_secs.is_some() {
                usize::MAX
            } else {
                1
            }
        })
    }
}

#[derive(Debug, Subcommand)]
pub enum WsCommand {
    /// Subscribe to all mid prices.
    AllMids {
        #[command(flatten)]
        listen: WsListenArgs,
    },
    /// Subscribe to one market's funding-rate feed.
    FundingRate {
        #[arg(long)]
        symbol: String,
        #[command(flatten)]
        listen: WsListenArgs,
    },
    /// Subscribe to one market's orderbook feed.
    Orderbook {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value_t = false)]
        bypass_execution_band: bool,
        #[command(flatten)]
        listen: WsListenArgs,
    },
    /// Subscribe to one market metadata feed.
    Market {
        #[arg(long)]
        symbol: String,
        #[command(flatten)]
        listen: WsListenArgs,
    },
    /// Subscribe to one market's trade feed.
    Trades {
        #[arg(long)]
        symbol: String,
        #[command(flatten)]
        listen: WsListenArgs,
    },
    /// Subscribe to one market's candle feed.
    Candles {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "1m")]
        timeframe: String,
        #[command(flatten)]
        listen: WsListenArgs,
    },
    /// Subscribe to trader-state snapshots and deltas.
    TraderState {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        trader_pda_index: u8,
        #[command(flatten)]
        listen: WsListenArgs,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraderStateWsOutput {
    subscription: TraderStateSubscriptionOutput,
    messages_received: usize,
    snapshot_seen: bool,
    last_message: LastTraderStateMessageOutput,
    state: TraderStateOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraderStateSubscriptionOutput {
    authority: String,
    trader_pda_index: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LastTraderStateMessageOutput {
    slot: u64,
    message_type: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraderStateOutput {
    authority: String,
    trader_pda_index: u8,
    last_slot: u64,
    maker_fee_override_multiplier: f64,
    taker_fee_override_multiplier: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<TraderStateCapabilities>,
    total_collateral_lots: String,
    totals: TraderStateTotals,
    subaccounts: Vec<SubaccountOutput>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraderStateTotals {
    subaccounts: usize,
    positions: usize,
    limit_orders: usize,
    stop_loss_orders: usize,
    conditional_orders: usize,
    splines: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubaccountOutput {
    subaccount_index: u8,
    sequence: u64,
    collateral_lots: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<TraderStateCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cooldown_status: Option<CooldownStatus>,
    positions: Vec<PositionOutput>,
    limit_orders: Vec<OrderOutput>,
    stop_loss_orders: Vec<OrderOutput>,
    conditional_orders: Vec<OrderOutput>,
    splines: Vec<SplineOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionOutput {
    symbol: String,
    base_position_lots: i64,
    entry_price_ticks: i64,
    entry_price_usd: String,
    virtual_quote_position_lots: i64,
    unsettled_funding_quote_lots: i64,
    accumulated_funding_quote_lots: i64,
    take_profit_triggers: Vec<TraderStateTakeProfitTrigger>,
    stop_loss_triggers: Vec<TraderStateStopLossTrigger>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderOutput {
    symbol: String,
    order_sequence_number: u64,
    side: String,
    order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    conditional_kind: Option<String>,
    price_ticks: i64,
    price_usd: String,
    size_remaining_lots: u64,
    initial_size_lots: u64,
    reduce_only: bool,
    is_stop_loss: bool,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SplineOutput {
    symbol: String,
    mid_price_ticks: i64,
    bid_filled_amount_lots: i64,
    ask_filled_amount_lots: i64,
}

pub async fn run(cmd: WsCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let client = PhoenixWSClient::from_env_with_connection_status(ctx.env.clone())?;

    match cmd {
        WsCommand::AllMids { listen } => {
            let (mut rx, _handle) = client.subscribe_to_all_mids()?;
            let (message, _) = recv_last_or_timeout(&mut rx, &listen, "allMids").await?;
            print_json(&ServerMessage::AllMids(message), ctx.output)?;
        }
        WsCommand::FundingRate { symbol, listen } => {
            let (mut rx, _handle) =
                client.subscribe_to_funding_rate(symbol.to_ascii_uppercase())?;
            let (message, _) = recv_last_or_timeout(&mut rx, &listen, "fundingRate").await?;
            print_json(&ServerMessage::FundingRate(message), ctx.output)?;
        }
        WsCommand::Orderbook {
            symbol,
            bypass_execution_band,
            listen,
        } => {
            let (mut rx, _handle) = client.subscribe_to_orderbook_with_options(
                symbol.to_ascii_uppercase(),
                bypass_execution_band,
            )?;
            let (message, _) = recv_last_or_timeout(&mut rx, &listen, "orderbook").await?;
            print_json(&ServerMessage::Orderbook(message), ctx.output)?;
        }
        WsCommand::Market { symbol, listen } => {
            let (mut rx, _handle) = client.subscribe_to_market(symbol.to_ascii_uppercase())?;
            let (message, _) = recv_last_or_timeout(&mut rx, &listen, "market").await?;
            print_json(&ServerMessage::Market(message), ctx.output)?;
        }
        WsCommand::Trades { symbol, listen } => {
            let (mut rx, _handle) = client.subscribe_to_trades(symbol.to_ascii_uppercase())?;
            let (message, _) = recv_last_or_timeout(&mut rx, &listen, "trades").await?;
            print_json(&ServerMessage::Trades(message), ctx.output)?;
        }
        WsCommand::Candles {
            symbol,
            timeframe,
            listen,
        } => {
            let timeframe = Timeframe::from_str(&timeframe)
                .map_err(|e| format!("invalid timeframe '{timeframe}': {e}"))?;
            let (mut rx, _handle) =
                client.subscribe_to_candles(symbol.to_ascii_uppercase(), timeframe)?;
            let (message, _) = recv_last_or_timeout(&mut rx, &listen, "candles").await?;
            print_json(&ServerMessage::Candles(message), ctx.output)?;
        }
        WsCommand::TraderState {
            authority,
            trader_pda_index,
            listen,
        } => {
            let authority = parse_pubkey(&authority)?;
            let (mut rx, _handle) =
                client.subscribe_to_trader_state_with_pda(&authority, trader_pda_index)?;
            let output =
                recv_trader_state(&mut rx, &listen, authority.to_string(), trader_pda_index)
                    .await?;
            if ctx.output.json || ctx.output.pretty {
                print_json(&output, ctx.output)?;
            } else {
                print_trader_state_human(&output);
            }
        }
    }

    Ok(())
}

async fn recv_last_or_timeout<T>(
    rx: &mut mpsc::UnboundedReceiver<T>,
    listen: &WsListenArgs,
    label: &str,
) -> Result<(T, usize), Box<dyn Error>> {
    let mut last = None;
    let mut messages_received = 0usize;
    let max_messages = validate_listen_args(listen)?;
    let deadline = listen
        .duration_secs
        .map(|duration| Instant::now() + Duration::from_secs(duration));

    while messages_received < max_messages {
        let Some(wait_duration) = next_wait_duration(listen.timeout_secs, deadline) else {
            break;
        };

        match timeout(wait_duration, rx.recv()).await {
            Ok(Some(message)) => {
                last = Some(message);
                messages_received += 1;
            }
            Ok(None) => {
                if last.is_some() {
                    break;
                }
                return Err(format!("{label} channel closed before first message").into());
            }
            Err(_) => {
                if last.is_some() {
                    break;
                }
                return Err(format!("timed out waiting for first {label} message").into());
            }
        }
    }

    last.map(|message| (message, messages_received))
        .ok_or_else(|| format!("no {label} messages received").into())
}

async fn recv_trader_state(
    rx: &mut mpsc::UnboundedReceiver<TraderStateServerMessage>,
    listen: &WsListenArgs,
    authority: String,
    trader_pda_index: u8,
) -> Result<TraderStateWsOutput, Box<dyn Error>> {
    let authority_pubkey = parse_pubkey(&authority)?;
    let mut trader = Trader::new(TraderKey::new_with_idx(
        authority_pubkey,
        trader_pda_index,
        0,
    ));
    let mut last = None;
    let mut messages_received = 0usize;
    let mut snapshot_seen = false;
    let max_messages = validate_listen_args(listen)?;
    let deadline = listen
        .duration_secs
        .map(|duration| Instant::now() + Duration::from_secs(duration));

    while messages_received < max_messages {
        let Some(wait_duration) = next_wait_duration(listen.timeout_secs, deadline) else {
            break;
        };

        match timeout(wait_duration, rx.recv()).await {
            Ok(Some(message)) => {
                if matches!(&message.content, TraderStatePayload::Snapshot(_)) {
                    snapshot_seen = true;
                }
                trader.apply_update(&message);
                last = Some(message);
                messages_received += 1;
            }
            Ok(None) => {
                if last.is_some() {
                    break;
                }
                return Err("traderState channel closed before first message".into());
            }
            Err(_) => {
                if last.is_some() {
                    break;
                }
                return Err("timed out waiting for first traderState message".into());
            }
        }
    }

    let last_message: TraderStateServerMessage =
        last.ok_or("no traderState messages received before timeout")?;
    Ok(TraderStateWsOutput {
        subscription: TraderStateSubscriptionOutput {
            authority,
            trader_pda_index,
        },
        messages_received,
        snapshot_seen,
        last_message: LastTraderStateMessageOutput {
            slot: last_message.slot,
            message_type: trader_state_message_type(&last_message),
        },
        state: TraderStateOutput::from_trader(&trader),
    })
}

fn validate_listen_args(listen: &WsListenArgs) -> Result<usize, Box<dyn Error>> {
    if listen.timeout_secs == 0 {
        return Err("--timeout-secs must be greater than zero".into());
    }
    if listen.duration_secs == Some(0) {
        return Err("--duration-secs must be greater than zero".into());
    }
    let max_messages = listen.max_messages();
    if max_messages == 0 {
        return Err("--messages must be greater than zero".into());
    }
    Ok(max_messages)
}

fn next_wait_duration(timeout_secs: u64, deadline: Option<Instant>) -> Option<Duration> {
    let per_message_timeout = Duration::from_secs(timeout_secs);
    let Some(deadline) = deadline else {
        return Some(per_message_timeout);
    };

    let now = Instant::now();
    if now >= deadline {
        return None;
    }
    Some(deadline - now)
}

fn trader_state_message_type(message: &TraderStateServerMessage) -> &'static str {
    match &message.content {
        TraderStatePayload::Snapshot(_) => "snapshot",
        TraderStatePayload::Delta(_) => "delta",
    }
}

impl TraderStateOutput {
    fn from_trader(trader: &Trader) -> Self {
        let mut subaccounts: Vec<_> = trader
            .subaccounts
            .values()
            .map(SubaccountOutput::from_subaccount)
            .collect();
        subaccounts.sort_by_key(|subaccount| subaccount.subaccount_index);

        let totals = TraderStateTotals {
            subaccounts: subaccounts.len(),
            positions: subaccounts.iter().map(|sub| sub.positions.len()).sum(),
            limit_orders: subaccounts.iter().map(|sub| sub.limit_orders.len()).sum(),
            stop_loss_orders: subaccounts
                .iter()
                .map(|sub| sub.stop_loss_orders.len())
                .sum(),
            conditional_orders: subaccounts
                .iter()
                .map(|sub| sub.conditional_orders.len())
                .sum(),
            splines: subaccounts.iter().map(|sub| sub.splines.len()).sum(),
        };

        Self {
            authority: trader.key.authority.to_string(),
            trader_pda_index: trader.key.pda_index,
            last_slot: trader.last_slot,
            maker_fee_override_multiplier: trader.maker_fee_override_multiplier,
            taker_fee_override_multiplier: trader.taker_fee_override_multiplier,
            capabilities: trader.capabilities.clone(),
            total_collateral_lots: trader.total_collateral().to_string(),
            totals,
            subaccounts,
        }
    }
}

impl SubaccountOutput {
    fn from_subaccount(subaccount: &SubaccountState) -> Self {
        let mut positions: Vec<_> = subaccount
            .positions
            .values()
            .map(PositionOutput::from_position)
            .collect();
        positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        let mut limit_orders = Vec::new();
        let mut stop_loss_orders = Vec::new();
        let mut conditional_orders = Vec::new();
        for order in subaccount.orders.values() {
            let output = OrderOutput::from_order(order);
            if order.is_stop_loss
                || order
                    .conditional_kind
                    .as_deref()
                    .is_some_and(|kind| kind.to_ascii_lowercase().contains("stop"))
            {
                stop_loss_orders.push(output);
            } else if order.conditional_kind.is_some() {
                conditional_orders.push(output);
            } else {
                limit_orders.push(output);
            }
        }
        sort_orders(&mut limit_orders);
        sort_orders(&mut stop_loss_orders);
        sort_orders(&mut conditional_orders);

        let mut splines: Vec<_> = subaccount
            .splines
            .values()
            .map(SplineOutput::from_spline)
            .collect();
        splines.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        Self {
            subaccount_index: subaccount.subaccount_index,
            sequence: subaccount.sequence,
            collateral_lots: subaccount.collateral.to_string(),
            capabilities: subaccount.capabilities.clone(),
            cooldown_status: subaccount.cooldown_status.clone(),
            positions,
            limit_orders,
            stop_loss_orders,
            conditional_orders,
            splines,
        }
    }
}

impl PositionOutput {
    fn from_position(position: &Position) -> Self {
        Self {
            symbol: position.symbol.clone(),
            base_position_lots: position.base_position_lots,
            entry_price_ticks: position.entry_price_ticks,
            entry_price_usd: position.entry_price_usd.to_string(),
            virtual_quote_position_lots: position.virtual_quote_position_lots,
            unsettled_funding_quote_lots: position.unsettled_funding_quote_lots,
            accumulated_funding_quote_lots: position.accumulated_funding_quote_lots,
            take_profit_triggers: position.take_profit_triggers.clone(),
            stop_loss_triggers: position.stop_loss_triggers.clone(),
        }
    }
}

impl OrderOutput {
    fn from_order(order: &LimitOrder) -> Self {
        Self {
            symbol: order.symbol.clone(),
            order_sequence_number: order.order_sequence_number,
            side: order.side.clone(),
            order_type: order.order_type.clone(),
            conditional_kind: order.conditional_kind.clone(),
            price_ticks: order.price_ticks,
            price_usd: order.price_usd.to_string(),
            size_remaining_lots: order.size_remaining_lots,
            initial_size_lots: order.initial_size_lots,
            reduce_only: order.reduce_only,
            is_stop_loss: order.is_stop_loss,
            status: order.status.clone(),
        }
    }
}

impl SplineOutput {
    fn from_spline(spline: &Spline) -> Self {
        Self {
            symbol: spline.symbol.clone(),
            mid_price_ticks: spline.mid_price_ticks,
            bid_filled_amount_lots: spline.bid_filled_amount_lots,
            ask_filled_amount_lots: spline.ask_filled_amount_lots,
        }
    }
}

fn sort_orders(orders: &mut [OrderOutput]) {
    orders.sort_by(|a, b| {
        a.symbol
            .cmp(&b.symbol)
            .then(a.order_sequence_number.cmp(&b.order_sequence_number))
    });
}

fn print_trader_state_human(output: &TraderStateWsOutput) {
    println!(
        "Trader state {} pda {} | messages {} | last {} @ slot {} | snapshot seen {}",
        output.subscription.authority,
        output.subscription.trader_pda_index,
        output.messages_received,
        output.last_message.message_type,
        output.last_message.slot,
        output.snapshot_seen
    );
    println!(
        "Total collateral lots: {} | subaccounts {} | positions {} | limit orders {} | stop \
         losses {} | conditionals {} | splines {}",
        output.state.total_collateral_lots,
        output.state.totals.subaccounts,
        output.state.totals.positions,
        output.state.totals.limit_orders,
        output.state.totals.stop_loss_orders,
        output.state.totals.conditional_orders,
        output.state.totals.splines
    );
    if let Some(capabilities) = &output.state.capabilities {
        println!(
            "Capabilities: state={} flags={}",
            capabilities.state, capabilities.flags
        );
    }

    for subaccount in &output.state.subaccounts {
        println!(
            "\nSubaccount {} | sequence {} | collateral lots {}",
            subaccount.subaccount_index, subaccount.sequence, subaccount.collateral_lots
        );
        if let Some(capabilities) = &subaccount.capabilities {
            println!(
                "  Capabilities: state={} flags={}",
                capabilities.state, capabilities.flags
            );
        }
        if let Some(cooldown) = &subaccount.cooldown_status {
            println!(
                "  Cooldown: last_deposit_slot={} cooldown_slots={}",
                cooldown.last_deposit_slot, cooldown.cooldown_period_in_slots
            );
        }

        print_positions(&subaccount.positions);
        print_orders("Limit orders", &subaccount.limit_orders);
        print_orders("Stop loss orders", &subaccount.stop_loss_orders);
        print_orders("Conditional orders", &subaccount.conditional_orders);
        print_splines(&subaccount.splines);
    }
}

fn print_positions(positions: &[PositionOutput]) {
    if positions.is_empty() {
        println!("  Positions: none");
        return;
    }
    println!("  Positions:");
    for position in positions {
        println!(
            "    {} base_lots={} entry_ticks={} entry_usd={} vquote_lots={} unsettled_funding={} \
             accumulated_funding={}",
            position.symbol,
            position.base_position_lots,
            position.entry_price_ticks,
            position.entry_price_usd,
            position.virtual_quote_position_lots,
            position.unsettled_funding_quote_lots,
            position.accumulated_funding_quote_lots
        );
        for trigger in &position.stop_loss_triggers {
            println!(
                "      SL {} side={:?} trigger_ticks={} exec_ticks={} kind={} status={}",
                trigger.stop_loss_id,
                trigger.trigger.side,
                trigger.trigger.trigger_price_ticks,
                trigger.trigger.execution_price_ticks,
                trigger.trigger.kind,
                trigger.status
            );
        }
        for trigger in &position.take_profit_triggers {
            println!(
                "      TP {} side={:?} trigger_ticks={} exec_ticks={} kind={} status={}",
                trigger.take_profit_id,
                trigger.trigger.side,
                trigger.trigger.trigger_price_ticks,
                trigger.trigger.execution_price_ticks,
                trigger.trigger.kind,
                trigger.status
            );
        }
    }
}

fn print_orders(label: &str, orders: &[OrderOutput]) {
    if orders.is_empty() {
        println!("  {label}: none");
        return;
    }
    println!("  {label}:");
    for order in orders {
        let conditional = order.conditional_kind.as_deref().unwrap_or("-");
        println!(
            "    {} #{} {} type={} conditional={} price_ticks={} price_usd={} remaining_lots={} \
             initial_lots={} reduce_only={} status={}",
            order.symbol,
            order.order_sequence_number,
            order.side,
            order.order_type,
            conditional,
            order.price_ticks,
            order.price_usd,
            order.size_remaining_lots,
            order.initial_size_lots,
            order.reduce_only,
            order.status
        );
    }
}

fn print_splines(splines: &[SplineOutput]) {
    if splines.is_empty() {
        println!("  Splines: none");
        return;
    }
    println!("  Splines:");
    for spline in splines {
        println!(
            "    {} mid_ticks={} bid_filled_lots={} ask_filled_lots={}",
            spline.symbol,
            spline.mid_price_ticks,
            spline.bid_filled_amount_lots,
            spline.ask_filled_amount_lots
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_single_message_without_duration() {
        let listen = WsListenArgs {
            timeout_secs: 10,
            messages: None,
            duration_secs: None,
        };

        assert_eq!(listen.max_messages(), 1);
    }

    #[test]
    fn duration_defaults_to_open_ended_message_count() {
        let listen = WsListenArgs {
            timeout_secs: 10,
            messages: None,
            duration_secs: Some(5),
        };

        assert_eq!(listen.max_messages(), usize::MAX);
    }

    #[test]
    fn rejects_zero_listen_values() {
        let mut listen = WsListenArgs {
            timeout_secs: 0,
            messages: Some(1),
            duration_secs: None,
        };
        assert!(validate_listen_args(&listen).is_err());

        listen.timeout_secs = 1;
        listen.messages = Some(0);
        assert!(validate_listen_args(&listen).is_err());

        listen.messages = Some(1);
        listen.duration_secs = Some(0);
        assert!(validate_listen_args(&listen).is_err());
    }

    #[test]
    fn duration_wait_uses_total_deadline() {
        let wait = next_wait_duration(1, Some(Instant::now() + Duration::from_secs(5))).unwrap();

        assert!(wait > Duration::from_secs(4));
    }
}
