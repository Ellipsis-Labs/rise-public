use serde::Serialize;
use solana_pubkey::Pubkey;

use super::internal::{FifoOrderId, SequenceNumber};
use super::stop_losses::{StopLossDirection, StopLossOrderKind, StopLossTradeSide};
use super::{AccountDeserialize, AccountDeserializeError};
use crate::conditional_orders as borrowed;

const ACCOUNT: &str = "ConditionalOrderCollection";
const MAX_CONDITIONAL_ORDER_CAPACITY: u8 = 192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConditionalOrderCollection {
    pub header: ConditionalOrderHeader,
    pub orders: Vec<ConditionalOrder>,
    pub active_order_indices: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConditionalOrderHeader {
    pub trader_key: Pubkey,
    pub funding_key: Pubkey,
    pub sequence_number: SequenceNumber,
    pub len: u8,
    pub capacity: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConditionalOrder {
    pub sequence_number: u64,
    pub order_id: Option<FifoOrderId>,
    pub max_size: u64,
    pub fillable_size: u64,
    pub filled_size: u64,
    pub slot: u64,
    pub asset_id: u32,
    pub use_percent: bool,
    pub percent: u8,
    pub greater_trigger_order: ConditionalOrderTrigger,
    pub less_trigger_order: ConditionalOrderTrigger,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConditionalOrderTrigger {
    pub trigger_price: u64,
    pub execution_price: u64,
    pub position_sequence_number: u8,
    pub is_active: bool,
    pub execution_direction: StopLossDirection,
    pub trade_side: StopLossTradeSide,
    pub order_kind: StopLossOrderKind,
}

impl AccountDeserialize for ConditionalOrderCollection {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        borrowed::ConditionalOrders::try_from_account_bytes(data)?.try_into()
    }
}

impl TryFrom<borrowed::ConditionalOrders<'_>> for ConditionalOrderCollection {
    type Error = AccountDeserializeError;

    fn try_from(value: borrowed::ConditionalOrders<'_>) -> Result<Self, Self::Error> {
        validate_capacity(value.header().capacity())?;
        let header = ConditionalOrderHeader {
            trader_key: Pubkey::new_from_array(value.header().trader_key()),
            funding_key: Pubkey::new_from_array(value.header().funding_key()),
            sequence_number: value.header().sequence_number().into(),
            len: value.header().len(),
            capacity: value.header().capacity(),
        };
        let mut orders = Vec::with_capacity(value.order_capacity());
        let mut active_order_indices = Vec::with_capacity(header.len as usize);
        for (index, order) in value.orders().enumerate() {
            let order: ConditionalOrder = order?.into();
            if index != 0 && order.is_active {
                active_order_indices.push(index as u8);
            }
            orders.push(order);
        }
        Ok(Self {
            header,
            orders,
            active_order_indices,
        })
    }
}

impl ConditionalOrderCollection {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }

    pub fn active_orders(&self) -> impl Iterator<Item = (u8, &ConditionalOrder)> {
        self.active_order_indices
            .iter()
            .copied()
            .map(|index| (index, &self.orders[index as usize]))
    }
}

impl ConditionalOrder {
    pub fn is_greater_trigger_order_active(&self) -> bool {
        self.greater_trigger_order.is_active
    }

    pub fn is_less_trigger_order_active(&self) -> bool {
        self.less_trigger_order.is_active
    }

    pub fn active_trigger_directions(&self) -> Vec<StopLossDirection> {
        let mut directions = Vec::with_capacity(2);
        if self.is_greater_trigger_order_active() {
            directions.push(StopLossDirection::GreaterThan);
        }
        if self.is_less_trigger_order_active() {
            directions.push(StopLossDirection::LessThan);
        }
        directions
    }
}

fn validate_capacity(capacity: u8) -> Result<(), AccountDeserializeError> {
    if capacity == 0 || capacity > MAX_CONDITIONAL_ORDER_CAPACITY {
        return Err(AccountDeserializeError::invalid_data(
            ACCOUNT,
            format!(
                "conditional order capacity {capacity} is outside the valid range \
                 1..={MAX_CONDITIONAL_ORDER_CAPACITY}"
            ),
        ));
    }
    Ok(())
}

impl From<borrowed::ConditionalOrder> for ConditionalOrder {
    fn from(value: borrowed::ConditionalOrder) -> Self {
        let order_id = value.order_id();
        let greater_trigger_order: ConditionalOrderTrigger = value.greater_trigger_order().into();
        let less_trigger_order: ConditionalOrderTrigger = value.less_trigger_order().into();
        Self {
            sequence_number: value.sequence_number(),
            order_id: order_id.is_some().then_some(FifoOrderId {
                price_in_ticks: order_id.price_in_ticks(),
                order_sequence_number: order_id.order_sequence_number(),
            }),
            max_size: value.max_size(),
            fillable_size: value.fillable_size(),
            filled_size: value.filled_size(),
            slot: value.slot(),
            asset_id: value.asset_id(),
            use_percent: value.flags().uses_percent(),
            percent: value.percent(),
            greater_trigger_order,
            less_trigger_order,
            is_active: greater_trigger_order.is_active || less_trigger_order.is_active,
        }
    }
}

