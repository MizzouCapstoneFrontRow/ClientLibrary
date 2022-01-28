#![deny(unsafe_op_in_unsafe_fn)]
#[macro_use]
pub(crate) mod util;
//pub(crate) mod native_callback;
pub(crate) mod callbacks;

use std::{
    ffi::CStr,
    ptr::NonNull,
    sync::Arc,
    collections::HashMap,
};
use libc::{c_char, c_void};
use indexmap::map::IndexMap; 
use util::*;
use marshall::*;

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
    let handle = unwrap_or_return!(handle.as_connected_mut(), false);
    todo!("LibraryUpdate");
    true
}

unsafe fn parse_descriptors(descriptors: *const [*const c_char; 2]) -> Option<IndexMap<String, Type>> {
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
            None,
        );
        let r#type: &str = unwrap_or_return!(
            unsafe { CStr::from_ptr(r#type.as_ptr()) }.to_str(),
            None,
        );
        let r#type = unwrap_or_return!(
            Type::from_str(r#type),
            None,
        );
        map.insert(name.to_owned(), r#type);
    }
    Some(map)
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

    let parameters = unwrap_or_return!(
        unsafe { parse_descriptors(parameters) },
        false,
    );
    let returns = unwrap_or_return!(
        unsafe { parse_descriptors(returns) },
        false,
    );

    dbg!(&parameters);
    dbg!(&returns);

    let function = Function {
        parameters,
        returns,
        fn_ptr: callback,
    };

    handle.functions.insert(name.to_owned(), function);
    true
}

