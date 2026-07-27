---
title: Run the Virtual USB laboratory
description: "Provision and run the Docker-only kernel-emulated USB input laboratory, inspect its English control ladder and Korean, Chinese, and Japanese input-method results, and expose bounded status in Ledger."
---

# Run the Virtual USB laboratory

**Diátaxis quadrant:** How-to.  
**Audience:** operators and evaluators running an isolated test machine they own.

> **Important:** This is a reference laboratory for an authorized deployment. It
> creates a Universal Serial Bus (USB) device in a Linux kernel with its virtual
> host-controller driver; it does not use or attest physical USB hardware. A
> complete result does not prove human input,
> electrical transport, an external cable, independent firmware, or general
> support for every input method.

## Implementation status

The accepted architecture uses an outer Docker Compose project to start and
destroy a pinned QEMU micro virtual machine. That machine boots its own Linux
kernel and pinned Docker Engine. Inner Compose projects run the detector, stock
browsers, controller, observers, Virtual USB lifecycle, and evaluators inside the
guest. The outer Docker host's kernel is not the measurement kernel.

The Docker-managed independent-kernel runner and its six-device kernel smoke are
implemented. The browser measurement is still incomplete: the runner has not yet
published an accepted Chromium or Firefox result that includes the
server-authoritative score receipt and residue-free terminal evidence. A native
Linux host path remains a diagnostic only. Do not report a complete English
control ladder or complete input-method result until the independent-kernel
runner produces and validates those artifacts.

The target QEMU service has no network interface, forwarded port, host input or
display device, Docker socket, shared host filesystem, or QEMU keyboard/mouse
input channel. A read-only seed image carries the fixed run manifest and preloaded
inner images. Separate bounded serial channels carry one nonce-bound run command
and the terminal result. A fresh copy-on-write guest disk is removed after every
ladder.

## What the run measures

The English ladder runs first:

1. virtual control with framebuffer observation;
2. read-only document observation plus virtual control;
3. framebuffer observation plus kernel-emulated USB control;
4. read-only document observation plus kernel-emulated USB control.

Chromium runs all four combinations, followed by Firefox. Every combination uses a fresh
browser profile and inner Compose project. The two kernel-emulated USB
combinations must change the page
through the guest kernel's Virtual USB gadget, Human Interface Device driver, and
input-event path. QEMU keyboard, mouse, tablet, monitor input, native-host
input-event devices, and schema-only receipts cannot substitute for a measured
mode.

For every English mode, the browser must load and instantiate the production
WebAssembly detector, submit the normal collection report, and receive the one
verdict calculated by Core. After interaction ends, a separate evaluator binds
Core's risk score and verdict for that session to the coarse framebuffer verdict.
It does not calculate a score or expose detector data to the controller. A missing
WebAssembly signal, mismatched session, recomputed score, or different verdict
invalidates the run.

English browser images do not contain the Chinese, Japanese, and Korean fonts,
Intelligent Input Bus
framework, or language engines. The input-method axis selects a separate
all-locale image for Chromium or Firefox. The attempt manifest binds that choice
to the axis and rejects an English image in an input-method episode or an
input-method image in an English episode.

After the English ladder completes, a separate input-method axis sends only
allowlisted United States keyboard-layout Human Interface Device (HID) key
positions through the emulated USB device. Korean, Simplified Chinese, and
Japanese run once in Chromium and once in Firefox. The pinned Intelligent Input
Bus (IBus) engine creates the Unicode text; USB HID reports carry key positions,
not Unicode.

The input-method result requires a visible native-composition cue and a separate
committed-value cue from the fixed local fixture. The input actor issues only
the fixed pointer and key sequence and cannot write a framebuffer verdict. A
separate read-only framebuffer observer records the preedit cue before the
committed-value cue; it has no input or browser-document channel. Neither
process receives the expected text, clipboard access, an input-method control
socket, or a direct-Unicode command. The final receipt binds the actor, observer,
fixed input policy, emulated-device reports, and exact keyboard press/release
sequence. These six results are not averaged with detector verdicts.

Each input-method run also records the selected engine, font, and keyboard-data
package versions and content inventories. Its private session bus and
input-method state live only in the browser container's temporary filesystem.
The run fails unless the engine and its descendants stop and those state
directories are removed without residue.

## Provision pinned local images

Provisioning is separate from a measured run. The target provisioning command
builds the project-owned images plus the pinned QEMU image, guest kernel,
initial memory filesystem, immutable base disk, guest Docker Engine and Compose
plugin, and preloaded inner-image archive. It records every exact digest. A local
build does not become canonical merely because it was signed. Canonical catalog
generation additionally requires the release-produced attestation bundle and a
private key matching the public verification key pinned in the verifier image.
Do not commit the private key.

The provisioner enforces these storage ceilings:

