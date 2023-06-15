use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};
use serde_json::json;

use super::model::{Command, RtsCommand};

fn screaming_kebab_case(value: &str) -> String {
    let mut string = String::default();

    for (i, ch) in value.char_indices() {
        if ch.is_ascii_uppercase() && i > 0 {
            string.push('-');
        }

        string.push(ch.to_ascii_uppercase())
    }

    string
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string_repr = screaming_kebab_case(&format!("{:?}", self));
        let ident = string_repr.split('(').next().unwrap();

        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::CmdDongle(cmd) => {
                map.serialize_entry(ident, cmd)?;
            }

            Self::CmdRts(cmd) => {
                map.serialize_entry(ident, cmd)?;
            }

            Self::Led(color, action, length) => {
                let array = json!([color, action, *length]);

                map.serialize_entry(ident, &array)?;
            }

            Self::GetAddress(id) | Self::ResetAddress(id) => {
                map.serialize_entry(ident, id)?;
            }

            _ => todo!(),
        }

        map.end()
    }
}

impl Serialize for RtsCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;

        let string_repr = format!("{:?}", self).to_uppercase();

        let ident = string_repr.split('(').next().unwrap();
        
        match self {
            Self::Up(value)
            | Self::Down(value)
            | Self::Prog(value)
            | Self::My(value)
            | Self::Stop(value) => {
                seq.serialize_element(ident)?;
                seq.serialize_element(value)?;
            }

            Self::ProgRt(value) => {
                seq.serialize_element("PROG_RT")?;
                seq.serialize_element(value)?;
            }

            Self::FourCycles(value) => {
                seq.serialize_element("4_CYCLES")?;
                seq.serialize_element(value)?;
            }
        }
        seq.end()
    }
}
