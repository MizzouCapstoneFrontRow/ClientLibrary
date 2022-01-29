#![deny(unsafe_op_in_unsafe_fn)]
#[macro_use]
pub(crate) mod util;
//pub(crate) mod native_callback;
pub(crate) mod callbacks;
pub(crate) mod marshall;
pub mod message;

use std::{
    ffi::CStr,
    ptr::NonNull,
    sync::Arc,
    collections::HashMap,
};
use libc::{c_char, c_void};
use indexmap::map::IndexMap; 
use util::*;
use callbacks::*;
use message::{Message, MessageInner, try_read_message, try_write_message};

pub type Stream = ();

#[derive(Default)]
pub struct UnconnectedClient {
    name: Option<String>,
    streams: HashMap<String, Stream>,
    sensors: HashMap<String, Sensor>,
    axes: HashMap<String, Axis>,
    functions: HashMap<String, Function>,
}

pub struct ConnectedClient {
    name: String,
    streams: HashMap<String, Stream>,
    sensors: HashMap<String, Sensor>,
    axes: HashMap<String, Axis>,
    functions: HashMap<String, Function>,
    connection: std::net::TcpStream,
}

pub enum ClientHandle {
    Unconnected(UnconnectedClient),
    Connected(ConnectedClient),
}

use ClientHandle::*;

impl ClientHandle {
    fn as_unconnected_mut(&mut self) -> Result<&mut UnconnectedClient, &mut ConnectedClient> {
        match self { Unconnected(c) => Ok(c), Connected(c) => Err(c) }
    }
    fn as_connected_mut(&mut self) -> Result<&mut ConnectedClient, &mut UnconnectedClient> {
        match self { Connected(c) => Ok(c), Unconnected(c) => Err(c) }
    }
}



#[no_mangle]
pub extern "C" fn InitializeLibrary() -> Option<Box<ClientHandle>> {
    Some(Box::new(
        ClientHandle::Unconnected(
            Default::default()
        )
    ))
}

#[no_mangle]
pub extern "C" fn ShutdownLibrary(handle: Option<Box<ClientHandle>>) {
    shadow_or_return!(mut handle, ());
    match *handle {
        Unconnected(_) => {}, // nothing to do
        Connected(handle) => {
            try_write_message(&handle.connection, &Message::new(MessageInner::Disconnect {})); // TODO: error handle
        },
    };
}

#[no_mangle]
pub extern "C" fn SetName(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>
) -> bool {
    shadow_or_return!(mut handle, false, with_message "Error setting name: Invalid handle (null)");
    shadow_or_return!(name,       false, with_message "Error setting name: Invalid name (null)");
    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false, with_message "Error setting name: Cannot set name after connecting to server.");
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
        false,
        with_message "Error setting name: Invalid name (not UTF-8)",
    );
    handle.name = Some(name.to_owned());
    true
}

