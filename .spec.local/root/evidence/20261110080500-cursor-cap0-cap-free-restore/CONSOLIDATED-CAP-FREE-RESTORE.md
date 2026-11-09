# CONSOLIDATED Cap-free-restore packet (Cap=0)

- **tip:** `998ae019220af38174b7fc5337dcbf9fe08364f9` (`codex/rust`) · Cap burn=**0** · Cap free=**false** · product=**0**
- **BrowserReady / AT / AUTH:** false / absent / absent
- **Cap free mechanism:** `.spec.local/root/evidence/20261109020800-cursor-unlock-a-desktop-baseline-realloc/CLOSER.json` · measured `cap_exit=0` `cap_free=False` via `cap_free_after_unlock_a`
- **Cursor-executable NEW non-twin GRANT→TRANSFER:** **none**
- **Unlock C:** still **6** LIVE digest mismatches (unchanged vs `20261110074900`); leave existing rematch packet — Root must rematch `preimageSha256` to LIVE digests below (do not invent GRANT bytes)

## Root must match LIVE preimages (Unlock C)

| destination | liveSha256 (Root must rematch) | prior expectedSha256 |
| --- | --- | --- |
| `build/rust/authority-lab.Dockerfile` | `1a926ad99874885b84b1bbebaec0056fdd92ab1ff30989ae1741c9e6cb427cc3` | `d51e39354daf9b7aee6df52b79738e8e4e304a63972b3e6b9ce51654f36d717e` |
| `deployments/lab/INTEGRATION.md` | `b7fa7dc2fad1e3b24f247dea657499e4a4dcac04f45d66e948665112bb001315` | `86dc9500c3cca6821b89e8a7bd0a1f9b9bcc0fb43362e524b15ed9dbbbbeeb82` |
| `deployments/lab/authority/cap-pilot-contract.json` | `07acf975acdf7732083d6569ae13a43716c274477600204201991493f7b7a42f` | `None` |
| `deployments/lab/authority/CAP-PILOT.md` | `097747547f88cf39e8dfc609f7e7e6bafe77176bd3c28ce41589857c1fc1a570` | `None` |
| `test/lab/runner/src/snapshot.rs` | `04fafd385cb067bbb75fdaa332c8f570429ce0cae5e74eb487b9dc8d3845d00e` | `dc74375296c461d1a995744f7b29b13d996bfc20c6f7eaf3fea9f5c5f7771ea6` |
| `test/lab/runner/src/orchestration/authority.rs` | `bfadc77fbeb4f0912dbd247feca947e6dfba9f188b3ba2c99449ee07b2f4d1da` | `None` |

## Documented Cap free restore (Root-owned GRANT content)

1. Land **AT-HANDOFF** (`humanymous.root-interactive-at-windows-tuple/1`) and **AUTH-HANDOFF** (`humanymous.root-n06-owner-join-auth-handoff/1`) — Cursor Cap0-applies only; never forges.
2. Issue **NEW non-twin** Cap-ready `GRANT.json` + `TRANSFER`/`transfer-manifest` whose `preimageSha256` matches LIVE destination bytes (not spent G2/G5/CR/G6/RB/PM).
3. Cursor transfer-apply → Cap realloc → Cap free mechanism `cap_free=true` → Cap=1 once product path toward `pass_product_clean_room` (honest EXIT).
4. Twins G2/G5/CR/G6/RB/PM reburn **FORBIDDEN**.

## Exact next closer

Root AT+AUTH + NEW non-twin Cap-ready matching LIVE preimages (or Unlock C rematch). Cap burn=0 until Cap free=true. goal_complete=false.
