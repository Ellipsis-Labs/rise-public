import z from "zod";

export const numericBigint = (label: string): z.ZodType<bigint> =>
  z
    .bigint()
    .or(z.string().regex(/^-?\d+$/))
    .or(
      z
        .number()
        .int()
        .refine(
          (v) => Number.isSafeInteger(v),
          `${label} must be a safe integer`
        )
    )
    .transform((v) => (typeof v === "bigint" ? v : BigInt(v)));
