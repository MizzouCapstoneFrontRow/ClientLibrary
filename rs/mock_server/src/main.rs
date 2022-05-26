use std::io::Read;
use std::{collections::HashMap, io::BufReader};
use std::net::TcpListener;
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use common::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let srv = TcpListener::bind("localhost:45575").unwrap();
    let stream_srv = TcpListener::bind("localhost:45577").unwrap();
//    let mut threads = vec![];

//    loop {
        let (stream, addr) = srv.accept()?;
        println!("New connection from {:?}", addr);

        let mut read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = stream;

//        threads.push(thread::spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let machine_description = loop {
                if let Some(machine_description) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
                    break machine_description;
                }
            };
            dbg!(&machine_description);
            let (name, functions, sensors, axes, streams) = match machine_description.unwrap() {
                Message{inner: MessageInner::MachineDescription { name, functions, sensors, axes, streams }, ..} => {
                    (name, functions, sensors, axes, streams)
                },
                _ => panic!("no stream"),
            };
            for _ in 0..streams.len() {
                // let (_, stream) = streams.iter().next().unwrap();
                // let addr = format!("http://{}:{}", stream.address, stream.port);
                // std::process::Command::new("firefox")
                //     .args([addr])
                //     .spawn().unwrap();
                let (stream_stream, stream_addr) = stream_srv.accept().unwrap();
                let mut stream_read_stream = BufReader::new(stream_stream);
                let msg = try_read_message(&mut stream_read_stream, None)?;
                let msg = msg.unwrap();
                let stream_name = match msg.inner {
                    MessageInner::StreamDescription { machine, stream: stream_name } => stream_name,
                    _ => unreachable!("should have a stream"),
                };
                let stream_thread = std::thread::spawn(move || {
                    let mut buf = vec![0; 4096];
                    loop {
                        match stream_read_stream.read(&mut buf[..]) {
                            Ok(0) => {
                                println!("EOF on stream {stream_name:?}");
                                break;
                            }
                            Ok(n) => {
                                println!("{n} bytes on stream {stream_name:?}");
                            }
                            Err(e) => {
                                println!("Error on stream {stream_name:?}: {e:?}");
                                break;
                            }
                        }
                    }
                });
            }


            let msg = Message::new(
                MessageInner::FunctionCall {
                    destination: name.as_ref().into(),
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
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::FunctionCall {
                    destination: name.as_ref().into(),
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
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::SensorRead {
                    destination: name.as_ref().into(),
                    name: "count".into(),
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::AxisChange {
                    destination: name.clone(),
                    name: "example".into(),
                    value: 6.0,
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::AxisChange {
                    destination: name.clone(),
                    name: "example".into(),
                    value: 42.0,
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);


            let msg = Message::new(
                MessageInner::Reset {destination: name.clone()},
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            // No reply expected


            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream, Some(std::time::Duration::from_secs(0))).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);
//            Ok(())
//        }));

//    }

    Ok(())
}
