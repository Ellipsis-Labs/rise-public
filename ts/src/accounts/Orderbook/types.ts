import type { OrderbookHeader } from "../OrderbookHeader";
import type { OrderbookEntry } from "../internal";

export interface Orderbook {
  header: OrderbookHeader;
  bids: OrderbookEntry[];
  asks: OrderbookEntry[];
}
