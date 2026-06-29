import { sha2_const } from "@/core/discriminants";
import { generateReadonlyAccount } from "@/core/utils/accountMeta";
import type {
  ActiveTraderBufferAddressArray,
  GlobalConfigurationAddress,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
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
  type Instruction,
} from "@solana/kit";

export const HAWKEYE_PROGRAM_ADDRESS: Address = address(
  "RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj"
);

export const HAWKEYE_SIMULATION_FEE_PAYER: Address = address(
  "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"
);

export const HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT: number = 1_400_000;
export const HAWKEYE_RETURN_VERSION: number = 1;

export type HawkeyeViewInstructionDiscriminators = {
  readonly margin: "global:view_margin";
  readonly asset: "global:view_margin_for_asset";
  readonly liquidation: "global:view_liquidation_price";
  readonly bbo: "global:view_bbo";
  readonly funding: "global:view_funding";
};

export const HAWKEYE_VIEW_INSTRUCTIONS: HawkeyeViewInstructionDiscriminators = {
  margin: "global:view_margin",
  asset: "global:view_margin_for_asset",
  liquidation: "global:view_liquidation_price",
  bbo: "global:view_bbo",
  funding: "global:view_funding",
};

type DiscriminantMap = Record<string, Uint8Array>;

export const HAWKEYE_DISCRIMINANTS: DiscriminantMap = {
  VIEW_MARGIN: sha2_const(HAWKEYE_VIEW_INSTRUCTIONS.margin),
  VIEW_MARGIN_FOR_ASSET: sha2_const(HAWKEYE_VIEW_INSTRUCTIONS.asset),
  VIEW_LIQUIDATION_PRICE: sha2_const(HAWKEYE_VIEW_INSTRUCTIONS.liquidation),
  VIEW_BBO: sha2_const(HAWKEYE_VIEW_INSTRUCTIONS.bbo),
  VIEW_FUNDING: sha2_const(HAWKEYE_VIEW_INSTRUCTIONS.funding),
};

export type HawkeyeReturnLabels = {
  readonly margin: "view_margin";
  readonly asset: "view_margin_for_asset";
  readonly liquidation: "view_liquidation_price";
  readonly bbo: "view_bbo";
  readonly funding: "view_funding";
};

export const HAWKEYE_RETURN_LABELS: HawkeyeReturnLabels = {
  margin: "view_margin",
  asset: "view_margin_for_asset",
  liquidation: "view_liquidation_price",
  bbo: "view_bbo",
  funding: "view_funding",
};

export type HawkeyeViewKind = keyof typeof HAWKEYE_VIEW_INSTRUCTIONS;

const HAWKEYE_RETURN_MAGICS = {
  margin: sha2_const("return:phoenix_hawkeye_margin"),
  asset: sha2_const("return:phoenix_hawkeye_asset"),
  liquidation: sha2_const("return:phoenix_hawkeye_liquidation_price"),
  bbo: sha2_const("return:phoenix_hawkeye_bbo"),
  funding: sha2_const("return:phoenix_hawkeye_funding"),
} as const;

export const HAWKEYE_BBO_HAS_BID: number = 1 << 0;
export const HAWKEYE_BBO_HAS_ASK: number = 1 << 1;
export const HAWKEYE_FUNDING_HAS_ACCUMULATED: number = 1 << 0;
export const HAWKEYE_FUNDING_HAS_UNSETTLED: number = 1 << 1;

const RISK_STATES = [
  "healthy",
  "unhealthy",
  "underwater",
  "zero_collateral_no_positions",
] as const;

const RISK_TIERS = [
  "safe",
  "at_risk",
  "cancellable",
  "liquidatable",
  "backstop_liquidatable",
  "high_risk",
] as const;

const LIQUIDATION_STATUSES = [
  "no_position",
  "found",
  "already_liquidatable",
  "no_price_in_bounds",
] as const;

const SIDES = ["flat", "long", "short"] as const;

export type HawkeyeCodeLabel<TLabel extends string = string> = {
  code: number;
  label: TLabel | "unknown";
};

export type HawkeyeMagic = {
  decimal: string;
  hex: string;
};

