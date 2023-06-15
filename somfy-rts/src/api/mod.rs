use std::borrow::Cow;

use bytes::BytesMut;
use log::trace;
use serde::de::DeserializeOwned;
use tokio::io::AsyncWriteExt;
use tokio_serial::SerialStream;

use self::{
    error::Error,
    model::Command,
    read::{JsonResponse, LinesResponse, Response},
};

use super::{AddressVal, Alive, Empty, LedAction, LedColor, RtsCommand};

mod error;
pub mod model;
mod read;
pub mod ser;

pub trait State {}

pub struct Waiting;
pub struct Factory(Vec<String>);
pub struct Ready(WireFormat);

impl State for Waiting {}
impl State for Factory {}
impl State for Ready {}

pub struct SomfyRTSDongle<S> {
    state: S,
    device: SerialStream,
}

pub fn somfy_dongle(device: SerialStream) -> SomfyRTSDongle<Waiting> {
    SomfyRTSDongle {
        state: Waiting,
        device,
    }
}

impl<S: State> SomfyRTSDongle<S> {
    async fn send_raw<T>(&mut self, cmd: &str, response: Box<dyn Response<T>>) -> Result<T, Error> {
        let mut buffer = BytesMut::with_capacity(1024);

        self.device.write_all(cmd.as_bytes()).await?;

        match read::try_read_stuff(&mut self.device, &mut buffer, response).await {
            Ok(response) => Ok(response),

            Err(read::Error::Io(e)) => Err(Error::Io(e)),

            Err(e) => Err(Error::Comm(e)),
        }
    }
}

impl SomfyRTSDongle<Waiting> {
    pub async fn initialize(
        mut self,
        format: WireFormat,
    ) -> Result<(String, SomfyRTSDongle<Ready>), Error> {
        let response = LinesResponse::new(1);

        let lines = self.send_raw(format.init_message(), response).await?;

        Self::parse_response(&lines[0]).map(|line| {
            (
                line.to_string(),
                SomfyRTSDongle {
                    state: Ready(format),
                    device: self.device,
                },
            )
        })
    }

    fn parse_response(line: &str) -> Result<&str, Error> {
        let mut parts = line.split(',');

        use read::Error::Incomplete;

        let missing_part = Error::Comm(Incomplete);

        match parts.next() {
            Some("RTSDONGLE") => match parts.next() {
                Some("OK") => parts.next().ok_or(missing_part),

                Some("KO") | Some(&_) | None => Err(Error::Dongle("Dongle KO".to_string())),
            },

            _ => Err(missing_part),
        }
    }

    #[allow(unused)]
    pub async fn factory_info(mut self) -> Result<SomfyRTSDongle<Factory>, Error> {
        let response = LinesResponse::new(11);

        self.send_raw("$GOTO-FACTORY", response)
            .await
            .map(|info| SomfyRTSDongle {
                state: Factory(info),
                device: self.device,
            })
    }
}

impl SomfyRTSDongle<Factory> {
    #[allow(unused)]
    pub fn data(self) -> Vec<String> {
        self.state.0
    }
}

#[allow(unused)]
impl SomfyRTSDongle<Ready> {
    async fn send_command<T: DeserializeOwned>(
        &mut self,
        cmd: Command,
    ) -> Result<model::Response<T>, Error> {
        let cmd = serde_json::to_string(&cmd).unwrap();

        trace!(target:"libsomfy_rts::send_command", "Sending: {}", cmd);

        let encoded = self.state.0.encode_data(&cmd);

        let response = Box::new(JsonResponse);

        let value = self.send_raw(&encoded, response).await?;

        trace!(target:"libsomfy_rts::send_command", "Raw cmd: {}", cmd);

        match value["ACK"].as_str() {
            Some("DONGLE_OK") => {
                let parsed = serde_json::from_value::<T>(value)?;

                Ok(model::Response::DongleOk(parsed))
            }
            Some("DONGLE_KO") => {
                let message = value["ERROR"]
                    .as_str()
                    .map(ToString::to_string)
                    .unwrap_or_default();

                Ok(model::Response::Err(message))
            }
            _ => unreachable!(),
        }
    }

    pub async fn test_alive(&mut self) -> Result<model::Response<Alive>, Error> {
        self.send_command(Command::CmdDongle(super::DongleCommand::Alive))
            .await
    }

    pub async fn reboot(&mut self) -> Result<model::Response<Empty>, Error> {
        self.send_command(Command::CmdDongle(super::DongleCommand::Resethw))
            .await
    }

    pub async fn factory_reset(&mut self) -> Result<model::Response<Empty>, Error> {
        self.send_command(Command::CmdDongle(super::DongleCommand::FactoryReset))
            .await
    }

    pub async fn b_check(&mut self) -> Result<model::Response<Empty>, Error> {
        self.send_command(Command::CmdDongle(super::DongleCommand::Bcheck))
            .await
    }

    pub async fn b_start(&mut self) -> Result<model::Response<Empty>, Error> {
        self.send_command(Command::CmdDongle(super::DongleCommand::Bstart))
            .await
    }

    pub async fn led(
        &mut self,
        color: LedColor,
        action: LedAction,
        duration: u16,
    ) -> Result<super::Response<Empty>, Error> {
        self.send_command(Command::Led(color, action, duration))
            .await
    }

    pub async fn get_blind(&mut self, id: u8) -> Result<super::Response<AddressVal>, Error> {
        self.send_command(Command::GetAddress(id)).await
    }

    pub async fn set_blind(
        &mut self,
        id: u8,
        address: u32,
        rolling_code: u16,
    ) -> Result<super::Response<AddressVal>, Error> {
        self.send_command(Command::SetAddress).await
    }

    pub async fn remove_blind(&mut self, id: u8) -> Result<super::Response<Empty>, Error> {
        self.send_command(Command::ResetAddress(id)).await
    }

    pub async fn operate_blind(
        &mut self,
        cmd: RtsCommand,
    ) -> Result<super::Response<AddressVal>, Error> {
        self.send_command(Command::CmdRts(cmd)).await
    }
}

pub enum WireFormat {
    Normal,
    CryptoOff,
}

impl WireFormat {
    pub fn init_message(&self) -> &'static str {
        match self {
            Self::Normal => "$HELLOSOMFYBG3174",
            Self::CryptoOff => "$CRYPTO_OFF_3145",
        }
    }

    pub fn encode_data<'a>(&self, data: &'a str) -> Cow<'a, str> {
        match self {
            Self::CryptoOff => Cow::Borrowed(data),
            Self::Normal => Cow::Owned(encrypt(data)),
        }
    }

    pub fn decode_data<'a>(&self, data: &'a str) -> Cow<'a, str> {
        match self {
            Self::CryptoOff => Cow::Borrowed(data),
            Self::Normal => Cow::Owned(decrypt(data)),
        }
    }
}

fn encrypt(data: &str) -> String {
    todo!()
}

fn decrypt(encrypted: &str) -> String {
    todo!()
}
