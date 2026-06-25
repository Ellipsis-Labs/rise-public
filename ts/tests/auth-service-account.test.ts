import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Buffer } from "node:buffer";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PhoenixAuthClient, loadServiceAccountCredentialFromEnv } from "@/auth";

const toBase64Url = (value: unknown): string =>
  Buffer.from(JSON.stringify(value)).toString("base64url");

const makeJwt = (payload: Record<string, unknown>): string =>
  `${toBase64Url({ alg: "none", typ: "JWT" })}.${toBase64Url(payload)}.sig`;

const parseJsonObject = (value: string): Record<string, unknown> => {
  const parsed: unknown = JSON.parse(value);
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Expected JSON object");
  }
  return parsed;
};

const expectStringField = (
  body: Record<string, unknown>,
  field: string
): string => {
  const value = body[field];
  expect(typeof value).toBe("string");
  return value;
};

const privateKey = Buffer.from(new Uint8Array(32).fill(7)).toString(
  "base64url"
);

const authResponse = {
  token_type: "Bearer" as const,
  access_token: makeJwt({
    sid: "sid-service",
    jti: "jti-service",
    exp: Math.floor(Date.now() / 1000) + 900,
  }),
  expires_in: 900,
  refresh_token: "refresh-service",
  refresh_expires_in: 3600,
  pop_key: Buffer.from("pop-key-service").toString("base64url"),
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("service-account credential env loading", () => {
  it("loads split env vars with rust-compatible aliases", async () => {
    await expect(
      loadServiceAccountCredentialFromEnv({
        PHOENIX_SERVICE_ACCOUNT_CLIENT_ID: " service-client ",
        PHOENIX_SERVICE_KEY_ID: " service-key ",
        PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY: ` ${privateKey} `,
      })
    ).resolves.toEqual({
      client_id: "service-client",
      key_id: "service-key",
      private_key: privateKey,
    });
  });

  it("loads credential file path before split env vars", async () => {
    const dir = await mkdtemp(join(tmpdir(), "rise-service-account-"));
    const credentialPath = join(dir, "credential.json");
    await writeFile(
      credentialPath,
      JSON.stringify({
        client_id: "file-client",
        key_id: "file-key",
        public_key: "ignored",
        private_key: privateKey,
      })
    );

    try {
      await expect(
        loadServiceAccountCredentialFromEnv({
          HOME: dir,
          PHOENIX_SERVICE_ACCOUNT_CREDENTIAL: "~/credential.json",
          PHOENIX_SERVICE_ACCOUNT_CLIENT_ID: "split-client",
          PHOENIX_SERVICE_ACCOUNT_KEY_ID: "split-key",
          PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY: privateKey,
        })
      ).resolves.toEqual({
        client_id: "file-client",
        key_id: "file-key",
        public_key: "ignored",
        private_key: privateKey,
      });
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  it("returns null when no service-account env is present", async () => {
    await expect(loadServiceAccountCredentialFromEnv({})).resolves.toBeNull();
  });

  it("fails closed on partial split env vars", async () => {
    await expect(
      loadServiceAccountCredentialFromEnv({
        PHOENIX_SERVICE_ACCOUNT_CLIENT_ID: "service-client",
      })
    ).rejects.toMatchObject({
      name: "PhoenixAuthError",
      code: "incomplete_service_account_credential_env",
    });
  });
});

describe("service-account authentication", () => {
  it("signs the service challenge and stores the resulting session", async () => {
    const fetchMock = vi.fn(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = String(input);
        const body = parseJsonObject(String(init?.body ?? "{}"));

        if (url.endsWith("/v1/auth/login/service/challenge")) {
          expect(body).toEqual({
            client_id: "service-client",
            key_id: "service-key",
          });
          return new Response(
            JSON.stringify({
              nonce: "nonce-1",
              message:
                "Phoenix service login\nnonce:nonce-1\ntimestamp:2026-06-25T12:00:00Z",
              expires_at: "2026-06-25T12:05:00Z",
              key_id: "service-key",
            }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }

        if (url.endsWith("/v1/auth/login/service")) {
          expect(body).toEqual({
            client_id: "service-client",
            key_id: "service-key",
            nonce: "nonce-1",
            timestamp: "2026-06-25T12:00:00Z",
            signature: expect.any(String),
          });
          expect(expectStringField(body, "signature").length).toBeGreaterThan(
            40
          );
          return new Response(JSON.stringify(authResponse), {
            status: 200,
            headers: { "content-type": "application/json" },
          });
        }

        return new Response("not found", { status: 404 });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixAuthClient({ apiUrl: "https://example.com" });
    const session = await client.loginWithServiceAccountCredential({
      client_id: "service-client",
      key_id: "service-key",
      private_key: privateKey,
    });

    expect(session.refreshToken).toBe("refresh-service");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});
