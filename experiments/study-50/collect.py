#!/usr/bin/env python3
"""Study analyzer: does UNI predict human acceptance better than the agent's
own "DONE"?

Metric priority (product stance): a FALSE ACCEPT is the catastrophe, so
False Acceptance Rate is primary; False Rejection Rate is secondary; cost and
latency come last. The agent baseline is exact and needs no modelling: coding
agents self-report DONE, so the agent accepts 100% of tasks by construction.

Usage:
  python3 collect.py results.csv [--json]

CSV columns (extra columns are ignored; human_review empty = not yet reviewed):
  issue, agent_done, uni_decision, human_review, claims_total, claims_verified
  optional: agent_seconds, agent_cost_usd
"""
import csv
import json
import sys

ACCEPT = "ACCEPTED"


def load(path):
    with open(path, newline="") as f:
        return list(csv.DictReader(f))


def rate(num, den):
    return (num / den) if den else None


def fmt(v):
    return "n/a" if v is None else f"{v:.0%}"


def analyze(rows):
    reviewed = [r for r in rows if r.get("human_review") in ("0", "1")]
    unreviewed = len(rows) - len(reviewed)
    n = len(reviewed)

    uni_accept = [r for r in reviewed if r["uni_decision"].upper() == ACCEPT]
    ta = sum(1 for r in uni_accept if r["human_review"] == "1")
    fa = sum(1 for r in uni_accept if r["human_review"] == "0")
    uni_reject = [r for r in reviewed if r["uni_decision"].upper() != ACCEPT]
    fr = sum(1 for r in uni_reject if r["human_review"] == "1")
    tr = sum(1 for r in uni_reject if r["human_review"] == "0")

    human_accept = ta + fr
    human_reject = fa + tr

    decisions = {}
    for r in rows:
        decisions[r["uni_decision"]] = decisions.get(r["uni_decision"], 0) + 1

    total_claims = sum(int(r["claims_total"] or 0) for r in rows)
    tested = sum(int(r["claims_verified"] or 0) for r in rows)

    seconds = [float(r["agent_seconds"]) for r in rows if r.get("agent_seconds")]
    cost = [float(r["agent_cost_usd"]) for r in rows if r.get("agent_cost_usd")]

    return {
        "n_rows": len(rows),
        "n_reviewed": n,
        "n_unreviewed": unreviewed,
        "matrix": {"true_accept": ta, "false_accept": fa, "false_reject": fr, "true_reject": tr},
        "human_accept": human_accept,
        "human_reject": human_reject,
        "uni_accept": len(uni_accept),
        "uni_reject": len(uni_reject),
        "false_acceptance_rate": rate(fa, n),
        "false_acceptance_rate_vs_human_rejects": rate(fa, human_reject),
        "false_rejection_rate": rate(fr, human_accept),
        "precision": rate(ta, ta + fa),
        "recall": rate(ta, ta + fr),
        # Agent baseline = trust every task the agent itself claimed DONE on.
        "agent_accepts": sum(1 for r in reviewed if r.get("agent_done") == "1"),
        "agent_matrix": {
            "true_accept": sum(1 for r in reviewed if r.get("agent_done") == "1" and r["human_review"] == "1"),
            "false_accept": sum(1 for r in reviewed if r.get("agent_done") == "1" and r["human_review"] == "0"),
        },
        "agent_false_acceptance_rate": rate(
            sum(1 for r in reviewed if r.get("agent_done") == "1" and r["human_review"] == "0"),
            max(1, sum(1 for r in reviewed if r.get("agent_done") == "1")),
        ),
        "decisions": decisions,
        "evidence_coverage": rate(tested, total_claims),
        "claims_verified": tested,
        "claims_total": total_claims,
        "agent_seconds_total": sum(seconds) if seconds else None,
        "agent_cost_usd_total": sum(cost) if cost else None,
        "cost_per_true_accept": rate(sum(cost), ta) if cost and ta else None,
    }


def report(a):
    m = a["matrix"]
    print(f"rows={a['n_rows']}  reviewed={a['n_reviewed']}  unreviewed={a['n_unreviewed']}")
    print("")
    print("                 human accepts   human rejects")
    print(f"UNI accepts      {m['true_accept']:>4} (TA)       {m['false_accept']:>4} (FA)  <- catastrophe")
    print(f"UNI rejects      {m['false_reject']:>4} (FR)       {m['true_reject']:>4} (TR)")
    print("")
    print(f"FALSE ACCEPTANCE RATE  {fmt(a['false_acceptance_rate'])}   (primary; must be 0)")
    print(f"  of human rejects     {fmt(a['false_acceptance_rate_vs_human_rejects'])}")
    print(f"FALSE REJECTION RATE   {fmt(a['false_rejection_rate'])}   (secondary; tolerable early)")
    print(f"precision              {fmt(a['precision'])}")
    print(f"recall                 {fmt(a['recall'])}")
    print("")
    claimed = a["agent_accepts"]
    print(f"AGENT BASELINE (claimed DONE on {claimed}/{a['n_reviewed']}): "
          f"FA={a['agent_matrix']['false_accept']}  FAR={fmt(a['agent_false_acceptance_rate'])}")
    print("")
    print(f"decision mix           {a['decisions']}")
    print(f"evidence coverage      {fmt(a['evidence_coverage'])} "
          f"({a['claims_verified']}/{a['claims_total']} claims)")
    if a["agent_seconds_total"] is not None:
        print(f"agent time (total)     {a['agent_seconds_total']:.0f}s")
    if a["agent_cost_usd_total"] is not None:
        print(f"agent cost (total)     ${a['agent_cost_usd_total']:.4f}")
        print(f"cost per true accept   ${a['cost_per_true_accept']:.4f}" if a["cost_per_true_accept"] else
              "cost per true accept   n/a (no true accepts)")
    if a["n_unreviewed"]:
        print("")
        print(f"NOTE: {a['n_unreviewed']} row(s) excluded from the matrix (human_review empty)")


def main(path, as_json):
    rows = load(path)
    if not rows:
        print("no rows")
        return 0
    a = analyze(rows)
    if as_json:
        print(json.dumps(a, indent=2, sort_keys=True))
    else:
        report(a)
    # Exit non-zero when the release criterion is violated: useful in CI.
    return 1 if (a["matrix"]["false_accept"] > 0) else 0


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    sys.exit(main(args[0] if args else "results.csv", "--json" in sys.argv))
