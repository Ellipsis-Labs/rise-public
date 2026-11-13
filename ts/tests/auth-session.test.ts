import { describe, expect, it } from "vitest";
import { getRefreshRetryAfterMs } from "@/auth/refreshErrors";
import { fromExternalSnapshot, fromSnapshot } from "@/auth/session";

const toBase64Url = (value: unknown): string =>
  Buffer.from(JSON.stringify(value)).toString("base64url");

const makeJwt = (payload: Record<string, unknown>): string =>
  `${toBase64Url({ alg: "none", typ: "JWT" })}.${toBase64Url(payload)}.sig`;

describe("auth session snapshots", () => {
  it("prefers JWT exp over stale snapshot access expiry", () => {
    const jwtExpiresAt = Math.floor((Date.now() + 30_000) / 1000);
    const session = fromSnapshot({
      sessionId: "sid-1",
      accessToken: makeJwt({
        sid: "sid-1",
        jti: "jti-1",
        exp: jwtExpiresAt,
      }),
      refreshToken: "refresh-token",
      popKey: Buffer.from("pop-key").toString("base64url"),
      accessJti: "jti-1",
      expiresAt: Date.now() + 900_000,
      refreshExpiresAt: Date.now() + 900_000,
      counter: "0",
    });

    expect(session.expiresAt).toBe(jwtExpiresAt * 1000);
  });

  it("accepts external tokens without jti or sid", () => {
    const expiresAt = Date.now() + 60_000;
    const jwtExp = Math.floor(expiresAt / 1000) + 30;
    const base = {
      refreshToken: "",
      popKey: "",
      accessJti: "",
      expiresAt,
      refreshExpiresAt: expiresAt,
      counter: "0",
    };

    const jwtSession = fromExternalSnapshot({
      ...base,
      accessToken: makeJwt({ exp: jwtExp }),
    });
    expect(jwtSession.sessionId).toBe("");
    expect(jwtSession.expiresAt).toBe(jwtExp * 1000);

    const opaqueSession = fromExternalSnapshot({
      ...base,
      accessToken: "opaque",
    });
    expect(opaqueSession.accessToken).toBe("opaque");
    expect(opaqueSession.expiresAt).toBe(expiresAt);
  });
});

describe("auth refresh errors", () => {
  it("ignores non-finite retry-after values", () => {
    expect(getRefreshRetryAfterMs({ retryAfterMs: Number.NaN })).toBeNull();
    expect(
      getRefreshRetryAfterMs({ retryAfterMs: Number.POSITIVE_INFINITY })
    ).toBeNull();
    expect(
      getRefreshRetryAfterMs({ retryAfterSeconds: Number.NaN })
    ).toBeNull();
    expect(
      getRefreshRetryAfterMs({ retryAfterSeconds: Number.POSITIVE_INFINITY })
    ).toBeNull();
    expect(getRefreshRetryAfterMs({ retryAfterSeconds: 2 })).toBe(2_000);
  });
});
