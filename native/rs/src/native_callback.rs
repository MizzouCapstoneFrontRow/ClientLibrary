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
    sys::{
        jint,
        jobject,
        jobjectArray,
    },
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
pub extern "C" fn JNI_OnLoad_NativeCallback(
    _vm: *mut jni::sys::JavaVM,
    _reserved: *mut c_void,
) -> jint {
    jni::sys::JNI_VERSION_1_8
}

#[no_mangle]
pub extern "C" fn Java_frontrow_client_NativeCallback_call(
    env: JNIEnv<'_>,
    this: JObject<'_>,
    parameters: jobjectArray,
) -> jobjectArray {
    println!("TODO: NativeCallback.call");
    std::ptr::null_mut()
}
