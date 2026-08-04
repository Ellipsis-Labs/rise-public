/**
 * Native SOL spot collateral instructions.
 *
 * Native SOL can be posted as collateral alongside the canonical quote token.
 * Its lamports live in the trader account itself, above the rent floor, and the
 * *accounted* portion is tracked as a header-extension entry in the trader
 * position map. Lamports beyond the accounted balance ("excess") carry no
 * margin value and can be swept out freely.
 *
 * Admin instructions (configure, activate) are deliberately not exposed.
 *
 * Two custom program errors are specific to these instructions:
 *
 * - **7100 `FeatureDisabled`** — the exchange has native SOL collateral turned
 *   off, or the asset is not active.
 * - **7101 `PositionAuthoritySwapDisabled`** — a `SwapNative` was signed by a
 *   position authority while either opt-out gate is set. See
 *   `buildSwapNativeIx`.
 */

import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  SPL_TOKEN_PROGRAM_ADDRESS,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import {
  getPhoenixGlobalVaultAddress,
  getPhoenixNativeSolAuthorityAddress,
} from "@/pdas";
import type { AccountMeta, Address, Instruction } from "@solana/kit";
import { AccountRole } from "@solana/kit";
import {
  encodeLiquidateNativeSol,
  encodeSwapNative,
  encodeSyncNative,
  encodeTransferNativeSol,
  encodeTransferNativeSolFromChildToParent,
  encodeWithdrawNativeSol,
} from "./codec";
import type {
  LiquidateNativeSolParams,
  NativeSolAccounts,
  NativeSolIx,
  PackedVenueInstructions,
  SwapNativeParams,
  SyncNativeParams,
  TransferNativeSolFromChildToParentParams,
  TransferNativeSolParams,
  WithdrawNativeSolParams,
  WithdrawNativeSolAction,
} from "./types";

/**
 * The packed venue-instruction encoding addresses accounts with 6 bits, so a
 * swap can reference at most this many distinct accounts.
 */
export const MAX_PACKED_EXTERNAL_ACCOUNTS = 64;

const isSignerRole = (role: AccountRole): boolean =>
  role === AccountRole.READONLY_SIGNER || role === AccountRole.WRITABLE_SIGNER;

const isWritableRole = (role: AccountRole): boolean =>
  role === AccountRole.WRITABLE || role === AccountRole.WRITABLE_SIGNER;

const roleFor = (isSigner: boolean, isWritable: boolean): AccountRole => {
  if (isSigner) {
    return isWritable
      ? AccountRole.WRITABLE_SIGNER
      : AccountRole.READONLY_SIGNER;
  }
  return isWritable ? AccountRole.WRITABLE : AccountRole.READONLY;
};

const makeMeta = (
  address: string,
  isSigner: boolean,
  isWritable: boolean
): AccountMeta => ({
  address: address as Address,
  role: roleFor(isSigner, isWritable),
});

/** Union the signer and writable bits of a repeated account. */
const mergeMeta = (
  meta: AccountMeta,
  isSigner: boolean,
  isWritable: boolean
): AccountMeta =>
  makeMeta(
    meta.address,
    isSignerRole(meta.role) || isSigner,
    isWritableRole(meta.role) || isWritable
  );

const requireIndexAccounts = (params: {
  globalTraderIndex: readonly unknown[];
  activeTraderBuffer: readonly unknown[];
}) => {
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error(
      "Active trader buffer array is required and must not be empty"
    );
  }
};

const actionLamports = (action: WithdrawNativeSolAction): bigint | undefined =>
  action.kind === "allExcess" ? undefined : action.lamports;

/**
 * Reconcile a trader's accounted native SOL collateral against the lamports
 * actually in the account.
 *
 * Permissionless — anyone may crank it. An upward reconciliation is clamped to
 * the per-trader and exchange-wide caps; whatever the clamp leaves over stays
 * in the account as uncounted excess, recoverable by a later sync once headroom
 * frees up.
 */
export const buildSyncNativeIx = (params: SyncNativeParams): NativeSolIx => {
  requireIndexAccounts(params);
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: NativeSolAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // Writable: syncing moves the exchange-wide collateral tally.
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.traderAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return { programAddress, accounts, data: encodeSyncNative() };
};

/**
 * Withdraw native SOL spot collateral.
 *
 * Unlike a quote withdrawal this charges no fee, ignores the deposit cooldown,
 * and is never enqueued: if the exchange-wide throttle cannot absorb the
 * withdrawal's quote-notional value right now, the instruction fails rather
 * than queueing. Excess-only withdrawals bypass the throttle entirely.
 */
