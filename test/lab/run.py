"""Host coordinates only Docker; all network test clients run in containers.

Each invocation owns a unique project and removes only that project's resources.
Evidence belongs in a pre-created local Specfold evidence bundle.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
DOCKER = shutil.which("docker") or "C:/Program Files/Docker/Docker/resources/bin/docker.exe"
os.environ["PATH"] = str(Path(DOCKER).parent) + os.pathsep + os.environ["PATH"]
COMPOSE = ROOT / "deployments/lab/compose.yaml"


def freeze_build_context(destination, output):
    """Copy allowed source only, then hash the immutable bytes actually built."""
    sources = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for folder, suffixes in [("crates", {".rs", ".json", ".toml"}), ("build/rust", {".Dockerfile", ".dockerignore"}), ("deployments/lab", {".py", ".mjs", ".json", ".yaml"}), ("adapters/node/src", {".mjs", ".cjs"})]:
        sources.extend(path for path in ROOT.joinpath(folder).rglob("*") if path.is_file() and path.suffix in suffixes)
    sources.extend(ROOT / "adapters/node" / name for name in ("package.json", "package-lock.json"))
    manifest = []
    for source in sorted(set(sources)):
        relative = source.relative_to(ROOT)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        data = source.read_bytes()
        target.write_bytes(data)
        manifest.append({"path": relative.as_posix(), "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)})
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.splitlines()
    provenance = {"gitHead": sha, "dirtyPaths": dirty, "meaning": "image built from immutable allowlisted snapshot; file digests, not HEAD alone, identify its source", "files": manifest}
    output.joinpath("source-provenance.json").write_text(json.dumps(provenance, indent=2), encoding="utf-8")
    return hashlib.sha256(json.dumps(manifest, sort_keys=True).encode()).hexdigest()


class Lab:
    def __init__(self, output, authority=False):
        self.project = "hm-rust-lab-" + uuid.uuid4().hex[:12]
        self.output = output
        self.transcript = []
        self.cases = []
        self.authority = authority
        self.authority_report = None
        os.environ["HM_LAB_EXPIRES"] = str(int(time.time()) + 1800)

    def docker(self, *args, check=True):
        result = subprocess.run([DOCKER, *args], cwd=ROOT, text=True, encoding="utf-8", errors="replace", capture_output=True, timeout=600)
        self.transcript.append({"argv": list(args), "code": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
        if check and result.returncode:
            raise RuntimeError(f"Docker command failed ({result.returncode}): {args}; {result.stderr[-2000:]}")
        return result

    def compose(self, *args, **kwargs):
        files = ["-f", str(COMPOSE)]
        if self.authority:
            files.extend(["-f", str(ROOT / "deployments/lab/compose.authority.yaml"), "--profile", "authority"])
        return self.docker("compose", "-p", self.project, *files, "--profile", "controls", *args, **kwargs)

    def record(self, name, passed, detail):
        self.cases.append({"id": name, "status": "passed" if passed else "failed", "detail": detail})
        if not passed:
            raise AssertionError(name)

    def inspect_service(self, name):
        container_id = self.compose("ps", "--all", "-q", name).stdout.strip()
        inspected = json.loads(self.docker("inspect", container_id).stdout)[0]
        self.record("owned-container-" + name, inspected["Config"]["Labels"]["com.docker.compose.project"] == self.project, container_id)
        return inspected

    def topology(self):
        resolved = self.compose("config", "--format", "json").stdout
        config = json.loads(resolved)
        self.output.joinpath("resolved-compose.json").write_text(resolved, encoding="utf-8")
        self.record("resolved-config-no-published-ports", all(not s.get("ports") for s in config["services"].values()), hashlib.sha256(resolved.encode()).hexdigest())
        self.compose("build", "origin")
        self.compose("up", "-d", "--wait", "--wait-timeout", "60", "origin", "outside-canary")
        self.compose("create", "attacker")
        origin = self.inspect_service("origin")
        canary = self.inspect_service("outside-canary")
        inspections = [origin, canary]
        self.output.joinpath("containers.json").write_text(json.dumps(inspections, indent=2), encoding="utf-8")
        for container in inspections:
            host = container["HostConfig"]
            self.record("effective-hardening-" + container["Name"], host["ReadonlyRootfs"] and host["CapDrop"] == ["ALL"] and "no-new-privileges:true" in host["SecurityOpt"] and not host["Privileged"] and not host["PortBindings"] and container["Config"]["User"] == "65532:65532" and not container["Mounts"], "nonroot/read-only/no-caps/no-ports/no-mounts")
        networks = [json.loads(line) for line in self.docker("network", "ls", "--filter", "label=com.docker.compose.project=" + self.project, "--format", "json").stdout.splitlines()]
        network_inspections = [json.loads(self.docker("network", "inspect", n["ID"]).stdout)[0] for n in networks]
        self.output.joinpath("networks.json").write_text(json.dumps(network_inspections, indent=2), encoding="utf-8")
        self.record("all-networks-internal", all(n["Internal"] for n in network_inspections), "owned bridge networks only; IPv6 not qualified")
        origin_ip = next(iter(origin["NetworkSettings"]["Networks"].values()))["IPAddress"]
        outside_ip = next(iter(canary["NetworkSettings"]["Networks"].values()))["IPAddress"]
        ingress = next(n for n in network_inspections if n["Labels"]["com.docker.compose.network"] == "ingress")
        gateway = ingress["IPAM"]["Config"][0]["Gateway"]
        self.compose("run", "--rm", "--no-deps", "backend-control")
        self.record("origin-positive-control", True, "same fixture reachable from private backend client")
        result = self.compose("run", "--rm", "--no-deps", "attacker", "python", "/lab/probe.py", origin_ip, outside_ip, gateway)
        probe = json.loads(result.stdout.strip().splitlines()[-1])
        self.cases.extend(probe["cases"])
        self.compose("stop", "origin")
        stopped = self.compose("run", "--rm", "--no-deps", "backend-control", check=False)
        self.record("dependency-lost-observed", stopped.returncode != 0, "positive client fails after origin stop; no Gate behavior claimed")
        self.compose("up", "-d", "--force-recreate", "--wait", "origin")
        self.compose("run", "--rm", "--no-deps", "backend-control")
        self.record("fixture-recreation", True, "origin recreated and healthy")
        image = json.loads(self.docker("image", "inspect", origin["Image"]).stdout)[0]
        self.output.joinpath("fixture-image.json").write_text(json.dumps(image, indent=2), encoding="utf-8")

    def actual_authority(self):
        if not self.authority:
            return
        if not ROOT.joinpath("crates/authority/INTEGRATION.md").is_file():
            raise RuntimeError("actual Rust authority integration contract missing")
        self.compose("build", "authority-pilot")
        self.compose("create", "authority-pilot")
        container = self.inspect_service("authority-pilot")
        host = container["HostConfig"]
        self.record("actual-authority-effective-hardening", host["NetworkMode"] == "none" and host["ReadonlyRootfs"] and host["CapDrop"] == ["ALL"] and "no-new-privileges:true" in host["SecurityOpt"] and not host["Privileged"] and not host["PortBindings"] and container["Config"]["User"] == "65532:65532" and not container["Mounts"], "network-none/nonroot/read-only/no-caps/no-ports/no-mounts")
        self.output.joinpath("authority-container.json").write_text(json.dumps(container, indent=2), encoding="utf-8")
        result = self.docker("start", "-a", container["Id"], check=False)
        self.authority_report = json.loads(result.stdout.strip().splitlines()[-1])
        self.output.joinpath("authority-report.json").write_text(json.dumps(self.authority_report, indent=2), encoding="utf-8")
        after = json.loads(self.docker("inspect", container["Id"]).stdout)[0]
        self.record("actual-authority-ipc-stage", result.returncode == after["State"]["ExitCode"] == 0 and all(c["status"] == "passed" for c in self.authority_report["cases"]), "see separate rust-authority-node-ipc report")
        image = json.loads(self.docker("image", "inspect", container["Image"]).stdout)[0]
        self.output.joinpath("authority-image.json").write_text(json.dumps(image, indent=2), encoding="utf-8")

    def cleanup(self):
        # Compose scopes deletion to this generated project. Never global prune.
        result = self.compose("down", "--volumes", "--remove-orphans", "--timeout", "5", check=False)
        remaining = self.docker("ps", "-aq", "--filter", "label=com.docker.compose.project=" + self.project, check=False)
        networks = self.docker("network", "ls", "-q", "--filter", "label=com.docker.compose.project=" + self.project, check=False)
        self.cases.append({"id": "cleanup-owned-project", "status": "passed" if result.returncode == remaining.returncode == networks.returncode == 0 and not remaining.stdout.strip() and not networks.stdout.strip() else "failed", "detail": self.project})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--require-full", action="store_true", help="exit 2 when mandatory product acceptance is unavailable")
    parser.add_argument("--authority", action="store_true", help="build and exercise actual Rust authority + Node frameworks")
    args = parser.parse_args()
    output = args.evidence.resolve()
    output.relative_to(ROOT / ".spec.local/root/evidence")
    if not output.joinpath("index.evidence.md").is_file():
        parser.error("create a local Specfold evidence bundle first")
    lab = Lab(output, args.authority)
    started = time.time()
    failure = None
    snapshot = tempfile.TemporaryDirectory(prefix="hm-rust-lab-build-")
    try:
        digest = freeze_build_context(Path(snapshot.name), output)
        os.environ["HM_LAB_BUILD_CONTEXT"] = snapshot.name
        lab.record("immutable-source-snapshot", True, digest)
        lab.topology()
        lab.actual_authority()
    except Exception as error:
        failure = str(error)
        lab.cases.append({"id": "execution", "status": "failed", "detail": failure})
    finally:
        try:
            lab.cleanup()
        except Exception as error:
            lab.cases.append({"id": "cleanup-owned-project", "status": "failed", "detail": str(error)})
        finally:
            snapshot.cleanup()
    unavailable = ["actual-gate-core-http-tls", "installation-cryptographic-network-target-binding", "independent-witness-durable-head", "browser-human-allow-control", "keyboard-challenge", "assistive-challenge", "zero-motor-control", "coherent-browser-ceiling", "host-all-ports-ipv6", "actual-nginx-authority", "provider-remote"]
    if not lab.authority_report:
        unavailable.append("actual-node-authority")
    report = {"kind": "topology-fixture", "project": lab.project, "duration_seconds": time.time() - started, "cases": lab.cases, "unavailable": unavailable, "releaseReady": False, "failure": failure, "ttl": "1800s expiry labels; run test/lab/cleanup.py externally after runner loss; no scheduled janitor asserted"}
    output.joinpath("lab-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    output.joinpath("docker-transcript.json").write_text(json.dumps(lab.transcript, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 1 if any(c["status"] == "failed" for c in lab.cases) else 2 if args.require_full else 0


if __name__ == "__main__":
    raise SystemExit(main())
