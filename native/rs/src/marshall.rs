use std::{
    ffi::CStr,
    ptr::NonNull,
    sync::Arc,
    collections::HashMap,
    marker::PhantomData,
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
        jboolean,
        jint,
        jlong,
        jobject,
        jarray,
        jobjectArray,
    },
};
use crate::util::*;
use crate::native_callback::CallbackFnPtr;

pub(crate) trait InputMarshall<'a> {
    /// Convert a Java object to the input marshalled version
    fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> where Self: Sized;
    /// Get the data pointer that will be passed to the C callback
    fn data(&self) -> *const c_void;
    /// Release any resources associated with this marshall
    /// The marshall type should have a std::ops::Drop impl that ignores errors
    /// and this function should "drop" the marshall *not* ignoring errors
    fn release(self: Box<Self>, env: JNIEnv<'a>) -> JResult<()>;
}

pub(crate) trait OutputMarshall<'a> {
    /// The initial value before the callback is run
    fn default_return(env: JNIEnv<'a>) -> JResult<Box<Self>> where Self: Sized;
    /// Get the data pointer that will be passed to the C callback
    fn data(&mut self) -> *mut c_void;
    /// 
    fn to_object(self: Box<Self>, env: JNIEnv<'a>) -> JResult<JObject<'a>>;
}

pub(crate) type InputMarshaller = for<'a> fn(JNIEnv<'a>, JObject<'a>) -> JResult<Box<dyn InputMarshall<'a> + 'a>>;
pub(crate) type OutputMarshaller = for<'a> fn(JNIEnv<'a>) -> JResult<Box<dyn OutputMarshall<'a> + 'a>>;

macro_rules! dynify_input_marshaller {
    ( $m:ty ) => { {
        fn temp<'a>(env: JNIEnv<'a>, obj: JObject<'a>) -> JResult<Box<dyn InputMarshall<'a> + 'a>> {
            match (<$m as InputMarshall>::from_object)(env, obj) { Ok(b) => Ok(b), Err(e) => Err(e) }
        }
//        (|env, obj| { ($func)(env, obj).map(|b| b) }) as InputMarshaller
        temp as InputMarshaller
    } }
}
macro_rules! dynify_output_marshaller {
    ( $m:ty ) => { {
        fn temp<'a>(env: JNIEnv<'a>) -> JResult<Box<dyn OutputMarshall<'a> + 'a>> {
            match (<$m as OutputMarshall>::default_return)(env) { Ok(b) => Ok(b), Err(e) => Err(e) }
        }
//        (|env| { ($func)(env) }) as OutputMarshaller
        temp as OutputMarshaller
    } }
}

