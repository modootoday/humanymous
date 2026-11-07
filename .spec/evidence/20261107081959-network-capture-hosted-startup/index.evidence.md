---
id: evidence:root/20261107081959
title: Capture candidate hosted startup failure with zero execution
status: active
created: 2026-11-07T08:19:59Z
updated: 2026-11-07T08:19:59Z
files:
  - {path: receipt.json, sha256: e76abfc67735ab9929e0eccc54ac626d6cc12211341b37e3cefaa130ae107047, bytes: 1254, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107072531]
---

# Hosted startup failure

Run42354873001 targets exactlya26d1c3f08b365ca93545c6a51343095868edb45.
GitHub metadata independently confirms completed/startup_failure and an empty
jobs array:0jobs,0tests, no runner-assigned capture execution. This is a new
failure, separate from historical63f69ae qualification failures. Root reports
the workflow diagnostic as nested policy actions:read exceeding the caller's
actions:none ceiling; the capture owner did not independently fetch annotations.
CI owner is responsible for the caller permission fix and regression; root alone
pushes/dispatches the next candidate. No execution/push occurred from this owner.

Owned capture commitb0363e9 is an ancestor of the exact candidate. Its transport
blobf97d9398a9da89b8b2bf5a29f715dcaa9a03f921 matches the candidate blob, verified
with Git object reads. No local WIP is projected into that source identity.

The candidate rust-validation workflow and delivery pilot source contain no
capture/hosted-pilot/hosted_transport invocation. Generic CI recovery therefore
does not establish execution of the required five ignored feature-gated capture
pilots. Root was explicitly notified to add their actual standard-hosted command
through the CI owner. hosted-feature compilation, actual5pilots and runtime/
Docker request-body/stream/effect qualification remain open, not passed.

Run: https://github.com/modootoday/humanymous/actions/runs/42354873001
