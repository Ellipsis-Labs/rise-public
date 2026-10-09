/**
 * SPL spot collateral instructions.
 *
 * SPL tokens can be posted as collateral alongside the canonical quote token
 * and native SOL. Each configured spot asset is identified by its mint, and
 * its tokens are custodied in the associated token account of the trader's
 * wallet PDA (seeds `["wallet", traderAccount]`) for the asset's mint. The *accounted*
 * balance is tracked in the trader position map; tokens sitting in the custody
 * ATA beyond it ("excess") carry no margin value.
 *
 * The builders here derive the wallet PDA and its custody ATAs internally from
 * `traderAccount` and the asset's mint, so callers never assemble those
 * addresses by hand. Admin instructions (add, configure) are deliberately not
 * exposed, matching the native SOL module.
 */

import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  requireIndexAccounts,
  requireSwapAmounts,
} from "@/core/ixBuilders/NativeSol/ix";
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
  getAssociatedTokenAccountAddress,
  getPhoenixGlobalVaultAddress,
  getPhoenixNativeSolAuthorityAddress,
  getPhoenixTraderWalletAddress,
} from "@/pdas";
import type { MintAddress, TraderAddress } from "@/primitives";
import type { TokenAccountAddress, TraderWalletAddress } from "@/primitives";
import {
  encodeLiquidateSpot,
  encodeSwapSpotWithSol,
  encodeSwapSpotWithSpot,
  encodeSwapSpotWithUsdc,
  encodeSyncSpot,
  encodeTransferSpot,
  encodeTransferSpotFromChildToParent,
  encodeWithdrawSpot,
} from "./codec";
import type {
  LiquidateSpotParams,
  SpotAccounts,
  SpotIx,
  SwapSpotWithSolParams,
  SwapSpotWithSpotParams,
  SwapSpotWithUsdcParams,
  SyncSpotParams,
  TransferSpotFromChildToParentParams,
  TransferSpotParams,
  WithdrawSpotAction,
  WithdrawSpotParams,
} from "./types";

const actionAmount = (action: WithdrawSpotAction): bigint | undefined =>
  action.kind === "allExcess" ? undefined : action.amount;

/** Derive the trader wallet PDA and its custody ATA for `mint`. */
const custodyAccounts = async (
  traderAccount: TraderAddress,
  mint: MintAddress,
  programAddress: Parameters<typeof getPhoenixTraderWalletAddress>[1]
): Promise<{ wallet: TraderWalletAddress; walletAta: TokenAccountAddress }> => {
  const wallet = await getPhoenixTraderWalletAddress(
    traderAccount,
    programAddress
  );
  const walletAta = await getAssociatedTokenAccountAddress(wallet, mint);
  return { wallet, walletAta };
};

/**
 * Reconcile a trader's accounted spot collateral against the token balance
 * actually in the custody ATA.
 *
 * Permissionless — anyone may crank it. An upward reconciliation is clamped to
 * the per-trader and exchange-wide caps; whatever the clamp leaves over stays
 * in the custody ATA as uncounted excess, recoverable by a later sync once
 * headroom frees up. Depositing is a plain SPL transfer into the custody ATA
 * followed by this instruction.
 */
export const buildSyncSpotIx = async (
  params: SyncSpotParams
): Promise<SpotIx> => {
  requireIndexAccounts(params);
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.mint) {
    throw new Error("Mint is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const { walletAta } = await custodyAccounts(
    params.traderAccount,
    params.mint,
    programAddress
  );

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // Unlike `SyncNative`, the global configuration is readonly: the spot
    // metadata and exchange-wide tally live in the perp asset map, which is
    // writable instead.
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateReadonlyAccount(params.mint),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(walletAta),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeSyncSpot(),
  };
};

/**
 * Withdraw SPL spot collateral to a token account of the asset's mint.
 *
 * Like a native SOL withdrawal this charges no fee, ignores the deposit
 * cooldown, and is never enqueued: if the exchange-wide throttle cannot absorb
 * the withdrawal's quote-notional value right now, the instruction fails
 * rather than queueing. Excess-only withdrawals bypass the throttle entirely.
 */
export const buildWithdrawSpotIx = async (
  params: WithdrawSpotParams
): Promise<SpotIx> => {
  requireIndexAccounts(params);
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.mint) {
    throw new Error("Mint is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.destination) {
    throw new Error("Destination is required");
  }
  if (!params.withdrawQueue) {
    throw new Error("Withdraw queue is required");
  }
  const amount = actionAmount(params.action);
  if (amount !== undefined && amount <= 0n) {
    throw new Error("Withdraw amount must be greater than 0");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const { wallet, walletAta } = await custodyAccounts(
    params.traderAccount,
    params.mint,
    programAddress
  );
  if (params.destination === walletAta) {
    throw new Error(
      "Withdraw destination must be a token account other than the custody ATA"
    );
  }

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    generateReadonlyAccount(params.mint),
    generateReadonlyAccount(wallet),
    generateWritableAccount(walletAta),
    generateWritableAccount(params.destination),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    // The withdraw queue precedes the index arenas here, matching the native
    // SOL withdrawal.
    generateWritableAccount(params.withdrawQueue),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeWithdrawSpot(params.action),
  };
};

