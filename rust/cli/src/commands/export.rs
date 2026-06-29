use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use clap::{Args, Subcommand};
use phoenix_rise::types::prelude::{
    ApiCandle, CandlesQueryParams, FundingHistoryEvent, FundingHistoryQueryParams,
    OrderHistoryItem, OrderHistoryQueryParams, Timeframe, TradeHistoryItem,
    TradeHistoryQueryParams, UserLiquidationHistoryPoint, UserLiquidationHistoryQueryParams,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::context::{CommandCtx, expand_cli_path};
use crate::output::print_json;
use crate::util::parse_pubkey;

const HISTORY_PAGE_LIMIT: u32 = 1_000;
const LIQUIDATION_PAGE_LIMIT: u32 = 100;
const CANDLE_PAGE_LIMIT: u32 = 2_500;
const DEFAULT_HISTORY_PAGE_SIZE: u32 = 1_000;
const DEFAULT_LIQUIDATION_PAGE_SIZE: u32 = 100;
const DEFAULT_CANDLE_PAGE_SIZE: u32 = 2_500;

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    /// Fetch every page of trade history matching the requested bounds.
    TradeHistory(TradeHistoryExportArgs),
    /// Fetch every page of order history matching the requested bounds.
    OrderHistory(OrderHistoryExportArgs),
    /// Fetch every page of funding history matching the requested bounds.
    FundingHistory(FundingHistoryExportArgs),
    /// Fetch every page of liquidation/backstop/ADL history matching bounds.
    LiquidationHistory(LiquidationHistoryExportArgs),
    /// Fetch candle windows for a market and save one merged JSON payload.
    Candles(CandlesExportArgs),
}

