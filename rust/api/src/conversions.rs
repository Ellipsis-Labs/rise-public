//! Conversion utilities for building margin calculation types from
//! HTTP/WebSocket data.

use phoenix_rise_math::{
    MarketCalculator, PerpAssetMetadata, QuoteLots, SignedBaseLots, SignedQuoteLots,
};
use phoenix_rise_types::core::Decimal;

const MAX_PRICE_DECIMALS: i8 = 18;

pub fn decimal_from_quote_lots(val: QuoteLots) -> Decimal {
    Decimal::from_i64_with_decimals(val.as_inner() as i64, 6)
}

pub fn decimal_from_signed_quote_lots(val: SignedQuoteLots) -> Decimal {
    Decimal::from_i64_with_decimals(val.as_inner(), 6)
}

fn decimal_from_raw_lots(value: i128, base_lot_decimals: i8) -> Decimal {
    if base_lot_decimals >= 0 {
        let i64_value = value as i64;
        Decimal::from_i64_with_decimals(i64_value, base_lot_decimals)
    } else {
        let exponent = (-base_lot_decimals) as u32;
        let multiplier = 10i128.pow(exponent);
        let scaled = value.saturating_mul(multiplier);
        let i64_scaled = scaled as i64;
        Decimal::from_i64_with_decimals(i64_scaled, 0)
    }
}

pub fn decimal_from_signed_base_lots(base_lots: SignedBaseLots, base_lot_decimals: i8) -> Decimal {
    decimal_from_raw_lots(base_lots.as_inner() as i128, base_lot_decimals)
}

pub fn price_to_decimal(price: f64, calculator: &MarketCalculator) -> Decimal {
    let decimals = ((calculator.quote_lot_decimals as i32) - (calculator.base_lot_decimals as i32))
        .clamp(0, MAX_PRICE_DECIMALS as i32) as i8;
    let scale = 10f64.powi(decimals as i32);
    let scaled = price * scale;

    if !scaled.is_finite() || scaled > i64::MAX as f64 || scaled < i64::MIN as f64 {
        Decimal::from_i64_with_decimals(-1, 0)
    } else {
        Decimal::from_i64_with_decimals(scaled.round() as i64, decimals)
    }
}

pub fn calculator_for_metadata(metadata: &PerpAssetMetadata) -> MarketCalculator {
    MarketCalculator::new(metadata.base_lot_decimals(), metadata.tick_size())
}
