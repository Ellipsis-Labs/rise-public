import type { AllMidsAdapterOptions } from "./all-mids/adapter";
import type { CandlesAdapterOptions } from "./candles/adapter";
import type { ExchangeAdapterOptions } from "./exchange/adapter";
import type { ExchangeStatusAdapterOptions } from "./exchange-status/adapter";
import type { FillsAdapterOptions } from "./fills/adapter";
import type { FundingRateAdapterOptions } from "./funding-rate/adapter";
import type { L2BookAdapterOptions } from "./l2-book/adapter";
import type { MarkPriceAdapterOptions } from "./mark-price/adapter";
import type { MarketAdapterOptions } from "./market/adapter";
import type { MarketStatsAdapterOptions } from "./market-stats/adapter";
import type { MarketStatsV2AdapterOptions } from "./market-stats-v2/adapter";
import type { NotificationsAdapterOptions } from "./notifications/adapter";
import type { OrderbookAdapterOptions } from "./orderbook/adapter";
import type { TraderStateAdapterOptions } from "./trader-state/adapter";

export interface PublicAdapterOptions {
  allMids?: AllMidsAdapterOptions;
  candles?: CandlesAdapterOptions;
  exchange?: ExchangeAdapterOptions;
  exchangeStatus?: ExchangeStatusAdapterOptions;
  fills?: FillsAdapterOptions;
  fundingRate?: FundingRateAdapterOptions;
  l2Book?: L2BookAdapterOptions;
  markPrice?: MarkPriceAdapterOptions;
  market?: MarketAdapterOptions;
  marketStats?: MarketStatsAdapterOptions;
  marketStatsV2?: MarketStatsV2AdapterOptions;
  notifications?: NotificationsAdapterOptions;
  orderbook?: L2BookAdapterOptions;
  orderbookSnapshot?: OrderbookAdapterOptions;
  traderState?: TraderStateAdapterOptions;
}
