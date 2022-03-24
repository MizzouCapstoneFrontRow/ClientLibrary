use std::{collections::HashMap, io::BufReader};
use std::net::{TcpListener, ToSocketAddrs};
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use common::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let msg = Message{
        message_id: 4096,
        inner: MessageInner::MachineDescription {
            name: "machine name".to_owned(),
            functions: HashMap::from(
                [("test".into(), Function{ parameters: [].into(), returns: [].into() })]
            ),
            sensors: HashMap::from(
                [("test".into(), Sensor{ output_type: "float".into(), min: None, max: Some(to_raw_value::<f64>(&4.5).unwrap()) })]
            ),
            axes: HashMap::from(
                [("test".into(), Axis{ input_type: "float".into(), min: None, max: Some(to_raw_value::<f64>(&4.5).unwrap()) })]
            ),
            streams: HashMap::from(
                [("test".into(), Stream{
                    format: "mjpeg".into(),
                    address: "192.168.1.11".into(),
                    port: 8554,
                })]
            ),
        },
    };
    dbg!(to_raw_value(&msg));
    Ok(())
}
