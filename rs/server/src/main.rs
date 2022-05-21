use std::{collections::HashMap, sync::Arc, time::Duration, net::SocketAddr};
use tokio::{net::{TcpListener, ToSocketAddrs, tcp::{OwnedReadHalf, OwnedWriteHalf}}, sync::{RwLock, mpsc::{Sender, Receiver}}, io::{BufReader, AsyncBufReadExt}, runtime::Handle};
//use std::thread;
use serde_json::value::{RawValue, to_raw_value};
use common::message::*;

#[derive(Debug, Clone)]
enum MessageSource {
    Machine(Arc<str>),
    Environment {
        environment: SocketAddr,
        destination_machine: Option<Arc<str>>,
    },
}
type MessageWithSource = (Message, MessageSource);

#[derive(Debug)]
struct ServerState {
    machines: RwLock<HashMap<Arc<str>, Arc<Machine>>>,
    environments: RwLock<HashMap<SocketAddr, Arc<Environment>>>,
    /// Messages will be sent to the message_handler task
    message_handler_tx: Sender<MessageWithSource>
}

impl ServerState {
    fn new(message_handler_tx: Sender<MessageWithSource>) -> Self {
        Self {
            machines: Default::default(),
            environments: Default::default(),
            message_handler_tx,
        }
    }
}

#[derive(Debug)]
struct Machine {
    name: Arc<str>,
    addr: SocketAddr,
    message_tx: Sender<Message>,
    functions: HashMap<String, Function>,
    sensors: HashMap<String, Sensor>,
    axes: HashMap<String, Axis>,
    streams: HashMap<String, (Stream, RwLock<Option<()>>)>,
}

#[derive(Debug)]
struct Environment {
    addr: SocketAddr,
    message_tx: Sender<Message>,
}

async fn machine_listener(state: Arc<ServerState>, machine_srv: TcpListener) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        let (stream, addr) = machine_srv.accept().await?;
        eprintln!("New machine connection from {:?}", addr);
        let (rx, tx) = stream.into_split();

        let mut msg_buf = String::with_capacity(4096);
        let mut rx = BufReader::new(rx);
        let result = rx.read_line(&mut msg_buf).await;
        match result {
            Ok(0) => {
                eprintln!("Machine at {addr:?} disconnected without giving a description.");
                continue;
            }
            Err(err) => {
                eprintln!("Error reading machine description at {addr:?}: {err:?}.");
                continue;
            }
            Ok(_) => {}
        };
        let (message_tx, message_rx) = tokio::sync::mpsc::channel(16);
        let machine = match serde_json::from_str::<Message>(&msg_buf) {
            Err(err) => {
                eprintln!("Error parsing machine description at {addr:?}: {err:?}.");
                continue;
            }
            Ok(Message { inner: MessageInner::MachineDescription {
                name,
                functions,
                sensors,
                axes,
                streams
            }, .. }) => Machine {
                name: name.into(),
                addr,
                message_tx,
                functions,
                sensors,
                axes,
                streams: streams.into_iter().map(|(k, v)| (k, (v, RwLock::new(None)))).collect(),
            },
            Ok(_) => {
                eprintln!("Machine at {addr:?} did not give a description ({msg_buf:?}).");
                continue;
            }
        };
        let name = Arc::clone(&machine.name);
        let machine = Arc::new(machine);
        {
            let mut guard = state.machines.write().await;
            if guard.contains_key(&name) {
                eprintln!("Machine at {addr:?} tried to connect with a name that already exists: {name}");
                continue;
            }
            guard.insert(name, Arc::clone(&machine));
        }
        tokio::spawn(machine_handler(machine, message_rx, state.message_handler_tx.clone(), rx, tx));
    }
}

async fn machine_stream_listener(state: Arc<ServerState>, machine_stream_srv: TcpListener) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let handle = Handle::current();
    loop {
        let (stream, addr) = machine_stream_srv.accept().await?;
        eprintln!("New machine stream connection from {:?}", addr);
        let (rx, tx) = stream.into_split();

        let mut msg_buf = String::with_capacity(4096);
        let mut rx = BufReader::new(rx);
        let result = rx.read_line(&mut msg_buf).await;
        match result {
            Ok(0) => {
                eprintln!("Machine stream at {addr:?} disconnected without giving a description.");
                continue;
            }
            Err(err) => {
                eprintln!("Error reading stream description at {addr:?}: {err:?}.");
                continue;
            }
            Ok(_) => {}
        };
        let (machine, stream) = match serde_json::from_str::<Message>(&msg_buf) {
            Err(err) => {
                eprintln!("Error parsing stream description at {addr:?}: {err:?}.");
                continue;
            }
            Ok(Message { inner: MessageInner::StreamDescription {
                stream,
                machine,
            }, .. }) => (machine, stream),
            Ok(_) => {
                eprintln!("Machine at {addr:?} did not give a stream description ({msg_buf:?}).");
                continue;
            }
        };
        // Setup stream on a different task, so if it has to wait, it
    }
}

