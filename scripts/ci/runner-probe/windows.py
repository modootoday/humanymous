"""Cargo manifest acquisition and complete standard-user harness accounting."""
import hashlib
import json
import re
from pathlib import Path
import subprocess

SUMMARY = re.compile(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")


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
    total = zero = 0
    for index, row in enumerate(rows):
        listing = (directory / f"harness-{index}-list.log").read_text(encoding="utf-8-sig")
        log = (directory / f"harness-{index}-run.log").read_text(encoding="utf-8-sig")
        discovered = len(re.findall(r"^.+: test$", listing, re.MULTILINE))
        summaries = SUMMARY.findall(log)
        if row["list_exit"] != 0 or row["run_exit"] != 0 or len(summaries) != 1:
            raise RuntimeError(f"harness {index} failed enumeration/execution/summary")
        passed, failed, ignored, measured, filtered = map(int, summaries[0])
        if failed or ignored or measured or filtered or passed != discovered:
            raise RuntimeError(f"harness {index} incomplete: discovered={discovered}, summary={summaries[0]}")
        row.update(discovered=discovered, passed=passed, ignored=ignored, filtered=filtered, zero_test_harness=discovered == 0)
        total += passed
        zero += discovered == 0
    if total == 0:
        raise RuntimeError("empty workspace cannot qualify")
    result.update(required_harnesses=len(manifest), executed_harnesses=len(rows), zero_test_harnesses=zero, passed_tests=total, ignored_tests=0)
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
