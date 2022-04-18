use std::io::Read;
use std::time::Duration;
use std::{collections::HashMap, io::BufReader};
use std::net::{TcpListener, ToSocketAddrs};
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use common::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let srv = TcpListener::bind("localhost:45575")?;
    let stream_srv = TcpListener::bind("localhost:45577")?;
//    let mut threads = vec![];

//    loop {
        let (stream, addr) = srv.accept()?;
        println!("New connection from {:?}", addr);

        let mut read_stream = BufReader::new(stream.try_clone()?);
        let write_stream = stream;

        //        threads.push(thread::spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let machine_description = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&machine_description);
            let (name, functions, sensors, axes, streams) = match machine_description.unwrap() {
                Message{inner: MessageInner::MachineDescription { name, functions, sensors, axes, streams }, ..} => {
                    (name, functions, sensors, axes, streams)
                },
                _ => panic!("invalid machine description"),
            };
            let streams_ = streams.clone();
            dbg!(&streams);
            let stream_thread = std::thread::spawn(move || {
                dbg!("stream thread");
                while let Ok((stream, addr)) = stream_srv.accept() {
                    dbg!(addr);
                    let mut write_stream = stream.try_clone().unwrap();
                    let mut read_stream = BufReader::new(stream);
                    // if let Some(msg) = try_read_message(&mut stream, Some(Duration::from_secs(1))).transpose() {
                    if let Some(msg) = try_read_message(&mut read_stream, None).transpose() {
                        let msg = msg.unwrap();
                        match msg.inner {
                            MessageInner::StreamDescription { machine, stream: stream_name } => {
                                let fmt = &streams.get(&stream_name).expect("stream not found").format;
                                println!("Stream {} (format {}) on thread {:?}", stream_name, fmt, std::thread::current());
                                std::thread::spawn(
                                    move || {
                                        dbg!("aaaa");
                                        let mut buf = vec![0u8; 4096];
                                        loop {
                                            match read_stream.read(&mut buf) {
                                                Ok(0) => {
                                                    println!("Stream ended");
                                                    break
                                                },
                                                Ok(n) => println!("Stream read {n} bytes"),
                                                Err(e) => {
                                                    println!("Stream error: {e:?}");
                                                    break;
                                                }
                                            };
                                        }
                                        dbg!("bbbb");
                                    }
                                );
                            },
                            i => {
                                println!("uhhhhh");
                                dbg!(i);
                            },
                        };
                    } else {
                        dbg!("No stream?");
                    }
                }
            });
            // for (_, stream) in streams.iter() {
            //     let addr = format!("http://{}:{}", stream.address, stream.port);
            //     std::process::Command::new("firefox")
            //         .args([addr])
            //         .spawn().unwrap();
            // }
            let msg = Message::new(
                MessageInner::SetupResponse { connected: true }
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            dbg!("sent");
            // No reply expected


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
            dbg!("sent");
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
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
            dbg!("sent");
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::SensorRead {
                    name: "count".to_owned(),
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            dbg!("sent");
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);
            try_write_message(&write_stream, &msg)?;
            dbg!("sent");
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::AxisChange {
                    name: "example".to_owned(),
                    value: 6.0,
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::AxisChange {
                    name: "example".to_owned(),
                    value: 42.0,
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::Reset {  },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);
//            Ok(())
//        }));

//    }

    Ok(())
}
