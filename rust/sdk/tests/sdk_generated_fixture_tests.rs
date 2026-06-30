use std::fs;
use std::path::PathBuf;

use base64::Engine;
use phoenix_rise::accounts::owned::{self as accounts, PerpAssetMapOwned};
use phoenix_rise::accounts::perp_asset_map::{
    AssetFlags, BookPriceComponent, FundingAccumulator, MarkPrice, OpenInterestParams, OracleData,
    PerpAssetMap as BorrowedPerpAssetMap, PerpAssetMetadata,
};
use phoenix_rise::ix::discriminants::{
    EmberInstruction, FlightInstruction, HawkeyeInstruction, PhoenixInstruction,
};
use serde::Deserialize;
use solana_pubkey::Pubkey;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountFixtureFile {
    accounts: Vec<AccountFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountFixture {
    name: String,
    account_type: String,
    data_base64: String,
    expected: AccountExpected,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "accountType")]
enum AccountExpected {
    PerpAssetMap(PerpAssetMapExpected),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpAssetMapExpected {
    sequence_number: SequenceNumberExpected,
    num_assets: u16,
    capacity: u64,
    symbols: Vec<String>,
    assets: Vec<PerpAssetExpected>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpAssetExpected {
    symbol: String,
    static_market_params: StaticMarketParamsExpected,
    price_sequence_number: SequenceNumberExpected,
    mark_price: MarkPriceExpected,
    funding_accumulator: FundingAccumulatorExpected,
    open_interest_params: OpenInterestParamsExpected,
    asset_flags: AssetFlagsExpected,
    commodities_after_hours_radius: String,
    last_known_index_price: Option<String>,
    last_index_expiry_timestamp: String,
    commodities_after_hours_radius_bps: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StaticMarketParamsExpected {
    market_account: String,
    tick_size: String,
    asset_id: u32,
    base_lot_decimals: i8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SequenceNumberExpected {
    sequence_number: String,
    last_update_slot: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkPriceExpected {
    price: TicksAtSlotExpected,
    spot_price_component: SpotPriceComponentExpected,
    perp_price_component: PerpPriceComponentExpected,
    book_price_component: BookPriceComponentExpected,
    oracle_parameters: OracleParametersExpected,
    oracle_responses: Vec<OracleResponseExpected>,
    mark_price_last_validated_slot: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotPriceComponentExpected {
    last_exchange_spot_price: Vec<TicksAtSlotExpected>,
    weight: String,
    stale_threshold: String,
    slot: String,
    mid_spot_diff_ema_ticks: String,
    mid_spot_diff_ema_ticks_dust: String,
    ema_period_slots: String,
    ema_diff_radius: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpPriceComponentExpected {
    last_exchange_perp_price: Vec<TicksAtSlotExpected>,
    weight: String,
    stale_threshold: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BookPriceComponentExpected {
    clamped_book_mid_price: TicksAtSlotExpected,
    weight: String,
    stale_threshold: String,
    book_price_radius: String,
    last_best_bid: TicksAtSlotExpected,
    last_best_ask: TicksAtSlotExpected,
    last_trade_price: TicksAtSlotExpected,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TicksAtSlotExpected {
    slot: String,
    ticks: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OracleParametersExpected {
    oracle_divergence_radius: u16,
    min_oracle_responses: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OracleResponseExpected {
    index: usize,
    oracle_pubkey: String,
    divergence_score: u8,
    last_spot_price: TicksAtSlotExpected,
    last_perp_price: TicksAtSlotExpected,
    last_updated_timestamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FundingAccumulatorExpected {
    acc: String,
    last_diff: String,
    cumulative_funding_rate: String,
    start_interval_timestamp: String,
    last_funding_update_timestamp: String,
    funding_interval_seconds: String,
    funding_period_seconds: String,
    max_funding_rate: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenInterestParamsExpected {
    open_interest: String,
    open_interest_cap: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetFlagsExpected {
    bits: u8,
    is_commodity: bool,
    is_commodities_reopen: bool,
    is_commodities_after_hours: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstructionFixtureFile {
    instructions: Vec<InstructionFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstructionFixture {
    enum_name: String,
    instruction_name: String,
    snake_case_name: String,
    preimage: String,
    tag: String,
    discriminator_hex: String,
    discriminator_base64: String,
}

fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../litesvm-test/test-fixtures")
        .join(file_name)
}

fn account_fixtures() -> AccountFixtureFile {
    serde_json::from_str(
        &fs::read_to_string(fixture_path("sdk-account-fixtures.json"))
            .expect("generated account fixture should be readable"),
    )
    .expect("generated account fixture should parse")
}

fn instruction_fixtures() -> InstructionFixtureFile {
    serde_json::from_str(
        &fs::read_to_string(fixture_path("sdk-instruction-fixtures.json"))
            .expect("generated instruction fixture should be readable"),
    )
    .expect("generated instruction fixture should parse")
}

#[test]
fn generated_perp_asset_map_account_fixture_matches_rise_decoders() {
    let fixture = account_fixtures()
        .accounts
        .into_iter()
        .find(|account| account.account_type == "PerpAssetMap")
        .expect("PerpAssetMap fixture should exist");
    assert_eq!(fixture.name, "mainnetPerpAssetMap");
    let data = base64::engine::general_purpose::STANDARD
        .decode(&fixture.data_base64)
        .expect("account data should be base64");
    let AccountExpected::PerpAssetMap(expected) = fixture.expected;

    let borrowed = BorrowedPerpAssetMap::try_from_account_bytes(&data)
        .expect("borrowed PerpAssetMap should decode");
    let owned =
        PerpAssetMapOwned::try_from_account_bytes(&data).expect("owned PerpAssetMap should decode");

    assert_sequence_number(owned.sequence_number, &expected.sequence_number);
    assert_eq!(owned.num_assets, expected.num_assets);
    assert_eq!(owned.metadata.capacity, expected.capacity);
    assert_eq!(borrowed.num_assets(), expected.num_assets);
    assert_eq!(borrowed.capacity(), expected.capacity);
    assert_eq!(expected.symbols.len(), expected.assets.len());
    assert_eq!(
        owned
            .metadata
            .entries
            .iter()
            .map(|(symbol, _)| symbol.as_str())
            .collect::<Vec<_>>(),
        expected
            .symbols
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );

    for expected_asset in &expected.assets {
        let owned_asset = owned
            .get_by_symbol(&expected_asset.symbol)
            .unwrap_or_else(|| panic!("owned map missing {}", expected_asset.symbol));
        assert_owned_asset(owned_asset, expected_asset);

        let borrowed_asset = borrowed
            .iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("borrowed entries should decode")
            .into_iter()
            .find(|entry| entry.symbol.as_str() == expected_asset.symbol)
            .unwrap_or_else(|| panic!("borrowed map missing {}", expected_asset.symbol));
        assert_borrowed_asset(borrowed_asset.metadata, expected_asset);
    }
}

#[test]
fn generated_instruction_fixtures_match_rise_discriminants() {
    for fixture in instruction_fixtures().instructions {
        let (snake_case_name, tag, discriminant) = match fixture.enum_name.as_str() {
            "PhoenixInstruction" => {
                let instruction =
                    PhoenixInstruction::from_instruction_name(&fixture.instruction_name)
                        .expect("Phoenix instruction should exist");
                (
                    instruction.snake_case_instruction_name(),
                    instruction.tag(),
                    instruction.discriminant(),
                )
            }
            "EmberInstruction" => {
                let instruction =
                    EmberInstruction::from_instruction_name(&fixture.instruction_name)
                        .expect("Ember instruction should exist");
                (
                    instruction.snake_case_instruction_name(),
                    instruction.tag(),
                    instruction.discriminant(),
                )
            }
            "FlightInstruction" => {
                let instruction =
                    FlightInstruction::from_instruction_name(&fixture.instruction_name)
                        .expect("Flight instruction should exist");
                (
                    instruction.snake_case_instruction_name(),
                    instruction.tag(),
                    instruction.discriminant(),
                )
            }
            "HawkeyeInstruction" => {
                let instruction =
                    HawkeyeInstruction::from_instruction_name(&fixture.instruction_name)
                        .expect("Hawkeye instruction should exist");
                (
                    instruction.snake_case_instruction_name(),
                    instruction.tag(),
                    instruction.discriminant(),
                )
            }
            other => panic!("unexpected enum name {other}"),
        };

        assert_eq!(snake_case_name, fixture.snake_case_name);
        assert_eq!(format!("global:{snake_case_name}"), fixture.preimage);
        assert_eq!(tag.to_string(), fixture.tag);
        assert_eq!(hex_lower(discriminant), fixture.discriminator_hex);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(fixture.discriminator_base64)
                .expect("discriminator base64 should decode"),
            discriminant
        );
    }
}

fn assert_owned_asset(actual: &accounts::PerpAssetMetadata, expected: &PerpAssetExpected) {
    assert_static_market_params(
        &actual.static_market_params.market_account.to_string(),
        actual.static_market_params.tick_size,
        actual.static_market_params.asset_id,
        actual.static_market_params.base_lot_decimals,
        &expected.static_market_params,
    );
    assert_sequence_number(
        actual.oracle_price.price_sequence_number,
        &expected.price_sequence_number,
    );
    assert_owned_mark_price(&actual.oracle_price.mark_price, &expected.mark_price);
    assert_funding_accumulator(actual.funding_accumulator, &expected.funding_accumulator);
    assert_open_interest(actual.open_interest_params, &expected.open_interest_params);
    assert_asset_flags(actual.asset_flags, &expected.asset_flags);
    assert_eq!(
        actual.commodities_after_hours_radius.to_string(),
        expected.commodities_after_hours_radius
    );
    assert_eq!(
        actual.last_known_index_price.map(|value| value.to_string()),
        expected.last_known_index_price
    );
    assert_eq!(
        actual.last_index_expiry_timestamp.to_string(),
        expected.last_index_expiry_timestamp
    );
    assert_eq!(
        actual.commodities_after_hours_radius_bps.to_string(),
        expected.commodities_after_hours_radius_bps
    );
}

fn assert_borrowed_asset(actual: PerpAssetMetadata, expected: &PerpAssetExpected) {
    let static_market_params = actual.static_market_params();
    assert_static_market_params(
        &Pubkey::new_from_array(static_market_params.market_account).to_string(),
        static_market_params.tick_size,
        static_market_params.asset_id(),
        static_market_params.base_lot_decimals,
        &expected.static_market_params,
    );
    assert_sequence_number(
        actual.oracle_price().price_sequence_number,
        &expected.price_sequence_number,
    );
    assert_borrowed_mark_price(&actual.oracle_price().mark_price, &expected.mark_price);
    assert_funding_accumulator(actual.funding_accumulator(), &expected.funding_accumulator);
    assert_open_interest(
        actual.open_interest_params(),
        &expected.open_interest_params,
    );
    assert_asset_flags(actual.asset_flags(), &expected.asset_flags);
    assert_eq!(
        actual.commodities_after_hours_radius().to_string(),
        expected.commodities_after_hours_radius
    );
    assert_eq!(
        actual
            .last_known_index_price()
            .map(|value| value.to_string()),
        expected.last_known_index_price
    );
    assert_eq!(
        actual.last_index_expiry_timestamp().to_string(),
        expected.last_index_expiry_timestamp
    );
    assert_eq!(
        actual.commodities_after_hours_radius_bps().to_string(),
        expected.commodities_after_hours_radius_bps
    );
}

fn assert_static_market_params(
    actual_market_account: &str,
    actual_tick_size: impl ToString,
    actual_asset_id: u32,
    actual_base_lot_decimals: i8,
    expected: &StaticMarketParamsExpected,
) {
    assert_eq!(actual_market_account, expected.market_account);
    assert_eq!(actual_tick_size.to_string(), expected.tick_size);
    assert_eq!(actual_asset_id, expected.asset_id);
    assert_eq!(actual_base_lot_decimals, expected.base_lot_decimals);
}

fn assert_owned_mark_price(actual: &accounts::MarkPrice, expected: &MarkPriceExpected) {
    assert_ticks_at_slot(actual.price, &expected.price);
    assert_spot_price_component(
        &actual.spot_price_component.last_exchange_spot_price,
        actual.spot_price_component.weight,
        actual.spot_price_component.stale_threshold,
        actual.spot_price_component.slot,
        actual.spot_price_component.mid_spot_diff_ema_ticks,
        actual.spot_price_component.mid_spot_diff_ema_ticks_dust,
        actual.spot_price_component.ema_period_slots,
        actual.spot_price_component.ema_diff_radius,
        &expected.spot_price_component,
    );
    assert_perp_price_component(
        &actual.perp_price_component.last_exchange_perp_price,
        actual.perp_price_component.weight,
        actual.perp_price_component.stale_threshold,
        &expected.perp_price_component,
    );
    assert_book_price_component(&actual.book_price_component, &expected.book_price_component);
    assert_eq!(
        actual.oracle_parameters.oracle_divergence_radius,
        expected.oracle_parameters.oracle_divergence_radius
    );
    assert_eq!(
        actual.oracle_parameters.min_oracle_responses,
        expected.oracle_parameters.min_oracle_responses
    );
    assert_oracle_responses(
        &actual.oracle_data,
        &actual.spot_price_component.last_exchange_spot_price,
        &actual.perp_price_component.last_exchange_perp_price,
        &actual.oracle_last_updated_timestamps,
        &expected.oracle_responses,
    );
    assert_string_vec(
        &actual.mark_price_last_validated_slot,
        &expected.mark_price_last_validated_slot,
    );
}

fn assert_borrowed_mark_price(actual: &MarkPrice, expected: &MarkPriceExpected) {
    assert_ticks_at_slot(actual.price, &expected.price);
    assert_spot_price_component(
        &actual.spot_price_component.last_exchange_spot_price,
        actual.spot_price_component.weight,
        actual.spot_price_component.stale_threshold,
        actual.spot_price_component.slot,
        actual.spot_price_component.mid_spot_diff_ema_ticks,
        actual.spot_price_component.mid_spot_diff_ema_ticks_dust,
        actual.spot_price_component.ema_period_slots,
        actual.spot_price_component.ema_diff_radius,
        &expected.spot_price_component,
    );
    assert_perp_price_component(
        &actual.perp_price_component.last_exchange_perp_price,
        actual.perp_price_component.weight,
        actual.perp_price_component.stale_threshold,
        &expected.perp_price_component,
    );
    assert_book_price_component(&actual.book_price_component, &expected.book_price_component);
    assert_eq!(
        actual.oracle_parameters().oracle_divergence_radius,
        expected.oracle_parameters.oracle_divergence_radius
    );
    assert_eq!(
        actual.oracle_parameters().min_oracle_responses,
        expected.oracle_parameters.min_oracle_responses
    );
    assert_oracle_responses(
        &actual.oracle_data,
        &actual.spot_price_component.last_exchange_spot_price,
        &actual.perp_price_component.last_exchange_perp_price,
        &actual.oracle_last_updated_timestamps,
        &expected.oracle_responses,
    );
    assert_string_vec(
        &actual.mark_price_last_validated_slot,
        &expected.mark_price_last_validated_slot,
    );
}

fn assert_spot_price_component<T: Copy + Into<accounts::TicksAtSlot>>(
    last_exchange_spot_price: &[T],
    weight: u64,
    stale_threshold: u64,
    slot: u64,
    mid_spot_diff_ema_ticks: impl ToString,
    mid_spot_diff_ema_ticks_dust: i64,
    ema_period_slots: u64,
    ema_diff_radius: u64,
    expected: &SpotPriceComponentExpected,
) {
    assert_ticks_at_slot_vec(last_exchange_spot_price, &expected.last_exchange_spot_price);
    assert_eq!(weight.to_string(), expected.weight);
    assert_eq!(stale_threshold.to_string(), expected.stale_threshold);
    assert_eq!(slot.to_string(), expected.slot);
    assert_eq!(
        mid_spot_diff_ema_ticks.to_string(),
        expected.mid_spot_diff_ema_ticks
    );
    assert_eq!(
        mid_spot_diff_ema_ticks_dust.to_string(),
        expected.mid_spot_diff_ema_ticks_dust
    );
    assert_eq!(ema_period_slots.to_string(), expected.ema_period_slots);
    assert_eq!(ema_diff_radius.to_string(), expected.ema_diff_radius);
}

fn assert_perp_price_component<T: Copy + Into<accounts::TicksAtSlot>>(
    last_exchange_perp_price: &[T],
    weight: u64,
    stale_threshold: u64,
    expected: &PerpPriceComponentExpected,
) {
    assert_ticks_at_slot_vec(last_exchange_perp_price, &expected.last_exchange_perp_price);
    assert_eq!(weight.to_string(), expected.weight);
    assert_eq!(stale_threshold.to_string(), expected.stale_threshold);
}

fn assert_book_price_component(
    actual: &impl BookPriceComponentView,
    expected: &BookPriceComponentExpected,
) {
    assert_ticks_at_slot(
        actual.clamped_book_mid_price(),
        &expected.clamped_book_mid_price,
    );
    assert_eq!(actual.weight().to_string(), expected.weight);
    assert_eq!(
        actual.stale_threshold().to_string(),
        expected.stale_threshold
    );
    assert_eq!(
        actual.book_price_radius().to_string(),
        expected.book_price_radius
    );
    assert_ticks_at_slot(actual.last_best_bid(), &expected.last_best_bid);
    assert_ticks_at_slot(actual.last_best_ask(), &expected.last_best_ask);
    assert_ticks_at_slot(actual.last_trade_price(), &expected.last_trade_price);
}

fn assert_oracle_responses<O, S, P>(
    oracle_data: &[O],
    spot_prices: &[S],
    perp_prices: &[P],
    timestamps: &[u64],
    expected: &[OracleResponseExpected],
) where
    O: OracleDataView,
    S: Copy + Into<accounts::TicksAtSlot>,
    P: Copy + Into<accounts::TicksAtSlot>,
{
    assert_eq!(oracle_data.len(), expected.len());
    assert_eq!(spot_prices.len(), expected.len());
    assert_eq!(perp_prices.len(), expected.len());
    assert_eq!(timestamps.len(), expected.len());
    for expected_response in expected {
        let index = expected_response.index;
        assert_eq!(
            oracle_data[index].oracle_pubkey_string(),
            expected_response.oracle_pubkey
        );
        assert_eq!(
            oracle_data[index].divergence_score(),
            expected_response.divergence_score
        );
        assert_ticks_at_slot(spot_prices[index], &expected_response.last_spot_price);
        assert_ticks_at_slot(perp_prices[index], &expected_response.last_perp_price);
        assert_eq!(
            timestamps[index].to_string(),
            expected_response.last_updated_timestamp
        );
    }
}

fn assert_funding_accumulator(
    actual: impl FundingAccumulatorView,
    expected: &FundingAccumulatorExpected,
) {
    assert_eq!(actual.acc().to_string(), expected.acc);
    assert_eq!(actual.last_diff().to_string(), expected.last_diff);
    assert_eq!(
        actual.cumulative_funding_rate().to_string(),
        expected.cumulative_funding_rate
    );
    assert_eq!(
        actual.start_interval_timestamp().to_string(),
        expected.start_interval_timestamp
    );
    assert_eq!(
        actual.last_funding_update_timestamp().to_string(),
        expected.last_funding_update_timestamp
    );
    assert_eq!(
        actual.funding_interval_seconds().to_string(),
        expected.funding_interval_seconds
    );
    assert_eq!(
        actual.funding_period_seconds().to_string(),
        expected.funding_period_seconds
    );
    assert_eq!(
        actual.max_funding_rate().to_string(),
        expected.max_funding_rate
    );
}

fn assert_open_interest(actual: impl OpenInterestView, expected: &OpenInterestParamsExpected) {
    assert_eq!(actual.open_interest().to_string(), expected.open_interest);
    assert_eq!(
        actual.open_interest_cap().to_string(),
        expected.open_interest_cap
    );
}

fn assert_asset_flags(actual: impl AssetFlagsView, expected: &AssetFlagsExpected) {
    assert_eq!(actual.bits(), expected.bits);
    assert_eq!(actual.is_commodity(), expected.is_commodity);
    assert_eq!(
        actual.is_commodities_reopen(),
        expected.is_commodities_reopen
    );
    assert_eq!(
        actual.is_commodities_after_hours(),
        expected.is_commodities_after_hours
    );
}

fn assert_sequence_number(
    actual: impl Into<accounts::SequenceNumber>,
    expected: &SequenceNumberExpected,
) {
    let actual = actual.into();
    assert_eq!(actual.sequence_number.to_string(), expected.sequence_number);
    assert_eq!(
        actual.last_update_slot.to_string(),
        expected.last_update_slot
    );
}

fn assert_ticks_at_slot(actual: impl Into<accounts::TicksAtSlot>, expected: &TicksAtSlotExpected) {
    let actual = actual.into();
    assert_eq!(actual.slot.to_string(), expected.slot);
    assert_eq!(actual.ticks.to_string(), expected.ticks);
}

fn assert_ticks_at_slot_vec<T: Copy + Into<accounts::TicksAtSlot>>(
    actual: &[T],
    expected: &[TicksAtSlotExpected],
) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_ticks_at_slot(*actual, expected);
    }
}

fn assert_string_vec(actual: &[u64], expected: &[String]) {
    assert_eq!(
        actual.iter().map(ToString::to_string).collect::<Vec<_>>(),
        expected
    );
}

fn hex_lower(bytes: [u8; 8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

trait BookPriceComponentView {
    fn clamped_book_mid_price(&self) -> accounts::TicksAtSlot;
    fn weight(&self) -> u64;
    fn stale_threshold(&self) -> u64;
    fn book_price_radius(&self) -> u64;
    fn last_best_bid(&self) -> accounts::TicksAtSlot;
    fn last_best_ask(&self) -> accounts::TicksAtSlot;
    fn last_trade_price(&self) -> accounts::TicksAtSlot;
}

impl BookPriceComponentView for accounts::BookPriceComponent {
    fn clamped_book_mid_price(&self) -> accounts::TicksAtSlot {
        self.clamped_book_mid_price
    }

    fn weight(&self) -> u64 {
        self.weight
    }

    fn stale_threshold(&self) -> u64 {
        self.stale_threshold
    }

    fn book_price_radius(&self) -> u64 {
        self.book_price_radius
    }

    fn last_best_bid(&self) -> accounts::TicksAtSlot {
        self.last_best_bid
    }

    fn last_best_ask(&self) -> accounts::TicksAtSlot {
        self.last_best_ask
    }

    fn last_trade_price(&self) -> accounts::TicksAtSlot {
        self.last_trade_price
    }
}

impl BookPriceComponentView for BookPriceComponent {
    fn clamped_book_mid_price(&self) -> accounts::TicksAtSlot {
        self.clamped_book_mid_price.into()
    }

    fn weight(&self) -> u64 {
        self.weight
    }

    fn stale_threshold(&self) -> u64 {
        self.stale_threshold
    }

    fn book_price_radius(&self) -> u64 {
        self.book_price_radius
    }

    fn last_best_bid(&self) -> accounts::TicksAtSlot {
        self.last_best_bid.into()
    }

    fn last_best_ask(&self) -> accounts::TicksAtSlot {
        self.last_best_ask.into()
    }

    fn last_trade_price(&self) -> accounts::TicksAtSlot {
        self.last_trade_price.into()
    }
}

trait OracleDataView {
    fn oracle_pubkey_string(&self) -> String;
    fn divergence_score(&self) -> u8;
}

impl OracleDataView for accounts::OracleData {
    fn oracle_pubkey_string(&self) -> String {
        self.oracle_pubkey.to_string()
    }

    fn divergence_score(&self) -> u8 {
        self.divergence_score
    }
}

impl OracleDataView for OracleData {
    fn oracle_pubkey_string(&self) -> String {
        Pubkey::new_from_array(self.oracle_pubkey).to_string()
    }

    fn divergence_score(&self) -> u8 {
        self.divergence_score
    }
}

trait FundingAccumulatorView {
    fn acc(&self) -> String;
    fn last_diff(&self) -> String;
    fn cumulative_funding_rate(&self) -> String;
    fn start_interval_timestamp(&self) -> String;
    fn last_funding_update_timestamp(&self) -> String;
    fn funding_interval_seconds(&self) -> String;
    fn funding_period_seconds(&self) -> String;
    fn max_funding_rate(&self) -> String;
}

impl FundingAccumulatorView for accounts::FundingAccumulator {
    fn acc(&self) -> String {
        self.acc.to_string()
    }

    fn last_diff(&self) -> String {
        self.last_diff.to_string()
    }

    fn cumulative_funding_rate(&self) -> String {
        self.cumulative_funding_rate.to_string()
    }

    fn start_interval_timestamp(&self) -> String {
        self.start_interval_timestamp.to_string()
    }

    fn last_funding_update_timestamp(&self) -> String {
        self.last_funding_update_timestamp.to_string()
    }

    fn funding_interval_seconds(&self) -> String {
        self.funding_interval_seconds.to_string()
    }

    fn funding_period_seconds(&self) -> String {
        self.funding_period_seconds.to_string()
    }

    fn max_funding_rate(&self) -> String {
        self.max_funding_rate.to_string()
    }
}

impl FundingAccumulatorView for FundingAccumulator {
    fn acc(&self) -> String {
        self.acc.to_string()
    }

    fn last_diff(&self) -> String {
        self.last_diff.to_string()
    }

    fn cumulative_funding_rate(&self) -> String {
        self.cumulative_funding_rate.to_string()
    }

    fn start_interval_timestamp(&self) -> String {
        self.start_interval_timestamp.to_string()
    }

    fn last_funding_update_timestamp(&self) -> String {
        self.last_funding_update_timestamp.to_string()
    }

    fn funding_interval_seconds(&self) -> String {
        self.funding_interval_seconds.to_string()
    }

    fn funding_period_seconds(&self) -> String {
        self.funding_period_seconds.to_string()
    }

    fn max_funding_rate(&self) -> String {
        self.max_funding_rate.to_string()
    }
}

impl<T: FundingAccumulatorView + ?Sized> FundingAccumulatorView for &T {
    fn acc(&self) -> String {
        (**self).acc()
    }

    fn last_diff(&self) -> String {
        (**self).last_diff()
    }

    fn cumulative_funding_rate(&self) -> String {
        (**self).cumulative_funding_rate()
    }

    fn start_interval_timestamp(&self) -> String {
        (**self).start_interval_timestamp()
    }

    fn last_funding_update_timestamp(&self) -> String {
        (**self).last_funding_update_timestamp()
    }

    fn funding_interval_seconds(&self) -> String {
        (**self).funding_interval_seconds()
    }

    fn funding_period_seconds(&self) -> String {
        (**self).funding_period_seconds()
    }

    fn max_funding_rate(&self) -> String {
        (**self).max_funding_rate()
    }
}

trait OpenInterestView {
    fn open_interest(&self) -> String;
    fn open_interest_cap(&self) -> String;
}

impl OpenInterestView for accounts::OpenInterestParams {
    fn open_interest(&self) -> String {
        self.open_interest.to_string()
    }

    fn open_interest_cap(&self) -> String {
        self.open_interest_cap.to_string()
    }
}

impl OpenInterestView for OpenInterestParams {
    fn open_interest(&self) -> String {
        self.open_interest.to_string()
    }

    fn open_interest_cap(&self) -> String {
        self.open_interest_cap.to_string()
    }
}

impl<T: OpenInterestView + ?Sized> OpenInterestView for &T {
    fn open_interest(&self) -> String {
        (**self).open_interest()
    }

    fn open_interest_cap(&self) -> String {
        (**self).open_interest_cap()
    }
}

trait AssetFlagsView {
    fn bits(&self) -> u8;
    fn is_commodity(&self) -> bool;
    fn is_commodities_reopen(&self) -> bool;
    fn is_commodities_after_hours(&self) -> bool;
}

impl AssetFlagsView for accounts::AssetFlags {
    fn bits(&self) -> u8 {
        self.bits
    }

    fn is_commodity(&self) -> bool {
        self.is_commodity
    }

    fn is_commodities_reopen(&self) -> bool {
        self.is_commodities_reopen
    }

    fn is_commodities_after_hours(&self) -> bool {
        self.is_commodities_after_hours
    }
}

impl AssetFlagsView for AssetFlags {
    fn bits(&self) -> u8 {
        self.bits
    }

    fn is_commodity(&self) -> bool {
        self.is_commodity
    }

    fn is_commodities_reopen(&self) -> bool {
        self.is_commodities_reopen
    }

    fn is_commodities_after_hours(&self) -> bool {
        self.is_commodities_after_hours
    }
}