lazy_static::lazy_static! {
    pub(crate) static ref INPUT_MARSHALLERS: HashMap<&'static CStr, InputMarshaller> = HashMap::from([
//        (c_str!("bool"), dynify_input_marshaller!(bool)),
        (c_str!("byte"), dynify_input_marshaller!(i8)),
        (c_str!("short"), dynify_input_marshaller!(i16)),
        (c_str!("int"), dynify_input_marshaller!(i32)),
        (c_str!("long"), dynify_input_marshaller!(i64)),
        (c_str!("float"), dynify_input_marshaller!(f32)),
        (c_str!("double"), dynify_input_marshaller!(f64)),
        (c_str!("bool"), dynify_input_marshaller!(BoolMarshall)),
        (c_str!("string"), dynify_input_marshaller!(InputStringMarshall)),
        (c_str!("byte[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<i8>)),
        (c_str!("short[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<i16>)),
        (c_str!("int[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<i32>)),
        (c_str!("long[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<i64>)),
        (c_str!("float[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<f32>)),
        (c_str!("double[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<f64>)),
        (c_str!("bool[]"), dynify_input_marshaller!(InputPrimitiveArrayMarshall<BoolMarshall>)),
    ]);
    pub(crate) static ref OUTPUT_MARSHALLERS: HashMap<&'static CStr, OutputMarshaller> = HashMap::from([
        (c_str!("byte"), dynify_output_marshaller!(i8)),
        (c_str!("short"), dynify_output_marshaller!(i16)),
        (c_str!("int"), dynify_output_marshaller!(i32)),
        (c_str!("long"), dynify_output_marshaller!(i64)),
        (c_str!("float"), dynify_output_marshaller!(f32)),
        (c_str!("double"), dynify_output_marshaller!(f64)),
        (c_str!("bool"), dynify_output_marshaller!(BoolMarshall)),
        (c_str!("string"), dynify_output_marshaller!(OutputStringMarshall)),
        (c_str!("byte[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<i8>)),
        (c_str!("short[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<i16>)),
        (c_str!("int[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<i32>)),
        (c_str!("long[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<i64>)),
        (c_str!("float[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<f32>)),
        (c_str!("double[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<f64>)),
        (c_str!("bool[]"), dynify_output_marshaller!(OutputPrimitiveArrayMarshall<BoolMarshall>)),
    ]);
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
            fn default_return(_env: JNIEnv<'a>) -> JResult<Box<Self>> {
                Ok(Box::new( $default ))
            }
            fn data(&mut self) -> *mut c_void { self as *mut Self as *mut c_void }
            fn to_object(self: Box<Self>, env: JNIEnv<'a>, ) -> JResult<JObject<'a>> {
                env.new_object($to_type, $to_type_sig, &[(*self).into()])
            }
        }
    };
}

//marshall_primitive!(bool, "boolValue", "()Z", z, "java/lang/Boolean", "(Z)V", false); // 
marshall_primitive!(i8, "byteValue", "()B", b, "java/lang/Byte", "(B)V", 0);
marshall_primitive!(i16, "shortValue", "()S", s, "java/lang/Short", "(S)V", 0);
marshall_primitive!(i32, "intValue", "()I", i, "java/lang/Integer", "(I)V", 0);
marshall_primitive!(i64, "longValue", "()J", j, "java/lang/Long", "(J)V", 0);

marshall_primitive!(f32, "floatValue", "()F", f, "java/lang/Float", "(F)V", 0.0);
marshall_primitive!(f64, "doubleValue", "()D", d, "java/lang/Double", "(D)V", 0.0);

#[derive(Debug, Default)]
struct NoCopy;
struct InputStringMarshall<'a> {
    data: *const c_char,
    env: Option<(JNIEnv<'a>, JString<'a>, NoCopy)>,
}

impl<'a> InputMarshall<'a> for InputStringMarshall<'a> {
    fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> {
        let string_object: JString<'a> = env.call_method(object, "toString", "()Ljava/lang/String;", &[])?.l()?.into();
        let data = env.get_string_utf_chars(string_object)?;
        Ok(Box::new(InputStringMarshall{
            data,
            env: Some((env, string_object, NoCopy)),
        }))
    }
    fn data(&self) -> *const c_void {
        &self.data as *const *const c_char as *const c_void
    }
    fn release(mut self: Box<Self>, _env: JNIEnv<'a>) -> JResult<()>{
        let (env, string_object, _) = self.env.take().unwrap();
        env.release_string_utf_chars(string_object, self.data)?;
        env.delete_local_ref(string_object.into())?;
        Ok(())
    }
}

impl<'a> std::ops::Drop for InputStringMarshall<'a> {
    fn drop(&mut self) {
        if let Some((env, string_object, _)) = self.env.take() {
            env.release_string_utf_chars(string_object, self.data).unwrap_or_else(|_| eprintln!("Failed to release utf chars (memory leak)."));
            env.delete_local_ref(string_object.into()).unwrap_or_else(|_| eprintln!("Failed to release utf chars (memory leak)."));
        }
    }
}

#[repr(C)]
struct OutputStringMarshall {
    data: *const c_char,
    release: Option<unsafe extern "C" fn(*const c_char)>,
}

impl<'a> OutputMarshall<'a> for OutputStringMarshall {
    fn default_return(_env: JNIEnv<'a>) -> JResult<Box<Self>> {
        Ok(Box::new(OutputStringMarshall {
            data: std::ptr::null(),
            release: None,
        }))
    }
    fn data(&mut self) -> *mut c_void {
        self as *mut Self as *mut c_void
    }
    fn to_object(self: Box<Self>, env: JNIEnv<'a>) -> JResult<JObject<'a>> {
        if self.data.is_null() {
            env.throw_new("java/lang/NullPointerException", "returned marshalled string was null")?;
            return Err(jni::errors::Error::NullPtr("returned marshalled string was null"));
        }
        let string = unsafe { CStr::from_ptr(self.data) }.to_str().expect("String was not UTF8");
        env.new_string(string).map(Into::into)
    }
}

impl std::ops::Drop for OutputStringMarshall {
    fn drop(&mut self) {
        if let Some(release) = self.release {
            unsafe { release(self.data) }
        }
    }
}

#[repr(transparent)]
#[derive(Default, Clone, Copy)]
struct BoolMarshall {
    data: u8,
}

impl<'a> InputMarshall<'a> for BoolMarshall {
    fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> {
        let data: jboolean = env.call_method(object, "booleanValue", "()Z", &[])?.z()?.into();
        Ok(Box::new(BoolMarshall{
            data,
        }))
    }
    fn data(&self) -> *const c_void {
        &self.data as *const u8 as *const c_void
    }
    fn release(mut self: Box<Self>, _env: JNIEnv<'a>) -> JResult<()> {
        Ok(())
    }
}

impl<'a> OutputMarshall<'a> for BoolMarshall {
    fn default_return(env: JNIEnv<'a>) -> JResult<Box<Self>> {
        Ok(Box::new(BoolMarshall { data: 0 }))
    }
    fn data(&mut self) -> *mut c_void {
        &mut self.data as *mut u8 as *mut c_void
    }
    fn to_object(self: Box<Self>, env: JNIEnv<'a>) -> JResult<JObject<'a>> {
        env.new_object("java/lang/Boolean", "(Z)V", &[(self.data != 0).into()]) // self.data != 0 to ensure that we don't pass non-{0,1} value into Java
    }
}

trait Primitive: Sized {
    fn ty() -> &'static str;
    fn unwrap_jvalue<'a>(value: JValue<'a>) -> Option<Self>;
    fn get_array<'a>(env: JNIEnv<'a>, array: jarray) -> JResult<Vec<Self>>;
    fn set_array<'a>(env: JNIEnv<'a>, array: jarray, buf: &mut [Self]) -> JResult<()>;
    fn new_array<'a>(env: JNIEnv<'a>, buf: &mut [Self]) -> JResult<jarray>;
}
macro_rules! impl_primitive {
    ($t:ty, $ty:literal, $field:ident, $get_array_region:ident, $set_array_region:ident, $new_array:ident) => {
        impl Primitive for $t {
            fn ty() -> &'static str { $ty }
            fn unwrap_jvalue<'a>(value: JValue<'a>) -> Option<Self> { value.$field().ok() }
            fn get_array<'a>(env: JNIEnv<'a>, array: jarray) -> JResult<Vec<Self>> {
                let length = env.get_array_length(array)?;
                let mut vec = vec![Self::default(); length as usize];
                env.$get_array_region(array, 0, &mut vec)?;
                Ok(vec)
            }
            fn set_array<'a>(env: JNIEnv<'a>, array: jarray, buf: &mut [Self]) -> JResult<()> {
                env.$set_array_region(array, 0, buf)
            }
            fn new_array<'a>(env: JNIEnv<'a>, buf: &mut [Self]) -> JResult<jarray> {
                let array = env.$new_array(buf.len().try_into().expect("invalid length"))?;
                Self::set_array(env, array, buf)?;
                Ok(array)
            }
        }
    };
}
impl_primitive!(i8, "B", b, get_byte_array_region, set_byte_array_region, new_byte_array);
impl_primitive!(i16, "S", s, get_short_array_region, set_short_array_region, new_short_array);
impl_primitive!(i32, "I", i, get_int_array_region, set_int_array_region, new_int_array);
impl_primitive!(i64, "J", j, get_long_array_region, set_long_array_region, new_long_array);
impl_primitive!(f32, "F", f, get_float_array_region, set_float_array_region, new_float_array);
impl_primitive!(f64, "D", d, get_double_array_region, set_double_array_region, new_double_array);

impl Primitive for BoolMarshall {
    fn ty() -> &'static str { "Z" }
    fn unwrap_jvalue<'a>(value: JValue<'a>) -> Option<Self> {
        Some(Self { data: value.z().ok()? as u8 })
    }
    fn get_array<'a>(env: JNIEnv<'a>, array: jarray) -> JResult<Vec<Self>> {
        let length = env.get_array_length(array)?;
        let mut vec = vec![0u8; length as usize];
        env.get_boolean_array_region(array, 0, &mut vec)?;
        let vec = unsafe {
//            let (ptr, len, cap) = vec.into_raw_parts();
            let ptr = vec.as_mut_ptr();
            let len = vec.len();
            let cap = vec.capacity();
            std::mem::forget(vec);
            Vec::<BoolMarshall>::from_raw_parts(ptr as *mut BoolMarshall, len, cap)
        };
        Ok(vec)
    }
    fn set_array<'a>(env: JNIEnv<'a>, array: jarray, buf: &mut [Self]) -> JResult<()> {
        buf.iter_mut().for_each(|b| b.data = (b.data != 0) as u8);
        let buf: &[Self] = buf;
        let buf: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len()) };
        env.set_boolean_array_region(array, 0, buf)
    }
    fn new_array<'a>(env: JNIEnv<'a>, buf: &mut [Self]) -> JResult<jarray> {
        let array = env.new_boolean_array(buf.len().try_into().expect("invalid length"))?;
        Self::set_array(env, array, buf)?;
        Ok(array)
    }
}

#[repr(C)]
/// Called struct ArrayInputParameter_t in C
struct InputPrimitiveArrayMarshallInner {
    length: i32,
    data: *const c_void,
}

struct InputPrimitiveArrayMarshall<T> {
    inner: InputPrimitiveArrayMarshallInner,
    data: Vec<T>,
}

impl<'a, T: Default + Copy + Primitive> InputMarshall<'a> for InputPrimitiveArrayMarshall<T> {
    fn from_object(env: JNIEnv<'a>, object: JObject<'a>) -> JResult<Box<Self>> {
        let array: jarray = *object;
        let data: Vec<T> = <T as Primitive>::get_array(env, array)?;
        Ok(Box::new(InputPrimitiveArrayMarshall{
            inner: InputPrimitiveArrayMarshallInner {
                length: data.len() as i32,
                data: data.as_ptr() as *const c_void,
            },
            data
        }))
    }
    fn data(&self) -> *const c_void {
        &self.inner as *const InputPrimitiveArrayMarshallInner as *const c_void
    }
    fn release(mut self: Box<Self>, _env: JNIEnv<'a>) -> JResult<()>{
        self.data = vec![];
        Ok(())
    }
}


#[repr(C)]
/// Called struct ArrayOutputParameter_t in C
struct OutputPrimitiveArrayMarshall<T> {
    length: i32,
    data: *mut c_void,
    release: Option<unsafe extern "C" fn(i32, *mut c_void)>,
    phantom: PhantomData<T>,
}

impl<'a, T: Default + Copy + Primitive> OutputMarshall<'a> for OutputPrimitiveArrayMarshall<T> {
    fn default_return(env: JNIEnv<'a>) -> JResult<Box<Self>> {
        Ok(Box::new(OutputPrimitiveArrayMarshall{
            length: 0,
            data: std::ptr::null_mut(),
            release: None,
            phantom: PhantomData,
        }))
    }
    fn data(&mut self) -> *mut c_void {
        self as *mut Self as *mut c_void
    }
    fn to_object(self: Box<Self>, env: JNIEnv<'a>, ) -> JResult<JObject<'a>> {
        if self.data.is_null() {
            if self.length <= 0 { // return empty array
                <T as Primitive>::new_array(env, &mut []).map(Into::into)
            } else {
                env.throw_new("java/lang/NullPointerException", "returned marshalled string was null")?;
                Err(jni::errors::Error::NullPtr("returned marshalled string was null"))
            }
        } else {
            let data = unsafe { std::slice::from_raw_parts_mut(self.data as *mut T, self.length.try_into().expect("Invalid length")) };
            let array = <T as Primitive>::new_array(env, data);
            if let Some(release) = self.release {
               unsafe { release(self.length, self.data); }
            }
            array.map(Into::into)
        }
    }
}
