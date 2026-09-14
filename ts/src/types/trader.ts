import z from "zod";
import { type TokenAmount, TokenAmountSchema } from "@/primitives/TokenAmount";
import { type Side, side } from "@/primitives/Side";
import { type Symbol, symbol } from "@/primitives/Symbol";
import type { Authority } from "@/primitives/_addressTypes";

const zSymbol = z.string().transform(symbol);
const zAuthority = z
  .string()
  .transform((value): Authority => value as Authority);

export enum RiskState {
  Healthy = "healthy",
  Unhealthy = "unhealthy",
  Underwater = "underwater",
  ZeroCollateralNoPositions = "zeroCollateralNoPositions",
}

export enum RiskTier {
  Safe = "safe",
  AtRisk = "atRisk",
  Cancellable = "cancellable",
  Liquidatable = "liquidatable",
  BackstopLiquidatable = "backstopLiquidatable",
  HighRisk = "highRisk",
}

export interface Position {
  symbol: Symbol;
  positionSize: TokenAmount;
  virtualQuotePosition: TokenAmount;
  entryPrice: TokenAmount;
  unrealizedPnl: TokenAmount;
  discountedUnrealizedPnl?: TokenAmount;
  positionInitialMargin: TokenAmount;
  initialMargin: TokenAmount;
  maintenanceMargin: TokenAmount;
  backstopMargin: TokenAmount;
  limitOrderMargin: TokenAmount;
  positionValue: TokenAmount;
  unsettledFunding: TokenAmount;
  accumulatedFunding: TokenAmount;
  liquidationPrice: TokenAmount;
  takeProfitPrice?: TokenAmount | null;
  stopLossPrice?: TokenAmount | null;
}

export const PositionSchema: z.ZodType<Position> = z.object({
  symbol: zSymbol,
  positionSize: TokenAmountSchema,
  virtualQuotePosition: TokenAmountSchema,
  entryPrice: TokenAmountSchema,
  unrealizedPnl: TokenAmountSchema,
  discountedUnrealizedPnl: TokenAmountSchema.optional(),
  positionInitialMargin: TokenAmountSchema,
  initialMargin: TokenAmountSchema,
  maintenanceMargin: TokenAmountSchema,
  backstopMargin: TokenAmountSchema,
  limitOrderMargin: TokenAmountSchema,
  positionValue: TokenAmountSchema,
  unsettledFunding: TokenAmountSchema,
  accumulatedFunding: TokenAmountSchema,
  liquidationPrice: TokenAmountSchema,
  takeProfitPrice: TokenAmountSchema.nullable().optional(),
  stopLossPrice: TokenAmountSchema.nullable().optional(),
});

export interface LimitOrder {
  price: TokenAmount;
  side: Side;
  orderSequenceNumber: string;
  initialTradeSize: TokenAmount;
  tradeSizeRemaining: TokenAmount;
  marginRequirement: TokenAmount;
  marginFactor: TokenAmount;
  isReduceOnly: boolean;
  isStopLoss?: boolean;
  isStopLossDirection?: boolean;
  /**
   * Client-assigned scale set id (1-255) when the order was placed as part
   * of a scale (ladder) order batch; absent for standalone orders.
   */
  scaleSetId?: number;
}

export const LimitOrderSchema: z.ZodType<LimitOrder> = z.object({
  price: TokenAmountSchema,
  side: z
    .union([
      z.enum(["bid", "ask", "Bid", "Ask", "buy", "sell", "Buy", "Sell"]),
      z.literal(0),
      z.literal(1),
    ])
    .transform((value) => {
      if (typeof value === "number") {
        return value === 0 ? side("bid") : side("ask");
      }
      return side(value as "bid" | "buy" | "ask" | "sell");
    }),
  orderSequenceNumber: z.string(),
  initialTradeSize: TokenAmountSchema,
  tradeSizeRemaining: TokenAmountSchema,
  marginRequirement: TokenAmountSchema,
  marginFactor: TokenAmountSchema,
  isReduceOnly: z.boolean(),
  isStopLoss: z.boolean().optional().default(false),
  isStopLossDirection: z.boolean().optional().default(false),
  scaleSetId: z.number().int().min(1).max(255).optional(),
});

export interface CapabilityAccess {
  immediate: boolean;
  viaColdActivation: boolean;
}

const CapabilityAccessSchema: z.ZodType<CapabilityAccess> = z.object({
  immediate: z.boolean(),
  viaColdActivation: z.boolean().optional().default(false),
});

export interface TraderCapabilities {
  placeLimitOrder: CapabilityAccess;
  placeMarketOrder: CapabilityAccess;
  riskIncreasingTrade: CapabilityAccess;
  riskReducingTrade: CapabilityAccess;
  depositCollateral: CapabilityAccess;
  withdrawCollateral: CapabilityAccess;
}

export const TraderCapabilitiesSchema: z.ZodType<TraderCapabilities> = z.object(
  {
    placeLimitOrder: CapabilityAccessSchema,
    placeMarketOrder: CapabilityAccessSchema,
    riskIncreasingTrade: CapabilityAccessSchema,
    riskReducingTrade: CapabilityAccessSchema,
    depositCollateral: CapabilityAccessSchema,
    withdrawCollateral: CapabilityAccessSchema,
  }
);

