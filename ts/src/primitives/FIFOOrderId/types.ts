import type { Ticks } from "../_numberTypes";

export interface FIFOOrderId {
  /**
  The price of the order, in ticks. Each market has a designated
  tick size (some number of quote lots per base unit) that is used to
  convert the price to ticks. For example, if the tick size is 0.01,
  then a price of 1.23 is converted to 123 ticks. If the quote lot
  size is 0.001, this means that there is a spacing of 10 quote lots
  in between each tick.
  */
  priceInTicks: Ticks;
  /**
  This is the unique identifier of the order, which is used to determine
  the side of the order. It is derived from the sequence number of the
  market.

  If the order is a bid, the sequence number will have its bits inverted,
  and if it is an ask, the sequence number will be used as is.

  The way to identify the side of the order is to check the leading bit of
  `order_id`. A leading bit of 0 indicates an ask, and a leading bit
  of 1 indicates a bid. See Side::from_order_id.
  */
  orderSequenceNumber: bigint; // u64
}
