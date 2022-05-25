use std::{cell::RefCell, rc::Rc};
use glib::{MainContext, Cast};
use tokio::{net::TcpStream, io::{AsyncWriteExt}, sync::mpsc::error::TryRecvError};
use common::message::{Message, MessageInner, try_read_message_async, try_write_message_async};
use gio::{traits::ApplicationExt, prelude::ApplicationExtManual};
use gtk::{Application, traits::{GtkApplicationExt, EntryExt, WidgetExt, ContainerExt, ListBoxExt, LabelExt}, Builder, prelude::{BuilderExtManual, DialogExtManual}, Dialog, ResponseType, Entry, MessageType, builders::{MessageDialogBuilder}, ButtonsType, ListBox, Label};

fn main() {
    // Register and include resources
    gio::resources_register_include!("compiled.gresource")
        .expect("Failed to register resources.");

    // Create a new application
    let app = Application::builder()
        .application_id("com.github.mizzoucapstonefrontrow.environment")
        .build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_logic);

    // Run the application
    app.run();
}

async fn run_connect_dialog(app: &Application) -> Option<(String, u16, u16)> {
    let connect_dialog_builder = Builder::from_resource("/mizzoucapstonefrontrow/environment/connect_dialog.ui");
    let connect_dialog: Dialog = connect_dialog_builder.object("connect_dialog").unwrap();
    let connect_dialog_address_entry: Entry = connect_dialog_builder.object("address_entry").unwrap();
    let connect_dialog_server_port_entry: Entry = connect_dialog_builder.object("server_port_entry").unwrap();
    let connect_dialog_stream_port_entry: Entry = connect_dialog_builder.object("stream_port_entry").unwrap();

    app.add_window(&connect_dialog);
    let response = loop {
        // connect_dialog.show();
        let x = connect_dialog.run_future().await;
        connect_dialog.hide();
        match x {
            ResponseType::Cancel | ResponseType::DeleteEvent => {
                break None;
            }
            ResponseType::Ok | ResponseType::Accept => {
                let server = connect_dialog_address_entry.buffer().text();
                let server_port_str = connect_dialog_server_port_entry.buffer().text();
                let stream_port_str = connect_dialog_stream_port_entry.buffer().text();

                let result = match (server, server_port_str.trim().parse::<u16>(), stream_port_str.trim().parse::<u16>()) {
                    (_, server_port@Err(_), _) | (_, server_port, Err(_)) => {
                        let (which, port_str) = if server_port.is_err() {
                            ("server", &server_port_str)
                        } else {
                            ("stream", &stream_port_str)
                        };
                        let error_dialog = MessageDialogBuilder::new()
                            .message_type(MessageType::Error)
                            .text(&format!("Invalid port"))
                            .secondary_text(&format!("Invalid {which} port: {port_str}"))
                            .buttons(ButtonsType::Ok)
                            .build();

                        error_dialog.run_future().await;
                        error_dialog.hide();
                        dbg!("A");
                        continue;
                    }
                    (server, Ok(server_port), Ok(stream_port)) => (server, server_port, stream_port),
                };
                dbg!("A");
                // connect_dialog.close();
                dbg!("A");
                break Some(result);
            }
            _ => continue,
        };
    };
    app.remove_window(&connect_dialog);
    response
}


enum ChooserResult {
    Machine{
        connection: ServerConnection,
        machine: String,
    },
    ConnectError(Box<dyn std::error::Error + Send + Sync + 'static>),
    Back,
    Quit,
}

struct ServerConnection {
    send_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    recv_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    communication_thread: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>,
}

fn connect_to_server(server: &str, server_port: u16, _stream_port: u16) -> Result<ServerConnection, std::io::Error> {
    let stream = match std::net::TcpStream::connect((&*server, server_port))
        .and_then(|s| {s.set_nonblocking(true)?; Ok(s)}
    ) {
        Ok(stream) => stream,
        Err(err) => {
            return Err(err);
        }
    };

    let read_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(err) => return Err(err),
    };
    let write_stream = stream;

