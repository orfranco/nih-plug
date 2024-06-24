use rust_socketio::client::Client;
use rust_socketio::{ClientBuilder, Payload, RawClient};
use serde::Deserialize;
use serde_json::{from_str, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct SensorData {
    sensor_id: String,
    timestamp: String,
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
            let curr_data_clone = curr_data.clone();
            SensorDataReceiver::handle_message(&payload, curr_data_clone);
        };

        self.client = ClientBuilder::new("http://localhost:3001")
            .on("receive-data", inner_callback)
            .connect()
            .ok()
    }

    fn handle_message(payload: &Payload, curr_data: Arc<Mutex<HashMap<String, SensorData>>>) {
        match payload {
            Payload::Text(text) => {
                if let Some(Value::String(msg)) = text.first() {
                    let sensor_data: SensorData =
                        from_str(msg).expect("JSON was not well-formatted");
                    curr_data
                        .lock()
                        .expect("failed to lock")
                        .insert(sensor_data.sensor_id.clone(), sensor_data);
                }
            }
            _ => println!("recieved a weird message"),
        }
    }
}