/**
 * Move SPL spot collateral between two of a trader's accounts, moving the
 * backing tokens between the custody ATAs.
 *
 * The source debit is margin checked. The destination custody ATA must already
 * exist (anyone may create it idempotently). The exchange-wide tally is
 * unchanged, since the collateral never leaves the protocol. The trailing
 * permission account enables the position-authority path, exactly as for a
 * quote collateral transfer.
 */
export const buildTransferSpotIx = async (
  params: TransferSpotParams
): Promise<SpotIx> => {
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
  if (!params.mint) {
    throw new Error("Mint is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (params.amount === undefined || params.amount <= 0n) {
    throw new Error("Amount must be greater than 0");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [src, dst] = await Promise.all([
    custodyAccounts(params.srcTraderAccount, params.mint, programAddress),
    custodyAccounts(params.dstTraderAccount, params.mint, programAddress),
  ]);

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // The token-movement accounts precede the regular collateral transfer
    // group, which is reused verbatim for permissioning.
    generateReadonlyAccount(params.mint),
    generateReadonlyAccount(src.wallet),
    generateWritableAccount(src.walletAta),
    generateWritableAccount(dst.walletAta),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
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
    data: encodeTransferSpot(params.amount),
  };
};

/**
 * Sweep a flat isolated child account's balance of one spot collateral asset
 * into its parent, moving the backing tokens between the custody ATAs. The
 * parent's custody ATA must already exist. One asset per call.
 *
 * This is a silent no-op — not an error — when the child still has splines,
 * open orders, a position, or a negative quote balance, or when it holds none
 * of the asset.
 */
export const buildTransferSpotFromChildToParentIx = async (
  params: TransferSpotFromChildToParentParams
): Promise<SpotIx> => {
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
  if (!params.mint) {
    throw new Error("Mint is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const traderSigns = params.traderSigns ?? true;
  const [child, parent] = await Promise.all([
    custodyAccounts(params.childTraderAccount, params.mint, programAddress),
    custodyAccounts(params.parentTraderAccount, params.mint, programAddress),
  ]);

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // The token-movement accounts precede the regular collateral
    // child-to-parent group, which is reused verbatim for permissioning.
    generateReadonlyAccount(params.mint),
    generateReadonlyAccount(child.wallet),
    generateWritableAccount(child.walletAta),
    generateWritableAccount(parent.walletAta),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
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
    data: encodeTransferSpotFromChildToParent(),
  };
};

/**
 * Swap between an SPL spot collateral asset and quote collateral through an
 * external venue.
 *
 * The program withdraws the input leg to the signer, runs the caller-supplied
 * venue instructions, deposits the output leg, and then checks that the
 * trader's margin state did not worsen. The exchange-wide withdrawal throttle
 * is charged only the swap's *value loss* (slippage plus venue fees), not the
 * full amount moved.
 *
 * `minAmountOut` is the **only** price protection this instruction has — there
 * is no oracle floor. Its unit is the output asset: quote lots for a `Sell`,
 * base units of the spot token for a `Buy`. The signer may be the trader's
 * position authority; the same opt-out gates apply as for `SwapNative` (error
 * **7101 `PositionAuthoritySwapDisabled`**).
 */
export const buildSwapSpotWithUsdcIx = async (
  params: SwapSpotWithUsdcParams
): Promise<SpotIx> => {
  requireIndexAccounts(params);
  if (!params.signer) {
    throw new Error("Signer is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.quoteMint) {
    throw new Error("Quote mint is required");
  }
  if (!params.spotMint) {
    throw new Error("Spot mint is required");
  }
  requireSwapAmounts(params);

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [globalVault, { wallet, walletAta: spotWalletAta }] = await Promise.all(
    [
      getPhoenixGlobalVaultAddress(params.quoteMint, programAddress),
      custodyAccounts(params.traderAccount, params.spotMint, programAddress),
    ]
  );

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateReadonlyAccount(params.spotMint),
    generateWritableAccount(globalVault),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(wallet),
    generateWritableSignerAccount(params.signer),
    generateWritableAccount(params.signerQuoteTokenAccount),
    generateWritableAccount(params.signerSpotTokenAccount),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.withdrawQueue),
    generateWritableAccount(spotWalletAta),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...params.venue.accounts,
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeSwapSpotWithUsdc(
      params.direction,
      params.amountIn,
      params.minAmountOut,
      params.venue.instructions
    ),
  };
};

/**
 * Swap between an SPL spot collateral asset and native SOL collateral through
 * an external venue.
 *
 * Same shape and caveats as `buildSwapSpotWithUsdcIx`, with lamports as the
 * non-spot leg: the signer's own account carries the lamports in both
 * directions.
 */
export const buildSwapSpotWithSolIx = async (
  params: SwapSpotWithSolParams
): Promise<SpotIx> => {
  requireIndexAccounts(params);
  if (!params.signer) {
    throw new Error("Signer is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.spotMint) {
    throw new Error("Spot mint is required");
  }
  requireSwapAmounts(params);

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [nativeSolAuthority, { wallet, walletAta: spotWalletAta }] =
    await Promise.all([
      getPhoenixNativeSolAuthorityAddress(programAddress),
      custodyAccounts(params.traderAccount, params.spotMint, programAddress),
    ]);

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateReadonlyAccount(params.spotMint),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
    generateReadonlyAccount(nativeSolAuthority),
    generateReadonlyAccount(wallet),
    generateWritableSignerAccount(params.signer),
    generateWritableAccount(params.signerSpotTokenAccount),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.withdrawQueue),
    generateWritableAccount(spotWalletAta),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...params.venue.accounts,
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeSwapSpotWithSol(
      params.direction,
      params.amountIn,
      params.minAmountOut,
      params.venue.instructions
    ),
  };
};

