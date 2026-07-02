import { spawnSync } from "node:child_process";

type Target = {
  file: string;
  label: string;
  expectedPassed: number;
  fullTestName?: string;
};

const FLOW_SUITE = "SDK localnet common flows";
const VM_SUITE = "SDK localnet VM harness";

const targets: readonly Target[] = [
  {
    file: "tests/sdk-localnet-fixture.test.ts",
    label: "SDK localnet fixture",
    expectedPassed: 7,
  },
  {
    file: "tests/sdk-localnet-vm.test.ts",
    label: "SDK localnet VM order flow",
    expectedPassed: 1,
    fullTestName: `${VM_SUITE} loads locally built programs, replays fixture setup, and runs an SDK-built order`,
  },
  {
    file: "tests/sdk-localnet-vm.test.ts",
    label: "SDK localnet VM deposit/withdraw",
    expectedPassed: 1,
    fullTestName: `${VM_SUITE} builds and executes SDK deposit and withdraw instructions`,
  },
  ...[
    "deposits collateral",
    "registers a trader subaccount and syncs parent capabilities",
    "places a market order",
    "attaches a stop loss after opening a position",
    "closes a stop loss account funded by a separate payer",
    "closes a position with an opposite market order",
    "places a limit order",
    "places a limit order with attached conditional orders",
    "cancels an attached conditional order",
    "cancels a limit order by id",
    "cancels a limit order by id through the public helper using tick price",
    "cancels all orders",
    "transfers collateral to a subaccount",
    "places an isolated market order from a subaccount",
    "places an isolated limit order from a subaccount",
    "closes an isolated position",
    "transfers collateral from a subaccount back to its parent",
    "rejects withdraw before 150 slots have passed",
    "waits 150 slots and withdraws collateral",
  ].map((name) => ({
    file: "tests/sdk-localnet-flows.test.ts",
    label: `SDK localnet flow: ${name}`,
    expectedPassed: 1,
    fullTestName: `${FLOW_SUITE} ${name}`,
  })),
];

const baseArgs = [
  "vitest",
  "run",
  "--pool=forks",
  "--no-file-parallelism",
  "--maxWorkers=1",
  "--reporter=verbose",
  "--hideSkippedTests",
  "--no-color",
];

for (const target of targets) {
  const args = [...baseArgs, target.file];
  if (target.fullTestName) {
    args.push("-t", `^${escapeRegex(target.fullTestName)}$`);
  }

  group(target.label);
  const result = spawnSync("bunx", args, {
    encoding: "utf8",
    env: process.env,
  });
  const stdout = result.stdout ?? "";
  const stderr = result.stderr ?? "";
  process.stdout.write(stdout);
  process.stderr.write(stderr);
  endGroup();

  if (result.error) {
    console.error(result.error);
    process.exit(1);
  }

  if (result.signal) {
    console.error(`${target.label} terminated by signal ${result.signal}`);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  const passed = countPassedTests(`${stdout}\n${stderr}`);
  if (passed < target.expectedPassed) {
    console.error(
      `${target.label} only reported ${passed} passed test(s); expected ${target.expectedPassed}.`
    );
    process.exit(1);
  }
}

function escapeRegex(value: string): string {
  return value.replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
}

function group(label: string): void {
  if (process.env.GITHUB_ACTIONS === "true") {
    console.log(`::group::${label}`);
    return;
  }

  console.log(`\n${label}`);
}

function endGroup(): void {
  if (process.env.GITHUB_ACTIONS === "true") {
    console.log("::endgroup::");
  }
}

function countPassedTests(output: string): number {
  const testsSummary = /^ *Tests +(\d+) passed/m.exec(output);
  if (!testsSummary?.[1]) {
    return 0;
  }

  return Number(testsSummary[1]);
}
