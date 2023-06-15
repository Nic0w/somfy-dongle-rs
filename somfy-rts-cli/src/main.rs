use std::{
    ops::{Range, RangeBounds},
    process::exit,
};

use clap::{Arg, ArgAction, Args, Command, Parser, Subcommand};

use somfy_rts::WireFormat;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Serial port name to operate on. If none is provided, we will attemp to find one.
    #[arg(short, long, value_name = "SERIAL PORT")]
    serial: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

fn validate_range(arg: &str) -> Result<Box<dyn Iterator<Item = u8>>, String> {
    let (start, end) = arg
        .split_once("..")
        .ok_or(format!("`{arg}` isn't a valid range."))?;

    let start_bound = 1;
    let end_bound = 100;

    let start_bound = (!start.is_empty())
        .then(|| {
            start
                .parse::<u8>()
                .or(Err(format!("`{start}` isn't a valid bound.")))
        })
        .unwrap_or(Ok(start_bound))?;

    let end_stripped = end.strip_prefix('=').map(str::parse::<u8>);

    if let Some(end_val) = end_stripped {
        let mut end_bound = end_val.or(Err(format!("`{end}` isn't a valid bound.")))?;

        if end_bound > 100 {
            end_bound = 100
        }

        Ok(Box::new(start_bound..=end_bound))
    } else if end.is_empty() {
        Ok(Box::new(start_bound..end_bound))
    } else {
        let end_bound: u8 = end
            .parse()
            .or(Err(format!("`{end}` isn't a valid bound.")))?;

        Ok(Box::new(start_bound..end_bound))
    }
}

#[derive(Subcommand)]
enum Commands {
    Up(Blind),
    Down(Blind),
    Stop(Blind),
    My(Blind),
    Prog(Blind),

    GetAddress(BlindRange),
    SetAddress(BlindRange),
    ResetAddress(BlindRange),
}

#[derive(Args)]
struct Blind {
    ///Id of the blind to command, from 1 to 100
    blind: u8,
}

#[derive(Args)]
struct BlindRange {
    ///Range of the blinds to affect; ex: 1..20
    range: String,
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let args = Cli::parse();

    let provided_dongle = args.serial.as_deref().map(somfy_rts::new);

    let somfy_dongle = match provided_dongle {
        None => {
            println!("No dongle was provided.");
            let dongles = somfy_rts::detect();

            println!("Found {} dongles.", dongles.len());

            if dongles.is_empty() {
                println!("Exiting!");
                exit(-1);
            }

            println!("Using dongle at: {}", &dongles[0].port_name);

            somfy_rts::new(&dongles[0].port_name)
        }

        Some(dongle) => dongle,
    }
    .unwrap_or_else(|e| {
        println!("Failed to open selected dongle: {}", e);
        exit(-1);
    });

    let (_, mut dongle_ready) = somfy_dongle
        .initialize(WireFormat::CryptoOff)
        .await
        .unwrap();

    match dongle_ready.test_alive().await.unwrap() {
        somfy_rts::Response::Err(e) => {
            println!("Dongle returned error: {}", e);
            exit(-1);
        }

        somfy_rts::Response::DongleOk(info) => {
            println!("Dongle id:{}, {} {}", info.id[0], info.id[2], info.id[1]);

            use Commands::*;
            match args.command {
                Some(Up(Blind { blind })) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Up(blind))
                        .await
                        .unwrap();
                }
                Some(Down(Blind { blind })) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Down(blind))
                        .await
                        .unwrap();
                }
                Some(Stop(Blind { blind })) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Stop(blind))
                        .await
                        .unwrap();
                }
                Some(My(Blind { blind })) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::My(blind))
                        .await
                        .unwrap();
                }
                Some(Prog(Blind { blind })) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Prog(blind))
                        .await
                        .unwrap();
                }

                Some(GetAddress(BlindRange { range })) => {
                    let range = validate_range(&range).unwrap_or_else(|e| {
                        println!("Invalid range: {}", e);
                        exit(-1);
                    });

                    for i in range {
                        let blind_data = dongle_ready.get_blind(i).await.unwrap();
                        println!("{:?}", blind_data);
                    }
                }

                Some(ResetAddress(BlindRange { range })) => {
                    let range = validate_range(&range).unwrap_or_else(|e| {
                        println!("Invalid range: {}", e);
                        exit(-1);
                    });

                    for i in range {
                        let blind_data = dongle_ready.remove_blind(i).await.unwrap();
                        println!("{:?}", blind_data);
                    }
                }

                _ => (),
            }
        }
    }
}
