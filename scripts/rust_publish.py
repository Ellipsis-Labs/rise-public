#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# ///
"""Validate, package, and publish Rise Rust workspace crates."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, NoReturn

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_WORKSPACE = REPO_ROOT / "rust"
DEFAULT_REGISTRY_URL = "https://crates.io"
USER_AGENT = "Ellipsis-Labs/rise-release-ci"


@dataclasses.dataclass(frozen=True)
class Package:
    id: str
    name: str
    version: str
    manifest_path: Path
    publish: Any
    dependencies: list[dict[str, Any]]


@dataclasses.dataclass(frozen=True)
class Workspace:
    root: Path
    packages: dict[str, Package]
    publishable: dict[str, Package]


@dataclasses.dataclass(frozen=True)
class ValidatedOrder:
    workspace: Workspace
    order: list[str]
    version: str | None


def fail(message: str, code: int = 1) -> NoReturn:
    print(f"Error: {message}", file=sys.stderr)
    raise SystemExit(code)


def cargo_command(toolchain: str | None) -> list[str]:
    if toolchain:
        return ["cargo", f"+{toolchain}"]
    return ["cargo"]


def run(
    cmd: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    capture: bool = False,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    print("+ " + " ".join(cmd))
    result = subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        check=False,
        capture_output=capture,
        text=True,
    )
    if check and result.returncode != 0:
        if result.stdout:
            print(result.stdout.rstrip(), file=sys.stderr)
        if result.stderr:
            print(result.stderr.rstrip(), file=sys.stderr)
        fail("command failed: " + " ".join(cmd))
    return result


def is_publishable(package: Package) -> bool:
    if package.publish is False or package.publish == []:
        return False
    if isinstance(package.publish, list) and "crates-io" not in package.publish:
        return False
    return True


def load_workspace(workspace: Path, toolchain: str | None) -> Workspace:
    result = run(
        cargo_command(toolchain) + ["metadata", "--format-version", "1", "--locked", "--no-deps"],
        cwd=workspace,
        capture=True,
    )
    metadata = json.loads(result.stdout)
    workspace_members = metadata["workspace_members"]
    packages_by_id = {package["id"]: package for package in metadata["packages"]}

    packages: dict[str, Package] = {}
    for package_id in workspace_members:
        payload = packages_by_id[package_id]
        package = Package(
            id=payload["id"],
            name=payload["name"],
            version=payload["version"],
            manifest_path=Path(payload["manifest_path"]),
            publish=payload.get("publish"),
            dependencies=payload.get("dependencies", []),
        )
        if package.name in packages:
            fail(f"duplicate workspace package name: {package.name}")
        packages[package.name] = package

    publishable = {
        name: package
        for name, package in packages.items()
        if is_publishable(package)
    }
    return Workspace(root=workspace, packages=packages, publishable=publishable)


def internal_dependencies(workspace: Workspace, package: Package) -> set[str]:
    dependencies: set[str] = set()
    for dependency in package.dependencies:
        dependency_name = dependency["name"]
        dependency_package = workspace.packages.get(dependency_name)
        if dependency_package is None:
            continue
        if not is_publishable(dependency_package):
            fail(
                f"publishable crate {package.name} depends on non-publishable "
                f"workspace crate {dependency_name}"
            )

        dependency_req = dependency.get("req", "").replace(" ", "")
        expected_req = f"={dependency_package.version}"
        if dependency_req != expected_req:
            fail(
                f"{package.name} depends on {dependency_name} with version requirement "
                f"{dependency_req!r}; expected exact requirement {expected_req!r}"
            )

        dependencies.add(dependency_name)
    return dependencies


def generated_publish_order(workspace: Workspace) -> list[str]:
    """Return a deterministic topological publish order from cargo metadata."""
    base_order = [name for name in workspace.packages if name in workspace.publishable]
    dependencies = {
        name: internal_dependencies(workspace, package)
        for name, package in workspace.publishable.items()
    }

    ordered: list[str] = []
    emitted: set[str] = set()
    remaining = set(base_order)
    while remaining:
        progressed = False
        for name in base_order:
            if name not in remaining:
                continue
            missing_dependencies = dependencies[name] - emitted
            if missing_dependencies:
                continue
            ordered.append(name)
            emitted.add(name)
            remaining.remove(name)
            progressed = True

        if not progressed:
            cycle_details = ", ".join(
                f"{name} -> {', '.join(sorted(dependencies[name] & remaining))}"
                for name in sorted(remaining)
            )
            fail("cyclic publishable Rust workspace dependencies: " + cycle_details)

    return ordered


def validate_order(
    *,
    workspace_path: Path,
    expected_version: str | None,
    toolchain: str | None,
) -> ValidatedOrder:
    workspace = load_workspace(workspace_path, toolchain)
    order = generated_publish_order(workspace)

    versions = {package.version for package in workspace.publishable.values()}
    if expected_version is not None:
        wrong_versions = sorted(
            f"{package.name}={package.version}"
            for package in workspace.publishable.values()
            if package.version != expected_version
        )
        if wrong_versions:
            fail(
                f"all publishable Rust crates must use release version {expected_version}; "
                + ", ".join(wrong_versions)
            )
        version = expected_version
    elif len(versions) > 1:
        fail(
            "all publishable Rust crates must share one version; found: "
            + ", ".join(sorted(versions))
        )
    else:
        version = next(iter(versions), None)

    print("Generated Rust publish order:")
    for index, name in enumerate(order, start=1):
        package = workspace.publishable[name]
        print(f"{index}. {package.name} v{package.version}")

    return ValidatedOrder(
        workspace=workspace,
        order=order,
        version=version,
    )


def write_output(key: str, value: str) -> None:
    output_file = os.environ.get("GITHUB_OUTPUT")
    if not output_file:
        return
    with open(output_file, "a", encoding="utf-8") as handle:
        handle.write(f"{key}={value}\n")


def write_json_output(key: str, value: Any) -> None:
    write_output(key, json.dumps(value, separators=(",", ":")))


def crate_version_exists(
    *,
    registry_url: str,
    name: str,
    version: str,
    attempts: int,
    retry_delay: float,
) -> bool:
    escaped_name = urllib.parse.quote(name, safe="")
    escaped_version = urllib.parse.quote(version, safe="")
    url = f"{registry_url.rstrip('/')}/api/v1/crates/{escaped_name}/{escaped_version}"
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})

    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                if response.status == 200:
                    return True
                fail(f"unexpected crates.io response {response.status} for {name} {version}")
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return False
            if error.code in {429, 500, 502, 503, 504} and attempt < attempts:
                print(
                    f"crates.io returned {error.code} for {name} {version}; "
                    f"retrying in {retry_delay:g}s"
                )
                time.sleep(retry_delay)
                continue
            body = error.read().decode("utf-8", errors="replace")
            fail(
                f"crates.io returned {error.code} while checking {name} {version}: "
                + body.strip()
            )
        except (TimeoutError, urllib.error.URLError, socket.timeout) as error:
            if attempt < attempts:
                print(
                    f"network error while checking {name} {version}: {error}; "
                    f"retrying in {retry_delay:g}s"
                )
                time.sleep(retry_delay)
                continue
            fail(f"network error while checking {name} {version}: {error}")

    return False


def plan_crates(
    order: ValidatedOrder,
    *,
    registry_url: str,
    status_attempts: int,
    status_retry_delay: float,
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    missing: list[dict[str, str]] = []
    existing: list[dict[str, str]] = []
    for name in order.order:
        package = order.workspace.publishable[name]
        item = {"name": package.name, "version": package.version}
        if crate_version_exists(
            registry_url=registry_url,
            name=package.name,
            version=package.version,
            attempts=status_attempts,
            retry_delay=status_retry_delay,
        ):
            existing.append(item)
            print(f"{package.name} v{package.version} is already published")
        else:
            missing.append(item)
            print(f"{package.name} v{package.version} needs publishing")

    return missing, existing


def write_plan_outputs(
    *,
    missing: list[dict[str, str]],
    existing: list[dict[str, str]],
    order: ValidatedOrder,
) -> None:
    ordered = [
        {"name": name, "version": order.workspace.publishable[name].version}
        for name in order.order
    ]
    all_published = not missing
    write_output("publish_required", "true" if missing else "false")
    write_output("all_published", "true" if all_published else "false")
    write_json_output("missing", missing)
    write_json_output("existing", existing)
    write_json_output("ordered", ordered)


def command_validate(args: argparse.Namespace) -> None:
    validate_order(
        workspace_path=args.workspace,
        expected_version=args.expected_version,
        toolchain=args.toolchain,
    )


def command_plan(args: argparse.Namespace) -> None:
    order = validate_order(
        workspace_path=args.workspace,
        expected_version=args.expected_version,
        toolchain=args.toolchain,
    )
    missing, existing = plan_crates(
        order,
        registry_url=args.registry_url,
        status_attempts=args.status_attempts,
        status_retry_delay=args.status_retry_delay,
    )
    write_plan_outputs(missing=missing, existing=existing, order=order)


def command_package(args: argparse.Namespace) -> None:
    order = validate_order(
        workspace_path=args.workspace,
        expected_version=args.expected_version,
        toolchain=args.toolchain,
    )
    list_package_contents(order, args.toolchain)
    package_order(
        order,
        args.toolchain,
        skip_internal_dependencies=args.skip_internal_dependencies,
    )


def list_package_contents(order: ValidatedOrder, toolchain: str | None) -> None:
    for name in order.order:
        result = run(
            cargo_command(toolchain) + ["package", "-p", name, "--locked", "--list"],
            cwd=order.workspace.root,
            capture=True,
        )
        package_file_count = len(result.stdout.splitlines())
        print(f"Package file list ok for {name}: {package_file_count} files")


def package_order(
    order: ValidatedOrder,
    toolchain: str | None,
    *,
    skip_internal_dependencies: bool,
) -> None:
    for name in order.order:
        package = order.workspace.publishable[name]
        dependencies = internal_dependencies(order.workspace, package)
        if skip_internal_dependencies and dependencies:
            print(
                f"Skipping cargo package -p {name}: internal workspace dependencies "
                "must be published to crates.io before Cargo can package this crate"
            )
            continue

        run(
            cargo_command(toolchain) + ["package", "-p", name, "--locked"],
            cwd=order.workspace.root,
        )


def command_ci(args: argparse.Namespace) -> None:
    order = validate_order(
        workspace_path=args.workspace,
        expected_version=args.expected_version,
        toolchain=args.toolchain,
    )
    list_package_contents(order, args.toolchain)
    package_order(order, args.toolchain, skip_internal_dependencies=True)
    run(
        cargo_command(args.toolchain)
        + [
            "nextest",
            "run",
            "--manifest-path",
            str(args.workspace / "Cargo.toml"),
            "--workspace",
            "--locked",
        ],
        cwd=REPO_ROOT,
    )


def wait_for_published_version(
    *,
    registry_url: str,
    name: str,
    version: str,
    timeout_seconds: float,
    poll_seconds: float,
    status_attempts: int,
    status_retry_delay: float,
) -> None:
    deadline = time.monotonic() + timeout_seconds
    while True:
        if crate_version_exists(
            registry_url=registry_url,
            name=name,
            version=version,
            attempts=status_attempts,
            retry_delay=status_retry_delay,
        ):
            print(f"{name} v{version} is visible on crates.io")
            return
        if time.monotonic() >= deadline:
            fail(f"timed out waiting for {name} v{version} to become visible on crates.io")
        print(f"waiting {poll_seconds:g}s for {name} v{version} to become visible")
        time.sleep(poll_seconds)


def command_publish(args: argparse.Namespace) -> None:
    order = validate_order(
        workspace_path=args.workspace,
        expected_version=args.expected_version,
        toolchain=args.toolchain,
    )
    missing, existing = plan_crates(
        order,
        registry_url=args.registry_url,
        status_attempts=args.status_attempts,
        status_retry_delay=args.status_retry_delay,
    )

    if missing:
        list_package_contents(order, args.toolchain)
        package_order(order, args.toolchain, skip_internal_dependencies=True)

    if args.dry_run:
        if missing:
            print("Dry run: would publish missing crates in this order:")
            for item in missing:
                print(f"- {item['name']} v{item['version']}")
        else:
            print("Dry run: no missing crates to publish")
        write_output("published_any", "false")
        write_json_output("published", [])
        write_plan_outputs(missing=missing, existing=existing, order=order)
        return

    if args.publish_disabled:
        if missing:
            print("RISE_RELEASE_PUBLISH_ENABLED is not true; skipping crates.io publish")
        else:
            print("No missing crates to publish")
        write_output("published_any", "false")
        write_json_output("published", [])
        write_plan_outputs(missing=missing, existing=existing, order=order)
        return

    if missing and not os.environ.get("CARGO_REGISTRY_TOKEN"):
        fail("CARGO_REGISTRY_TOKEN must be set for real crates.io publishing")

    published: list[dict[str, str]] = []
    env = os.environ.copy()
    for item in missing:
        name = item["name"]
        version = item["version"]
        if crate_version_exists(
            registry_url=args.registry_url,
            name=name,
            version=version,
            attempts=args.status_attempts,
            retry_delay=args.status_retry_delay,
        ):
            print(f"{name} v{version} appeared before publish; skipping")
            existing.append(item)
            continue

        for attempt in range(1, args.publish_attempts + 1):
            result = run(
                cargo_command(args.toolchain) + ["publish", "-p", name, "--locked"],
                cwd=args.workspace,
                env=env,
                check=False,
            )
            if result.returncode == 0:
                break

            if crate_version_exists(
                registry_url=args.registry_url,
                name=name,
                version=version,
                attempts=args.status_attempts,
                retry_delay=args.status_retry_delay,
            ):
                print(f"{name} v{version} is published despite cargo publish exit code")
                break

            if attempt == args.publish_attempts:
                fail(
                    f"cargo publish failed for {name} v{version} after "
                    f"{args.publish_attempts} attempts"
                )

            print(
                f"cargo publish failed for {name} v{version}; "
                f"retrying in {args.publish_retry_delay:g}s"
            )
            time.sleep(args.publish_retry_delay)

        wait_for_published_version(
            registry_url=args.registry_url,
            name=name,
            version=version,
            timeout_seconds=args.registry_timeout,
            poll_seconds=args.registry_poll,
            status_attempts=args.status_attempts,
            status_retry_delay=args.status_retry_delay,
        )
        published.append(item)
        existing.append(item)

    final_missing, final_existing = plan_crates(
        order,
        registry_url=args.registry_url,
        status_attempts=args.status_attempts,
        status_retry_delay=args.status_retry_delay,
    )
    write_output("published_any", "true" if published else "false")
    write_json_output("published", published)
    write_plan_outputs(missing=final_missing, existing=final_existing, order=order)


def add_common_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--workspace",
        type=Path,
        default=DEFAULT_WORKSPACE,
        help="Path to the Rust workspace",
    )
    parser.add_argument(
        "--expected-version",
        help="Expected shared version for all publishable Rust crates",
    )
    parser.add_argument(
        "--toolchain",
        help="Rust toolchain version to pass as cargo +<toolchain>",
    )


def add_registry_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--registry-url",
        default=DEFAULT_REGISTRY_URL,
        help="crates.io-compatible registry base URL",
    )
    parser.add_argument("--status-attempts", type=int, default=3)
    parser.add_argument("--status-retry-delay", type=float, default=5)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser("validate", help="Validate generated publish order")
    add_common_args(validate)
    validate.set_defaults(func=command_validate)

    ci = subparsers.add_parser("ci", help="Run Rust package and nextest checks")
    add_common_args(ci)
    ci.set_defaults(func=command_ci)

    package = subparsers.add_parser("package", help="Package publishable crates")
    add_common_args(package)
    package.add_argument(
        "--skip-internal-dependencies",
        action="store_true",
        help=(
            "Skip crates with publishable workspace dependencies that Cargo cannot "
            "package until those dependencies are published to crates.io"
        ),
    )
    package.set_defaults(func=command_package)

    plan = subparsers.add_parser("plan", help="Plan missing crates.io publishes")
    add_common_args(plan)
    add_registry_args(plan)
    plan.set_defaults(func=command_plan)

    publish = subparsers.add_parser("publish", help="Publish missing crates")
    add_common_args(publish)
    add_registry_args(publish)
    publish.add_argument("--dry-run", action="store_true")
    publish.add_argument("--publish-disabled", action="store_true")
    publish.add_argument("--publish-attempts", type=int, default=3)
    publish.add_argument("--publish-retry-delay", type=float, default=30)
    publish.add_argument("--registry-timeout", type=float, default=600)
    publish.add_argument("--registry-poll", type=float, default=15)
    publish.set_defaults(func=command_publish)

    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
