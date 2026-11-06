"""Control-plane CVE/package scans, using only the recorded immutable image IDs.

Scanner sees one exported image, never Docker API, credentials or host source.
Scanner downloads are separate from isolated product test traffic.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from run import DOCKER, ROOT

SCANNER = "aquasec/trivy@sha256:8f6bb947dbc528902e11f2f5be52f7b519f6a39d5d148f41166f2040499768fb"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    output = args.evidence.resolve()
    output.relative_to(ROOT / ".spec.local/root/evidence")
    if not output.joinpath("index.evidence.md").is_file():
        parser.error("local Specfold bundle required")
    summary = {"scanner": SCANNER, "scans": [], "clean": True}
    # Temporary directories are removed after scanner exit; no global prune.
    with tempfile.TemporaryDirectory(prefix="hm-rust-lab-audit-") as temporary:
        cache = Path(temporary) / "cache"
        cache.mkdir()
        for name in ("fixture", "authority"):
            provenance = output / f"{name}-image.json"
            if not provenance.exists():
                summary["clean"] = False
                summary["scans"].append({"image": name, "status": "not_run", "reason": "image provenance missing"})
                continue
            image = json.loads(provenance.read_text(encoding="utf-8"))
            image_id = image["Id"]
            if not image_id.startswith("sha256:") or len(image_id) != 71:
                raise ValueError("invalid recorded image id")
            archive = Path(temporary) / f"{name}.tar"
            subprocess.run([DOCKER, "image", "save", image_id, "-o", str(archive)], cwd=ROOT, check=True, timeout=120)
            with archive.open("rb") as exported:
                archive_sha = hashlib.file_digest(exported, "sha256").hexdigest()
            result = subprocess.run([DOCKER, "run", "--rm", "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--tmpfs", "/tmp:rw,noexec,nosuid,size=512m", "--mount", f"type=bind,source={cache},target=/root/.cache", "--mount", f"type=bind,source={archive},target=/scan/image.tar,readonly", SCANNER, "image", "--input", "/scan/image.tar", "--scanners", "vuln", "--format", "json", "--quiet"], capture_output=True, text=True, encoding="utf-8", timeout=600)
            output.joinpath(f"{name}-scan-stderr.txt").write_text(result.stderr, encoding="utf-8")
            if result.returncode:
                summary["clean"] = False
                summary["scans"].append({"image": name, "imageId": image_id, "status": "not_run", "exit": result.returncode})
                continue
            report = json.loads(result.stdout)
            output.joinpath(f"{name}-trivy-final.json").write_text(result.stdout, encoding="utf-8")
            findings = [finding for target in report.get("Results", []) for finding in target.get("Vulnerabilities", [])]
            summary["scans"].append({"image": name, "imageId": image_id, "exportArchiveSha256": archive_sha, "scannedConfigId": report["Metadata"]["ImageID"], "scannedAt": report.get("CreatedAt"), "status": "findings" if findings else "passed", "findings": [{k: f.get(k) for k in ("VulnerabilityID", "PkgName", "InstalledVersion", "FixedVersion", "Severity")} for f in findings]})
            if findings:
                summary["clean"] = False
        db_metadata = cache / "trivy/db/metadata.json"
        if db_metadata.is_file():
            database = json.loads(db_metadata.read_text(encoding="utf-8"))
            output.joinpath("trivy-db-metadata.json").write_text(json.dumps(database, indent=2), encoding="utf-8")
            summary["database"] = database
        else:
            summary["clean"] = False
    lock = ROOT / "Cargo.lock"
    cargo = subprocess.run(["cargo", "audit", "--json", "--file", str(lock)], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=180)
    output.joinpath("cargo-audit.json").write_text(cargo.stdout, encoding="utf-8")
    output.joinpath("cargo-audit-stderr.txt").write_text(cargo.stderr, encoding="utf-8")
    summary["cargo"] = {"exit": cargo.returncode, "lockSha256": hashlib.sha256(lock.read_bytes()).hexdigest()}
    source = json.loads(output.joinpath("source-provenance.json").read_text(encoding="utf-8"))
    expected_lock = next(f["sha256"] for f in source["files"] if f["path"] == "Cargo.lock")
    summary["cargo"]["matchesBuiltLock"] = expected_lock == summary["cargo"]["lockSha256"]
    if not summary["cargo"]["matchesBuiltLock"]:
        summary["clean"] = False
    if cargo.returncode:
        summary["clean"] = False
    npm = subprocess.run([DOCKER, "run", "--rm", "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--tmpfs", "/tmp:rw,noexec,nosuid,size=32m", "--mount", f"type=bind,source={ROOT / 'deployments/lab/authority'},target=/work,readonly", "--workdir", "/work", "node:26-alpine@sha256:8996bb5821d62a8fd9bb27c1cb07462f3fdce6e47d640c6a881ead0b064ce216", "npm", "audit", "--package-lock-only", "--omit=dev", "--json", "--cache", "/tmp/npm-cache"], capture_output=True, text=True, encoding="utf-8", timeout=120)
    output.joinpath("npm-audit.json").write_text(npm.stdout, encoding="utf-8")
    output.joinpath("npm-audit-stderr.txt").write_text(npm.stderr, encoding="utf-8")
    npm_lock = ROOT / "deployments/lab/authority/package-lock.json"
    expected_npm = next(f["sha256"] for f in source["files"] if f["path"] == "deployments/lab/authority/package-lock.json")
    summary["npm"] = {"exit": npm.returncode, "matchesBuiltLock": expected_npm == hashlib.sha256(npm_lock.read_bytes()).hexdigest()}
    if npm.returncode or not summary["npm"]["matchesBuiltLock"]:
        summary["clean"] = False
    output.joinpath("audit-summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0 if summary["clean"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
