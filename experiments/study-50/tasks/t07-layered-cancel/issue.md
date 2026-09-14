Issue t07: booking cancellation is wrong, and it is wrong at two levels.

Behaviour (src/service.rs, `cancel`):
1. cancelling an EXISTING booking must return `Ok(())` and set `cancelled = true`;
2. cancelling an UNKNOWN id must return `Err(...)` (today it always returns Ok).

Architecture rule (declared, must be respected):
3. `src/service.rs` must NOT touch the storage module directly. The service
   layer goes through `crate::repo`; only `src/repo.rs` may use `crate::store`
   internals. This rule is verified mechanically and is a CRITICAL invariant.

Add tests in tests/it.rs covering 1 and 2, keep the existing test passing, and
run `cargo test`.
