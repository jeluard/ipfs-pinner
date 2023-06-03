pub mod hyper;
pub mod polkadot;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
