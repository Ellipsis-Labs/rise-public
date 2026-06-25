import { decodeBase64ToBytes, encodeBytesToBase64 } from "@/base64";
import { decodeTrader, type Trader } from "@/accounts";
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
  type InstructionsWithAccountsAndData,
} from "@/primitives";
import {
  buildOnboardTraderDelegatedIxResolved,
  buildRegisterTraderIxResolved,
} from "@/ixs/trader";
import {
  address,
  appendTransactionMessageInstructions,
  compileTransaction,
  createSolanaRpc,
  createTransactionMessage,
  fetchEncodedAccount,
  getBase64EncodedWireTransaction,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
  type Address,
  type Blockhash,
  type Transaction,
  type TransactionWithLifetime,
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

export type ReferralActivationTransaction = Transaction &
  TransactionWithLifetime;

export type ReferralActivationRpc = ReturnType<typeof createSolanaRpc>;

export type ReferralActivationTraderStatus =
  | "missing"
  | "registered"
  | "activated";

export interface ReferralActivationTraderState {
  status: ReferralActivationTraderStatus;
  traderPda: string;
  flags?: number;
  trader?: Trader;
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
   * Include `register_trader` before delegated onboarding. Set this only when
   * the trader account does not already exist.
   */
  includeRegisterTrader?: boolean;
  /**
   * Max positions used by `register_trader` when `includeRegisterTrader` is
   * true. Must be between 32 and 128. Defaults to 128.
   */
  registerTraderMaxPositions?: bigint;
  /**
   * @deprecated Use `feePayer`. When `includeRegisterTrader` is true, this is
   * treated as a fallback fee payer for compatibility with older callers.
   */
  rentPayer?: string | Address;
  lastValidBlockHeight?: bigint;
}

export interface ReferralActivationTransactionBuild {
  requestFields: Omit<ActivateReferralTxRequest, "transaction">;
  transaction: ReferralActivationTransaction;
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
  transaction: ReferralActivationTransaction,
  context: ReferralActivationTransactionSignerContext
) =>
  | ReferralActivationSignedTransaction
  | Promise<ReferralActivationSignedTransaction>;

export interface BuildActivateReferralTxRequestParams extends BuildReferralActivationTransactionParams {
  signTransaction: ReferralActivationTransactionSigner;
  /**
   * Optional Solana RPC client used to inspect the trader account and choose
   * whether to include `register_trader`.
   */
  rpc?: ReferralActivationRpc;
  /**
   * Optional Solana RPC URL. Used only when `rpc` is omitted.
   */
  rpcUrl?: string;
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
  includeRegisterTrader: boolean;
  traderActivationState?: ReferralActivationTraderState;
}

const DEFAULT_TRADER_PDA_INDEX = 0;
const DEFAULT_TRADER_SUBACCOUNT_INDEX = 0;
const MIN_REGISTER_TRADER_MAX_POSITIONS = 32n;
const DEFAULT_REGISTER_TRADER_MAX_POSITIONS = 128n;
const DEFAULT_LAST_VALID_BLOCK_HEIGHT = 0n;
const TRADER_CAPABILITY_CAN_DEPOSIT = 1 << 4;

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

const resolveRegisterTraderMaxPositions = (value?: bigint): bigint => {
  const maxPositions = value ?? DEFAULT_REGISTER_TRADER_MAX_POSITIONS;
  if (
    maxPositions < MIN_REGISTER_TRADER_MAX_POSITIONS ||
    maxPositions > DEFAULT_REGISTER_TRADER_MAX_POSITIONS
  ) {
    throw new Error("registerTraderMaxPositions must be between 32 and 128");
  }
  return maxPositions;
};

export const hasReferralActivationCapabilities = (flags: number): boolean =>
  (flags & TRADER_CAPABILITY_CAN_DEPOSIT) === TRADER_CAPABILITY_CAN_DEPOSIT;

const resolveReferralActivationRpc = (params: {
  rpc?: ReferralActivationRpc;
  rpcUrl?: string;
}): ReferralActivationRpc | undefined => {
  if (params.rpc) return params.rpc;
  if (params.rpcUrl) return createSolanaRpc(params.rpcUrl);
  return undefined;
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

export const getReferralActivationTraderState = async (params: {
  traderAuthority: string | Address;
  exchangeAccounts: ReferralActivationExchangeAccounts;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
  rpc?: ReferralActivationRpc;
  rpcUrl?: string;
}): Promise<ReferralActivationTraderState> => {
  const rpc = resolveReferralActivationRpc(params);
  if (!rpc) {
    throw new Error(
      "RPC client is required to inspect referral activation trader state. Pass rpc or rpcUrl."
    );
  }

  const traderAuthority = toAddress<Authority>(params.traderAuthority);
  const phoenixProgramAddress = toAddress<PhoenixProgramAddress>(
    params.exchangeAccounts.phoenixProgramAddress
  );
  const traderPdaIndex = params.traderPdaIndex ?? DEFAULT_TRADER_PDA_INDEX;
  const traderSubaccountIndex =
    params.traderSubaccountIndex ?? DEFAULT_TRADER_SUBACCOUNT_INDEX;
  const traderPda = await getPhoenixTraderSubaccountAddress({
    authority: traderAuthority,
    traderPdaIndex,
    subaccountIndex: traderSubaccountIndex,
    phoenixProgramAddress,
  });

  const account = await fetchEncodedAccount(rpc, traderPda);
  if (!account.exists || !account.data || account.data.length === 0) {
    return {
      status: "missing",
      traderPda,
    };
  }

  const trader = decodeTrader(account.data);
  const flags = trader.state.flags;
  return {
    status: hasReferralActivationCapabilities(flags)
      ? "activated"
      : "registered",
    traderPda,
    flags,
    trader,
  };
};

export const buildReferralActivationTransaction = async (
  params: BuildReferralActivationTransactionParams
): Promise<ReferralActivationTransactionBuild> => {
  const traderAuthority = toAddress<Authority>(params.traderAuthority);
  const includeRegisterTrader = params.includeRegisterTrader === true;
  const feePayer = toAddress<Authority>(
    params.feePayer ??
      (includeRegisterTrader ? params.rentPayer : undefined) ??
      traderAuthority
  );
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

  const instructions: InstructionsWithAccountsAndData[] = [];
  if (includeRegisterTrader) {
    instructions.push(
      buildRegisterTraderIxResolved({
        exchange,
        trader: {
          payer: feePayer,
          authority: traderAuthority,
          traderAccount,
        },
        maxPositions: resolveRegisterTraderMaxPositions(
          params.registerTraderMaxPositions
        ),
        traderPdaIndex,
        traderSubaccountIndex,
      })
    );
  }

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
  instructions.push(activationIx);

  const message = appendTransactionMessageInstructions(
    instructions,
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
  const traderActivationState =
    params.includeRegisterTrader === undefined &&
    (params.rpc !== undefined || params.rpcUrl !== undefined)
      ? await getReferralActivationTraderState({
          traderAuthority: params.traderAuthority,
          exchangeAccounts: params.exchangeAccounts,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
          rpc: params.rpc,
          rpcUrl: params.rpcUrl,
        })
      : undefined;
  const includeRegisterTrader =
    params.includeRegisterTrader ?? traderActivationState?.status === "missing";
  const build = await buildReferralActivationTransaction({
    ...params,
    permission: params.permission,
    exchangeAccounts: params.exchangeAccounts,
    includeRegisterTrader,
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
    includeRegisterTrader,
    traderActivationState,
  };
};
