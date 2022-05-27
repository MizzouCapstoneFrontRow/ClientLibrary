use std::{collections::HashMap, sync::Arc, time::Duration, net::SocketAddr};
use tokio::{
    net::{TcpListener, tcp::{OwnedReadHalf, OwnedWriteHalf}},
    sync::{
        RwLock,
        mpsc::{Sender, Receiver, error::TrySendError},
        broadcast::{Sender as BroadcastSender, Receiver as BroadcastReveiver, error::RecvError as BroadcastRecvError}
    },
    io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, runtime::Handle};
use common::{message::*, unwrap_or_return, jpeg::{ImageData, read_jpeg}};

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
    description: MessageInner,
    addr: SocketAddr,
    message_tx: Sender<Message>,
    streams: HashMap<Arc<str>, (Stream, RwLock<Option<BroadcastSender<Arc<ImageData>>>>)>,
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
            Ok(Message { inner: inner@MessageInner::MachineDescription {..}, .. }) => {
                let (name, streams) = match &inner {
                    MessageInner::MachineDescription { name, streams, .. } => (name, streams),
                    _ => unreachable!()
                };
                Machine {
                    name: (&**name).into(),
                    addr,
                    message_tx,
                    streams: streams.iter().map(|(k, v)| (k.clone(), (v.clone(), RwLock::new(None)))).collect(),
                    description: inner,
                }
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
        let (rx, _) = stream.into_split();
        let mut rx = BufReader::new(rx);
        
        let message = try_read_message_async(&mut rx, None).await;
        let (machine_name, stream_name) = match message {
            Err(err) => {
                eprintln!("Error parsing stream description at {addr:?}: {err:?}.");
                continue;
            }
            Ok(Some(Message { inner: MessageInner::StreamDescription {
                stream,
                machine,
            }, .. })) => (machine, stream),
            Ok(msg) => {
                eprintln!("Machine at {addr:?} did not give a stream description ({msg:?}).");
                continue;
            }
        };
        let machine = {
            let mut guard = state.machines.write().await;
            match guard.get(&machine_name) {
                Some(machine) => machine.clone(),
                None => {
                    eprintln!("Machine stream at {addr:?} gave an unknown machine name {machine_name:?}.");
                    continue;
                },
            }
        };

        // Buffer 10 frames
        let (image_tx, _) = tokio::sync::broadcast::channel::<Arc<ImageData>>(10);
        let stream = match machine.streams.get(&stream_name) {
            Some(stream) => stream,
            None => {
                eprintln!("Machine stream at {addr:?} gave an unknown stream name {stream_name:?}.");
                continue;
            },
        };

        let handler_fn = match &*stream.0.format {
            "jpeg" | "mjpeg" => machine_jpeg_stream_handler,
            format => {
                eprintln!("Machine stream at {addr:?} gave an unknown format {format:?}.");
                continue;
            }
        };

        *stream.1.write().await = Some(image_tx.clone());

        // Setup stream on a different task, so if it has to wait, it doesn't block this task
        tokio::spawn(handler_fn(machine_name, stream_name, rx, image_tx));
    }
}

