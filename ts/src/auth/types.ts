import z from "zod";
import type { AuthSession, AuthSessionSnapshot } from "./session";
import type { AuthSessionManager } from "./manager";
import type { AuthSessionStorage } from "./storage";

export interface AuthResponse {
  token_type: "Bearer";
  access_token: string;
  expires_in: number;
  refresh_token: string;
  refresh_expires_in: number;
  pop_key: string;
}

export const AuthResponseSchema: z.ZodType<AuthResponse> = z.object({
  token_type: z.literal("Bearer"),
  access_token: z.string(),
  expires_in: z.number(),
  refresh_token: z.string(),
  refresh_expires_in: z.number(),
  pop_key: z.string(),
});

export interface PrivyLoginRequest {
  privy_token: string;
}

export interface WalletLoginRequest {
  wallet_pubkey: string;
  signature: string;
  nonce_id: string;
}

export interface ServiceChallengeRequest {
  client_id: string;
  key_id?: string;
}

export interface ServiceLoginRequest {
  client_id: string;
  key_id?: string;
  nonce: string;
  timestamp: string;
  signature: string;
}

export interface RefreshRequest {
  refresh_token: string;
}

export interface WalletNonceResponse {
  nonce_id: string;
  message: string;
  expires_at: string;
}

export const WalletNonceResponseSchema: z.ZodType<WalletNonceResponse> =
  z.object({
    nonce_id: z.string(),
    message: z.string(),
    expires_at: z.string(),
  });

export interface AuthChallengeResponse {
  nonce: string;
  message: string;
  expires_at: string;
  key_id: string;
  timestamp?: string;
}

export const AuthChallengeResponseSchema: z.ZodType<AuthChallengeResponse> =
  z.object({
    nonce: z.string(),
    message: z.string(),
    expires_at: z.string(),
    key_id: z.string(),
    timestamp: z.string().optional(),
  });

export interface JsonWebKeySet {
  keys: JsonWebKey[];
}

export const JsonWebKeySetSchema: z.ZodType<JsonWebKeySet> = z.object({
  keys: z.array(z.unknown()).transform((keys) => keys as JsonWebKey[]),
});

/**
 * Controls whether the SDK owns session refresh / persistence or relies on an
 * external session owner.
 *
 * `managed` remains supported for compatibility and behaves like the default
 * SDK-owned mode.
 */
export type RiseSessionControl = "managed" | "external";
export type RiseWsAuthMode = "auto" | "authenticated" | "anonymous";

export const isExternalSessionControl = (
  sessionControl?: RiseSessionControl
): boolean => sessionControl === "external";

export interface RiseAuthConfig {
  initialSession?: AuthSession | AuthSessionSnapshot;
  sessionManager?: AuthSessionManager;
  storage?: AuthSessionStorage;
  sessionControl?: RiseSessionControl;
}

export type LogoutResponse = void;
