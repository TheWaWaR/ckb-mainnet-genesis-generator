use ckb_chain_spec::{ChainSpec, IssuedCell};
use ckb_sdk::{Address, NetworkType, OldAddress};
use ckb_types::{bytes::Bytes, core::Capacity, packed, prelude::*, H160, H256};
use clap::{App, AppSettings, Arg};
use std::cmp::Ordering;
use std::collections::HashMap;

mod consts;
mod data;
mod last_round;
mod other;
mod previous_rounds;

// TODO Tasks:
//   [ ] read other sighash_all_records
//   [ ] read other multisig_all_records
//   [x] spec: difficulty, timestamp
//   [ ] The rest cell for fundation cell

//  TODO Checks:
//   [ ] Check Genesis Allocation => 33.6
//   [ ] Output all addresses' capacity for check

fn main() {
    env_logger::init();
    let matches = App::new("CKB main net chain spec generator")
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("testnet-rpc-server")
                .long("testnet-rpc-server")
                .short("S")
                .takes_value(true)
                .required(true)
                .default_value("http://127.0.0.1:8114")
                .help("Current testnet rpc server"),
        )
        .arg(
            Arg::with_name("confirmations")
                .long("confirmations")
                .short("C")
                .takes_value(true)
                .required(true)
                .default_value("20")
                .validator(|input| {
                    input
                        .parse::<u16>()
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .help("Confirmations of block for security"),
        )
        .get_matches();

    let testnet_rpc_server = matches.value_of("testnet-rpc-server").unwrap();
    let confirmations: u16 = matches.value_of("confirmations").unwrap().parse().unwrap();

    let mut spec: ChainSpec = toml::from_slice(data::CHAIN_CHAIN_SPEC.as_bytes()).unwrap();
    println!(
        "==== base spec ====: \n{}\n",
        toml::to_string_pretty(&spec).unwrap()
    );

    // ==== Testnet rewards
    let (testnet_rewards, timestamp, difficulty) = {
        let previous_rewards = previous_rounds::all_rewards();
        let current_testnet_result = last_round::read_last_round(testnet_rpc_server, confirmations);
        let last_rewards = current_testnet_result.real_rewards();
        println!("CurrentTestnetResult: {}", current_testnet_result);
        let mut rewards = Vec::new();
        rewards.extend(previous_rewards);
        rewards.extend(last_rewards);
        (
            rewards,
            current_testnet_result.last_timestamp,
            current_testnet_result.avg_difficulty,
        )
    };

    // === Other records
    let sighash_all_records = other::read_sighash_all_records();
    let multisig_all_records = other::read_multisig_all_records();

    // ==== Summary sighash-all records
    let mut sighash_all_map: HashMap<H160, u64> = HashMap::default();
    for (lock_arg, capacity) in testnet_rewards
        .into_iter()
        .chain(sighash_all_records.into_iter())
    {
        *sighash_all_map.entry(lock_arg).or_default() += capacity;
    }
    let mut sighash_all_vec: Vec<_> = sighash_all_map.into_iter().collect();
    sighash_all_vec.sort_by(|a, b| a.0.cmp(&b.0));

    // ==== Summary multihash-all records
    let mut multisig_all_map: HashMap<(H160, u128), u64> = HashMap::default();
    for (lock_arg, since, capacity) in multisig_all_records {
        *multisig_all_map.entry((lock_arg, since)).or_default() += capacity;
    }
    let mut multisig_all_vec: Vec<_> = multisig_all_map.into_iter().collect();
    multisig_all_vec.sort_by(|a, b| match (a.0).0.cmp(&(b.0).0) {
        Ordering::Equal => (a.0).1.cmp(&(b.0).1),
        result => result,
    });

    // ==== Put records into spec
    for issued_cell in sighash_all_vec.into_iter().map(|(lock_arg, capacity)| {
        let lock_script = packed::Script::new_builder()
            .code_hash(consts::SECP_TYPE_SCRIPT_HASH.pack())
            .args(Bytes::from(lock_arg.as_bytes()).pack())
            .build();
        IssuedCell {
            capacity: Capacity::shannons(capacity),
            lock: lock_script.into(),
        }
    }) {
        spec.genesis.issued_cells.push(issued_cell);
    }

    for issued_cell in multisig_all_vec
        .into_iter()
        .map(|((lock_arg, since), capacity)| {
            let mut args_data = lock_arg.as_bytes().to_vec();
            args_data.extend(since.to_le_bytes().iter());
            let lock_script = packed::Script::new_builder()
                .code_hash(consts::SECP_TYPE_SCRIPT_HASH.pack())
                .args(Bytes::from(args_data).pack())
                .build();
            IssuedCell {
                capacity: Capacity::shannons(capacity),
                lock: lock_script.into(),
            }
        })
    {
        spec.genesis.issued_cells.push(issued_cell);
    }

    println!(
        "==== final spec ====: \n{}\n",
        toml::to_string_pretty(&spec).unwrap()
    );
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
