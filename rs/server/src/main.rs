use std::{collections::HashMap, sync::Arc, time::Duration, net::SocketAddr};
use tokio::{net::{TcpListener, tcp::{OwnedReadHalf, OwnedWriteHalf}}, sync::{RwLock, mpsc::{Sender, Receiver, error::TrySendError}}, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, runtime::Handle};
use common::{message::*, unwrap_or_return};

#[derive(Debug, Clone)]
enum MessageSource {
    Machine(Arc<str>),
    Environment(SocketAddr),
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
    functions: HashMap<Arc<str>, Function>,
    sensors: HashMap<Arc<str>, Sensor>,
    axes: HashMap<Arc<str>, Axis>,
    streams: HashMap<Arc<str>, (Stream, RwLock<Option<()>>)>,
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
            guard.insert(Arc::clone(&name), machine);
        }
        let source = MessageSource::Machine(name);
        tokio::spawn(connection_handler(source, addr, message_rx, state.message_handler_tx.clone(), rx, tx));
    }
}

#[allow(unused)]
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
        // Setup stream on a different task, so if it has to wait, it doesn't block this task
        eprintln!("TODO: connect machine streams");
    }
}

async fn connection_handler(
    source: MessageSource,
    addr: SocketAddr,
    mut message_rx: Receiver<Message>,
    message_handler_tx: Sender<MessageWithSource>,
    mut rx: BufReader<OwnedReadHalf>,
    mut tx: OwnedWriteHalf
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let source_ = source.clone();
    let receive_handler = async move {
        loop {
            let message = match try_read_message_async(&mut rx, None).await {
                Err(TryReadMessageError::EOF) => {
                    eprintln!("Connection at {addr:?} disconnected ({source:?}).");
                    // This message also tells the handler to remove this source from the server state.
                    message_handler_tx.send((Message::new(MessageInner::Disconnect {}), source_.clone())).await.expect("Failed to send message to handler");
                    return Ok(());
                }
                Err(err) => {
                    eprintln!("Error reading message from {addr:?}: {err:?}.");
                    return Err(err);
                }
                Ok(Some(message)) => message,
                Ok(None) => unreachable!("timeout was zero"),
            };
            message_handler_tx.send((message, source_.clone())).await.expect("Failed to send message to handler");
        }
    };

    let send_handler = async move {
        loop {
            let msg = unwrap_or_return!(
                message_rx.recv().await,
                Ok(()),
                with_message "Message sender was dropped (recv returned None)"
            );
            let r1 = try_write_message_async(&mut tx, &msg).await;
            let r2 = tx.flush().await;
            // TODO: Load-bearing heartbeats. Flush doesn't seem to work, i.e. the "last" message isn't necessarily actually sent,
            // it appears, so heartbeats must be sent to ensure each message goes through.
            match r1.and(r2.map_err(Into::into)) {
                Err(err) => {
                    eprintln!("Error writing message to {addr:?}: {err:?}.");
                    return Err(err);
                }
                Ok(()) => {}
            }
        }
    };

    let res = tokio::try_join! {
        tokio::spawn(receive_handler),
        tokio::spawn(send_handler),
    }?;
    res.0?;
    res.1?;
    Ok(())
}

async fn environment_listener(state: Arc<ServerState>, environment_srv: TcpListener) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        let (stream, addr) = environment_srv.accept().await?;
        eprintln!("New environment connection from {:?}", addr);
        let (rx, tx) = stream.into_split();

        let rx = BufReader::new(rx);

        let (message_tx, message_rx) = tokio::sync::mpsc::channel(16);

        let environment = Arc::new(Environment {
            addr,
            message_tx,
        });
        {
            let mut guard = state.environments.write().await;
            if guard.contains_key(&addr) {
                eprintln!("Machine at {addr:?} tried to connect with an address that already exists: {addr}");
                continue;
            }
            guard.insert(addr, environment);
        }
        let source = MessageSource::Environment(addr);
        tokio::spawn(connection_handler(source, addr, message_rx, state.message_handler_tx.clone(), rx, tx));
    }
}