    let (send_tx, mut send_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let (recv_tx, mut recv_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let (heartbeat_shortcut_tx, mut heartbeat_shortcut_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let communication_thread = std::thread::spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let mut read_stream = tokio::io::BufReader::new(TcpStream::from_std(read_stream)?);
            let mut write_stream = TcpStream::from_std(write_stream)?;

            let message_sender = async move {
                while let Some(msg) = tokio::select!{
                    msg = send_rx.recv() => msg,
                    msg = heartbeat_shortcut_rx.recv() => msg,
                } {
                    // If either send_tx or heartbeat_shortcut_tx was dropped, then we know this connection is ending and we should stop
                    eprintln!("\x1b[32mWriting message {msg:?}!\x1b[0m");
                    try_write_message_async(&mut write_stream, &msg).await?;
                    write_stream.flush().await?;
                    eprintln!("\x1b[32mWrote message!\x1b[0m");
                }
                eprintln!("\x1b[32msend_tx or heartbeat_shortcut_rx was dropped.\x1b[0m");
                Ok(())
            };
            let message_reciever = async move {
                loop {
                    eprintln!("\x1b[31mReading message!\x1b[0m");
                    // timeout is None so we will either get a message, or the server disconnected
                    let message = match try_read_message_async(&mut read_stream, None).await? {
                        Some(Message { inner: MessageInner::Heartbeat { is_reply }, .. }) => {
                            eprintln!("\x1b[31mRead heartbeat message!\x1b[0m");
                            // Handle heartbeats transparently to the rest of the program
                            if !is_reply {
                                match heartbeat_shortcut_tx.send(Message::new(MessageInner::Heartbeat { is_reply: true })) {
                                    Err(_) => {
                                        // If we failed to send (on an unbounded channel), that means
                                        // the other end disconnected, so we are closing this connection
                                        return Ok(());
                                    }
                                    // Success
                                    _ => {}
                                };
                            }
                            continue;
                        }
                        Some(msg) => msg,
                        None => {
                            // We gave no timeout, so None means the server disconnected
                            return Ok::<_, Box<dyn std::error::Error + Send + Sync + 'static>>(());
                        }
                    };
                    eprintln!("\x1b[31mRead message {message:?}!\x1b[0m");
                    match recv_tx.send(message) {
                        Err(_) => {
                            // If we failed to send (on an unbounded channel), that means
                            // the other end disconnected, so we are closing this connection
                            return Ok(());
                        }
                        // Success
                        _ => {}
                    };
                }
            };
            tokio::try_join!{
                message_sender,
                message_reciever,
            }?;
            Ok(())
        })
    });

    Ok(ServerConnection {
        send_tx,
        recv_rx,
        communication_thread,
    })
}


