import {
  createKeyPairSignerFromPrivateKeyBytes,
  createSignableMessage,
} from "@solana/kit";
import { base64urlnopad } from "@scure/base";
import z from "zod";

import { PhoenixAuthError } from "@/errors";
import type { AuthSession } from "./session";
import type {
  AuthChallengeResponse,
  ServiceChallengeRequest,
  ServiceLoginRequest,
} from "./types";

const ENV_SERVICE_ACCOUNT_CREDENTIAL = "PHOENIX_SERVICE_ACCOUNT_CREDENTIAL";
const ENV_SERVICE_ACCOUNT_CLIENT_ID = "PHOENIX_SERVICE_ACCOUNT_CLIENT_ID";
const ENV_SERVICE_ACCOUNT_KEY_ID = "PHOENIX_SERVICE_ACCOUNT_KEY_ID";
const ENV_SERVICE_ACCOUNT_PRIVATE_KEY = "PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY";
const ENV_SERVICE_CLIENT_ID = "PHOENIX_SERVICE_CLIENT_ID";
const ENV_SERVICE_KEY_ID = "PHOENIX_SERVICE_KEY_ID";
const ENV_SERVICE_PRIVATE_KEY = "PHOENIX_SERVICE_PRIVATE_KEY";

export interface PhoenixServiceAccountCredential {
  client_id: string;
  key_id: string;
  public_key?: string;
  private_key: string;
}

const ServiceAccountCredentialSchema: z.ZodType<PhoenixServiceAccountCredential> =
  z.object({
    client_id: z.string().min(1),
    key_id: z.string().min(1),
    public_key: z.string().optional(),
    private_key: z.string().min(1),
  });

export interface PhoenixServiceAccountAuthSigner {
  clientId: string;
  keyId?: string;
  signChallenge(challenge: AuthChallengeResponse): Promise<string>;
}

export interface PhoenixServiceAccountAuthClient {
  serviceChallenge(
    request: ServiceChallengeRequest
  ): Promise<AuthChallengeResponse>;
  loginWithServiceSignature(request: ServiceLoginRequest): Promise<AuthSession>;
}

export type PhoenixServiceAccountCredentialEnv = Readonly<
  Record<string, string | undefined>
>;

const readFirstTrimmedEnv = (
  env: PhoenixServiceAccountCredentialEnv,
  names: readonly string[]
): string | undefined => {
  for (const name of names) {
    const value = env[name]?.trim();
    if (value) return value;
  }
  return undefined;
};

const currentEnv = (): PhoenixServiceAccountCredentialEnv => {
  const processLike = globalThis as typeof globalThis & {
    process?: { env?: PhoenixServiceAccountCredentialEnv };
  };
  return processLike.process?.env ?? {};
};

const expandHomePath = (
  path: string,
  env: PhoenixServiceAccountCredentialEnv
): string => {
  if (path !== "~" && !path.startsWith("~/")) {
    return path;
  }

  const home = env.HOME?.trim();
  if (!home) {
    throw new PhoenixAuthError(
      "Unable to resolve home directory",
      "home_dir_unavailable"
    );
  }
  return path === "~" ? home : `${home}/${path.slice(2)}`;
};

export const loadServiceAccountCredentialFromPath = async (
  path: string
): Promise<PhoenixServiceAccountCredential> =>
  loadServiceAccountCredentialFromResolvedPath(path, currentEnv());

export const loadServiceAccountCredentialFromEnv = async (
  env: PhoenixServiceAccountCredentialEnv = currentEnv()
): Promise<PhoenixServiceAccountCredential | null> => {
  const credentialPath = readFirstTrimmedEnv(env, [
    ENV_SERVICE_ACCOUNT_CREDENTIAL,
  ]);
  if (credentialPath) {
    return loadServiceAccountCredentialFromResolvedPath(credentialPath, env);
  }

  const clientId = readFirstTrimmedEnv(env, [
    ENV_SERVICE_ACCOUNT_CLIENT_ID,
    ENV_SERVICE_CLIENT_ID,
  ]);
  const keyId = readFirstTrimmedEnv(env, [
    ENV_SERVICE_ACCOUNT_KEY_ID,
    ENV_SERVICE_KEY_ID,
  ]);
  const privateKey = readFirstTrimmedEnv(env, [
    ENV_SERVICE_ACCOUNT_PRIVATE_KEY,
    ENV_SERVICE_PRIVATE_KEY,
  ]);

  if (!clientId && !keyId && !privateKey) {
    return null;
  }

  if (clientId && keyId && privateKey) {
    return {
      client_id: clientId,
      key_id: keyId,
      private_key: privateKey,
    };
  }

  const missing: string[] = [];
  if (!clientId) {
    missing.push(
      `${ENV_SERVICE_ACCOUNT_CLIENT_ID} (or ${ENV_SERVICE_CLIENT_ID})`
    );
  }
  if (!keyId) {
    missing.push(`${ENV_SERVICE_ACCOUNT_KEY_ID} (or ${ENV_SERVICE_KEY_ID})`);
  }
  if (!privateKey) {
    missing.push(
      `${ENV_SERVICE_ACCOUNT_PRIVATE_KEY} (or ${ENV_SERVICE_PRIVATE_KEY})`
    );
  }
  throw new PhoenixAuthError(
    `Incomplete service-account credential env; missing: ${missing.join(", ")}`,
    "incomplete_service_account_credential_env"
  );
};