export type HawkeyeMarginReturn = {
  kind: typeof HAWKEYE_RETURN_LABELS.margin;
  magic: HawkeyeMagic;
  version: number;
  positionCount: number;
  riskState: HawkeyeCodeLabel<(typeof RISK_STATES)[number]>;
  riskTier: HawkeyeCodeLabel<(typeof RISK_TIERS)[number]>;
  isLiquidatable: boolean;
  collateralQuoteLots: bigint;
  effectiveCollateralQuoteLots: bigint;
  freeCollateralQuoteLots: bigint;
  withdrawableCollateralQuoteLots: bigint;
  initialMarginQuoteLots: bigint;
  maintenanceMarginQuoteLots: bigint;
  cancelMarginQuoteLots: bigint;
  backstopMarginQuoteLots: bigint;
  highRiskMarginQuoteLots: bigint;
  unrealizedPnlQuoteLots: bigint;
  discountedUnrealizedPnlQuoteLots: bigint;
  unsettledFundingQuoteLots: bigint;
};

export type HawkeyeAssetReturn = {
  kind: typeof HAWKEYE_RETURN_LABELS.asset;
  magic: HawkeyeMagic;
  assetId: number;
  version: number;
  hasPositionOrOrders: boolean;
  riskState: HawkeyeCodeLabel<(typeof RISK_STATES)[number]>;
  riskTier: HawkeyeCodeLabel<(typeof RISK_TIERS)[number]>;
  baseLots: bigint;
  virtualQuoteLots: bigint;
  markPriceTicks: bigint;
  entryPriceQuoteLotsPerBaseLot: bigint;
  positionValueQuoteLots: bigint;
  unrealizedPnlQuoteLots: bigint;
  discountedUnrealizedPnlQuoteLots: bigint;
  unsettledFundingQuoteLots: bigint;
  initialMarginQuoteLots: bigint;
  maintenanceMarginQuoteLots: bigint;
  cancelMarginQuoteLots: bigint;
  backstopMarginQuoteLots: bigint;
  highRiskMarginQuoteLots: bigint;
};

export type HawkeyeLiquidationPriceReturn = {
  kind: typeof HAWKEYE_RETURN_LABELS.liquidation;
  magic: HawkeyeMagic;
  assetId: number;
  version: number;
  status: HawkeyeCodeLabel<(typeof LIQUIDATION_STATUSES)[number]>;
  side: HawkeyeCodeLabel<(typeof SIDES)[number]>;
  liquidationPriceTicks: bigint;
  markPriceTicks: bigint;
  entryPriceQuoteLotsPerBaseLot: bigint;
  effectiveCollateralQuoteLots: bigint;
  maintenanceMarginQuoteLots: bigint;
};

export type HawkeyeBboReturn = {
  kind: typeof HAWKEYE_RETURN_LABELS.bbo;
  magic: HawkeyeMagic;
  version: number;
  flags: number;
  bestBidTicks: bigint | null;
  bestAskTicks: bigint | null;
  markPriceTicks: bigint;
  indexPriceTicks: bigint;
  markPriceLastUpdatedSlot: bigint;
  indexPriceLastUpdatedSlot: bigint;
};

export type HawkeyeFundingReturn = {
  kind: typeof HAWKEYE_RETURN_LABELS.funding;
  magic: HawkeyeMagic;
  assetId: number;
  version: number;
  flags: number;
  totalAccumulatedFundingQuoteLots: bigint | null;
  totalUnsettledFundingQuoteLots: bigint | null;
  currentFundingRateMicroBps: bigint;
  projected1hFundingRateMicroBps: bigint;
};

export type HawkeyeReturnData =
  | HawkeyeMarginReturn
  | HawkeyeAssetReturn
  | HawkeyeLiquidationPriceReturn
  | HawkeyeBboReturn
  | HawkeyeFundingReturn;

export type HawkeyeBaseAccounts = {
  phoenixProgramAddress: PhoenixProgramAddress;
  globalConfigurationAddress: GlobalConfigurationAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  perpAssetMap: PerpAssetMapAddress;
};