async fn machine_handler(machine: Arc<Machine>, message_rx: Receiver<Message>, message_handler_tx: Sender<MessageWithSource>, mut rx: BufReader<OwnedReadHalf>, mut tx: OwnedWriteHalf) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let machine_receive_handler = async move {
        let mut msg_buf = String::with_capacity(1024);
        loop {
            msg_buf.clear();
            let result = rx.read_line(&mut msg_buf).await;

            match result {
                Ok(0) => {
                    eprintln!("Machine at {addr:?} disconnected.", addr = machine.addr);
                    return Ok(());
                }
                Err(err) => {
                    eprintln!("Error reading machine description at {addr:?}: {err:?}.", addr = machine.addr);
                    return Err(err);
                }
                Ok(_) => {}
            };
            let message = match serde_json::from_str::<Message>(&msg_buf) {
                Err(err) => {
                    eprintln!("Error parsing message from machine at {addr:?}: {err:?}.", addr = machine.addr);
                    continue;
                }
                Ok(msg) => msg,
            };
            message_handler_tx.send((message, MessageSource::Machine(Arc::clone(&machine.name)))).await;
        }
    };

    Ok(tokio::select! {
        res = tokio::spawn(machine_receive_handler) => res??,
    })
}

async fn environment_listener(state: Arc<ServerState>, environment_srv: TcpListener) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        let (stream, addr) = environment_srv.accept().await?;
        eprintln!("New environment connection from {:?}", addr);
        dbg!("TODO");
    }
}

async fn message_handler(state: Arc<ServerState>, mut message_handler_rx: Receiver<MessageWithSource>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut reply_ids: HashMap<i64, MessageSource> = HashMap::new();
    loop {
        match message_handler_rx.recv().await {
            Some((message, source)) => {
                let destination = if let Some(destination) = message.reply_to().map(|id| reply_ids.remove(&id)).flatten() {
                    destination
                } else if let MessageSource::Environment { destination_machine: Some(destination), .. } = &source {
                    MessageSource::Machine(Arc::clone(destination))
                } else {
                    eprintln!("Got message I don't know what to do with ({message:?}, {source:?})");
                    continue;
                };
                let mut destination = match destination {
                    MessageSource::Machine(machine) => {
                        let machines = state.machines.read().await;
                        let machine = match machines.get(&machine) {
                            Some(machine) => machine,
                            None => {
                                eprintln!("TODO: Tried to send message to disconnected machine {machine:?}");
                                continue;
                            }
                        };
                        machine.message_tx.clone()
                    },
                    MessageSource::Environment { environment, .. } => {
                        let environments = state.environments.read().await;
                        let environment = match environments.get(&environment) {
                            Some(environment) => environment,
                            None => {
                                eprintln!("TODO: Tried to send message to disconnected environment {environment:?}");
                                continue;
                            }
                        };
                        environment.message_tx.clone()
                    },
                };

                if message.expects_reply() {
                    reply_ids.insert(message.message_id, source).map(|_| panic!("TODO: handle duplicate message_ids"));
                }

                destination.send(message).await.expect("Failed to send message (buffer full? or destination disconnected?)");
            },
            None => return Ok(()),
        };
    }
}



#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (message_handler_tx, message_handler_rx) = tokio::sync::mpsc::channel(128);
    let state = Arc::new(ServerState::new(message_handler_tx));

    let machine_srv = TcpListener::bind("localhost:45575").await?;
    let environment_srv = TcpListener::bind("localhost:45576").await?;

    let machine_stream_srv = TcpListener::bind("localhost:45577").await?;
    let environment_stream_srv = TcpListener::bind("localhost:45578").await?;

    let res = tokio::try_join!{
        tokio::spawn(machine_listener(Arc::clone(&state), machine_srv)),
        tokio::spawn(environment_listener(Arc::clone(&state), environment_srv)),
        tokio::spawn(machine_stream_listener(Arc::clone(&state), machine_stream_srv)),
        // tokio::spawn(environment_stream_listener(Arc::clone(&state), environment_stream_srv)),
        tokio::spawn(message_handler(state, message_handler_rx)),
    };
    dbg!(res)?;
//    let mut threads = vec![];
#[cfg(any())]
   loop {
        let (stream, addr) = srv.accept()?;
        eprintln!("New connection from {:?}", addr);

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
            // {
            //     let (_, stream) = streams.iter().next().unwrap();
            //     let addr = format!("http://{}:{}", stream.address, stream.port);
            //     std::process::Command::new("firefox")
            //         .args([addr])
            //         .spawn().unwrap();
            // }

            eprintln!("TODO: allocate ports for streams and handle them");
            // let msg = Message::new(
            //     MessageInner::SetupResponse { connected: true }
            // );
            // dbg!(&msg);
            // try_write_message(&write_stream, &msg)?;
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
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);

            let msg = Message::new(
                MessageInner::SensorRead {
                    name: "count".to_owned(),
                },
            );
            dbg!(&msg);
            try_write_message(&write_stream, &msg)?;
            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);
            try_write_message(&write_stream, &msg)?;
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


            let reply = try_read_message(&mut read_stream, None).transpose().unwrap();
            dbg!(&reply);
//            Ok(())
//        }));

   }

    Ok(())
}
