/**
 * Example: grant a delegated onboarding budget, then onboard traders with it.
 *
 * This demonstrates the off-chain integrator flow:
 *
 * 1. An external team gives Phoenix a `user_key`.
 * 2. Phoenix, or a signer with trader-management delegation from the risk
 *    authority, grants `risk_authority -> user_key` trader-onboarding
 *    permission with N signer actions.
 * 3. The holder of `user_key` signs `RegisterTrader` plus
 *    `SetTraderCapabilitiesDelegated` to onboard a trader.
 *
 * Run the user-side onboarding flow:
 *   bun examples/08-delegated-trader-management-onboarding.ts <TRADER_AUTHORITY> \
 *     --user-keypair-path ./user-keypair.json
 *
 * Run the optional Phoenix-side grant first:
 *   bun examples/08-delegated-trader-management-onboarding.ts <TRADER_AUTHORITY> \
 *     --user-keypair-path ./user-keypair.json \
 *     --grant-manager-keypair-path ./trader-management-keypair.json \
 *     --grant-uses 25
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";

import {
  buildCreatePermissionIx,
  buildRegisterTraderIx,
  buildSetPermissionDelegatedIx,
  createPhoenixClient,
  fetchPermission,
  TRADER_MANAGEMENT_PERMISSION,
  TRADER_ONBOARDING_PERMISSION,
  type AccountFetcherClient,
  type Authority,
  type TraderAddress,
} from "@/index";
import {
  addSignersToInstruction,
  createKeyPairSignerFromBytes,
  setTransactionMessageFeePayerSigner,
  signTransactionMessageWithSigners,
  type KeyPairSigner,
} from "@solana/signers";
import {
  address,
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  fetchEncodedAccount,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageLifetimeUsingBlockhash,
  type Address,
} from "@solana/kit";

type CliArgs = {
  apiUrl: string;
  rpcUrl: string;
  userKeypairPath: string;
  grantManagerKeypairPath: string | undefined;
  grantUses: bigint;
  traderAuthority: string;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  maxPositions: number;
};

const DEFAULT_API_URL = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";
const DEFAULT_KEYPAIR_PATH = `${homedir()}/.config/solana/id.json`;
const DEFAULT_MAX_POSITIONS = 128;

const usage = `Usage:
  bun examples/08-delegated-trader-management-onboarding.ts <TRADER_AUTHORITY> [options]

Options:
  --api-url <url>                       Phoenix API URL
  --rpc-url <url>                       Solana RPC URL
  --user-keypair-path <path>            Delegated user keypair that receives the onboarding budget
  --grant-manager-keypair-path <path>   Optional signer with risk authority or trader-management delegation
  --grant-uses <n>                      Number of onboarding uses to grant when --grant-manager-keypair-path is present (default: 1)
  --trader-pda-index <n>                Trader PDA index (default: 0)
  --trader-subaccount-index <n>         Trader subaccount index (default: 0)
  --max-positions <n>                   Max positions if registering first (default: 128 for subaccount 0, 1 otherwise)
  -h, --help                            Show this help`;

const fail = (message: string): never => {
  console.error(message);
  console.error("");
  console.error(usage);
  process.exit(1);
};

const formatError = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

const parseInteger = (value: string, flag: string): number => {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isInteger(parsed) || parsed < 0) {
    fail(`Invalid value for ${flag}: ${value}`);
  }
  return parsed;
};

const parsePositiveInteger = (value: string, flag: string): number => {
  const parsed = parseInteger(value, flag);
  if (parsed <= 0) {
    fail(`${flag} must be greater than 0`);
  }
  return parsed;
};

const parsePositiveBigInt = (value: string, flag: string): bigint => {
  const parsed = BigInt(parsePositiveInteger(value, flag));
  if (parsed <= 0n) {
    fail(`${flag} must be greater than 0`);
  }
  return parsed;
};

const parseAddress = <T extends Address>(value: string, label: string): T => {
  try {
    return address(value) as T;
  } catch (error) {
    fail(`Invalid ${label}: ${formatError(error)}`);
  }
};

const readKeypairBytes = (path: string): Uint8Array => {
  const raw = JSON.parse(readFileSync(path, "utf8")) as unknown;
  if (!Array.isArray(raw)) {
    throw new Error(`Keypair file must be a JSON array of bytes: ${path}`);
  }

  const bytes = Uint8Array.from(
    raw.map((value) => {
      if (
        typeof value !== "number" ||
        !Number.isInteger(value) ||
        value < 0 ||
        value > 255
      ) {
        throw new Error(`Invalid keypair byte in ${path}: ${String(value)}`);
      }
      return value;
    })
  );

  if (bytes.length !== 64) {
    throw new Error(
      `Keypair file must contain 64 bytes, found ${bytes.length}: ${path}`
    );
  }

  return bytes;
};

const toRpcWebSocketUrl = (rpcUrl: string): string => {
  if (rpcUrl.startsWith("https://")) {
    return `wss://${rpcUrl.slice("https://".length)}`;
  }
  if (rpcUrl.startsWith("http://")) {
    return `ws://${rpcUrl.slice("http://".length)}`;
  }
  return rpcUrl;
};

const parseArgs = (argv: string[]): CliArgs => {
  let apiUrl = process.env.PHOENIX_API_URL ?? DEFAULT_API_URL;
  let rpcUrl =
    process.env.PHOENIX_RPC_URL ??
    process.env.SOLANA_RPC_URL ??
    DEFAULT_RPC_URL;
  let userKeypairPath =
    process.env.DELEGATED_USER_KEYPAIR_PATH ??
    process.env.KEYPAIR_PATH ??
    DEFAULT_KEYPAIR_PATH;
  let grantManagerKeypairPath = process.env.TRADER_MANAGEMENT_KEYPAIR_PATH;
  let grantUses = 1n;
  let traderPdaIndex = 0;
  let traderSubaccountIndex = 0;
  let maxPositions: number | undefined;
  const positional: string[] = [];

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--api-url":
        apiUrl = argv[++index] ?? fail("Missing value for --api-url");
        break;
      case "--rpc-url":
        rpcUrl = argv[++index] ?? fail("Missing value for --rpc-url");
        break;
      case "--user-keypair-path":
        userKeypairPath =
          argv[++index] ?? fail("Missing value for --user-keypair-path");
        break;
      case "--grant-manager-keypair-path":
        grantManagerKeypairPath =
          argv[++index] ??
          fail("Missing value for --grant-manager-keypair-path");
        break;
      case "--grant-uses":
        grantUses = parsePositiveBigInt(
          argv[++index] ?? fail("Missing value for --grant-uses"),
          "--grant-uses"
        );
        break;
      case "--trader-pda-index":
        traderPdaIndex = parseInteger(
          argv[++index] ?? fail("Missing value for --trader-pda-index"),
          "--trader-pda-index"
        );
        break;
      case "--trader-subaccount-index":
        traderSubaccountIndex = parseInteger(
          argv[++index] ?? fail("Missing value for --trader-subaccount-index"),
          "--trader-subaccount-index"
        );
        break;
      case "--max-positions":
        maxPositions = parsePositiveInteger(
          argv[++index] ?? fail("Missing value for --max-positions"),
          "--max-positions"
        );
        break;
      case "-h":
      case "--help":
        console.log(usage);
        process.exit(0);
      default:
        if (arg.startsWith("-")) {
          fail(`Unknown argument: ${arg}`);
        }
        positional.push(arg);
    }
  }

  if (positional.length !== 1) {
    fail("Expected exactly one positional argument: <TRADER_AUTHORITY>");
  }
  if (traderPdaIndex > 255) {
    fail("--trader-pda-index must be between 0 and 255");
  }
  if (traderSubaccountIndex > 255) {
    fail("--trader-subaccount-index must be between 0 and 255");
  }
  const registrationMaxPositions =
    maxPositions ?? (traderSubaccountIndex === 0 ? DEFAULT_MAX_POSITIONS : 1);
  if (registrationMaxPositions > DEFAULT_MAX_POSITIONS) {
    fail("--max-positions cannot exceed 128");
  }
  if (traderSubaccountIndex > 0 && registrationMaxPositions !== 1) {
    fail(
      "--max-positions must be 1 when --trader-subaccount-index is greater than 0"
    );
  }

  return {
    apiUrl,
    rpcUrl,
    userKeypairPath,
    grantManagerKeypairPath,
    grantUses,
    traderAuthority: positional[0],
    traderPdaIndex,
    traderSubaccountIndex,
    maxPositions: registrationMaxPositions,
  };
};

const createAccountClient = (
  rpc: ReturnType<typeof createSolanaRpc>
): AccountFetcherClient => ({
  fetchAccount: async (accountAddress: Address) => {
    const account = await fetchEncodedAccount(rpc, accountAddress);
    if (!account.exists || !account.data || account.data.length === 0) {
      throw new Error(`Account does not exist: ${accountAddress}`);
    }
    return { data: account.data };
  },
  _cacheEnabled: false,
});

const accountExists = async (
  rpc: ReturnType<typeof createSolanaRpc>,
  accountAddress: Address
): Promise<boolean> => {
  const account = await fetchEncodedAccount(rpc, accountAddress);
  return account.exists && !!account.data && account.data.length > 0;
};

const sendInstructions = async (params: {
  instructions: ReturnType<typeof addSignersToInstruction>[];
  signer: KeyPairSigner;
  rpcUrl: string;
}): Promise<string> => {
  const rpc = createSolanaRpc(params.rpcUrl);
  const rpcSubscriptions = createSolanaRpcSubscriptions(
    toRpcWebSocketUrl(params.rpcUrl)
  );
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  const latestBlockhash = await rpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(params.signer, tx),
    (tx) =>
      setTransactionMessageLifetimeUsingBlockhash(latestBlockhash.value, tx),
    (tx) => appendTransactionMessageInstructions(params.instructions, tx)
  );
  const signedTransaction =
    await signTransactionMessageWithSigners(transactionMessage);
  const signature = getSignatureFromTransaction(signedTransaction);

  await sendAndConfirm(
    {
      ...signedTransaction,
      lifetimeConstraint: {
        lastValidBlockHeight: latestBlockhash.value.lastValidBlockHeight,
      },
    },
    { commitment: "confirmed" }
  );

  return signature;
};

const validatePermissionAccount = (
  permission: Awaited<ReturnType<typeof fetchPermission>>,
  expectedAuthority: Address,
  expectedUser: Address,
  requiredPermission: bigint,
  label: string
) => {
  if (permission.permissionAuthority !== expectedAuthority) {
    throw new Error(
      `${label} permission authority mismatch: expected ${expectedAuthority}, got ${permission.permissionAuthority}`
    );
  }
  if (permission.delegatedKey !== expectedUser) {
    throw new Error(
      `${label} permission delegated key mismatch: expected ${expectedUser}, got ${permission.delegatedKey}`
    );
  }
  if ((permission.permission & requiredPermission) !== requiredPermission) {
    throw new Error(
      `${label} permission is missing required bit ${requiredPermission.toString()}`
    );
  }
  if (permission.allowedSignerActions === 0n) {
    throw new Error(`${label} permission has no remaining signer actions`);
  }
};

async function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    console.log(
      "This example can submit live permission-grant and onboarding transactions."
    );
    console.log(
      "The user-side transaction must be signed by the delegated user key that received trader-onboarding permission.\n"
    );
    console.log(usage);
    return;
  }

  const args = parseArgs(argv);
  const traderAuthority = parseAddress<Authority>(
    args.traderAuthority,
    "trader authority"
  );
  const userSigner = await createKeyPairSignerFromBytes(
    readKeypairBytes(args.userKeypairPath)
  );
  const userAuthority = userSigner.address as Authority;
  const rpc = createSolanaRpc(args.rpcUrl);
  const accountClient = createAccountClient(rpc);
  const client = createPhoenixClient({
    apiUrl: args.apiUrl,
    rpcUrl: args.rpcUrl,
    exchangeMetadata: { stream: false },
  });

  try {
    const snapshot = await client.exchange.ready();
    const riskAuthority = parseAddress<Address>(
      snapshot.exchange.currentAuthorities.riskAuthority,
      "risk authority"
    );
    const [logAuthorityAddress, globalConfigurationAddress, traderAccount] =
      await Promise.all([
        client.pda.getLogAuthorityAddress(),
        client.pda.getGlobalConfigurationAddress(),
        client.pda.getTraderAddress({
          authority: traderAuthority,
          traderPdaIndex: args.traderPdaIndex,
          subaccountIndex: args.traderSubaccountIndex,
        }),
      ]);
    const programAddress = client.pda.getProgramAddress();
    const userPermissionAccount = await client.pda.getPermissionAddress({
      permissionAuthority: riskAuthority,
      delegatedKey: userAuthority,
      phoenixProgramAddress: snapshot.exchange.programId,
    });

    let grantSignature: string | undefined;
    if (args.grantManagerKeypairPath) {
      const grantManagerSigner = await createKeyPairSignerFromBytes(
        readKeypairBytes(args.grantManagerKeypairPath)
      );
      const grantAuthority = grantManagerSigner.address as Authority;
      const authorityPermissionAccount =
        grantAuthority === riskAuthority
          ? (grantAuthority as Address)
          : await client.pda.getPermissionAddress({
              permissionAuthority: riskAuthority,
              delegatedKey: grantAuthority,
              phoenixProgramAddress: snapshot.exchange.programId,
            });

      if (grantAuthority !== riskAuthority) {
        const managerPermission = await fetchPermission({
          client: accountClient,
          address: authorityPermissionAccount,
          skipCache: true,
        });
        validatePermissionAccount(
          managerPermission,
          riskAuthority,
          grantAuthority,
          TRADER_MANAGEMENT_PERMISSION,
          "Grant manager"
        );
        console.log(
          `Grant manager uses left before grant: ${managerPermission.allowedSignerActions.toString()}`
        );
      }

      const grantInstructions = [];
      if (!(await accountExists(rpc, userPermissionAccount))) {
        grantInstructions.push(
          buildCreatePermissionIx({
            programAddress,
            logAuthorityAddress,
            payer: grantAuthority,
            permissionAuthority: riskAuthority,
            delegatedKey: userAuthority,
            permissionPda: userPermissionAccount,
          })
        );
      }
      grantInstructions.push(
        buildSetPermissionDelegatedIx({
          programAddress,
          logAuthorityAddress,
          globalConfigurationAddress,
          authority: grantAuthority,
          authorityPermissionAccount,
          targetPermissionAccount: userPermissionAccount,
          permissionAuthority: riskAuthority,
          delegatedKey: userAuthority,
          permission: TRADER_ONBOARDING_PERMISSION,
          allowedSignerActions: args.grantUses,
        })
      );

      const signedGrantInstructions = grantInstructions.map((instruction) =>
        addSignersToInstruction([grantManagerSigner], instruction)
      );
      grantSignature = await sendInstructions({
        instructions: signedGrantInstructions,
        signer: grantManagerSigner,
        rpcUrl: args.rpcUrl,
      });
    }

    const permission = await fetchPermission({
      client: accountClient,
      address: userPermissionAccount,
      skipCache: true,
    });
    validatePermissionAccount(
      permission,
      riskAuthority,
      userAuthority,
      TRADER_ONBOARDING_PERMISSION,
      "User"
    );

    const instructions = [];
    const traderAlreadyRegistered = await accountExists(rpc, traderAccount);
    if (!traderAlreadyRegistered) {
      instructions.push(
        buildRegisterTraderIx({
          programAddress,
          logAuthorityAddress,
          globalConfigurationAddress,
          payer: userAuthority,
          trader: traderAuthority,
          traderAccount: traderAccount as TraderAddress,
          maxPositions: BigInt(args.maxPositions),
          traderPdaIndex: args.traderPdaIndex,
          traderSubaccountIndex: args.traderSubaccountIndex,
        })
      );
    }

    instructions.push(
      await client.ixs.buildOnboardTraderDelegated({
        authority: userAuthority,
        traderAuthority,
        permissionAccount: userPermissionAccount,
        traderPdaIndex: args.traderPdaIndex,
        traderSubaccountIndex: args.traderSubaccountIndex,
      })
    );

    const signedInstructions = instructions.map((instruction) =>
      addSignersToInstruction([userSigner], instruction)
    );
    const onboardingSignature = await sendInstructions({
      instructions: signedInstructions,
      signer: userSigner,
      rpcUrl: args.rpcUrl,
    });

    console.log(
      JSON.stringify(
        {
          grantSignature,
          onboardingSignature,
          userAuthority,
          traderAuthority,
          traderAccount,
          riskAuthority,
          userPermissionAccount,
          permission: permission.permission.toString(),
          allowedSignerActions: permission.allowedSignerActions.toString(),
          traderAlreadyRegistered,
          grantUses: args.grantUses.toString(),
          instructionCount: instructions.length,
        },
        null,
        2
      )
    );
  } finally {
    client.dispose();
  }
}

void main().catch((error) => {
  console.error(formatError(error));
  process.exitCode = 1;
});
