import type {
  ActiveTraderBufferHeaderAddress,
  Authority,
  GlobalConfigurationAddress,
  GlobalTraderIndexHeaderAddress,
  GlobalVaultAddress,
  MarketAddress,
  MintAddress,
  PerpAssetMapAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "./types";
import {
  address,
  fixDecoderSize,
  fixEncoderSize,
  getBase58Decoder,
  getBase58Encoder,
  transformDecoder,
  transformEncoder,
  type Address,
  type Decoder,
  type Encoder,
} from "@solana/kit";

const PUBKEY_BYTES = 32;

export const getPubkeyEncoder = (): Encoder<Address> =>
  fixEncoderSize(getBase58Encoder(), PUBKEY_BYTES);

export const getPubkeyDecoder = (): Decoder<Address> =>
  transformDecoder(
    fixDecoderSize(getBase58Decoder(), PUBKEY_BYTES),
    (str: string): Address => address(str)
  );

export const getAuthorityEncoder = (): Encoder<Authority> =>
  transformEncoder(getPubkeyEncoder(), (addr: Authority): Address => addr);

export const getAuthorityDecoder = (): Decoder<Authority> =>
  transformDecoder(
    getPubkeyDecoder(),
    (addr: Address): Authority => addr as Authority
  );

export const getGlobalConfigurationAddressEncoder =
  (): Encoder<GlobalConfigurationAddress> =>
    transformEncoder(
      getPubkeyEncoder(),
      (addr: GlobalConfigurationAddress): Address => addr
    );

export const getGlobalConfigurationAddressDecoder =
  (): Decoder<GlobalConfigurationAddress> =>
    transformDecoder(
      getPubkeyDecoder(),
      (addr: Address): GlobalConfigurationAddress =>
        addr as GlobalConfigurationAddress
    );

export const getMintAddressEncoder = (): Encoder<MintAddress> =>
  transformEncoder(getPubkeyEncoder(), (addr: MintAddress): Address => addr);

export const getMintAddressDecoder = (): Decoder<MintAddress> =>
  transformDecoder(
    getPubkeyDecoder(),
    (addr: Address): MintAddress => addr as MintAddress
  );

export const getGlobalVaultAddressEncoder = (): Encoder<GlobalVaultAddress> =>
  transformEncoder(
    getPubkeyEncoder(),
    (addr: GlobalVaultAddress): Address => addr
  );

export const getGlobalVaultAddressDecoder = (): Decoder<GlobalVaultAddress> =>
  transformDecoder(
    getPubkeyDecoder(),
    (addr: Address): GlobalVaultAddress => addr as GlobalVaultAddress
  );

export const getPerpAssetMapAddressEncoder = (): Encoder<PerpAssetMapAddress> =>
  transformEncoder(
    getPubkeyEncoder(),
    (addr: PerpAssetMapAddress): Address => addr
  );

export const getPerpAssetMapAddressDecoder = (): Decoder<PerpAssetMapAddress> =>
  transformDecoder(
    getPubkeyDecoder(),
    (addr: Address): PerpAssetMapAddress => addr as PerpAssetMapAddress
  );

export const getGlobalTraderIndexAddressHeaderEncoder =
  (): Encoder<GlobalTraderIndexHeaderAddress> =>
    transformEncoder(
      getPubkeyEncoder(),
      (addr: GlobalTraderIndexHeaderAddress): Address => addr
    );

export const getGlobalTraderIndexAddressHeaderDecoder =
  (): Decoder<GlobalTraderIndexHeaderAddress> =>
    transformDecoder(
      getPubkeyDecoder(),
      (addr: Address): GlobalTraderIndexHeaderAddress =>
        addr as GlobalTraderIndexHeaderAddress
    );

export const getActiveTraderBufferAddressHeaderEncoder =
  (): Encoder<ActiveTraderBufferHeaderAddress> =>
    transformEncoder(
      getPubkeyEncoder(),
      (addr: ActiveTraderBufferHeaderAddress): Address => addr
    );

export const getActiveTraderBufferAddressHeaderDecoder =
  (): Decoder<ActiveTraderBufferHeaderAddress> =>
    transformDecoder(
      getPubkeyDecoder(),
      (addr: Address): ActiveTraderBufferHeaderAddress =>
        addr as ActiveTraderBufferHeaderAddress
    );

export const getWithdrawQueueAddressEncoder =
  (): Encoder<WithdrawQueueAddress> =>
    transformEncoder(
      getPubkeyEncoder(),
      (addr: WithdrawQueueAddress): Address => addr
    );

export const getWithdrawQueueAddressDecoder =
  (): Decoder<WithdrawQueueAddress> =>
    transformDecoder(
      getPubkeyDecoder(),
      (addr: Address): WithdrawQueueAddress => addr as WithdrawQueueAddress
    );

export const getMarketAddressEncoder = (): Encoder<MarketAddress> =>
  transformEncoder(getPubkeyEncoder(), (addr: MarketAddress): Address => addr);

export const getMarketAddressDecoder = (): Decoder<MarketAddress> =>
  transformDecoder(
    getPubkeyDecoder(),
    (addr: Address): MarketAddress => addr as MarketAddress
  );

export const getTraderAddressEncoder = (): Encoder<TraderAddress> =>
  transformEncoder(getPubkeyEncoder(), (addr: TraderAddress): Address => addr);

export const getTraderAddressDecoder = (): Decoder<TraderAddress> =>
  transformDecoder(
    getPubkeyDecoder(),
    (addr: Address): TraderAddress => addr as TraderAddress
  );
