use std::str::FromStr;

use log::trace;
use tokio_serial::{DataBits, SerialPortInfo, SerialPortType, StopBits, UsbPortInfo};

use self::api::somfy_dongle;

pub use api::model::*;
pub use api::{ SomfyRTSDongle, Waiting, Ready, WireFormat};

mod api;

pub fn detect() -> Vec<SerialPortInfo> {
    tokio_serial::available_ports()
        .into_iter()
        .flatten()
        .inspect(|port| trace!(target:"libsomfy_rts::detect", "Detected serial port: {:?}", port))
        .filter(|port| port.port_name.contains("tty"))
        .filter(|port| {
            matches!(
                &port.port_type,
                SerialPortType::UsbPort(UsbPortInfo {
                    vid: 8883,
                    pid: 1551,
                    ..
                })
            )
        })
        .collect()
}

pub fn new(path: &str) -> Result<SomfyRTSDongle<Waiting>, tokio_serial::Error> {
    let serial_port = tokio_serial::new(path, 9600)
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One);

    tokio_serial::SerialStream::open(&serial_port).map(somfy_dongle)
}

impl<T> From<Response<T>> for Result<T, String> {
    fn from(value: Response<T>) -> Self {
        match value {
            Response::DongleOk(t) => Ok(t),
            Response::Err(e) => Err(e),
        }
    }
}

impl FromStr for SomfyRTSDongle<Waiting> {
    type Err = tokio_serial::Error;

    fn from_str(serial: &str) -> Result<Self, Self::Err> {
        self::new(serial)
    }
}