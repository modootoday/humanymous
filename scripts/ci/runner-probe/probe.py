"""Native observation and current-crate test evidence; never full runtime qualification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time


def command(*args):
    return subprocess.check_output(args, text=True, encoding="utf-8").strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--os", required=True, choices=["linux", "windows", "macos"])
    parser.add_argument("--arch", required=True, choices=["x64", "arm64"])
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    observed_os = {"Linux": "linux", "Windows": "windows", "Darwin": "macos"}.get(platform.system())
    observed_arch = {"amd64": "x64", "x86_64": "x64", "arm64": "arm64", "aarch64": "arm64"}.get(platform.machine().lower())
    report = {
        "schema": "humanymous.runner-probe/1", "scope": "implemented-crate-native-tests",
        "full_runtime_qualified": False, "status": "failed", "started_unix": time.time(),
        "expected": {"os": args.os, "arch": args.arch},
        "observed": {"os": observed_os, "arch": observed_arch, "release": platform.release(),
                     "version": platform.version(), "machine": platform.machine()},
        "runner": {key: os.environ.get(key) for key in ["RUNNER_OS", "RUNNER_ARCH", "RUNNER_ENVIRONMENT", "ImageOS", "ImageVersion"]},
        "github": {key: os.environ.get(key) for key in ["GITHUB_REPOSITORY", "GITHUB_SHA", "GITHUB_WORKFLOW_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_JOB"]},
    }
    try:
        if (observed_os, observed_arch) != (args.os, args.arch):
            raise RuntimeError("observed native OS/architecture differs from required tuple")
        report["source_sha"] = command("git", "rev-parse", "HEAD")
        report["worktree_dirty"] = bool(command("git", "status", "--porcelain"))
        if os.environ.get("GITHUB_ACTIONS") == "true" and report["worktree_dirty"]:
            raise RuntimeError("remote probe requires a clean checkout")
        if os.environ.get("GITHUB_SHA") and report["source_sha"] != os.environ["GITHUB_SHA"]:
            raise RuntimeError("checkout SHA differs from workflow SHA")
        report["cargo_lock_sha256"] = hashlib.sha256(Path("Cargo.lock").read_bytes()).hexdigest()
        report["rustc"] = command("rustc", "-vV")
        report["cargo"] = command("cargo", "--version")
        if args.os == "windows":
            from windows import execute
            report["windows_standard_user"] = execute(args.report.parent)
            report["status"] = "passed"
            return_code = 0
        else:
            return_code = run_cargo(args.report.parent, report)
        if return_code:
            raise RuntimeError("implemented workspace tests failed")
        report["status"] = "passed"
    except Exception as error:
        report["error"] = str(error)
    finally:
        report["finished_unix"] = time.time()
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "passed" else 1


def run_cargo(directory, report):
        log = directory / "cargo-tests.log"
        with log.open("w", encoding="utf-8") as stream:
            result = subprocess.run(["cargo", "test", "--workspace", "--all-targets", "--locked"], stdout=stream, stderr=subprocess.STDOUT)
        report["test_exit_code"] = result.returncode
        report["test_log_sha256"] = hashlib.sha256(log.read_bytes()).hexdigest()
        if result.returncode:
            print(log.read_text(encoding="utf-8"), file=sys.stderr)
        return result.returncode


if __name__ == "__main__":
    sys.exit(main())
