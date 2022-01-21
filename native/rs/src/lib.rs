mod native_callback;

use std::{
    ffi::CStr,
    ptr::NonNull,
    sync::Arc,
};
use libc::{c_char, c_void};
use jni::{
    JavaVM,
    JNIEnv,
    Executor,
    InitArgsBuilder,
    objects,
};

pub struct ClientHandle {
    jvm: Arc<JavaVM>,
    executor: Executor,
    client: objects::GlobalRef,
}

macro_rules! unwrap_or_return {
    ( $value:expr, $retval:expr $(,)? ) => {
        match ($value).map(Some).unwrap_or_default() {
            Some(x) => x,
            None => return $retval,
        }
    };
}
macro_rules! shadow_or_return {
    ( 2 $( $rest:tt )* ) => {
        shadow_or_return!($( $rest )*);
        shadow_or_return!($( $rest )*);
    };
    ( mut $shadow:ident, $retval:expr ) => {
        let mut $shadow = unwrap_or_return!($shadow, $retval);
    };
    ( $shadow:ident, $retval:expr ) => {
        let $shadow = unwrap_or_return!($shadow, $retval);
    };
}

#[no_mangle]
pub extern "C" fn InitializeLibrary(
    jar_path: Option<NonNull<c_char>>
) -> Option<Box<ClientHandle>> {
    let args = match jar_path {
        Some(jar_path) => {
            let option: String = format!(
                "-Djava.class.path={}",
                unsafe { CStr::from_ptr(jar_path.as_ptr()) }.to_str().ok()?,
            );
            InitArgsBuilder::new()
                .option(&option)
                .build()
        },
        None => InitArgsBuilder::new().build(),
    }.ok()?;
    let jvm = JavaVM::new(args).ok()?;
    let jvm = Arc::new(jvm);
    let executor = Executor::new(Arc::clone(&jvm));

    let client = executor.with_attached(|env| {
        let client = env.new_object("frontrow/client/Client", "()V", &[])?;
        env.new_global_ref(client)
    });
    let client = match client {
        Ok(c) => c,
        Err(_) => {
            eprintln!("TODO: Gracefully shutdown jvm if initialization fails");
            return None
        },
    };

    println!("InitializeLibrary {:?}\n", jar_path);
    Some(Box::new(
        ClientHandle {
            jvm,
            executor,
            client,
        }
    ))
}

#[no_mangle]
pub extern "C" fn ShutdownLibrary(handle: Option<Box<ClientHandle>>) {
    // TODO: Is this right? Do we need to detach in ShutdownLibrary before destroying jvm?
    shadow_or_return!(handle, ());
    let ClientHandle {
        jvm,
        executor,
        client,
    } = *handle;
    drop(client);
    drop(executor);
    let jvm = jvm.get_java_vm_pointer();
    if let Some(destroy) = unsafe {&**jvm}.DestroyJavaVM {
        unsafe { (destroy)(jvm); }
    }
}

#[no_mangle]
pub extern "C" fn SetName(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>
) -> bool {
    shadow_or_return!(handle, false);
    shadow_or_return!(name, false);
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(name.as_ptr()) }.to_str(),
        false,
    );
    handle.executor.with_attached(|env| {
        let name = env.new_string(name)?;
        env.call_method(&handle.client, "SetName", "(Ljava/lang/String;)V", &[name.into()]);
        Ok(())
    }).is_ok()
}

#[no_mangle]
pub extern "C" fn LibraryUpdate(handle: Option<&mut ClientHandle>) -> bool {
    shadow_or_return!(handle, false);
    println!("LibraryUpdate {:?}\n", handle as *const _);
    false
}

#[no_mangle]
pub extern "C" fn RegisterFunction(
    handle: Option<&mut ClientHandle>,
    name: Option<NonNull<c_char>>,
    parameters: *const [*const char; 2],
    returns: *const [*const char; 2],
    callback: extern "C" fn (*const *const c_void, *const *mut c_void),
) -> bool {
    shadow_or_return!(handle, false);
    println!("LibraryUpdate {:?}\n", handle as *const _);
    false
}

