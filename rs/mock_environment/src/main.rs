use std::{collections::HashMap, io::BufReader};
use std::net::TcpStream;
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use common::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let stream = TcpStream::connect("localhost:45576").unwrap();
    let stream_stream = TcpStream::connect("localhost:45578").unwrap();

    let mut read_stream = BufReader::new(stream.try_clone()?);
    let mut write_stream = stream;

    dbg!("A");

    let machine_list_request = Message::new(MessageInner::MachineListRequest {});
    try_write_message(&mut write_stream, &machine_list_request).expect("writing failed");

    dbg!("A");

    let machine_list = try_read_message(&mut read_stream, None).expect("reading failed").expect("server disconnected");
    let machine_list = match machine_list.inner {
        MessageInner::MachineListReply { machines } => machines,
        _ => panic!("server did not give machine list"),
    };
    dbg!(&machine_list);

    dbg!("A");

    let machine = machine_list.into_iter().next().expect("no machines");


    let msg = Message::new(
        MessageInner::FunctionCall {
            destination: machine.clone(),
            name: "count_bools".into(),
            parameters: HashMap::<_, Box<RawValue>>::from([
                ("values".into(), to_raw_value(&[true, true, false, true, false])?),
            ]),
        },
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    let reply = loop {
        if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
            if !matches!(reply, Ok(Message { inner: MessageInner::Heartbeat { .. }, ..})) {
                break reply;
            }
        }
    };
    dbg!(&reply);

    let msg = Message::new(
        MessageInner::FunctionCall {
            destination: machine.clone(),
            name: "average".into(),
            parameters: HashMap::<_, Box<RawValue>>::from([
                ("x".into(), to_raw_value(&[4.5, 3.7, 20.0, 45.2])?),
            ]),
        },
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    let reply = loop {
        if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
            if !matches!(reply, Ok(Message { inner: MessageInner::Heartbeat { .. }, ..})) {
                break reply;
            }
        }
    };
    dbg!(&reply);

    let msg = Message::new(
        MessageInner::SensorRead {
            destination: machine.clone(),
            name: "count".into(),
        },
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    let reply = loop {
        if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
            if !matches!(reply, Ok(Message { inner: MessageInner::Heartbeat { .. }, ..})) {
                break reply;
            }
        }
    };
    dbg!(&reply);
    try_write_message(&write_stream, &msg)?;
    let reply = loop {
        if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
            if !matches!(reply, Ok(Message { inner: MessageInner::Heartbeat { .. }, ..})) {
                break reply;
            }
        }
    };
    dbg!(&reply);

    let msg = Message::new(
        MessageInner::AxisChange {
            destination: machine.clone(),
            name: "example".into(),
            value: 6.0,
        },
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    let reply = loop {
        if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
            if !matches!(reply, Ok(Message { inner: MessageInner::Heartbeat { .. }, ..})) {
                break reply;
            }
        }
    };
    dbg!(&reply);

    let msg = Message::new(
        MessageInner::AxisChange {
            destination: machine.clone(),
            name: "example".into(),
            value: 42.0,
        },
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    let reply = loop {
        if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
            if !matches!(reply, Ok(Message { inner: MessageInner::Heartbeat { .. }, ..})) {
                break reply;
            }
        }
    };
    dbg!(&reply);


    let msg = Message::new(
        MessageInner::Reset {destination: machine.clone()},
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    // No reply expected


    let msg = Message::new(
        MessageInner::Disconnect {},
    );
    dbg!(&msg);
    try_write_message(&write_stream, &msg)?;
    // No reply expected


    Ok(())
}
