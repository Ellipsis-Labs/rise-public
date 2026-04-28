import * as rise from "@/index";
import { describe, expect, it } from "vitest";
import {
  API_NAMESPACE_CLIENTS,
  HTTP_CLIENT_ABSENT_METHODS,
  HTTP_CLIENT_PRESENT_METHODS,
  HTTP_RESOURCE_ABSENT_METHODS,
  HTTP_RESOURCE_PRESENT_METHODS,
  MIGRATED_INSTRUCTION_BUILDERS,
  PHOENIX_CLIENT_EXCHANGE_PROPERTIES,
  PHOENIX_CLIENT_PRESENT_PROPERTIES,
  PHOENIX_IX_PROPERTIES,
  PHOENIX_ORDER_PACKET_PROPERTIES,
  PHOENIX_RPC_PROPERTIES,
  ROOT_ABSENT_PROPERTIES,
  ROOT_PRESENT_PROPERTIES,
} from "./public-surface.manifest";

const expectProperties = (
  target: object,
  properties: readonly string[],
  expected: boolean = true
): void => {
  for (const property of properties) {
    expect(property in target).toBe(expected);
  }
};

describe("rise public surface", () => {
  it("exports the public parity clients from the package api namespace", () => {
    expectProperties(rise.api, API_NAMESPACE_CLIENTS);
  });

  it("exposes the public parity accessors on PhoenixHttpClient", () => {
    const client = new rise.PhoenixHttpClient();

    expectProperties(client, HTTP_CLIENT_PRESENT_METHODS);
    expectProperties(client, HTTP_CLIENT_ABSENT_METHODS, false);
  });

  it("keeps the exported resource methods on the public/data parity surface", () => {
    const client = new rise.PhoenixHttpClient();

    for (const [resource, methods] of Object.entries(
      HTTP_RESOURCE_PRESENT_METHODS
    )) {
      expectProperties(
        (client as Record<string, () => object>)[resource](),
        methods as readonly string[]
      );
    }

    for (const [resource, methods] of Object.entries(
      HTTP_RESOURCE_ABSENT_METHODS
    )) {
      expectProperties(
        (client as Record<string, () => object>)[resource](),
        methods as readonly string[],
        false
      );
    }
  });

  it("does not expose route or path catalog helpers from the root package", () => {
    for (const property of ROOT_ABSENT_PROPERTIES) {
      expect(rise).not.toHaveProperty(property);
    }
  });

  it("exports the migrated instruction builders from the root package", () => {
    for (const property of MIGRATED_INSTRUCTION_BUILDERS) {
      expect(rise).toHaveProperty(property);
    }
  });

  it("exports the public websocket subscription factories from the root package", () => {
    expect(rise.DEFAULT_PHOENIX_API_URL).toBe("https://perp-api.phoenix.trade");
    for (const property of ROOT_PRESENT_PROPERTIES) {
      expect(rise).toHaveProperty(property);
    }
  });

  it("exposes order packet builders on PhoenixClient", () => {
    const client = rise.createPhoenixClient();

    expectProperties(client, PHOENIX_CLIENT_PRESENT_PROPERTIES);
    expectProperties(client.exchange, PHOENIX_CLIENT_EXCHANGE_PROPERTIES);
    expect(client.marketData()).toBe(client.marketData());
    expect(client.orderbooks()).toBe(client.orderbooks());
    expectProperties(client.orderPackets, PHOENIX_ORDER_PACKET_PROPERTIES);
    expectProperties(client.ixs, PHOENIX_IX_PROPERTIES);
    expectProperties(client.rpc, PHOENIX_RPC_PROPERTIES);

    client.dispose();
  });

  it("allows opting out of the default websocket transport", () => {
    const client = rise.createPhoenixClient({
      ws: false,
    });

    expect(client.streams).toBeUndefined();

    client.dispose();
  });
});
