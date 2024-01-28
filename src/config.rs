use std::sync::OnceLock;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    main: Main,
}

#[derive(Deserialize)]
pub struct Main {
    server: String,
    force_insecure: bool,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();