use std::collections::HashMap;
use std::net::TcpListener;
use std::io::Write;
use serde_json::value::{RawValue, to_raw_value};
use client::message::*;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut srv = TcpListener::bind("localhost:8089")?;

    loop {
        let result = (|| -> Result<(), Box<dyn std::error::Error + 'static>> {
            let (mut stream, addr) = srv.accept()?;
            println!("New connection from {:?}", addr);
            let msg = Message {
                message_id: 1,
                inner: MessageInner::FunctionCall {
                    name: "count_bools".to_owned(),
                    parameters: HashMap::<String, Box<RawValue>>::from([
                        ("values".to_owned(), to_raw_value(&[true, true, false, true, false])?),
                    ]),
                },
            };

            let msg = serde_json::ser::to_vec(&msg)?;
            let byte_count = msg.len();
            let byte_count_buf = byte_count.to_be_bytes();

            stream.write_all(&byte_count_buf)?;
            stream.write_all(&msg)?;

            let reply = loop {
                if let Some(reply) = try_read_message(&stream).transpose() {
                    break reply;
                }
            };
            dbg!(&reply);

            Ok(())
        })();
        
    }

    Ok(())
}
