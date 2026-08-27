import {
  AccountRole,
  address,
  getU32Decoder,
  getU64Decoder,
} from "@solana/kit";
import { describe, expect, it } from "vitest";
import { buildTransferSolIx } from "@/core/ixBuilders/SystemTransferSol";
import type { Authority } from "@/primitives";

const SOURCE = address("11111111111111111111111111111112") as Authority;
const DESTINATION = address("11111111111111111111111111111113");

describe("buildTransferSolIx", () => {
  it("puts the funder first as a writable signer and the recipient second", () => {
    const ix = buildTransferSolIx({
      source: SOURCE,
      destination: DESTINATION,
      lamports: 1n,
    });

    expect(ix.accounts).toEqual([
      { address: SOURCE, role: AccountRole.WRITABLE_SIGNER },
      { address: DESTINATION, role: AccountRole.WRITABLE },
    ]);
  });

  it("targets the System Program", () => {
    const ix = buildTransferSolIx({
      source: SOURCE,
      destination: DESTINATION,
      lamports: 1n,
    });

    expect(ix.programAddress).toBe("11111111111111111111111111111111");
  });

  it("encodes the transfer discriminant and lamport amount", () => {
    const ix = buildTransferSolIx({
      source: SOURCE,
      destination: DESTINATION,
      lamports: 123_456_789n,
    });

    const data = new Uint8Array(ix.data);
    expect(data.length).toBe(12);
    expect(getU32Decoder().decode(data.subarray(0, 4))).toBe(2);
    expect(getU64Decoder().decode(data.subarray(4))).toBe(123_456_789n);
  });

  it("rejects a non-positive amount", () => {
    expect(() =>
      buildTransferSolIx({
        source: SOURCE,
        destination: DESTINATION,
        lamports: 0n,
      })
    ).toThrow("Lamports must be greater than 0");
  });
});
