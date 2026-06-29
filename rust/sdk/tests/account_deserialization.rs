use std::fs;
use std::path::PathBuf;

use base64::Engine;
use phoenix_rise::accounts::owned::{
    GlobalConfiguration, Orderbook, PerpAssetMapOwned, SplineCollection as OwnedSplineCollection,
    Trader,
};
use phoenix_rise::accounts::spline_collection::SplineCollection as SplineCollectionView;
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
        .join("../../ts/tests/mocks")
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
    let map = PerpAssetMapOwned::try_from_account_bytes(&mock_bytes("perp_asset_map.json"))
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
    let bytes = mock_bytes("splines.json");
    let splines = OwnedSplineCollection::try_from_account_bytes(&bytes)
        .expect("spline collection should decode");

    assert_eq!(splines.asset_symbol, "TAO");
    assert_eq!(splines.num_splines, 1);
    assert_eq!(splines.splines.len(), 1);

    let spline_view = SplineCollectionView::try_from_account_bytes(&bytes)
        .expect("spline collection view should decode");
    assert_eq!(spline_view.asset_symbol().as_str(), "TAO");
    assert_eq!(spline_view.num_splines(), 1);
    assert_eq!(spline_view.spline_capacity(), 1);

    let first_spline = spline_view
        .iter()
        .next()
        .expect("spline row should exist")
        .expect("spline row should decode");
    assert_eq!(first_spline.trader(), splines.splines[0].trader.to_bytes());
    assert_eq!(first_spline.mid_price(), splines.splines[0].mid_price);
    assert_eq!(
        first_spline.user_price_sequence_number(),
        splines.splines[0].user_price_sequence_number
    );
    assert_eq!(
        first_spline.user_parameter_sequence_number(),
        splines.splines[0].user_parameter_sequence_number
    );
}
