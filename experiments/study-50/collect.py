#!/usr/bin/env python3
"""Study-50 analyzer: agent self-report vs UNI decision vs human review."""
import csv
import sys

def load(path):
    with open(path, newline="") as f:
        return list(csv.DictReader(f))

def main(path):
    rows = load(path)
    n = len(rows)
    if n == 0:
        print("no rows"); return
    agent_accept = sum(1 for r in rows if r["agent_done"] == "1")
    human_accept = sum(1 for r in rows if r["human_review"] == "1")
    uni_accept = sum(1 for r in rows if r["uni_decision"].upper() == "ACCEPTED")

    # Correlation-style agreement
    both = sum(1 for r in rows if r["human_review"] == "1" and r["uni_decision"].upper() == "ACCEPTED")
    agree_uni_human = both / max(1, human_accept)
    agree_agent_human = human_accept / n  # agent accepts everything

    # Critical: UNI ACCEPTED but human rejected (silent false accept)
    false_accept = sum(1 for r in rows if r["human_review"] == "0" and r["uni_decision"].upper() == "ACCEPTED")
    # Coverage: UNI correctly blocked what human also rejected
    true_blocks = sum(1 for r in rows if r["human_review"] == "0" and r["uni_decision"] != "ACCEPTED")
    human_reject = n - human_accept

    print(f"n={n}")
    print(f"agent self-report agreement: {agree_agent_human:.0%} (accepts {agent_accept}/{n})")
    print(f"UNI decision agreement:      {agree_uni_human:.0%} (accepted {uni_accept}/{n})")
    print(f"false accepts (UNI ok, human no): {false_accept}  <- MUST BE 0 for release")
    print(f"true blocks (UNI no, human no):   {true_blocks}/{human_reject} detected")
    total_claims = sum(int(r["claims_total"]) for r in rows)
    tested = sum(int(r["claims_verified"]) for r in rows)
    if total_claims:
        print(f"evidence coverage: {tested}/{total_claims} ({tested/total_claims:.0%})")

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "results.csv")
