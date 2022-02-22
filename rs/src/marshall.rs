use std::ffi::{CString, CStr};
use std::error::Error;
use std::marker::PhantomData;
use std::collections::HashMap;
use libc::{c_void, c_char};
use serde_json::value::RawValue;
use crate::util::*;


pub(crate) trait InputMarshall {
    fn from_json(json: &RawValue) -> Result<Box<Self>, Box<dyn Error + Send + Sync + 'static>> where Self: Sized;
    fn data(&self) -> *const c_void;
}

pub(crate) trait OutputMarshall {
    fn empty() -> Box<Self> where Self: Sized;
    fn data(&mut self) -> *mut c_void;
    fn to_json(&self) -> Result<Box<RawValue>, Box<dyn Error + Send + Sync + 'static>>;
}

macro_rules! impl_primitive_marshall {
    ($ty:ty, $empty:expr) => {
        impl_primitive_marshall!(@input $ty);
        impl_primitive_marshall!(@output $ty, $empty);
    };
    (@input $ty:ty) => {
        impl InputMarshall for $ty {
            fn from_json(json: &RawValue) -> Result<Box<Self>, Box<dyn Error + Send + Sync + 'static>> {
                Ok(Box::new(serde_json::from_str::<$ty>(json.get())?))
            }
            fn data(&self) -> *const c_void {
                self as *const Self as *const c_void
            }
        }
    };
    (@output $ty:ty, $empty:expr) => {
        impl OutputMarshall for $ty {
            fn empty() -> Box<Self> {
                Box::new($empty)
            }
            fn data(&mut self) -> *mut c_void {
                self as *mut Self as *mut c_void
            }
            fn to_json(&self) -> Result<Box<RawValue>, Box<dyn Error + Send + Sync + 'static>> {
                Ok(RawValue::from_string(serde_json::to_string(&*self)?)?)
            }
        }
    };
}


#[repr(transparent)]
struct OutputBoolMarshall(u8); // To ensure we don't have invalid bools floating around

impl serde::ser::Serialize for OutputBoolMarshall {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            0 => false.serialize(serializer),
            _ => false.serialize(serializer),
        }
    }
}

impl_primitive_marshall!(@input bool);
impl_primitive_marshall!(@output OutputBoolMarshall, OutputBoolMarshall(0));
impl_primitive_marshall!(i8, 0);
impl_primitive_marshall!(i16, 0);
impl_primitive_marshall!(i32, 0);
impl_primitive_marshall!(i64, 0);
impl_primitive_marshall!(f32, 0.0);
impl_primitive_marshall!(f64, 0.0);

struct InputStringMarshall {
    ptr: *const c_char, // points into data
    data: CString,
}
    
impl InputMarshall for InputStringMarshall {
    fn from_json(json: &RawValue) -> Result<Box<Self>, Box<dyn Error + Send + Sync + 'static>> {
        eprintln!("TODO: check for memory leaks with strings");
        let data = serde_json::from_str::<CString>(json.get())?;
        let ptr = data.as_ptr();
        Ok(Box::new(Self { ptr, data }))
    }
    fn data(&self) -> *const c_void {
        &self.ptr as *const *const c_char as *const c_void
    }
}

#[repr(C)]
struct InputArrayMarshallInner {
    length: i32,
    data: *const c_void,
}

struct InputArrayMarshall<T> {
    inner: InputArrayMarshallInner,
    data: Vec<T>,
}

impl<T: for<'a> serde::de::Deserialize<'a>> InputMarshall for InputArrayMarshall<T> {
    fn from_json(json: &RawValue) -> Result<Box<Self>, Box<dyn Error + Send + Sync + 'static>> {
        let data = serde_json::from_str::<Vec<T>>(json.get())?;
        let length: i32 = data.len().try_into().or(Err("input array was too long"))?;
        let inner = InputArrayMarshallInner { length, data: data.as_ptr() as *const c_void };
        Ok(Box::new(Self { inner, data }))
    }
    fn data(&self) -> *const c_void {
        &self.inner as *const InputArrayMarshallInner as *const c_void
    }
}

#[repr(C)]
struct InputStringArrayMarshallInner {
    length: i32,
    data: *const *const c_char,
}

struct InputStringArrayMarshall {
    inner: InputStringArrayMarshallInner,
    ptrs: Vec<*const c_char>,
    data: Vec<CString>,
}

