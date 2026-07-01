use std::error::Error;
use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use clap::{Args, Subcommand, ValueEnum};
use phoenix_rise::accounts::owned::{
    AccountDeserialize, Mint, Orderbook, OrderbookHeader, SplineCollection, TokenAccount,
};
use phoenix_rise::accounts::prelude as account_views;
use phoenix_rise::core::events::{
    RpcInnerInstructionGroup, RpcInnerInstructionView, RpcInstructionView,
    parse_events_from_rpc_instructions_with_errors,
};
use phoenix_rise::ix::discriminants::{
    EmberInstruction, FlightInstruction, HawkeyeInstruction, PhoenixInstruction,
};
use serde::Serialize;
use serde_json::Value;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcTransactionConfig;
use solana_signature::Signature;
use solana_transaction_status_client_types::option_serializer::OptionSerializer;
use solana_transaction_status_client_types::{
    UiCompiledInstruction, UiInstruction, UiParsedInstruction, UiTransactionEncoding,
};

use crate::context::CommandCtx;
use crate::output::{print_json, sha256_hex};
use crate::util::parse_pubkey;

#[derive(Debug, Subcommand)]
pub enum RpcCommand {
    Account {
        /// Account pubkey to fetch and decode.
        #[arg(long)]
        address: String,

        /// Expected account layout.
        #[arg(long, value_enum)]
        account_type: RpcAccountType,
    },
    PerpAsset {
        /// PerpAssetMap account pubkey.
        #[arg(long)]
        perp_asset_map: String,

        /// Asset symbol to find in the PerpAssetMap.
        #[arg(long)]
        symbol: String,
    },
    /// Alias for the top-level parse-tx command.
    ParseTx(ParseTxArgs),
}

#[derive(Debug, Args)]
pub struct ParseTxArgs {
    /// Transaction signature to fetch from RPC.
    #[arg(long)]
    pub signature: String,

    /// Phoenix program id whose events should be parsed. Defaults to the active
    /// SDK program id.
    #[arg(long)]
    pub program_id: Option<String>,

