import {
  isAddress,
  type Address,
  type Instruction,
  type TransactionError,
} from "@solana/kit";
import { PROGRAMS, type ProgramMetadata } from "./generated/programMetadata";

/** A custom error defined by a program's IDL. Messages retain `<field>` placeholders. */
export interface ProgramErrorInfo {
  readonly code: number;
  readonly name: string;
  readonly message: string;
}

/** The identity of an instruction, without decoding its arguments. */
export interface InstructionInfo {
  readonly programAddress: Address;
  readonly programName: string | null;
  readonly instructionName: string | null;
}

export interface InstructionFailure {
  readonly instruction: InstructionInfo;
  /** Null for Solana built-in instruction errors. */
  readonly errorCode: number | null;
  readonly errorName: string | null;
  readonly message: string;
  /** Diagnostic context from the failing invocation, not a program error code. */
  readonly hint: InstructionFailureHint | null;
}

export type InstructionFailureHint = "conditionalOrdersAccountUninitialized";

/** Pass only the returning invocation's runtime reason and own logs. */
export function identifyInstructionFailureHint({
  programAddress,
  reason,
  logs,
}: {
  readonly programAddress: Address;
  readonly reason: string;
  readonly logs: readonly string[];
}): InstructionFailureHint | null {
  if (
    metadata(programAddress)?.name !== "Eternal" ||
    reason !== "invalid account data for instruction"
  )
    return null;
  return logs.some((line) => {
    if (!line.startsWith("Program log: ")) return false;
    let detail = line.slice("Program log: ".length);
    if (detail.startsWith('"')) {
      const end = detail.indexOf('"\n', 1);
      if (end < 0) return false;
      detail = detail.slice(1, end);
    }
    const match =
      /^Invalid account data length for (?:[A-Za-z_][A-Za-z_0-9]*::)*ConditionalOrderHeader\. Size: 0, Expected: ([1-9][0-9]*)$/.exec(
        detail
      );
    return match !== null;
  })
    ? "conditionalOrdersAccountUninitialized"
    : null;
}

type RpcInstructionError = Extract<
  TransactionError,
  { InstructionError: unknown }
>["InstructionError"][1];

export interface TransactionFailure {
  readonly instructionIndex: number;
  readonly instruction: InstructionInfo | null;
  /** The original Solana error survives even when hints cannot be decoded. */
  readonly error: RpcInstructionError;
  /**
   * A nested failure is a propagation hint: a caller can catch and later return
   * the same error. Its instruction name is absent without CPI data.
   */
  readonly failure: InstructionFailure | null;
}

const metadata = (programAddress: Address): ProgramMetadata | undefined =>
  PROGRAMS[
    programAddress === "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
      ? "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
      : programAddress
  ];

const isErrorCode = (code: number): boolean =>
  Number.isInteger(code) && code >= 0 && code <= 0xffff_ffff;

/** Looks up an error only in the IDL of the program that returned it. */
export function decodeProgramError(
  programAddress: Address,
  code: number
): ProgramErrorInfo | null {
  if (!isErrorCode(code)) return null;
  const error = metadata(programAddress)?.errors.find(
    ([candidate]) => candidate === code
  );
  return error ? { code, name: error[1], message: error[2] } : null;
}

/** Unknown programs, short data, and unknown discriminators have no decoded name. */
export function identifyInstruction(instruction: Instruction): InstructionInfo {
  const program = metadata(instruction.programAddress);
  const { data } = instruction;
  const entry =
    data !== undefined
      ? program?.instructions
          .filter(
            ([discriminator]) =>
              data.length >= discriminator.length &&
              discriminator.every((byte, index) => data[index] === byte)
          )
          .sort(([left], [right]) => right.length - left.length)[0]
      : undefined;
  return {
    programAddress: instruction.programAddress,
    programName: program?.name ?? null,
    instructionName: entry?.[1] ?? null,
  };
}

/**
 * Decodes a custom error for its originating instruction. For a propagated CPI
 * error, pass the CPI instruction rather than the caller. Unknown codes retain
 * their number and a generic message.
 */
export function decodeInstructionError(
  instruction: Instruction,
  code: number
): InstructionFailure {
  if (!isErrorCode(code))
    throw new RangeError("Custom error code must be a u32");
  const error = decodeProgramError(instruction.programAddress, code);
  return {
    instruction: identifyInstruction(instruction),
    errorCode: code,
    errorName: error?.name ?? null,
    message: error?.message ?? `Custom program error: 0x${code.toString(16)}`,
    hint: null,
  };
}

