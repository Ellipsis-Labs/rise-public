// TS <-> Rust parity pin for the V2 order-packet wire format: the vectors below
// are shared verbatim with `rise/rust/ix/src/{order_packet,multi_limit_order}.rs`
// (see `V2_PARITY_PACKET_BYTES`) — change either side in both places.
//
// This file pins cross-language agreement on one shared vector. The separate
// byte-lock against program-core's canonical vector lives in
// `ixs.test.ts` → `describe("buildPlaceMultiLimitOrderV2Ix")`.
import {
  getCondensedOrderV2Encoder,
  getMultipleOrderPacketV2Encoder,
  CondensedOrderFlags,
} from "@/primitives/OrderPacket";
import { getPlaceMultiLimitOrderV2Encoder } from "@/core/ixBuilders/PlaceMultiLimitOrder";
import { DISCRIMINANTS } from "@/core/discriminants";
import { describe, expect, it } from "vitest";

// Matches `order_packet::tests::test_condensed_order_v2_byte_layout`.
const CONDENSED_ORDER_V2 = {
  priceInTicks: 50_000n,
  sizeInBaseLots: 1_000n,
  lastValidSlot: 12_345n,
  flags: CondensedOrderFlags.Slide,
};

// prettier-ignore
const CONDENSED_ORDER_V2_BYTES = [
  0x50, 0xc3, 0, 0, 0, 0, 0, 0, // priceInTicks = 50_000 (u64 LE)
  0xe8, 0x03, 0, 0, 0, 0, 0, 0, // sizeInBaseLots = 1_000 (u64 LE)
  0x39, 0x30, 0, 0, 0, 0, 0, 0, // lastValidSlot = 12_345 (raw u64 LE, no Option tag)
  0x01,                         // flags = Slide (0b01)
];

// Matches `order_packet::tests::test_multiple_order_packet_v2_byte_layout`.
const MULTIPLE_ORDER_PACKET_V2 = {
  bids: [
    {
      priceInTicks: 50_000n,
      sizeInBaseLots: 1_000n,
      lastValidSlot: null,
      flags: CondensedOrderFlags.None,
    },
  ],
  asks: [
    {
      priceInTicks: 51_000n,
      sizeInBaseLots: 500n,
      lastValidSlot: 999n,
      flags: CondensedOrderFlags.ReduceOnly,
    },
  ],
  clientOrderId: 0x0102030405060708090a0b0c0d0e0f10n,
  scaleSetId: 7,
};

// The Rust counterpart of this array is `order_packet::V2_PARITY_PACKET_BYTES`.
// prettier-ignore
const MULTIPLE_ORDER_PACKET_V2_BYTES = [
  1, 0, 0, 0,                   // bids: Vec len = 1
  80, 195, 0, 0, 0, 0, 0, 0,    // bid.priceInTicks = 50_000
  232, 3, 0, 0, 0, 0, 0, 0,     // bid.sizeInBaseLots = 1_000
  0, 0, 0, 0, 0, 0, 0, 0,       // bid.lastValidSlot = null (raw 0)
  0,                            // bid.flags = None
  1, 0, 0, 0,                   // asks: Vec len = 1
  56, 199, 0, 0, 0, 0, 0, 0,    // ask.priceInTicks = 51_000
  244, 1, 0, 0, 0, 0, 0, 0,     // ask.sizeInBaseLots = 500
  231, 3, 0, 0, 0, 0, 0, 0,     // ask.lastValidSlot = 999
  2,                            // ask.flags = ReduceOnly
  1,                            // clientOrderId: Option tag = Some
  16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, // clientOrderId bytes
  7,                            // scaleSetId
];

describe("order packet V2 TS<->Rust parity vectors", () => {
  it("encodes a single CondensedOrderV2 byte-for-byte", () => {
    const bytes = getCondensedOrderV2Encoder().encode(CONDENSED_ORDER_V2);

    expect(Array.from(bytes)).toEqual(CONDENSED_ORDER_V2_BYTES);
  });

  it("encodes a full MultipleOrderPacketV2 byte-for-byte", () => {
    const bytes = getMultipleOrderPacketV2Encoder().encode(
      MULTIPLE_ORDER_PACKET_V2
    );

    expect(Array.from(bytes)).toEqual(MULTIPLE_ORDER_PACKET_V2_BYTES);
  });

  // Rust counterpart: `multi_limit_order::tests::test_v2_instruction_data_matches_ts_parity_vector`.
  it("encodes the V2 instruction data as discriminant ++ packet", () => {
    const data = getPlaceMultiLimitOrderV2Encoder().encode(
      MULTIPLE_ORDER_PACKET_V2
    );

    expect(Array.from(data)).toEqual([
      ...DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER_V2,
      ...MULTIPLE_ORDER_PACKET_V2_BYTES,
    ]);
  });
});
