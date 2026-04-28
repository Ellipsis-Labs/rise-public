#!/usr/bin/env bun
/**
 * Publish alpha version of Rise locally for testing
 *
 * This script:
 * 1. Creates an alpha version with format: {version}-alpha.{short-sha}
 * 2. Temporarily updates package.json
 * 3. Builds and publishes to GitHub Packages
 * 4. Restores package.json (NO git commit)
 *
 * Usage:
 *   bun run publish:alpha
 *
 * WARNING: This script is for local testing only.
 * Production releases should be published via CI when merged to master.
 */

import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);
const packageJsonPath = path.join(rootDir, "package.json");
const isDryRun = process.argv.includes("--dry-run");

function run(command: string, options = {}) {
  console.log(`Running: ${command}`);
  execSync(command, { cwd: rootDir, stdio: "inherit", ...options });
}

function getShortSha(): string {
  return execSync("git rev-parse --short HEAD", {
    cwd: rootDir,
    encoding: "utf8",
  }).trim();
}

function checkGitStatus() {
  try {
    const status = execSync("git status --porcelain", {
      cwd: rootDir,
      encoding: "utf8",
    });

    if (status.trim()) {
      console.warn("\n⚠️  Warning: You have uncommitted changes");
      console.warn("Alpha publish will NOT commit the version change");
      console.warn("This is expected behavior.\n");
    }
  } catch {
    console.error("Failed to check git status");
  }
}

function readPackage() {
  const raw = readFileSync(packageJsonPath, "utf8");
  return JSON.parse(raw);
}

function serializePackage(pkg: unknown) {
  return `${JSON.stringify(pkg, null, 2)}\n`;
}

function writePackage(pkg: unknown) {
  writeFileSync(packageJsonPath, serializePackage(pkg), "utf8");
}

function verifyPackageRestored(expectedPkg: unknown) {
  const current = readFileSync(packageJsonPath, "utf8");
  const expected = serializePackage(expectedPkg);

  if (current !== expected) {
    console.error("\n❌ Error: package.json was not properly restored!");
    console.error("Please restore rise/ts/package.json before continuing.");
    process.exit(1);
  }

  console.log("✅ package.json restored successfully");
}

console.log("🚀 Publishing alpha version...\n");

checkGitStatus();

const pkg = readPackage();
const baseVersion = pkg.version as string;
const cleanVersion = baseVersion.split("-")[0];
const shortSha = getShortSha();
const alphaVersion = `${cleanVersion}-alpha.${shortSha}`;

console.log(`📦 Package: ${pkg.name}`);
console.log(`📌 Base version: ${baseVersion}`);
console.log(`🔖 Alpha version: ${alphaVersion}`);
console.log(`🔗 Git SHA: ${shortSha}\n`);
if (isDryRun) {
  console.log("🧪 Dry run enabled; package will be built but not published.\n");
}

const originalPkg = { ...pkg };
pkg.version = alphaVersion;
writePackage(pkg);

try {
  console.log("🔨 Building...");
  run("bun run build");

  if (isDryRun) {
    console.log("\n🧪 Skipping publish because --dry-run was provided");
  } else {
    console.log("\n📤 Publishing to GitHub Packages...");
    run("bun publish --access restricted --tag alpha");
  }

  if (!isDryRun) {
    console.log(`\n✅ Successfully published ${alphaVersion}`);
    console.log("\n📥 Install with:");
    console.log(`   bun add ${pkg.name}@${alphaVersion}`);
  }
} catch (error) {
  console.error("\n❌ Publish failed");
  throw error;
} finally {
  console.log("\n🔄 Restoring package.json...");
  writePackage(originalPkg);
  verifyPackageRestored(originalPkg);
}
