//! Phoenix market event parsing from `LogEventLengths` and `Log` instruction
//! batches.
//!
//! Use [`parse`] when indexing should skip malformed payloads and return only
//! decoded [`MarketEvent`] values. Use [`parse_with_errors`] when you need the
//! skipped byte slices and parse failures for fixture analysis, replay tests,
//! or alerting. Higher-level adapters for RPC transaction responses live behind
//! the `events` feature in `phoenix-rise-core`.

use std::collections::{BTreeMap, VecDeque};

use borsh::BorshDeserialize;
use thiserror::Error;

pub mod market_events;
pub use market_events::{
    MarketEvent, MarketEventType, OffChainMarketEvent, OffChainMarketEventLengths,
};

/// 8-byte little-endian tag for `PhoenixInstruction::Log`.
pub const LOG_INSTRUCTION_TAG: u64 = 0xaacfd109f2d6e68d;
/// 8-byte little-endian tag for `PhoenixInstruction::LogEventLengths`.
pub const LOG_EVENT_LENGTHS_INSTRUCTION_TAG: u64 = 0x479947b5cb8607f7;

/// `Log` instruction data starts with two little-endian `u32`s:
/// `batch_index` and `total_events`.
pub const LOG_HEADER_LEN: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhoenixLogInstructionKind {
    Log,
    LogEventLengths,
}

impl PhoenixLogInstructionKind {
    #[inline]
    pub const fn from_tag(tag: u64) -> Option<Self> {
        match tag {
            LOG_INSTRUCTION_TAG => Some(Self::Log),
            LOG_EVENT_LENGTHS_INSTRUCTION_TAG => Some(Self::LogEventLengths),
            _ => None,
        }
    }

    #[inline]
    pub const fn tag(self) -> u64 {
        match self {
            Self::Log => LOG_INSTRUCTION_TAG,
            Self::LogEventLengths => LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
        }
    }

    #[inline]
    pub const fn discriminant(self) -> [u8; 8] {
        self.tag().to_le_bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhoenixLogInstruction<'a> {
    pub kind: PhoenixLogInstructionKind,
    /// Instruction data after the 8-byte instruction discriminant.
    pub data: &'a [u8],
}

impl<'a> PhoenixLogInstruction<'a> {
    #[inline]
    pub const fn new(kind: PhoenixLogInstructionKind, data: &'a [u8]) -> Self {
        Self { kind, data }
    }

    pub fn from_parts(tag: u64, data: &'a [u8]) -> Option<Self> {
        Some(Self {
            kind: PhoenixLogInstructionKind::from_tag(tag)?,
            data,
        })
    }

    pub fn from_instruction_data(data: &'a [u8]) -> Option<Self> {
        let (tag_bytes, payload) = data.split_first_chunk::<8>()?;
        let tag = u64::from_le_bytes(*tag_bytes);
        Self::from_parts(tag, payload)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum LogParseError {
    #[error("Log payload too short: {actual_len} bytes, expected at least {min_len}")]
    LogTooShort { actual_len: usize, min_len: usize },
    #[error("LogEventLengths payload too short: {actual_len} bytes, expected at least {min_len}")]
    LengthsTooShort { actual_len: usize, min_len: usize },
    #[error(
        "LogEventLengths payload has trailing or missing bytes: actual {actual_len}, expected \
         {expected_len}"
    )]
    InvalidLengthsByteCount {
        actual_len: usize,
        expected_len: usize,
    },
    #[error("Log batch {batch_index} had no matching LogEventLengths batch")]
    MissingLengths { batch_index: u32 },
    #[error("event {event_index} in batch {batch_index} exceeded Log payload length")]
    EventOutOfBounds {
        batch_index: u32,
        event_index: usize,
        offset: usize,
        event_len: usize,
        data_len: usize,
    },
    #[error("event {event_index} in batch {batch_index} failed to deserialize: {error}")]
    EventDecodeFailed {
        batch_index: u32,
        event_index: usize,
        error: String,
    },
    #[error("legacy Log batch failed to deserialize: {error}")]
    LegacyLogDecodeFailed { error: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogParseFailure<'a> {
    pub error: LogParseError,
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug, Default)]
pub struct ParsedLogEvents<'a> {
    pub events: Vec<MarketEvent>,
    /// Event payload byte slices that were length-delimited successfully but
    /// failed to deserialize as a `MarketEvent`.
    pub skipped_event_bytes: Vec<&'a [u8]>,
    pub failures: Vec<LogParseFailure<'a>>,
}

/// Parse Phoenix log instructions and return successfully decoded events.
///
/// This is the forgiving variant intended for indexing paths that should keep
/// moving when an individual event payload is malformed. Use
/// [`parse_with_errors`] when skipped payloads need to be inspected.
pub fn parse<'a, I>(instructions: I) -> Vec<MarketEvent>
where
    I: IntoIterator<Item = PhoenixLogInstruction<'a>>,
{
    parse_with_errors(instructions).events
}

