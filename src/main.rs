use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{Read, Write};
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

fn main() {
    env_logger::init();
    let matches = App::new("CKB main net chain spec generator")
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("base-spec")
                .long("base-spec")
                .short("B")
                .takes_value(true)
                .required(true)
                .help("Base chain spec config file"),
        )
        .arg(
            Arg::with_name("prev-testnet-rewards")
                .long("prev-testnet-rewards")
                .short("P")
                .takes_value(true)
                .required(true)
                .help("Previous rounds testnet rewards config file"),
        )
        .arg(
            Arg::with_name("other-rewards")
                .long("other-rewards")
                .short("O")
                .takes_value(true)
                .required(true)
                .help("Other rewards"),
        )
        .arg(
            Arg::with_name("current-testnet")
                .long("current-testnet")
                .short("T")
                .takes_value(true)
                .required(true)
                .default_value("http://127.0.0.1:8114")
                .help("Current testnet RPC url"),
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

    let base_spec = matches.value_of("base-spec").unwrap();
    let prev_testnet_rewards = matches.value_of("prev-testnet-rewards").unwrap();
    let other_rewards = matches.value_of("other-rewards").unwrap();
    let current_testnet = matches.value_of("current-testnet").unwrap();
    let target_block_number: u64 = matches
        .value_of("target-block-number")
        .unwrap()
        .parse()
        .unwrap();
    println!("base-spec: {}", base_spec);
    println!("prev-testnet-rewards: {}", prev_testnet_rewards);
    println!("other-rewards: {}", other_rewards);
    println!("current-testnet: {}", current_testnet);
    println!("target-block-number: {}", target_block_number);

    println!("==============================");

    let base_spec = read_base_chain_spec(base_spec);
    let current_testnet_result = read_current_testnet(current_testnet, target_block_number);
    println!("CurrentTestnetResult: {}", current_testnet_result);
    let prev_rewards = read_prev_rewards(prev_testnet_rewards);
    // println!("base_spec: {:?}", base_spec);
    // println!("prev_rewards: {:?}", prev_rewards);
}

fn read_base_chain_spec(path: &str) -> ChainSpec {
    let res = Resource::file_system(path.into());
    ChainSpec::load_from(&res).expect("load spec by name")
}

fn read_prev_rewards(path: &str) -> Vec<(H160, u64)> {
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

#[derive(Debug)]
pub struct CurrentTestnetResult {
    pub rewards: HashMap<H160, u64>,
    pub total_base_reward: u64,
    pub last_block_hash: H256,
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

fn read_current_testnet(url: &str, block_number: u64) -> CurrentTestnetResult {
    let mut client = HttpRpcClient::from_uri(url);
    let mut rewards = HashMap::default();
    let mut last_block_hash = H256::default();
    let mut total_base_reward: u64 = 0;
    let secp_type_script_hash =
        h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");

    fn get_tip_block_number(client: &mut HttpRpcClient) -> u64 {
        client.get_tip_block_number().call().unwrap().value()
    }
    let mut tip_number = get_tip_block_number(&mut client);

    for number in 1..=block_number {
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
        }
        let cellbase: packed::Transaction = block.transactions[0].clone().inner.into();
        let lock_script = cellbase
            .into_view()
            .witnesses()
            .get(0)
            .map(|data| packed::CellbaseWitness::from_slice(&data.raw_data()).unwrap())
            .unwrap()
            .lock();
        if lock_script.code_hash() == secp_type_script_hash.pack()
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
    }
    CurrentTestnetResult {
        rewards,
        total_base_reward,
        last_block_hash,
    }
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
