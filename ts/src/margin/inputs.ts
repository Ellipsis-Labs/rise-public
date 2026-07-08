import { minBigInt, toBigInt } from "./math";
import type { LimitOrderMarginInput, LimitOrderMarginState } from "./types";

const isActiveOrder = (order: LimitOrderMarginInput): boolean =>
  order.status ? order.status === "active" : true;

/**
 * Aggregate active orders into a `LimitOrderMarginState`.
 *
 * `basePositionLots` is the signed base-lot position; reduce-only totals are
 * capped by the position they can actually reduce (a reduce-only ask can only
 * reduce a long, a reduce-only bid only a short), mirroring the canonical
 * `From<&TraderPositionState>`. Without the cap a large reduce-only order
 * against a small opposite position would over-reserve adverse-fill margin.
 */
export const buildLimitOrderMarginStateFromOrders = (
  orders: LimitOrderMarginInput[],
  basePositionLots: bigint = 0n
): LimitOrderMarginState => {
  let totalNonReduceOnlyBidBaseLots = 0n;
  let totalNonReduceOnlyAskBaseLots = 0n;
  let totalReduceOnlyBidBaseLots = 0n;
  let totalReduceOnlyAskBaseLots = 0n;
  let highestBid = 0n;
  let lowestAsk = 0n;
  let numBidOrders = 0;
  let numAskOrders = 0;

  for (const order of orders) {
    if (!isActiveOrder(order)) {
      continue;
    }
    const size = toBigInt(order.sizeRemainingLots);
    const priceTicks = toBigInt(order.priceTicks);
    if (order.side === "bid") {
      numBidOrders += 1;
      if (priceTicks > highestBid) {
        highestBid = priceTicks;
      }
      if (order.reduceOnly) {
        totalReduceOnlyBidBaseLots += size;
      } else {
        totalNonReduceOnlyBidBaseLots += size;
      }
    } else {
      numAskOrders += 1;
      if (lowestAsk === 0n || priceTicks < lowestAsk) {
        lowestAsk = priceTicks;
      }
      if (order.reduceOnly) {
        totalReduceOnlyAskBaseLots += size;
      } else {
        totalNonReduceOnlyAskBaseLots += size;
      }
    }
  }

  // A reduce-only ask can only reduce a long position; a reduce-only bid only a
  // short. Cap each reduce-only total by the reducible position size.
  const reducibleLong = basePositionLots > 0n ? basePositionLots : 0n;
  const reducibleShort = basePositionLots < 0n ? -basePositionLots : 0n;
  const cappedReduceOnlyAskBaseLots = minBigInt(
    totalReduceOnlyAskBaseLots,
    reducibleLong
  );
  const cappedReduceOnlyBidBaseLots = minBigInt(
    totalReduceOnlyBidBaseLots,
    reducibleShort
  );

  return {
    numAskOrders,
    numBidOrders,
    lowestAsk: lowestAsk.toString(),
    highestBid: highestBid.toString(),
    totalNonReduceOnlyAskBaseLots: totalNonReduceOnlyAskBaseLots.toString(),
    totalReduceOnlyAskBaseLots: cappedReduceOnlyAskBaseLots.toString(),
    totalNonReduceOnlyBidBaseLots: totalNonReduceOnlyBidBaseLots.toString(),
    totalReduceOnlyBidBaseLots: cappedReduceOnlyBidBaseLots.toString(),
  };
};
