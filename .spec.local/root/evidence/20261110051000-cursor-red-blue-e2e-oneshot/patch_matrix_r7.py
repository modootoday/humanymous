#!/usr/bin/env python3
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

root = Path(r"C:/workspaces/humanymous")
path = root / ".spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/matrix.json"
m = json.loads(path.read_text(encoding="utf-8"))

# Recover schema: PowerShell ConvertTo-Json tip-bind renamed/retained cells as combinations.
cells = m.get("cells") or m.get("combinations") or []
if not cells:
    raise SystemExit("no cells/combinations found")
m["cells"] = cells
m["combinations"] = cells

head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
short = subprocess.check_output(["git", "rev-parse", "--short=8", "HEAD"], cwd=root, text=True).strip()
utc = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

evidence = [
    ".spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/FULL-SWARM-EXIT1-OVERLAYS.json",
    ".spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/console-r7.log",
    ".spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/BOUNDED-EXIT0.json",
]
note = (
    "Docker Desktop e2e measured: bounded SkipSwarm EXIT0 + full r7 swarm PASS "
    "(proxy_rotation; overall EXIT1 overlays pool-overlap). "
    "counts_as=oracle_red_blue_desktop_component_not_product ONLY. "
    "Never invent pass_product_clean_room; oracle != Rust M3."
)

updated = []
for c in cells:
    if c.get("scenario") == "oracle-detector-bots" and str(c.get("gate", "")).startswith("red_blue_"):
        c["prior_status"] = c.get("status")
        c["status"] = "pass_component_desktop"
        c["counts_as_product"] = False
        c["evidence"] = evidence
        c["notes"] = note
        updated.append(c["gate"])

m["observed_utc"] = utc
m["prior_head_sha_cap0_tip_bind"] = m.get("head_sha")
m["head_sha"] = head
m["tip_bind"] = short
m["tip_bind_note"] = (
    "Cap=0 tip-bind after full Docker e2e r7 (swarm PASS + proxy_rotation; "
    "overall EXIT1 overlays); all five oracle red_blue_* → pass_component_desktop; "
    "Cap free=false; product=0; never invent pass_product_clean_room"
)
m["product"] = 0
m["pass_product_clean_room"] = 0

waves = m.setdefault("waves", {})
waves["full_e2e_swarm_r7_exit1_overlays"] = {
    "observed_utc": utc,
    "head_sha": head,
    "overall_exit": 1,
    "swarm_passed": True,
    "swarm_proxy_rotation": True,
    "overlays_failed": True,
    "cap_free": False,
    "product": 0,
    "pass_product_clean_room": 0,
    "counts_as": "oracle_red_blue_desktop_component_not_product",
    "evidence": ".spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/FULL-SWARM-EXIT1-OVERLAYS.json",
    "note": "Full Docker e2e without SkipSwarm: all five red_blue_* remapped pass_component_desktop; overlays EXIT1 retained honest; product=0",
}

path.write_text(json.dumps(m, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print("updated", updated)
print("tip", m["tip_bind"], "ppc", m["pass_product_clean_room"], "cells", len(m["cells"]))
