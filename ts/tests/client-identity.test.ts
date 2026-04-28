import { PhoenixHttpClient } from "@/client";
import {
  PHOENIX_CLIENT_HEADER_NAME,
  PHOENIX_CLIENT_HEADER_VALUE,
} from "@/clientIdentity";
import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  delete process.env.PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER;
  delete process.env.NEXT_PUBLIC_PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER;
  vi.unstubAllGlobals();
});

describe("PhoenixHttpClient client identification", () => {
  it("sends the language, sdk name, and version header on requests", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      })
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient();

    await client.fetch("GET", "/v1/view/exchange");

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(String(fetchMock.mock.calls[0]?.[0])).toBe(
      "https://perp-api.phoenix.trade/v1/view/exchange"
    );
    const options = fetchMock.mock.calls[0][1] as RequestInit;
    const headers = options.headers as Record<string, string>;
    expect(headers[PHOENIX_CLIENT_HEADER_NAME]).toBe(
      PHOENIX_CLIENT_HEADER_VALUE
    );
  });

  it("omits the header when PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER is enabled", async () => {
    process.env.PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER = "1";

    const fetchMock = vi.fn().mockResolvedValueOnce(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      })
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
    });

    await client.fetch("GET", "/v1/view/exchange");

    const options = fetchMock.mock.calls[0][1] as RequestInit;
    const headers = options.headers as Record<string, string>;
    expect(headers[PHOENIX_CLIENT_HEADER_NAME]).toBeUndefined();
  });
});
