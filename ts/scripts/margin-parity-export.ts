import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import {
  buildMarginParityOutput,
  type MarginParityOutputFile,
  type MarginParitySnapshotFile,
} from "./lib/marginParityExport";

const args = process.argv.slice(2);

const getArg = (name: string): string | undefined => {
  const index = args.indexOf(name);
  if (index === -1) {
    return undefined;
  }
  return args[index + 1];
};

const snapshotFile = getArg("--snapshot-file");
const outputFile = getArg("--output");

if (!snapshotFile || !outputFile) {
  console.error(
    "usage: bun run scripts/margin-parity-export.ts --snapshot-file <file> --output <file>"
  );
  process.exit(1);
}

const snapshot = JSON.parse(
  readFileSync(resolve(snapshotFile), "utf8")
) as MarginParitySnapshotFile;
const output: MarginParityOutputFile = buildMarginParityOutput(snapshot);

mkdirSync(dirname(resolve(outputFile)), { recursive: true });
writeFileSync(resolve(outputFile), `${JSON.stringify(output, null, 2)}\n`);
