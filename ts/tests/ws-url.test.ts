import { toWebSocketUrl } from "@/ws/url";
import { describe, expect, it } from "vitest";

describe("toWebSocketUrl", () => {
  it("maps http urls to ws", () => {
    expect(toWebSocketUrl("http://example.com")).toBe("ws://example.com/v1/ws");
  });

  it("maps https urls to wss", () => {
    expect(toWebSocketUrl("https://example.com")).toBe(
      "wss://example.com/v1/ws"
    );
  });

  it("preserves ws urls", () => {
    expect(toWebSocketUrl("ws://example.com")).toBe("ws://example.com/v1/ws");
  });

  it("preserves wss urls", () => {
    expect(toWebSocketUrl("wss://example.com")).toBe("wss://example.com/v1/ws");
  });

  it("throws for unsupported protocols", () => {
    expect(() => toWebSocketUrl("ftp://example.com")).toThrow(
      "Unsupported websocket URL protocol 'ftp:' in 'ftp://example.com'"
    );
  });
});
