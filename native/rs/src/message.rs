use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::net::TcpStream;
use std::io::{Read, Write};
use serde_json::value::{RawValue, to_raw_value};
use polling::{Poller, Event};

#[derive(Debug)]
pub struct Message {
    pub message_id: i64,
    pub inner: MessageInner,
}

macro_rules! message_inner_enum_with_name {
    (
        $( #[$($meta:tt)*] )?
        $vis:vis enum $name:ident {
            $( $variant:ident $data:tt = $variant_str:literal, )*
        }
    ) => {
        $( #[$($meta)*] )?
        $vis enum $name {
            $( $variant $data , )*
        }
        impl $name {
            fn variant_name(&self) -> &'static str {
                use $name::*;
                match self {
                    $( $variant { .. } => $variant_str ),*
                }
            }
        }
        static MESSAGE_INNER_VARIANT_NAMES: &'static [&'static str] = &[
            $( $variant_str , )*
        ];
    };
}
message_inner_enum_with_name!{
#[derive(Debug)]
pub enum MessageInner {
    MachineDescription {
        name: String,
        functions: HashMap<String, Function>,
        sensors: HashMap<String, Sensor>,
        axes: HashMap<String, Axis>,
        streams: HashMap<String, Stream>,
    } = "machine_description",
    FunctionCall {
        name: String,
        parameters: HashMap<String, Box<RawValue>>,
    } = "function_call",
    FunctionReturn {
        reply_to: i64,
        returns: HashMap<String, Box<RawValue>>,
    } = "function_return",
    SensorRead {
        name: String,
    } = "sensor_read",
    SensorReturn {
        reply_to: i64,
        value: Box<RawValue>,
    } = "sensor_return",
    AxisChange {
        name: String,
        value: Box<RawValue>,
    } = "axis_change",
    AxisReturn {
        reply_to: i64,
    } = "axis_return",
    UnsupportedOperation {
        reply_to: i64,
        operation: String,
        reason: String
    } = "unsupported_operation",
    Other(HashMap<String, Box<RawValue>>) = "other",
}
}

#[derive(Debug)]
pub struct Function {
    parameters: HashMap<String, String>,
    returns: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Sensor {
    r#type: String,
    min: Box<RawValue>,
    max: Box<RawValue>,
}

#[derive(Debug)]
pub struct Axis {
    r#type: String,
    min: Box<RawValue>,
    max: Box<RawValue>,
}

// TODO
#[derive(Debug)]
pub struct Stream {
    todo: Box<RawValue>,
}

lazy_static::lazy_static! {
    static ref POLLER: Poller = Poller::new().unwrap_or_else(|e| panic!("Failed to create poller: {:?}", e));
    static ref KEY: AtomicUsize = AtomicUsize::new(0);
}

pub fn try_read_message(mut stream: &TcpStream) -> Result<Option<Message>, Box<dyn std::error::Error + 'static>> {
    let poller: &Poller = &*POLLER;
    let key = KEY.fetch_add(1, Ordering::Relaxed);
    poller.add(stream, Event::readable(key))?;
    let mut events = Vec::with_capacity(1);
    poller.wait(&mut events, Some(std::time::Duration::from_secs(0)))?;
    poller.delete(stream)?;
    if events.len() > 0 {
        let mut byte_count_buf = [0u8; 4];
        stream.read_exact(&mut byte_count_buf[..])?;
        let byte_count = u32::from_be_bytes(byte_count_buf);
        let mut msg_buf = vec![0u8; byte_count as usize];
        stream.read_exact(&mut msg_buf[..])?;
        let msg_buf = std::str::from_utf8(&msg_buf)?;
        Ok(Some(serde_json::from_str::<Message>(msg_buf)?))
    } else {
        Ok(None)
    }
}

pub fn try_write_message(mut stream: &TcpStream, msg: &Message) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let msg = serde_json::to_vec(msg)?;
    let byte_count_buf = u32::to_be_bytes(msg.len().try_into().or(Err("message too long"))?);
    stream.write_all(&byte_count_buf)?;
    stream.write_all(&msg)?;
    Ok(())
}

