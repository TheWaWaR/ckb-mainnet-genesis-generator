use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use ckb_chain_spec::ChainSpec;
use ckb_jsonrpc_types::BlockNumber;
use ckb_resource::Resource;
use ckb_sdk::{Address, HttpRpcClient, NetworkType, OldAddress};
use ckb_types::{
    core::{ScriptHashType, TransactionView},
    h256, packed,
    prelude::*,
    H160, H256,
};
use clap::{App, AppSettings, Arg};
use serde::{Deserialize, Serialize};

mod config;

const SECP_TYPE_SCRIPT_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");

fn main() {
    env_logger::init();
    let matches = App::new("CKB main net chain spec generator")
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("root")
                .long("root")
                .short("R")
                .takes_value(true)
                .required(true)
                .help("Path in config relative path root"),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("C")
                .takes_value(true)
                .required(true)
                .help("Config file"),
        )
        .arg(
            Arg::with_name("target-block-number")
                .long("target-block-number")
                .short("N")
                .takes_value(true)
                .required(true)
                .validator(|input| {
                    input
                        .parse::<u64>()
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .help("Target testnet block number"),
        )
        .get_matches();

    let root_path = matches.value_of("root").unwrap();
    let config_path = matches.value_of("config").unwrap();
    let target_block_number: u64 = matches
        .value_of("target-block-number")
        .unwrap()
        .parse()
        .unwrap();

    let config = config::Config::from_path(config_path);
    println!("Config: {:?}", config);
    println!("==============================");

    let base_spec_path: PathBuf = [root_path, config.base_spec.as_str()].iter().collect();
    let base_spec = read_base_chain_spec(base_spec_path);
    let testnet_cache_path: Option<PathBuf> = config
        .testnet_rewards
        .round5
        .cache
        .clone()
        .map(|filename| [root_path, filename.as_str()].iter().collect());
    let current_testnet_result = read_current_testnet(
        config.testnet_rewards.round5.url.as_str(),
        target_block_number,
        testnet_cache_path,
    );
    println!("CurrentTestnetResult: {}", current_testnet_result);
    let round1_path: PathBuf = [root_path, config.testnet_rewards.round1.as_str()]
        .iter()
        .collect();
    let round1_rewards = read_round1_rewards(round1_path);
    // println!("base_spec: {:?}", base_spec);
    // println!("prev_rewards: {:?}", prev_rewards);
}

fn read_base_chain_spec(path: PathBuf) -> ChainSpec {
    let res = Resource::file_system(path);
    ChainSpec::load_from(&res).expect("load spec by name")
}

fn read_round1_rewards(path: PathBuf) -> Vec<(H160, u64)> {
    let file = fs::File::open(path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let mut results = Vec::new();
    for record in rdr.records() {
        let record = record.unwrap();
        let address_str = record.get(0).unwrap();
        if address_str.is_empty() {
            log::error!("empty address lock_hash={}", record.get(1).unwrap());
            continue;
        }
        match AddressParser.parse(address_str) {
            Ok(hash) => {
                let capacity: u64 = record.get(2).unwrap().parse().unwrap();
                log::debug!("{:#} => {}", hash, capacity);
                results.push((hash, capacity));
            }
            Err(err) => {
                log::error!("invalid address {}, error: {}", address_str, err);
            }
        }
    }
    results
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentTestnetResult {
    pub rewards: Vec<(H160, u64)>,
    pub total_base_reward: u64,
    pub last_block_hash: H256,
    pub last_block_number: u64,
}

impl CurrentTestnetResult {
    pub fn new(
        rewards: HashMap<H160, u64>,
        total_base_reward: u64,
        last_block_hash: H256,
        last_block_number: u64,
    ) -> Self {
        let rewards: Vec<(H160, u64)> = rewards.into_iter().collect();
        CurrentTestnetResult {
            rewards,
            total_base_reward,
            last_block_hash,
            last_block_number,
        }
    }

    pub fn map(&self) -> HashMap<H160, u64> {
        self.rewards.iter().cloned().collect()
    }
}

impl fmt::Display for CurrentTestnetResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{{")?;
        writeln!(f, "  total_base_reward: {}", self.total_base_reward)?;
        writeln!(f, "  last_block_hash: {:#}", self.last_block_hash)?;
        for (lock_arg, reward) in &self.rewards {
            writeln!(f, "  > lock_arg: {:#}, reward: {}", lock_arg, reward)?;
        }
        writeln!(f, "}}")
    }
}

fn read_current_testnet(
    url: &str,
    block_number: u64,
    cache_path: Option<PathBuf>,
) -> CurrentTestnetResult {
    let mut client = HttpRpcClient::from_uri(url);
    let (mut rewards, mut last_block_hash, mut last_block_number, mut total_base_reward) =
        match cache_path.as_ref() {
            Some(path) if Path::new(path).exists() => {
                let file = fs::File::open(path).unwrap();
                let result: CurrentTestnetResult = serde_json::from_reader(file).unwrap();
                (
                    result.map(),
                    result.last_block_hash.clone(),
                    result.last_block_number,
                    result.total_base_reward,
                )
            }
            _ => (HashMap::default(), H256::default(), 0, 0),
        };

    fn get_tip_block_number(client: &mut HttpRpcClient) -> u64 {
        client.get_tip_block_number().call().unwrap().value()
    }
    let mut tip_number = get_tip_block_number(&mut client);
    log::info!(
        "tip: {}, last_block_number: {}, last_block_hash: {:#}, total_base_reward: {}",
        tip_number,
        last_block_number,
        last_block_hash,
        total_base_reward,
    );

    for number in (last_block_number + 1)..=block_number {
        // Wait for next block
        let mut check_round = 0;
        loop {
            if number > tip_number {
                if check_round % 20 == 0 {
                    log::info!("Wait for next block: {}", number);
                }
                thread::sleep(Duration::from_millis(100));
                tip_number = get_tip_block_number(&mut client);
                check_round += 1;
            } else {
                break;
            }
        }

        if number % 1000 == 0 {
            log::info!("block: {}", number);
        } else {
            log::debug!("block: {}", number);
        }
        let block = client
            .get_block_by_number(BlockNumber::from(number))
            .call()
            .unwrap()
            .0
            .unwrap();
        let block_hash = &block.header.hash;
        if number == block_number {
            last_block_hash = block_hash.clone();
            last_block_number = number;
        }
        let cellbase: packed::Transaction = block.transactions[0].clone().inner.into();
        let lock_script = cellbase
            .into_view()
            .witnesses()
            .get(0)
            .map(|data| packed::CellbaseWitness::from_slice(&data.raw_data()).unwrap())
            .unwrap()
            .lock();
        if lock_script.code_hash() == SECP_TYPE_SCRIPT_HASH.pack()
            && lock_script.hash_type().unpack() == ScriptHashType::Type
            && lock_script.args().raw_data().len() == 20
        {
            let lock_arg = H160::from_slice(&lock_script.args().raw_data()).unwrap();
            let base_reward: u64 = client
                .get_cellbase_output_capacity_details(block_hash.clone())
                .call()
                .unwrap()
                .0
                .unwrap()
                .primary
                .value();
            total_base_reward += base_reward;
            *rewards.entry(lock_arg).or_default() += base_reward;
        } else {
            log::warn!(
                "Invalid lock script: {}, block number: {}",
                lock_script,
                number
            );
        }

        if number % 2000 == 0 && tip_number - number > 300 {
            if let Some(path) = cache_path.as_ref() {
                let file = fs::File::create(path).unwrap();
                let result = CurrentTestnetResult::new(
                    rewards.clone(),
                    total_base_reward,
                    block_hash.clone(),
                    number,
                );
                serde_json::to_writer(file, &result).unwrap();
                log::info!(
                    "Current block number: {}, save cache to: {:?}",
                    number,
                    path
                );
            }
        }
    }
    CurrentTestnetResult::new(
        rewards,
        total_base_reward,
        last_block_hash,
        last_block_number,
    )
}

pub struct AddressParser;
impl AddressParser {
    fn parse(&self, input: &str) -> Result<H160, String> {
        if let Ok((_network, address)) = Address::from_input(input) {
            return Ok(address.hash().clone());
        }

        let prefix = input.chars().take(3).collect::<String>();
        let network = NetworkType::from_prefix(prefix.as_str())
            .ok_or_else(|| format!("Invalid address prefix: {}", prefix))?;
        let old_address = OldAddress::from_input(network, input)?;
        Ok(old_address.hash().clone())
    }
}
