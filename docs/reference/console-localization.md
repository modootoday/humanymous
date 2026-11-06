---
layout: doc
title: Interface language and accessibility
permalink: /reference/console-localization.html
reviewed: 2026-11-07
---
# Interface language and accessibility

**Diátaxis quadrant:** Reference. **Audience:** interface contributors and reviewers.

The Rust console's locale coverage is not release-qualified. English public
documentation does not imply a translated or accessible management interface.

Each supported locale needs complete status, validation, recovery and approval
messages. Preserve exact machine identifiers while explaining them in ordinary
language. Test overflow, keyboard focus, screen-reader announcements, dates and
bidirectional content with the actual interface.

Do not use color alone to distinguish outcomes. Challenge instructions must
support non-pointer input and alternatives. See [accessible recovery](../help/challenge-accessibility.md).
