"""Reproduce delivery workflow structure (never generates reader prose)."""
from pathlib import Path
import yaml

ROOT = Path(__file__).resolve().parents[2]
PINS = {
    "checkout": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
    "rust": "dtolnay/rust-toolchain@683b588c487a69d01f6c6ddf40f036ff682cc2c8",
    "python": "actions/setup-python@ece7cb06caefa5fff74198d8649806c4678c61a1",
    "node": "actions/setup-node@249970729cb0ef3589644e2896645e5dc5ba9c38",
    "upload": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
    "download": "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093",
    "buildx": "docker/setup-buildx-action@bb05f3f5519dd87d3ba754cc423b652a5edd6d2c",
    "ruby": "ruby/setup-ruby@9d13dd6ce2979b207024f719160a99112fb38eea",
    "pages-upload": "actions/upload-pages-artifact@7b1f4a764d45c48632c6b24a0339c27f5614fb0b",
    "pages-deploy": "actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e",
    "crates-auth": "rust-lang/crates-io-auth-action@3268eb413133e2960fa4ae0e963ca76380c54cd9",
    "bun": "oven-sh/setup-bun@0c5077e51419868618aeaa5fe8019c62421857d6",
}


def action(action_id, **inputs):
    step = {"uses": PINS[action_id]}
    if inputs:
        step["with"] = inputs
    return step


def checkout(full=False):
    return action("checkout", **{"persist-credentials": False, "fetch-depth": 0 if full else 1})


def command(text, **kwargs):
    return {"run": text, **kwargs}


def job(steps, timeout=30, **kwargs):
    return {"runs-on": "ubuntu-latest", "timeout-minutes": timeout, "steps": steps, **kwargs}


def workflow(name, triggers, jobs, **kwargs):
    return {"name": name, "on": triggers, "permissions": {"contents": "read"}, **kwargs, "jobs": jobs}


def write(name, value):
    class Dumper(yaml.SafeDumper):
        def ignore_aliases(self, data):
            return True
    (ROOT / ".github/workflows" / name).write_text(
        "# Canon: .spec/sot/delivery.sot.md; reproduce with scripts/ci/render_workflows.py\n" +
        yaml.dump(value, Dumper=Dumper, sort_keys=False, width=100), encoding="utf-8", newline="\n")


