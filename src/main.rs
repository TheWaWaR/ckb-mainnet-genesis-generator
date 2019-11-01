use ckb_chain_spec::ChainSpec;
use clap::{App, AppSettings, Arg};

mod consts;
mod data;
mod last_round;
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
                .default_value("9")
                .validator(|input| {
                    input
                        .parse::<u16>()
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .help("Confirmations of block for security"),
        )
        .arg(
            Arg::with_name("cache-path")
                .long("cache-path")
                .takes_value(true)
                .help("Cache data for current testnet"),
        )
        .get_matches();

    let testnet_rpc_server = matches.value_of("testnet-rpc-server").unwrap();
    let confirmations: u16 = matches.value_of("confirmations").unwrap().parse().unwrap();
    let cache_path = matches.value_of("cache-path");

    let base_spec: ChainSpec = toml::from_slice(data::CHAIN_CHAIN_SPEC.as_bytes()).unwrap();
    println!("==== base_spec ====: \n{}\n", toml::to_string_pretty(&base_spec).unwrap());

    let _previous_rewards = previous_rounds::all_rewards();

    let current_testnet_result =
        last_round::read_last_round(testnet_rpc_server, confirmations, cache_path);
    println!("CurrentTestnetResult: {}", current_testnet_result);
}

