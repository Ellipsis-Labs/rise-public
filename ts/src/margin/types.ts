export interface LeverageTierParams {
  upperBoundSize: string;
  maxLeverage: string;
  limitOrderRiskFactorBps: string;
}

export interface RiskFactorParams {
  maintenanceMarginFactorBps: string;
  backstopMarginFactorBps: string;
  highRiskMarginFactorBps: string;
}

export interface SnapshotPrice {
  price: number;
  slot: number;
}

export interface SnapshotL2Orderbook {
  bids: Array<[number, number]>;
  asks: Array<[number, number]>;
  mid: number | null;
}

export interface MarketParams {
  symbol: string;
  assetId: number;
  spotPrice?: SnapshotPrice;
  markPriceTicks: string;
  l2Orderbook?: SnapshotL2Orderbook;
  tickSize: string;
  baseLotDecimals: number;
  leverageTiers: LeverageTierParams[];
  riskFactors: RiskFactorParams;
  cancelOrderRiskFactorBps: string;
  upnlRiskFactor: string;
  upnlRiskFactorForWithdrawals: string;
  isolatedOnly: boolean;
}

export interface MarginPositionState {
  basePositionLots: string;
  virtualQuotePositionLots: string;
  entryPriceTicks: string;
  /**
   * Funding with the margin-engine sign convention: positive increases
   * effective collateral.
   */
  unsettledFundingQuoteLots: string;
  accumulatedFundingQuoteLots: string;
}

export interface LimitOrderMarginState {
  numAskOrders: number;
  numBidOrders: number;
  lowestAsk: string;
  highestBid: string;
  totalNonReduceOnlyAskBaseLots: string;
  totalReduceOnlyAskBaseLots: string;
  totalNonReduceOnlyBidBaseLots: string;
  totalReduceOnlyBidBaseLots: string;
}

export interface LimitOrderMarginInput {
  orderSequenceNumber: string;
  side: "bid" | "ask";
  priceTicks: string;
  sizeRemainingLots: string;
  initialSizeLots: string;
  reduceOnly: boolean;
  isStopLoss?: boolean;
  isStopLossDirection?: boolean;
  /**
   * Optional status hint. When provided, non-active rows are ignored by the
   * snapshot helpers.
   */
  status?: string;
}

export interface MarketMarginInputs {
  symbol: string;
  position?: MarginPositionState;
  limitOrderMargin?: LimitOrderMarginState;
  limitOrders?: LimitOrderMarginInput[];
}

/**
 * Per-symbol product/order leverage preferences. The margin calculator floors
 * finite values to safe integers and ignores non-finite values, unsafe values,
 * or values below 1.
 */
export type OrderLeverageLimitsBySymbol = Record<string, number>;

export interface MarginCalculationOptions {
  orderLeverageLimitsBySymbol?: OrderLeverageLimitsBySymbol;
}

/**
 * One spot collateral asset (native SOL today) with the context needed to
 * value it the way the on-chain RiskView does: balance at the pricing perp
 * market's index price, discounted along the linear margin-discount curve.
 * The curve parameters come from `/v1/collateral/assets`.
 */
export interface SpotCollateralMarginInput {
  assetIndex: number;
  /** Spot asset symbol ("SOL"), not necessarily a perp market symbol. */
  symbol: string;
  /** Balance in the asset's native units (lamports for SOL). */
  balance: string;
  /** Native-unit decimals of the asset (9 for SOL). */
  decimals: number;
  /**
   * Perp market whose price values the asset. Defaults to `symbol`.
   */
  pricingMarketSymbol?: string;
  /**
   * Valuation price in ticks of the pricing market. Defaults to the pricing
   * market's mark price (on-chain uses the index price; supply it here when
   * available).
   */
  indexPriceTicks?: string;
  /** Global balance cap in native units — the discount curve's right endpoint. */
  maxGlobalBalance: string;
  /** Margin discount at zero balance, basis points. */
  minMarginDiscountBps: number;
  /** Margin discount at the global cap, basis points. */
  maxMarginDiscountBps: number;
}

export interface SubaccountMarginInputs {
  subaccountIndex: number;
  collateralBalanceQuoteLots: string;
  markets: MarketMarginInputs[];
  spotCollaterals?: SpotCollateralMarginInput[];
}

export interface TraderMarginInputs {
  authority: string;
  traderPdaIndex: number;
  subaccounts: SubaccountMarginInputs[];
}

export type MarginRiskState =
  | "healthy"
  | "unhealthy"
  | "underwater"
  | "zeroCollateralNoPositions";

export type MarginRiskTier =
  | "safe"
  | "atRisk"
  | "cancellable"
  | "liquidatable"
  | "backstopLiquidatable"
  | "highRisk";

