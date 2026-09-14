Issue t08: config port parsing is missing.

`parse_port` in src/config.rs currently always fails. Make it work: accept a
numeric port and reject anything that is not a valid port. Add tests in
tests/it.rs and run `cargo test`.
