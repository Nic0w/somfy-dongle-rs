use anyhow::Result;
use rumqttc::{AsyncClient, QoS};
use serde_json::json;
use somfy_rts::AddressVal;

const HA_DISCOVERY_PREFIX: &str = "homeassistant";
const HA_MQTT_COMPONENT: &str = "cover";
const HA_MQTT_NODEID: &str = "somfy-rts";

pub async fn setup_mqtt_autodiscovery<'a>(
    client: &'a AsyncClient,
    blinds: &[(u8, AddressVal)],
    dongle_id: &str,
) -> Result<&'a AsyncClient> {
    for (
        id,
        AddressVal {
            address_val: [_, addr, _],
        },
    ) in blinds
    {
        let addr = addr.as_str().unwrap();

        let command_topic = format!("{}/cover/{}/set", HA_MQTT_NODEID, id);
        let config_topic = format!(
            "{HA_DISCOVERY_PREFIX}/{HA_MQTT_COMPONENT}/{}/{}/config",
            dongle_id, addr
        );

        let config = config_for_blind(*id, dongle_id, addr);
        let config_payload = serde_json::to_string(&config).unwrap();

        client.subscribe(command_topic, QoS::AtMostOnce).await?;

        client
            .publish(config_topic, QoS::AtLeastOnce, true, config_payload)
            .await?;
    }

    Ok(client)
}

pub async fn set_state_on(client: &AsyncClient) -> Result<()> {
    let state_topic = format!("{}/dongle/state", HA_MQTT_NODEID);

    Ok(client
        .publish(state_topic, QoS::AtLeastOnce, true, "online")
        .await?)
}

fn config_for_blind(id: u8, dongle_serial: &str, addr: &str) -> serde_json::Value {
    json!({
        "availability": [   
            {
                "topic": format!("{}/dongle/state", HA_MQTT_NODEID),
            }
        ],
        "device_class" : "shutter",
        "device": {
            "manufacturer": "Somfy",
            "model": "Shutter",
            "name": "Somfy RTS Shutter",
            "identifiers": [
                format!("{}_{}", dongle_serial, addr)
            ]
        },
        "name": format!("Somfy Shutter n°{} ({})", id, addr),
        "retain":true,
        "payload_close": "DOWN",
        "payload_open": "UP",
        "payload_stop": "STOP",
        "command_topic": format!("{}/cover/{}/set", HA_MQTT_NODEID, id)
    })
}
