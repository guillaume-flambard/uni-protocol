# Stale Evidence Benchmark

A structural experiment, not an agent study. It measures one thing:

> How often does ordinary verification keep saying green after the context the
> green was produced in has changed?

The agent study needed a model to misbehave; this does not. The drift is
manufactured, so the ground truth is known by construction and the numbers are
reproducible.

## Method

For each scenario: build a small repo with a contract, a trusted registry and a
green run. Then apply one drift. Then ask three oracles on the delivered
revision:

| Oracle | Model of |
|---|---|
| `B0 exit-code CI` | ordinary CI: run the commands, green when they exit 0 |
| `B1 cached CI` | a result cache, test-impact analysis, or a path-filtered workflow: reuse the previous green unless the tests themselves changed |
| `UNI` | `uni verify`: exit codes plus expected output, watched content, frozen files, and the whole verification context |

Ground truth is written by the scenario author, independently of UNI, and says
whether the delivered artifact still satisfies the claims.

## Metrics

- **UNI detection rate**: not-accepted when the artifact no longer satisfies the claims
- **UNI false-stale rate**: not-accepted while the artifact still satisfies them
- **B0 / B1 miss rate**: green while the artifact no longer satisfies them
- **boundary / freshness enforcement**: the run named a registry change or marked a proof stale

## Run

```bash
cargo build --release
python3 run.py            # writes results.csv and prints the tables
```