/**
 * A trader's balance in one spot collateral asset, valued for margin.
 *
 * Spot collateral is collateral held in an asset other than the quote token;
 * native SOL is the only one today. `balance` and `withdrawable` are in the
 * asset's own units (SOL, 9 decimals), while `notional` and `discounted` are
 * quote units.
 */
export interface SpotCollateralBalance {
  /** Raw asset-index key of the asset in the trader position map. This is not
   * the perp market asset id. */
  assetIndex: number;
  /** Spot asset symbol ("SOL" for native SOL), not the perp market symbol. */
  symbol: string;
  /** Balance in the asset's own units. */
  balance: TokenAmount;
  /** Balance valued at the index price, undiscounted. */
  notional: TokenAmount;
  /** Notional after the margin haircut — this asset's contribution to
   * effective collateral. */
  discounted: TokenAmount;
  /** Maximum balance currently withdrawable while the account stays healthy.
   * Zero when the account is not currently healthy. Excludes uncounted excess
   * lamports, which are always withdrawable. */
  withdrawable: TokenAmount;
}

export const SpotCollateralBalanceSchema: z.ZodType<SpotCollateralBalance> =
  z.object({
    assetIndex: z.number(),
    symbol: z.string(),
    balance: TokenAmountSchema,
    notional: TokenAmountSchema,
    discounted: TokenAmountSchema,
    withdrawable: TokenAmountSchema,
  });
export interface TraderView {
  flags: number;
  state: string;
  capabilities: TraderCapabilities;
  slot: number;
  slotIndex: number;
  traderKey: Authority;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  authority: Authority;
  collateralBalance: TokenAmount;
  spotCollaterals?: SpotCollateralBalance[];
  effectiveCollateral: TokenAmount;
  effectiveCollateralForWithdrawals: TokenAmount;
  unrealizedPnl: TokenAmount;
  discountedUnrealizedPnl: TokenAmount;
  unsettledFundingOwed: TokenAmount;
  accumulatedFunding: TokenAmount;
  portfolioValue: TokenAmount;
  maintenanceMargin: TokenAmount;
  cancelMargin: TokenAmount;
  initialMargin: TokenAmount;
  initialMarginForWithdrawals: TokenAmount;
  riskState: RiskState;
  riskTier: RiskTier;
  positions: Position[];
  limitOrders: Record<Symbol, LimitOrder[]>;
  maxPositions: number;
  lastDepositSlot: number;
  isInActiveTraders: boolean;
  makerFeeOverrideMultiplier: number;
  takerFeeOverrideMultiplier: number;
  verifyCapabilities(): boolean;
}

export const TraderViewSchema: z.ZodType<TraderView> = z
  .object({
    flags: z.number(),
    state: z.string(),
    capabilities: TraderCapabilitiesSchema,
    traderKey: zAuthority,
    slot: z.number(),
    slotIndex: z.number(),
    traderPdaIndex: z.number(),
    traderSubaccountIndex: z.number(),
    authority: zAuthority,
    collateralBalance: TokenAmountSchema,
    spotCollaterals: z
      .array(SpotCollateralBalanceSchema)
      .optional()
      .default([]),
    effectiveCollateral: TokenAmountSchema,
    effectiveCollateralForWithdrawals: TokenAmountSchema,
    unsettledFundingOwed: TokenAmountSchema,
    accumulatedFunding: TokenAmountSchema,
    portfolioValue: TokenAmountSchema,
    maintenanceMargin: TokenAmountSchema,
    cancelMargin: TokenAmountSchema,
    initialMargin: TokenAmountSchema,
    initialMarginForWithdrawals: TokenAmountSchema,
    unrealizedPnl: TokenAmountSchema,
    discountedUnrealizedPnl: TokenAmountSchema,
    riskState: z.enum(RiskState),
    riskTier: z.enum(RiskTier),
    positions: z.array(PositionSchema),
    limitOrders: z
      .record(z.string(), z.array(LimitOrderSchema))
      .transform(
        (limitOrders): Record<Symbol, LimitOrder[]> =>
          Object.fromEntries(
            Object.entries(limitOrders).map(([marketSymbol, orders]) => [
              symbol(marketSymbol),
              orders,
            ])
          ) as Record<Symbol, LimitOrder[]>
      ),
    maxPositions: z.number(),
    lastDepositSlot: z.number().default(0),
    isInActiveTraders: z.boolean(),
    makerFeeOverrideMultiplier: z.number().default(1),
    takerFeeOverrideMultiplier: z.number().default(1),
  })
  .transform(
    (trader): TraderView => ({
      ...trader,
      verifyCapabilities() {
        // placeLimitOrder: at least one of immediate or viaColdActivation must be true
        if (
          !this.capabilities.placeLimitOrder.immediate &&
          !this.capabilities.placeLimitOrder.viaColdActivation
        ) {
          return false;
        }

        // All other capabilities: both immediate and viaColdActivation must be true
        const otherCapabilities = [
          this.capabilities.placeMarketOrder,
          this.capabilities.riskIncreasingTrade,
          this.capabilities.riskReducingTrade,
          this.capabilities.depositCollateral,
          this.capabilities.withdrawCollateral,
        ];

        return otherCapabilities.every(
          (cap) => cap.immediate || cap.viaColdActivation
        );
      },
    })
  );