enum DeserializeError {
    InvalidType(serde::de::Unexpected<'static>, &'static dyn serde::de::Expected),
    InvalidValue(serde::de::Unexpected<'static>, &'static dyn serde::de::Expected),
    InvalidLength(usize, &'static dyn serde::de::Expected),
    UnknownVariant(&'static str, &'static [&'static str]),
    UnknownField(String, &'static [&'static str]),
    MissingField(&'static str),
    DuplicateField(&'static str),
}

impl DeserializeError {
    fn into_serde<E: serde::de::Error>(self: DeserializeError) -> E {
        use DeserializeError::*;
        match self {
            InvalidType(ue, e) => E::invalid_type(ue, e),
            InvalidValue(ue, e) => E::invalid_value(ue, e),
            InvalidLength(l, e) => E::invalid_length(l, e),
            UnknownVariant(v, e) => E::unknown_variant(v, e),
            UnknownField(f, e) => E::unknown_field(&f, e),
            MissingField(f) => E::missing_field(f),
            DuplicateField(f) => E::duplicate_field(f),
        }
    }
}


lazy_static::lazy_static! {
    static ref MESSAGE_INNER_DESERIALIZERS: HashMap<
        &'static str,
        for<'a> fn(message_id: i64, json: HashMap<&'a str, &'a RawValue>) -> Result<Message, DeserializeError>,
    > = {
        use serde::de::*;
        let mut map = HashMap::<
            &'static str,
            for<'a> fn(message_id: i64, json: HashMap<&'a str, &'a RawValue>) -> Result<Message, DeserializeError>,
        >::new();
        map.insert("function_call", |message_id: i64, mut json: HashMap<&str, &RawValue>| {
            let name = json.remove("name")
                .ok_or_else(|| DeserializeError::MissingField("name"))?;
            let name = serde_json::from_str::<String>(name.get())
                .map_err(|_| DeserializeError::InvalidType(
                    Unexpected::Other("TODO: unknown"),
                    &"a string",
                ))?;
            let parameters = json.remove("parameters")
                .ok_or_else(|| DeserializeError::MissingField("parameters"))?;
            let parameters = serde_json::from_str::<HashMap<String, Box<RawValue>>>(parameters.get())
                .map_err(|_| DeserializeError::InvalidType(
                    Unexpected::Other("TODO: unknown"),
                    &"function parameters",
                ))?;
            if let Some((field, _value)) = json.into_iter().next() {
                Err(DeserializeError::UnknownField(
                    field.to_owned().into(),
                    &["message_id", "message_type", "name", "parameters"],
                ))?;
            }
            Ok(Message {
                message_id,
                inner: MessageInner::FunctionCall {
                    name,
                    parameters,
                },
            })
        });
        map.insert("function_return", |message_id: i64, mut json: HashMap<&str, &RawValue>| {
            let reply_to = json.remove("reply_to")
                .ok_or_else(|| DeserializeError::MissingField("reply_to"))?;
            let reply_to = serde_json::from_str::<i64>(reply_to.get())
                .map_err(|_| DeserializeError::InvalidType(
                    Unexpected::Other("TODO: unknown"),
                    &"a string",
                ))?;
            let returns = json.remove("returns")
                .ok_or_else(|| DeserializeError::MissingField("returns"))?;
            let returns = serde_json::from_str::<HashMap<String, Box<RawValue>>>(returns.get())
                .map_err(|_| DeserializeError::InvalidType(
                    Unexpected::Other("TODO: unknown"),
                    &"function returns",
                ))?;
            if let Some((field, _value)) = json.into_iter().next() {
                Err(DeserializeError::UnknownField(
                    field.to_owned().into(),
                    &["message_id", "message_type", "reply_to", "returns"],
                ))?;
            }
            Ok(Message {
                message_id,
                inner: MessageInner::FunctionReturn {
                    reply_to,
                    returns,
                },
            })
        });
//        todo!();
        map
    };
}

impl<'de> serde::de::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        use serde::de::*;
        let mut json = <HashMap<&'de str, &'de RawValue> as Deserialize<'de>>::deserialize(deserializer)?;
        let message_id = json.remove("message_id")
            .ok_or_else(|| D::Error::missing_field("message_id"))?;
        let message_id = match serde_json::from_str::<i64>(message_id.get()) {
            Ok(id @ 0..) => id,
            _ => Err(D::Error::invalid_type(
                Unexpected::Other("TODO: unknown"),
                &"a positive integer",
            ))?,
        };
        let message_type = json.remove("message_type")
            .ok_or_else(|| D::Error::missing_field("message_type"))?;
        let message_type = serde_json::from_str::<&'de str>(message_type.get())
            .map_err(|_| D::Error::invalid_type(
                Unexpected::Other("TODO: unknown"),
                &"a string",
            ))?;
        let inner_deserializer = MESSAGE_INNER_DESERIALIZERS
            .get(message_type)
            .ok_or_else(|| D::Error::unknown_variant(
                message_type,
                MESSAGE_INNER_VARIANT_NAMES,
            ))?;
        Ok(inner_deserializer(message_id, json).map_err(DeserializeError::into_serde)?)
//        match message_type {
//            _ => Err(D::Error::unknown_variant
        
        
//        todo!()
    }
}

impl serde::ser::Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::ser::Serializer
    {
        use serde::ser::*;
        dbg!(line!());
        let make_map = || -> Result<HashMap::<&'static str, Box<RawValue>>, serde_json::Error> {
            let mut map = HashMap::<&'static str, Box<RawValue>>::with_capacity(8);
            map.insert("message_id", to_raw_value(&self.message_id)?);
            map.insert("message_type", to_raw_value(self.inner.variant_name())?);
            dbg!(line!());

            use MessageInner::*;
            dbg!(line!());
            match &self.inner {
                FunctionCall { name, parameters } => {
            dbg!(line!());
                    map.insert("name", to_raw_value(&name)?);
                    map.insert("parameters", to_raw_value(&parameters)?);
                },
                FunctionReturn { reply_to, returns } => {
            dbg!(line!());
                    map.insert("reply_to", to_raw_value(&reply_to)?);
                    map.insert("returns", to_raw_value(&returns)?);
                },
                _ => todo!(),
            };
            dbg!(line!());
            Ok(map)
        };
        match make_map() {
            Ok(map) => map.serialize(serializer),
            Err(e) => {
                dbg!(&e);
                Err(S::Error::custom(e))
            },
        }
    }
}