impl From<borrowed::TriggerOrder> for ConditionalOrderTrigger {
    fn from(value: borrowed::TriggerOrder) -> Self {
        let flags = value.flags();
        Self {
            trigger_price: value.trigger_price(),
            execution_price: value.execution_price(),
            position_sequence_number: value.position_sequence_number(),
            is_active: flags.is_active(),
            execution_direction: if flags.is_greater_than_trigger() {
                StopLossDirection::GreaterThan
            } else {
                StopLossDirection::LessThan
            },
            trade_side: if flags.is_bid() {
                StopLossTradeSide::Bid
            } else {
                StopLossTradeSide::Ask
            },
            order_kind: if flags.is_limit_order() {
                StopLossOrderKind::Limit
            } else {
                StopLossOrderKind::IOC
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhoenixAccount;

    const CONDITIONAL_ORDER_HEADER_SIZE: usize = 96;
    const CONDITIONAL_ORDER_SIZE: usize = 112;

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_pubkey(bytes: &mut [u8], offset: usize, fill: u8) {
        bytes[offset..offset + 32].fill(fill);
    }

    fn write_trigger(
        bytes: &mut [u8],
        offset: usize,
        trigger_price: u64,
        execution_price: u64,
        position_sequence_number: u8,
        flags: u8,
    ) {
        write_u64(bytes, offset, trigger_price);
        write_u64(bytes, offset + 8, execution_price);
        bytes[offset + 16] = position_sequence_number;
        bytes[offset + 17] = flags;
    }

    fn create_collection_bytes() -> Vec<u8> {
        let capacity = 4u8;
        let mut bytes =
            vec![0u8; CONDITIONAL_ORDER_HEADER_SIZE + (capacity as usize) * CONDITIONAL_ORDER_SIZE];
        bytes[..8].copy_from_slice(&PhoenixAccount::ConditionalOrderCollection.discriminant());
        write_pubkey(&mut bytes, 8, 7);
        write_pubkey(&mut bytes, 40, 9);
        write_u64(&mut bytes, 72, 11);
        write_u64(&mut bytes, 80, 12);
        bytes[88] = 2;
        bytes[89] = capacity;

        let order_one_offset = CONDITIONAL_ORDER_HEADER_SIZE + CONDITIONAL_ORDER_SIZE;
        write_u64(&mut bytes, order_one_offset, 21);
        write_u64(&mut bytes, order_one_offset + 24, 100);
        write_u64(&mut bytes, order_one_offset + 32, 90);
        write_u64(&mut bytes, order_one_offset + 40, 10);
        write_u64(&mut bytes, order_one_offset + 48, 700);
        write_u32(&mut bytes, order_one_offset + 56, 5);
        bytes[order_one_offset + 60] = 0x01;
        bytes[order_one_offset + 61] = 25;
        write_trigger(&mut bytes, order_one_offset + 64, 101, 102, 3, 0x0f);
        write_trigger(&mut bytes, order_one_offset + 88, 0, 0, 0, 0x00);

        let order_two_offset = CONDITIONAL_ORDER_HEADER_SIZE + (2 * CONDITIONAL_ORDER_SIZE);
        write_u64(&mut bytes, order_two_offset, 22);
        write_u64(&mut bytes, order_two_offset + 8, 33);
        write_u64(&mut bytes, order_two_offset + 16, 44);
        write_u64(&mut bytes, order_two_offset + 24, 200);
        write_u64(&mut bytes, order_two_offset + 32, 150);
        write_u64(&mut bytes, order_two_offset + 40, 50);
        write_u64(&mut bytes, order_two_offset + 48, 701);
        write_u32(&mut bytes, order_two_offset + 56, 6);
        write_trigger(&mut bytes, order_two_offset + 64, 201, 202, 4, 0x03);
        write_trigger(&mut bytes, order_two_offset + 88, 301, 302, 4, 0x0d);

        bytes
    }

    #[test]
    fn decodes_conditional_order_collection() {
        let collection =
            ConditionalOrderCollection::try_from_account_bytes(&create_collection_bytes()).unwrap();

        assert_eq!(
            collection.header.trader_key,
            Pubkey::new_from_array([7; 32])
        );
        assert_eq!(
            collection.header.funding_key,
            Pubkey::new_from_array([9; 32])
        );
        assert_eq!(collection.header.sequence_number.sequence_number, 11);
        assert_eq!(collection.header.sequence_number.last_update_slot, 12);
        assert_eq!(collection.header.len, 2);
        assert_eq!(collection.header.capacity, 4);
        assert_eq!(collection.active_order_indices, vec![1, 2]);

        let first_order = &collection.orders[1];
        assert!(first_order.is_active);
        assert!(first_order.use_percent);
        assert_eq!(first_order.percent, 25);
        assert_eq!(first_order.asset_id, 5);
        assert_eq!(first_order.max_size, 100);
        assert_eq!(first_order.fillable_size, 90);
        assert_eq!(first_order.filled_size, 10);
        assert!(first_order.order_id.is_none());
        assert_eq!(
            first_order.active_trigger_directions(),
            vec![StopLossDirection::GreaterThan]
        );

        let second_order = &collection.orders[2];
        assert!(second_order.is_greater_trigger_order_active());
        assert!(second_order.is_less_trigger_order_active());
        assert_eq!(second_order.asset_id, 6);
        assert_eq!(second_order.order_id.unwrap().price_in_ticks, 33);
        assert_eq!(
            second_order.active_trigger_directions(),
            vec![StopLossDirection::GreaterThan, StopLossDirection::LessThan]
        );
    }

    #[test]
    fn tolerates_stale_header_len() {
        let mut bytes = create_collection_bytes();
        bytes[88] = 0;

        let collection = ConditionalOrderCollection::try_from_account_bytes(&bytes).unwrap();

        assert_eq!(collection.header.len, 0);
        assert_eq!(collection.active_order_indices, vec![1, 2]);
    }
}