export interface MarginTotals {
  collateralBalanceQuoteLots: string;
  effectiveCollateralQuoteLots: string;
  effectiveCollateralForWithdrawalsQuoteLots: string;
  portfolioValueQuoteLots: string;
  initialMarginQuoteLots: string;
  /**
   * Initial margin computed with optional order leverage limits.
   * Present only when it differs from protocol initial margin.
   */
  orderLeverageAdjustedInitialMarginQuoteLots?: string;
  initialMarginForWithdrawalsQuoteLots: string;
  maintenanceMarginQuoteLots: string;
  cancelMarginQuoteLots: string;
  backstopMarginQuoteLots: string;
  highRiskMarginQuoteLots: string;
  limitOrderMarginQuoteLots: string;
  unrealizedPnlQuoteLots: string;
  discountedUnrealizedPnlQuoteLots: string;
  discountedPnlForWithdrawalsQuoteLots: string;
  unsettledFundingQuoteLots: string;
  accumulatedFundingQuoteLots: string;
  /**
   * Undiscounted spot collateral notional included in portfolioValue.
   * Present only when the inputs carry spot collaterals.
   */
  spotCollateralNotionalQuoteLots?: string;
  /**
   * Discounted spot collateral value included in effectiveCollateral (but
   * never in effectiveCollateralForWithdrawals, mirroring the on-chain
   * WithdrawQuoteCollateral semantics). Present only when the inputs carry
   * spot collaterals.
   */
  spotCollateralDiscountedQuoteLots?: string;
  riskState: MarginRiskState;
  riskTier: MarginRiskTier;
}

export interface MarketMarginResult {
  symbol: string;
  basePositionLots: string;
  virtualQuotePositionLots: string;
  entryPriceTicks: string;
  unrealizedPnlQuoteLots: string;
  discountedUnrealizedPnlQuoteLots: string;
  positionInitialMarginQuoteLots: string;
  initialMarginQuoteLots: string;
  maintenanceMarginQuoteLots: string;
  cancelMarginQuoteLots: string;
  backstopMarginQuoteLots: string;
  highRiskMarginQuoteLots: string;
  limitOrderMarginQuoteLots: string;
  positionValueQuoteLots: string;
  unsettledFundingQuoteLots: string;
  accumulatedFundingQuoteLots: string;
}

export interface OrderMarginResult {
  symbol: string;
  orderSequenceNumber: string;
  side: "bid" | "ask";
  priceTicks: string;
  tradeSizeRemainingLots: string;
  initialSizeLots: string;
  reduceOnly: boolean;
  isStopLoss: boolean;
  isStopLossDirection: boolean;
  marginRequirementQuoteLots: string;
  /**
   * Margin requirement computed with optional order leverage limits.
   * Present only when it differs from protocol margin requirement.
   */
  orderLeverageAdjustedMarginRequirementQuoteLots?: string;
  marginFactorBps: string;
}

/** One spot collateral asset valued for margin. */
export interface SpotCollateralMarginResult {
  assetIndex: number;
  symbol: string;
  /** Balance in the asset's native units. */
  balance: string;
  /** Balance valued at the pricing market's price (undiscounted). */
  notionalQuoteLots: string;
  /** Notional with the margin discount applied. */
  discountedQuoteLots: string;
}

export interface SubaccountMarginResult {
  subaccountIndex: number;
  margin: MarginTotals;
  marketMargins: MarketMarginResult[];
  limitOrders: OrderMarginResult[];
  /** Present only when the inputs carry spot collaterals. */
  spotCollaterals?: SpotCollateralMarginResult[];
}

export interface TraderMarginResult {
  authority: string;
  traderPdaIndex: number;
  subaccounts: SubaccountMarginResult[];
}

export interface MarginSnapshotSource {
  type: "s3" | "file" | "rpc";
  bucket?: string;
  prefix?: string;
  slot?: number;
  path?: string;
  url?: string;
}

export interface TraderExpectedSnapshot {
  subaccounts: Array<{
    traderKey: string;
    traderSubaccountIndex: number;
    margin: MarginTotals;
    marketMargins: MarketMarginResult[];
    limitOrders: OrderMarginResult[];
  }>;
}

export interface MarginSnapshotTrader {
  authority: string;
  traderPdaIndex: number;
  message: unknown;
  expected: TraderExpectedSnapshot;
}

export interface MarginSnapshotFile {
  schemaVersion: number;
  generatedAt: string;
  slot: number;
  snapshotSource: MarginSnapshotSource;
  markets: MarketParams[];
  traders: MarginSnapshotTrader[];
}
