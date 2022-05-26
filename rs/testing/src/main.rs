use std::{collections::HashMap, sync::Arc};
use serde_json::value::to_raw_value;
use common::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let double_str: Arc<str> = "double".into();
    let axis_group = Some("movement".into());
    let msg = Message{
        message_id: 4096,
        inner: MessageInner::MachineDescription {
            name: "machine name".into(),
            functions: HashMap::from(
                [("test".into(), Function{ parameters: [].into(), returns: [].into() })]
            ),
            sensors: HashMap::from(
                [("test".into(), Sensor{ output_type: double_str.clone(), min: -1.0, max: 1.0 })]
            ),
            axes: HashMap::from(
                [
                    ("test1".into(), Axis{ input_type: double_str.clone(), min: 0.0, max: 1.0, group: axis_group.clone(), direction: Some("y".into()) }),
                    ("test2".into(), Axis{ input_type: double_str.clone(), min: -1.0, max: 1.0, group: axis_group.clone(), direction: Some("x".into()) }),
                ]
            ),
            streams: HashMap::from(
                [("test".into(), Stream{
                    format: "mjpeg".into(),
                    buffer_method: BufferMethod::Frames,
                })]
            ),
        },
    };
    dbg!(to_raw_value(&msg));
    let msg = Message{
        message_id: 4096,
        inner: MessageInner::AxisChange { destination: "machine name".into(), name: "xAxis".into(), value: 3.0 },
    };
    dbg!(to_raw_value(&msg));
    let msg = Message{
        message_id: 4096,
        inner: MessageInner::StreamDescription { machine: "machine name".into(), stream: "stream name".into() },
    };
    dbg!(to_raw_value(&msg));
    Ok(())
}
