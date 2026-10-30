import { describe, expect, it } from "vitest";
import wireFixtures from "../test-fixtures/spline-price-metadata.json";
import {
  getUpdateSplinePriceWithMetadataCodec,
  getUpdateSplinePriceWithMetadataDecoder,
  getUpdateSplinePriceWithMetadataEncoder,
  getUpdateSplinePriceWithMetadataParamsDecoder,
  getUpdateSplinePriceWithMetadataParamsEncoder,
  getUpdateSplinePriceWithOrderingDecoder,
  getUpdateSplinePriceParamsWithOrderingDecoder,
} from "../src";
import { getSplineDecoder } from "../src/accounts/internal";
import {
  getUpdateSplinePriceParamsWithOrderingEncoder,
  getUpdateSplinePriceWithOrderingEncoder,
} from "../src/core/ixBuilders/Spline/codec";

describe("spline metadata wire format", () => {
  it("encodes every flag combination at fixed offsets", () => {
    for (let flags = 0; flags < 32; flags++) {
      const params = {
        newMidPrice: 123456n,
        refreshRegions: (flags & 1) !== 0,
        overrideSequenceNumber: (flags & 2) !== 0,
        userSequenceNumber: 42n,
        userUpdateSlot: (flags & 4) !== 0 ? 0n : null,
        bidWidth: (flags & 8) !== 0 ? 0n : null,
        askWidth: (flags & 16) !== 0 ? 0xffff_ffffn : null,
        clientOrderId: Uint8Array.from({ length: 16 }, (_, index) => index),
      };
      const expected = new Uint8Array(56);
      const view = new DataView(expected.buffer);
      expected.set([153, 162, 192, 201, 188, 88, 30, 224]);
      view.setUint32(8, 123456, true);
      view.setUint32(12, flags, true);
      view.setBigUint64(16, 42n, true);
      view.setBigUint64(24, 0n, true);
      view.setUint32(32, 0, true);
      view.setUint32(36, Number(params.askWidth ?? 0n), true);
      expected.set(params.clientOrderId, 40);
      const data = getUpdateSplinePriceWithMetadataEncoder().encode(params);
      expect(data).toEqual(expected);
      expect(getUpdateSplinePriceWithMetadataDecoder().decode(data)).toEqual(
        params
      );
      const codec = getUpdateSplinePriceWithMetadataCodec();
      expect(codec.encode(params)).toEqual(expected);
      expect(codec.decode(data)).toEqual(params);
      const wrongDiscriminator = Uint8Array.from(data);
      wrongDiscriminator[0] ^= 1;
      expect(() =>
        getUpdateSplinePriceWithMetadataDecoder().decode(wrongDiscriminator)
      ).toThrow();
    }
  });

  it("matches the Rust SDK wire fixtures byte for byte", () => {
    const encoder = getUpdateSplinePriceWithMetadataParamsEncoder();
    const decoder = getUpdateSplinePriceWithMetadataParamsDecoder();
    for (const fixture of wireFixtures) {
      const params = {
        newMidPrice: 123456n,
        userUpdateSlot: fixture.slot === null ? null : BigInt(fixture.slot),
        refreshRegions: true,
        userSequenceNumber: 42n,
        overrideSequenceNumber: false,
        clientOrderId: new Uint8Array(16).fill(0xab),
        bidWidth: fixture.bidWidth === null ? null : BigInt(fixture.bidWidth),
        askWidth: fixture.askWidth === null ? null : BigInt(fixture.askWidth),
      };
      const expected = Uint8Array.from(Buffer.from(fixture.hex, "hex"));
      expect(encoder.encode(params)).toEqual(expected);
      expect(decoder.decode(expected)).toEqual(params);
    }
  });

  it("decodes updates without a complete client ID tail", () => {
    const params = {
      newMidPrice: 7n,
      userUpdateSlot: null,
      refreshRegions: true,
      userSequenceNumber: 42n,
    };
    const data = getUpdateSplinePriceWithMetadataParamsEncoder().encode(params);
    for (let length = 32; length < 48; length++) {
      const decoded = getUpdateSplinePriceWithMetadataParamsDecoder().decode(
        data.slice(0, length)
      );
      expect(decoded.clientOrderId).toEqual(new Uint8Array(16));
      expect(decoded.newMidPrice).toBe(7n);
      expect(decoded.userSequenceNumber).toBe(42n);
    }

    // Untyped JS callers may omit the slot; that must not set slot 0.
    const { userUpdateSlot: _, ...withoutSlot } = params;
    const omitted = getUpdateSplinePriceWithMetadataParamsEncoder().encode(
      withoutSlot as typeof params
    );
    expect(omitted).toEqual(data);
  });

  it("rejects incomplete metadata and out-of-range tick inputs", () => {
    const decoder = getUpdateSplinePriceWithMetadataParamsDecoder();
    for (let length = 0; length < 80; length++) {
      if (length < 32) {
        expect(() => decoder.decode(new Uint8Array(length))).toThrow();
      }
    }
    const params = {
      newMidPrice: 1n,
      userUpdateSlot: null,
      refreshRegions: false,
      userSequenceNumber: 1n,
    };
    for (const field of ["newMidPrice", "bidWidth", "askWidth"]) {
      for (const value of [-1n, 0x1_0000_0000n]) {
        expect(() =>
          getUpdateSplinePriceWithMetadataParamsEncoder().encode({
            ...params,
            [field]: value,
          })
        ).toThrow();
      }
    }
  });

  it("preserves the old ordered wire format", () => {
    for (const userUpdateSlot of [null, 123n]) {
      const params = {
        newMidPrice: 123456n,
        userUpdateSlot,
        refreshRegions: true,
        userSequenceNumber: 42n,
        clientOrderId: new Uint8Array(16).fill(9),
      };
      const old =
        getUpdateSplinePriceParamsWithOrderingEncoder().encode(params);
      expect(old.length).toBe(userUpdateSlot === null ? 35 : 43);
      expect(
        getUpdateSplinePriceParamsWithOrderingDecoder().decode(old)
          .clientOrderId
      ).toEqual(params.clientOrderId);
      expect(
        getUpdateSplinePriceWithOrderingDecoder().decode(
          getUpdateSplinePriceWithOrderingEncoder().encode(params)
        ).clientOrderId
      ).toEqual(params.clientOrderId);
    }
  });

  it("reads widths from reserved words without changing the spline stride", () => {
    const bytes = new Uint8Array(1336);
    const view = new DataView(bytes.buffer);
    view.setBigUint64(1112, 17n, true);
    view.setBigUint64(1120, 29n, true);
    const [spline, end] = getSplineDecoder().read(bytes, 0);
    expect(end).toBe(1336);
    expect(spline.bidBaseWidth).toBe(17n);
    expect(spline.askBaseWidth).toBe(29n);
  });
});
