use std::str::FromStr;

use log::{info, debug, warn};
use somfy_rts::{SomfyRTSDongle, Waiting, Ready, Alive, AddressVal, WireFormat, Response};
use anyhow::{Result, bail, anyhow};

pub async fn get_rts_dongle(serial: Option<String>) -> Result<SomfyRTSDongle<Waiting>> {
    match serial.as_deref().map(SomfyRTSDongle::<Waiting>::from_str) {
        None => {
            info!(target: "get_rts_dongle", "No dongle was provided.");
            let dongles = somfy_rts::detect();

            if dongles.is_empty() {
                bail!("Found 0 dongles");
            }
            info!(
                "Found {} dongles, using dongle at: {}",
                dongles.len(),
                &dongles[0].port_name
            );

            Ok(somfy_rts::new(&dongles[0].port_name)?)
        }

        Some(dongle) => Ok(dongle?),
    }
}

pub async fn init_dongle(dongle: SomfyRTSDongle<Waiting>) -> Result<(SomfyRTSDongle<Ready>, Alive)> {
    let (_, mut dongle_ready) = dongle.initialize(WireFormat::CryptoOff).await?;

    let is_alive: Result<Alive, String> = dongle_ready.test_alive().await?.into();

    if let Ok(is_alive) = is_alive.as_ref() {
        debug!(target: "init_dongle", "Is dongle alive ?
         {:?}", is_alive);
    }

    Ok((dongle_ready, is_alive.map_err(|e| anyhow!(e))?))
}

pub async fn list_usable_blinds(dongle: &mut SomfyRTSDongle<Ready>) -> Vec<(u8, AddressVal)> {
    let mut blinds = Vec::with_capacity(100);

    for i in 1..=100 {
        if let Ok(Response::DongleOk(blind)) = dongle.get_blind(i).await {
            if let Some(str) = blind.address_val[2].as_str() {
                if !"0000".eq_ignore_ascii_case(str) {
                    blinds.push((i, blind))
                }
            }
        }
        else {
            warn!(target: "somfy", "Failed to get blind {}", i)
        }
    }

    blinds
}