use indexmap::map::IndexMap;
use std::collections::HashMap;
use serde_json::value::RawValue;
use crate::marshall::{
    InputMarshall,
    OutputMarshall,
    InputMarshaller,
    OutputMarshaller,
    INPUT_MARSHALLERS,
    OUTPUT_MARSHALLERS,
};
use crate::RawFd;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PrimType {
    Bool,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Prim(PrimType),
    PrimArray(PrimType),
    String,
    StringArray,
}

macro_rules! to_and_from_str {
    ($ty:ident :
        $( $s:literal => $a:ident $( ( $b:ident ) )? ),* $(,)?
    ) => {
        impl $ty {
            pub(crate) fn from_str(s: &str) -> Option<Self> {
                use $ty::*;
                use PrimType::*;
                Some(match s {
                    $( $s => $a $( ( $b ) )? ),* ,
                    _ => return None,
                })
            }
            pub(crate) fn to_str(&self) -> &'static str {
                use $ty::*;
                use PrimType::*;
                match self {
                    $( $a $( ( $b ) )? => $s ),*
                }
            }
        }
    }
}

//impl Type {
//    pub(crate) fn from_str(s: &str) -> Option<Self> {
//        use Type::*;
//        use PrimType::*;
//        Some(match s {
to_and_from_str!(Type:
    "bool" => Prim(Bool),
    "byte" => Prim(Byte),
    "short" => Prim(Short),
    "int" => Prim(Int),
    "long" => Prim(Long),
    "float" => Prim(Float),
    "double" => Prim(Double),
    "bool[]" => PrimArray(Bool),
    "byte[]" => PrimArray(Byte),
    "short[]" => PrimArray(Short),
    "int[]" => PrimArray(Int),
    "long[]" => PrimArray(Long),
    "float[]" => PrimArray(Float),
    "double[]" => PrimArray(Double),
    "string" => String,
    "string[]" => StringArray,
);
//            _ => return None,
//        })
//    }
//}

pub(crate) struct Function {
    pub(crate) parameters: IndexMap<String, (Type, InputMarshaller)>,
    pub(crate) returns: IndexMap<String, (Type, OutputMarshaller)>,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        parameters: *const *const libc::c_void,
        returns: *const *mut libc::c_void,
    ),
}

#[allow(unused)] // TODO: once axes are implemented, remove this allow
pub(crate) struct Axis {
    pub(crate) input_type: Type,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) group: String,
    pub(crate) direction: String,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        input: f64,
    ),
}

pub(crate) struct Sensor {
    pub(crate) output_type: Type,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        output: *mut f64,
    ),
}

pub(crate) struct Stream {
    pub(crate) format: String,
    pub(crate) fd: RawFd,
}

impl Function {
    pub(crate) fn new(
        parameters: IndexMap<String, Type>,
        returns: IndexMap<String, Type>,
        fn_ptr: unsafe extern "C" fn(
            parameters: *const *const libc::c_void,
            returns: *const *mut libc::c_void,
        ),
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let parameters = parameters
            .into_iter()
            .map(|(name, r#type)| -> Result<(String, (Type, InputMarshaller)), Box<dyn std::error::Error + Send + Sync + 'static>> {
                Ok((
                    name,
                    (
                        r#type,
                        *INPUT_MARSHALLERS.get(
                            &r#type
                        ).ok_or(format!("unsupported input type: {:?}", r#type))?
                    ),
                ))
            }).collect::<Result<_,_>>()?;
        let returns = returns
            .into_iter()
            .map(|(name, r#type)| -> Result<(String, (Type, OutputMarshaller)), Box<dyn std::error::Error + Send + Sync + 'static>> {
                Ok((
                    name,
                    (
                        r#type,
                        *OUTPUT_MARSHALLERS.get(
                            &r#type
                        ).ok_or(format!("unsupported output type: {:?}", r#type))?,
                    ),
                ))
            }).collect::<Result<_,_>>()?;
        Ok(Self { parameters, returns, fn_ptr })
    }
    pub(crate) fn call(&self, parameters: &HashMap<String, Box<RawValue>>) -> Result<HashMap<String, Box<RawValue>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        eprintln!("TODO: check for extraneous parameters");

        let parameterbuffer: Vec<Box<dyn InputMarshall>> =
            self.parameters.iter().map(|(name, (_type, marshaller))| {
                let value = parameters.get(name).ok_or_else(|| format!("Missing parameter: {}", name))?;
                marshaller(value)
            }).collect::<Result<_,_>>()?;
        let mut returnbuffer: Vec<Box<dyn OutputMarshall>> =
            self.returns.iter().map(|(_, (_, marshaller))| marshaller()).collect();

        let parameters: Vec<*const libc::c_void> = parameterbuffer.iter().map(|im| im.data()).collect();
        let returns: Vec<*mut libc::c_void> = returnbuffer.iter_mut().map(|om| om.data()).collect();

        unsafe {
            (self.fn_ptr)(parameters.as_ptr(), returns.as_ptr());
        }

        drop(parameters);
        drop(returns);
        // Don't drop parameterbuffer until we have parsed returns in case data is shared
        // (e.g. a borrowed string or array)

        let result = returnbuffer.iter().zip(self.returns.iter()).map(
            |(om, (name, _))| {
                let value = om.to_json()?;
                Ok((name.to_owned(), value))
            }).collect::<Result<HashMap<String, Box<RawValue>>, _>>();
        drop(parameterbuffer);
        drop(returnbuffer);
        result
    }
}

impl Axis {
    pub(crate) fn new(
        min: f64,
        max: f64,
        group: String,
        direction: String,
        fn_ptr: unsafe extern "C" fn(
            input: f64,
        ),
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let input_type = Type::Prim(PrimType::Double);
        Ok(Self { input_type, min, max, group, direction, fn_ptr })
    }
    pub(crate) fn call(&self, input: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // let input = serde_json::from_str::<f64>(input.get())?;
        unsafe {
            (self.fn_ptr)(input);
        }
        Ok(())
    }
}

impl Sensor {
    pub(crate) fn new(
        min: f64,
        max: f64,
        fn_ptr: unsafe extern "C" fn(
            input: *mut f64,
        ),
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let output_type = Type::Prim(PrimType::Double);
        // let output_marshaller = *OUTPUT_MARSHALLERS.get(&output_type).ok_or(format!("unsupported output type: {:?}", output_type))?;
        Ok(Self { output_type, min, max, fn_ptr })
    }
    pub(crate) fn call(&self) -> Result<Box<RawValue>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut output: f64 = 0.0;
        unsafe {
            (self.fn_ptr)(&mut output);
        }
        Ok(output.to_json()?)
    }
}

impl Stream {
    pub(crate) fn new(
        format: &str,
        fd: RawFd,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(Self {
            format: format.to_owned(),
            fd,
        })
    }
}