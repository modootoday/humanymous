"""Cargo manifest acquisition and complete standard-user harness accounting."""
import hashlib
import json
import re
from pathlib import Path
import subprocess

SUMMARY = re.compile(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")
KEY_HELPERS = {
    "child_lock_probe": "competing_process_cannot_load_or_create",
    "child_interrupted_writer": "process_exit_releases_lock_and_rotation_pending_requires_recovery",
}


def helper_count(entry, log, summaries, discovered):
    """Recognize only source-bound custody subprocesses, never general skips."""
    if (entry.get("target") != "key_custody" or entry.get("kind") != ["test"]
            or "#humanymous-audit@" not in entry.get("package_id", "")):
        return 0
    ignored = re.findall(r"^test (.+?) \.\.\. ignored(?:,.*)?$", log, re.MULTILINE)
    if set(ignored) != set(KEY_HELPERS) or len(ignored) != len(KEY_HELPERS):
        raise RuntimeError("custody helper inventory mismatch")
    for parent in KEY_HELPERS.values():
        if not re.search(r"^test " + re.escape(parent) + r" \.\.\. ok$", log, re.MULTILINE):
            raise RuntimeError("custody helper parent missing")
    if not re.search(r"^test child_lock_probe \.\.\. ok$", log, re.MULTILINE):
        raise RuntimeError("custody lock subprocess proof missing")
    if len(summaries) != 2 or tuple(map(int, summaries[0])) != (1, 0, 0, 0, discovered - 1):
        raise RuntimeError("unexpected custody subprocess summary")
    return len(KEY_HELPERS)


def manifest_from_messages(messages):
    found = {}
    for line in messages.splitlines():
        item = json.loads(line)
        if item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test") and item.get("executable"):
            path = str(Path(item["executable"]).resolve())
            found[path] = {"executable": path, "package_id": item["package_id"], "target": item["target"]["name"], "kind": item["target"]["kind"]}
    if not found:
        raise RuntimeError("Cargo produced no test executables")
    return list(found.values())


def verify_results(manifest, result, directory):
    rows = result["harnesses"]
    if len(rows) != len(manifest) or [r["executable"] for r in rows] != [m["executable"] for m in manifest]:
        raise RuntimeError("required manifest coverage mismatch")
    total = zero = ignored_total = 0
    for index, row in enumerate(rows):
        listing = (directory / f"harness-{index}-list.log").read_text(encoding="utf-8-sig")
        log = (directory / f"harness-{index}-run.log").read_text(encoding="utf-8-sig")
        discovered = len(re.findall(r"^.+: test$", listing, re.MULTILINE))
        summaries = SUMMARY.findall(log)
        helpers = helper_count(manifest[index], log, summaries, discovered)
        if row["list_exit"] != 0 or row["run_exit"] != 0 or len(summaries) != (2 if helpers else 1):
            raise RuntimeError(f"harness {index} failed enumeration/execution/summary")
        passed, failed, ignored, measured, filtered = map(int, summaries[-1])
        if failed or ignored != helpers or measured or filtered or passed + helpers != discovered:
            raise RuntimeError(f"harness {index} incomplete: discovered={discovered}, summary={summaries[0]}")
        row.update(discovered=discovered, passed=passed, ignored=ignored, filtered=filtered, zero_test_harness=discovered == 0)
        if helpers:
            row.update(subprocess_helpers=KEY_HELPERS, required_parent_tests=discovered-helpers)
        ignored_total += ignored
        total += passed
        zero += discovered == 0
    if total == 0:
        raise RuntimeError("empty workspace cannot qualify")
    result.update(required_harnesses=len(manifest), executed_harnesses=len(rows), zero_test_harnesses=zero, passed_tests=total, ignored_tests=ignored_total, ignored_helpers=ignored_total)
    return result


def execute(directory):
    directory = directory.resolve()
    with (directory / "cargo-build.jsonl").open("w", encoding="utf-8") as output, (directory / "cargo-build.log").open("w", encoding="utf-8") as errors:
        build = subprocess.run(["cargo", "test", "--workspace", "--all-targets", "--locked", "--no-run", "--message-format=json"], stdout=output, stderr=errors)
    if build.returncode:
        raise RuntimeError("Cargo test executable compilation failed")
    manifest = manifest_from_messages((directory / "cargo-build.jsonl").read_text(encoding="utf-8"))
    for entry in manifest:
        entry["sha256"] = hashlib.sha256(Path(entry["executable"]).read_bytes()).hexdigest()
    manifest_path = directory / "test-executables.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    launcher = Path(__file__).with_name("windows-launch.ps1").resolve()
    process = subprocess.run(["pwsh", "-NoProfile", "-NonInteractive", "-File", str(launcher), "-Manifest", str(manifest_path), "-ReportDirectory", str(directory), "-Workspace", str(Path.cwd())])
    if process.returncode:
        raise RuntimeError("standard-user launcher failed; inspect launcher-result.json and harness logs")
    result = json.loads((directory / "launcher-result.json").read_text(encoding="utf-8-sig"))
    result = verify_results(manifest, result, directory)
    (directory / "standard-user-coverage.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    return result