#[derive(Debug, Clone, Args)]
pub struct TimeBoundsArgs {
    /// Inclusive lower timestamp bound. Accepts unix seconds, unix
    /// milliseconds, RFC3339, or YYYY-MM-DD.
    #[arg(long)]
    pub start_time: Option<String>,
    /// Inclusive upper timestamp bound. Accepts unix seconds, unix
    /// milliseconds, RFC3339, or YYYY-MM-DD.
    #[arg(long)]
    pub end_time: Option<String>,
    /// Human-readable duration ending at --end-time or now, for example 7d,
    /// 24h, or 1h30m.
    #[arg(long, conflicts_with = "start_time")]
    pub lookback: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ExportOutputArgs {
    /// Write the complete JSON response to this path instead of stdout.
    #[arg(long, visible_alias = "out")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct TradeHistoryExportArgs {
    #[arg(long)]
    pub authority: String,
    #[arg(long, default_value_t = 0)]
    pub pda_index: u8,
    #[arg(long)]
    pub market_symbol: Option<String>,
    #[arg(long, default_value_t = DEFAULT_HISTORY_PAGE_SIZE)]
    pub page_size: u32,
    #[arg(long)]
    pub max_pages: Option<usize>,
    #[arg(long)]
    pub max_items: Option<usize>,
    #[command(flatten)]
    pub time: TimeBoundsArgs,
    #[command(flatten)]
    pub output: ExportOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct OrderHistoryExportArgs {
    #[arg(long)]
    pub authority: String,
    #[arg(long, default_value_t = 0)]
    pub pda_index: u8,
    #[arg(long)]
    pub market_symbol: Option<String>,
    #[arg(long, default_value_t = DEFAULT_HISTORY_PAGE_SIZE)]
    pub page_size: u32,
    #[arg(long)]
    pub max_pages: Option<usize>,
    #[arg(long)]
    pub max_items: Option<usize>,
    #[command(flatten)]
    pub time: TimeBoundsArgs,
    #[command(flatten)]
    pub output: ExportOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct FundingHistoryExportArgs {
    #[arg(long)]
    pub authority: String,
    #[arg(long, default_value_t = 0)]
    pub pda_index: u8,
    #[arg(long)]
    pub symbol: Option<String>,
    #[arg(long, default_value_t = DEFAULT_HISTORY_PAGE_SIZE)]
    pub page_size: u32,
    #[arg(long)]
    pub max_pages: Option<usize>,
    #[arg(long)]
    pub max_items: Option<usize>,
    #[command(flatten)]
    pub time: TimeBoundsArgs,
    #[command(flatten)]
    pub output: ExportOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct LiquidationHistoryExportArgs {
    #[arg(long)]
    pub authority: String,
    #[arg(long, default_value_t = 0)]
    pub pda_index: i32,
    #[arg(long)]
    pub subaccount_index: Option<i32>,
    #[arg(long)]
    pub symbol: Option<String>,
    #[arg(long, default_value_t = DEFAULT_LIQUIDATION_PAGE_SIZE)]
    pub page_size: u32,
    #[arg(long)]
    pub max_pages: Option<usize>,
    #[arg(long)]
    pub max_items: Option<usize>,
    #[command(flatten)]
    pub time: TimeBoundsArgs,
    #[command(flatten)]
    pub output: ExportOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct CandlesExportArgs {
    #[arg(long)]
    pub symbol: String,
    #[arg(long, default_value = "1m")]
    pub timeframe: String,
    #[arg(long, default_value_t = DEFAULT_CANDLE_PAGE_SIZE)]
    pub page_size: u32,
    #[arg(long)]
    pub max_pages: Option<usize>,
    #[arg(long)]
    pub max_items: Option<usize>,
    #[command(flatten)]
    pub time: TimeBoundsArgs,
    #[command(flatten)]
    pub output: ExportOutputArgs,
}

#[derive(Debug, Clone, Copy)]
struct TimeBounds {
    start_ms: Option<i64>,
    end_ms: Option<i64>,
}

impl TimeBounds {
    fn has_bounds(self) -> bool {
        self.start_ms.is_some() || self.end_ms.is_some()
    }

    fn contains(self, timestamp_ms: i64) -> bool {
        if let Some(start_ms) = self.start_ms
            && timestamp_ms < start_ms
        {
            return false;
        }
        if let Some(end_ms) = self.end_ms
            && timestamp_ms > end_ms
        {
            return false;
        }
        true
    }

    fn require_candle_window(self) -> Result<(i64, i64), Box<dyn Error>> {
        let start_ms = self
            .start_ms
            .ok_or("candles export requires --start-time or --lookback")?;
        let end_ms = self.end_ms.unwrap_or_else(now_ms);
        if start_ms > end_ms {
            return Err("--start-time must be less than or equal to --end-time".into());
        }
        Ok((start_ms, end_ms))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportBoundsOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time_iso: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time_iso: Option<String>,
}

impl From<TimeBounds> for ExportBoundsOutput {
    fn from(bounds: TimeBounds) -> Self {
        Self {
            start_time: bounds.start_ms,
            start_time_iso: bounds.start_ms.and_then(ms_to_iso),
            end_time: bounds.end_ms,
            end_time_iso: bounds.end_ms.and_then(ms_to_iso),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult<T> {
    kind: &'static str,
    query: Value,
    bounds: ExportBoundsOutput,
    page_size: u32,
    pages_fetched: usize,
    item_count: usize,
    exhausted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
    data: Vec<T>,
}

pub async fn run(cmd: ExportCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        ExportCommand::TradeHistory(args) => export_trade_history(args, ctx).await,
        ExportCommand::OrderHistory(args) => export_order_history(args, ctx).await,
        ExportCommand::FundingHistory(args) => export_funding_history(args, ctx).await,
        ExportCommand::LiquidationHistory(args) => export_liquidation_history(args, ctx).await,
        ExportCommand::Candles(args) => export_candles(args, ctx).await,
    }
}

async fn export_trade_history(
    args: TradeHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_trade_history(args, ctx).await?;
    emit_export(&result, output, ctx)
}

async fn export_order_history(
    args: OrderHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_order_history(args, ctx).await?;
    emit_export(&result, output, ctx)
}

async fn export_funding_history(
    args: FundingHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_funding_history(args, ctx).await?;
    emit_export(&result, output, ctx)
}

async fn export_liquidation_history(
    args: LiquidationHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_liquidation_history(args, ctx).await?;
    emit_export(&result, output, ctx)
}

async fn export_candles(args: CandlesExportArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_candles(args, ctx).await?;
    emit_export(&result, output, ctx)
}

pub async fn show_trade_history(
    args: TradeHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_trade_history(args, ctx).await?;
    emit_table_or_json(&result, output, ctx, print_trade_history_table)
}

pub async fn show_order_history(
    args: OrderHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_order_history(args, ctx).await?;
    emit_table_or_json(&result, output, ctx, print_order_history_table)
}

pub async fn show_funding_history(
    args: FundingHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_funding_history(args, ctx).await?;
    emit_table_or_json(&result, output, ctx, print_funding_history_table)
}

pub async fn show_liquidation_history(
    args: LiquidationHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_liquidation_history(args, ctx).await?;
    emit_table_or_json(&result, output, ctx, print_liquidation_history_table)
}

pub async fn show_candles(args: CandlesExportArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let output = args.output.output.clone();
    let result = fetch_candles(args, ctx).await?;
    emit_table_or_json(&result, output, ctx, print_candles_table)
}

async fn fetch_trade_history(
    args: TradeHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<ExportResult<TradeHistoryItem>, Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let bounds = resolve_time_bounds(&args.time)?;
    let page_limit = validate_page_size(args.page_size, HISTORY_PAGE_LIMIT)?;
    let client = ctx.http_client()?;

    let mut data = Vec::new();
    let mut cursor = None;
    let mut pages_fetched = 0;
    let mut exhausted = false;
    let market_symbol = args.market_symbol.map(|symbol| symbol.to_ascii_uppercase());

    while can_fetch_more(pages_fetched, args.max_pages, data.len(), args.max_items) {
        let mut params = TradeHistoryQueryParams::new()
            .with_pda_index(args.pda_index)
            .with_limit(page_limit);
        if let Some(symbol) = &market_symbol {
            params = params.with_market_symbol(symbol.clone());
        }
        if let Some(next_cursor) = cursor.clone() {
            params = params.with_cursor(next_cursor);
        }

        let response = client.get_trade_history(&authority, params).await?;
        pages_fetched += 1;
        let page_is_older = page_is_older_than_start(&response.data, bounds, trade_timestamp_ms);
        collect_in_bounds(
            &response.data,
            bounds,
            &mut data,
            args.max_items,
            trade_timestamp_ms,
        );

        cursor = response.next_cursor;
        if !response.has_more || cursor.is_none() || page_is_older {
            exhausted = true;
            break;
        }
    }

    Ok(ExportResult {
        kind: "tradeHistory",
        query: json!({
            "authority": authority.to_string(),
            "pdaIndex": args.pda_index,
            "marketSymbol": market_symbol,
        }),
        bounds: bounds.into(),
        page_size: args.page_size,
        pages_fetched,
        item_count: data.len(),
        exhausted,
        next_cursor: cursor,
        data,
    })
}

async fn fetch_order_history(
    args: OrderHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<ExportResult<OrderHistoryItem>, Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let bounds = resolve_time_bounds(&args.time)?;
    let page_limit = validate_page_size(args.page_size, HISTORY_PAGE_LIMIT)?;
    let client = ctx.http_client()?;

    let mut data = Vec::new();
    let mut cursor = None;
    let mut pages_fetched = 0;
    let mut exhausted = false;
    let market_symbol = args.market_symbol.map(|symbol| symbol.to_ascii_uppercase());

    while can_fetch_more(pages_fetched, args.max_pages, data.len(), args.max_items) {
        let mut params = OrderHistoryQueryParams::new(page_limit).with_pda_index(args.pda_index);
        if let Some(symbol) = &market_symbol {
            params = params.with_market_symbol(symbol.clone());
        }
        if let Some(next_cursor) = cursor.clone() {
            params = params.with_cursor(next_cursor);
        }

        let response = client.get_order_history(&authority, params).await?;
        pages_fetched += 1;
        let page_is_older = page_is_older_than_start(&response.data, bounds, order_timestamp_ms);
        collect_in_bounds(
            &response.data,
            bounds,
            &mut data,
            args.max_items,
            order_timestamp_ms,
        );

        cursor = response.next_cursor;
        if !response.has_more || cursor.is_none() || page_is_older {
            exhausted = true;
            break;
        }
    }

    Ok(ExportResult {
        kind: "orderHistory",
        query: json!({
            "authority": authority.to_string(),
            "pdaIndex": args.pda_index,
            "marketSymbol": market_symbol,
        }),
        bounds: bounds.into(),
        page_size: args.page_size,
        pages_fetched,
        item_count: data.len(),
        exhausted,
        next_cursor: cursor,
        data,
    })
}

async fn fetch_funding_history(
    args: FundingHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<ExportResult<FundingHistoryEvent>, Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let bounds = resolve_time_bounds(&args.time)?;
    let page_limit = validate_page_size(args.page_size, HISTORY_PAGE_LIMIT)?;
    let client = ctx.http_client()?;

    let mut data = Vec::new();
    let mut cursor = None;
    let mut pages_fetched = 0;
    let mut exhausted = false;
    let symbol = args.symbol.map(|symbol| symbol.to_ascii_uppercase());

    while can_fetch_more(pages_fetched, args.max_pages, data.len(), args.max_items) {
        let mut params = FundingHistoryQueryParams::new()
            .with_pda_index(args.pda_index)
            .with_limit(page_limit);
        if let Some(symbol) = &symbol {
            params = params.with_symbol(symbol.clone());
        }
        if let Some(next_cursor) = cursor.clone() {
            params = params.with_cursor(next_cursor);
        } else {
            if let Some(start_ms) = bounds.start_ms {
                params = params.with_start_time(start_ms);
            }
            if let Some(end_ms) = bounds.end_ms {
                params = params.with_end_time(end_ms);
            }
        }

        let response = client.get_funding_history(&authority, params).await?;
        pages_fetched += 1;
        let page_is_older =
            page_is_older_than_start(&response.events, bounds, funding_timestamp_ms);
        collect_in_bounds(
            &response.events,
            bounds,
            &mut data,
            args.max_items,
            funding_timestamp_ms,
        );

        cursor = response.next_cursor;
        if !response.has_more || cursor.is_none() || page_is_older {
            exhausted = true;
            break;
        }
    }

    Ok(ExportResult {
        kind: "fundingHistory",
        query: json!({
            "authority": authority.to_string(),
            "pdaIndex": args.pda_index,
            "symbol": symbol,
        }),
        bounds: bounds.into(),
        page_size: args.page_size,
        pages_fetched,
        item_count: data.len(),
        exhausted,
        next_cursor: cursor,
        data,
    })
}

async fn fetch_liquidation_history(
    args: LiquidationHistoryExportArgs,
    ctx: &CommandCtx,
) -> Result<ExportResult<UserLiquidationHistoryPoint>, Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let bounds = resolve_time_bounds(&args.time)?;
    let page_limit = validate_page_size(args.page_size, LIQUIDATION_PAGE_LIMIT)?;
    let client = ctx.http_client()?;

    let mut data = Vec::new();
    let mut cursor = None;
    let mut pages_fetched = 0;
    let mut exhausted = false;
    let symbol = args.symbol.map(|symbol| symbol.to_ascii_uppercase());

    while can_fetch_more(pages_fetched, args.max_pages, data.len(), args.max_items) {
        let mut params = UserLiquidationHistoryQueryParams::new().with_pda_index(args.pda_index);
        params = params.with_limit(page_limit);
        if let Some(subaccount_index) = args.subaccount_index {
            params = params.with_subaccount_index(subaccount_index);
        }
        if let Some(symbol) = &symbol {
            params = params.with_symbol(symbol.clone());
        }
        if let Some(next_cursor) = cursor.clone() {
            params = params.with_cursor(next_cursor);
        }

        let response = client
            .get_user_liquidation_history(&authority, params)
            .await?;
        pages_fetched += 1;
        let page_is_older =
            page_is_older_than_start(&response.data, bounds, liquidation_timestamp_ms);
        collect_in_bounds(
            &response.data,
            bounds,
            &mut data,
            args.max_items,
            liquidation_timestamp_ms,
        );

        cursor = response.next_cursor;
        if !response.has_more || cursor.is_none() || page_is_older {
            exhausted = true;
            break;
        }
    }

    Ok(ExportResult {
        kind: "liquidationHistory",
        query: json!({
            "authority": authority.to_string(),
            "pdaIndex": args.pda_index,
            "subaccountIndex": args.subaccount_index,
            "symbol": symbol,
        }),
        bounds: bounds.into(),
        page_size: args.page_size,
        pages_fetched,
        item_count: data.len(),
        exhausted,
        next_cursor: cursor,
        data,
    })
}

async fn fetch_candles(
    args: CandlesExportArgs,
    ctx: &CommandCtx,
) -> Result<ExportResult<ApiCandle>, Box<dyn Error>> {
    let bounds = resolve_time_bounds(&args.time)?;
    let (mut current_start_ms, end_ms) = bounds.require_candle_window()?;
    let page_size = validate_candle_page_size(args.page_size)?;
    let timeframe = Timeframe::from_str(&args.timeframe)
        .map_err(|e| format!("invalid timeframe '{}': {e}", args.timeframe))?;
    let timeframe_ms = timeframe.as_seconds().saturating_mul(1_000).max(1);
    let symbol = args.symbol.to_ascii_uppercase();
    let client = ctx.http_client()?;

    let mut candles_by_time = BTreeMap::new();
    let mut pages_fetched = 0;
    let mut exhausted = false;

    while current_start_ms <= end_ms
        && can_fetch_more(
            pages_fetched,
            args.max_pages,
            candles_by_time.len(),
            args.max_items,
        )
    {
        let params = CandlesQueryParams::new(symbol.clone(), timeframe)
            .with_start_time(current_start_ms)
            .with_end_time(end_ms)
            .with_limit(page_size);
        let page = client.get_candles(params).await?;
        pages_fetched += 1;

        if page.is_empty() {
            exhausted = true;
            break;
        }

        let mut max_timestamp_ms = None;
        let page_len = page.len();
        for candle in page {
            let timestamp_ms = candle_timestamp_ms(&candle);
            max_timestamp_ms = max_timestamp_ms.max(Some(timestamp_ms));
            if bounds.contains(timestamp_ms) {
                if let Some(max_items) = args.max_items
                    && candles_by_time.len() >= max_items
                {
                    break;
                }
                candles_by_time.entry(candle.time).or_insert(candle);
            }
        }

        let Some(max_timestamp_ms) = max_timestamp_ms else {
            exhausted = true;
            break;
        };
        if page_len < page_size as usize || max_timestamp_ms >= end_ms {
            exhausted = true;
            break;
        }

        let next_start_ms = max_timestamp_ms.saturating_add(timeframe_ms);
        current_start_ms = if next_start_ms <= current_start_ms {
            current_start_ms.saturating_add(timeframe_ms)
        } else {
            next_start_ms
        };
    }

    let data: Vec<ApiCandle> = candles_by_time.into_values().collect();
    Ok(ExportResult {
        kind: "candles",
        query: json!({
            "symbol": symbol,
            "timeframe": timeframe.to_string(),
        }),
        bounds: bounds.into(),
        page_size,
        pages_fetched,
        item_count: data.len(),
        exhausted,
        next_cursor: None,
        data,
    })
}

fn collect_in_bounds<T, F>(
    items: &[T],
    bounds: TimeBounds,
    output: &mut Vec<T>,
    max_items: Option<usize>,
    timestamp_ms: F,
) where
    T: Clone,
    F: Fn(&T) -> Option<i64>,
{
    for item in items {
        if let Some(max_items) = max_items
            && output.len() >= max_items
        {
            break;
        }

        let include = timestamp_ms(item)
            .map(|ts| bounds.contains(ts))
            .unwrap_or_else(|| !bounds.has_bounds());
        if include {
            output.push(item.clone());
        }
    }
}

fn page_is_older_than_start<T, F>(items: &[T], bounds: TimeBounds, timestamp_ms: F) -> bool
where
    F: Fn(&T) -> Option<i64>,
{
    let Some(start_ms) = bounds.start_ms else {
        return false;
    };

    let mut saw_timestamp = false;
    for item in items {
        if let Some(timestamp_ms) = timestamp_ms(item) {
            saw_timestamp = true;
            if timestamp_ms >= start_ms {
                return false;
            }
        }
    }
    saw_timestamp
}

fn can_fetch_more(
    pages_fetched: usize,
    max_pages: Option<usize>,
    items_fetched: usize,
    max_items: Option<usize>,
) -> bool {
    if let Some(max_pages) = max_pages
        && pages_fetched >= max_pages
    {
        return false;
    }
    if let Some(max_items) = max_items
        && items_fetched >= max_items
    {
        return false;
    }
    true
}

fn resolve_time_bounds(args: &TimeBoundsArgs) -> Result<TimeBounds, Box<dyn Error>> {
    let end_ms = args
        .end_time
        .as_deref()
        .map(parse_timestamp_ms)
        .transpose()?;
    let start_ms = args
        .start_time
        .as_deref()
        .map(parse_timestamp_ms)
        .transpose()?;

    let bounds = if let Some(lookback) = args.lookback.as_deref() {
        let lookback_ms = parse_lookback_ms(lookback)?;
        let end_ms = end_ms.unwrap_or_else(now_ms);
        TimeBounds {
            start_ms: Some(end_ms.saturating_sub(lookback_ms)),
            end_ms: Some(end_ms),
        }
    } else {
        TimeBounds { start_ms, end_ms }
    };

    if let (Some(start_ms), Some(end_ms)) = (bounds.start_ms, bounds.end_ms)
        && start_ms > end_ms
    {
        return Err("--start-time must be less than or equal to --end-time".into());
    }

    Ok(bounds)
}

fn parse_timestamp_ms(raw: &str) -> Result<i64, Box<dyn Error>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("timestamp cannot be empty".into());
    }

    if let Ok(value) = raw.parse::<i64>() {
        return Ok(if value.unsigned_abs() >= 10_000_000_000 {
            value
        } else {
            value.saturating_mul(1_000)
        });
    }

    if let Ok(date_time) = DateTime::parse_from_rfc3339(raw) {
        return Ok(date_time.with_timezone(&Utc).timestamp_millis());
    }

    if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        let date_time = date
            .and_hms_opt(0, 0, 0)
            .ok_or("invalid date timestamp")?
            .and_utc();
        return Ok(date_time.timestamp_millis());
    }

    Err(format!("invalid timestamp '{raw}'").into())
}

fn parse_lookback_ms(raw: &str) -> Result<i64, Box<dyn Error>> {
    let raw = raw.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return Err("lookback cannot be empty".into());
    }

    if let Ok(seconds) = raw.parse::<i64>() {
        if seconds <= 0 {
            return Err("lookback must be greater than zero".into());
        }
        return Ok(seconds.saturating_mul(1_000));
    }

    let bytes = raw.as_bytes();
    let mut index = 0;
    let mut total_ms: i128 = 0;
    let mut saw_component = false;

    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b',') {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        let number_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if number_start == index {
            return Err(format!("invalid lookback component in '{raw}'").into());
        }
        let amount: i128 = raw[number_start..index].parse()?;

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        let unit_start = index;
        while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
            index += 1;
        }
        let unit = &raw[unit_start..index];
        let multiplier = duration_unit_ms(unit)?;
        total_ms = total_ms
            .checked_add(amount.saturating_mul(multiplier))
            .ok_or("lookback duration is too large")?;
        saw_component = true;
    }

    if !saw_component || total_ms <= 0 {
        return Err("lookback must be greater than zero".into());
    }
    if total_ms > i64::MAX as i128 {
        return Err("lookback duration is too large".into());
    }
    Ok(total_ms as i64)
}

fn duration_unit_ms(unit: &str) -> Result<i128, Box<dyn Error>> {
    match unit {
        "ms" | "msec" | "millisecond" | "milliseconds" => Ok(1),
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(1_000),
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(60 * 1_000),
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(60 * 60 * 1_000),
        "d" | "day" | "days" => Ok(24 * 60 * 60 * 1_000),
        "w" | "week" | "weeks" => Ok(7 * 24 * 60 * 60 * 1_000),
        _ => Err(format!("unknown lookback unit '{unit}'").into()),
    }
}

fn validate_page_size(page_size: u32, max_page_size: u32) -> Result<i64, Box<dyn Error>> {
    if page_size == 0 {
        return Err("--page-size must be greater than zero".into());
    }
    if page_size > max_page_size {
        return Err(format!("--page-size cannot exceed {max_page_size}").into());
    }
    Ok(i64::from(page_size))
}

fn validate_candle_page_size(page_size: u32) -> Result<u32, Box<dyn Error>> {
    if page_size == 0 {
        return Err("--page-size must be greater than zero".into());
    }
    if page_size > CANDLE_PAGE_LIMIT {
        return Err(format!("--page-size cannot exceed {CANDLE_PAGE_LIMIT}").into());
    }
    Ok(page_size)
}

fn emit_export<T: Serialize>(
    value: &T,
    output_path: Option<PathBuf>,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let Some(path) = output_path else {
        return print_json(value, ctx.output);
    };

    let path = expand_cli_path(path)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut output = if ctx.output.compact_json() {
        serde_json::to_string(value)?
    } else {
        serde_json::to_string_pretty(value)?
    };
    output.push('\n');
    fs::write(path, output)?;
    Ok(())
}

fn emit_table_or_json<T: Serialize, F>(
    result: &ExportResult<T>,
    output_path: Option<PathBuf>,
    ctx: &CommandCtx,
    print_table: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&ExportResult<T>),
{
    if output_path.is_some() || ctx.output.json || ctx.output.pretty {
        return emit_export(result, output_path, ctx);
    }

    print_table(result);
    Ok(())
}

fn print_trade_history_table(result: &ExportResult<TradeHistoryItem>) {
    print_result_summary(result);
    print_table(
        &[
            "time",
            "market",
            "type",
            "liquidity",
            "price",
            "size",
            "pnl",
            "fees",
            "signature",
        ],
        result
            .data
            .iter()
            .map(|item| {
                vec![
                    item.timestamp.to_rfc3339(),
                    item.market_symbol.clone(),
                    format!("{:?}", item.trade_type),
                    format!("{:?}", item.liquidity),
                    item.price.clone(),
                    item.base_lots_delta.clone(),
                    item.realized_pnl.clone(),
                    item.fees.clone(),
                    item.signature
                        .as_deref()
                        .map(short_value)
                        .unwrap_or_default(),
                ]
            })
            .collect(),
    );
}

fn print_order_history_table(result: &ExportResult<OrderHistoryItem>) {
    print_result_summary(result);
    print_table(
        &[
            "placed",
            "completed",
            "market",
            "side",
            "status",
            "price",
            "base",
            "filled",
            "remaining",
            "seq",
        ],
        result
            .data
            .iter()
            .map(|item| {
                vec![
                    item.placed_at
                        .as_ref()
                        .map(DateTime::<Utc>::to_rfc3339)
                        .unwrap_or_default(),
                    item.completed_at
                        .as_ref()
                        .map(DateTime::<Utc>::to_rfc3339)
                        .unwrap_or_default(),
                    item.market_symbol.clone(),
                    format!("{:?}", item.side),
                    format!("{:?}", item.status),
                    item.price.clone(),
                    item.base_qty.clone(),
                    item.filled_base_qty.clone(),
                    item.remaining_base_qty.clone(),
                    short_value(&item.order_sequence_number),
                ]
            })
            .collect(),
    );
}

fn print_funding_history_table(result: &ExportResult<FundingHistoryEvent>) {
    print_result_summary(result);
    print_table(
        &["time", "symbol", "payment", "rate_pct", "position", "side"],
        result
            .data
            .iter()
            .map(|item| {
                vec![
                    item.timestamp.to_rfc3339(),
                    item.symbol.clone(),
                    item.funding_payment.clone(),
                    item.funding_rate_percentage.clone(),
                    item.position_size.clone(),
                    item.position_side.clone(),
                ]
            })
            .collect(),
    );
}

fn print_liquidation_history_table(result: &ExportResult<UserLiquidationHistoryPoint>) {
    print_result_summary(result);
    print_table(
        &[
            "time",
            "symbol",
            "kind",
            "type",
            "role",
            "side",
            "size",
            "price",
            "signature",
        ],
        result
            .data
            .iter()
            .map(|item| {
                vec![
                    format_timestamp_ms(timestamp_number_to_ms(item.timestamp)),
                    item.symbol.clone(),
                    format!("{:?}", item.kind),
                    format!("{:?}", item.r#type),
                    format!("{:?}", item.role),
                    item.side.clone().unwrap_or_default(),
                    item.size.clone().unwrap_or_default(),
                    item.price.clone().unwrap_or_default(),
                    item.signature
                        .as_deref()
                        .map(short_value)
                        .unwrap_or_default(),
                ]
            })
            .collect(),
    );
}

fn print_candles_table(result: &ExportResult<ApiCandle>) {
    print_result_summary(result);
    print_table(
        &["time", "open", "high", "low", "close", "volume", "trades"],
        result
            .data
            .iter()
            .map(|item| {
                vec![
                    format_timestamp_ms(candle_timestamp_ms(item)),
                    item.open.to_string(),
                    item.high.to_string(),
                    item.low.to_string(),
                    item.close.to_string(),
                    item.volume
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    item.trade_count
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ]
            })
            .collect(),
    );
}

fn print_result_summary<T>(result: &ExportResult<T>) {
    println!(
        "{}: {} item(s), {} page(s), exhausted={}",
        result.kind, result.item_count, result.pages_fetched, result.exhausted
    );
}

fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    if rows.is_empty() {
        println!("(no rows)");
        return;
    }

    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<usize>>();
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(display_cell(cell).len()).min(32);
            }
        }
    }

    print_row(
        headers.iter().map(|value| (*value).to_string()).collect(),
        &widths,
    );
    print_row(
        widths.iter().map(|width| "-".repeat(*width)).collect(),
        &widths,
    );
    for row in rows {
        print_row(row, &widths);
    }
}

