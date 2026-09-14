# Drift demo

A tiny green repo used by the `stale evidence demo` workflow and by the docs.
It is not part of the test suite: it exists to be broken on camera.

1. `uni verify c.uni` on the committed revision: Accepted, both claims proved.
2. Edit `src/ledger.rs` so the watched bytes move.
3. `uni verify c.uni` again: Rejected, and `uni explain --annotations` names the
   file that moved.

The registry's `expect_sha256` is a placeholder (`REPLACED_AT_RUNTIME`); the
workflow fills in the reviewed hash before the first verify, the way a reviewer
would pin the file they read.
