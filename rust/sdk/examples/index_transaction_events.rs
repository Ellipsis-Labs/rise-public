//! Index Phoenix events from a normalized executed-instruction stream.
//!
//! Run the offline demo:
//!   cargo run -p phoenix-rise --example index_transaction_events --features
//! events,serde
//!
//! Or read one transaction from JSON (see docs/event-indexing.md):
//!   cargo run -p phoenix-rise --example index_transaction_events --features
//! events,serde -- transaction.json

use std::error::Error;
use std::io;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use phoenix_rise::core::events::{
    ExecutedInstructionView, parse_events_from_executed_instructions_with_errors,
};
use phoenix_rise::events::fill_id::generate_fill_id_from_pnl;
use phoenix_rise::events::market_events::{PnLEvent, SlotContextEvent};
use phoenix_rise::events::{
    MarketEvent, OffChainMarketEvent, OffChainMarketEventLengths, PhoenixLogInstructionKind,
};
use phoenix_rise::ix::PHOENIX_PROGRAM_ID;
use phoenix_rise::ix::discriminants::PhoenixInstruction;
use serde::Deserialize;
use serde_json::json;
use solana_pubkey::Pubkey;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    signature: String,
    slot: u64,
    succeeded: bool,
    #[serde(deserialize_with = "deserialize_pubkey")]
    phoenix_program_id: Pubkey,
    instructions: Vec<ExecutedInstruction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecutedInstruction {
    #[serde(deserialize_with = "deserialize_pubkey")]
    program_id: Pubkey,
    /// Full instruction bytes, including the eight-byte discriminant.
    data_base64: String,
    /// Top-level index followed by sibling indices at each CPI depth.
    stack_path: Vec<u8>,
}

fn deserialize_pubkey<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Pubkey, D::Error> {
    String::deserialize(deserializer)?
        .parse()
        .map_err(serde::de::Error::custom)
}

fn main() -> Result<(), Box<dyn Error>> {
    let transaction = if let Some(path) = std::env::args_os().nth(1) {
        serde_json::from_reader(std::fs::File::open(path)?)?
    } else {
        eprintln!("Offline synthetic CPI demo; no transaction is sent or fetched.");
        demo_transaction()?
    };

    // Failed Solana transactions can still contain logs, but commit no state.
    if !transaction.succeeded {
        return Err(io::Error::other("refusing to index a failed transaction").into());
    }
    let data = transaction
        .instructions
        .iter()
        .map(|instruction| BASE64.decode(&instruction.data_base64))
        .collect::<Result<Vec<_>, _>>()?;
    let views = transaction
        .instructions
        .iter()
        .zip(&data)
        .map(|(instruction, data)| ExecutedInstructionView {
            program_id: instruction.program_id.as_ref(),
            data,
            stack_path: &instruction.stack_path,
        });
    let parsed = parse_events_from_executed_instructions_with_errors(
        views,
        transaction.phoenix_program_id.as_ref(),
    );

    // Do not publish a partial transaction or advance its checkpoint.
    if !parsed.orphan_logs.is_empty() {
        return Err(io::Error::other("unattached Phoenix log instructions").into());
    }
    for instruction in &parsed.instructions {
        if !instruction.failures.is_empty() {
            for failure in &instruction.failures {
                eprintln!(
                    "path={:?}: {}; bytes={}",
                    instruction.stack_path,
                    failure.error,
                    BASE64.encode(failure.bytes)
                );
            }
            return Err(
                io::Error::other("incomplete event decoding; retain input for replay").into(),
            );
        }
    }

    let mut rows = Vec::new();
    for instruction in parsed.instructions {
        let phoenix_instruction_index = i32::try_from(instruction.phoenix_instruction_index)?;
        for (event_index, event) in instruction.events.iter().enumerate() {
            let fill_id = match event {
                MarketEvent::PnL(pnl) => Some(
                    generate_fill_id_from_pnl(
                        &transaction.signature,
                        phoenix_instruction_index,
                        pnl,
                    )?
                    .to_string(),
                ),
                _ => None,
            };
            rows.push(json!({
                "signature": transaction.signature,
                "slot": transaction.slot,
                "instructionPath": instruction.stack_path,
                "phoenixInstructionIndex": phoenix_instruction_index,
                "eventIndex": event_index,
                "eventType": event.event_type(),
                "fillId": fill_id,
                "event": event,
            }));
        }
    }
    // Persist rows and a durable transaction checkpoint atomically in your
    // sink.
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

fn demo_transaction() -> Result<Transaction, Box<dyn Error>> {
    let events = vec![
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 1_700_000_000,
            slot: 42,
        }),
        MarketEvent::PnL(PnLEvent {
            trader: [1; 32],
            asset_id: 7,
            asset_symbol: Default::default(),
            realized_pnl: 12.into(),
            funding_payment: 0.into(),
            base_lots_before: 10.into(),
            base_lots_after: 5.into(),
            virtual_quote_lots_before: (-100).into(),
            virtual_quote_lots_after: (-50).into(),
        }),
    ];
    let lengths = OffChainMarketEventLengths {
        batch_index: 0,
        lengths: events
            .iter()
            .map(|event| Ok(u16::try_from(borsh::to_vec(event)?.len())?))
            .collect::<Result<_, Box<dyn Error>>>()?,
    };
    let log = OffChainMarketEvent {
        batch_index: 0,
        events,
    };
    let instruction = |tag: [u8; 8], payload: Vec<u8>, stack_path: Vec<u8>| ExecutedInstruction {
        program_id: *PHOENIX_PROGRAM_ID,
        data_base64: BASE64.encode([tag.as_slice(), payload.as_slice()].concat()),
        stack_path,
    };
    Ok(Transaction {
        // A synthetic, base58-encoded 64-byte signature for fill-ID derivation.
        signature: "1".repeat(64),
        slot: 42,
        succeeded: true,
        phoenix_program_id: *PHOENIX_PROGRAM_ID,
        instructions: vec![
            instruction(
                PhoenixInstruction::CancelAll.discriminant(),
                vec![],
                vec![0],
            ),
            instruction(
                PhoenixLogInstructionKind::LogEventLengths.discriminant(),
                borsh::to_vec(&lengths)?,
                vec![0, 0],
            ),
            instruction(
                PhoenixLogInstructionKind::Log.discriminant(),
                borsh::to_vec(&log)?,
                vec![0, 1],
            ),
        ],
    })
}