    /// Include malformed event payloads and parser failures in the output.
    #[arg(long)]
    pub with_errors: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RpcAccountType {
    ConditionalOrders,
    GlobalConfiguration,
    Mint,
    Orderbook,
    OrderbookHeader,
    Permission,
    PerpAssetMap,
    SplineCollection,
    StopLosses,
    TokenAccount,
    Trader,
    WithdrawQueue,
    WithdrawQueueHeader,
}

impl RpcAccountType {
    fn name(self) -> &'static str {
        match self {
            Self::ConditionalOrders => "conditional-orders",
            Self::GlobalConfiguration => "global-configuration",
            Self::Mint => "mint",
            Self::Orderbook => "orderbook",
            Self::OrderbookHeader => "orderbook-header",
            Self::Permission => "permission",
            Self::PerpAssetMap => "perp-asset-map",
            Self::SplineCollection => "spline-collection",
            Self::StopLosses => "stop-losses",
            Self::TokenAccount => "token-account",
            Self::Trader => "trader",
            Self::WithdrawQueue => "withdraw-queue",
            Self::WithdrawQueueHeader => "withdraw-queue-header",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RpcAccountOutput {
    address: String,
    owner: String,
    lamports: u64,
    executable: bool,
    rent_epoch: u64,
    data_len: usize,
    data_base64: String,
    data_sha256: String,
    decoded_type: &'static str,
    decoded: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PerpAssetMetadataOutput {
    perp_asset_map: String,
    symbol: String,
    metadata: account_views::PerpAssetMetadata,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParsedTransactionOutput {
    signature: String,
    slot: u64,
    program_id: String,
    instruction_count: usize,
    parsed_instructions: Vec<ParsedInstructionOutput>,
    orphan_log_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParsedInstructionOutput {
    stack_path: Vec<u8>,
    program_id: String,
    instruction_tag: u64,
    instruction_name: Option<String>,
    data_base64: String,
    event_count: usize,
    events: Vec<ParsedMarketEventOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_event_bytes_base64: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    failures: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParsedMarketEventOutput {
    event_type: phoenix_rise::events::MarketEventType,
    event: phoenix_rise::events::MarketEvent,
}

struct OwnedInnerInstruction {
    program_id: Pubkey,
    data: Vec<u8>,
    stack_height: Option<u32>,
}

struct OwnedInnerInstructionGroup {
    index: u8,
    instructions: Vec<OwnedInnerInstruction>,
}

pub fn run(cmd: RpcCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        RpcCommand::Account {
            address,
            account_type,
        } => decode_rpc_account(&address, account_type, ctx),
        RpcCommand::PerpAsset {
            perp_asset_map,
            symbol,
        } => decode_perp_asset(&perp_asset_map, &symbol, ctx),
        RpcCommand::ParseTx(args) => parse_tx(args, ctx),
    }
}

pub fn parse_tx(args: ParseTxArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let signature = Signature::from_str(&args.signature)
        .map_err(|e| format!("invalid transaction signature '{}': {e}", args.signature))?;
    let program_id = args
        .program_id
        .as_deref()
        .map(parse_pubkey)
        .transpose()?
        .unwrap_or_else(|| *phoenix_rise::ix::PHOENIX_PROGRAM_ID);

    let client = RpcClient::new(ctx.rpc_url.clone());
    let response = client.get_transaction_with_config(
        &signature,
        RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    )?;

    let tx = response
        .transaction
        .transaction
        .decode()
        .ok_or_else(|| std::io::Error::other("failed to decode binary transaction payload"))?;
    let meta = response
        .transaction
        .meta
        .as_ref()
        .ok_or_else(|| std::io::Error::other("transaction metadata unavailable"))?;

    let account_keys = transaction_account_keys(tx.message.static_account_keys(), meta)?;
    let top_level_views = top_level_instruction_views(tx.message.instructions(), &account_keys)?;
    let owned_inner_groups = owned_inner_instruction_groups(meta, &account_keys)?;
    let inner_view_storage = owned_inner_groups
        .iter()
        .map(|group| {
            group
                .instructions
                .iter()
                .map(|ix| RpcInnerInstructionView {
                    program_id: ix.program_id.as_ref(),
                    data: &ix.data,
                    stack_height: ix.stack_height,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let inner_views = owned_inner_groups
        .iter()
        .zip(&inner_view_storage)
        .map(|(group, instructions)| RpcInnerInstructionGroup {
            index: group.index,
            instructions,
        })
        .collect::<Vec<_>>();

    let parsed = parse_events_from_rpc_instructions_with_errors(
        &top_level_views,
        &inner_views,
        program_id.as_ref(),
    );

    let mut parsed_instructions = Vec::with_capacity(parsed.instructions.len());
    for item in parsed.instructions {
        let data_base64 = BASE64_STANDARD.encode(item.instruction_data);
        let tag_bytes = item.instruction_tag.to_le_bytes();
        let instruction_name =
            decode_instruction_name(&program_id, item.instruction_tag, tag_bytes)
                .map(ToString::to_string);
        let events = item
            .events
            .into_iter()
            .map(|event| ParsedMarketEventOutput {
                event_type: event.event_type(),
                event,
            })
            .collect::<Vec<_>>();
        parsed_instructions.push(ParsedInstructionOutput {
            stack_path: item.stack_path,
            program_id: program_id.to_string(),
            instruction_tag: item.instruction_tag,
            instruction_name,
            data_base64,
            event_count: events.len(),
            events,
            skipped_event_bytes_base64: if args.with_errors {
                item.skipped_event_bytes
                    .into_iter()
                    .map(|bytes| BASE64_STANDARD.encode(bytes))
                    .collect()
            } else {
                Vec::new()
            },
            failures: if args.with_errors {
                item.failures
                    .into_iter()
                    .map(|failure| failure.error.to_string())
                    .collect()
            } else {
                Vec::new()
            },
        });
    }

    print_json(
        &ParsedTransactionOutput {
            signature: signature.to_string(),
            slot: response.slot,
            program_id: program_id.to_string(),
            instruction_count: parsed_instructions.len(),
            parsed_instructions,
            orphan_log_count: parsed.orphan_logs.len(),
        },
        ctx.output,
    )
}

fn decode_rpc_account(
    address: &str,
    account_type: RpcAccountType,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let client = RpcClient::new(ctx.rpc_url.clone());
    let address = parse_pubkey(address)?;
    let account = client.get_account(&address)?;
    let decoded = decode_account(&account.data, account_type)?;
    let response = RpcAccountOutput {
        address: address.to_string(),
        owner: account.owner.to_string(),
        lamports: account.lamports,
        executable: account.executable,
        rent_epoch: account.rent_epoch,
        data_len: account.data.len(),
        data_base64: BASE64_STANDARD.encode(&account.data),
        data_sha256: sha256_hex(&account.data),
        decoded_type: account_type.name(),
        decoded,
    };
    print_json(&response, ctx.output)
}

fn decode_perp_asset(
    perp_asset_map: &str,
    symbol: &str,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let client = RpcClient::new(ctx.rpc_url.clone());
    let address = parse_pubkey(perp_asset_map)?;
    let account = client.get_account(&address)?;
    let map = account_views::PerpAssetMap::try_from_account_bytes(&account.data)?;
    let symbol = symbol.to_ascii_uppercase();
    let entry = map
        .find_by_symbol(&symbol)?
        .ok_or_else(|| format!("symbol {symbol} not found in PerpAssetMap account {address}"))?;
    let response = PerpAssetMetadataOutput {
        perp_asset_map: address.to_string(),
        symbol,
        metadata: entry.metadata,
    };
    print_json(&response, ctx.output)
}

fn decode_account(data: &[u8], account_type: RpcAccountType) -> Result<Value, Box<dyn Error>> {
    match account_type {
        RpcAccountType::ConditionalOrders => to_json_value(
            account_views::ConditionalOrders::try_from_account_bytes(data)?,
        ),
        RpcAccountType::GlobalConfiguration => {
            to_json_value(account_views::GlobalConfig::try_from_account_bytes(data)?)
        }
        RpcAccountType::Mint => decode_account_value::<Mint>(data),
        RpcAccountType::Orderbook => decode_account_value::<Orderbook>(data),
        RpcAccountType::OrderbookHeader => decode_account_value::<OrderbookHeader>(data),
        RpcAccountType::Permission => to_json_value(
            account_views::PermissionAccount::try_from_account_bytes(data)?,
        ),
        RpcAccountType::PerpAssetMap => {
            to_json_value(account_views::PerpAssetMap::try_from_account_bytes(data)?)
        }
        RpcAccountType::SplineCollection => decode_account_value::<SplineCollection>(data),
        RpcAccountType::StopLosses => {
            to_json_value(account_views::StopLosses::try_from_account_bytes(data)?)
        }
        RpcAccountType::TokenAccount => decode_account_value::<TokenAccount>(data),
        RpcAccountType::Trader => {
            to_json_value(account_views::Trader::try_from_account_bytes(data)?)
        }
        RpcAccountType::WithdrawQueue => {
            to_json_value(account_views::WithdrawQueue::try_from_account_bytes(data)?)
        }
        RpcAccountType::WithdrawQueueHeader => to_json_value(
            account_views::WithdrawQueueHeader::try_from_account_bytes(data)?,
        ),
    }
}

fn decode_account_value<T>(data: &[u8]) -> Result<Value, Box<dyn Error>>
where
    T: AccountDeserialize + Serialize,
{
    Ok(serde_json::to_value(T::try_from_account_bytes(data)?)?)
}

fn to_json_value<T>(value: T) -> Result<Value, Box<dyn Error>>
where
    T: Serialize,
{
    Ok(serde_json::to_value(value)?)
}

fn transaction_account_keys(
    static_keys: &[Pubkey],
    meta: &solana_transaction_status_client_types::UiTransactionStatusMeta,
) -> Result<Vec<Pubkey>, Box<dyn Error>> {
    let mut account_keys = static_keys.to_vec();
    if let OptionSerializer::Some(loaded) = &meta.loaded_addresses {
        for address in loaded.writable.iter().chain(loaded.readonly.iter()) {
            account_keys.push(parse_pubkey(address)?);
        }
    }
    Ok(account_keys)
}

fn top_level_instruction_views<'a>(
    instructions: &'a [solana_message::compiled_instruction::CompiledInstruction],
    account_keys: &'a [Pubkey],
) -> Result<Vec<RpcInstructionView<'a>>, Box<dyn Error>> {
    instructions
        .iter()
        .map(|ix| {
            let program_id = account_keys
                .get(ix.program_id_index as usize)
                .ok_or_else(|| {
                    format!(
                        "instruction references missing program id index {}",
                        ix.program_id_index
                    )
                })?;
            Ok(RpcInstructionView {
                program_id: program_id.as_ref(),
                data: &ix.data,
            })
        })
        .collect()
}

fn owned_inner_instruction_groups(
    meta: &solana_transaction_status_client_types::UiTransactionStatusMeta,
    account_keys: &[Pubkey],
) -> Result<Vec<OwnedInnerInstructionGroup>, Box<dyn Error>> {
    let OptionSerializer::Some(groups) = &meta.inner_instructions else {
        return Ok(Vec::new());
    };

    groups
        .iter()
        .map(|group| {
            let mut instructions = Vec::with_capacity(group.instructions.len());
            for ix in &group.instructions {
                if let Some(inner) = owned_inner_instruction(ix, account_keys)? {
                    instructions.push(inner);
                }
            }
            Ok(OwnedInnerInstructionGroup {
                index: group.index,
                instructions,
            })
        })
        .collect()
}

fn owned_inner_instruction(
    ix: &UiInstruction,
    account_keys: &[Pubkey],
) -> Result<Option<OwnedInnerInstruction>, Box<dyn Error>> {
    match ix {
        UiInstruction::Compiled(compiled) => {
            owned_compiled_inner_instruction(compiled, account_keys)
        }
        UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(partial)) => {
            Ok(Some(OwnedInnerInstruction {
                program_id: parse_pubkey(&partial.program_id)?,
                data: bs58::decode(&partial.data).into_vec()?,
                stack_height: partial.stack_height,
            }))
        }
        UiInstruction::Parsed(UiParsedInstruction::Parsed(_)) => Ok(None),
    }
}

fn owned_compiled_inner_instruction(
    compiled: &UiCompiledInstruction,
    account_keys: &[Pubkey],
) -> Result<Option<OwnedInnerInstruction>, Box<dyn Error>> {
    let Some(program_id) = account_keys.get(compiled.program_id_index as usize) else {
        return Ok(None);
    };
    Ok(Some(OwnedInnerInstruction {
        program_id: *program_id,
        data: bs58::decode(&compiled.data).into_vec()?,
        stack_height: compiled.stack_height,
    }))
}

fn decode_instruction_name(
    program_id: &Pubkey,
    tag: u64,
    discriminant: [u8; 8],
) -> Option<&'static str> {
    if program_id == &*phoenix_rise::ix::PHOENIX_PROGRAM_ID {
        return PhoenixInstruction::instruction_name_from_discriminant(discriminant);
    }
    if program_id == &phoenix_rise::ix::EMBER_PROGRAM_ID {
        return EmberInstruction::instruction_name_from_discriminant(discriminant);
    }
    if program_id == &phoenix_rise::ix::HAWKEYE_PROGRAM_ID {
        return HawkeyeInstruction::instruction_name_from_discriminant(discriminant);
    }
    FlightInstruction::from_tag(tag).map(FlightInstruction::instruction_name)
}
