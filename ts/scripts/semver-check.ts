#!/usr/bin/env bun
/**
 * Fail when package.json's version is too small a bump for the public API
 * changes since the latest npm release (the TypeScript counterpart of
 * `cargo semver-checks` for the Rust SDK).
 *
 * Diffs the built declarations in dist/ (run `bun run build` first) against
 * the latest release on the public npm registry with ts-semver-checks. As in
 * Cargo, a 0.x minor bump is the breaking-change bump and a 0.x patch bump
 * covers additions.
 *
 * Usage:
 *   bun scripts/semver-check.ts
 */
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import z from "zod";

const BumpLevelSchema = z.enum(["major", "minor", "patch"]);
type Level = z.infer<typeof BumpLevelSchema>;
type Version = { major: number; minor: number; patch: number };

const NPM_REGISTRY = "https://registry.npmjs.org";
const PackageReleaseSchema = z.object({
  name: z.string(),
  version: z.string(),
});
const NpmMetadataSchema = z.object({
  "dist-tags": z.object({ latest: z.string().min(1) }),
});

const rootDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);

function parseVersion(version: string): Version {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) {
    throw new Error(`Expected an X.Y.Z version, got: ${version}`);
  }
  const [major, minor, patch] = match.slice(1).map(Number);
  return { major, minor, patch };
}

/** The largest API change the local version is allowed to carry over `base`. */
function allowedLevel(base: Version, local: Version): Level {
  const order =
    local.major - base.major ||
    local.minor - base.minor ||
    local.patch - base.patch;
  if (order < 0) {
    throw new Error("package.json version is older than the latest release");
  }
  if (local.major > base.major) return "major";
  if (local.major === 0) {
    if (local.minor > base.minor) return "major";
    if (local.patch > base.patch) return "minor";
    return "patch";
  }
  if (local.minor > base.minor) return "minor";
  return "patch";
}

/** Smallest version the semver rules accept for a `level` change over `base`. */
function requiredVersion(base: Version, level: Level): string {
  if (base.major === 0) {
    return level === "major"
      ? `0.${base.minor + 1}.0`
      : `0.${base.minor}.${base.patch + 1}`;
  }
  if (level === "major") return `${base.major + 1}.0.0`;
  if (level === "minor") return `${base.major}.${base.minor + 1}.0`;
  return `${base.major}.${base.minor}.${base.patch + 1}`;
}

/** Emits a GitHub Actions error annotation so the fix shows on the PR check. */
function annotate(title: string, message: string): void {
  if (process.env.GITHUB_ACTIONS !== "true") return;
  const escaped = message
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A");
  console.log(`::error title=${title}::${escaped}`);
}

function fail(title: string, message: string, status: number): never {
  console.error(`\n${message}`);
  annotate(title, message);
  process.exit(status);
}

async function latestRelease(name: string): Promise<string> {
  const url = `${NPM_REGISTRY}/${name.replace("/", "%2F")}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`GET ${url} failed: ${response.status}`);
  }
  const metadata = NpmMetadataSchema.safeParse(await response.json());
  if (!metadata.success) {
    throw new Error(`${name} has no latest release on ${NPM_REGISTRY}`);
  }
  return metadata.data["dist-tags"].latest;
}

const pkg = PackageReleaseSchema.parse(
  JSON.parse(readFileSync(path.join(rootDir, "package.json"), "utf8"))
);
const baseline = await latestRelease(pkg.name);
const baselineVersion = parseVersion(baseline);
const expect = allowedLevel(baselineVersion, parseVersion(pkg.version));

console.log(
  `${pkg.name}: ${baseline} (npm) -> ${pkg.version} allows ${expect}-level API changes`
);

const result = spawnSync(
  "ts-semver-checks",
  ["--baseline", baseline, "--expect", expect, "--no-color"],
  {
    cwd: rootDir,
    stdio: ["inherit", "pipe", "inherit"],
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }
);
if (result.error) {
  throw result.error;
}
process.stdout.write(result.stdout);

// ts-semver-checks exits 0 when within --expect, 1 when the changes need a
// bigger bump, and 2/3 for usage and baseline-fetch errors.
if (result.status === 0) {
  process.exit(0);
}
const required = BumpLevelSchema.safeParse(
  /Required version bump: (MAJOR|MINOR|PATCH)/i
    .exec(result.stdout)?.[1]
    ?.toLowerCase()
);
if (result.status !== 1 || !required.success) {
  fail(
    "Rise TypeScript semver check could not run",
    `ts-semver-checks failed before it could compare ${pkg.name} against ${baseline} (see the error above). This is not an API finding: fix the build (bun run build) or registry error, then rerun.`,
    result.status ?? 1
  );
}
const level = required.data;

const target = requiredVersion(baselineVersion, level);
const change = level === "major" ? "breaking" : "additive (non-breaking)";
fail(
  `Rise TypeScript SDK needs version ${target} (currently ${pkg.version})`,
  `${pkg.name} has ${change} public API changes since the latest npm release ${baseline}, but package.json is at ${pkg.version}. Semver needs at least ${target}. The findings above list each change.

Either:

1. Release a new version: set "version" in rise/ts/package.json to "${target}",
   then rerun this check: bun run build && bun run semver-check

2. Or, if the change is unintended, restore compatibility for each finding
   (e.g. make a new interface member optional, or keep a removed export as a
   @deprecated alias instead of deleting it).`,
  1
);
