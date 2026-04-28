import { toBigInt } from "./math";
import type { LimitOrderMarginInput, LimitOrderMarginState } from "./types";

const isActiveOrder = (order: LimitOrderMarginInput): boolean =>
  order.status ? order.status === "active" : true;

export const buildLimitOrderMarginStateFromOrders = (
  orders: LimitOrderMarginInput[]
): LimitOrderMarginState => {
  let totalNonReduceOnlyBidBaseLots = 0n;
  let totalNonReduceOnlyAskBaseLots = 0n;
  let numBidOrders = 0;
  let numAskOrders = 0;

  for (const order of orders) {
    if (!isActiveOrder(order)) {
      continue;
    }
    if (order.side === "bid") {
      numBidOrders += 1;
      if (!order.reduceOnly) {
        totalNonReduceOnlyBidBaseLots += toBigInt(order.sizeRemainingLots);
      }
    } else {
      numAskOrders += 1;
      if (!order.reduceOnly) {
        totalNonReduceOnlyAskBaseLots += toBigInt(order.sizeRemainingLots);
      }
    }
  }

  return {
    totalNonReduceOnlyBidBaseLots: totalNonReduceOnlyBidBaseLots.toString(),
    totalNonReduceOnlyAskBaseLots: totalNonReduceOnlyAskBaseLots.toString(),
    numBidOrders,
    numAskOrders,
  };
};
