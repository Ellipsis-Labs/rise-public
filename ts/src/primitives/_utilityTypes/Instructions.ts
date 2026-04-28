import type {
  AccountMeta,
  InstructionWithAccounts,
  InstructionWithData,
  ReadonlyUint8Array,
} from "@solana/kit";
import type { FixedLengthArray } from "./FixedLengthArray";

export type InstructionsWithAccountsAndData = Readonly<
  InstructionWithAccounts<readonly AccountMeta[]> &
    InstructionWithData<ReadonlyUint8Array>
>;

export type InstructionsWithFixedAccountsAndData<N extends number> =
  InstructionWithAccounts<FixedLengthArray<AccountMeta, N>> &
    InstructionWithData<ReadonlyUint8Array>;
