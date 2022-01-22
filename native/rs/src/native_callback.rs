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
        jlong,
        jobject,
        jobjectArray,
    },
};
use crate::util::*;
use crate::marshall::*;

pub type CallbackFnPtr = unsafe extern "C" fn(*const *const c_void, *const *mut c_void);

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
    let result = env.with_local_frame(32, || {
        let fn_ptr: jlong = env.get_field(this, "function_pointer", "J")?.j()?;
        let fn_ptr: Option<CallbackFnPtr> = unsafe { std::mem::transmute(fn_ptr) };
        shadow_or_return!(fn_ptr, {
            env.throw_new("java/lang/IllegalArgumentException", "function_pointer was null")?;
            Err(jni::errors::Error::JavaException)
        });
        let parameter_descriptors: jobjectArray = env.get_field(this, "parameters", "[Lfrontrow/client/Parameter;")?.l()?.into_inner();
        let return_descriptors: jobjectArray = env.get_field(this, "returns", "[Lfrontrow/client/Parameter;")?.l()?.into_inner();

        let parameter_count = env.get_array_length(parameter_descriptors)?;
        if parameter_count != env.get_array_length(parameters)? {
            env.throw_new("java/lang/IllegalArgumentException", "wrong number of arguments")?;
            return Err(jni::errors::Error::JavaException);
        }
        let return_count = env.get_array_length(return_descriptors)?;

        let mut parameterbuffer: Vec<Box<dyn InputMarshall>> = Vec::with_capacity(parameter_count as usize);
        for i in 0..parameter_count {
            let parameter = env.get_object_array_element(parameters, i)?;
            let descriptor = env.get_object_array_element(parameter_descriptors, i)?;
            let r#type = env.get_field(descriptor, "type", "Ljava/lang/String;")?.l()?;
            let type_jstr = env.get_string(r#type.into())?;
            let marshaller = unwrap_or_return!(
                INPUT_MARSHALLERS.get::<CStr>(&type_jstr),
                {
                    println!("TODO: also do this validation in RegisterFunction: unrecognized parameter type: {:?}", &**type_jstr);
                    env.throw_new("java/lang/IllegalArgumentException", format!("unrecognized parameter type: {:?}", &**type_jstr))?;
                    Err(jni::errors::Error::JavaException)
                }
            );
            parameterbuffer.push(marshaller(env, parameter)?);
//            if &**type_jstr == c_str!("int") {
//                parameterbuffer.push(<i32 as InputMarshall>::from_object(env, parameter)?);
////            } else if &**type_jstr == c_str!("string") {
////                parameterbuffer.push(<StringMarshall as InputMarshall>::from_object(env, parameter)?);
//            } else {
//                println!("TODO: do this validation in RegisterFunction");
//                env.throw_new("java/lang/IllegalArgumentException", format!("unrecognized parameter type: {:?}", &**type_jstr))?;
//                return Err(jni::errors::Error::JavaException);
//            }
        }

        let mut returnbuffer: Vec<Box<dyn OutputMarshall>> = Vec::with_capacity(return_count as usize);
        for i in 0..return_count {
            let descriptor = env.get_object_array_element(return_descriptors, i)?;
            let r#type = env.get_field(descriptor, "type", "Ljava/lang/String;")?.l()?;
            let type_jstr = env.get_string(r#type.into())?;
            let marshaller = unwrap_or_return!(
                OUTPUT_MARSHALLERS.get::<CStr>(&type_jstr),
                {
                    println!("TODO: also do this validation in RegisterFunction: unrecognized parameter type: {:?}", &**type_jstr);
                    env.throw_new("java/lang/IllegalArgumentException", format!("unrecognized return type: {:?}", &**type_jstr))?;
                    Err(jni::errors::Error::JavaException)
                }
            );
            returnbuffer.push(marshaller(env)?);

            if &**type_jstr == c_str!("int") {
//            } else if &**type_jstr == c_str!("string") {
//                returnbuffer.push(<StringMarshall as OutputMarshall>::default_return(env)?);
            } else {
            }
        }

        let parameters: Vec<*const c_void> = parameterbuffer.iter().map(|m| m.data()).collect();
        let returns: Vec<*mut c_void> = returnbuffer.iter_mut().map(|m| m.data()).collect();

        unsafe {
            fn_ptr(parameters.as_ptr(), returns.as_ptr());
        }

        drop(parameters);
        drop(returns);

        // Release all parameters (if one fails, the rest will be `drop`ped, but not `release`d
        parameterbuffer.into_iter().map(|p| p.release(env)).collect::<JResult<_>>()?;

        let returns = env.new_object_array(return_count, "java/lang/Object", JObject::null())?;
        for (i, value) in returnbuffer.into_iter().enumerate() {
            env.set_object_array_element(returns, i as _, value.to_object(env)?)?;
        }

        Ok(returns.into())
    });

    match result {
        Ok(result) => result.into_inner(),
        Err(_) => std::ptr::null_mut(),
    }
}
