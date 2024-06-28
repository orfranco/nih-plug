use rust_socketio::client::Client;
use rust_socketio::{ClientBuilder, Payload, RawClient};
use serde::Deserialize;
use serde_json::{from_str, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use nih_plug::{nih_error, nih_log};

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

    pub fn initialize(&mut self) {
        let curr_data = self.curr_data.clone();
        let inner_callback = move |payload: Payload, _: RawClient| {
            SensorDataReceiver::handle_message(&payload, curr_data.clone());
        };

        self.client = ClientBuilder::new("http://localhost:3001")
            .on("receive-data", inner_callback)
            .connect()
            .map_err(|e| nih_error!("Failed Connecting to Sensors Data Socket"))
            .ok()
    }

    fn handle_message(payload: &Payload, curr_data: Arc<Mutex<HashMap<String, SensorData>>>) {
        match payload {
            Payload::Text(text) => {
                if let Some(Value::String(msg)) = text.first() {
                    let sensor_data: SensorData =
                        from_str(msg).expect("JSON was not well-formatted"); // TODO: error handling.
                    curr_data
                        .lock()
                        .expect("failed to lock") // TODO: error handling.
                        .insert(sensor_data.sensor_id.clone(), sensor_data);
                }
            }
            _ => println!("recieved a weird message"),
        }
    }
}
