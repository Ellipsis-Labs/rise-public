import { decodeBase64ToBytes, encodeBytesToBase64 } from "@/base64";
import {
  getPhoenixLogAuthorityAddress,
  getPhoenixTraderSubaccountAddress,
} from "@/pdas";
import { generateReadonlySignerAccount } from "@/core/utils/accountMeta";
import {
  type ActiveTraderBufferAddressArray,
  type Authority,
  type GlobalConfigurationAddress,
  type GlobalTraderIndexAddressArray,
  type LogAuthorityAddress,
  type PhoenixProgramAddress,
} from "@/primitives";
import { buildOnboardTraderDelegatedIxResolved } from "@/ixs/trader";
import {
  address,
  appendTransactionMessageInstructions,
  compileTransaction,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
  type Address,
  type Blockhash,
  type Transaction,
} from "@solana/kit";
import type { ExchangeStateSnapshot } from "../exchange";
import type {
  ActivateReferralTxRequest,
  ReferralActivationPermissionResponse,
} from "./types";

export interface ReferralActivationExchangeAccounts {
  phoenixProgramAddress: string;
  logAuthorityAddress: string;
  globalConfigurationAddress: string;
  globalTraderIndex: readonly string[];
  activeTraderBuffer: readonly string[];
}

export interface BuildReferralActivationTransactionParams {
  referralCode: string;
  traderAuthority: string | Address;
  recentBlockhash: string | Blockhash;
  permission: ReferralActivationPermissionResponse;
  exchangeAccounts: ReferralActivationExchangeAccounts;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
  feePayer?: string | Address;
  /**
   * @deprecated Referral activate-tx no longer creates trader accounts, so no
   * rent payer is used. Register the trader before building this request.
   */
  rentPayer?: string | Address;
  lastValidBlockHeight?: bigint;
}

export interface ReferralActivationTransactionBuild {
  requestFields: Omit<ActivateReferralTxRequest, "transaction">;
  transaction: Transaction;
  unsignedTransactionBase64: string;
  /**
   * Full unsigned wire transaction bytes. Browser wallet adapters that use
   * web3.js can deserialize these bytes into a VersionedTransaction, sign it,
   * and return the signed object from the signer callback.
   */
  unsignedTransactionBytes: Uint8Array;
  traderPda: string;
}

export interface SerializableTransactionLike {
  serialize(): Uint8Array | ArrayLike<number>;
}

export type ReferralActivationSignedTransaction =
  | Transaction
  | string
  | Uint8Array
  | ArrayBuffer
  | SerializableTransactionLike;

export type ReferralActivationTransactionSignerContext =
  ReferralActivationTransactionBuild;

export type ReferralActivationTransactionSigner = (
  transaction: Transaction,
  context: ReferralActivationTransactionSignerContext
) =>
  | ReferralActivationSignedTransaction
  | Promise<ReferralActivationSignedTransaction>;

export interface BuildActivateReferralTxRequestParams extends BuildReferralActivationTransactionParams {
  signTransaction: ReferralActivationTransactionSigner;
}

export interface BuildActivateReferralTxRequestWithClientParams extends Omit<
  BuildActivateReferralTxRequestParams,
  "permission" | "exchangeAccounts"
> {
  permission?: ReferralActivationPermissionResponse;
  exchangeAccounts?: ReferralActivationExchangeAccounts;
}

export interface BuildActivateReferralTxRequestResult extends ReferralActivationTransactionBuild {
  request: ActivateReferralTxRequest;
  signedTransactionBase64: string;
}

const DEFAULT_TRADER_PDA_INDEX = 0;
const DEFAULT_TRADER_SUBACCOUNT_INDEX = 0;
const DEFAULT_LAST_VALID_BLOCK_HEIGHT = 0n;

const toAddress = <T extends Address>(value: string | Address): T =>
  address(value) as T;

const toNonEmptyAddressArray = <T extends [Address, ...Address[]]>(
  values: readonly string[],
  fieldName: string
): T => {
  if (values.length === 0) {
    throw new Error(`${fieldName} must include at least one address`);
  }
  return values.map((value) => address(value)) as unknown as T;
};

const isKitTransaction = (value: unknown): value is Transaction => {
  if (typeof value !== "object" || value === null) return false;
  return "messageBytes" in value && "signatures" in value;
};

const isSerializableTransactionLike = (
  value: unknown
): value is SerializableTransactionLike => {
  if (typeof value !== "object" || value === null) return false;
  return "serialize" in value && typeof value.serialize === "function";
};

const toUint8Array = (value: ArrayBuffer | ArrayLike<number>): Uint8Array => {
  if (value instanceof Uint8Array) return value;
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  return Uint8Array.from(value);
};

