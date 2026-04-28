import { Side } from "./types";

/**
 * Converts a string representation of a side to the Side enum.
 * Accepts "bid", "buy" (converts to Side.Bid) or "ask", "sell" (converts to Side.Ask).
 * Case-insensitive.
 *
 * @param value - The string representation of the side: "bid", "buy", "ask", or "sell"
 * @returns The corresponding Side enum value
 * @throws Error if the input string is not a valid side value
 *
 * @example
 * ```typescript
 * side("bid")  // Returns Side.Bid
 * side("buy")  // Returns Side.Bid
 * side("ask")  // Returns Side.Ask
 * side("sell") // Returns Side.Ask
 * side("BID")  // Returns Side.Bid (case-insensitive)
 * ```
 */
export const side = (value: "bid" | "buy" | "ask" | "sell"): Side => {
  const normalized = value.toLowerCase();

  if (normalized === "bid" || normalized === "buy") {
    return Side.Bid;
  }

  if (normalized === "ask" || normalized === "sell") {
    return Side.Ask;
  }

  throw new Error(
    `Invalid side value: "${value}". Expected "bid", "buy", "ask", or "sell".`
  );
};
