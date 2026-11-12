//! Fill ID generation for trade reconciliation.
//!
//! This mirrors the Phoenix API UUID v3 fill-id derivation so off-chain
//! consumers can reproduce the exact same identifier from parsed events.

use thiserror::Error;
use uuid::Uuid;

use crate::MarketEvent;
use crate::market_events::PnLEvent;

/// Canonical UUID v3 namespace for Phoenix fill IDs.
pub const FILL_ID_NAMESPACE: Uuid = Uuid::from_u128(0x6c6f1488972c4e3aa68dbba0d8f12e87);
const SIGNATURE_BYTES_LEN: usize = 64;
const TRADER_PDA_BYTES_LEN: usize = 32;

#[derive(Debug, Error)]
pub enum FillIdDerivationError {
    #[error("failed to decode transaction signature from base58: {0:?}")]
    InvalidSignatureBase58(five8::DecodeError),
    #[error("failed to decode trader PDA from base58: {0:?}")]
    InvalidTraderPdaBase58(five8::DecodeError),
}

/// Minimal caller-supplied context needed to tag parsed events off-chain.
///
/// `phoenix_instruction_index` is the zero-based ordinal among non-logger
/// Phoenix instructions in transaction execution order, including instructions
/// that emit no events. Use the ordinal supplied by the transaction parser,
/// not a top-level instruction index or an event index.
#[derive(Debug, Clone)]
pub struct EventParserCtx {
    pub signature: String,
    pub top_level_instruction_index: u8,
    pub stack_height: Option<u32>,
    pub phoenix_instruction_index: i32,
    pub instruction_type: String,
}

/// Parsed PnL event tagged with the caller-supplied parser context and derived
/// fill ID needed to align parser output with Phoenix backend surfaces.
#[derive(Debug, Clone)]
pub struct ParsedPnlEvent {
    pub ctx: EventParserCtx,
    pub fill_id: Uuid,
    pub event: PnLEvent,
}

fn decode_signature_bytes(
    signature: &str,
) -> Result<[u8; SIGNATURE_BYTES_LEN], FillIdDerivationError> {
    let mut bytes = [0u8; SIGNATURE_BYTES_LEN];
    five8::decode_64(signature.as_bytes(), &mut bytes)
        .map_err(FillIdDerivationError::InvalidSignatureBase58)?;
    Ok(bytes)
}

fn decode_trader_pda_bytes(
    trader_pda: &str,
) -> Result<[u8; TRADER_PDA_BYTES_LEN], FillIdDerivationError> {
    let mut bytes = [0u8; TRADER_PDA_BYTES_LEN];
    five8::decode_32(trader_pda.as_bytes(), &mut bytes)
        .map_err(FillIdDerivationError::InvalidTraderPdaBase58)?;
    Ok(bytes)
}

/// Build the deterministic UUID v3 used for fill reconciliation from a
/// canonical byte encoding.
///
/// The UUID name payload is the concatenation of:
/// 1. the 64 raw signature bytes
/// 2. `instruction_index` as little-endian bytes
/// 3. the 32 raw trader PDA bytes
/// 4. `asset_id` as little-endian bytes
/// 5. `base_before_lots` as little-endian bytes
/// 6. `base_after_lots` as little-endian bytes
/// 7. `realized_pnl_quote_lots` as little-endian bytes
pub fn generate_fill_id_v3_from_bytes(
    signature: &[u8; SIGNATURE_BYTES_LEN],
    instruction_index: i32,
    trader_pda: &[u8; TRADER_PDA_BYTES_LEN],
    asset_id: u32,
    base_before_lots: i64,
    base_after_lots: i64,
    realized_pnl_quote_lots: i64,
) -> Uuid {
    let mut name = Vec::with_capacity(
        SIGNATURE_BYTES_LEN
            + std::mem::size_of::<i32>()
            + TRADER_PDA_BYTES_LEN
            + std::mem::size_of::<u32>()
            + std::mem::size_of::<i64>() * 3,
    );
    name.extend_from_slice(signature);
    name.extend_from_slice(&instruction_index.to_le_bytes());
    name.extend_from_slice(trader_pda);
    name.extend_from_slice(&asset_id.to_le_bytes());
    name.extend_from_slice(&base_before_lots.to_le_bytes());
    name.extend_from_slice(&base_after_lots.to_le_bytes());
    name.extend_from_slice(&realized_pnl_quote_lots.to_le_bytes());
    Uuid::new_v3(&FILL_ID_NAMESPACE, &name)
}

/// Derive a fill ID from base58 signature and trader account strings.
///
/// The instruction index is the emitting instruction's Phoenix ordinal, not
/// its top-level transaction index. All quantities use their on-chain lot
/// units.
pub fn generate_fill_id_v3(
    signature: &str,
    instruction_index: i32,
    trader_pda: &str,
    asset_id: u32,
    base_before_lots: i64,
    base_after_lots: i64,
    realized_pnl_quote_lots: i64,
) -> Result<Uuid, FillIdDerivationError> {
    let signature = decode_signature_bytes(signature)?;
    let trader_pda = decode_trader_pda_bytes(trader_pda)?;

    Ok(generate_fill_id_v3_from_bytes(
        &signature,
        instruction_index,
        &trader_pda,
        asset_id,
        base_before_lots,
        base_after_lots,
        realized_pnl_quote_lots,
    ))
}

/// Derive the Phoenix API fill ID for a parsed PnL event.
///
/// `instruction_index` must be the Phoenix ordinal of the instruction that
/// emitted this event. `Log` and `LogEventLengths` do not receive ordinals.
/// Nested Phoenix invocations each have their own ordinal.
pub fn generate_fill_id_from_pnl(
    signature: &str,
    instruction_index: i32,
    pnl_event: &PnLEvent,
) -> Result<Uuid, FillIdDerivationError> {
    let signature = decode_signature_bytes(signature)?;
    let trader_pda = pnl_event.trader;

    Ok(generate_fill_id_v3_from_bytes(
        &signature,
        instruction_index,
        &trader_pda,
        pnl_event.asset_id,
        pnl_event.base_lots_before.as_inner(),
        pnl_event.base_lots_after.as_inner(),
        pnl_event.realized_pnl.as_inner(),
    ))
}

/// Filter market events down to `PnLEvent`s and attach the derived fill ID to
/// each one using the supplied parser context.
///
/// The caller is responsible for supplying the transaction signature and the
/// instruction metadata corresponding to this event batch.
pub fn collect_parsed_pnls_with_fill_id(
    ctx: &EventParserCtx,
    events: &[MarketEvent],
) -> Result<Vec<ParsedPnlEvent>, FillIdDerivationError> {
    events
        .iter()
        .filter_map(|event| match event {
            MarketEvent::PnL(pnl_event) => Some(*pnl_event),
            _ => None,
        })
        .map(|pnl_event| {
            Ok(ParsedPnlEvent {
                ctx: ctx.clone(),
                fill_id: generate_fill_id_from_pnl(
                    &ctx.signature,
                    ctx.phoenix_instruction_index,
                    &pnl_event,
                )?,
                event: pnl_event,
            })
        })
        .collect()
}
