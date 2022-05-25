use std::{cell::RefCell, rc::Rc};
use glib::{MainContext, Cast};
use tokio::{net::TcpStream, io::AsyncWriteExt};
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

async fn show_error_dialog(text: impl AsRef<str>, secondary_text: impl AsRef<str>) {
    let error_dialog = MessageDialogBuilder::new()
        .message_type(MessageType::Error)
        .text(text.as_ref())
        .secondary_text(secondary_text.as_ref())
        .buttons(ButtonsType::Ok)
        .build();

    error_dialog.run_future().await;
    error_dialog.hide();
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
                        show_error_dialog(
                            format!("Invalid port"),
                            format!("Invalid {which} port: {port_str}")
                        ).await;
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
    Machine(String),
    ConnectError(&'static str),
    Back,
    Quit,
}

struct ServerConnection {
    send_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    recv_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    communication_thread: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>,
}

async fn connect_to_server(server: &str, server_port: u16, _stream_port: u16) -> Result<ServerConnection, std::io::Error> {
    let server = server.to_owned();

    let (connection_tx, connection_rx) = tokio::sync::oneshot::channel();

    let communication_thread = std::thread::spawn(move || -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let stream = match TcpStream::connect((&*server, server_port)).await {
                Ok(stream) => stream,
                Err(err) => {
                    connection_tx.send(Err(err)).unwrap();
                    return Ok(());
                },
            };

            let (read_stream, mut write_stream) = stream.into_split();
            let mut read_stream = tokio::io::BufReader::new(read_stream);

            let (send_tx, mut send_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
            let (recv_tx, recv_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

            let (heartbeat_shortcut_tx, mut heartbeat_shortcut_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

            let connection = (send_tx, recv_rx);
            connection_tx.send(Ok(connection)).unwrap();


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

    let unused = eprintln!("TODO: This is assuming it is safe to use tokio::sync::oneshot channels from other runtimes");
    let (send_tx, recv_rx) = connection_rx.await.unwrap()?;

    Ok(ServerConnection {
        send_tx,
        recv_rx,
        communication_thread,
    })
}


async fn run_chooser_dialog(app: &Application, server: &mut ServerConnection) -> ChooserResult {
    let ServerConnection {
        send_tx,
        recv_rx,
        ..
    } = server;

    'refresh_loop: loop {
        match send_tx.send(Message::new(MessageInner::MachineListRequest{})) {
            Ok(_) => {},
            Err(_) => {
                // Server disconnected
                return ChooserResult::ConnectError("Server disconnected before machine list request was sent".into());
            },
        }
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
                return ChooserResult::Machine(machine);
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

#[allow(unused)]
async fn run_machine_window(
    app: &Application,
    server_connection: &mut ServerConnection,
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
            let mut server_connection = match connect_to_server(&server, server_port, stream_port).await {
                Ok(connection) => connection,
                Err(err) => {
                    show_error_dialog(
                        format!("Failed to connect to server"),
                        format!("Failed to connect to server: {err:?}"),
                    ).await;
                    continue 'connect_dialog;
                },
            };
            'machine_chooser: loop {
                match run_chooser_dialog(&app, &mut server_connection).await {
                    ChooserResult::Machine(machine) => {
                        match run_machine_window(&app, &mut server_connection, &machine).await {
                            MachineResult::DisconnectMachine => continue 'machine_chooser,
                            MachineResult::DisconnectServer => continue 'connect_dialog,
                            MachineResult::Quit => return,
                        };
                    },
                    ChooserResult::ConnectError(msg) => {
                        show_error_dialog(
                            format!("Failed to connect to server"),
                            format!("Failed to connect to server: {msg}"),
                        ).await;
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
