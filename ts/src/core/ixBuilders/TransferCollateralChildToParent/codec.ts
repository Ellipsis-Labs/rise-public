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
 * Encoder for TransferCollateralChildToParent instruction.
 * This instruction has no additional data - just the discriminant.
 */
export const getTransferCollateralChildToParentEncoder = (): Encoder<void> =>
  fixEncoderSize(
    getConstantEncoder(DISCRIMINANTS.TRANSFER_COLLATERAL_CHILD_TO_PARENT),
    8
  );

/**
 * Decoder for TransferCollateralChildToParent instruction.
 * This instruction has no additional data - just the discriminant.
 */
export const getTransferCollateralChildToParentDecoder = (): Decoder<void> =>
  fixDecoderSize(
    getConstantDecoder(DISCRIMINANTS.TRANSFER_COLLATERAL_CHILD_TO_PARENT),
    8
  );

/**
 * Codec for TransferCollateralChildToParent instruction.
 */
export const getTransferCollateralChildToParentCodec = (): Codec<void> =>
  combineCodec(
    getTransferCollateralChildToParentEncoder(),
    getTransferCollateralChildToParentDecoder()
  );