impl InputMarshall for InputStringArrayMarshall {
    fn from_json(json: &RawValue) -> Result<Box<Self>, Box<dyn Error + Send + Sync + 'static>> {
        eprintln!("TODO: check for memory leaks with strings");
        let data = serde_json::from_str::<Vec<CString>>(json.get())?;
        let length: i32 = data.len().try_into().or(Err("input array was too long"))?;
        let ptrs: Vec<*const c_char> = data.iter().map(|cs| cs.as_ptr()).collect();

        let inner = InputStringArrayMarshallInner { length, data: ptrs.as_ptr() };
        Ok(Box::new(Self { inner, ptrs, data }))
    }
    fn data(&self) -> *const c_void {
        &self.inner as *const InputStringArrayMarshallInner as *const c_void
    }
}



#[repr(C)]
struct OutputStringMarshall {
    data: *const c_char,
    release: Option<unsafe extern "C" fn(*const c_char)>,
}

impl std::ops::Drop for OutputStringMarshall {
    fn drop(&mut self) {
        if let Some(release) = self.release {
            unsafe { release(self.data) }
        }
    }
}

impl OutputMarshall for OutputStringMarshall {
    fn empty() -> Box<Self> {
        eprintln!("TODO: check for memory leaks with strings");
        Box::new(Self {
            data: std::ptr::null(),
            release: None,
        })
    }
    fn data(&mut self) -> *mut c_void {
        self as *mut Self as *mut c_void
    }
    fn to_json(&self) -> Result<Box<RawValue>, Box<dyn Error + Send + Sync + 'static>> {
        if self.data.is_null() {
            Err("output string was null pointer")?;
        }
        let cstr = unsafe { CStr::from_ptr(self.data) };
        Ok(RawValue::from_string(serde_json::to_string(cstr)?)?)
    }
}

#[repr(C)]
struct OutputArrayMarshall<T: Sized> {
    length: i32,
    data: *mut T,
    release: Option<unsafe extern "C" fn(i32, *mut T)>,
    phantom: PhantomData<T>,
}

impl<T: Sized> std::ops::Drop for OutputArrayMarshall<T> {
    fn drop(&mut self) {
        if let Some(release) = self.release {
            unsafe { release(self.length, self.data) }
        }
    }
}

impl<T: serde::ser::Serialize> OutputMarshall for OutputArrayMarshall<T> {
    fn empty() -> Box<Self> {
        Box::new(Self {
            length: 0,
            data: std::ptr::null_mut(),
            release: None,
            phantom: PhantomData,
        })
    }
    fn data(&mut self) -> *mut c_void {
        self as *mut Self as *mut c_void
    }
    fn to_json(&self) -> Result<Box<RawValue>, Box<dyn Error + Send + Sync + 'static>> {
        let length: usize = match self.length {
            0 => { // empty array, return early
                let slice: &[T] = &[];
                return Ok(RawValue::from_string(serde_json::to_string(slice)?)?)
            },
            1.. => self.length.try_into().or(Err("output array too long"))?,
            _ => Err("output array had invalid (negative) length")?,
        };
        if self.data.is_null() {
            Err("output array was null pointer")?;
        }
        let slice: &[T] = unsafe {
            std::slice::from_raw_parts(self.data, length)
        };
        Ok(RawValue::from_string(serde_json::to_string(slice)?)?)
    }
}

#[repr(C)]
struct OutputStringArrayMarshall {
    length: i32,
    data: *mut OutputStringMarshall,
    release: Option<unsafe extern "C" fn(i32, *mut OutputStringMarshall)>,
}

impl std::ops::Drop for OutputStringArrayMarshall {
//OutputArrayMarshall<T> {
    fn drop(&mut self) {
        let length: usize = match self.length {
            0.. => unwrap_or_return!(self.length.try_into(), eprintln!("output array too long")),
            _ => return eprintln!("output array had invalid (negative) length"),
        };
        if self.data.is_null() {
            return eprintln!("output array was null pointer");
        }
        let slice: &mut [OutputStringMarshall] = unsafe {
            std::slice::from_raw_parts_mut(self.data, length)
        };
        unsafe {
            std::ptr::drop_in_place(slice)
        };
        if let Some(release) = self.release {
            unsafe { release(self.length, self.data) }
        }
    }
}

