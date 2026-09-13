# Plan - 002 expect_not

- VerifierSpec gains expect_not: String. Parsed from table field like expect, default "".
- run_spec: if !spec.expect_not.is_empty() && excerpt.contains(expect_not) → Invalid.
- Upsert: expec and expect_not are AND-verifiers when both set.
- Fixture: examples/forbid/forbid.uni with no-evil registry (! grep "EVIL" examples/forbid/sources) and dirty-tree scenario test.
tasks → uni-cli tests green + dogfood.
