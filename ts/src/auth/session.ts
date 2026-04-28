import { base64urlnopad } from "@scure/base";

type AuthResponseLike = {
  access_token: string;
  refresh_token: string;
  refresh_expires_in: number;
  pop_key: string;
};

export interface AuthSession {
  sessionId: string;
  accessToken: string;
  refreshToken: string;
  popKeyBytes: Uint8Array;
  accessJti: string;
  expiresAt: number;
  refreshExpiresAt: number;
  counter: bigint;
}

export interface AuthSessionSnapshot {
  sessionId?: string;
  accessToken: string;
  refreshToken: string;
  popKey: string;
  accessJti: string;
  expiresAt: number;
  refreshExpiresAt: number;
  counter: string;
}

const decodeJwtPayload = (token: string): Record<string, unknown> => {
  const parts = token.split(".");
  if (parts.length !== 3) {
    throw new Error("Invalid JWT format");
  }
  const payloadBytes = base64urlnopad.decode(parts[1]);
  const payload = new TextDecoder().decode(payloadBytes);
  return JSON.parse(payload) as Record<string, unknown>;
};

export const resetFromAuthResponse = (
  response: AuthResponseLike
): AuthSession => {
  const payload = decodeJwtPayload(response.access_token);
  const jti = payload.jti;
  const sid = payload.sid;
  const exp = payload.exp;

  if (typeof jti !== "string") throw new Error("Missing jti in access token");
  if (typeof sid !== "string") throw new Error("Missing sid in access token");
  if (typeof exp !== "number") throw new Error("Missing exp in access token");

  const popKeyBytes = base64urlnopad.decode(response.pop_key);
  const nowMs = Date.now();

  return {
    sessionId: sid,
    accessToken: response.access_token,
    refreshToken: response.refresh_token,
    popKeyBytes: Uint8Array.from(popKeyBytes),
    accessJti: jti,
    expiresAt: exp * 1000,
    refreshExpiresAt: nowMs + response.refresh_expires_in * 1000,
    counter: 0n,
  };
};

export const advanceCounter = (session: AuthSession): AuthSession => ({
  ...session,
  counter: session.counter + 1n,
});

export const isAccessExpired = (session: AuthSession): boolean =>
  Date.now() >= session.expiresAt;

export const isAccessExpiringWithin = (
  session: AuthSession,
  withinMs: number
): boolean => Date.now() + withinMs >= session.expiresAt;

export const isRefreshExpired = (session: AuthSession): boolean =>
  Date.now() >= session.refreshExpiresAt;

export const toSnapshot = (session: AuthSession): AuthSessionSnapshot => ({
  sessionId: session.sessionId,
  accessToken: session.accessToken,
  refreshToken: session.refreshToken,
  popKey: base64urlnopad.encode(session.popKeyBytes),
  accessJti: session.accessJti,
  expiresAt: session.expiresAt,
  refreshExpiresAt: session.refreshExpiresAt,
  counter: session.counter.toString(10),
});

export const fromSnapshot = (snapshot: AuthSessionSnapshot): AuthSession => {
  const payload = decodeJwtPayload(snapshot.accessToken);
  const sid =
    typeof snapshot.sessionId === "string"
      ? snapshot.sessionId
      : typeof payload.sid === "string"
        ? payload.sid
        : null;

  if (!sid) {
    throw new Error("Missing sid in access token snapshot");
  }

  return {
    sessionId: sid,
    accessToken: snapshot.accessToken,
    refreshToken: snapshot.refreshToken,
    popKeyBytes: Uint8Array.from(base64urlnopad.decode(snapshot.popKey)),
    accessJti: snapshot.accessJti,
    expiresAt: snapshot.expiresAt,
    refreshExpiresAt: snapshot.refreshExpiresAt,
    counter: BigInt(snapshot.counter),
  };
};