export const buildWithdrawNativeSolIx = (
  params: WithdrawNativeSolParams
): NativeSolIx => {
  requireIndexAccounts(params);
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.destination) {
    throw new Error("Destination is required");
  }
  if (params.destination === params.traderAccount) {
    throw new Error(
      "Withdraw destination must be a system account other than the trader account"
    );
  }
  if (!params.withdrawQueue) {
    throw new Error("Withdraw queue is required");
  }
  const lamports = actionLamports(params.action);
  if (lamports !== undefined && lamports <= 0n) {
    throw new Error("Withdraw amount must be greater than 0");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: NativeSolAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    generateWritableAccount(params.destination),
    // The withdraw queue precedes the index arenas here, unlike a quote
    // withdrawal where it trails them.
    generateWritableAccount(params.withdrawQueue),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeWithdrawNativeSol(params.action),
  };
};

/**
 * Move native SOL spot collateral between two of a trader's accounts.
 *
 * The source debit is margin checked. The destination is **rejected**, not
 * clamped, if the credit would push it past the per-trader cap. The
 * exchange-wide tally is unchanged, since the collateral never leaves the
 * protocol.
 *
 * This shares its account layout exactly with a quote collateral transfer.
 */
export const buildTransferNativeSolIx = (
  params: TransferNativeSolParams
): NativeSolIx => {
  requireIndexAccounts(params);
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.srcTraderAccount) {
    throw new Error("Source trader account is required");
  }
  if (!params.dstTraderAccount) {
    throw new Error("Destination trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (params.lamports === undefined || params.lamports <= 0n) {
    throw new Error("Lamports must be greater than 0");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: NativeSolAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // Both the global configuration and the perp asset map are readonly for a
    // transfer, unlike a withdrawal where both are writable.
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.srcTraderAccount),
    generateWritableAccount(params.dstTraderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...(params.permissionAccount
      ? [generateWritableAccount(params.permissionAccount)]
      : []),
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeTransferNativeSol(params.lamports),
  };
};

/**
 * Sweep a flat isolated child account's native SOL spot collateral into its
 * parent.
 *
 * This is a silent no-op — not an error — when the child still has splines,
 * open orders, a position, or a negative quote balance, or when it holds no
 * native SOL.
 */
