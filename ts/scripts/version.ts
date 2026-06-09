#!/usr/bin/env bun
/**
 * Bump package version and commit the change
 *
 * ⚠️  WARNING: This script should only be used for PRODUCTION releases.
 *
 * For prerelease/alpha/beta versions, use:
 *   bun run publish:alpha
 *
 * This script:
 * 1. Bumps the version in package.json (following semver)
 * 2. Commits the change with message "chore(rise/ts): release v{version}"
 * 3. Does NOT publish (publishing happens via CI when merged to master)
 *
 * Usage:
 *   bun scripts/version.ts [patch|minor|major]
 */
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

type ReleaseType = "patch" | "minor" | "major";

const allowedTypes: ReleaseType[] = ["patch", "minor", "major"];
const typeArg = (process.argv[2] ?? "patch").toLowerCase();

if (!allowedTypes.includes(typeArg as ReleaseType)) {
  console.error(
    `Unsupported release type "${typeArg}". Use one of: ${allowedTypes.join(
      ", "
    )}.`
  );
  process.exit(1);
}

const releaseType = typeArg as ReleaseType;
const rootDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);
const packageJsonPath = path.join(rootDir, "package.json");

function readPackage(): { version: string; [key: string]: unknown } {
  const raw = readFileSync(packageJsonPath, "utf8");
  return JSON.parse(raw);
}

function bumpVersion(version: string, type: ReleaseType): string {
  const segments = version
    .split(".")
    .map((segment) => Number.parseInt(segment, 10));
  if (segments.length !== 3 || segments.some(Number.isNaN)) {
    throw new Error(`Invalid semver string: ${version}`);
  }

  const [major, minor, patch] = segments;
  switch (type) {
    case "major":
      return `${major + 1}.0.0`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "patch":
    default:
      return `${major}.${minor}.${patch + 1}`;
  }
}

function run(command: string, args: string[]) {
  const result = spawnSync(command, args, { cwd: rootDir, stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(" ")}`);
  }
}

const pkg = readPackage();
const newVersion = bumpVersion(pkg.version as string, releaseType);
pkg.version = newVersion;

writeFileSync(packageJsonPath, `${JSON.stringify(pkg, null, 2)}\n`, "utf8");

const filesToStage = ["package.json"];
const bunLockPath = path.join(rootDir, "bun.lockb");
if (existsSync(bunLockPath)) {
  filesToStage.push("bun.lockb");
}

run("git", ["add", ...filesToStage]);
run("git", ["commit", "-m", `chore(rise/ts): release v${newVersion}`]);

console.log(`Bumped version to ${newVersion}`);
