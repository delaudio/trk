#!/usr/bin/env python3
"""Local Renoise demo parity harness.

Reimports local XRNS files into ignored Salieri fixtures and writes a JSON
summary that can be compared over time. Use --synthetic for CI-safe smoke runs
that do not require third-party Renoise demo assets.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT_DIR = REPO_ROOT / "fixtures" / "local" / "renoise-demos"
SYNTHETIC_XML = REPO_ROOT / "fixtures" / "xrns" / "parity-smoke-song.xml"


def main() -> int:
    args = parse_args()
    xrns_files, cleanup = resolve_inputs(args)
    try:
        if not xrns_files:
            print("No XRNS files found", file=sys.stderr)
            return 1

        args.output_dir.mkdir(parents=True, exist_ok=True)
        args.report.parent.mkdir(parents=True, exist_ok=True)
        report = {
            "source": "synthetic" if args.synthetic else "local",
            "output_dir": str(args.output_dir),
            "songs": [],
            "totals": {
                "songs": 0,
                "tracks": 0,
                "patterns": 0,
                "sequence_entries": 0,
                "samples": 0,
                "extracted_samples": 0,
                "unsupported_devices": 0,
                "unsupported_phrases": 0,
                "unsupported_effect_commands": 0,
                "dropped_extra_effect_columns": 0,
                "errors": 0,
            },
        }

        for xrns_file in xrns_files:
            song_report = import_song(args, xrns_file)
            report["songs"].append(song_report)
            add_totals(report["totals"], song_report)

        report["totals"]["songs"] = len(report["songs"])
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(f"Wrote Renoise parity report: {args.report}")
        print(format_summary(report))
        return 1 if report["totals"]["errors"] else 0
    finally:
        cleanup()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="*", type=Path, help="XRNS files or directories")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help="ignored output directory for imported .salieri files",
    )
    parser.add_argument(
        "--report",
        type=Path,
        default=DEFAULT_OUTPUT_DIR / "parity-report.json",
        help="JSON summary report path",
    )
    parser.add_argument(
        "--salieri",
        default=os.environ.get("SALIERI_BIN", "cargo run -q -p salieri-app --"),
        help="Salieri command prefix; defaults to cargo run",
    )
    parser.add_argument(
        "--convert-samples-to-wav",
        action="store_true",
        help="pass through to salieri import xrns",
    )
    parser.add_argument(
        "--synthetic",
        action="store_true",
        help="build and import a committed synthetic XRNS fixture",
    )
    return parser.parse_args()


def resolve_inputs(args: argparse.Namespace) -> tuple[list[Path], callable]:
    if args.synthetic:
        temp_dir = Path(tempfile.mkdtemp(prefix="salieri-xrns-parity-"))
        xrns_path = temp_dir / "parity-smoke-song.xrns"
        with zipfile.ZipFile(xrns_path, "w", compression=zipfile.ZIP_STORED) as archive:
            archive.write(SYNTHETIC_XML, "Song.xml")
            archive.writestr("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE")
        return [xrns_path], lambda: shutil.rmtree(temp_dir, ignore_errors=True)

    paths = args.inputs or [Path(os.environ.get("RENOISE_DEMO_DIR", ""))]
    xrns_files: list[Path] = []
    for path in paths:
        if not str(path):
            continue
        path = path.expanduser()
        if path.is_dir():
            xrns_files.extend(sorted(path.rglob("*.xrns")))
        elif path.suffix.lower() == ".xrns":
            xrns_files.append(path)
    return xrns_files, lambda: None


def import_song(args: argparse.Namespace, xrns_file: Path) -> dict:
    slug = slugify(xrns_file.stem)
    output_path = args.output_dir / f"{slug}.salieri"
    sample_dir = args.output_dir / "samples" / slug
    command = [
        *shlex.split(args.salieri),
        "import",
        "xrns",
        str(xrns_file),
        str(output_path),
        "--sample-dir",
        str(sample_dir),
        "--sample-path-prefix",
        f"samples/{slug}",
    ]
    if args.convert_samples_to_wav:
        command.append("--convert-samples-to-wav")

    completed = subprocess.run(
        command,
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    metrics = parse_import_output(completed.stdout, completed.stderr)
    metrics.update(
        {
            "name": xrns_file.stem,
            "input": str(xrns_file),
            "output": str(output_path),
            "sample_dir": str(sample_dir),
            "status": "ok" if completed.returncode == 0 else "failed",
            "exit_code": completed.returncode,
            "diagnostics": diagnostic_lines(completed.stderr),
        }
    )
    if completed.returncode == 0 and output_path.exists():
        metrics.update(project_metrics(output_path))
    return metrics


def parse_import_output(stdout: str, stderr: str) -> dict:
    metrics = {
        "tracks": 0,
        "patterns": 0,
        "sequence_entries": 0,
        "samples": 0,
        "extracted_samples": 0,
        "unsupported_devices": 0,
        "unsupported_phrases": 0,
        "unsupported_effect_commands": 0,
        "dropped_extra_effect_columns": 0,
        "errors": 0,
    }
    match = re.search(
        r": (?P<tracks>\d+) tracks, (?P<patterns>\d+) patterns, "
        r"(?P<sequence>\d+) sequence entries, (?P<samples>\d+) samples, "
        r"(?P<extracted>\d+) extracted sample files",
        stdout,
    )
    if match:
        metrics["tracks"] = int(match.group("tracks"))
        metrics["patterns"] = int(match.group("patterns"))
        metrics["sequence_entries"] = int(match.group("sequence"))
        metrics["samples"] = int(match.group("samples"))
        metrics["extracted_samples"] = int(match.group("extracted"))

    diagnostics = diagnostic_lines(stderr)
    metrics["unsupported_devices"] = count_contains(diagnostics, "unsupported Renoise device")
    metrics["unsupported_phrases"] = count_contains(diagnostics, "phrase")
    metrics["unsupported_effect_commands"] = count_contains(
        diagnostics, "effect command"
    )
    metrics["dropped_extra_effect_columns"] = count_contains(
        diagnostics, "extra XRNS effect column was dropped"
    )
    metrics["errors"] = sum(1 for line in diagnostics if "Error" in line)
    return metrics


def project_metrics(path: Path) -> dict:
    project = json.loads(path.read_text())
    song = project.get("song", project)
    return {
        "tracks": len(song.get("tracks", [])),
        "patterns": len(song.get("patterns", [])),
        "sequence_entries": len(song.get("sequence", [])),
        "samples": len(song.get("samples", [])),
    }


def diagnostic_lines(stderr: str) -> list[str]:
    return [line for line in stderr.splitlines() if line.startswith("XRNS ")]


def add_totals(totals: dict, song: dict) -> None:
    for key in totals:
        if key != "songs":
            totals[key] += int(song.get(key, 0))


def count_contains(lines: list[str], needle: str) -> int:
    return sum(1 for line in lines if needle in line)


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-zA-Z0-9._-]+", "-", value.strip().lower()).strip("-")
    return slug or "renoise-song"


def format_summary(report: dict) -> str:
    totals = report["totals"]
    return (
        f"{totals['songs']} song(s), {totals['tracks']} tracks, "
        f"{totals['patterns']} patterns, {totals['sequence_entries']} sequence entries, "
        f"{totals['samples']} samples, {totals['extracted_samples']} extracted samples, "
        f"{totals['unsupported_devices']} unsupported devices, "
        f"{totals['unsupported_phrases']} unsupported phrases, "
        f"{totals['unsupported_effect_commands']} unsupported effect commands"
    )


if __name__ == "__main__":
    raise SystemExit(main())
