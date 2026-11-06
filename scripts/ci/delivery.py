"""Fail-closed Rust delivery gates. No command mutates authored documentation."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import subprocess
import sys
import tempfile
import tarfile
import tomllib
import urllib.error
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
CASES = {
    "native": {"startup", "configuration-invalid", "doctor", "install-dry-run-no-mutation",
               "local-only-admin", "restart-replay", "storage-failure-closed", "documentation-examples"},
    "browser": {"collector-parity", "challenge-keyboard", "challenge-assistive",
                "missing-signals-closed", "human-control", "coherent-browser-ceiling", "documentation-examples"},
    "docker": {"attack-catalog", "gate", "pass", "swarm", "deployment",
               "human-control", "raw-tls-h2", "audit-restart", "origin-isolation", "documentation-examples"},
}
RUNNERS = {"native": "scripts/migration/pilot-native.ps1",
           "browser": "scripts/migration/pilot-browser.sh",
           "docker": "scripts/migration/e2e-rust-docker.sh"}
FORBIDDEN = {".spec", ".spec.local", ".agents", ".codex", ".claude", ".git", "node_modules", "test", "tests", "fixtures",
             "bots", "redteam", "wargame", "target"}
BINARIES = {"humanymous", "humanymous-server", "humanymous-gate"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*args, cwd=ROOT, capture=False, env=None):
    result = subprocess.run(args, cwd=cwd, text=True, encoding="utf-8",
                            stdout=subprocess.PIPE if capture else None,
                            check=True, env=env)
    return result.stdout if capture else None


def metadata():
    return json.loads(run("cargo", "metadata", "--locked", "--no-deps",
                          "--format-version", "1", capture=True))


def release_packages(data):
    members = set(data["workspace_members"])
    packages = [p for p in data["packages"] if p["id"] in members and p["publish"] != []]
    require(packages, "no publishable Rust packages: publish=false remains a release blocker")
    by_name = {p["name"]: p for p in packages}
    ordered, visiting, visited = [], set(), set()

    def visit(package):
        name = package["name"]
        require(name not in visiting, f"registry dependency cycle: {name}")
        if name in visited:
            return
        visiting.add(name)
        for dep in package["dependencies"]:
            if dep.get("path"):
                require(dep["name"] in by_name, f"{name}: unpublished required dependency {dep['name']}")
                require(dep["req"] != "*", f"{name}: path dependency lacks registry version")
                visit(by_name[dep["name"]])
        visiting.remove(name)
        visited.add(name)
        ordered.append(package)

    for package in sorted(packages, key=lambda p: p["name"]):
        visit(package)
    return ordered


def safe_files(files, *, rust=False):
    require(files, "empty package surface")
    for name in files:
        path = PurePosixPath(name.replace("\\", "/"))
        require(not path.is_absolute() and not PureWindowsPath(name).drive and ".." not in path.parts,
                f"unsafe package path: {name}")
        # Rust unit tests and fixtures can be needed to verify the archive. The
        # defense lab/private outputs may never ship. Node excludes test assets.
        forbidden = FORBIDDEN - ({"test", "tests", "fixtures"} if rust else set())
        require(not forbidden.intersection(path.parts), f"private/lab package content: {name}")
        require(not name.endswith((".pem", ".key", ".env")), f"credential-like package content: {name}")


def validate_report(report, kind, sha, review_sha):
    require(report.get("schema") == 1, "pilot schema must be 1")
    require(report.get("source_sha") == sha, "stale/mismatched pilot SHA")
    require(report.get("documentation_review_sha256") == review_sha, "pilot did not exercise current authored documentation examples")
    require(report.get("kind") == kind and report.get("status") == "passed", "pilot did not pass")
    require(report.get("errors") == 0 and report.get("skips") == 0, "pilot errors/skips are not success")
    cases = report.get("cases", [])
    require(isinstance(cases, list) and cases, "missing pilot cases")
    ids = [case.get("id") for case in cases]
    require(all(isinstance(i, str) and i for i in ids), "invalid pilot case ID")
    require(len(set(ids)) == len(ids), "duplicate pilot case ID")
    require(CASES[kind].issubset(ids), f"missing required {kind} cases")
    require(all(case.get("status") == "passed" for case in cases), "non-passing pilot case")
    if kind == "docker":
        require(report.get("isolation") == "docker" and report.get("runtime") == "rust",
                "only isolated Rust Docker evidence is authoritative")
        images = report.get("images", [])
        require(isinstance(images, list) and images and
                all(re.fullmatch(r"sha256:[0-9a-f]{64}", x) for x in images),
                "Docker report lacks immutable image identities")


def pilot(kind):
    runner = ROOT / RUNNERS[kind]
    require(runner.is_file(), f"unimplemented required Rust {kind} runner: {runner.relative_to(ROOT)}")
    if kind == "docker":
        require(sys.platform == "linux", "Docker authority runs only on Linux")
        require(run("docker", "info", "--format", "{{.OSType}}", capture=True).strip() == "linux",
                "Linux Docker engine required")
    sha = run("git", "rev-parse", "HEAD", capture=True).strip()
    review_sha = hashlib.sha256((ROOT / "docs/reviews.json").read_bytes()).hexdigest()
    # Unique new directory prevents stale committed report acceptance.
    destination = Path(os.environ.get("RUNNER_TEMP", tempfile.gettempdir()))
    with tempfile.TemporaryDirectory(prefix=f"rust-{kind}-", dir=destination) as temp:
        report = Path(temp) / "report.json"
        args = (["pwsh", "-NoProfile", "-File", str(runner), "-Report"] if kind == "native"
                else ["bash", str(runner), "--report"])
        run(*args, str(report))
        require(report.is_file(), f"{kind} runner produced no execution report")
        result = json.loads(report.read_text(encoding="utf-8-sig"))
        validate_report(result, kind, sha, review_sha)
        out = ROOT / "target" / "delivery-evidence"
        out.mkdir(parents=True, exist_ok=True)
        (out / f"{kind}-{sys.platform}.json").write_text(json.dumps(result, indent=2), encoding="utf-8")


def readiness():
    data = metadata()
    packages = release_packages(data)
    binaries = {t["name"] for p in data["packages"] for t in p["targets"] if "bin" in t["kind"]}
    require(BINARIES.issubset(binaries), f"missing shipped Rust binaries: {sorted(BINARIES - binaries)}")
    for runner in RUNNERS.values():
        require((ROOT / runner).is_file(), f"missing runtime prerequisite: {runner}")
    migration_acceptance()
    image_sources()
    for package in packages:
        for field in ("description", "license", "repository", "readme"):
            require(package.get(field), f"{package['name']}: missing publication metadata {field}")
    prerequisites = json.loads((ROOT / "scripts/ci/registry-prerequisites.json").read_text())
    require(prerequisites.get("schema") == 1, "registry prerequisites schema mismatch")
    for registry in ("crates_io", "npm"):
        record = prerequisites[registry]
        require(record.get("verified") is True and len(record.get("evidence", "")) > 20,
                f"external {registry} ownership/bootstrap/trust is unverified")
        require(record.get("workflow") == "release.yml" and record.get("environment") == "release",
                f"incorrect {registry} trusted publisher binding")


def migration_acceptance():
    inventory = ROOT / ".spec/evidence/20261106194112-go-migration-source-inventory/go-inventory.json"
    data = json.loads(inventory.read_text(encoding="utf-8"))
    require(data.get("baseline") == "981d25cf71a5a54855d7e2b4359e832a9d5998e5", "migration oracle identity changed")
    files = data.get("files", [])
    require(files and len({row["path"] for row in files}) == len(files), "empty/duplicate migration inventory")
    baseline_paths = {path for path in run("git", "ls-tree", "-r", "--name-only", data["baseline"],
                                          "--", "cmd", "internal", "pkg", capture=True).splitlines()
                      if path.endswith(".go")}
    require(baseline_paths == {row["path"] for row in files}, "migration inventory dropped/added baseline responsibilities")
    for row in files:
        state = row.get("parity_status")
        require(state in {"verified_runtime_parity", "retired_with_review"},
                f"incomplete source/test migration: {row['path']} ({state})")
        evidence = row.get("parity_evidence", [])
        require(evidence and isinstance(evidence, list), f"missing canonical parity evidence: {row['path']}")
        for path in evidence:
            candidate = PurePosixPath(path)
            require(path.startswith(".spec/evidence/") and ".." not in candidate.parts and
                    (ROOT / path).is_file(), f"missing/unsafe CI-visible parity evidence: {path}")
        if state == "verified_runtime_parity":
            paths = row.get("rust_paths", [])
            require(paths and all(path.startswith("crates/") and ".." not in PurePosixPath(path).parts and
                                  (ROOT / path).exists() for path in paths),
                    f"missing Rust responsibility mapping: {row['path']}")
        else:
            require(len(row.get("disposition", "").strip()) >= 30, f"unreviewed retirement: {row['path']}")
    tracked = run("git", "ls-files", "--", "*.go", "go.mod", "go.sum", capture=True).splitlines()
    require(not tracked, "final Rust shipping tree still contains Go product/migration source; complete reviewed cutover cleanup")


def image_sources():
    for dockerfile in ("build/core.Dockerfile", "build/gate.Dockerfile"):
        source = (ROOT / dockerfile).read_text(encoding="utf-8")
        require(not re.search(r"(?im)^\s*FROM\s+\S*golang|\bgo\s+build\b", source),
                f"Go shipping path still present: {dockerfile}")
        require(re.search(r"\bcargo\s+(?:build|chef)\b", source), f"unverified Rust image: {dockerfile}")


def native_artifacts():
    import platform
    source_sha = run("git", "rev-parse", "HEAD", capture=True).strip()
    out = ROOT / "target/delivery-artifacts"
    out.mkdir(parents=True, exist_ok=True)
    files = [ROOT / "target/release" / (name + (".exe" if os.name == "nt" else "")) for name in sorted(BINARIES)]
    files.append(ROOT / "LICENSE")
    require(all(path.is_file() for path in files), "missing required native release binary")
    stem = f"humanymous-{sys.platform}-{platform.machine().lower()}-{source_sha}"
    archive = out / (stem + (".zip" if os.name == "nt" else ".tar.gz"))
    if os.name == "nt":
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in files:
                bundle.write(path, path.name)
    else:
        with tarfile.open(archive, "w:gz") as bundle:
            for path in files:
                bundle.add(path, arcname=path.name)
    (out / f"{stem}.sha256").write_text(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n", encoding="utf-8")


def package_check():
    packages = release_packages(metadata())
    for package in packages:
        files = run("cargo", "package", "--locked", "--list", "-p", package["name"], capture=True).splitlines()
        safe_files(files, rust=True)
    # Package interdependent crates together: Cargo's temporary registry can
    # resolve the same wave before these immutable versions exist on crates.io.
    args = [arg for package in packages for arg in ("-p", package["name"])]
    run("cargo", "package", "--locked", *args)


def node_check():
    folder = ROOT / "adapters/node"
    require((folder / "package-lock.json").is_file(), "Node adapter locked package is unimplemented")
    npm = "npm.cmd" if os.name == "nt" else "npm"
    run(npm, "ci", cwd=folder)
    run(npm, "run", "typecheck", cwd=folder)
    run(npm, "test", cwd=folder)
    run(npm, "run", "test:pilot", cwd=folder)
    run(npm, "run", "test:bun", cwd=folder)
    run(npm, "run", "check:package", cwd=folder)
    run(npm, "audit", "--audit-level=low", cwd=folder)
    destination = ROOT / "target/delivery-node"
    destination.mkdir(parents=True, exist_ok=True)
    packed = json.loads(run(npm, "pack", "--json", "--ignore-scripts", "--pack-destination",
                            str(destination), cwd=folder, capture=True))
    require(len(packed) == 1, "expected one Node package")
    safe_files([item["path"] for item in packed[0]["files"]])
    manifest = json.loads((folder / "package.json").read_text(encoding="utf-8"))
    require(manifest.get("private") is not True, "private Node adapter is not publishable")
    return folder, str(destination / packed[0]["filename"]), manifest


def package_waves(data):
    packages = release_packages(data)
    levels = {}
    waves = []
    for package in packages:
        dependencies = [levels[d["name"]] for d in package["dependencies"] if d.get("path")]
        level = 1 + max(dependencies, default=-1)
        levels[package["name"]] = level
        while len(waves) <= level:
            waves.append([])
        waves[level].append(package["name"])
    require(len(waves) <= 8, "release graph exceeds reviewed eight OIDC waves; extend workflow before publishing")
    return waves


def publish_crate(name):
    packages = {p["name"]: p for p in release_packages(metadata())}
    require(name in packages, "package is outside reviewed release graph")
    package = packages[name]
    url = f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}"
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "humanymous-release"}), timeout=30):
            pass
    except urllib.error.HTTPError as error:
        require(error.code == 404, f"registry lookup failed: {error.code}")
    else:
        raise ValueError("immutable version already exists: identity review required; refusing blind rerun")
    run("cargo", "publish", "--locked", "-p", name)


def release_gate():
    require(os.environ.get("GITHUB_EVENT_NAME") == "push" and
            os.environ.get("GITHUB_REF", "").startswith("refs/tags/"), "release requires tag push")
    tag = os.environ["GITHUB_REF"][len("refs/tags/"):]
    require(re.fullmatch(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", tag), "strict stable SemVer tag required")
    head = run("git", "rev-parse", "HEAD", capture=True).strip()
    require(head == os.environ.get("GITHUB_SHA"), "checkout differs from exact release SHA")
    require(run("git", "rev-parse", f"refs/tags/{tag}^{{commit}}", capture=True).strip() == head,
            "tag commit differs from checkout")
    run("git", "merge-base", "--is-ancestor", head, "origin/main")
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    version = workspace["workspace"]["package"]["version"]
    require(tag == f"v{version}", "tag/workspace version mismatch")
    for package in release_packages(metadata()):
        require(package["version"] == version, f"{package['name']}: release version mismatch")
    node = json.loads((ROOT / "adapters/node/package.json").read_text(encoding="utf-8"))
    require(node["version"] == version, "Node/tag version mismatch")
    readiness()


def check_jobs(raw):
    results = json.loads(raw)
    require(results and all(result["result"] == "success" for result in results.values()),
            "every required validation job must succeed; skipped/cancelled is not success")


def policy():
    # actionlint is the syntax/type authority; these are additional shipping rules.
    import yaml
    for path in sorted((ROOT / ".github/workflows").glob("*.yml")):
        doc = yaml.load(path.read_text(encoding="utf-8"), Loader=yaml.BaseLoader)
        require(doc.get("permissions") == {"contents": "read"}, f"{path.name}: broad top-level permissions")
        require("pull_request_target" not in doc["on"], "untrusted PR privileged trigger forbidden")
        for name, job in doc["jobs"].items():
            permissions = job.get("permissions", {})
            require(permissions.get("contents") != "write" or
                    (path.name == "release.yml" and name == "gh-release"), "unexpected contents write")
            require(permissions.get("packages") != "write" or
                    (path.name == "release.yml" and name == "images"), "unexpected image publication")
            require(permissions.get("pages") != "write" or
                    (path.name == "pages.yml" and name == "deploy"), "unexpected Pages publication")
            if permissions.get("id-token") == "write":
                require(path.name in {"release.yml", "pages.yml"} and
                        name in {"crates", "npm", "images", "deploy"}, f"unexpected OIDC job {path.name}/{name}")
            for step in job.get("steps", []):
                action = step.get("uses", "")
                require(not action or action.startswith("./") or
                        re.fullmatch(r"[\w./-]+@[0-9a-f]{40}", action), f"unpinned action: {action}")
        if path.name == "release.yml":
            require(doc["concurrency"]["cancel-in-progress"] == "false", "release must not cancel")
            require(doc["jobs"]["validate"]["uses"] == "./.github/workflows/rust-validation.yml",
                    "release must rerun same-commit validation")
            require("workflow_dispatch" not in doc["on"], "manual publication forbidden")
            images = doc["jobs"]["images"]
            require({item["name"] for item in images["strategy"]["matrix"]["include"]} == {"core", "gate"},
                    "only defender core/gate images may publish")
            require(images["needs"] == "readiness" and doc["jobs"]["readiness"]["needs"] == "validate",
                    "image publication must require exact-commit readiness")
            for name in ("crates", "npm"):
                needs = doc["jobs"][name]["needs"]
                require("readiness" in ([needs] if isinstance(needs, str) else needs), "registry publication bypasses readiness")
        if path.name == "rust-validation.yml":
            expected = {"policy", "native", "msrv", "quality", "browser", "docker", "images", "packages", "node", "advisories", "documentation", "pages"}
            require(set(doc["jobs"]["ready"]["needs"]) == expected, "required validation job removed from aggregate")
            for name in expected:
                require("if" not in doc["jobs"][name], f"required gate may not be conditional: {name}")
                require(not doc["jobs"][name].get("continue-on-error"), f"required gate may not tolerate failure: {name}")
        require(not re.search(r"(?:NODE_AUTH_TOKEN|CARGO_REGISTRY_TOKEN):\s*\$\{\{\s*secrets\.",
                              path.read_text(encoding="utf-8")), "registry token fallback forbidden")
    print("workflow pins and delivery permission policy passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["policy", "readiness", "package", "node", "release", "pilot", "jobs", "publish-crate", "publish-npm", "crate-order", "crate-waves", "publish-wave", "image-sources", "native-artifacts"])
    parser.add_argument("argument", nargs="?")
    args = parser.parse_args()
    if args.command == "pilot":
        require(args.argument in CASES, "unknown pilot kind")
        pilot(args.argument)
    elif args.command == "jobs":
        check_jobs(os.environ["NEEDS_JSON"])
    elif args.command == "publish-crate":
        release_gate()
        publish_crate(args.argument)
    elif args.command == "crate-waves":
        waves = package_waves(metadata())
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write(f"count={len(waves)}\n")
    elif args.command == "publish-wave":
        release_gate()
        for name in package_waves(metadata())[int(args.argument)]:
            publish_crate(name)
    elif args.command == "crate-order":
        print(json.dumps([p["name"] for p in release_packages(metadata())]))
    elif args.command == "publish-npm":
        release_gate()
        folder, archive, _ = node_check()
        require(not os.environ.get("NODE_AUTH_TOKEN") and not os.environ.get("NPM_TOKEN"), "OIDC only")
        run("npm", "publish", archive, "--access", "public", "--provenance", cwd=folder)
    else:
        {"policy": policy, "readiness": readiness, "package": package_check,
         "node": node_check, "release": release_gate, "image-sources": image_sources,
         "native-artifacts": native_artifacts}[args.command]()


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"delivery blocked: {error}", file=sys.stderr)
        sys.exit(1)
