import { defineConfig } from "tsdown";

export default defineConfig([
  {
    entry: { index: "./src/index.ts" },
    platform: "neutral",
    dts: { oxc: true },
  },
  {
    entry: { index: "./src/index.ts" },
    outDir: "dist/native",
    platform: "neutral",
    dts: { oxc: true },
    inputOptions: {
      resolve: {
        extensions: [".native.ts", ".tsx", ".ts", ".jsx", ".js", ".json"],
      },
    },
  },
]);
