use borsh::BorshDeserialize;
use phoenix_rise_events::market_events::{
    BaseLots, LiquidationWithFeeEvent, MarketEvent, QuoteLots, Ticks,
};

fn event(
    liquidator_trader: Option<[u8; 32]>,
    liquidation_fee_bps: u16,
    filled_notional: u64,
    fee_charged: u64,
    fee_charged_liquidator_portion: u64,
) -> LiquidationWithFeeEvent {
    LiquidationWithFeeEvent {
        liquidator: [1; 32],
        liquidated_trader: [2; 32],
        asset_id: 7,
        liquidation_size: BaseLots::new(1_000),
        mark_price: Ticks::new(5_000),
        base_lots_filled: BaseLots::new(900),
        quote_lots_filled: QuoteLots::new(4_500_000),
        position_closed: true,
        liquidator_trader,
        liquidation_fee_bps,
        filled_notional: QuoteLots::new(filled_notional),
        fee_charged: QuoteLots::new(fee_charged),
        fee_charged_liquidator_portion: QuoteLots::new(fee_charged_liquidator_portion),
    }
}

/// Hand-built Borsh bytes for the matching `event`.
fn expected_bytes(
    liquidator_trader: Option<[u8; 32]>,
    liquidation_fee_bps: u16,
    filled_notional: u64,
    fee_charged: u64,
    fee_charged_liquidator_portion: u64,
) -> Vec<u8> {
    let mut bytes = vec![76];
    bytes.extend_from_slice(&[1; 32]);
    bytes.extend_from_slice(&[2; 32]);
    bytes.extend_from_slice(&7_u32.to_le_bytes());
    bytes.extend_from_slice(&1_000_u64.to_le_bytes());
    bytes.extend_from_slice(&5_000_u64.to_le_bytes());
    bytes.extend_from_slice(&900_u64.to_le_bytes());
    bytes.extend_from_slice(&4_500_000_u64.to_le_bytes());
    bytes.push(1);
    match liquidator_trader {
        Some(key) => {
            bytes.push(1);
            bytes.extend_from_slice(&key);
        }
        None => bytes.push(0),
    }
    bytes.extend_from_slice(&liquidation_fee_bps.to_le_bytes());
    bytes.extend_from_slice(&filled_notional.to_le_bytes());
    bytes.extend_from_slice(&fee_charged.to_le_bytes());
    bytes.extend_from_slice(&fee_charged_liquidator_portion.to_le_bytes());
    bytes
}

#[test]
fn liquidation_with_fee_event_matches_expected_layout_and_round_trips() {
    for (liquidator_trader, bps, notional, fee, liquidator_portion) in [
        (Some([3; 32]), 250, 1_000_000, 2_500, 1_875),
        (None, 0, 0, 0, 0),
    ] {
        let liquidation = event(liquidator_trader, bps, notional, fee, liquidator_portion);
        let bytes = borsh::to_vec(&MarketEvent::LiquidationWithFee(liquidation)).unwrap();
        assert_eq!(
            bytes,
            expected_bytes(liquidator_trader, bps, notional, fee, liquidator_portion)
        );
        match MarketEvent::try_from_slice(&bytes).unwrap() {
            MarketEvent::LiquidationWithFee(decoded) => assert_eq!(decoded, liquidation),
            other => panic!("expected LiquidationWithFee, got {other:?}"),
        }
    }
}
