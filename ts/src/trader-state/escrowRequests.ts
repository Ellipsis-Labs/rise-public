import type { TraderStateEscrowRequest } from "@/api/traders/traderState";

/** Whether `request` can no longer be accepted at `currentSlot`. Requests
 * without an expiration slot never expire. */
export function isEscrowRequestExpired(
  request: Pick<TraderStateEscrowRequest, "expirationSlot">,
  currentSlot: bigint
): boolean {
  return (
    request.expirationSlot !== null &&
    currentSlot > BigInt(request.expirationSlot)
  );
}
