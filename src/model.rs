use std::path::PathBuf;
use std::time::Duration;
use std::collections::HashMap;

use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Parser, Clone)]
pub struct Cli {
    #[clap(long, short, env, default_value = "/etc/auto-moderator.toml")]
    pub config: PathBuf,

    #[clap(long, short, env)]
    pub discord_token: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum Moderation {
    Disabled,
    Kick,
}

fn default_message_removal_range() -> Duration {
    Duration::from_mins(10)
}

fn default_message_removal_delay() -> Duration {
    Duration::from_secs(10)
}

fn default_moderation() -> Moderation {
    Moderation::Disabled
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(with = "humantime_serde", default = "default_message_removal_range")]
    pub message_removal_range: Duration,

    #[serde(with = "humantime_serde", default = "default_message_removal_delay")]
    pub message_removal_delay: Duration,

    pub traps: Vec<u64>,
    pub whitelist: Vec<u64>,

    #[serde(default = "default_moderation")]
    pub moderation: Moderation,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub servers: HashMap<u64, ServerConfig>,
}
