use std::io::{Read, Write};
use std::fs;
use std::str::FromStr;

use clap::{Arg, App, AppSettings};
use ckb_sdk::{
    Address, OldAddress, NetworkType,
};
use ckb_types::{
    H160, H256,
};
use ckb_chain_spec::{
    ChainSpec,
};
use ckb_resource::Resource;

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
                .help("Base chain spec config file")
        )
        .arg(
            Arg::with_name("prev-testnet-rewards")
                .long("prev-testnet-rewards")
                .short("P")
                .takes_value(true)
                .required(true)
                .help("Previous rounds testnet rewards config file")
        )
        .arg(
            Arg::with_name("other-rewards")
                .long("other-rewards")
                .short("O")
                .takes_value(true)
                .required(true)
                .help("Other rewards")
        )
        .arg(
            Arg::with_name("current-testnet")
                .long("current-testnet")
                .short("T")
                .takes_value(true)
                .required(true)
                .help("Current testnet RPC url")
        )
        .arg(
            Arg::with_name("target-block-number")
                .long("target-block-number")
                .short("N")
                .takes_value(true)
                .required(true)
                .help("Target testnet block number")
        )
        .get_matches();

    let base_spec = matches.value_of("base-spec").unwrap();
    let prev_testnet_rewards = matches.value_of("prev-testnet-rewards").unwrap();
    let other_rewards = matches.value_of("other-rewards").unwrap();
    let current_testnet = matches.value_of("current-testnet").unwrap();
    let target_block_number = matches.value_of("target-block-number").unwrap();
    println!("base-spec: {}", base_spec);
    println!("prev-testnet-rewards: {}", prev_testnet_rewards);
    println!("other-rewards: {}", other_rewards);
    println!("current-testnet: {}", current_testnet);
    println!("target-block-number: {}", current_testnet);

    println!("==============================");

    let base_spec = read_base_chain_spec(base_spec);
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
                log::info!("{:#} => {}", hash, capacity);
                results.push((hash, capacity));
            }
            Err(err) => {
                log::error!("invalid address {}, error: {}", address_str, err);
            }
        }
    }
    results
}

struct RewardTotal {
    pubkey_hash: H160,
    capacity: u64,
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