export const serializeReferralActivationSignedTransaction = (
  transaction: ReferralActivationSignedTransaction
): string => {
  if (typeof transaction === "string") return transaction;
  if (isKitTransaction(transaction)) {
    return getBase64EncodedWireTransaction(transaction);
  }
  if (transaction instanceof Uint8Array || transaction instanceof ArrayBuffer) {
    return encodeBytesToBase64(toUint8Array(transaction));
  }
  if (isSerializableTransactionLike(transaction)) {
    return encodeBytesToBase64(toUint8Array(transaction.serialize()));
  }

  throw new Error(
    "Unsupported signed transaction value. Return a base64 string, Kit transaction, bytes, or an object with serialize()."
  );
};

export const referralActivationExchangeAccountsFromSnapshot = async (
  snapshot: ExchangeStateSnapshot
): Promise<ReferralActivationExchangeAccounts> => {
  const phoenixProgramAddress = address(
    snapshot.programId
  ) as PhoenixProgramAddress;
  return {
    phoenixProgramAddress,
    logAuthorityAddress: await getPhoenixLogAuthorityAddress(
      phoenixProgramAddress
    ),
    globalConfigurationAddress: snapshot.globalConfig,
    globalTraderIndex: snapshot.globalTraderIndex,
    activeTraderBuffer: snapshot.activeTraderBuffer,
  };
};

export const buildReferralActivationTransaction = async (
  params: BuildReferralActivationTransactionParams
): Promise<ReferralActivationTransactionBuild> => {
  const traderAuthority = toAddress<Authority>(params.traderAuthority);
  const feePayer = toAddress<Address>(params.feePayer ?? traderAuthority);
  const traderPdaIndex = params.traderPdaIndex ?? DEFAULT_TRADER_PDA_INDEX;
  const traderSubaccountIndex =
    params.traderSubaccountIndex ?? DEFAULT_TRADER_SUBACCOUNT_INDEX;

  const exchange = {
    phoenixProgramAddress: toAddress<PhoenixProgramAddress>(
      params.exchangeAccounts.phoenixProgramAddress
    ),
    logAuthorityAddress: toAddress<LogAuthorityAddress>(
      params.exchangeAccounts.logAuthorityAddress
    ),
    globalConfigurationAddress: toAddress<GlobalConfigurationAddress>(
      params.exchangeAccounts.globalConfigurationAddress
    ),
    globalTraderIndex: toNonEmptyAddressArray<GlobalTraderIndexAddressArray>(
      params.exchangeAccounts.globalTraderIndex,
      "globalTraderIndex"
    ),
    activeTraderBuffer: toNonEmptyAddressArray<ActiveTraderBufferAddressArray>(
      params.exchangeAccounts.activeTraderBuffer,
      "activeTraderBuffer"
    ),
  };

  const traderAccount = await getPhoenixTraderSubaccountAddress({
    authority: traderAuthority,
    traderPdaIndex,
    subaccountIndex: traderSubaccountIndex,
    phoenixProgramAddress: exchange.phoenixProgramAddress,
  });

  const onboardTraderIx = buildOnboardTraderDelegatedIxResolved({
    exchange,
    trader: {
      authority: toAddress<Authority>(params.permission.trader_onboarder),
      permissionAccount: toAddress<Address>(
        params.permission.permission_account
      ),
      traderAccount,
    },
  });
  const activationIx =
    feePayer === traderAuthority
      ? onboardTraderIx
      : {
          ...onboardTraderIx,
          accounts: [
            ...onboardTraderIx.accounts,
            generateReadonlySignerAccount(traderAuthority),
          ],
        };

  const message = appendTransactionMessageInstructions(
    [activationIx],
    setTransactionMessageLifetimeUsingBlockhash(
      {
        blockhash: params.recentBlockhash as Blockhash,
        lastValidBlockHeight:
          params.lastValidBlockHeight ?? DEFAULT_LAST_VALID_BLOCK_HEIGHT,
      },
      setTransactionMessageFeePayer(
        feePayer,
        createTransactionMessage({ version: 0 })
      )
    )
  );
  const transaction = compileTransaction(message);
  const unsignedTransactionBase64 =
    getBase64EncodedWireTransaction(transaction);

  return {
    requestFields: {
      referral_code: params.referralCode,
      trader_authority: traderAuthority,
      trader_pda_index: traderPdaIndex,
      trader_subaccount_index: traderSubaccountIndex,
      recent_blockhash: params.recentBlockhash,
    },
    transaction,
    unsignedTransactionBase64,
    unsignedTransactionBytes: decodeBase64ToBytes(unsignedTransactionBase64),
    traderPda: traderAccount,
  };
};

export const buildActivateReferralTxRequest = async (
  params: BuildActivateReferralTxRequestParams
): Promise<BuildActivateReferralTxRequestResult> => {
  const build = await buildReferralActivationTransaction({
    ...params,
    permission: params.permission,
    exchangeAccounts: params.exchangeAccounts,
  });
  const signedTransaction = await params.signTransaction(
    build.transaction,
    build
  );
  const signedTransactionBase64 =
    serializeReferralActivationSignedTransaction(signedTransaction);

  return {
    ...build,
    request: {
      ...build.requestFields,
      transaction: signedTransactionBase64,
    },
    signedTransactionBase64,
  };
};
