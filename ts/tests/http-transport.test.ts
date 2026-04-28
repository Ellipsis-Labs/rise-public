import { PhoenixHttpError } from "@/errors";
import { send, type HttpTransport } from "@/http/transport";
import { describe, expect, it } from "vitest";
import z from "zod";

describe("http transport errors", () => {
  it("includes the HTTP status code and URL when a request fails", async () => {
    const response = new Response(null, {
      status: 404,
      statusText: "Not Found",
    });

    Object.defineProperty(response, "url", {
      configurable: true,
      value: "https://perp-api.phoenix.trade/exchange/market/SOL",
    });

    const transport: HttpTransport = {
      fetch: async () => response,
    };

    await expect(
      send(transport, "GET", "/exchange/market/SOL", z.unknown())
    ).rejects.toEqual(
      expect.objectContaining<PhoenixHttpError>({
        message:
          "GET https://perp-api.phoenix.trade/exchange/market/SOL failed with HTTP 404 Not Found",
        name: "PhoenixHttpError",
        status: 404,
      })
    );
  });

  it("accepts a deprecated routeId and ignores it", async () => {
    const response = new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });

    const transport: HttpTransport = {
      fetch: async () => response,
    };

    await expect(
      send(transport, "GET", "/health", z.object({ ok: z.boolean() }), {
        routeId: "legacy.health",
      })
    ).resolves.toEqual({ ok: true });
  });
});
