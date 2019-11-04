use ckb_chain_spec::ChainSpec;
use ckb_types::{H160, H256};
use clap::{App, AppSettings, Arg};
use std::collections::HashMap;

mod consts;
mod data;
mod last_round;
mod other;
mod previous_rounds;

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

    let base_spec: ChainSpec = toml::from_slice(data::CHAIN_CHAIN_SPEC.as_bytes()).unwrap();
    println!(
        "==== base_spec ====: \n{}\n",
        toml::to_string_pretty(&base_spec).unwrap()
    );

    let previous_rewards = previous_rounds::all_rewards();
    let current_testnet_result = last_round::read_last_round(testnet_rpc_server, confirmations);
    let last_rewards = current_testnet_result.real_rewards();
    println!("CurrentTestnetResult: {}", current_testnet_result);

    let sighash_all_records = other::read_sighash_all_records();
    let multisig_all_records = other::read_multisig_all_records();

    // ==== Summary sighash-all records
    let sighash_all_map: HashMap<H160, u64> = HashMap::default();
    for (lock_arg, capacity) in previous_rewards
        .into_iter()
        .chain(last_rewards.into_iter())
        .chain(sighash_all_records.into_iter())
    {}

    // ==== Summary multihash-all records
    let multisig_all_map: HashMap<(H160, u128), u64> = HashMap::default();
    for (lock_arg, since, capacity) in multisig_all_records {}

    // ==== Put records into spec
}
