#!/usr/bin/env python3
"""Validate workspace crate edges against the versioned dependency policy."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read {path}: {error}") from error


def cargo_metadata(repo_root: Path) -> dict[str, Any]:
    process = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=repo_root,
        check=False,
        capture_output=True,
        text=True,
    )
    if process.returncode != 0:
        raise ValueError(f"cargo metadata failed:\n{process.stderr.rstrip()}")
    try:
        return json.loads(process.stdout)
    except json.JSONDecodeError as error:
        raise ValueError(f"cargo metadata returned invalid JSON: {error}") from error


def workspace_edges(metadata: dict[str, Any]) -> dict[str, set[str]]:
    packages = metadata.get("packages")
    workspace_members = metadata.get("workspace_members")
    if not isinstance(packages, list) or not isinstance(workspace_members, list):
        raise ValueError("metadata must contain packages and workspace_members arrays")

    workspace_ids = set(workspace_members)
    workspace_packages = {
        package["name"]: package
        for package in packages
        if package.get("id") in workspace_ids
    }
    workspace_names = set(workspace_packages)

    edges: dict[str, set[str]] = {}
    for name, package in workspace_packages.items():
        dependencies = package.get("dependencies", [])
        if not isinstance(dependencies, list):
            raise ValueError(f"package {name} has invalid dependencies metadata")
        edges[name] = {
            dependency["name"]
            for dependency in dependencies
            if dependency.get("name") in workspace_names
        }
    return edges


def validate_policy(
    edges: dict[str, set[str]], policy: dict[str, Any]
) -> list[str]:
    errors: list[str] = []
    workspace_names = set(edges)
    policy_names = set(policy)

    for name in sorted(workspace_names - policy_names):
        errors.append(f"workspace crate {name} is missing from the policy")
    for name in sorted(policy_names - workspace_names):
        errors.append(f"policy contains unknown workspace crate {name}")

    for name in sorted(workspace_names & policy_names):
        entry = policy[name]
        if not isinstance(entry, dict):
            errors.append(f"policy entry for {name} must be an object")
            continue
        allowed = entry.get("allowedDependencies")
        owner = entry.get("owns")
        if not isinstance(allowed, list) or not all(
            isinstance(dependency, str) for dependency in allowed
        ):
            errors.append(f"policy entry for {name} has invalid allowedDependencies")
            continue
        if not isinstance(owner, str) or not owner.strip():
            errors.append(f"policy entry for {name} must document ownership")

        allowed_set = set(allowed)
        for dependency in sorted(allowed_set - workspace_names):
            errors.append(f"{name} allows unknown workspace crate {dependency}")
        if name in allowed_set:
            errors.append(f"{name} cannot allow a dependency on itself")
        for dependency in sorted(edges[name] - allowed_set):
            errors.append(f"forbidden dependency: {name} -> {dependency}")

    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check internal Cargo dependencies against the workspace policy."
    )
    parser.add_argument(
        "--policy",
        type=Path,
        help="policy JSON path (defaults to config/crate-dependency-policy.json)",
    )
    parser.add_argument(
        "--metadata",
        type=Path,
        help="read Cargo metadata JSON from a file instead of invoking Cargo",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parent.parent
    policy_path = args.policy or repo_root / "config/crate-dependency-policy.json"

    try:
        policy = load_json(policy_path)
        if not isinstance(policy, dict):
            raise ValueError("dependency policy must be a JSON object")
        metadata = load_json(args.metadata) if args.metadata else cargo_metadata(repo_root)
        if not isinstance(metadata, dict):
            raise ValueError("Cargo metadata must be a JSON object")
        edges = workspace_edges(metadata)
        errors = validate_policy(edges, policy)
    except ValueError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 2

    print("Workspace crate dependencies:")
    for name, dependencies in sorted(edges.items()):
        rendered = ", ".join(sorted(dependencies)) if dependencies else "(none)"
        print(f"  {name}: {rendered}")

    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        print(
            f"Crate dependency check failed with {len(errors)} violation(s).",
            file=sys.stderr,
        )
        return 1

    print("Crate dependency check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
