# UNI v0.1 — Vertical tracer bullets (Matt to-tickets)

- [x] T1 — First end-to-end outcome (hello.uni, cargo check → ACCEPTED). Blocked by: none. DONE (dogfood on).
- [x] T2 — Explainability (`uni explain` human + --json). Blocked by: T1. DONE.
- [x] T3 — Git-bound evidence (SHA + dirty + SHA-256 → STALE). Blocked by: T2. DONE.
- [x] T4 — Multiple claims + invariants (multi.uni: CLAIM ×2 + INVARIANT CRITICAL). Blocked by: T3. DONE.
- [x] T5 — Real software oracle (booking.cancel, 3 claims incl. ledger invariant). Blocked by: T4. DONE.
- [x] T6 — Playwright pipeline entry (browser.uni + registry `e2e.smoke`). Blocked by: T5. DONE (smoke verifier wired; real Playwright run is per-project).
- [x] T7 — GitHub CI (`.github/workflows/uni.yml` + `adapters/github/action.yml`). Blocked by: T6. DONE.
- [x] P5 — SpecKit importer minimal (`uni import-speckit` → candidate JSON, review required). DONE.
- [x] v0.2a — Proptest determinism + truth table + critical rejection, exhaustive decision matrix, parser fixtures. DONE.
- [x] v0.2b — Persisted-evidence pipeline (re-run only when stale/missing/invalid), STALE-never-masks-new-commit e2e git fixture, registry security tests (inline shell refused, unknown ref refused), CLI golden tests. DONE. Next: decision-matrix CLI report, then 50-issue study.
- [x] v0.3a — `uni explain` enriched report (claims table, summary 3/3, Assurance A0-A4, per-claim evidence detail, match filter), PRD §16 error UX (`Run: uni verify <claim>`), negative example + study-50 harness (collect.py, false-accept release criterion). DONE. Next: study-50 real data, then v0.4 agent comparison.
- [x] v0.4a — study-50 execution harness (run.py fixture/codex/claude agents, per-test verifier registry with expect matcher, stdout/stderr separation), smoke: t01 ACCEPTED 4/4, t02 partial fix blocked. Registry expressiveness caught the single-verifier-for-all-claims hole. DONE. Next: real 50-issue data + human review column, then cost per accepted outcome.