def main():
    rust = action("rust", toolchain="stable")
    python = action("python", **{"python-version": "3.12"})
    node = action("node", **{"node-version": "24"})
    bun = action("bun", **{"bun-version": "1.3.14"})
    call = {"uses": "./.github/workflows/rust-validation.yml"}
    write("ci.yml", workflow("ci", {"push": {"branches": ["main", "codex/rust"]},
          "pull_request": None, "workflow_dispatch": None}, {"validate": call},
          concurrency={"group": "rust-ci-${{ github.ref }}", "cancel-in-progress": True}))
    jobs = {
        "policy": job([checkout(), python,
            command("python -m pip install PyYAML==6.0.3"),
            command("python -m unittest discover -s scripts/ci -p 'test_*.py'"),
            command("python scripts/ci/delivery.py policy"),
            command("python scripts/ci/check_action_pins.py"),
            command('go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.11\n"$(go env GOPATH)/bin/actionlint" -shellcheck=""')], 15),
        "native": job([checkout(), rust, python,
            command("cargo build --workspace --all-targets --locked"),
            command("cargo test --workspace --all-targets --locked"),
            command("cargo build --workspace --bins --release --locked"),
            command("python scripts/ci/delivery.py pilot native"),
            command("python scripts/ci/delivery.py native-artifacts"),
            action("upload", name="native-binaries-${{ matrix.os }}-${{ github.sha }}",
                   path="target/delivery-artifacts/", **{"if-no-files-found": "error"}),
            action("upload", name="native-${{ matrix.os }}-${{ github.sha }}",
                   path="target/delivery-evidence/", **{"if-no-files-found": "error"})], 40,
            **{"runs-on": "${{ matrix.os }}", "strategy": {"fail-fast": False,
                "matrix": {"os": ["ubuntu-latest", "macos-latest", "windows-latest"]}}}),
        "msrv": job([checkout(), action("rust", toolchain="1.88.0"),
            command("cargo build --workspace --all-targets --locked"),
            command("cargo test --workspace --all-targets --locked"),
            command("cargo test --manifest-path scripts/ci/docs-review/Cargo.toml --locked")]),
        "quality": job([checkout(), action("rust", toolchain="stable", components="rustfmt, clippy"),
            command("cargo fmt --all -- --check"),
            command("cargo clippy --workspace --all-targets --locked -- -D warnings"),
            command("cargo fmt --manifest-path scripts/ci/docs-review/Cargo.toml -- --check"),
            command("cargo clippy --manifest-path scripts/ci/docs-review/Cargo.toml --all-targets --locked -- -D warnings"),
            command("cargo test --manifest-path scripts/ci/docs-review/Cargo.toml --locked")]),
        "browser": job([checkout(), action("rust", toolchain="stable", targets="wasm32-unknown-unknown"), node,
            command("cargo build --locked --target wasm32-unknown-unknown -p humanymous-wasm --release"),
            command("python3 scripts/ci/delivery.py pilot browser"),
            action("upload", name="browser-linux-${{ github.sha }}", path="target/delivery-evidence/",
                   **{"if-no-files-found": "error"})], 35),
        "docker": job([checkout(), action("buildx"),
            command("python3 scripts/ci/delivery.py pilot docker"),
            action("upload", name="docker-linux-${{ github.sha }}", path="target/delivery-evidence/",
                   **{"if-no-files-found": "error"})], 75),
        "images": job([checkout(), action("buildx"), command("python3 scripts/ci/delivery.py image-sources"),
            {"uses": "docker/build-push-action@53b7df96c91f9c12dcc8a07bcb9ccacbed38856a",
             "with": {"context": ".", "file": "build/${{ matrix.name }}.Dockerfile", "push": False,
                      "load": True, "tags": "humanymous-${{ matrix.name }}:validation"}},
            {"uses": "aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25",
             "with": {"image-ref": "humanymous-${{ matrix.name }}:validation", "exit-code": "1",
                      "severity": "UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL", "ignore-unfixed": False}}], 40,
             strategy={"fail-fast": False, "matrix": {"name": ["core", "gate"]}}),
        "packages": job([checkout(), rust, command("python3 scripts/ci/delivery.py package")], 35),
        "node": job([checkout(), node, bun, command("python3 scripts/ci/delivery.py node")]),
        "advisories": job([checkout(), rust,
            command("cargo install cargo-audit --version 0.22.2 --locked"),
            command("mkdir -p target/delivery-evidence\ncargo audit --json > target/delivery-evidence/cargo-audit.json\ncargo audit --file scripts/ci/docs-review/Cargo.lock --json > target/delivery-evidence/docs-tool-audit.json"),
            {"uses": "aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25",
             "with": {"scan-type": "fs", "scan-ref": ".", "exit-code": "1",
                      "severity": "UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL", "ignore-unfixed": False,
                      "skip-dirs": ".git,.spec.local,target,scripts/ci/docs-review/target"}},
            {**action("upload", name="advisories-${{ github.sha }}", path="target/delivery-evidence/",
                      **{"if-no-files-found": "error"}), "if": "always()"}]),
        "documentation": job([checkout(), rust,
            command("cargo run --locked --manifest-path scripts/ci/docs-review/Cargo.toml -- .")]),
        "pages": {"uses": "./.github/workflows/pages-build.yml"},
    }
    jobs["ready"] = job([checkout(), command("python3 scripts/ci/delivery.py jobs",
         env={"NEEDS_JSON": "${{ toJSON(needs) }}"})], 5, **{"if": "always()", "needs": list(jobs)})
    write("rust-validation.yml", workflow("Rust validation authority", {"workflow_call": None}, jobs,
          defaults={"run": {"shell": "bash"}}))
    write("pages-build.yml", workflow("Checked Pages artifact", {"workflow_call": None}, {
        "build": job([checkout(), action("ruby", **{"ruby-version": "3.3.12", "working-directory": "docs", "bundler-cache": True}),
            command("test -s Gemfile.lock\ntest -f scripts/validate-pages.sh\nbundle check\nbundle exec bundle-audit check --update"),
            command('bash scripts/validate-pages.sh --destination "$RUNNER_TEMP/pages-apex" --baseurl \'\'\nbash scripts/validate-pages.sh --destination "$RUNNER_TEMP/pages-subpath" --baseurl /humanymous'),
            action("pages-upload", name="checked-pages-${{ github.sha }}", path="${{ runner.temp }}/pages-apex")],
            env={"BUNDLE_FROZEN": "true"}, defaults={"run": {"working-directory": "docs", "shell": "bash"}})
    }))
    write("pages.yml", workflow("Pages deployment", {"push": {"branches": ["main"]}, "workflow_dispatch": None}, {
        "validate": call,
        "deploy": job([{**action("pages-deploy", artifact_name="checked-pages-${{ github.sha }}"), "id": "deployment"}], 10,
            **{"if": "github.ref == 'refs/heads/main'", "needs": "validate",
               "environment": {"name": "github-pages", "url": "${{ steps.deployment.outputs.page_url }}"},
               "permissions": {"pages": "write", "id-token": "write"}})
    }, concurrency={"group": "pages-production", "cancel-in-progress": False}))

    # Keep the reviewed image signing/SBOM/provenance/release-notes machinery;
    # replace only its authorization gate and narrow the old global privileges.
    release = yaml.safe_load((ROOT / "scripts/ci/frozen-go-release.yml.txt").read_text(encoding="utf-8"))
    image_jobs = {k: v for k, v in release["jobs"].items() if k != "require-ci"}
    image_jobs["images"]["needs"] = "readiness"
    image_jobs["images"]["permissions"] = {"contents": "read", "packages": "write", "id-token": "write"}
    image_jobs["images"]["environment"] = "release"
    image_jobs["images-ok"]["permissions"] = {"contents": "read", "packages": "read"}
    image_jobs["gh-release"]["needs"] = ["images", "images-ok", "crates", "npm"]
    image_jobs["gh-release"]["environment"] = "release"
    image_jobs["gh-release"]["steps"].insert(1, action("download", pattern="native-binaries-*", path="release-assets", **{"merge-multiple": True}))
    image_jobs["gh-release"]["steps"][-1]["with"]["files"] = "release-assets/*"
    image_jobs["gh-release"]["steps"][-1]["with"]["fail_on_unmatched_files"] = True
    for item in image_jobs.values():
        for step in item.get("steps", []):
            if step.get("uses") == PINS["checkout"]:
                step.setdefault("with", {})["persist-credentials"] = False
    release_jobs = {"validate": call,
        "readiness": job([checkout(True), rust, node,
            command("python3 scripts/ci/delivery.py release"),
            command("python3 scripts/ci/delivery.py crate-waves", id="waves")], 15,
            needs="validate", outputs={"count": "${{ steps.waves.outputs.count }}"}),
        **image_jobs}
    crate_steps = [checkout(True), rust]
    for wave in range(8):
        condition = f"${{{{ fromJSON(needs.readiness.outputs.count) > {wave} }}}}"
        crate_steps.extend([
            {**action("crates-auth"), "id": f"auth{wave}", "if": condition},
            command(f"python3 scripts/ci/delivery.py publish-wave {wave}", **{"if": condition},
                    env={"CARGO_REGISTRY_TOKEN": f"${{{{ steps.auth{wave}.outputs.token }}}}"})])
    release_jobs["crates"] = job(crate_steps, 90, needs="readiness", environment="release",
        permissions={"contents": "read", "id-token": "write"})
    release_jobs["npm"] = job([checkout(True), rust, node, bun,
        command("npm install --global npm@11.16.0"),
        command("python3 scripts/ci/delivery.py publish-npm")], 25, needs=["readiness", "crates"], environment="release",
        permissions={"contents": "read", "id-token": "write"})
    write("release.yml", workflow("release", {"push": {"tags": ["v*.*.*"]}}, release_jobs,
        concurrency={"group": "release", "cancel-in-progress": False}))


if __name__ == "__main__":
    main()
