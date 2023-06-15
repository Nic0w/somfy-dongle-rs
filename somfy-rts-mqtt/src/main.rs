use std::{time::{Duration, SystemTime}};

use anyhow::{Result};
use clap::Parser;
use futures::TryFutureExt;
use log::{info, trace, debug, warn};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet};


mod ha;
mod somfy;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Serial port name to operate on. If none is provided, we will attemp to find one.
    #[arg(short, long, value_name = "SERIAL PORT")]
    serial: Option<String>,

    #[arg(value_name = "MQTT BROKER", help = "MQTT connection string in the following format: id:host:port" )]
    #[arg(value_parser = mqtt_option)]
    mqtt_opt: MqttOptions,
}

fn mqtt_option(arg: &str) -> std::result::Result<MqttOptions, String> {
    let mut parts = arg.split(':');

    let id = parts
        .next()
        .ok_or("Missing 'id' in MQTT option string".to_string())?;
    let host = parts
        .next()
        .ok_or("Missing 'host' in MQTT option string".to_string())?;
    let port = parts
        .next()
        .ok_or("Missing 'port' in MQTT option string".to_string())?
        .parse::<u16>()
        .map_err(|e| format!("Bad 'port' value in MQTT option string: {}", e))?;

    Ok(MqttOptions::new(id, host, port))
}

fn init_logging() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("rumqttc::state", log::LevelFilter::Error)
        .level_for("mio_serial", log::LevelFilter::Error)
        .chain(std::io::stdout())
        //.chain(fern::log_file("/var/log/output.log")?)
        .apply()?;
    Ok(())

}


#[tokio::main]
async fn main() -> Result<()> {

    init_logging().unwrap();

    let args = Cli::parse();

    if let Some(port) = args.serial.as_ref() {
        debug!(target: "main", "{:?} supplied as serial port.", port);
    }

    let (mut dongle_ready, dongle_alive) =
        somfy::get_rts_dongle(args.serial.clone()).and_then(somfy::init_dongle).await?;

    info!(target: "main", "Successfully initialized dongle at '{:?}'.", args.serial);

    let mut mqttoptions = args.mqtt_opt;
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let active_blinds = somfy::list_usable_blinds(&mut dongle_ready).await;

    info!(target: "main", "Found {} useable blinds.", active_blinds.len());

    ha::setup_mqtt_autodiscovery(&client, &active_blinds, &dongle_alive.id[0])
        .and_then(ha::set_state_on)
        .await?;

    info!(target: "main", "Successfully set HA MQTT discovery up.");

    while let Ok(notification) = eventloop.poll().await {
        trace!(target:"main", "Received = {:?}", notification);

        if let Event::Incoming(Packet::Publish(data)) = notification {
            debug!(target:"main", "{} -> {:?}", data.topic, data.payload);

            let mut topic = data.topic.split('/');

            let blind_id = topic.nth(2).map(str::parse::<u8>);

            let payload = std::str::from_utf8(&data.payload);

            let response = match (blind_id, payload) {

                (Some(Ok(id)), Ok("UP")) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Up(id))
                        .await
                },
                (Some(Ok(id)), Ok("DOWN")) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Down(id))
                        .await
                },
                (Some(Ok(id)), Ok("STOP")) => {
                    dongle_ready
                        .operate_blind(somfy_rts::RtsCommand::Stop(id))
                        .await
                },

                (None, _) => {
                    warn!(target:"main", "Received message with bad topic: '{}'", data.topic);
                    continue;
                },

                (Some(Err(e)), _) => {
                    warn!(target:"main", "Received message with bad blind id: '{}' ({})", data.topic, e);
                    continue;
                },

                (_, Err(e)) => {
                    warn!(target:"main", "Received message with bad payload: {}", e);
                    continue;
                }

                (_, Ok(o)) => {
                    warn!(target:"main", "Received message with unknown order: {}'", o);
                    continue;
                }
            };

            if let Err(e) = response {
                warn!(target:"main", "Order failed: {}", e)
            }
        }
    }

    Ok(())
}