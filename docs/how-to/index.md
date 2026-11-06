---
layout: doc
title: Evaluation preparation guides
permalink: /how-to/
reviewed: 2026-11-07
---
# Evaluation preparation guides

**Diátaxis quadrant:** How-to. **Audience:** operators and contributors.

The Rust replacement is not release-qualified. These guides prepare concrete
reviews and pilots; they do not supply an operational installation procedure.

| Task | Guide |
| --- | --- |
| Deployment | [Policy readiness](deployment-policy-operations.md), [TLS](https-tls-certificates.md), [request integrity](configure-attested-routes.md), [credentials](configure-credential-verifiers.md) |
| Artifact lifecycle | [Image review](verify-the-image.md), [release readiness](cut-a-release.md), [migration](upgrade-migration.md) |
| Audit and privacy | [Keys](key-management.md), [audit continuity](verify-audit-log.md), [telemetry export](observability-siem.md), [console review](audit-console-tour.md) |
| Evaluation | [Owned lab](self-validation-red-team.md), [detector integration](run-detection-engine.md), [device scope](run-virtual-usb-lab.md), [regression cases](write-a-red-profile.md) |
| Model and interface | [Model operations](operate-behavioral-model.md), [observatory](detection-observatory.md), [decision view](observatory-tour.md) |
| Contribution and diagnosis | [Detection proposals](extend-detection.md), [failures](troubleshooting-faq.md) |

For an introductory planning exercise, use the [monitor evaluation tutorial](../tutorials/quickstart-monitor-mode.md).
