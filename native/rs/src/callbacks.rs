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

impl Type {
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        use Type::*;
        use PrimType::*;
        Some(match s {
            "bool" => Prim(Bool),
            "byte" => Prim(Byte),
            "short" => Prim(Short),
            "int" => Prim(Int),
            "long" => Prim(Byte),
            "float" => Prim(Float),
            "double" => Prim(Double),
            "bool[]" => PrimArray(Bool),
            "byte[]" => PrimArray(Byte),
            "short[]" => PrimArray(Short),
            "int[]" => PrimArray(Int),
            "long[]" => PrimArray(Byte),
            "float[]" => PrimArray(Float),
            "double[]" => PrimArray(Double),
            "string" => String,
            "string[]" => StringArray,
            _ => return None,
        })
    }
}

pub(crate) struct Function {
    pub(crate) parameters: IndexMap<String, InputMarshaller>,
    pub(crate) returns: IndexMap<String, OutputMarshaller>,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        parameters: *const *const libc::c_void,
        returns: *const *mut libc::c_void,
    ),
}

pub(crate) struct Axis {
    pub(crate) input_marshaller: InputMarshaller,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        input: *const libc::c_void,
    ),
}

pub(crate) struct Sensor {
    pub(crate) output_marshaller: OutputMarshaller,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        output: *mut libc::c_void,
    ),
}

impl Function {
    pub(crate) fn new(
        parameters: IndexMap<String, Type>,
        returns: IndexMap<String, Type>,
        fn_ptr: unsafe extern "C" fn(
            parameters: *const *const libc::c_void,
            returns: *const *mut libc::c_void,
        ),
    ) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        let parameters = parameters
            .into_iter()
            .map(|(name, r#type)| -> Result<(String, InputMarshaller), Box<dyn std::error::Error + 'static>> {
                Ok((
                    name,
                    *INPUT_MARSHALLERS.get(
                        &r#type
                    ).ok_or(format!("unsupported input type: {:?}", r#type))?,
                ))
            }).collect::<Result<_,_>>()?;
        let returns = returns
            .into_iter()
            .map(|(name, r#type)| -> Result<(String, OutputMarshaller), Box<dyn std::error::Error + 'static>> {
                Ok((
                    name,
                    *OUTPUT_MARSHALLERS.get(
                        &r#type
                    ).ok_or(format!("unsupported output type: {:?}", r#type))?,
                ))
            }).collect::<Result<_,_>>()?;
        Ok(Self { parameters, returns, fn_ptr })
    }
    pub(crate) fn call(&self, parameters: HashMap<String, &RawValue>) -> Result<HashMap<String, Box<RawValue>>, Box<dyn std::error::Error + 'static>> {
        eprintln!("TODO: check for extraneous parameters");

        let parameterbuffer: Vec<Box<dyn InputMarshall>> = 
            self.parameters.iter().enumerate().map(|(i, (name, marshaller))| {
                let value = parameters.get(name).ok_or_else(|| format!("Missing parameter: {}", name))?;
                marshaller(value)
            }).collect::<Result<_,_>>()?;
        let mut returnbuffer: Vec<Box<dyn OutputMarshall>> =
            self.returns.iter().map(|(_, marshaller)| marshaller()).collect();

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
        input_type: Type,
        fn_ptr: unsafe extern "C" fn(
            input: *const libc::c_void,
        ),
    ) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        let input_marshaller = *INPUT_MARSHALLERS.get(&input_type).ok_or(format!("unsupported input type: {:?}", input_type))?;
        Ok(Self { input_marshaller, fn_ptr })
    }
    pub(crate) fn call(&self, input: &RawValue) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let input: Box<dyn InputMarshall> = (self.input_marshaller)(input)?;
        unsafe {
            (self.fn_ptr)(input.data());
        }
        Ok(())
    }
}

impl Sensor {
    pub(crate) fn new(
        output_type: Type,
        fn_ptr: unsafe extern "C" fn(
            input: *mut libc::c_void,
        ),
    ) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        let output_marshaller = *OUTPUT_MARSHALLERS.get(&output_type).ok_or(format!("unsupported output type: {:?}", output_type))?;
        Ok(Self { output_marshaller, fn_ptr })
    }
    pub(crate) fn call(&self) -> Result<Box<RawValue>, Box<dyn std::error::Error + 'static>> {
        let mut output: Box<dyn OutputMarshall> = (self.output_marshaller)();
        unsafe {
            (self.fn_ptr)(output.data());
        }
        Ok(output.to_json()?)
    }
}
