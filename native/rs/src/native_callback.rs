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

pub type CallbackFnPtr = extern "C" fn(*const *const c_void, *const *mut c_void);

#[no_mangle]
pub extern "C" fn JNI_OnLoad_NativeCallback(
    _vm: *mut jni::sys::JavaVM,
    _reserved: *mut c_void,
) -> jint {
    jni::sys::JNI_VERSION_1_8
}

trait InputMarshall<'a> {
    fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> where Self: Sized;
    fn data(&self) -> *const c_void;
    fn release(self: Box<Self>, env: JNIEnv<'a>) -> JResult<()>;
}

trait OutputMarshall<'a> {
    fn default_return(env: JNIEnv<'a>) -> JResult<Box<Self>> where Self: Sized;
    fn data(&mut self) -> *mut c_void;
    fn to_object(self: Box<Self>, env: JNIEnv<'a>) -> JResult<JObject<'a>>;
}

macro_rules! marshall_primitive {
    ( $t:ty, $from_method:literal, $from_sig:literal, $jvalue_unwrap:ident, $to_type:literal, $to_type_sig:literal, $default:literal ) => {
        impl<'a> InputMarshall<'a> for $t {
            fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> {
                Ok(Box::new(
                    env.call_method(object, $from_method, $from_sig, &[])?. $jvalue_unwrap ()?
                ))
            }
            fn data(&self) -> *const c_void { self as *const Self as *const c_void }
            fn release(self: Box<Self>, _env: JNIEnv<'a>) -> JResult<()> { Ok(()) }
        }
        impl<'a> OutputMarshall<'a> for $t {
            fn default_return(env: JNIEnv<'a>) -> JResult<Box<Self>> {
                Ok(Box::new( $default ))
            }
            fn data(&mut self) -> *mut c_void { self as *mut Self as *mut c_void }
            fn to_object(self: Box<Self>, env: JNIEnv<'a>, ) -> JResult<JObject<'a>> {
                env.new_object($to_type, $to_type_sig, &[(*self).into()])
            }
        }
    };
}

marshall_primitive!(bool, "boolValue", "()Z", z, "java/lang/Boolean", "(Z)V", false);
marshall_primitive!(i8, "byteValue", "()B", b, "java/lang/Byte", "(B)V", 0);
marshall_primitive!(i16, "shortValue", "()S", s, "java/lang/Short", "(S)V", 0);
marshall_primitive!(i32, "intValue", "()I", i, "java/lang/Integer", "(I)V", 0);
marshall_primitive!(i64, "longValue", "()J", j, "java/lang/Long", "(J)V", 0);

marshall_primitive!(f32, "floatValue", "()F", f, "java/lang/Float", "(F)V", 0.0);
marshall_primitive!(f64, "doubleValue", "()D", d, "java/lang/Double", "(D)V", 0.0);

struct StringMarshall<'a> {
    data: *const c_char,
    string_object: Option<JString<'a>>,
}

impl<'a> InputMarshall<'a> for StringMarshall<'a> {
    fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> {
        let string_object: JString<'a> = env.call_method(object, "toString", "()Ljava/lang/String;", &[])?.l()?.into();
        let data = env.get_string_utf_chars(string_object)?;
        Ok(Box::new(StringMarshall{
            data,
            string_object: Some(string_object),
        }))
    }
    fn data(&self) -> *const c_void {
        &self.data as *const *const c_char as *const c_void
    }
    fn release(self: Box<Self>, env: JNIEnv<'a>) -> JResult<()>{
        let StringMarshall {
            data,
            string_object,
        } = *self;
        let string_object = string_object.unwrap();
        env.release_string_utf_chars(string_object, data)?;
        env.delete_local_ref(string_object.into())?;
        Ok(())
    }
}

impl<'a> OutputMarshall<'a> for StringMarshall<'a> {
    fn default_return(env: JNIEnv<'a>) -> JResult<Box<Self>> {
        Ok(Box::new(StringMarshall {
            data: std::ptr::null(),
            string_object: None,
        }))
    }
    fn data(&mut self) -> *mut c_void {
        &mut self.data as *mut *const c_char as *mut c_void
    }
    fn to_object(self: Box<Self>, env: JNIEnv<'a>, ) -> JResult<JObject<'a>> {
        dbg!("TODO: fix memory leaks");
        if self.data.is_null() {
            env.throw_new("java/lang/NullPointerException", "returned marshalled string was null")?;
            return Err(jni::errors::Error::NullPtr("returned marshalled string was null"));
        }
        let string = unsafe { CStr::from_ptr(self.data) }.to_str().expect("String was not UTF8");
        env.new_string(string).map(Into::into)
    }
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
            if &**type_jstr == c_str!("int") {
                parameterbuffer.push(<i32 as InputMarshall>::from_object(env, parameter)?);
            } else if &**type_jstr == c_str!("string") {
                parameterbuffer.push(<StringMarshall as InputMarshall>::from_object(env, parameter)?);
            } else {
                println!("TODO: do this validation in RegisterFunction");
                env.throw_new("java/lang/IllegalArgumentException", format!("unrecognized parameter type: {:?}", &**type_jstr))?;
                return Err(jni::errors::Error::JavaException);
            }
        }

        let mut returnbuffer: Vec<Box<dyn OutputMarshall>> = Vec::with_capacity(return_count as usize);
        for i in 0..return_count {
            let descriptor = env.get_object_array_element(return_descriptors, i)?;
            let r#type = env.get_field(descriptor, "type", "Ljava/lang/String;")?.l()?;
            let type_jstr = env.get_string(r#type.into())?;
            if &**type_jstr == c_str!("int") {
                returnbuffer.push(<i32 as OutputMarshall>::default_return(env)?);
            } else if &**type_jstr == c_str!("string") {
                returnbuffer.push(<StringMarshall as OutputMarshall>::default_return(env)?);
            } else {
                println!("TODO: do this validation in RegisterFunction");
                env.throw_new("java/lang/IllegalArgumentException", format!("unrecognized return type: {:?}", &**type_jstr))?;
                return Err(jni::errors::Error::JavaException);
            }
        }

        let parameters: Vec<*const c_void> = parameterbuffer.iter().map(|m| m.data()).collect();
        let returns: Vec<*mut c_void> = returnbuffer.iter_mut().map(|m| m.data()).collect();

        unsafe {
            fn_ptr(parameters.as_ptr(), returns.as_ptr());
        }

        drop(parameters);
        drop(returns);

        parameterbuffer.into_iter().map(|p| p.release(env)).collect::<Vec<_>>().into_iter().collect::<JResult<_>>()?;

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
