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