export interface ParseTransactionErrorInput {
  readonly instructions: readonly Instruction[];
  readonly error: TransactionError | null;
  readonly logs?: readonly string[] | null;
}

/**
 * Parses an RPC/simulation error using resolved top-level instructions. Returns
 * null for success or transaction-wide errors (e.g. BlockhashNotFound). Missing,
 * truncated, or mismatched logs leave failure null; no caller's custom error is
 * decoded without invocation evidence.
 */
export function parseTransactionError({
  instructions,
  error,
  logs,
}: ParseTransactionErrorInput): TransactionFailure | null {
  if (
    error === null ||
    typeof error !== "object" ||
    !("InstructionError" in error)
  )
    return null;
  const [index, instructionError] = error.InstructionError;
  if (!Number.isInteger(index) || index < 0 || index > 255) return null;
  const instruction = instructions.at(index);
  const roots = logs ? parseFrames(logs) : null;
  const result: TransactionFailure = {
    instructionIndex: index,
    instruction: instruction ? identifyInstruction(instruction) : null,
    error: instructionError,
    failure: null,
  };
  if (
    !instruction ||
    !roots ||
    roots.length !== index + 1 ||
    roots.some(
      (frame, i) =>
        frame.programAddress !== instructions[i]?.programAddress ||
        (i < index && frame.reason !== null)
    )
  )
    return result;
  let frame = roots[index];
  if (frame.reason === null) return result;
  const reason = frame.reason;
  const code = customCode(reason);
  const nativeReason =
    typeof instructionError === "string"
      ? NATIVE_ERROR_REASONS[instructionError]
      : undefined;
  if (
    typeof instructionError === "object"
      ? code !== instructionError.Custom
      : code !== null || nativeReason === undefined || reason !== nativeReason
  )
    return result;
  let nested = false;
  for (;;) {
    const child = frame.children
      .slice()
      .reverse()
      .find((child) => child.reason === frame.reason);
    if (!child) break;
    frame = child;
    nested = true;
  }
  const failedInstruction = nested
    ? { programAddress: frame.programAddress }
    : instruction;
  let failure: InstructionFailure =
    code === null
      ? {
          instruction: identifyInstruction(failedInstruction),
          errorCode: null,
          errorName: null,
          message: reason,
          hint:
            instructionError === "InvalidAccountData"
              ? identifyInstructionFailureHint({
                  programAddress: frame.programAddress,
                  reason,
                  logs: frame.logs,
                })
              : null,
        }
      : decodeInstructionError(failedInstruction, code);
  if (failure.errorName !== null) {
    const prefix = `Program log: ${failure.errorName}: `;
    const message = frame.logs
      .slice()
      .reverse()
      .find((line) => line.startsWith(prefix));
    if (message !== undefined)
      failure = { ...failure, message: message.slice(prefix.length) };
  }
  return { ...result, failure };
}

const NATIVE_ERROR_REASONS: Record<
  Exclude<RpcInstructionError, object>,
  string