#[no_mangle]
pub extern "C" fn LibraryUpdate(handle: Option<&mut ClientHandle>) -> bool {
    shadow_or_return!(handle, false, with_message "Error updating: Invalid handle (null)");
    let handle = unwrap_or_return!(handle.as_connected_mut(), false, with_message "Error updating: Cannot update before connecting to server.");
    while let Ok(Some(message)) = message::try_read_message(&handle.connection) {
        eprintln!("TODO: handle I/O errors in LibraryUpdate");
        dbg!(&message);

        use crate::message::Message;
        use crate::message::MessageInner::*;
        match message.inner {
            FunctionCall { name, parameters } => {
                if let Some(function) = handle.functions.get(&name) {
                    let result = function.call(&parameters).unwrap(); // TODO: error handle
                    dbg!(&result);
                    let reply = Message::new(
                        FunctionReturn {
                            reply_to: message.message_id,
                            returns: result,
                        },
                    );
                    dbg!(&reply);
                    message::try_write_message(&handle.connection, &reply);// TODO: error handle
                } else {
                    eprintln!("TODO: reply with unsupported operation");
//                    Message
                }
            },
            _ => {todo!()},
        }
        
    }

//    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false);
//    eprintln!("TODO: LibraryUpdate");
//
//    let result = handle.functions.get("print").unwrap().call(serde_json::from_str(
//        r#"{ "name": "Zachary" }"#
//    ).unwrap()).unwrap();
//    dbg!(serde_json::to_string(&result));
//
//
//    let result = handle.functions.get("multiply").unwrap().call(serde_json::from_str(
//        r#"{ "x": 4, "y": 5}"#
//    ).unwrap()).unwrap();
//    dbg!(serde_json::to_string(&result));
//
//
//    let result = handle.functions.get("average").unwrap().call(serde_json::from_str(
//        r#"{ "x": [1, 2, 3, 4, 5, 20]}"#
//    ).unwrap()).unwrap();
//    dbg!(serde_json::to_string(&result));
//
//
//    let result = handle.functions.get("sequence").unwrap().call(serde_json::from_str(
//        r#"{ "n": 20}"#
//    ).unwrap()).unwrap();
//    dbg!(serde_json::to_string(&result));
//
//
//    let result = handle.functions.get("count_bools").unwrap().call(serde_json::from_str(
//        r#"{ "values": [true, true, false, true, false]}"#
//    ).unwrap()).unwrap();
//    dbg!(serde_json::to_string(&result));


    true
}

unsafe fn parse_descriptors(descriptors: *const [*const c_char; 2]) -> Result<IndexMap<String, Type>, &'static str> {
    let slice = if descriptors.is_null() {
        &[]
    } else {
        let mut count = 0;
        loop {
            let ptr = unsafe { descriptors.add(count) };
            let descriptor = unsafe { &*ptr };
            if descriptor.iter().copied().any(<*const c_char>::is_null) {
                break unsafe {
                    std::slice::from_raw_parts(descriptors as *const [NonNull<c_char>; 2], count) // TODO
                };
            }
            count += 1;
        }
    };
    let mut map = IndexMap::with_capacity(slice.len());
    for [name, r#type] in slice {
        let name: &str = unwrap_or_return!(
            unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
            Err("invalid (non-UTF8) name"),
        );
        let r#type: &str = unwrap_or_return!(
            unsafe { CStr::from_ptr(r#type.as_ptr()) }.to_str(),
            Err("invalid (non-UTF8) type"),
        );
        let r#type = unwrap_or_return!(
            Type::from_str(r#type),
            Err("unrecognized type"),
        );
        map.insert(name.to_owned(), r#type);
    }
    Ok(map)
}


#[no_mangle]
pub extern "C" fn RegisterFunction(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>,
    parameters: *const [*const c_char; 2],
    returns: *const [*const c_char; 2],
    callback: Option<extern "C" fn (*const *const c_void, *const *mut c_void)>,
) -> bool {
    shadow_or_return!(handle,   false, with_message "Error registering function: Invalid handle (null)");
    shadow_or_return!(callback, false, with_message "Error registering function: Invalid callback (null)");
    shadow_or_return!(name,     false, with_message "Error registering function: Invalid name (null)");
    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false, with_message "Error registering function: Cannot register functions after connecting to server.");
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
        false,
        with_message "Error registering function: Invalid handle (null)",
    );

    if handle.functions.contains_key(name) {
        eprintln!("Error registering function: attempted to register function {:?}, but a function with that name was already registered.", name);
        return false;
    }

    let parameters = unwrap_or_return!(
        unsafe { parse_descriptors(parameters) },
        false,
        with_message(s) "Error parsing function parameters: {}", s,
    );
    let returns = unwrap_or_return!(
        unsafe { parse_descriptors(returns) },
        false,
        with_message(s) "Error parsing function returns: {}", s,
    );

    dbg!(&parameters);
    dbg!(&returns);

    let function = unwrap_or_return!(
        Function::new(parameters, returns, callback),
        false,
        with_message(e) "Error registering function: {:?}", e
    );

    handle.functions.insert(name.to_owned(), function);
    true
}

