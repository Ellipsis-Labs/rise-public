use solana_pubkey::Pubkey;

use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{
    FifoOrderId, Reader, SequenceNumber, read_sequence_number, verify_discriminant,
};
use super::stop_losses::{StopLossDirection, StopLossOrderKind, StopLossTradeSide};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "ConditionalOrderCollection";
const CONDITIONAL_ORDER_HEADER_SIZE: usize = 96;
const CONDITIONAL_ORDER_SIZE: usize = 112;
const MAX_CONDITIONAL_ORDER_CAPACITY: u8 = 192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalOrderCollection {
    pub header: ConditionalOrderHeader,
    pub orders: Vec<ConditionalOrder>,
    pub active_order_indices: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalOrderHeader {
    pub trader_key: Pubkey,
    pub funding_key: Pubkey,
    pub sequence_number: SequenceNumber,
    pub len: u8,
    pub capacity: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        verify_discriminant(
            ACCOUNT,
            data,
            ACCOUNT_DISCRIMINANTS.conditional_order_collection,
        )?;

        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let header = ConditionalOrderHeader {
            trader_key: reader.read_pubkey()?,
            funding_key: reader.read_pubkey()?,
            sequence_number: read_sequence_number(&mut reader)?,
            len: reader.read_u8()?,
            capacity: reader.read_u8()?,
        };
        reader.skip(6)?;

        validate_capacity(header.capacity)?;

        let expected_size = CONDITIONAL_ORDER_HEADER_SIZE
            .checked_add((header.capacity as usize) * CONDITIONAL_ORDER_SIZE)
            .ok_or_else(|| {
                AccountDeserializeError::invalid_data(
                    ACCOUNT,
                    "conditional order collection size overflow",
                )
            })?;
        if data.len() < expected_size {
            return Err(AccountDeserializeError::too_short(
                ACCOUNT,
                expected_size,
                data.len(),
            ));
        }

        let mut orders = Vec::with_capacity(header.capacity as usize);
        let mut active_order_indices = Vec::with_capacity(header.len as usize);
        for index in 0..header.capacity {
            let order = read_conditional_order(&mut reader)?;
            if index != 0 && order.is_active {
                active_order_indices.push(index);
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

fn read_conditional_order(
    reader: &mut Reader<'_>,
) -> Result<ConditionalOrder, AccountDeserializeError> {
    let sequence_number = reader.read_u64()?;
    let order_id = read_optional_fifo_order_id(reader)?;
    let max_size = reader.read_u64()?;
    let fillable_size = reader.read_u64()?;
    let filled_size = reader.read_u64()?;
    let slot = reader.read_u64()?;
    let asset_id = reader.read_u32()?;
    let flags = reader.read_u8()?;
    let percent = reader.read_u8()?;
    reader.skip(2)?;
    let greater_trigger_order = read_conditional_order_trigger(reader)?;
    let less_trigger_order = read_conditional_order_trigger(reader)?;
    Ok(ConditionalOrder {
        sequence_number,
        order_id,
        max_size,
        fillable_size,
        filled_size,
        slot,
        asset_id,
        use_percent: flags & 0x01 != 0,
        percent,
        greater_trigger_order,
        less_trigger_order,
        is_active: greater_trigger_order.is_active || less_trigger_order.is_active,
    })
}

fn read_optional_fifo_order_id(
    reader: &mut Reader<'_>,
) -> Result<Option<FifoOrderId>, AccountDeserializeError> {
    let price_in_ticks = reader.read_u64()?;
    let order_sequence_number = reader.read_u64()?;
    if price_in_ticks == 0 && order_sequence_number == 0 {
        Ok(None)
    } else {
        Ok(Some(FifoOrderId {
            price_in_ticks,
            order_sequence_number,
        }))
    }
}

fn read_conditional_order_trigger(
    reader: &mut Reader<'_>,
) -> Result<ConditionalOrderTrigger, AccountDeserializeError> {
    let trigger_price = reader.read_u64()?;
    let execution_price = reader.read_u64()?;
    let position_sequence_number = reader.read_u8()?;
    let flags = reader.read_u8()?;
    reader.skip(6)?;
    Ok(ConditionalOrderTrigger {
        trigger_price,
        execution_price,
        position_sequence_number,
        is_active: flags & 0x01 != 0,
        execution_direction: if flags & 0x02 != 0 {
            StopLossDirection::GreaterThan
        } else {
            StopLossDirection::LessThan
        },
        trade_side: if flags & 0x04 != 0 {
            StopLossTradeSide::Bid
        } else {
            StopLossTradeSide::Ask
        },
        order_kind: if flags & 0x08 != 0 {
            StopLossOrderKind::Limit
        } else {
            StopLossOrderKind::IOC
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        bytes[..8].copy_from_slice(&ACCOUNT_DISCRIMINANTS.conditional_order_collection);
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