fn print_row(row: Vec<String>, widths: &[usize]) {
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            print!("  ");
        }
        let cell = row.get(index).map(String::as_str).unwrap_or_default();
        print!("{:<width$}", display_cell(cell), width = width);
    }
    println!();
}

fn display_cell(cell: &str) -> String {
    const MAX_WIDTH: usize = 32;
    let cell = cell.replace('\n', " ");
    if cell.len() <= MAX_WIDTH {
        return cell;
    }
    format!("{}...", &cell[..MAX_WIDTH.saturating_sub(3)])
}

fn short_value(value: &str) -> String {
    const PREFIX: usize = 10;
    const SUFFIX: usize = 6;
    if value.len() <= PREFIX + SUFFIX + 3 {
        return value.to_string();
    }
    format!("{}...{}", &value[..PREFIX], &value[value.len() - SUFFIX..])
}

fn format_timestamp_ms(ms: i64) -> String {
    ms_to_iso(ms).unwrap_or_else(|| ms.to_string())
}

fn trade_timestamp_ms(item: &TradeHistoryItem) -> Option<i64> {
    Some(item.timestamp.timestamp_millis())
}

fn order_timestamp_ms(item: &OrderHistoryItem) -> Option<i64> {
    item.completed_at
        .as_ref()
        .or(item.placed_at.as_ref())
        .map(|timestamp| timestamp.timestamp_millis())
}