/// Parse Phoenix `LogEventLengths` and `Log` batches.
///
/// Length batches are authoritative when present. If a group has `Log` batches
/// but no `LogEventLengths` batches at all, this falls back to the legacy
/// `OffChainMarketEvent` layout for those logs.
pub fn parse_with_errors<'a, I>(instructions: I) -> ParsedLogEvents<'a>
where
    I: IntoIterator<Item = PhoenixLogInstruction<'a>>,
{
    let instructions: Vec<_> = instructions.into_iter().collect();
    let has_log = instructions
        .iter()
        .any(|ix| ix.kind == PhoenixLogInstructionKind::Log);
    let has_lengths = instructions
        .iter()
        .any(|ix| ix.kind == PhoenixLogInstructionKind::LogEventLengths);

    let parsed = parse_with_lengths(&instructions);
    if has_log && !has_lengths && parsed.events.is_empty() {
        let legacy = parse_legacy(&instructions);
        if !legacy.events.is_empty() {
            return legacy;
        }
    }

    parsed
}

fn parse_with_lengths<'a>(instructions: &[PhoenixLogInstruction<'a>]) -> ParsedLogEvents<'a> {
    let mut lengths_by_batch = BTreeMap::<u32, VecDeque<Vec<usize>>>::new();
    let mut out = ParsedLogEvents::default();

    for instruction in instructions {
        match instruction.kind {
            PhoenixLogInstructionKind::LogEventLengths => {
                match parse_lengths_payload(instruction.data) {
                    Ok((batch_index, lengths)) => {
                        lengths_by_batch
                            .entry(batch_index)
                            .or_default()
                            .push_back(lengths);
                    }
                    Err(error) => out.failures.push(LogParseFailure {
                        error,
                        bytes: instruction.data,
                    }),
                }
            }
            PhoenixLogInstructionKind::Log => {
                parse_log_payload(instruction.data, &mut lengths_by_batch, &mut out);
            }
        }
    }

    out
}

fn parse_legacy<'a>(instructions: &[PhoenixLogInstruction<'a>]) -> ParsedLogEvents<'a> {
    let mut out = ParsedLogEvents::default();

    for instruction in instructions {
        if instruction.kind != PhoenixLogInstructionKind::Log {
            continue;
        }

        match OffChainMarketEvent::try_from_slice(instruction.data) {
            Ok(parsed) => out.events.extend(parsed.events),
            Err(error) => out.failures.push(LogParseFailure {
                error: LogParseError::LegacyLogDecodeFailed {
                    error: error.to_string(),
                },
                bytes: instruction.data,
            }),
        }
    }

    out
}

fn parse_log_payload<'a>(
    data: &'a [u8],
    lengths_by_batch: &mut BTreeMap<u32, VecDeque<Vec<usize>>>,
    out: &mut ParsedLogEvents<'a>,
) {
    let Some(header) = parse_log_header(data) else {
        out.failures.push(LogParseFailure {
            error: LogParseError::LogTooShort {
                actual_len: data.len(),
                min_len: LOG_HEADER_LEN,
            },
            bytes: data,
        });
        return;
    };

    let Some(lengths) = lengths_by_batch
        .get_mut(&header.batch_index)
        .and_then(VecDeque::pop_front)
    else {
        out.failures.push(LogParseFailure {
            error: LogParseError::MissingLengths {
                batch_index: header.batch_index,
            },
            bytes: data,
        });
        return;
    };

    let mut offset = LOG_HEADER_LEN;
    for (event_index, event_len) in lengths
        .iter()
        .copied()
        .enumerate()
        .take(header.total_events as usize)
    {
        let end = offset.saturating_add(event_len);
        if end > data.len() || end < offset {
            out.failures.push(LogParseFailure {
                error: LogParseError::EventOutOfBounds {
                    batch_index: header.batch_index,
                    event_index,
                    offset,
                    event_len,
                    data_len: data.len(),
                },
                bytes: data,
            });
            break;
        }

        let event_bytes = &data[offset..end];
        match MarketEvent::try_from_slice(event_bytes) {
            Ok(event) => out.events.push(event),
            Err(error) => {
                out.skipped_event_bytes.push(event_bytes);
                out.failures.push(LogParseFailure {
                    error: LogParseError::EventDecodeFailed {
                        batch_index: header.batch_index,
                        event_index,
                        error: error.to_string(),
                    },
                    bytes: event_bytes,
                });
            }
        }
        offset = end;
    }
}

fn parse_lengths_payload(data: &[u8]) -> Result<(u32, Vec<usize>), LogParseError> {
    let Some(header) = parse_log_header(data) else {
        return Err(LogParseError::LengthsTooShort {
            actual_len: data.len(),
            min_len: LOG_HEADER_LEN,
        });
    };

    let expected_len = LOG_HEADER_LEN + header.total_events as usize * core::mem::size_of::<u16>();
    if data.len() != expected_len {
        return Err(LogParseError::InvalidLengthsByteCount {
            actual_len: data.len(),
            expected_len,
        });
    }

    let lengths = data[LOG_HEADER_LEN..]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]) as usize)
        .collect();

    Ok((header.batch_index, lengths))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LogHeader {
    batch_index: u32,
    total_events: u32,
}

fn parse_log_header(data: &[u8]) -> Option<LogHeader> {
    let header = data.get(..LOG_HEADER_LEN)?;
    let batch_index = u32::from_le_bytes(header[0..4].try_into().ok()?);
    let total_events = u32::from_le_bytes(header[4..8].try_into().ok()?);
    Some(LogHeader {
        batch_index,
        total_events,
    })
}