async fn run_chooser_dialog(app: &Application, server: &str, server_port: u16, stream_port: u16) -> ChooserResult {
    let ServerConnection {
        send_tx,
        mut recv_rx,
        communication_thread,
    } = match connect_to_server(server, server_port, stream_port) {
        Ok(connection) => connection,
        Err(err) => return ChooserResult::ConnectError(err.into()),
    };

    'refresh_loop: loop {
        send_tx.send(Message::new(MessageInner::MachineListRequest{})).expect("Failed to send message");
        eprintln!("\x1b[33mSent machine list request message\x1b[0m");

        struct Disconnected;
        let mut reply = None;
        for _ in 0..100 {
            // https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html
            // Tokio mpsc channels are executor-agnostic, so it's okay
            // that we're .awaiting on glib-rs mainloop
            match recv_rx.recv().await {
                Some(message@Message {inner: MessageInner::MachineListReply { .. }, .. }) => {
                    reply = Some(Ok(message));
                    break;
                }
                Some(message) => {
                    // TODO: maybe should just ignore other message types?
                    reply = Some(Ok(message));
                }
                None => { // Disconnected
                    reply = Some(Err(Disconnected));
                    break;
                },
            };
        }

        let machines = match reply {
            Some(Ok(Message { inner: MessageInner::MachineListReply { machines }, ..})) => machines,
            Some(Ok(_msg)) =>
                return ChooserResult::ConnectError("Server did not reply with machine list (replied with another message type)".into()),
            Some(Err(Disconnected)) =>
                return ChooserResult::ConnectError("Server did not reply with machine list (disconnected)".into()),
            None =>
                return ChooserResult::ConnectError("Server did not reply with machine list (timed out)".into()),
        };


        let chooser_dialog_builder = Builder::from_resource("/mizzoucapstonefrontrow/environment/machine_chooser_dialog.ui");
        let chooser_dialog: Dialog = chooser_dialog_builder.object("machine_chooser_dialog").unwrap();
        let chooser_dialog_machine_list_box: ListBox = chooser_dialog_builder.object("machine_list_box").unwrap();

        let machine_choice: Rc<RefCell<Option<String>>> = Rc::default();

        for machine in machines {
            let label = Label::new(Some(&machine));
            chooser_dialog_machine_list_box.add(&label);
            label.show();
        }

        chooser_dialog_machine_list_box.connect_row_selected({
            let machine_choice = Rc::clone(&machine_choice);
            move |_, row| {
                match row {
                    Some(row) => {
                        let label = row.children().get(0).cloned().expect("ListRow did not contain anything");
                        let label = label.downcast::<Label>().expect("ListRow did not contain a Label");
                        *machine_choice.borrow_mut() = Some(label.text().into());
                    },
                    None => *machine_choice.borrow_mut() = None,
                }
            }
        });



        app.add_window(&chooser_dialog);
        let result = chooser_dialog.run_future().await;
        chooser_dialog.hide();
        app.remove_window(&chooser_dialog);
        match result {
            ResponseType::Yes => {
                let machine = match machine_choice.borrow().clone() {
                    Some(machine) => machine,
                    None => {
                        eprintln!("TODO: Error dialog and refresh when no machine selected");
                        continue 'refresh_loop;
                    }
                };
                // Connect
                return ChooserResult::Machine {
                    connection: ServerConnection {
                        send_tx,
                        recv_rx,
                        communication_thread,
                    },
                    machine,
                };
            },
            ResponseType::No => {
                // Back
                return ChooserResult::Back;
            },
            ResponseType::Other(2) => {
                // Refresh
                continue 'refresh_loop;
            },
            ResponseType::DeleteEvent => {
                return ChooserResult::Quit;
            }
            response => todo!("implement {response:?}"),
        }

    }
}

enum MachineResult {
    /// Go back to machine chooser (currently reconnects; may refactor later to reuse same connection by making write/read streams inputs to run_chooser_dialog)
    /// and having them fields of this variant
    DisconnectMachine,
    /// Go back to connection dialog
    DisconnectServer,
    /// Quit
    Quit,
}

async fn run_machine_window(
    app: &Application,
    send_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    recv_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    communication_thread: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>,
    machine: &str,
) -> MachineResult {
    todo!() as MachineResult
}


fn build_logic(app_: &Application) {
    struct ApplicationHoldGuard {
        app: Application,
    }
    impl ApplicationHoldGuard {
        fn new(app: Application) -> Self {
            app.hold();
            Self { app }
        }
    }
    impl Drop for ApplicationHoldGuard {
        fn drop(&mut self) {
            self.app.release();
        }
    }

    let _app_guard = ApplicationHoldGuard::new(app_.clone());
    let app = app_.clone();
    let main_logic = async move {
        let _app_guard = _app_guard;

        'connect_dialog: while let Some((server, server_port, stream_port)) = run_connect_dialog(&app).await {
            'machine_chooser: loop {
                match run_chooser_dialog(&app, &server, server_port, stream_port).await {
                    ChooserResult::Machine {
                        connection: ServerConnection { send_tx, recv_rx, communication_thread },
                        machine,
                    } => {
                        match run_machine_window(&app, send_tx, recv_rx, communication_thread, &machine).await {
                            MachineResult::DisconnectMachine => continue 'machine_chooser,
                            MachineResult::DisconnectServer => continue 'connect_dialog,
                            MachineResult::Quit => return,
                        };
                    },
                    ChooserResult::ConnectError(err) => {
                        let error_dialog = MessageDialogBuilder::new()
                            .message_type(MessageType::Error)
                            .text(&format!("Failed to connect to server"))
                            .secondary_text(&format!("Failed to connect to server: {err:?}"))
                            .buttons(ButtonsType::Ok)
                            .build();

                        error_dialog.run_future().await;
                        error_dialog.hide();
                        continue 'connect_dialog;
                    },
                    ChooserResult::Back => continue 'connect_dialog,
                    ChooserResult::Quit => return,
                }
            }
        }
    };

    MainContext::default().spawn_local(main_logic);
}
