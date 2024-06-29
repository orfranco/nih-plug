use anyhow::{anyhow, Context, Result};
use nih_plug::{nih_error, nih_log};
use rust_socketio::client::Client;
use rust_socketio::{ClientBuilder, Error, Payload, RawClient};
use serde::Deserialize;
use serde_json::{from_str, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const SOCKET_ADDR: &str = "http://localhost:3001";

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct SensorData {
    pub sensor_id: String,
    pub timestamp: String,
    pub euler_x: f32,
    pub euler_y: f32,
    pub euler_z: f32,
}

pub struct SensorDataReceiver {
    client: Option<Client>,
    pub curr_data: Arc<Mutex<HashMap<String, SensorData>>>,
}

impl SensorDataReceiver {
    pub fn new() -> SensorDataReceiver {
        SensorDataReceiver {
            client: None,
            curr_data: Arc::default(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), Error> {
        nih_log!("Initializing Sensors Data Receiver. Socket Address: {SOCKET_ADDR}");
        let curr_data = self.curr_data.clone();
        let inner_callback = move |payload: Payload, _: RawClient| {
            SensorDataReceiver::handle_message(&payload, curr_data.clone())
                .map_err(|err| nih_error!("Failed handling Sensors Data Message. Error: {err}"))
                .ok();
        };

        self.client = Some(
            ClientBuilder::new(SOCKET_ADDR)
                .on("receive-data", inner_callback)
                .connect()?,
        );

        Ok(())
    }

    fn handle_message(
        payload: &Payload,
        curr_data: Arc<Mutex<HashMap<String, SensorData>>>,
    ) -> Result<()> {
        if let Payload::Text(text) = payload {
            let msg = text
                .first()
                .ok_or_else(|| anyhow!("Expected text to contain at least one element"))?;

            let msg_str = match msg {
                Value::String(s) => s,
                _ => return Err(anyhow!("Expected first element to be a string")),
            };

            let sensor_data: SensorData =
                from_str(msg_str).context("Failed to deserialize JSON message")?;

            let mut data = curr_data
                .lock()
                .map_err(|err| anyhow::anyhow!("Failed to lock curr_data: {err}"))?;

            data.insert(sensor_data.sensor_id.clone(), sensor_data);

            Ok(())
        } else {
            Err(anyhow::anyhow!("Received an unexpected payload type"))
        }
    }
}
