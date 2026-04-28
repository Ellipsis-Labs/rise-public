import type { Decoder, Encoder } from "@solana/kit";
import { getEnumDecoder, getEnumEncoder } from "@solana/kit";
import { Side } from "./types";

export const getSideDecoder = (): Decoder<Side> => getEnumDecoder(Side);
export const getSideEncoder = (): Encoder<Side> => getEnumEncoder(Side);
