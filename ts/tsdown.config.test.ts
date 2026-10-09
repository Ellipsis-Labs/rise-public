import { readFile, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { base64urlnopad } from "@scure/base";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { build, Rolldown } from "tsdown";

import * as source from "@/index";

const privateKey = base64urlnopad.encode(new Uint8Array(32).fill(7));
const credential = {
  client_id: "service-client",
  key_id: "service-key",
  private_key: privateKey,
};

beforeAll(async () => {
  await build({ config: "tsdown.config.ts", logLevel: "silent" });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("built platform entries", () => {
  it("preserves every root and auth export and the public declaration contract", async () => {
    const defaultEntry = await import("./dist/index.js");
    const nativeEntry = await import("./dist/native/index.js");

    expect(Object.keys(defaultEntry).sort()).toEqual(
      Object.keys(source).sort()
    );
    expect(Object.keys(nativeEntry).sort()).toEqual(
      Object.keys(defaultEntry).sort()
    );
    expect(Object.keys(nativeEntry.auth).sort()).toEqual(
      Object.keys(defaultEntry.auth).sort()
    );
    const defaultTypes = await readFile(
      new URL("./dist/index.d.ts", import.meta.url),
      "utf8"
    );
    const nativeTypes = await readFile(
      new URL("./dist/native/index.d.ts", import.meta.url),
      "utf8"
    );
    expect(nativeTypes).toBe(defaultTypes);
  });

  it("selects the native root before default and preserves default metadata", async () => {
    const { default: metadata } = await import("./package.json");

    expect(metadata.exports["."]).toEqual({
      types: "./dist/index.d.ts",
      "react-native": "./dist/native/index.js",
      default: "./dist/index.js",
    });
    expect(Object.keys(metadata.exports["."])).toEqual([
      "types",
      "react-native",
      "default",
    ]);
    expect(metadata.main).toBe("./dist/index.js");
    expect(metadata.module).toBe("./dist/index.js");
    expect(metadata.types).toBe("./dist/index.d.ts");
  });

  it.each([
    { condition: "react-native", entry: "./dist/native/index.js" },
    { condition: "browser", entry: "./dist/index.js" },
    { condition: "node", entry: "./dist/index.js" },
  ])(
    "resolves the public package root for $condition",
    async ({ condition, entry }) => {
      const { default: metadata } = await import("./package.json");
      const bundle = await Rolldown.rolldown({
        input: metadata.name,
        external: Object.keys(metadata.dependencies),
        resolve: { conditionNames: [condition, "import", "default"] },
      });

      try {
        const output = await bundle.generate({ format: "esm" });
        const files = await bundle.watchFiles;
        expect(files).toContain(fileURLToPath(new URL(entry, import.meta.url)));
        if (condition === "react-native") {
          expect(files).not.toContain(
            fileURLToPath(new URL("./dist/index.js", import.meta.url))
          );
          for (const chunk of output.output) {
            if (chunk.type === "chunk") {
              expect(chunk.code).not.toMatch(
                /node:|fs\/promises|nodeFsPromises/
              );
            }
          }
        }
      } finally {
        await bundle.close();
      }
    }
  );

  it("excludes the filesystem loader from native output and keeps default loading lazy", async () => {
    const defaultCode = await readFile(
      new URL("./dist/index.js", import.meta.url),
      "utf8"
    );
    const nativeDirectory = new URL("./dist/native/", import.meta.url);
    const nativeFiles = await readdir(nativeDirectory);
    const nativeChunks = await Promise.all(
      nativeFiles
        .filter((file) => file.endsWith(".js"))
        .map((file) => readFile(new URL(file, nativeDirectory), "utf8"))
    );
    const nativeCode = nativeChunks.join("\n");

    expect(defaultCode).toContain(
      "await import(nodeFsPromisesModuleSpecifier())"
    );
    expect(nativeCode).not.toMatch(/node:|fs\/promises|nodeFsPromises/);
    expect(nativeCode).toContain("service_account_credential_file_unavailable");
  });

  it("rejects native file credentials without falling back to split env credentials", async () => {
    const nativeEntry = await import("./dist/native/index.js");

    await expect(
      nativeEntry.loadServiceAccountCredentialFromPath("credential.json")
    ).rejects.toBeInstanceOf(nativeEntry.PhoenixAuthError);
    await expect(
      nativeEntry.loadServiceAccountCredentialFromEnv({
        PHOENIX_SERVICE_ACCOUNT_CREDENTIAL: "~/credential.json",
        PHOENIX_SERVICE_ACCOUNT_CLIENT_ID: credential.client_id,
        PHOENIX_SERVICE_ACCOUNT_KEY_ID: credential.key_id,
        PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY: credential.private_key,
      })
    ).rejects.toMatchObject({
      code: "service_account_credential_file_unavailable",
    });
  });

  it("preserves native absent, partial, canonical and legacy env semantics", async () => {
    const nativeEntry = await import("./dist/native/index.js");

    await expect(
      nativeEntry.loadServiceAccountCredentialFromEnv({})
    ).resolves.toBeNull();
    await expect(
      nativeEntry.loadServiceAccountCredentialFromEnv({
        PHOENIX_SERVICE_ACCOUNT_CLIENT_ID: credential.client_id,
      })
    ).rejects.toMatchObject({
      code: "incomplete_service_account_credential_env",
    });
    await expect(
      nativeEntry.loadServiceAccountCredentialFromEnv({
        PHOENIX_SERVICE_ACCOUNT_CLIENT_ID: ` ${credential.client_id} `,
        PHOENIX_SERVICE_CLIENT_ID: "legacy-client",
        PHOENIX_SERVICE_KEY_ID: ` ${credential.key_id} `,
        PHOENIX_SERVICE_PRIVATE_KEY: ` ${credential.private_key} `,
      })
    ).resolves.toEqual(credential);
    vi.stubGlobal("process", undefined);
    await expect(
      nativeEntry.loadServiceAccountCredentialFromEnv()
    ).resolves.toBeNull();
  });

  it("preserves explicit native credential validation and signing", async () => {
    const nativeEntry = await import("./dist/native/index.js");
    const signer = await nativeEntry.createServiceAccountAuthSigner(credential);

    expect(signer.clientId).toBe(credential.client_id);
    expect(signer.keyId).toBe(credential.key_id);
    await expect(
      signer.signChallenge({
        nonce: "nonce-1",
        message: "Phoenix service login",
        expires_at: "2026-06-25T12:05:00Z",
        key_id: credential.key_id,
        timestamp: "2026-06-25T12:00:00Z",
      })
    ).resolves.toMatch(/^[A-Za-z0-9_-]{86}$/);
    await expect(
      nativeEntry.createServiceAccountAuthSigner({
        ...credential,
        private_key: "invalid",
      })
    ).rejects.toMatchObject({ code: "invalid_service_account_private_key" });
  });
});
