# Cap0 Cap-free mechanism deep-read (tip a241d8df)

- **tip:** `a241d8dfca2c64b20f4b6ef81f460db7c8818c7a` · Cap burn=**0** · Cap free=**false** · product=**0** · BrowserReady=**false** · AT/AUTH=**absent** · Unlock C=**false**
- **Authority CLOSER:** `.spec.local/root/evidence/20261109020800-cursor-unlock-a-desktop-baseline-realloc/CLOSER.json` · sha256=`1a37052f5b6bec02376df2c3ee3c54f093cbd114d7b84a50e92fc4d0387cb137` · `cap_exit=0` · `cap_free=false`
- **Probe:** `cap_free_after_unlock_a` reads that file only
- **NEW:** no Cap CLI free-restore/realloc subcommand; no Rust writer of `CLOSER.cap_free=true`; PM realloc receipt claimed free-after without updating authority CLOSER
- **Cap free restore attempt:** **none executable** (would require inventing CLOSER/grant bytes)
- **Cap1:** not attempted (Cap free=false; twin reburn forbidden)

## Exact Root bytes needed

1. `AT-HANDOFF.json` schema `humanymous.root-interactive-at-windows-tuple/1`
2. `AUTH-HANDOFF.json` schema `humanymous.root-n06-owner-join-auth-handoff/1`
3. Unlock C rematch LIVE `preimageSha256` (6 digests unchanged) **or** NEW non-twin GRANT→TRANSFER
4. Cap realloc that updates Unlock A CLOSER `cap_free=true` (probe authority)

## Next closer

Root AT+AUTH + NEW non-twin Cap-ready matching LIVE preimages + Unlock A CLOSER free flip. Cap burn=0. goal_complete=false.
