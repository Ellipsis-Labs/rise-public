pub mod direction;
pub mod errors;
pub mod fixed;
pub mod funding;
pub mod leverage_tiers;
pub mod limit_order_state;
pub mod margin;
pub mod margin_calc;
pub mod market_math;
pub mod perp_metadata;
pub mod portfolio;
pub mod price;
pub mod quantities;
pub mod risk;
pub mod trader_position;

pub use direction::*;
pub use errors::*;
pub use fixed::*;
pub use funding::*;
pub use leverage_tiers::*;
pub use limit_order_state::*;
pub use margin::*;
pub use margin_calc::*;
pub use market_math::*;
pub use perp_metadata::*;
pub use portfolio::*;
pub use price::*;
pub use quantities::*;
pub use risk::*;
use sha2_const_stable::Sha256;
pub use trader_position::*;

pub const fn sha2_const(input: &[u8]) -> u64 {
    let hash = Sha256::new().update(input).finalize();
    u64::from_le_bytes([
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
    ])
}