- 128 MiB for the pull-request kernel smoke image;
- 288 MiB for the full kernel-runner image;
- 672 MiB for one cell's compressed inner-image archive; and
- 4 GiB additional host storage during a complete laboratory run.

The standard detector-versus-bots continuous-integration job has a separate
6 GiB whole-job peak ceiling and 1.75 GiB post-build retained ceiling. A
quarter-second sampler stays active through image scans, browser tests, and
feature overlays. Build cache, guest disks, writable overlays, and inner-image
archives are removed rather than uploaded. These are fail-closed ceilings, not
claims that the pending browser measurement has passed.

```sh
bash scripts/external-input-vusb-provision.sh
```

The measured command uses `pull_policy: never`. A missing approved guest bundle or
preloaded image may return `UNAVAILABLE` before guest boot, but that status cannot
complete either kernel-emulated USB combination or a canonical baseline. Missing hardware acceleration alone
selects the separately keyed QEMU software-translation profile; it is not a final
unavailable result. A digest, signature, profile, or policy mismatch is rejected
before guest boot or kernel mutation. The catalog cannot replace its own
verification key.

## Run the canonical ladder

Use the allowlisted reference profile:

```sh
bash scripts/external-input-vusb-docker.sh --model reference-relative-v1
```

This command runs the complete ladder only on the prepared independent-kernel
laboratory authority. The native-host fallback remains diagnostic and cannot
complete the ladder.

The target supervisor runs Chromium modes 1 through 4, Firefox modes 1 through 4,
then the six language/browser input-method cells. It does not retry a failed
candidate selection or keep only successful attempts.

Results are written below
`deployments/artifacts/external-input-vusb/<run-id>/`. The bounded
`ladder-result.json` summary separates:

- English modes measured, defended outcomes, and honest residuals;
- input-method cells measured and passed;
- cleanup or purity failures.

Exit status `0` means both axes and terminal cleanup completed. Status `1` means
a test, purity check, or safety assertion failed, `2` means the command or
admission configuration was invalid, `3` means the required host capability was
unavailable, and `4` means a comparison result was intentionally non-canonical.

> **Warning:** A safety abort means the disposable machine may retain USB gadget,
> input-method, or Compose state. Do not start another measured run on that
> machine. Inspect the receipts, then discard or restore the machine.

## Show the status in Ledger

Mount the artifact root read-only into the Gate container or process and set the
result directory explicitly:

```text
-virtual-usb-results-dir /results/external-input-vusb
```

The equivalent environment variable is:

```text
HMN_VIRTUAL_USB_RESULTS_DIR=/results/external-input-vusb
```

Open Ledger and select **Virtual USB Lab**. The view is disabled by default and
accepts only bounded summaries with the exact English and input-method schemas.
This view performs schema checks only; it does not authenticate the artifact and
labels it as an unverified laboratory summary. It does not read raw device paths,
reports, typed text, screenshots, or detector signals, and it never recalculates
a score.

The acceptance target is shown separately. It is not the current measured status:

```text
English control ladder target: all eight combinations
Input-method composition target: all six language/browser combinations
Physical USB evidence: not present
```

## Interpret incomplete results

- `UNAVAILABLE` before guest boot means the pinned QEMU or guest bundle, image,
  browser, pinned document observer, input method, font, or required Docker
  capability was absent. It leaves the canonical ladder incomplete.
- `PURITY_FAIL` means a forbidden input path, host state, stale profile,
  direct-Unicode capability, or evidence mismatch was found.
- `FAIL` means the measured task or composition did not complete in the pinned
  environment.
- `SAFETY_ABORT` means input neutralization or residue-free cleanup was not
  proven.
- A detector `ALLOW` result is retained as an honest residual. It is not rewritten
  as a defended outcome.

Firefox Extended Support Release uses the project-owned, hash-pinned observer
from its distribution extension directory. The disposable image disables
extension-signature enforcement only for this fixed laboratory extension, using
the browser's supported enterprise configuration. This changes the extension
surface and is not presented as an extension-free browser baseline.

The two physical USB combinations remain declared in the external input control ladder
but are not part of this kernel-emulated result. They report unavailable until a
separate, independently attested device, firmware identity, physical path,
exclusive input seat, and safety controls are supplied.

## Verify while browser measurement is pending

You may still run schema, gateway, profile, outer/inner Compose, and receipt
contract tests. Those tests are useful implementation checks but cannot satisfy
the kernel-emulated USB ladder because they do not prove QEMU booted the pinned
independent Linux kernel, enumeration passed through that guest's virtual
host-controller and Human Interface Device input path, Core produced the real
WebAssembly-backed score, or inner and outer cleanup completed.

## Related

- [Self-validation: red-team your own deployment](self-validation-red-team.md)
- [Detection Observatory](detection-observatory.md)
- [Supported topologies](../reference/supported-topologies.md)
- [Glossary](../reference/glossary.md)
