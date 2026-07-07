import { describe, expect, it } from "vitest";
import {
  getLeverageConstant,
  getLimitOrderRiskFactor,
  type LeverageTier,
} from "@/margin/math";

describe("margin math", () => {
  it("matches Rust f64 interpolation for decreasing leverage tiers", () => {
    const tiers: LeverageTier[] = [
      {
        upperBoundSize: 1000n,
        maxLeverage: 20n,
        limitOrderRiskFactorBps: 1000n,
      },
      {
        upperBoundSize: 10000n,
        maxLeverage: 10n,
        limitOrderRiskFactorBps: 5000n,
      },
    ];

    expect(getLeverageConstant(tiers, 1500n)).toBe(19n);
  });

  it("uses the same interpolation helper for increasing risk factors", () => {
    const tiers: LeverageTier[] = [
      {
        upperBoundSize: 1000n,
        maxLeverage: 20n,
        limitOrderRiskFactorBps: 1000n,
      },
      {
        upperBoundSize: 10000n,
        maxLeverage: 10n,
        limitOrderRiskFactorBps: 5000n,
      },
    ];

    expect(getLimitOrderRiskFactor(tiers, 1500n)).toBe(1222n);
  });
});
