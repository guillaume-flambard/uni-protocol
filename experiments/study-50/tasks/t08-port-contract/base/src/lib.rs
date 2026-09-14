pub mod config;

pub use config::parse_port;

pub fn version() -> &'static str {
    "0.1"
}
