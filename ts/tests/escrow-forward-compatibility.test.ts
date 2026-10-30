import { describe, expect, it } from "vitest";

import { PendingEscrowRequestViewSchema } from "@/api/traders/types";
import { TraderStateServerMessageSchema } from "@/api/traders/traderState";
import { createEscrowActionSchema } from "@/api/utils/escrowAction";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import z from "zod";

describe("escrow forward compatibility", () => {
  it("keeps REST requests containing unfamiliar actions", () => {
    const request = PendingEscrowRequestViewSchema.parse({
      sequenceNumber: 7,
      senderAuthority: "sender",
      senderPdaIndex: 0,
      senderSubaccountIndex: 0,
      receiverPdaIndex: 0,
      receiverSubaccountIndex: 0,
      expirationOffset: null,
      initialSlot: 10,
      actions: [
        { kind: "cash", amount: 5 },
        { kind: "futureEscrowAction", newField: { value: true } },
        { kind: "lockedDeposit", amount: 100 },
      ],
    });
    expect(request.actions).toEqual([
      { kind: "cash", amount: 5 },
      { kind: "unknown" },
      { kind: "lockedDeposit", amount: 100 },
    ]);
  });

  it("keeps known delta fields when an escrow action is unfamiliar", () => {
    const message = {
      channel: "traderState",
      authority: "authority",
      traderPdaIndex: 0,
      slot: "11",
      messageType: "delta",
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "105",
          futureDeltaField: { value: "200" },
          escrowRequests: [
            {
              sequenceNumber: "7",
              senderAuthority: "sender",
              senderPdaIndex: 0,
              senderSubaccountIndex: 0,
              expirationOffset: null,
              initialSlot: "10",
              expirationSlot: null,
              actions: [
                { kind: "futureEscrowAction", amount: "200" },
                { kind: "cash", amount: "5" },
              ],
            },
          ],
        },
      ],
    };
    const parsed = TraderStateServerMessageSchema.parse(message);
    expect(parsed.deltas?.[0]).toMatchObject({
      sequence: 2,
      collateral: "105",
      escrowRequests: [
        { actions: [{ kind: "unknown" }, { kind: "cash", amount: "5" }] },
      ],
    });

    // Strict mode can diagnose added fields while still accepting unknown actions.
    const { futureDeltaField: _, ...delta } = message.deltas[0];
    expect(
      applyStrictModeRecursive(TraderStateServerMessageSchema).parse({
        ...message,
        deltas: [delta],
      }).deltas?.[0].escrowRequests?.[0].actions[0]
    ).toEqual({ kind: "unknown" });
  });

  it("rejects malformed known actions and invalid discriminators", () => {
    const schemas = [
      createEscrowActionSchema(z.number()),
      createEscrowActionSchema(z.string().regex(/^\d+$/)),
    ];
    for (const schema of schemas) {
      for (const action of [
        { kind: "cash" },
        { kind: "lockedDeposit", amount: {} },
        { kind: 123, amount: 5 },
        { amount: 5 },
        null,
      ]) {
        expect(schema.safeParse(action).success).toBe(false);
      }
    }
  });
});