> = {
  AccountAlreadyInitialized: "instruction requires an uninitialized account",
  AccountBorrowFailed:
    "instruction tries to borrow reference for an account which is already borrowed",
  AccountBorrowOutstanding:
    "instruction left account with an outstanding borrowed reference",
  AccountDataSizeChanged:
    "program other than the account's owner changed the size of the account data",
  AccountDataTooSmall: "account data too small for instruction",
  AccountNotExecutable: "instruction expected an executable account",
  AccountNotRentExempt:
    "An account does not have enough lamports to be rent-exempt",
  ArithmeticOverflow: "Program arithmetic overflowed",
  BorshIoError: "Failed to serialize or deserialize account data",
  BuiltinProgramsMustConsumeComputeUnits:
    "Builtin programs must consume compute units",
  CallDepth: "Cross-program invocation call depth too deep",
  ComputationalBudgetExceeded: "Computational budget exceeded",
  DuplicateAccountIndex: "instruction contains duplicate accounts",
  DuplicateAccountOutOfSync:
    "instruction modifications of multiply-passed account differ",
  ExecutableAccountNotRentExempt: "executable accounts must be rent exempt",
  ExecutableDataModified: "instruction changed executable accounts data",
  ExecutableLamportChange:
    "instruction changed the balance of an executable account",
  ExecutableModified: "instruction changed executable bit of an account",
  ExternalAccountDataModified:
    "instruction modified data of an account it does not own",
  ExternalAccountLamportSpend:
    "instruction spent from the balance of an account it does not own",
  GenericError: "generic instruction error",
  IllegalOwner: "Provided owner is not allowed",
  Immutable: "Account is immutable",
  IncorrectAuthority: "Incorrect authority provided",
  IncorrectProgramId: "incorrect program id for instruction",
  InsufficientFunds: "insufficient funds for instruction",
  InvalidAccountData: "invalid account data for instruction",
  InvalidAccountOwner: "Invalid account owner",
  InvalidArgument: "invalid program argument",
  InvalidError: "program returned invalid error code",
  InvalidInstructionData: "invalid instruction data",
  InvalidRealloc: "Failed to reallocate account data",
  InvalidSeeds: "Provided seeds do not result in a valid address",
  MaxAccountsDataAllocationsExceeded:
    "Accounts data allocations exceeded the maximum allowed per transaction",
  MaxAccountsExceeded: "Max accounts exceeded",
  MaxInstructionTraceLengthExceeded: "Max instruction trace length exceeded",
  MaxSeedLengthExceeded:
    "Length of the seed is too long for address generation",
  MissingAccount: "An account required by the instruction is missing",
  MissingRequiredSignature: "missing required signature for instruction",
  ModifiedProgramId:
    "instruction illegally modified the program id of an account",
  NotEnoughAccountKeys: "insufficient account keys for instruction",
  PrivilegeEscalation:
    "Cross-program invocation with unauthorized signer or writable account",
  ProgramEnvironmentSetupFailure:
    "Failed to create program execution environment",
  ProgramFailedToCompile: "Program failed to compile",
  ProgramFailedToComplete: "Program failed to complete",
  ReadonlyDataModified: "instruction modified data of a read-only account",
  ReadonlyLamportChange:
    "instruction changed the balance of a read-only account",
  ReentrancyNotAllowed:
    "Cross-program invocation reentrancy not allowed for this instruction",
  RentEpochModified: "instruction modified rent epoch of an account",
  UnbalancedInstruction:
    "sum of account balances before and after instruction do not match",
  UninitializedAccount: "instruction requires an initialized account",
  UnsupportedProgramId: "Unsupported program id",
  UnsupportedSysvar: "Unsupported sysvar",
};

const customCode = (reason: string): number | null => {
  const match = /^custom program error: 0x([\da-f]+)$/i.exec(reason);
  if (!match) return null;
  const code = Number.parseInt(match[1], 16);
  return isErrorCode(code) ? code : null;
};

interface Frame {
  programAddress: Address;
  reason: string | null;
  logs: string[];
  children: Frame[];
}

/** A complete balanced trace is required before any frame is used as evidence. */
function parseFrames(logs: readonly string[]): Frame[] | null {
  const roots: Frame[] = [];
  const stack: Frame[] = [];
  for (const line of logs) {
    const current = stack.at(-1);
    if (line.startsWith("Program log:") || line.startsWith("Program data:")) {
      if (!current) return null;
      current.logs.push(line);
      continue;
    }
    const invoke = /^Program (\S+) invoke \[(\d+)\]$/.exec(line);
    if (invoke) {
      if (
        !isAddress(invoke[1]) ||
        Number(invoke[2]) !== stack.length + 1 ||
        stack.length >= 64
      )
        return null;
      stack.push({
        programAddress: invoke[1],
        reason: null,
        logs: [],
        children: [],
      });
      continue;
    }
    const exit = /^Program (\S+) (success|failed: (.+))$/.exec(line);
    if (exit) {
      const frame = stack.pop();
      if (!frame || frame.programAddress !== exit[1]) return null;
      frame.reason = exit[2] === "success" ? null : exit[3];
      const parent = stack.at(-1);
      if (parent) parent.children.push(frame);
      else roots.push(frame);
      continue;
    }
    const consumed = /^Program (\S+) consumed /.exec(line);
    if (consumed) {
      if (!current || current.programAddress !== consumed[1]) return null;
      continue;
    }
    if (!current) return null;
    current.logs.push(line);
  }
  return stack.length === 0 ? roots : null;
}