/**
 * Swap one SPL spot collateral asset for another through an external venue.
 *
 * The venue must pay the output into `signerDstTokenAccount`; its balance
 * increase is moved into the destination custody ATA and credited. Same
 * slippage and position-authority caveats as `buildSwapSpotWithUsdcIx`.
 */
export const buildSwapSpotWithSpotIx = async (
  params: SwapSpotWithSpotParams
): Promise<SpotIx> => {
  requireIndexAccounts(params);
  if (!params.signer) {
    throw new Error("Signer is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.srcMint) {
    throw new Error("Source mint is required");
  }
  if (!params.dstMint) {
    throw new Error("Destination mint is required");
  }
  if (!params.signerSrcTokenAccount) {
    throw new Error("Signer source token account is required");
  }
  if (!params.signerDstTokenAccount) {
    throw new Error("Signer destination token account is required");
  }
  requireSwapAmounts(params);

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [src, dst] = await Promise.all([
    custodyAccounts(params.traderAccount, params.srcMint, programAddress),
    custodyAccounts(params.traderAccount, params.dstMint, programAddress),
  ]);

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateReadonlyAccount(params.srcMint),
    generateReadonlyAccount(params.dstMint),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(src.wallet),
    generateWritableSignerAccount(params.signer),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.withdrawQueue),
    generateWritableAccount(src.walletAta),
    generateWritableAccount(params.signerSrcTokenAccount),
    generateWritableAccount(params.signerDstTokenAccount),
    generateWritableAccount(dst.walletAta),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...params.venue.accounts,
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeSwapSpotWithSpot(
      params.amountIn,
      params.minAmountOut,
      params.venue.instructions
    ),
  };
};

/**
 * Seize a liquidatee's SPL spot collateral at the discounted index price. The
 * signer must be the risk authority or hold `permissionAccount` as a delegated
 * liquidation authority.
 */
export const buildLiquidateSpotIx = async (
  params: LiquidateSpotParams
): Promise<SpotIx> => {
  requireIndexAccounts(params);
  if (!params.signer) throw new Error("Signer is required");
  if (!params.liquidateeAccount)
    throw new Error("Liquidatee account is required");
  if (!params.quoteMint) throw new Error("Quote mint is required");
  if (!params.spotMint) throw new Error("Spot mint is required");
  if (!params.perpAssetMap) throw new Error("Perp asset map is required");
  if (!params.signerQuoteTokenAccount) {
    throw new Error("Signer quote token account is required");
  }
  if (!params.signerSpotTokenAccount) {
    throw new Error("Signer spot token account is required");
  }
  if (params.maxSpotAmount === undefined) {
    throw new Error("Maximum spot amount is required");
  }

  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const [globalVault, { wallet, walletAta }] = await Promise.all([
    getPhoenixGlobalVaultAddress(params.quoteMint, programAddress),
    custodyAccounts(params.liquidateeAccount, params.spotMint, programAddress),
  ]);
  const extraTraderAccounts = params.extraTraderAccounts ?? [];

  const accounts: SpotAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    generateReadonlyAccount(params.spotMint),
    generateWritableAccount(globalVault),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(wallet),
    generateWritableSignerAccount(params.signer),
    generateWritableAccount(params.permissionAccount ?? params.signer),
    generateWritableAccount(params.signerQuoteTokenAccount),
    generateWritableAccount(params.signerSpotTokenAccount),
    generateWritableAccount(params.liquidateeAccount),
    generateWritableAccount(walletAta),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...extraTraderAccounts.map(generateWritableAccount),
    ...params.venue.accounts,
  ] as const;

  return {
    programAddress,
    accounts,
    data: encodeLiquidateSpot(
      params.maxSpotAmount,
      BigInt(extraTraderAccounts.length),
      params.venue.instructions
    ),
  };
};