const loadServiceAccountCredentialFromResolvedPath = async (
  path: string,
  env: PhoenixServiceAccountCredentialEnv
): Promise<PhoenixServiceAccountCredential> => {
  try {
    const { readFile } = await loadNodeFsPromises();
    const contents = await readFile(expandHomePath(path, env), "utf8");
    return ServiceAccountCredentialSchema.parse(JSON.parse(contents));
  } catch (error) {
    if (error instanceof PhoenixAuthError) throw error;
    throw new PhoenixAuthError(
      error instanceof Error
        ? `Failed to load service-account credential: ${error.message}`
        : "Failed to load service-account credential",
      "service_account_credential_load_failed",
      { cause: error }
    );
  }
};

type NodeReadTextFile = (path: string, encoding: "utf8") => Promise<string>;

interface NodeFsPromisesModule {
  readFile: NodeReadTextFile;
}

const isNodeFsPromisesModule = (
  value: unknown
): value is NodeFsPromisesModule =>
  value !== null &&
  typeof value === "object" &&
  "readFile" in value &&
  typeof value.readFile === "function";

const loadNodeFsPromises = async (): Promise<NodeFsPromisesModule> => {
  const module: unknown = await import(nodeFsPromisesModuleSpecifier());
  if (!isNodeFsPromisesModule(module)) {
    throw new PhoenixAuthError(
      "Node fs/promises module is unavailable",
      "service_account_credential_file_unavailable"
    );
  }
  return module;
};

const nodeFsPromisesModuleSpecifier = (): string =>
  ["node:", "fs/promises"].join("");

const decodePrivateKey = (privateKey: string): Uint8Array => {
  try {
    const bytes = base64urlnopad.decode(privateKey);
    if (bytes.length !== 32) {
      throw new Error(`expected 32 bytes, got ${bytes.length}`);
    }
    return bytes;
  } catch (error) {
    throw new PhoenixAuthError(
      error instanceof Error
        ? `Invalid service-account private key: ${error.message}`
        : "Invalid service-account private key",
      "invalid_service_account_private_key",
      { cause: error }
    );
  }
};

const missingServiceAccountSignatureError = (): PhoenixAuthError =>
  new PhoenixAuthError(
    "Service-account signer did not return a message signature",
    "service_account_signature_missing"
  );

export const createServiceAccountAuthSigner = async (
  credential: PhoenixServiceAccountCredential
): Promise<PhoenixServiceAccountAuthSigner> => {
  const signer = await createKeyPairSignerFromPrivateKeyBytes(
    decodePrivateKey(credential.private_key)
  );

  return {
    clientId: credential.client_id,
    keyId: credential.key_id,
    async signChallenge(challenge) {
      const [signature] = await signer.signMessages([
        createSignableMessage(challenge.message),
      ]);
      if (!signature) {
        throw missingServiceAccountSignatureError();
      }
      const bytes = signature[signer.address];
      if (!bytes) {
        throw missingServiceAccountSignatureError();
      }
      return base64urlnopad.encode(bytes);
    },
  };
};

export const loginWithServiceAccountSigner = async (
  client: PhoenixServiceAccountAuthClient,
  signer: PhoenixServiceAccountAuthSigner
): Promise<AuthSession> => {
  const challenge = await client.serviceChallenge({
    client_id: signer.clientId,
    ...(signer.keyId ? { key_id: signer.keyId } : {}),
  });
  if (!challenge.timestamp) {
    throw new PhoenixAuthError(
      "Service-account challenge is missing a timestamp",
      "missing_timestamp_in_challenge"
    );
  }

  return client.loginWithServiceSignature({
    client_id: signer.clientId,
    key_id: challenge.key_id,
    nonce: challenge.nonce,
    timestamp: challenge.timestamp,
    signature: await signer.signChallenge(challenge),
  });
};

export const loginWithServiceAccountCredential = async (
  client: PhoenixServiceAccountAuthClient,
  credential: PhoenixServiceAccountCredential
): Promise<AuthSession> =>
  loginWithServiceAccountSigner(
    client,
    await createServiceAccountAuthSigner(credential)
  );

export const loginWithServiceAccountFromEnv = async (
  client: PhoenixServiceAccountAuthClient,
  env?: PhoenixServiceAccountCredentialEnv
): Promise<AuthSession | null> => {
  const credential = await loadServiceAccountCredentialFromEnv(env);
  if (!credential) return null;
  return loginWithServiceAccountCredential(client, credential);
};