export const buildTransferNativeSolFromChildToParentIx = (
  params: TransferNativeSolFromChildToParentParams
): NativeSolIx => {
  requireIndexAccounts(params);
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.childTraderAccount) {
    throw new Error("Child trader account is required");
  }
  if (!params.parentTraderAccount) {
    throw new Error("Parent trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const traderSigns = params.traderSigns ?? true;

  const accounts: NativeSolAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    traderSigns
      ? generateReadonlySignerAccount(params.trader)
      : generateReadonlyAccount(params.trader),
    generateWritableAccount(params.childTraderAccount),
    generateWritableAccount(params.parentTraderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeTransferNativeSolFromChildToParent(),
  };
};

/**
 * Pack venue instructions for a swap.
 *
 * Accounts are deduplicated by address, unioning their writable and signer
 * bits, and each instruction's metas become indices into that list.
 *
 * Only `signer` — the swap's signer — may be marked as a signer by a venue
 * instruction: the program has no other key to sign with.
 */
export const packVenueInstructions = (
  signer: string,
  instructions: readonly Instruction[]
): PackedVenueInstructions => {
  const accounts: AccountMeta[] = [];
  const packed: PackedVenueInstructions["instructions"] = [];

  const indexOf = (
    address: string,
    isSigner: boolean,
    isWritable: boolean
  ): number => {
    const existing = accounts.findIndex((meta) => meta.address === address);
    const found = existing >= 0 ? accounts[existing] : undefined;
    if (found) {
      accounts[existing] = mergeMeta(found, isSigner, isWritable);
      return existing;
    }
    if (accounts.length >= MAX_PACKED_EXTERNAL_ACCOUNTS) {
      throw new Error(
        `Venue instructions reference more than ${MAX_PACKED_EXTERNAL_ACCOUNTS} accounts, which the packed encoding cannot address`
      );
    }
    accounts.push(makeMeta(address, isSigner, isWritable));
    return accounts.length - 1;
  };

  for (const instruction of instructions) {
    const programIdIndex = indexOf(instruction.programAddress, false, false);
    const accountMetas = (instruction.accounts ?? []).map((meta) => {
      const isSigner = isSignerRole(meta.role);
      const isWritable = isWritableRole(meta.role);
      if (isSigner && meta.address !== signer) {
        throw new Error("Only the swap signer may sign a venue instruction");
      }
      return {
        index: indexOf(meta.address, isSigner, isWritable),
        isSigner,
        isWritable,
      };
    });

    packed.push({
      programIdIndex,
      data: new Uint8Array(instruction.data ?? new Uint8Array()),
      accountMetas,
    });
  }

  return { instructions: packed, accounts };
};

/**
 * Seize a liquidatee's native SOL collateral at the discounted index price.
 * The signer must be the risk authority or hold `permissionAccount` as a
 * delegated liquidation authority.
 */
export const buildLiquidateNativeSolIx = async (
  params: LiquidateNativeSolParams
): Promise<NativeSolIx> => {
  requireIndexAccounts(params);
  if (!params.signer) throw new Error("Signer is required");
  if (!params.liquidateeAccount)
    throw new Error("Liquidatee account is required");
  if (!params.mint) throw new Error("Mint is required");
  if (!params.perpAssetMap) throw new Error("Perp asset map is required");
  if (!params.signerQuoteTokenAccount) {
    throw new Error("Signer quote token account is required");
  }
  if (!params.withdrawQueue) throw new Error("Withdraw queue is required");
  if (params.maxNativeSolAmount === undefined) {
    throw new Error("Maximum native SOL amount is required");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [globalVault, nativeSolAuthority] = await Promise.all([
    getPhoenixGlobalVaultAddress(params.mint, programAddress),
    getPhoenixNativeSolAuthorityAddress(programAddress),
  ]);
  const extraTraderAccounts = params.extraTraderAccounts ?? [];
  const accounts: NativeSolAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateWritableAccount(globalVault),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(nativeSolAuthority),
    generateWritableSignerAccount(params.signer),
    generateWritableAccount(params.permissionAccount ?? params.signer),
    generateWritableAccount(params.signerQuoteTokenAccount),
    generateWritableAccount(params.liquidateeAccount),
    generateWritableAccount(params.withdrawQueue),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...extraTraderAccounts.map(generateWritableAccount),
    ...params.venue.accounts,
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeLiquidateNativeSol(
      params.maxNativeSolAmount,
      BigInt(extraTraderAccounts.length),
      params.venue.instructions
    ),
  };
};

/**
 * Swap between native SOL spot collateral and quote collateral through an
 * external venue.
 *
 * The program withdraws the input leg to the signer, runs the caller-supplied
 * venue instructions, deposits the output leg, and then checks that the
 * trader's margin state did not worsen. The exchange-wide withdrawal throttle
 * is charged only the swap's *value loss* (slippage plus venue fees), not the
 * full amount moved.
 *
 * ## Slippage
 *
 * `minAmountOut` is the **only** price protection this instruction has — there
 * is no oracle floor. Its unit is the output asset: quote lots for a sell,
 * lamports for a buy. Pass `"unprotected"` to disable it explicitly.
 *
 * ## Delegated authorities
 *
 * The signer may be the trader's *position authority* rather than its wallet,
 * and the venue instructions are entirely caller-supplied. A position authority
 * you do not control can therefore route a delegator's collateral through a
 * venue of its choosing, bounded only by `minAmountOut`. Two independent
 * opt-outs gate that path, and either one makes such a swap fail with error
 * **7101 `PositionAuthoritySwapDisabled`**:
 *
 * - the exchange-wide `positionAuthoritySwapDisabled` flag on the spot
 *   collateral configuration; and
 * - the trader's own `disablePositionAuthoritySwap` preference bit.
 *
 * Swaps signed by the trader's wallet are never gated. Check both flags with
 * the account decoders before building the instruction.
 *
 * A `Buy` additionally honors the deposit cooldown and is rejected while the
 * trader has a queued withdrawal.
 */
export const buildSwapNativeIx = async (
  params: SwapNativeParams
): Promise<NativeSolIx> => {
  requireIndexAccounts(params);
  if (!params.signer) {
    throw new Error("Signer is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.mint) {
    throw new Error("Mint is required");
  }
  if (params.amountIn === undefined || params.amountIn <= 0n) {
    throw new Error("Swap amount in must be greater than 0");
  }
  if (params.minAmountOut === undefined) {
    throw new Error(
      'minAmountOut is required; pass "unprotected" to disable slippage protection'
    );
  }
  if (typeof params.minAmountOut === "bigint" && params.minAmountOut <= 0n) {
    throw new Error(
      'minAmountOut must be greater than 0; pass "unprotected" to disable slippage protection'
    );
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [globalVault, nativeSolAuthority] = await Promise.all([
    getPhoenixGlobalVaultAddress(params.mint, programAddress),
    getPhoenixNativeSolAuthorityAddress(programAddress),
  ]);

  const accounts: NativeSolAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateWritableAccount(globalVault),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
    generateReadonlyAccount(nativeSolAuthority),
    generateWritableSignerAccount(params.signer),
    generateWritableAccount(params.signerQuoteTokenAccount),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.withdrawQueue),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...params.venue.accounts,
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeSwapNative(
      params.direction,
      params.amountIn,
      params.minAmountOut,
      params.venue.instructions
    ),
  };
};
