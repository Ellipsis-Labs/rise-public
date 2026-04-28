import { defineConfig } from "eslint/config";
import js from "@eslint/js";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsparser from "@typescript-eslint/parser";
import unusedImports from "eslint-plugin-unused-imports";

const tsStrictConfigs = tseslint.configs["flat/recommended-type-checked"];
const tsStylisticConfigs = [];

const tsOverrides = {
  files: ["src/**/*.ts"],
  languageOptions: {
    parser: tsparser,
    parserOptions: {
      project: "./tsconfig.json",
      tsconfigRootDir: import.meta.dirname,
    },
  },
  plugins: {
    "unused-imports": unusedImports,
  },
  rules: {
    "no-unused-vars": "off",
    "@typescript-eslint/no-unused-vars": "off",
    "unused-imports/no-unused-imports": "error",
    "unused-imports/no-unused-vars": [
      "warn",
      {
        args: "after-used",
        argsIgnorePattern: "^_",
        varsIgnorePattern: "^_",
        caughtErrorsIgnorePattern: "^_",
        ignoreRestSiblings: true,
      },
    ],
    "@typescript-eslint/no-explicit-any": "error",
    "@typescript-eslint/consistent-type-imports": "error",
    "@typescript-eslint/restrict-template-expressions": [
      "warn",
      { allowNumber: true, allowBoolean: true, allowNullish: true },
    ],
    "@typescript-eslint/require-await": "off",
    "prefer-const": "error",
    "no-var": "error",
  },
};

export default defineConfig([
  {
    ignores: ["dist/**", "*.config.ts", "*.config.mjs"],
  },
  js.configs.recommended,
  ...tsStrictConfigs,
  ...tsStylisticConfigs,
  tsOverrides,
]);