export type HawkeyeTraderViewAccounts = HawkeyeBaseAccounts & {
  traderAccount: TraderAddress;
};

export type HawkeyeBboViewAccounts = HawkeyeBaseAccounts & {
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
};

export type HawkeyeIx = InstructionsWithAccountsAndData;

export const buildHawkeyeViewMarginIx = (
  params: HawkeyeTraderViewAccounts
): HawkeyeIx => buildTraderViewIx(params, "margin");

export const buildHawkeyeViewMarginForAssetIx = (
  params: HawkeyeTraderViewAccounts & { assetId: number }
): HawkeyeIx => buildTraderViewIx(params, "asset", params.assetId);

export const buildHawkeyeViewLiquidationPriceIx = (
  params: HawkeyeTraderViewAccounts & { assetId: number }
): HawkeyeIx => buildTraderViewIx(params, "liquidation", params.assetId);

export const buildHawkeyeViewFundingIx = (
  params: HawkeyeTraderViewAccounts & { assetId: number }
): HawkeyeIx => buildTraderViewIx(params, "funding", params.assetId);

export const buildHawkeyeViewBboIx = (
  params: HawkeyeBboViewAccounts
): HawkeyeIx => ({
  programAddress: HAWKEYE_PROGRAM_ADDRESS,
  accounts: [
    ...buildBaseAccounts(params),
    generateReadonlyAccount(params.orderbook),
    generateReadonlyAccount(params.splineCollection),
  ],
  data: buildHawkeyeInstructionData("bbo"),
});

export const buildSetComputeUnitLimitIx = (
  units: number = HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT
): Instruction => {
  const data = new Uint8Array(5);
  const view = new DataView(data.buffer);
  view.setUint8(0, 2);
  view.setUint32(1, units, true);
  return {
    programAddress: address("ComputeBudget111111111111111111111111111111"),
    accounts: [],
    data,
  };
};

export const encodeHawkeyeSimulationTransaction = (params: {
  instruction: Instruction;
  blockhash: Blockhash;
  lastValidBlockHeight: bigint;
  feePayer?: Address;
  computeUnitLimit?: number;
}): string => {
  const message = appendTransactionMessageInstructions(
    [buildSetComputeUnitLimitIx(params.computeUnitLimit), params.instruction],
    setTransactionMessageLifetimeUsingBlockhash(
      {
        blockhash: params.blockhash,
        lastValidBlockHeight: params.lastValidBlockHeight,
      },
      setTransactionMessageFeePayer(
        params.feePayer ?? HAWKEYE_SIMULATION_FEE_PAYER,
        createTransactionMessage({ version: 0 })
      )
    )
  );

  return getBase64EncodedWireTransaction(compileTransaction(message));
};

export const decodeHawkeyeReturnData = (
  bytes: Uint8Array
): HawkeyeReturnData => {
  const kind = identifyHawkeyeReturnKind(bytes);
  switch (kind) {
    case "margin":
      return decodeMargin(bytes);
    case "asset":
      return decodeAsset(bytes);
    case "liquidation":
      return decodeLiquidation(bytes);
    case "bbo":
      return decodeBbo(bytes);
    case "funding":
      return decodeFunding(bytes);
  }
};

const buildTraderViewIx = (
  params: HawkeyeTraderViewAccounts,
  kind: "margin" | "asset" | "liquidation" | "funding",
  assetId?: number
): HawkeyeIx => ({
  programAddress: HAWKEYE_PROGRAM_ADDRESS,
  accounts: [
    ...buildBaseAccounts(params),
    generateReadonlyAccount(params.traderAccount),
  ],
  data: buildHawkeyeInstructionData(kind, assetId),
});

const buildBaseAccounts = (params: HawkeyeBaseAccounts) => [
  generateReadonlyAccount(params.phoenixProgramAddress),
  generateReadonlyAccount(params.globalConfigurationAddress),
  ...params.globalTraderIndex.map(generateReadonlyAccount),
  ...params.activeTraderBuffer.map(generateReadonlyAccount),
  generateReadonlyAccount(params.perpAssetMap),
];

