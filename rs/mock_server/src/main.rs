use std::{collections::HashMap, io::BufReader};
use std::net::{TcpListener, ToSocketAddrs};
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use common::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let srv = TcpListener::bind("localhost:45575")?;
//    let mut threads = vec![];

//    loop {
        let (stream, addr) = srv.accept()?;
        println!("New connection from {:?}", addr);

        let mut read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = stream;

//        threads.push(thread::spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let machine_description = loop {
                if let Some(machine_description) = try_read_message(&mut read_stream).transpose() {
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
            if let Some((_, stream)) = streams.iter().next() {
                let addr = format!("http://{}:{}", stream.address, stream.port);
                std::process::Command::new("firefox")
                    .args([addr])
                    .spawn().unwrap();
            }


            let msg = Message::new(
                MessageInner::FunctionCall {
                    name: "count_bools".to_owned(),
                    parameters: HashMap::<String, Box<RawValue>>::from([
                        ("values".to_owned(), to_raw_value(&[true, true, false, true, false])?),
                    ]),
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::FunctionCall {
                    name: "average".to_owned(),
                    parameters: HashMap::<String, Box<RawValue>>::from([
                        ("x".to_owned(), to_raw_value(&[4.5, 3.7, 20.0, 45.2])?),
                    ]),
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::SensorRead {
                    name: "count".to_owned(),
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::AxisChange {
                    name: "example".to_owned(),
                    value: 6.0,
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::AxisChange {
                    name: "example".to_owned(),
                    value: 42.0,
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);


            let msg = Message::new(
                MessageInner::Reset {},
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            // No reply expected


            let reply = loop {
                if let Some(reply) = try_read_message(&mut read_stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);
//            Ok(())
//        }));

//    }

    Ok(())
}
