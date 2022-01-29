use std::collections::HashMap;
use std::net::TcpListener;
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use client::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut srv = TcpListener::bind("localhost:8089")?;
//    let mut threads = vec![];

//    loop {
        let (mut stream, addr) = srv.accept()?;
        println!("New connection from {:?}", addr);

//        threads.push(thread::spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let msg = Message::new(
                MessageInner::FunctionCall {
                    name: "count_bools".to_owned(),
                    parameters: HashMap::<String, Box<RawValue>>::from([
                        ("values".to_owned(), to_raw_value(&[true, true, false, true, false])?),
                    ]),
                },
            );
            dbg!(&msg);
            try_write_message(&stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&stream).transpose() {
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
            try_write_message(&stream, &msg)?;
            let reply = loop {
                if let Some(reply) = try_read_message(&stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);


            let reply = loop {
                if let Some(reply) = try_read_message(&stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);
//            Ok(())
//        }));

//    }

    Ok(())
}