async fn message_handler(state: Arc<ServerState>, mut message_handler_rx: Receiver<MessageWithSource>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut reply_ids: HashMap<i64, MessageSource> = HashMap::new();
    loop {
        match message_handler_rx.recv().await {
            Some((mut message, source)) => {
                let destination = if let Some(destination) = message.reply_to() {
                    match reply_ids.remove(&destination) {
                        Some(destination) => destination,
                        None => {
                            eprintln!("Received reply for message with unrecognized id: {destination:?} ({message:?})");
                            continue;
                        }
                    }
                } else if let Some(destination) = message.destination_machine() {
                    MessageSource::Machine(destination.into())
                } else {
                    use common::NodeType::*;
                    match message.route() {
                        (_, Server | Any) => match &message.inner {
                            MessageInner::MachineDescription { .. } => {
                                eprintln!("Received unexpected machine description from {source:?}");
                                continue;
                            }
                            MessageInner::Disconnect {  } => {
                                match source {
                                    MessageSource::Machine(machine) => {
                                        state.machines.write().await.remove(&machine);
                                    }
                                    MessageSource::Environment(environment) => {
                                        state.environments.write().await.remove(&environment);
                                    }
                                };
                                continue;
                            },
                            MessageInner::StreamDescription { .. } => {
                                eprintln!("Received unexpected stream description from {source:?}");
                                continue;
                            }
                            MessageInner::Heartbeat { is_reply } => {
                                if *is_reply { eprintln!("Received heartbeat reply"); continue; }
                                eprintln!("Received heartbeat request");
                                message = Message::new(MessageInner::Heartbeat { is_reply: true });
                                source.clone() // Send heartbeat reply back to source
                            },
                            MessageInner::MachineListRequest {} => {
                                let machines = state.machines.read().await;
                                let machines = machines.iter().map(|(name, _)| name.clone()).collect();
                                message = Message::new(MessageInner::MachineListReply { machines });
                                source.clone() // Send reply back to source
                            }
                            _ => {
                                eprintln!("Received unexpected message from {source:?} ({message:?})");
                                continue;
                            }
                        },
                        _ => {
                            eprintln!("Received unexpected message from {source:?} ({message:?})");
                            continue;
                        }
                    }
                };
                let destination = match destination {
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
                    MessageSource::Environment(environment) => {
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

                if message.expects_forwarded_reply() {
                    reply_ids.insert(message.message_id, source).map(|_| eprintln!("TODO: handle duplicate message_ids"));
                }

                destination.send(message).await.expect("Failed to send message (buffer full? or destination disconnected?)");
            },
            None => return Ok(()),
        };
    }
}

async fn heartbeat(state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        {
            let machines = state.machines.read().await;
            for (name, machine) in &*machines {
                match machine.message_tx.try_send(Message::new(MessageInner::Heartbeat { is_reply: false })) {
                    Ok(()) => {},
                    Err(TrySendError::Full(_)) => {
                        eprintln!("Failed to write heartbeat to machine {name:?} (buffer full)");
                    },
                    Err(TrySendError::Closed(_)) => {
                        eprintln!("Failed to write heartbeat to machine {name:?} (stream closed)");
                    },
                }
            }
        }
        {
            let environments = state.environments.read().await;
            for (addr, environment) in &*environments {
                match environment.message_tx.try_send(Message::new(MessageInner::Heartbeat { is_reply: false })) {
                    Ok(()) => {},
                    Err(TrySendError::Full(_)) => {
                        eprintln!("Failed to write heartbeat to environment {addr:?} (buffer full)");
                    },
                    Err(TrySendError::Closed(_)) => {
                        eprintln!("Failed to write heartbeat to environment {addr:?} (stream closed)");
                    },
                }
            }
        }
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
        tokio::spawn(message_handler(Arc::clone(&state), message_handler_rx)),
        tokio::spawn(heartbeat(state)),
    };
    dbg!(res)?;

    Ok(())
}
