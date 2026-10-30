import z from "zod";

export type EscrowAction<Amount> =
  | { kind: "noop" }
  | { kind: "cash"; amount: Amount }
  | { kind: "lockedDeposit"; amount: Amount }
  | { kind: "unknown" };

/** Unknown actions remain visible so callers cannot mistake a partial request
 * for one whose complete effects they understand. */
export function createEscrowActionSchema<Amount>(
  amount: z.ZodType<Amount>
): z.ZodType<EscrowAction<Amount>> {
  const schema = z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("noop") }),
    z.object({ kind: z.literal("cash"), amount }),
    z.object({ kind: z.literal("lockedDeposit"), amount }),
    z.object({ kind: z.literal("unknown") }),
  ]);
  const knownKinds = new Set<string>(
    schema.options.map((option) => option.shape.kind.value)
  );

  return z.preprocess((value) => {
    if (
      typeof value === "object" &&
      value !== null &&
      "kind" in value &&
      typeof value.kind === "string" &&
      !knownKinds.has(value.kind)
    ) {
      return { kind: "unknown" };
    }
    return value;
  }, schema);
}
