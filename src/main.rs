use clap::{Arg, App, AppSettings};

fn main() {
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
}
