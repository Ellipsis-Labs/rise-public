import { PhoenixHttpClient } from "@/client";
import { PhoenixHttpError } from "@/errors";
import { send } from "@/http";
import {
  parseRetryAfter,
  rateLimitCooldownDelayMs,
  rateLimitDelayWithPositiveJitterMs,
  resolveRateLimitCooldownConfig,
} from "@/http/rateLimitRetry";
import { afterEach, describe, expect, it, vi } from "vitest";
import z from "zod";

const OkSchema = z.object({ ok: z.boolean() });

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("rate-limit retry handling", () => {
  it("parses numeric and HTTP-date Retry-After values", () => {
    const now = Date.UTC(2026, 0, 1, 0, 0, 0);
    const retryAt = new Date(now + 2_500).toUTCString();

    expect(parseRetryAfter("2", now)).toEqual({
      raw: "2",
      retryAfterMs: 2_000,
      retryAfterSeconds: 2,
    });
    expect(parseRetryAfter(retryAt, now)).toEqual({
      raw: retryAt,
      retryAfterMs: 2_000,
      retryAfterSeconds: 2,
    });
    expect(parseRetryAfter(new Date(now - 1_000).toUTCString(), now)).toEqual(
      expect.objectContaining({
        retryAfterMs: 0,
        retryAfterSeconds: 0,
      })
    );
    expect(parseRetryAfter("not a date", now)).toBeUndefined();
    expect(parseRetryAfter("Infinity", now)).toBeUndefined();
  });

  it("adds only positive jitter to Retry-After delays", () => {
    vi.spyOn(Math, "random").mockReturnValue(0.5);

    expect(rateLimitDelayWithPositiveJitterMs(2_000)).toBe(2_150);
  });

  it("caps the default shared cooldown at 30 seconds", () => {
    const config = resolveRateLimitCooldownConfig(undefined, null);
    if (config === null) {
      throw new Error("default cooldown config should be enabled");
    }

    const response = new Response(null, {
      status: 429,
      headers: { "retry-after": "300" },
    });
    expect(rateLimitCooldownDelayMs(response, config)).toBe(30_000);
  });

  it("retries GET rate limits up to the configured retry count", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: { "content-type": "application/json", "retry-after": "2" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: { "content-type": "application/json", "retry-after": "0" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({ apiUrl: "https://example.com" });
    const request = send(client, "GET", "/limited", OkSchema);

    await vi.advanceTimersByTimeAsync(2_000);
    await vi.runOnlyPendingTimersAsync();

    await expect(request).resolves.toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it("uses HTTP-date Retry-After values for retry delay", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0);
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
    const retryAt = new Date(Date.now() + 2_000).toUTCString();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: {
            "content-type": "application/json",
            "retry-after": retryAt,
          },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({ apiUrl: "https://example.com" });
    const request = send(client, "HEAD", "/limited", OkSchema);

    await vi.advanceTimersByTimeAsync(1_999);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);

    await expect(request).resolves.toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("falls back on invalid Retry-After values", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0.5);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: {
            "content-type": "application/json",
            "retry-after": "invalid",
          },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({ apiUrl: "https://example.com" });
    const request = send(client, "GET", "/limited", OkSchema);

    await vi.advanceTimersByTimeAsync(999);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);

    await expect(request).resolves.toEqual({ ok: true });
  });

  it("surfaces retry metadata when retries are disabled", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ error: "rate_limited" }), {
        status: 429,
        headers: { "content-type": "application/json", "retry-after": "3" },
      })
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitRetry: false,
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        retryAfterSeconds: 3,
        attempts: 1,
      })
    );
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("surfaces attempts when retryable GET rate limits exhaust max retries", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ error: "rate_limited" }), {
        status: 429,
        headers: { "content-type": "application/json", "retry-after": "0" },
      })
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitRetry: { maxRetries: 1 },
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        retryAfterSeconds: 0,
        attempts: 2,
      })
    );
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("does not retry non-idempotent requests", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ error: "rate_limited" }), {
        status: 429,
        headers: { "content-type": "application/json", "retry-after": "1" },
      })
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({ apiUrl: "https://example.com" });

    await expect(
      send(client, "POST", "/limited", OkSchema, undefined, {})
    ).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        retryAfterSeconds: 1,
        attempts: 1,
      })
    );
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("does not sleep past maxTotalWaitMs", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ error: "rate_limited" }), {
        status: 429,
        headers: { "content-type": "application/json", "retry-after": "2" },
      })
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitRetry: { maxTotalWaitMs: 1_000 },
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        retryAfterSeconds: 2,
        attempts: 1,
      })
    );
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("waits before an independent GET while shared cooldown is active", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: { "content-type": "application/json", "retry-after": "2" },
        })
      )
      .mockImplementationOnce(async (_input, init) => {
        expect(init?.signal?.aborted).toBe(false);
        return new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      });
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitRetry: false,
      timeout: 1_000,
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        attempts: 1,
      })
    );

    const request = send(client, "GET", "/limited", OkSchema);
    await Promise.resolve();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1_999);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);

    await expect(request).resolves.toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("preserves independent GET behavior when shared cooldown is disabled", async () => {
    vi.useFakeTimers();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: { "content-type": "application/json", "retry-after": "2" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitCooldown: false,
      rateLimitRetry: false,
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        attempts: 1,
      })
    );

    await expect(send(client, "GET", "/limited", OkSchema)).resolves.toEqual({
      ok: true,
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("shares a recorded cooldown across concurrent later GET and HEAD requests", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: { "content-type": "application/json", "retry-after": "2" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitRetry: false,
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        attempts: 1,
      })
    );

    const getRequest = send(client, "GET", "/limited", OkSchema);
    const headRequest = send(client, "HEAD", "/limited", OkSchema);
    await Promise.resolve();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(2_000);

    await expect(getRequest).resolves.toEqual({ ok: true });
    await expect(headRequest).resolves.toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it("does not apply shared cooldown to POST requests", async () => {
    vi.useFakeTimers();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: { "content-type": "application/json", "retry-after": "2" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitRetry: false,
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        attempts: 1,
      })
    );

    await expect(
      send(client, "POST", "/limited", OkSchema, undefined, {})
    ).resolves.toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("uses shared cooldown fallback delay when Retry-After is invalid", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "rate_limited" }), {
          status: 429,
          headers: {
            "content-type": "application/json",
            "retry-after": "invalid",
          },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        })
      );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      rateLimitCooldown: { fallbackDelayMs: 500 },
      rateLimitRetry: false,
    });

    await expect(send(client, "GET", "/limited", OkSchema)).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        status: 429,
        code: "rate_limited",
        attempts: 1,
      })
    );

    const request = send(client, "GET", "/limited", OkSchema);
    await Promise.resolve();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(499);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);

    await expect(request).resolves.toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});
