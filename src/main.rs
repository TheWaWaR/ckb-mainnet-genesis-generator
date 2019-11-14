use ckb_chain_spec::{ChainSpec, IssuedCell};
use basic::{Address, NetworkType, OldAddress};
use ckb_types::{bytes::Bytes, core::{Capacity, ScriptHashType}, packed, prelude::*, H160, H256, core::EpochNumberWithFraction};
use clap::{App, AppSettings, Arg};
use ckb_hash::blake2b_256;
use std::cmp::Ordering;
use std::collections::HashMap;
use chrono::prelude::*;
use std::fs;
use std::io::{Read, Write};

mod consts;
mod data;
mod last_round;
mod genesis_final;
mod previous_rounds;
mod basic;
mod client;

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
            Arg::with_name("last-epoch")
                .long("last-epoch")
                .short("E")
                .takes_value(true)
                .default_value("89")
                .help("Last epoch number")
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
    let last_epoch = matches.value_of("last-epoch").unwrap().parse::<u64>().unwrap();
    let confirmations: u16 = matches.value_of("confirmations").unwrap().parse().unwrap();

    {
        let mut boyu_file = fs::File::open("boyu-spec.toml").unwrap();
        let mut boyu_content = String::new();
        boyu_file.read_to_string(&mut boyu_content).unwrap();
        let boyu_spec: ChainSpec = toml::from_str(boyu_content.as_str()).unwrap();
        drop(boyu_file);
        let mut boyu_file = fs::File::create("boyu-spec-new.toml").unwrap();
        boyu_file.write_all(toml::to_string_pretty(&boyu_spec).unwrap().as_bytes()).unwrap();
    }

    let mut spec: ChainSpec = toml::from_str(data::CHAIN_CHAIN_SPEC).unwrap();

    // == Testnet rewards
    let testnet_result = previous_rounds::all_rewards(testnet_rpc_server, last_epoch, confirmations);
    // == Other records
    let genesis_final_records = genesis_final::read_all_records(last_epoch);

    for (lock_script, capacity) in genesis_final_records {
        spec.genesis.issued_cells.push(IssuedCell {
            capacity: Capacity::shannons(capacity),
            lock: lock_script.into(),
        });
    }

    {
        let addr = Address::from_input(crate::consts::FOUNDATION_RESERVE_ADDR).unwrap().1;
        let lock_arg: Bytes = build_multisig_lock_arg(
            addr,
            crate::consts::FOUNDATION_RESERVE_LOCK_TIME,
            last_epoch,
        ).into();
        let lock_script = packed::Script::new_builder()
            .code_hash(crate::consts::MULTISIG_TYPE_SCRIPT_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(lock_arg.pack())
            .build();
        spec.genesis.issued_cells.push(IssuedCell {
            capacity: Capacity::shannons(crate::consts::FOUNDATION_RESERVE),
            lock: lock_script.into(),
        });
    }
    // == Put testnet records into spec
    for (lock_arg, capacity) in testnet_result.rewards.iter().cloned() {
        let lock_script = packed::Script::new_builder()
            .code_hash(consts::SECP_TYPE_SCRIPT_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(lock_arg.pack())
            .build();
        spec.genesis.issued_cells.push(IssuedCell {
            capacity: Capacity::shannons(capacity),
            lock: lock_script.into(),
        });
    }

    spec.genesis.timestamp = testnet_result.last_timestamp;
    spec.genesis.genesis_cell.message = format!("lina {:#x}", testnet_result.last_block_hash);
    spec.genesis.compact_target = testnet_result.mainnet_difficulty;
    spec.params.genesis_epoch_length = testnet_result.last_epoch_length;
    println!(">> timestamp: {}", spec.genesis.timestamp);
    println!(">> message: {}", spec.genesis.genesis_cell.message);
    println!(">> compact_target: {:#x}", spec.genesis.compact_target);
    println!(">> genesis_epoch_length: {:#x}", spec.params.genesis_epoch_length);

    let consensus = spec.build_consensus().unwrap();

    // println!(
    //     "==== spec ====: \n{}\n",
    //     toml::to_string_pretty(&spec).unwrap()
    // );
    let mut file = fs::File::create("final-spec.toml").unwrap();
    file.write_all(toml::to_string_pretty(&spec).unwrap().as_bytes()).unwrap();

    let mut total_capacity = 0u64;
    for output in consensus.genesis_block().transactions()[0].outputs().into_iter() {
        let capacity: u64 = output.capacity().unpack();
        total_capacity += capacity;
    }
    println!("genesis hash: {:?}, total-capacity: {}", consensus.genesis_hash(), total_capacity);
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

pub fn build_multisig_lock_arg(address: Address, datetime_str: &str, last_epoch: u64) -> Vec<u8> {
    let datetime_string = if datetime_str.len() == 10 {
        format!("{}{}", datetime_str, crate::consts::DEFAULT_TIME_SUFFIX)
    } else {
        datetime_str.to_string()
    };
    let since_begin = DateTime::parse_from_rfc3339(crate::consts::SINCE_BEGIN).unwrap();
    let datetime = DateTime::parse_from_rfc3339(datetime_string.as_str()).unwrap();
    let seconds = if datetime <= since_begin {
        0
    } else {
        (datetime - since_begin).num_seconds() as u64
    };

    // number
    let se = seconds / 14400 + 89 - last_epoch;
    // index
    let sn = (seconds % 14400) * 1800 / 14400;
    // length
    let sd = 1800;
    let sf = 32;
    let epoch = EpochNumberWithFraction::new(se, sn, sd);
    let since = 0x2000_0000_0000_0000 | epoch.full_value();

    let mut data = {
        let mut buf = vec![0, 0, 1, 1];
        buf.extend_from_slice(address.hash().as_bytes());
        blake2b_256(&buf)[..20].to_vec()
    };
    data.extend(since.to_le_bytes().iter());
    data
}
