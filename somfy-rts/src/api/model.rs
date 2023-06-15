use hex::FromHex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(unused)]
#[derive(Debug)]
pub enum Command {
    CmdDongle(DongleCommand),
    CmdRts(RtsCommand),
    GetAddress(u8),
    SetAddress,
    ///TODO
    Led(LedColor, LedAction, u16),
    ResetAddress(u8),
}

#[allow(unused)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum DongleCommand {
    Alive,
    Resethw,
    FactoryReset,
    Bcheck,
    Bstart,
}

#[allow(unused)]
#[derive(Debug)]
pub enum RtsCommand {
    Up(u8),
    Down(u8),
    Prog(u8),
    My(u8),
    Stop(u8),
    ProgRt(u8),
    FourCycles(u8),
}

#[allow(unused)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum LedColor {
    Red,
    Green,
}

#[allow(unused)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum LedAction {
    Fix,
    Blink,
}

#[derive(Debug)]
pub enum Response<T> {
    DongleOk(T),

    Err(String),
}

#[derive(Debug, Deserialize)]
pub struct Empty {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub struct Alive {
    pub rssi_val: i32,
    pub id: [String; 3],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub struct AddressVal {
    pub address_val: [Value; 3],
}

pub struct BlindAddress {
    id: u8,
    addr: [u8; 3],
    rolling_code: [u8; 2],
}

pub enum BadBlindDetails {
    MissingId,
    MissingAddress,
    MissingRollingCode,
    BadHexValue,
}

impl TryFrom<AddressVal> for BlindAddress {
    type Error = BadBlindDetails;

    fn try_from(value: AddressVal) -> Result<Self, Self::Error> {
        let id = value.address_val[0]
            .as_u64()
            .ok_or(BadBlindDetails::MissingId)? as u8;

        let addr: [u8; 3] = value.address_val[1]
            .as_str()
            .map(FromHex::from_hex)
            .ok_or(BadBlindDetails::MissingAddress)?
            .or(Err(BadBlindDetails::BadHexValue))?;

        let rolling_code: [u8; 2] = value.address_val[2]
            .as_str()
            .map(FromHex::from_hex)
            .ok_or(BadBlindDetails::MissingRollingCode)?
            .or(Err(BadBlindDetails::BadHexValue))?;

        Ok(BlindAddress {
            id,
            addr,
            rolling_code,
        })
    }
}
