use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub main: Main,
}

#[derive(Deserialize)]
pub struct Main {
    pub server: String,
    pub force_insecure: bool,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();
