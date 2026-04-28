import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getOptionToNullDecoder,
  getOptionToNullEncoder,
} from "@/core/utils/optionCodec";
import {
  combineCodec,
  getArrayDecoder,
  getArrayEncoder,
  getConstantDecoder,
  getConstantEncoder,
  getDiscriminatedUnionDecoder,
  getDiscriminatedUnionEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  getU8Decoder,
  getU8Encoder,
  getU64Decoder,
  getU64Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { EscrowAction } from "./types";

export type InternalEscrowAction =
  | { __kind: "noop"; _padding0: number[]; _padding1: number[] }
  | { __kind: "cash"; amount: bigint | number; _padding0: number[] };

const getEscrowActionEncoder = () =>
  getDiscriminatedUnionEncoder([
    [
      "noop",
      getStructEncoder([
        ["_padding0", getArrayEncoder(getU8Encoder(), { size: 8 })],
        ["_padding1", getArrayEncoder(getU8Encoder(), { size: 128 })],
      ]),
    ],
    [
      "cash",
      getStructEncoder([
        ["amount", getU64Encoder()],
        ["_padding0", getArrayEncoder(getU8Encoder(), { size: 128 })],
      ]),
    ],
  ]);

const getEscrowActionDecoder = () =>
  getDiscriminatedUnionDecoder([
    [
      "noop",
      getStructDecoder([
        ["_padding0", getArrayDecoder(getU8Decoder(), { size: 8 })],
        ["_padding1", getArrayDecoder(getU8Decoder(), { size: 128 })],
      ]),
    ],
    [
      "cash",
      getStructDecoder([
        ["amount", getU64Decoder()],
        ["_padding0", getArrayDecoder(getU8Decoder(), { size: 128 })],
      ]),
    ],
  ]);

export interface CreateEscrowRequestData {
  senderPdaIndex: number;
  senderSubaccountIndex: number;
  receiverPdaIndex: number;
  receiverSubaccountIndex: number;
  lastValidSlot: bigint | null;
  actions: InternalEscrowAction[];
}

export const normalizeActions = (
  actions: EscrowAction[]
): InternalEscrowAction[] =>
  actions.map((action): InternalEscrowAction => {
    if (action.kind === "noop") {
      return {
        __kind: "noop",
        _padding0: new Array<number>(8).fill(0),
        _padding1: new Array<number>(128).fill(0),
      };
    }

    return {
      __kind: "cash",
      amount: action.amount,
      _padding0: new Array<number>(128).fill(0),
    };
  });

export const getCreateEscrowRequestParamsEncoder =
  (): Encoder<CreateEscrowRequestData> =>
    getStructEncoder([
      ["senderPdaIndex", getU8Encoder()],
      ["senderSubaccountIndex", getU8Encoder()],
      ["receiverPdaIndex", getU8Encoder()],
      ["receiverSubaccountIndex", getU8Encoder()],
      ["lastValidSlot", getOptionToNullEncoder(getU64Encoder())],
      ["actions", getArrayEncoder(getEscrowActionEncoder())],
    ]);

export const getCreateEscrowRequestParamsDecoder =
  (): Decoder<CreateEscrowRequestData> =>
    getStructDecoder([
      ["senderPdaIndex", getU8Decoder()],
      ["senderSubaccountIndex", getU8Decoder()],
      ["receiverPdaIndex", getU8Decoder()],
      ["receiverSubaccountIndex", getU8Decoder()],
      ["lastValidSlot", getOptionToNullDecoder(getU64Decoder())],
      ["actions", getArrayDecoder(getEscrowActionDecoder())],
    ]);

export const getCreateEscrowRequestParamsCodec =
  (): Codec<CreateEscrowRequestData> =>
    combineCodec(
      getCreateEscrowRequestParamsEncoder(),
      getCreateEscrowRequestParamsDecoder()
    );

export const getCreateEscrowRequestEncoder =
  (): Encoder<CreateEscrowRequestData> =>
    getHiddenPrefixEncoder(getCreateEscrowRequestParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.CREATE_ESCROW_REQUEST),
    ]);

export const getCreateEscrowRequestDecoder =
  (): Decoder<CreateEscrowRequestData> =>
    getHiddenPrefixDecoder(getCreateEscrowRequestParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.CREATE_ESCROW_REQUEST),
    ]);

export const getCreateEscrowRequestCodec = (): Codec<CreateEscrowRequestData> =>
  combineCodec(
    getCreateEscrowRequestEncoder(),
    getCreateEscrowRequestDecoder()
  );
