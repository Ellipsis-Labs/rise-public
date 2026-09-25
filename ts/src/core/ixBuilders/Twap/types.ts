import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  Authority,
  BaseLots,
  FlickerProgramAddress,
  PhoenixProgramAddress,
  QuoteLots,
  TraderAddress,
  TwapAccountAddress,
  TwapGlobalStateAddress,
  TwapLogAuthorityAddress,
  Ticks,
} from "@/primitives";
import type {
  ImmediateOrCancelOrderPacket,
  OrderFlags,
  SelfTradeBehavior,
} from "@/primitives/OrderPacket";
import type { Side } from "@/primitives/Side";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface TwapInstructionAddressOverrides extends PhoenixInstructionAddressOverrides {
  flickerProgramAddress?: FlickerProgramAddress;
  hawkeyeProgramAddress?: Address;
  twapGlobalStateAddress: TwapGlobalStateAddress;
  twapLogAuthorityAddress: TwapLogAuthorityAddress;
}

export interface TwapIocOrderPacketData {
  side: Side;
  priceInTicks: Ticks | null;
  numBaseLots: BaseLots;
  numQuoteLots: QuoteLots | null;
  minBaseLotsToFill: BaseLots;
  minQuoteLotsToFill: QuoteLots;
  selfTradeBehavior: SelfTradeBehavior;
  matchLimit: bigint | null;
  clientOrderId: bigint;
  lastValidSlot: bigint | null;
  orderFlags: OrderFlags;
  cancelExisting: boolean;
  /**
   * Size of each dust order in base lots, strictly less than `numBaseLots`.
   * Byte offset 104 of the fixed 152-byte packet.
   */
  dustOrderSize: BaseLots;
  /** Explicit dust count. Zero with nonzero dustOrderSize uses one final legacy dust order. */
  nDustOrders: bigint;
}

export interface CreateTwapAccountData {
  marketId: number;
}

export interface PlaceTwapOrderData {
  cooldownSlots: bigint;
  /** Regular child count for explicit dust; total execution count for legacy dust. */
  nChildOrders: bigint;
  /** Explicit dust count, at most nChildOrders. Defaults to zero; nonzero dustOrderSize then uses legacy final-child dust. */
  nDustOrders?: bigint;
  childOrderMaxSlippageBps: bigint | number;
  childOrderMinPriceInTicks?: Ticks | null;
  childOrderMaxPriceInTicks?: Ticks | null;
  childOrderPacket: ImmediateOrCancelOrderPacket;
  /** Size of each dust order in base lots; required and positive when nDustOrders > 0. */
  dustOrderSize?: BaseLots | null;
  childOrderCollateralQuoteLotsToTransfer?: QuoteLots | null;
  lastValidSlot?: bigint | null;
  transferCollateralAccountCount: number;
}

export interface ExecuteTwapOrderData {
  transferCollateralAccountCount: number;
}

export interface CreateTwapAccountParams extends TwapInstructionAddressOverrides {
  twapAccount: TwapAccountAddress;
  traderAccount: TraderAddress;
  payer: Authority;
  marketId: number;
}

export interface TwapChildOrderCpiTail {
  transferAccounts?: readonly AccountMeta[];
  orderAccounts: readonly AccountMeta[];
}

export interface PlaceTwapOrderParams
  extends TwapInstructionAddressOverrides, TwapChildOrderCpiTail {
  twapAccount: TwapAccountAddress;
  authority: Authority;
  cooldownSlots: bigint;
  /** Regular child count for explicit dust; total execution count for legacy dust. */
  nChildOrders: bigint;
  /** Explicit dust count, at most nChildOrders. Defaults to zero; nonzero dustOrderSize then uses legacy final-child dust. */
  nDustOrders?: bigint;
  childOrderMaxSlippageBps: bigint | number;
  childOrderMinPriceInTicks?: Ticks | null;
  childOrderMaxPriceInTicks?: Ticks | null;
  childOrderPacket: ImmediateOrCancelOrderPacket;
  /**
   * Size of each dust order in base lots, strictly less than the child order
   * size. Required and positive when nDustOrders > 0.
   */
  dustOrderSize?: BaseLots | null;
  childOrderCollateralQuoteLotsToTransfer?: QuoteLots | null;
  lastValidSlot?: bigint | null;
}

export interface ExecuteTwapOrderParams
  extends TwapInstructionAddressOverrides, TwapChildOrderCpiTail {
  twapAccount: TwapAccountAddress;
}

export interface CancelTwapOrderParams extends TwapInstructionAddressOverrides {
  twapAccount: TwapAccountAddress;
  authority: Authority;
}

export interface CloseInactiveTwapAccountParams extends TwapInstructionAddressOverrides {
  twapAccount: TwapAccountAddress;
  recipient: Address;
}

/**
 * Enrolls a trader in Flicker TWAP execution by granting the position
 * authority permission bit to the global Flicker delegate (the TWAP global
 * state PDA). `permissionPda` is the Eternal permission PDA of
 * (`traderAuthority`, `twapGlobalStateAddress`); derive it with
 * `getTwapDelegatePermissionAddress`.
 */
export interface EnableTwapParams extends PhoenixInstructionAddressOverrides {
  traderAuthority: Authority;
  twapGlobalStateAddress: TwapGlobalStateAddress;
  permissionPda: Address;
  /** Rent payer for CreatePermission. Defaults to `traderAuthority`. */
  payer?: Address;
}

export interface EnableTwapIxs {
  createPermission: InstructionsWithAccountsAndData;
  setPermission: InstructionsWithAccountsAndData;
  instructions: readonly [
    InstructionsWithAccountsAndData,
    InstructionsWithAccountsAndData,
  ];
}

export interface DisableTwapParams extends PhoenixInstructionAddressOverrides {
  traderAuthority: Authority;
  twapGlobalStateAddress: TwapGlobalStateAddress;
  permissionPda: Address;
}

export interface ResolvedTwapInstructionAddresses {
  phoenixProgramAddress: PhoenixProgramAddress;
  flickerProgramAddress: FlickerProgramAddress;
  twapGlobalStateAddress: TwapGlobalStateAddress;
  twapLogAuthorityAddress: TwapLogAuthorityAddress;
  hawkeyeProgramAddress: Address;
}

export type CreateTwapAccountIx = InstructionsWithAccountsAndData;
export type PlaceTwapOrderIx = InstructionsWithAccountsAndData;
export type ExecuteTwapOrderIx = InstructionsWithAccountsAndData;
export type CancelTwapOrderIx = InstructionsWithAccountsAndData;
export type CloseInactiveTwapAccountIx = InstructionsWithAccountsAndData;
export type DisableTwapIx = InstructionsWithAccountsAndData;

export type CreateTwapAccountAccounts = readonly AccountMeta[];
export type PlaceTwapOrderAccounts = readonly AccountMeta[];
export type ExecuteTwapOrderAccounts = readonly AccountMeta[];
export type CancelTwapOrderAccounts = readonly AccountMeta[];
export type CloseInactiveTwapAccountAccounts = readonly AccountMeta[];
