use indexmap::map::IndexMap;

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
    pub(crate) parameters: IndexMap<String, Type>,
    pub(crate) returns: IndexMap<String, Type>,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        parameters: *const *const libc::c_void,
        returns: *const *mut libc::c_void,
    ),
}

pub(crate) struct Axis {
    pub(crate) r#type: Type,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        input: *const libc::c_void,
    ),
}

pub(crate) struct Sensor {
    pub(crate) r#type: Type,
    pub(crate) fn_ptr: unsafe extern "C" fn(
        output: *mut libc::c_void,
    ),
}

