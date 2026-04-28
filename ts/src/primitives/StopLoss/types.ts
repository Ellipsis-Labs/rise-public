/**
 * Direction for stop loss trigger.
 * - GreaterThan: trigger when price rises above the threshold
 * - LessThan: trigger when price falls below the threshold
 */
export enum Direction {
  GreaterThan = 0,
  LessThan = 1,
}

/**
 * Order type for stop loss execution.
 * - IOC: execute immediately, cancelling any unfilled remainder
 * - Limit: place a limit order at the execution price
 */
export enum StopLossOrderKind {
  IOC = 0,
  Limit = 1,
}
