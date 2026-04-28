import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  fixDecoderSize,
  fixEncoderSize,
  getConstantDecoder,
  getConstantEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

/**
 * Encoder for SyncParentToChild instruction.
 * This instruction has no additional data - just the discriminant.
 */
export const getSyncParentToChildEncoder = (): Encoder<void> =>
  fixEncoderSize(getConstantEncoder(DISCRIMINANTS.SYNC_PARENT_TO_CHILD), 8);

/**
 * Decoder for SyncParentToChild instruction.
 * This instruction has no additional data - just the discriminant.
 */
export const getSyncParentToChildDecoder = (): Decoder<void> =>
  fixDecoderSize(getConstantDecoder(DISCRIMINANTS.SYNC_PARENT_TO_CHILD), 8);

/**
 * Codec for SyncParentToChild instruction.
 */
export const getSyncParentToChildCodec = (): Codec<void> =>
  combineCodec(getSyncParentToChildEncoder(), getSyncParentToChildDecoder());
