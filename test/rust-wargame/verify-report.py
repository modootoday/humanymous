"""Independently check the JSON ledger and optional byte-for-byte seed replay."""
import collections
import json
import pathlib
import sys

if not __debug__:
    raise SystemExit("report verification requires Python assertions enabled")


def verify(path):
    report = json.loads(path.read_text(encoding="utf-8-sig"))
    assert report["schema"] == "humanymous.wargame.v1"
    assert report["mode"] == "pure_engine_simulation"
    assert report["runtime_coverage_complete"] is False
    assert report["t4_ceiling"]
    assert type(report["seed"]) is int and 0 <= report["seed"] < 2**64
    rows = report["rows"]
    assert rows and len({row["id"] for row in rows}) == len(rows)
    keys = tuple(report["totals"])
    totals = collections.Counter(dict.fromkeys(keys, 0))
    by_class = {}
    by_tier = {}
    for row in rows:
        delta = collections.Counter(dict.fromkeys(keys, 0))
        delta["scheduled"] = 1
        status = row["status"]
        assert status in {"evaluated", "error", "skipped", "unsupported"}
        if status == "evaluated":
            delta["evaluated"] = 1
            assert row["input"] is not None
            actual = row["actual"]
            verdict = actual["result"]["verdict"]
            assert verdict in {"ALLOW", "CHALLENGE", "DENY"}
            assert actual["trace"]["verdict"] == verdict
            assert actual["report"]["scoring"] == actual["result"]
            assert actual["trace"]["score"] == actual["result"]["riskScore"]
            delta[verdict.lower()] = 1
            expected = row["expected"]
            assert isinstance(expected, dict) and expected
            matches = all(actual["result"].get(key) == value for key, value in expected.items())
            assert row["regression_matches"] is matches
            delta["regression_mismatches"] = int(not matches)
        else:
            delta[{"error": "errors", "skipped": "skipped", "unsupported": "unsupported"}[status]] = 1
            assert row["actual"] is None and row["regression_matches"] is None
            assert row["reason"]
        totals.update(delta)
        by_class.setdefault(row["class"], collections.Counter(dict.fromkeys(keys, 0))).update(delta)
        by_tier.setdefault(row["tier"], collections.Counter(dict.fromkeys(keys, 0))).update(delta)
    assert dict(totals) == report["totals"]
    assert {key: dict(value) for key, value in by_class.items()} == report["by_class"]
    assert {key: dict(value) for key, value in by_tier.items()} == report["by_tier"]
    assert totals["scheduled"] == sum(totals[key] for key in ("evaluated", "errors", "skipped", "unsupported"))
    assert totals["evaluated"] == totals["allow"] + totals["challenge"] + totals["deny"]
    assert totals["errors"] == totals["regression_mismatches"] == 0
    controls = [row for row in rows if row["class"] in {"human", "accessibility"}]
    controls_allow = bool(controls) and all(row["status"] == "evaluated" and row["actual"]["result"]["verdict"] == "ALLOW" for row in controls)
    assert report["human_controls_all_allow"] is controls_allow
    print(json.dumps({"verified": str(path), "totals": dict(totals)}, sort_keys=True))


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3):
        raise SystemExit("usage: verify-report.py REPORT [SAME-SEED-REPORT]")
    first = pathlib.Path(sys.argv[1])
    verify(first)
    if len(sys.argv) == 3:
        second = pathlib.Path(sys.argv[2])
        verify(second)
        assert first.read_bytes() == second.read_bytes(), "seed replay differs"
