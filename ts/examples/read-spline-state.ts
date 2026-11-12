/**
 * Inspect raw Phoenix account dumps without submitting transactions.
 *
 * bun examples/read-spline-state.ts --gti gti.bin --gti gti-arena-1.bin \
 *   --atb atb.bin --atb atb-arena-1.bin --trader <TRADER_ACCOUNT_ADDRESS>
 *
 * Pass every registered account in arena order, with each header account first.
 */
import { readFile } from "node:fs/promises";
import { parseArgs } from "node:util";
import { address } from "@solana/kit";
import { decodeActiveTraderBuffer, decodeGlobalTraderIndex } from "@/index";

const { values } = parseArgs({
  options: {
    gti: { type: "string", multiple: true },
    atb: { type: "string", multiple: true },
    trader: { type: "string" },
    help: { type: "boolean" },
  },
});

if (
  values.help ||
  !values.gti?.length ||
  !values.atb?.length ||
  !values.trader
) {
  console.log(
    "Usage: bun examples/read-spline-state.ts --gti <HEADER.bin> [--gti <ARENA.bin> ...] --atb <HEADER.bin> [--atb <ARENA.bin> ...] --trader <TRADER_ACCOUNT_ADDRESS>"
  );
} else {
  const traderAddress = address(values.trader);
  const [gtiBuffers, atbBuffers] = await Promise.all([
    Promise.all(values.gti.map((path) => readFile(path))),
    Promise.all(values.atb.map((path) => readFile(path))),
  ]);
  const gti = decodeGlobalTraderIndex(gtiBuffers);
  const atb = decodeActiveTraderBuffer(atbBuffers);
  const trader = gti.entries.find((entry) => entry.trader === traderAddress);
  if (!trader) {
    console.log(
      "Trader is absent from the active index; read its Trader account for cold state."
    );
  } else {
    const entries = atb.entries.filter(
      (entry) => entry.id.traderId === trader.nodeIndex
    );
    console.log(
      JSON.stringify(
        { trader, entries },
        (_key, value) => (typeof value === "bigint" ? value.toString() : value),
        2
      )
    );
  }
}