const buildHawkeyeInstructionData = (
  kind: HawkeyeViewKind,
  assetId?: number
): Uint8Array => {
  const needsAsset =
    kind === "asset" || kind === "liquidation" || kind === "funding";
  if (needsAsset && assetId === undefined) {
    throw new Error("assetId is required for this Hawkeye view");
  }
  const data = new Uint8Array(needsAsset ? 16 : 8);
  data.set(sha2_const(HAWKEYE_VIEW_INSTRUCTIONS[kind]), 0);
  if (needsAsset) {
    new DataView(data.buffer).setUint32(8, assetId!, true);
  }
  return data;
};

const identifyHawkeyeReturnKind = (bytes: Uint8Array): HawkeyeViewKind => {
  expectLengthAtLeast(bytes, 8, "Hawkeye return data");
  for (const [kind, magic] of Object.entries(HAWKEYE_RETURN_MAGICS)) {
    if (bytesEqual(bytes.subarray(0, 8), magic)) {
      return kind as HawkeyeViewKind;
    }
  }
  throw new Error(`Unknown Hawkeye return data magic ${readMagic(bytes).hex}`);
};

const decodeMargin = (bytes: Uint8Array): HawkeyeMarginReturn => {
  const view = viewFor(bytes, 112, "view_margin");
  return {
    kind: HAWKEYE_RETURN_LABELS.margin,
    magic: readMagic(bytes),
    version: view.getUint16(8, true),
    positionCount: view.getUint16(10, true),
    riskState: codeLabel(RISK_STATES, view.getUint8(12)),
    riskTier: codeLabel(RISK_TIERS, view.getUint8(13)),
    isLiquidatable: view.getUint8(14) !== 0,
    collateralQuoteLots: view.getBigInt64(16, true),
    effectiveCollateralQuoteLots: view.getBigInt64(24, true),
    freeCollateralQuoteLots: view.getBigInt64(32, true),
    withdrawableCollateralQuoteLots: view.getBigUint64(40, true),
    initialMarginQuoteLots: view.getBigUint64(48, true),
    maintenanceMarginQuoteLots: view.getBigUint64(56, true),
    cancelMarginQuoteLots: view.getBigUint64(64, true),
    backstopMarginQuoteLots: view.getBigUint64(72, true),
    highRiskMarginQuoteLots: view.getBigUint64(80, true),
    unrealizedPnlQuoteLots: view.getBigInt64(88, true),
    discountedUnrealizedPnlQuoteLots: view.getBigInt64(96, true),
    unsettledFundingQuoteLots: view.getBigInt64(104, true),
  };
};

const decodeAsset = (bytes: Uint8Array): HawkeyeAssetReturn => {
  const view = viewFor(bytes, 128, "view_margin_for_asset");
  return {
    kind: HAWKEYE_RETURN_LABELS.asset,
    magic: readMagic(bytes),
    assetId: view.getUint32(8, true),
    version: view.getUint16(12, true),
    hasPositionOrOrders: view.getUint8(14) !== 0,
    riskState: codeLabel(RISK_STATES, view.getUint8(16)),
    riskTier: codeLabel(RISK_TIERS, view.getUint8(17)),
    baseLots: view.getBigInt64(24, true),
    virtualQuoteLots: view.getBigInt64(32, true),
    markPriceTicks: view.getBigUint64(40, true),
    entryPriceQuoteLotsPerBaseLot: view.getBigUint64(48, true),
    positionValueQuoteLots: view.getBigInt64(56, true),
    unrealizedPnlQuoteLots: view.getBigInt64(64, true),
    discountedUnrealizedPnlQuoteLots: view.getBigInt64(72, true),
    unsettledFundingQuoteLots: view.getBigInt64(80, true),
    initialMarginQuoteLots: view.getBigUint64(88, true),
    maintenanceMarginQuoteLots: view.getBigUint64(96, true),
    cancelMarginQuoteLots: view.getBigUint64(104, true),
    backstopMarginQuoteLots: view.getBigUint64(112, true),
    highRiskMarginQuoteLots: view.getBigUint64(120, true),
  };
};

