#![deny(unsafe_op_in_unsafe_fn)]
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
    objects::*,
    errors::Result as JResult,
    sys::jobject
};

pub struct ClientHandle {
    jvm: Arc<JavaVM>,
    executor: Executor,
    client: GlobalRef,
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
        env.call_method(
            client,
            "InitializeLibrary",
            "()V",
            &[],
        )?;
        env.new_global_ref(client)
    });
    let client = match client {
        Ok(c) => c,
        Err(_) => {
            eprintln!("TODO: Gracefully shutdown jvm if initialization fails");
            return None
        },
    };

    match executor.with_attached(|env| {
        let class_bytes: &'static [u8] = include_bytes!(env!("NativeCallback_CLASS"));

        let class_loader = env.find_class("java/lang/ClassLoader")?;
        let system_loader = env.call_static_method(
            class_loader,
            "getSystemClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )?;

        env.define_class(
            "frontrow/client/NativeCallback",
            <JObject as TryFrom<JValue>>::try_from(system_loader).unwrap(),
            class_bytes,
        )?;
        Ok(())
    }) {
        Ok(()) => {},
        Err(_) => {
            eprintln!("TODO: Gracefully shutdown jvm if initialization fails");
            return None;
        },
    };

    println!("InitializeLibrary {:?}", jar_path);
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
        env.call_method(&handle.client, "SetName", "(Ljava/lang/String;)V", &[name.into()])?;
        env.exception_describe()?;
        Ok(())
    }).is_ok()
}

#[no_mangle]
pub extern "C" fn LibraryUpdate(handle: Option<&mut ClientHandle>) -> bool {
    shadow_or_return!(handle, false);
    handle.executor.with_attached(|env| {
        env.call_method(&handle.client, "LibraryUpdate", "()V", &[])?;
        env.exception_describe()?;
        Ok(())
    }).is_ok()
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
    let name: &str = unwrap_or_return!(
        unsafe { CStr::from_ptr(unwrap_or_return!(name, false).as_ptr()) }.to_str(),
        false,
    );

    unsafe fn descriptors_to_slice(descriptors: *const [*const c_char; 2]) -> &'static [[*const c_char; 2]] {
        if descriptors.is_null() {
            &[]
        } else {
            let mut count = 0;
            loop {
                let ptr = unsafe { descriptors.add(count) };
                let descriptor = unsafe { &*ptr };
                if descriptor.iter().copied().any(<*const c_char>::is_null) {
                    break unsafe {
                        std::slice::from_raw_parts(descriptors, count)
                    };
                }
                count += 1;
            }
        }
    }

    let parameters = unsafe { descriptors_to_slice(parameters) };
    let returns = unsafe { descriptors_to_slice(returns) };

    fn new_parameter(env: JNIEnv<'_>, descriptor: [*const c_char; 2]) -> JResult<JObject<'_>> {
        let name: &str = unwrap_or_return!(
            unsafe { CStr::from_ptr(descriptor[0]) }.to_str(),
            Err(jni::errors::Error::NullPtr("parameter name field")),
        );
        let r#type: &str = unwrap_or_return!(
            unsafe { CStr::from_ptr(descriptor[1]) }.to_str(),
            Err(jni::errors::Error::NullPtr("parameter type field")),
        );
        let name = env.new_string(name)?;
        let r#type = env.new_string(r#type)?;

        env.new_object("frontrow/client/Parameter", "(Ljava/lang/String;Ljava/lang/String;)V", &[name.into(), r#type.into()])
    }

    let result = handle.executor.with_attached(|env| {
        let new_parameter = |descriptor: &[*const c_char; 2]| new_parameter(*env, *descriptor);
        let parameter_array = env.new_object_array(
            parameters.len() as _,
            "frontrow/client/Parameter",
            JObject::null(),
        )?;
        let return_array = env.new_object_array(
            returns.len() as _,
            "frontrow/client/Parameter",
            JObject::null(),
        )?;
        for (i, parameter) in parameters.iter().map(new_parameter).enumerate() {
            let parameter = parameter?;
            env.set_object_array_element(parameter_array, i as _, parameter)?;
        }
        for (i, r#return) in returns.iter().map(new_parameter).enumerate() {
            let r#return = r#return?;
            env.set_object_array_element(return_array, i as _, r#return)?;
        }

        let (parameters, returns) = (
            env.new_local_ref::<()>(JObject::from(parameter_array as jobject))?,
            env.new_local_ref::<()>(JObject::from(return_array as jobject))?,
        );

        let name = env.new_string(name)?;

        // TODO: NativeCallback
        let native_callback = env.new_object(
            "frontrow/client/NativeCallback",
            "([Lfrontrow/client/Parameter;[Lfrontrow/client/Parameter;J)V",
            &[
                parameters.into(),
                returns.into(),
                (callback as isize as i64).into(),
            ],
        )?;

        env.call_method(
            &handle.client,
            "RegisterFunction",
            "(Ljava/lang/String;[Lfrontrow/client/Parameter;[Lfrontrow/client/Parameter;Lfrontrow/client/Callback;)V",
            &[
                name.into(),
                parameters.into(),
                returns.into(),
                native_callback.into()
            ],
        )?;
        Ok(())
    });

    handle.executor.with_attached(|env| env.exception_describe());
    result.is_ok()
}