async fn connection_handler(
    source_: MessageSource,
    addr: SocketAddr,
    mut message_rx: Receiver<Message>,
    message_handler_tx_: Sender<MessageWithSource>,
    mut rx: BufReader<OwnedReadHalf>,
    mut tx: OwnedWriteHalf
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let source = source_.clone();
    let message_handler_tx = message_handler_tx_.clone();
    let receive_handler = async move {
        loop {
            let message = match try_read_message_async(&mut rx, None).await {
                Err(TryReadMessageError::EOF) => {
                    eprintln!("Connection at {addr:?} disconnected ({source:?}).");
                    // This message also tells the handler to remove this source from the server state.
                    message_handler_tx.send((Message::new(MessageInner::Disconnect {}), source.clone())).await.expect("Failed to send message to handler");
                    return Ok(());
                }
                Err(err) => {
                    eprintln!("Error reading message from {addr:?}: {err:?}.");
                    return Err(err);
                }
                Ok(Some(message)) => message,
                Ok(None) => unreachable!("timeout was zero"),
            };
            message_handler_tx.send((message, source.clone())).await.expect("Failed to send message to handler");
        }
    };

    let source = source_;
    let message_handler_tx = message_handler_tx_;
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
                Err(TryWriteMessageError::Disconnected(err)) => {
                    // This message also tells the handler to remove this source from the server state.
                    message_handler_tx.send((Message::new(MessageInner::Disconnect {}), source.clone())).await.expect("Failed to send message to handler");
                    eprintln!("Error writing message to {addr:?}: {err:?} (disconnected).");
                    return Err(err.into());
                }
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

async fn machine_jpeg_stream_handler(
    machine: Arc<str>,
    stream: Arc<str>,
    mut rx: BufReader<OwnedReadHalf>,
    image_tx: BroadcastSender<Arc<ImageData>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        eprintln!("Reading a jpeg...");
        match read_jpeg(&mut rx).await {
            Ok(img) => match image_tx.send(Arc::new(img)) {
                Ok(_n) => {}, // Sent to _n environments
                Err(_err) => {}, // No environments are currently listening
            },
            Err(err) => {
                eprintln!("Machine {machine:?} stream {stream:?} failed: {err:?}.");
                return Err(err.into());
            },
        };
    }
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

async fn environment_stream_listener(
    state: Arc<ServerState>,
    environment_stream_srv: TcpListener
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        let (stream, addr) = environment_stream_srv.accept().await?;
        eprintln!("New environment stream connection from {addr:?}");
        let (rx, tx) = stream.into_split();
        let mut rx = BufReader::new(rx);
        
        let message = try_read_message_async(&mut rx, None).await;
        eprintln!("TODO: make env stream request a different message type");
        let (machine_name, stream_name) = match message {
            Err(err) => {
                eprintln!("Error parsing stream description at {addr:?}: {err:?}.");
                continue;
            }
            Ok(Some(Message { inner: MessageInner::StreamDescription {
                stream,
                machine,
            }, .. })) => (machine, stream),
            Ok(msg) => {
                eprintln!("Machine at {addr:?} did not give a stream description ({msg:?}).");
                continue;
            }
        };

        let mut image_rx = {
            let machine = {
                let guard = state.machines.read().await;
                match guard.get(&machine_name) {
                    Some(machine) => machine.clone(),
                    None => {
                        eprintln!("Environment requested stream from unknown machine {machine_name:?}");
                        continue;
                    },
                }
            };
            match machine.streams.get(&stream_name) {
                Some((_, sender)) => match &*sender.read().await {
                    Some(sender) => sender.subscribe(),
                    None => {
                        eprintln!("Environment requested stream {stream_name:?} from machine {machine_name:?}, but it has not yet connected");
                        eprintln!("TODO: spawn this on a separate task and wait until it connects?");
                        continue;
                    },
                },
                None => {
                    eprintln!("Environment requested unknown stream {stream_name:?} from machine {machine_name:?}");
                    continue;
                },
            }
        };

        tokio::spawn(environment_stream_handler(addr, machine_name, stream_name, tx, image_rx));
    }
}

async fn environment_stream_handler(
    addr: SocketAddr,
    machine_name: Arc<str>,
    stream_name: Arc<str>,
    mut tx: OwnedWriteHalf,
    mut image_rx: BroadcastReveiver<Arc<ImageData>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        match image_rx.recv().await {
            Ok(image) => match tx.write_all(&image).await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Failed to write image to environment {addr:?} (stream: {machine_name:?}, {stream_name:?}))");
                    return Err(e.into());
                },
            },
            Err(BroadcastRecvError::Lagged(frames)) => {
                eprintln!("LOG: environment {addr:?} skipping {frames} frames");
            },
            Err(BroadcastRecvError::Closed) => {
                // Stream ended
                return Ok(());
            },
        };
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
                        eprintln!("Failed to write heartbeat to machine {name:?} (stream closed) (this machine should be removed soon)");
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
                        eprintln!("Failed to write heartbeat to environment {addr:?} (stream closed) (this environment should be removed soon)");
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
        tokio::spawn(environment_stream_listener(Arc::clone(&state), environment_stream_srv)),
        tokio::spawn(message_handler(Arc::clone(&state), message_handler_rx)),
        tokio::spawn(heartbeat(state)),
    };
    dbg!(res)?;

    Ok(())
}
