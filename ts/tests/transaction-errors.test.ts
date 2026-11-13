import { address, type Instruction, type TransactionError } from "@solana/kit";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import * as z from "zod";
import {
  decodeInstructionError,
  decodeProgramError,
  identifyInstruction,
  parseTransactionError,
} from "../src/index";

const PHOENIX = address("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih");
const EMBER = address("EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy");
const SYSTEM = address("11111111111111111111111111111111");

const cases = z
  .array(
    z.object({
      name: z.string(),
      instructions: z.array(
        z.object({
          programAddress: z.string(),
          data: z.array(z.number().int().min(0).max(255)),
        })
      ),
      error: z.object({
        InstructionError: z.tuple([
          z.number().int(),
          z.union([
            z.object({ Custom: z.number().int() }),
            z.enum([
              "AccountAlreadyInitialized",
              "AccountDataTooSmall",
              "AccountNotRentExempt",
              "ArithmeticOverflow",
              "BorshIoError",
              "ComputationalBudgetExceeded",
              "IllegalOwner",
              "IncorrectAuthority",
              "IncorrectProgramId",
              "InsufficientFunds",
              "InvalidAccountData",
              "InvalidAccountOwner",
              "InvalidArgument",
              "InvalidInstructionData",
              "InvalidRealloc",
              "InvalidSeeds",
              "MissingAccount",
              "MissingRequiredSignature",
              "NotEnoughAccountKeys",
              "PrivilegeEscalation",
              "ProgramFailedToComplete",
              "ReadonlyDataModified",
              "ReadonlyLamportChange",
              "UninitializedAccount",
            ]),
          ]),
        ]),
      }),
      logs: z.array(z.string()).nullable(),
      instructionName: z.string().nullable(),
      failure: z
        .object({
          programAddress: z.string(),
          programName: z.string().nullable(),
          instructionName: z.string().nullable(),
          errorCode: z.number().nullable(),
          errorName: z.string().nullable(),
          message: z.string(),
          hint: z
            .literal("conditionalOrdersAccountUninitialized")
            .nullable()
            .optional(),
        })
        .nullable(),
    })
  )
  .parse(
    JSON.parse(
      readFileSync(
        new URL("../../test-fixtures/transaction-errors.json", import.meta.url),
        "utf8"
      )
    )
  );

describe("transaction error debugging hints", () => {
  it.each(cases)("$name", (testCase) => {
    const instructions: Instruction[] = testCase.instructions.map((ix) => ({
      programAddress: address(ix.programAddress),
      data: Uint8Array.from(ix.data),
    }));
    const error: TransactionError = testCase.error;
    const parsed = parseTransactionError({
      instructions,
      error,
      logs: testCase.logs,
    });
    expect(parsed?.instructionIndex).toBe(testCase.error.InstructionError[0]);
    expect(parsed?.error).toEqual(testCase.error.InstructionError[1]);
    expect(parsed?.instruction?.instructionName ?? null).toBe(
      testCase.instructionName
    );
    const failure = parsed?.failure;
    expect(
      failure
        ? {
            ...failure.instruction,
            errorCode: failure.errorCode,
            errorName: failure.errorName,
            message: failure.message,
            hint: failure.hint,
          }
        : null
    ).toEqual(
      testCase.failure === null
        ? null
        : { ...testCase.failure, hint: testCase.failure.hint ?? null }
    );
  });

  it("keeps error codes scoped to their program and tolerates future instructions", () => {
    expect(decodeProgramError(PHOENIX, 0x5001)?.name).toBe(
      "MarginInsufficient"
    );
    expect(decodeProgramError(EMBER, 0x5001)).toBeNull();
    expect(decodeProgramError(SYSTEM, 0x5001)).toBeNull();
    expect(
      decodeProgramError(
        address("phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"),
        0x5001
      )
    ).toEqual(decodeProgramError(PHOENIX, 0x5001));
    expect(
      identifyInstruction({ programAddress: PHOENIX, data: new Uint8Array(2) })
        .instructionName
    ).toBeNull();
    expect(
      decodeInstructionError({ programAddress: PHOENIX }, 0xffff_ffff).errorCode
    ).toBe(0xffff_ffff);
    expect(decodeProgramError(PHOENIX, -1)).toBeNull();
    expect(() =>
      decodeInstructionError({ programAddress: PHOENIX }, 1.5)
    ).toThrow(RangeError);
    expect(
      parseTransactionError({ instructions: [], error: "BlockhashNotFound" })
    ).toBeNull();
    expect(parseTransactionError({ instructions: [], error: null })).toBeNull();
  });

  it("decodes sidecars without mixing overlapping codes and supports short discriminators", () => {
    expect(
      decodeProgramError(
        address("FLickryJ5sBSyBRj7sSzMnsP93jvPxEqNKjvG49q9CEU"),
        8003
      )?.name
    ).toBe("BboUnavailableRetry");
    expect(decodeProgramError(PHOENIX, 8003)).toBeNull();
    const hawkeye = address("RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj");
    expect(decodeProgramError(hawkeye, 0x2102)?.name).toBe(
      "MarketInvalidAssetId"
    );
    expect(decodeProgramError(hawkeye, 0x3001)?.name).toBe(
      "OracleMarkPriceUnavailable"
    );
    expect(decodeProgramError(EMBER, 6000)?.name).toBe("Unauthorized");
    const punchcard = address("pcWKVSdcdDUKabPz4pVfaQ2jMod1kWv3LqeQivjKXiF");
    expect(
      identifyInstruction({ programAddress: punchcard, data: Uint8Array.of(0) })
        .instructionName
    ).toBe("Create");
    expect(
      identifyInstruction({ programAddress: punchcard }).instructionName
    ).toBeNull();
  });
});
