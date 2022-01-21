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
use crate::util::*;

pub struct ClientHandle {
    jvm: Arc<JavaVM>,
    executor: Executor,
    client: GlobalRef,
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