#[no_mangle]
pub extern "C" fn RegisterSensor(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>,
    output_type: Option<NonNull<c_char>>,
    callback: Option<extern "C" fn (*mut c_void)>,
) -> bool {
    shadow_or_return!(handle,       false, with_message "Error registering sensor: Invalid handle (null)");
    shadow_or_return!(callback,     false, with_message "Error registering sensor: Invalid callback (null)");
    shadow_or_return!(name,         false, with_message "Error registering sensor: Invalid name (null)");
    shadow_or_return!(output_type,  false, with_message "Error registering sensor: Invalid output type (null)");
    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false, with_message "Error registering sensor: Cannot register sensors after connecting to server.");
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
        false,
        with_message "Error registering sensor: Invalid name (not UTF-8)",
    );
    let output_type: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(output_type.as_ptr()) }.to_str(),
        false,
        with_message "Error registering sensor: Invalid output type (not UTF-8)",
    );

    if handle.sensors.contains_key(name) {
        eprintln!("Attempted to register sensor {:?}, but a function with that name was already registered.", name);
        return false;
    }

    let output_type = unwrap_or_return!(
        Type::from_str(output_type),
        false,
        with_message "Error registering axis: Unrecognized type when parsing sensor output type",
    );

    let sensor = unwrap_or_return!(
        Sensor::new(output_type, callback),
        false,
        with_message(e) "Error registering sensor: {:?}", e
    );

    handle.sensors.insert(name.to_owned(), sensor);
    true
}


#[no_mangle]
pub extern "C" fn RegisterAxis(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>,
    input_type: Option<NonNull<c_char>>,
    callback: Option<extern "C" fn (*const c_void)>,
) -> bool {
    shadow_or_return!(handle,     false, with_message "Error registering axis: Invalid handle (null)");
    shadow_or_return!(callback,   false, with_message "Error registering axis: Invalid callback (null)");
    shadow_or_return!(name,       false, with_message "Error registering axis: Invalid name (null)");
    shadow_or_return!(input_type, false, with_message "Error registering axis: Invalid input type (null)");
    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false, with_message "Error registering axis: Cannot register axes after connecting to server.");
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
        false,
        with_message "Error registering axis: Invalid name (not UTF-8)",
    );
    let input_type: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(input_type.as_ptr()) }.to_str(),
        false,
        with_message "Error registering axis: Invalid output type (not UTF-8)",
    );

    if handle.axes.contains_key(name) {
        eprintln!("Error registering axis: Attempted to register axis {:?}, but an axis  with that name was already registered.", name);
        return false;
    }

    let input_type = unwrap_or_return!(
        Type::from_str(input_type),
        false,
        with_message "Error registering axis: Unrecognized type when parsing axis input type",
    );

    let axis = unwrap_or_return!(
        Axis::new(input_type, callback),
        false,
        with_message(e) "Error registering axis: {:?}", e
    );

    handle.axes.insert(name.to_owned(), axis);
    true
}

#[no_mangle]
pub extern "C" fn ConnectToServer(
    handle: Option<&mut ClientHandle>,
    server: Option<NonNull<c_char>>,
    port: u16,
) -> bool {
    shadow_or_return!(handle, false, with_message "Error connecting to server: Invalid handle (null)");
    shadow_or_return!(server, false, with_message "Error connecting to server: Invalid server (null)");
    let handle_ = handle;
    let handle = unwrap_or_return!(
        handle_.as_unconnected_mut(),
        false,
        with_message "Error connecting to server: already connected",
    );
    if handle.name.is_none() {
        eprintln!("Error connecting to server: no name set");
        return false;
    }
    let server: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(server.as_ptr()) }.to_str(),
        false,
        with_message "Error connecting to server: server address not valid UTF-8",
    );
    let connection = unwrap_or_return!(
        std::net::TcpStream::connect((server, port)),
        false,
        with_message(e) "Error connecting to server: {:?}", e
    );
    eprintln!("TODO: send machine description to server");

    let UnconnectedClient {
        name, sensors, axes, functions, streams
    } = std::mem::take(handle);

    *handle_ = ClientHandle::Connected(ConnectedClient {
        name: name.unwrap(),
        sensors, axes, functions, streams,
        connection,
    });

    true
}
