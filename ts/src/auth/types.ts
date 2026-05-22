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
  /**
   * Solana wallet pubkey the caller is asserting ownership of. Optional for
   * legacy clients, but provide it to establish or confirm the DID to pubkey
   * mapping and obtain wallet-bound, user-scoped authorization. For
   * first-time bindings ownership must be proved either by supplying
   * `wallet_proof` (Solana memo transaction signed by the wallet — works for
   * Ledger) or, absent a proof, by Privy's `linked_accounts` API confirming
   * the wallet is linked to the authenticated user. An unverified pubkey is
   * rejected with `wallet_not_linked`. For returning users, if supplied, it
   * must match the bound pubkey. If omitted when no mapping exists,
   * authentication still succeeds but the issued token will not include a
   * `wallet` claim.
   */
  wallet_pubkey?: string;
  /**
   * Proof of ownership of `wallet_pubkey`, obtained by signing the
   * transaction returned from `POST /v1/auth/privy/wallet-challenge`. Use
   * this path for Ledger and any other wallet that can sign transactions but
   * not arbitrary off-chain messages.
   */
  wallet_proof?: PrivyWalletProof;
}

export interface PrivyWalletProof {
  /** Echo of the `nonce_id` returned from the wallet-challenge endpoint. */
  nonce_id: string;
  /**
   * Base64-encoded fully-signed Solana legacy transaction (bincode wire
   * format), exactly the bytes returned from the challenge endpoint, signed
   * by the wallet whose pubkey is being asserted.
   */
  signed_transaction: string;
}

export interface PrivyWalletChallengeRequest {
  privy_token: string;
  wallet_pubkey: string;
}

export interface PrivyWalletChallengeResponse {
  nonce_id: string;
  /**
   * Base64-encoded unsigned Solana legacy transaction with a single Memo
   * instruction. Sign byte-for-byte with the wallet and return as
   * `wallet_proof.signed_transaction` on the subsequent login call.
   */
  unsigned_transaction: string;
  expires_at: string;
}

export const PrivyWalletChallengeResponseSchema: z.ZodType<PrivyWalletChallengeResponse> =
  z.object({
    nonce_id: z.string(),
    unsigned_transaction: z.string(),
    expires_at: z.string(),
  });

export interface WalletLoginRequest {
  wallet_pubkey: string;
  signature: string;
  nonce_id: string;
}

export interface WalletTransactionChallengeRequest {
  wallet_pubkey: string;
}

export interface WalletTransactionChallengeResponse {
  nonce_id: string;
  /**
   * Base64-encoded unsigned Solana legacy transaction containing a single
   * Memo instruction. Wallets may prepend ComputeBudget instructions before
   * signing (some, like Phantom, inject these automatically) but the Memo
   * instruction, the recent blockhash, and the wallet's required-signer
   * slot must be preserved. The transaction uses a deterministic,
   * non-recent blockhash and is never submitted on-chain.
   */
  unsigned_transaction: string;
  expires_at: string;
}

export const WalletTransactionChallengeResponseSchema: z.ZodType<WalletTransactionChallengeResponse> =
  z.object({
    nonce_id: z.string(),
    unsigned_transaction: z.string(),
    expires_at: z.string(),
  });

export interface WalletTransactionLoginRequest {
  wallet_pubkey: string;
  nonce_id: string;
  /**
   * Base64-encoded fully-signed Solana legacy transaction (bincode wire
   * format), built from the bytes returned by
   * `getWalletTransactionChallenge`. Wallets are permitted to prepend
   * ComputeBudget instructions before signing; any other instruction other
   * than the issued Memo is rejected by the server. The server verifies the
   * signature, the Memo payload, and that the message still uses the issued
   * (deterministic, non-recent) blockhash so the signed bytes cannot be
   * broadcast on-chain.
   */
  signed_transaction: string;
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
