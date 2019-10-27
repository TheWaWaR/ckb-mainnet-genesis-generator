use std::fs;
use std::io::{Read, Write};

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub base_spec: String,
    pub other_tokens: String,
    pub testnet_rewards: ConfigTestnetRewards,
}

impl Config {
    pub fn from_path(path: &str) -> Self {
        let file = fs::File::open(path).unwrap();
        serde_json::from_reader(file).unwrap()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigTestnetRewards {
    pub round1: String,
    pub round2: ConfigLuckyEpoch,
    pub round3: ConfigLuckyEpoch,
    pub round4: String,
    pub round5: ConfigLastRound,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigLuckyEpoch {
    pub miner: String,
    pub epoch: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigLastRound {
    pub url: String,
    pub cache: Option<String>,
}
