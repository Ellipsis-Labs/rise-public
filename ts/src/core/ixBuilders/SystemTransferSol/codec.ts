import { getU32Encoder, getU64Encoder } from "@solana/kit";

/**
 * `SystemInstruction::Transfer` discriminant. The System Program is not a
 * Phoenix program, so this does not live in `DISCRIMINANTS`.
 */
const SYSTEM_TRANSFER_DISCRIMINANT = 2;

/** Encode System Program `Transfer { lamports }` instruction data. */
export const encodeSystemTransferSol = (lamports: bigint): Uint8Array => {
  const discriminant = getU32Encoder().encode(SYSTEM_TRANSFER_DISCRIMINANT);
  const amount = getU64Encoder().encode(lamports);
  const out = new Uint8Array(discriminant.length + amount.length);
  out.set(discriminant, 0);
  out.set(amount, discriminant.length);
  return out;
};
