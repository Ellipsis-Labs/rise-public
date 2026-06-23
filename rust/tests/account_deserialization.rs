use std::fs;
use std::path::PathBuf;

use base64::Engine;
use phoenix_rise::types::accounts::{
    GlobalConfiguration, Orderbook, PerpAssetMap, SplineCollection, Trader,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    account: FixtureAccount,
}

#[derive(Debug, Deserialize)]
struct FixtureAccount {
    data: [String; 2],
}

fn mock_bytes(file_name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../ts/tests/mocks")
        .join(file_name);
    let fixture: FixtureFile =
        serde_json::from_str(&fs::read_to_string(path).expect("fixture should be readable"))
            .expect("fixture should parse");
    base64::engine::general_purpose::STANDARD
        .decode(&fixture.account.data[0])
        .expect("fixture should contain base64 account bytes")
}

#[test]
fn decodes_global_configuration_fixture() {
    let config = GlobalConfiguration::try_from_account_bytes(&mock_bytes("global_config.json"))
        .expect("global config should decode");

    assert_eq!(config.quote_decimals, 6);
    assert_eq!(config.exchange_status, 131);
    assert_eq!(
        config.perp_asset_map_key.to_string(),
        "2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC"
    );
}

#[test]
fn decodes_perp_asset_map_fixture() {
    let map = PerpAssetMap::try_from_account_bytes(&mock_bytes("perp_asset_map.json"))
        .expect("perp asset map should decode");

    assert_eq!(map.metadata.len, 12);
    assert!(map.get_by_symbol("SOL").is_some());
    let sol = map.get_by_symbol("SOL").expect("SOL metadata should exist");
    assert_eq!(sol.static_market_params.base_lot_decimals, 2);
    assert_eq!(sol.asset_flags.bits, 0);
    assert!(!sol.asset_flags.is_commodity);
    assert!(!sol.asset_flags.is_commodities_reopen);
    assert!(!sol.asset_flags.is_commodities_after_hours);
    assert_eq!(sol.commodities_after_hours_radius, 0);
    assert_eq!(sol.last_known_index_price, None);
    assert_eq!(sol.last_index_expiry_timestamp, 0);
    assert_eq!(sol.commodities_after_hours_radius_bps, 0);
}

#[test]
fn decodes_trader_fixture() {
    let trader =
        Trader::try_from_account_bytes(&mock_bytes("trader.json")).expect("trader should decode");

    assert_eq!(trader.state.flags, 62);
    assert_eq!(trader.state.quote_lot_collateral, 27_656_754);
    assert_eq!(trader.positions.len, 1);
}

#[test]
fn decodes_orderbook_fixture() {
    let orderbook = Orderbook::try_from_account_bytes(&mock_bytes("orderbook.json"))
        .expect("orderbook should decode");

    assert_eq!(orderbook.header.asset_symbol, "TAO");
    assert_eq!(orderbook.header.asset_id, 11);
    assert!(orderbook.bids.is_empty());
    assert!(orderbook.asks.is_empty());
}

#[test]
fn decodes_spline_collection_fixture() {
    let splines = SplineCollection::try_from_account_bytes(&mock_bytes("splines.json"))
        .expect("spline collection should decode");

    assert_eq!(splines.asset_symbol, "TAO");
    assert_eq!(splines.num_splines, 1);
    assert_eq!(splines.splines.len(), 1);
}
