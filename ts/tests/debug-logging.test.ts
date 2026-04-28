import { afterEach, describe, expect, it, vi } from "vitest";
import { debugRise, isRiseDebugEnabled } from "@/internal/_debug";

const ORIGINAL_ENV = { ...process.env };

afterEach(() => {
  process.env = { ...ORIGINAL_ENV };
  vi.restoreAllMocks();
});

describe("rise debug logging", () => {
  it("stays silent when debug flags are disabled", () => {
    delete process.env.PHOENIX_DEBUG;
    delete process.env.PHOENIX_DEBUG_HTTP;
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});

    debugRise("http", "request:start", { method: "GET" });

    expect(debugSpy).not.toHaveBeenCalled();
    expect(isRiseDebugEnabled("http")).toBe(false);
  });

  it("logs timestamped namespace output when enabled", () => {
    process.env.PHOENIX_DEBUG_IX = "1";
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});

    debugRise("ix", "buildPlaceLimitOrder:start", { symbol: "SOL-PERP" });

    expect(isRiseDebugEnabled("ix")).toBe(true);
    expect(debugSpy).toHaveBeenCalledTimes(1);
    expect(debugSpy).toHaveBeenCalledWith(
      expect.stringMatching(
        /^\[[0-9]{4}-[0-9]{2}-[0-9]{2}T.*Z\] \[phoenix-rise:ix\] buildPlaceLimitOrder:start$/
      ),
      { symbol: "SOL-PERP" }
    );
  });
});
