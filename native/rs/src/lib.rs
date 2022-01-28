#![deny(unsafe_op_in_unsafe_fn)]
#[macro_use]
pub(crate) mod util;
//pub(crate) mod native_callback;
pub(crate) mod callbacks;
pub(crate) mod marshall;

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
        Connected(c) => {
            todo!("send disconnect message");
        },
    };
}

#[no_mangle]
pub extern "C" fn SetName(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>
) -> bool {
    shadow_or_return!(mut handle, false);
    shadow_or_return!(name, false);
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
        false,
    );
    match handle {
        Unconnected(c) => {
            c.name = Some(name.to_owned());
            true
        },
        Connected(_) => {
            false // Cannot change name after conncting
        },
    }
}

#[no_mangle]
pub extern "C" fn LibraryUpdate(handle: Option<&mut ClientHandle>) -> bool {
    shadow_or_return!(handle, false);
//    let handle = unwrap_or_return!(handle.as_connected_mut(), false);
    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false);
    eprintln!("TODO: LibraryUpdate");

    let result = handle.functions.get("print").unwrap().call(serde_json::from_str(
        r#"{ "name": "Zachary" }"#
    ).unwrap()).unwrap();
    dbg!(serde_json::to_string(&result));


    let result = handle.functions.get("multiply").unwrap().call(serde_json::from_str(
        r#"{ "x": 4, "y": 5}"#
    ).unwrap()).unwrap();
    dbg!(serde_json::to_string(&result));


    let result = handle.functions.get("average").unwrap().call(serde_json::from_str(
        r#"{ "x": [1, 2, 3, 4, 5, 20]}"#
    ).unwrap()).unwrap();
    dbg!(serde_json::to_string(&result));


    let result = handle.functions.get("sequence").unwrap().call(serde_json::from_str(
        r#"{ "n": 20}"#
    ).unwrap()).unwrap();
    dbg!(serde_json::to_string(&result));


    let result = handle.functions.get("count_bools").unwrap().call(serde_json::from_str(
        r#"{ "values": [true, true, false, true, false]}"#
    ).unwrap()).unwrap();
    dbg!(serde_json::to_string(&result));


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
    shadow_or_return!(handle, false);
    shadow_or_return!(callback, false);
    let handle = unwrap_or_return!(handle.as_unconnected_mut(), false);
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(unwrap_or_return!(name, false).as_ptr()) }.to_str(),
        false,
    );

    if handle.functions.contains_key(name) {
        eprintln!("Attempted to register function {:?}, but a function with that name was already registered.", name);
        return false;
    }

    let parameters = match unsafe { parse_descriptors(parameters) } {
        Ok(p) => p,
        Err(s) => { eprintln!("Error parsing parameters: {}", s); return false },
    };
    let returns = match unsafe { parse_descriptors(returns) } {
        Ok(r) => r,
        Err(s) => { eprintln!("Error parsing returns: {}", s); return false },
    };

    dbg!(&parameters);
    dbg!(&returns);

    let function = match Function::new(
        parameters,
        returns,
        callback,
    ) {
        Ok(f) => f,
        Err(e) => { eprintln!("Error registering function: {:?}", e); return false },
    };

    handle.functions.insert(name.to_owned(), function);
    true
}