impl OutputMarshall for OutputStringArrayMarshall {
//OutputArrayMarshall<T> {
    fn empty() -> Box<Self> {
        eprintln!("TODO: check for memory leaks with strings");
        Box::new(Self {
            length: 0,
            data: std::ptr::null_mut(),
            release: None,
//            phantom: PhantomData,
        })
    }
    fn data(&mut self) -> *mut c_void {
        self as *mut Self as *mut c_void
    }
    fn to_json(&self) -> Result<Box<RawValue>, Box<dyn Error + Send + Sync + 'static>> {
        let length: usize = match self.length {
            0 => { // empty array, return early
                let strings: Vec<&CStr> = vec![];
                return Ok(RawValue::from_string(serde_json::to_string(&strings)?)?)
            },
            1.. => self.length.try_into().or(Err("output array too long"))?,
            _ => Err("output array had invalid (negative) length")?,
        };
        if self.data.is_null() {
            Err("output array was null pointer")?;
        }
        let slice: &[OutputStringMarshall] = unsafe {
            std::slice::from_raw_parts(self.data, length)
        };
        let values: Vec<Box<RawValue>> = slice.iter()
            .map(OutputStringMarshall::to_json)
            .collect::<Result<_,_>>()?;
        Ok(RawValue::from_string(serde_json::to_string(&values)?)?)
    }
}

macro_rules! make_input_marshallers {
    ($map:ident, $( $typeval:expr, $ty:ty ),* $(,)?) => {
        $(
            $map.insert($typeval, (|rv: &RawValue| match <$ty>::from_json(rv) {
                Ok(b) => Ok(b), Err(e) => Err(e)
            }));
        )*
    };
}

macro_rules! make_output_marshallers {
    ($map:ident, $( $typeval:expr, $ty:ty ),* $(,)?) => {
        $(
            $map.insert($typeval, (|| <$ty>::empty()));
        )*
    };
}

use crate::callbacks::PrimType;
use crate::callbacks::Type;

pub(crate) type InputMarshaller = fn(&RawValue) -> Result<Box<dyn InputMarshall>, Box<dyn std::error::Error + Send + Sync + 'static>>;
pub(crate) type OutputMarshaller = fn() -> Box<dyn OutputMarshall>;

lazy_static::lazy_static! {
    pub(crate) static ref INPUT_MARSHALLERS: HashMap<Type, InputMarshaller> = {
        let mut map = HashMap::<Type, InputMarshaller>::with_capacity(16);
        use PrimType::*;
        use Type::*;
        make_input_marshallers!(map, 
            Prim(Bool), bool,
            Prim(Byte), i8,
            Prim(Short), i16,
            Prim(Int), i32,
            Prim(Long), i64,
            Prim(Float), f32,
            Prim(Double), f64,
            PrimArray(Bool), InputArrayMarshall<bool>,
            PrimArray(Byte), InputArrayMarshall<i8>,
            PrimArray(Short), InputArrayMarshall<i16>,
            PrimArray(Int), InputArrayMarshall<i32>,
            PrimArray(Long), InputArrayMarshall<i64>,
            PrimArray(Float), InputArrayMarshall<f32>,
            PrimArray(Double), InputArrayMarshall<f64>,
            String, InputStringMarshall,
            StringArray, InputStringArrayMarshall,
        );
        map
    };
    pub(crate) static ref OUTPUT_MARSHALLERS: HashMap<Type, OutputMarshaller> = {
        let mut map = HashMap::<Type, OutputMarshaller>::with_capacity(16);
        use PrimType::*;
        use Type::*;
        make_output_marshallers!(map, 
            Prim(Bool), OutputBoolMarshall,
            Prim(Byte), i8,
            Prim(Short), i16,
            Prim(Int), i32,
            Prim(Long), i64,
            Prim(Float), f32,
            Prim(Double), f64,
            PrimArray(Bool), OutputArrayMarshall<OutputBoolMarshall>,
            PrimArray(Byte), OutputArrayMarshall<i8>,
            PrimArray(Short), OutputArrayMarshall<i16>,
            PrimArray(Int), OutputArrayMarshall<i32>,
            PrimArray(Long), OutputArrayMarshall<i64>,
            PrimArray(Float), OutputArrayMarshall<f32>,
            PrimArray(Double), OutputArrayMarshall<f64>,
            String, OutputStringMarshall,
            StringArray, OutputStringArrayMarshall,
        );
        map
    };
}
