---
title: Verify a Released Image (cosign signature, SBOM, SLSA provenance)
description: "Before you run a pulled humanymous image, prove it: verify the keyless cosign signature against the release workflow identity, fetch and inspect its SPDX SBOM, and verify its SLSA provenance attestation — all pinned by digest."
keywords: ["cosign verify","keyless signing","OIDC issuer","certificate-identity-regexp","SLSA provenance attestation","SPDX SBOM","cosign download sbom","docker buildx imagetools inspect","pin by digest","ghcr.io humanymous-core humanymous-gate","supply-chain verification","sigstore","adopter image verification"]
---

# Verify a released image

**Diátaxis quadrant:** How-to. **Audience:** operators and integrators pulling the published `humanymous-core` / `humanymous-gate` images before running them.

The [release workflow](cut-a-release.md) does not just publish images to `ghcr.io` — it **cryptographically attests** them. Every image the `release.yml` pipeline pushes is:

- **keyless-signed with [cosign](https://docs.sigstore.dev/)** (Sigstore), where the signing identity is the release workflow itself (OIDC, no long-lived key to steal or leak);
- shipped with an **SPDX Software Bill of Materials (SBOM)** attestation listing what is inside it;
- shipped with a **[SLSA](https://slsa.dev/) provenance** attestation (`mode=max`) recording how and from what it was built.

This page gives you the exact adopter commands to check all three **before** you run the image, and explains what each one proves. Verification is the point: a pull alone tells you *what tag you got*, not *who built it or from what*. These checks tell you the rest.

> **Important:** cosign is **required tooling and is not bundled** with the images or this repo. Install it first — see the [Sigstore cosign install guide](https://docs.sigstore.dev/system_config/installation/). `docker buildx` (bundled with modern Docker) is used to read the SBOM + provenance attestations in steps (b) and (c).

---

## The identity you are verifying against

Keyless signatures do not verify against a public key you hold — they verify against an **identity** (who signed) and an **issuer** (which OIDC provider vouched for that identity). For humanymous releases those are fixed:

| What | Value |
| --- | --- |
| **Registry / images** | `ghcr.io/modootoday/humanymous-core`, `ghcr.io/modootoday/humanymous-gate` |
| **Certificate identity** (the signer) | the release workflow ref: `https://github.com/modootoday/humanymous/.github/workflows/release.yml@refs/tags/<tag>` |
| **Certificate OIDC issuer** | `https://token.actions.githubusercontent.com` |

Because releases are cut from tags (`v*.*.*`), the identity is matched with a **regexp** that pins the workflow file and the `refs/tags/` ref while allowing any tag. Pinning both the identity **and** the issuer is what stops a different repository, a pull-request build, or any other workflow from producing a signature your check would accept.

---

## Always pin by digest, not by tag

A tag such as `:latest` or `:0.2.0` is a **movable pointer** — it can be re-pushed to point at different bytes. A digest (`sha256:…`) names the exact image content and cannot be re-pointed. Resolve the tag to a digest once, then verify **and run** that digest.

```bash
# Resolve the tag you intend to run to an immutable digest.
docker pull ghcr.io/modootoday/humanymous-gate:latest
DIGEST=$(docker inspect --format='{{index .RepoDigests 0}}' ghcr.io/modootoday/humanymous-gate:latest)
echo "$DIGEST"     # e.g. ghcr.io/modootoday/humanymous-gate@sha256:abc123…
```

Use `$DIGEST` (the `name@sha256:…` form) in every command below. Verifying a tag and then running whatever that tag later points at defeats the whole exercise.

---

## a) Verify the cosign signature

```bash
cosign verify \
  --certificate-identity-regexp 'https://github.com/modootoday/humanymous/.github/workflows/release.yml@refs/tags/.*' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  "$DIGEST"
```

**What this proves.** The image content at that digest was signed by **this repository's release workflow, run on a tag**, as vouched for by GitHub's OIDC issuer — and cosign confirmed the signature is logged in the Sigstore transparency log. If the signature is missing, was produced by any other identity or issuer, or the image bytes differ from what was signed, the command **exits non-zero** — treat that as "do not run this image." On success cosign prints the verified certificate subject and issuer; confirm they read as the workflow ref and `token.actions.githubusercontent.com` above.

> **Note:** Both `--certificate-identity-regexp` and `--certificate-oidc-issuer` are **required** for a keyless verify. Omitting the issuer, or loosening the identity regexp so it matches other workflows/repos, weakens the check to the point of being decorative.

---

## b) Fetch and inspect the SBOM

The release attaches an **SPDX SBOM** describing the image's contents. Read it with docker buildx — the reliable path for a BuildKit-attached attestation:

```bash
docker buildx imagetools inspect "$DIGEST" --format '{{json .SBOM}}'
```

> **Note:** the SBOM and provenance are **BuildKit attestations** (attached to the image index as an `attestation-manifest`), not `cosign attest` predicates. `cosign download sbom` is deprecated and may not resolve a BuildKit-attached SBOM, so the `docker buildx imagetools` command above is the recommended retrieval. Only the attestation *retrieval* differs — the image **signature** in (a) is still cosign.

**What this proves / is for.** The SBOM is the ingredients list. Pipe it into your own tooling to answer supply-chain questions — "is package *X* at a vulnerable version in this exact image?", diffing the SBOM between two releases, or feeding it to a vulnerability scanner. It is inventory, not a verdict: pair it with the signature check in (a) (proves *who* produced this inventory) and CI's own scanning — the pipeline already gates on `govulncheck` and a Trivy HIGH/CRITICAL (fixable) scan before an image is published.

---

## c) Inspect the SLSA provenance

The release attaches **SLSA provenance** (`mode=max`, a BuildKit attestation) recording how the image was built. Read it with docker buildx:

```bash
docker buildx imagetools inspect "$DIGEST" --format '{{json .Provenance}}'
```

**What this proves.** The provenance is a record of **how the image was built** — the builder (GitHub Actions), the source repository and tag, and the build parameters (`buildDefinition`). Confirm its `buildDefinition` source/ref is the release you expected and the builder is GitHub Actions. Because it is a BuildKit attestation (not a `cosign attest` predicate), its authenticity rests on the image-index signature you already verified in (a): (a) proves the release workflow signed this exact index — which *includes* the attestation manifest — and (c) reads the signed-over provenance content. (`cosign verify-attestation --type slsaprovenance …` may also work if your cosign build resolves OCI referrers, but the buildx command is the reliable retrieval.)

---

## Put it together (adopter checklist)

Run these in order and stop on the first failure:

1. Resolve the tag to a **digest** and use only the digest afterwards.
2. `cosign verify …` — right signer, right issuer, signature valid. *(a)*
3. `docker buildx imagetools inspect --format '{{json .Provenance}}' …` — provenance `buildDefinition` source/ref as expected. *(c)*
4. `docker buildx imagetools inspect --format '{{json .SBOM}}' …` — capture the SBOM and feed it to your scanner / inventory. *(b)*
5. Deploy the **digest** you verified — never re-resolve the tag between verifying and running.

A failure at step 2 or 3 means the image is not authentically a humanymous release (or has been altered) — do not run it, and report it via [`SECURITY.md`](../../SECURITY.md).

---

## Related pages

- [Cut a release](cut-a-release.md) — the maintainer side: what a tag triggers, and where the signing/SBOM/provenance steps live in `release.yml`.
- [Security policy](../../SECURITY.md) — how to report a supply-chain or verification concern.
- [Security & code audit report](../reference/security-audit.md) — the supply-chain posture (govulncheck, CodeQL, Trivy, Dependabot) this verification complements.
- [HTTPS / TLS certificates](https-tls-certificates.md) — securing the edge once you trust the image, including admin-plane mutual Transport Layer Security.
