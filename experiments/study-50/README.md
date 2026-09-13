# Experiment 1 — Agent-done vs UNI-accepted vs human review (PRD §20)

Protocol:
1. Pick N=50 real GitHub issues in a subject repo (start: uni itself or a Next.js example).
2. Let a coding agent implement each issue; record `agent_says_done` (always true).
3. Run UNI: record contract decision (ACCEPTED / REJECTED / EVIDENCE_REQUIRED).
4. A human reviewer reviews the resulting diff blind to UNI's decision; records accept/reject.
5. Run `collect.py results.csv`:

CSV header: `issue,agent_done,uni_decision,human_review,claims_total,claims_verified`

Metrics:
- Agreement: `uni ACCEPTED & human accept` correlation vs `agent done & human accept`.
- Never-silently-wrong: any case UNI ACCEPTED but human rejected = critical (blocks release).
- UNI REJECTED/EEDED & human rejected = true positive detection.

Columns meaning:
- agent_done: always 1 (self-report)
- human_review: 1 accept, 0 reject
- uni_decision: ACCEPTED|REJECTED|EVIDENCE_REQUIRED

Usage: python3 collect.py results.csv