fn funding_timestamp_ms(item: &FundingHistoryEvent) -> Option<i64> {
    Some(item.timestamp.timestamp_millis())
}

fn liquidation_timestamp_ms(item: &UserLiquidationHistoryPoint) -> Option<i64> {
    Some(timestamp_number_to_ms(item.timestamp))
}

fn candle_timestamp_ms(item: &ApiCandle) -> i64 {
    item.time.saturating_mul(1_000)
}

fn timestamp_number_to_ms(timestamp: i64) -> i64 {
    if timestamp.unsigned_abs() >= 10_000_000_000 {
        timestamp
    } else {
        timestamp.saturating_mul(1_000)
    }
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn ms_to_iso(ms: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(ms).map(|timestamp| timestamp.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timestamp_seconds_milliseconds_and_rfc3339() {
        assert_eq!(parse_timestamp_ms("1775578550").unwrap(), 1_775_578_550_000);
        assert_eq!(
            parse_timestamp_ms("1775578550123").unwrap(),
            1_775_578_550_123
        );
        assert_eq!(
            parse_timestamp_ms("2026-04-07T12:15:50Z").unwrap(),
            1_775_564_150_000
        );
        assert_eq!(parse_timestamp_ms("2026-04-07").unwrap(), 1_775_520_000_000);
    }

    #[test]
    fn parses_human_lookbacks() {
        assert_eq!(parse_lookback_ms("7d").unwrap(), 604_800_000);
        assert_eq!(parse_lookback_ms("1h30m").unwrap(), 5_400_000);
        assert_eq!(parse_lookback_ms("2 weeks, 3 days").unwrap(), 1_468_800_000);
        assert_eq!(parse_lookback_ms("3600").unwrap(), 3_600_000);
    }

    #[test]
    fn rejects_invalid_time_inputs() {
        assert!(parse_timestamp_ms("").is_err());
        assert!(parse_timestamp_ms("not-a-time").is_err());
        assert!(parse_lookback_ms("1month").is_err());
        assert!(parse_lookback_ms("0s").is_err());
    }

    #[test]
    fn detects_pages_older_than_start() {
        #[derive(Clone)]
        struct Item(i64);

        let bounds = TimeBounds {
            start_ms: Some(1_000),
            end_ms: None,
        };
        assert!(page_is_older_than_start(
            &[Item(100), Item(999)],
            bounds,
            |item| Some(item.0),
        ));
        assert!(!page_is_older_than_start(
            &[Item(100), Item(1_000)],
            bounds,
            |item| Some(item.0),
        ));
    }
}
