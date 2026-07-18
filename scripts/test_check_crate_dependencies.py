#!/usr/bin/env python3

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check_crate_dependencies.py")


def metadata(edges: dict[str, list[str]]) -> dict[str, object]:
    packages = []
    members = []
    for name, dependencies in edges.items():
        package_id = f"path+file:///fixture/{name}#0.1.0"
        members.append(package_id)
        packages.append(
            {
                "id": package_id,
                "name": name,
                "dependencies": [
                    {"name": dependency, "path": f"/fixture/{dependency}"}
                    for dependency in dependencies
                ],
            }
        )
    return {"workspace_members": members, "packages": packages}


class CrateDependencyCheckerTests(unittest.TestCase):
    def run_checker(
        self, edges: dict[str, list[str]], policy: dict[str, object]
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metadata_path = root / "metadata.json"
            policy_path = root / "policy.json"
            metadata_path.write_text(json.dumps(metadata(edges)), encoding="utf-8")
            policy_path.write_text(json.dumps(policy), encoding="utf-8")
            return subprocess.run(
                [
                    "python3",
                    str(SCRIPT),
                    "--metadata",
                    str(metadata_path),
                    "--policy",
                    str(policy_path),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

    def test_accepts_documented_dependency(self) -> None:
        result = self.run_checker(
            {"core": [], "app": ["core"]},
            {
                "core": {"allowedDependencies": [], "owns": "model"},
                "app": {"allowedDependencies": ["core"], "owns": "runtime"},
            },
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_rejects_forbidden_dependency(self) -> None:
        result = self.run_checker(
            {"core": ["app"], "app": []},
            {
                "core": {"allowedDependencies": [], "owns": "model"},
                "app": {"allowedDependencies": ["core"], "owns": "runtime"},
            },
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("forbidden dependency: core -> app", result.stderr)

    def test_requires_policy_for_every_workspace_crate(self) -> None:
        result = self.run_checker(
            {"core": [], "app": []},
            {"core": {"allowedDependencies": [], "owns": "model"}},
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("workspace crate app is missing from the policy", result.stderr)


if __name__ == "__main__":
    unittest.main()