const decodeLiquidation = (
  bytes: Uint8Array
): HawkeyeLiquidationPriceReturn => {
  const view = viewFor(bytes, 56, "view_liquidation_price");
  return {
    kind: HAWKEYE_RETURN_LABELS.liquidation,
    magic: readMagic(bytes),
    assetId: view.getUint32(8, true),
    version: view.getUint16(12, true),
    status: codeLabel(LIQUIDATION_STATUSES, view.getUint8(14)),
    side: codeLabel(SIDES, view.getUint8(15)),
    liquidationPriceTicks: view.getBigUint64(16, true),
    markPriceTicks: view.getBigUint64(24, true),
    entryPriceQuoteLotsPerBaseLot: view.getBigUint64(32, true),
    effectiveCollateralQuoteLots: view.getBigInt64(40, true),
    maintenanceMarginQuoteLots: view.getBigUint64(48, true),
  };
};

const decodeBbo = (bytes: Uint8Array): HawkeyeBboReturn => {
  const view = viewFor(bytes, 64, "view_bbo");
  const flags = view.getUint8(10);
  return {
    kind: HAWKEYE_RETURN_LABELS.bbo,
    magic: readMagic(bytes),
    version: view.getUint16(8, true),
    flags,
    bestBidTicks:
      (flags & HAWKEYE_BBO_HAS_BID) !== 0 ? view.getBigUint64(16, true) : null,
    bestAskTicks:
      (flags & HAWKEYE_BBO_HAS_ASK) !== 0 ? view.getBigUint64(24, true) : null,
    markPriceTicks: view.getBigUint64(32, true),
    indexPriceTicks: view.getBigUint64(40, true),
    markPriceLastUpdatedSlot: view.getBigUint64(48, true),
    indexPriceLastUpdatedSlot: view.getBigUint64(56, true),
  };
};

const decodeFunding = (bytes: Uint8Array): HawkeyeFundingReturn => {
  const view = viewFor(bytes, 48, "view_funding");
  const flags = view.getUint8(14);
  return {
    kind: HAWKEYE_RETURN_LABELS.funding,
    magic: readMagic(bytes),
    assetId: view.getUint32(8, true),
    version: view.getUint16(12, true),
    flags,
    totalAccumulatedFundingQuoteLots:
      (flags & HAWKEYE_FUNDING_HAS_ACCUMULATED) !== 0
        ? view.getBigInt64(16, true)
        : null,
    totalUnsettledFundingQuoteLots:
      (flags & HAWKEYE_FUNDING_HAS_UNSETTLED) !== 0
        ? view.getBigInt64(24, true)
        : null,
    currentFundingRateMicroBps: view.getBigInt64(32, true),
    projected1hFundingRateMicroBps: view.getBigInt64(40, true),
  };
};

const viewFor = (
  bytes: Uint8Array,
  expectedLength: number,
  label: string
): DataView => {
  if (bytes.byteLength !== expectedLength) {
    throw new Error(
      `${label} return data expected ${expectedLength} bytes, got ${bytes.byteLength}`
    );
  }
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
};

const expectLengthAtLeast = (
  bytes: Uint8Array,
  expectedLength: number,
  label: string
) => {
  if (bytes.byteLength < expectedLength) {
    throw new Error(
      `${label} expected at least ${expectedLength} bytes, got ${bytes.byteLength}`
    );
  }
};

const readMagic = (bytes: Uint8Array): HawkeyeMagic => {
  const value = new DataView(
    bytes.buffer,
    bytes.byteOffset,
    bytes.byteLength
  ).getBigUint64(0, true);
  return {
    decimal: value.toString(),
    hex: `0x${value.toString(16).padStart(16, "0")}`,
  };
};

const codeLabel = <T extends readonly string[]>(
  labels: T,
  code: number
): HawkeyeCodeLabel<T[number]> => ({
  code,
  label: labels[code] ?? "unknown",
});

const bytesEqual = (left: Uint8Array, right: Uint8Array): boolean => {
  if (left.byteLength !== right.byteLength) {
    return false;
  }
  for (let index = 0; index < left.byteLength; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
};
