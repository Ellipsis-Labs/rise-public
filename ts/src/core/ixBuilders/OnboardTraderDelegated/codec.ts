import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  getArrayDecoder,
  getArrayEncoder,
  getBooleanDecoder,
  getBooleanEncoder,
  getConstantDecoder,
  getConstantEncoder,
  getEnumDecoder,
  getEnumEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  transformDecoder,
  transformEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

enum TraderCapabilityToggleTarget {
  PlaceLimitOrder = 0,
  PlaceMarketOrder = 1,
  RiskIncreasingTrade = 2,
  RiskReducingTrade = 3,
  DepositCollateral = 4,
  WithdrawCollateral = 5,
}

interface TraderCapabilityToggle {
  target: TraderCapabilityToggleTarget;
  enable: boolean;
}

interface TraderCapabilityUpdate {
  toggles: TraderCapabilityToggle[];
}

const ALL_TRADER_CAPABILITY_TOGGLES: TraderCapabilityToggle[] = [
  { target: TraderCapabilityToggleTarget.PlaceLimitOrder, enable: true },
  { target: TraderCapabilityToggleTarget.PlaceMarketOrder, enable: true },
  { target: TraderCapabilityToggleTarget.RiskReducingTrade, enable: true },
  { target: TraderCapabilityToggleTarget.RiskIncreasingTrade, enable: true },
  { target: TraderCapabilityToggleTarget.DepositCollateral, enable: true },
  { target: TraderCapabilityToggleTarget.WithdrawCollateral, enable: true },
];

const getTraderCapabilityToggleTargetEncoder =
  (): Encoder<TraderCapabilityToggleTarget> =>
    getEnumEncoder(TraderCapabilityToggleTarget);

const getTraderCapabilityToggleTargetDecoder =
  (): Decoder<TraderCapabilityToggleTarget> =>
    getEnumDecoder(TraderCapabilityToggleTarget);

const getTraderCapabilityToggleEncoder = (): Encoder<TraderCapabilityToggle> =>
  getStructEncoder([
    ["target", getTraderCapabilityToggleTargetEncoder()],
    ["enable", getBooleanEncoder()],
  ]);

const getTraderCapabilityToggleDecoder = (): Decoder<TraderCapabilityToggle> =>
  getStructDecoder([
    ["target", getTraderCapabilityToggleTargetDecoder()],
    ["enable", getBooleanDecoder()],
  ]) as unknown as Decoder<TraderCapabilityToggle>;

const getTraderCapabilityUpdateEncoder = (): Encoder<TraderCapabilityUpdate> =>
  getStructEncoder([
    ["toggles", getArrayEncoder(getTraderCapabilityToggleEncoder())],
  ]);

const getTraderCapabilityUpdateDecoder = (): Decoder<TraderCapabilityUpdate> =>
  getStructDecoder([
    ["toggles", getArrayDecoder(getTraderCapabilityToggleDecoder())],
  ]) as unknown as Decoder<TraderCapabilityUpdate>;

const getDelegatedCapabilityUpdateEncoder =
  (): Encoder<TraderCapabilityUpdate> =>
    getHiddenPrefixEncoder(getTraderCapabilityUpdateEncoder(), [
      getConstantEncoder(DISCRIMINANTS.SET_TRADER_CAPABILITIES_DELEGATED),
    ]);

const getDelegatedCapabilityUpdateDecoder =
  (): Decoder<TraderCapabilityUpdate> =>
    getHiddenPrefixDecoder(getTraderCapabilityUpdateDecoder(), [
      getConstantDecoder(DISCRIMINANTS.SET_TRADER_CAPABILITIES_DELEGATED),
    ]);

export const getOnboardTraderDelegatedEncoder = (): Encoder<void> =>
  transformEncoder(getDelegatedCapabilityUpdateEncoder(), (_: void) => ({
    toggles: ALL_TRADER_CAPABILITY_TOGGLES,
  }));

export const getOnboardTraderDelegatedDecoder = (): Decoder<void> =>
  transformDecoder(getDelegatedCapabilityUpdateDecoder(), () => undefined);

export const getOnboardTraderDelegatedCodec = (): Codec<void> =>
  combineCodec(
    getOnboardTraderDelegatedEncoder(),
    getOnboardTraderDelegatedDecoder()
  );
