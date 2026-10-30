import type { TraderStateLockedDeposit } from "@/api/traders/traderState";

/** Current grant and consumption across one trader PDA. All values are quote
 * lots. Isolated lock snapshots are excluded from the grant total. This does
 * not report lifetime grants after an exhausted entry has been removed. */
export function getLockedDepositTotals(
  subaccounts: readonly {
    subaccountIndex: number;
    lockedDeposit?: TraderStateLockedDeposit | null;
  }[]
): {
  totalQuoteLots: bigint;
  consumedQuoteLots: bigint;
  remainingQuoteLots: bigint;
} {
  const totalQuoteLots = BigInt(
    subaccounts.find((s) => s.subaccountIndex === 0)?.lockedDeposit
      ?.lockedQuoteLots ?? "0"
  );
  const consumed = subaccounts.reduce(
    (sum, s) => sum + BigInt(s.lockedDeposit?.consumedQuoteLots ?? "0"),
    0n
  );
  const consumedQuoteLots =
    consumed < totalQuoteLots ? consumed : totalQuoteLots;
  return {
    totalQuoteLots,
    consumedQuoteLots,
    remainingQuoteLots: totalQuoteLots - consumedQuoteLots,
  };
}
