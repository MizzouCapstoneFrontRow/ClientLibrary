use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, AtomicI64, Ordering};
use std::net::TcpStream;
use std::io::{Write, BufRead, BufReader};
use serde_json::value::{RawValue, to_raw_value};
use polling::{Poller, Event};
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct Message {
    pub message_id: i64,
    pub inner: MessageInner,
}

impl Message {
    pub fn new(inner: MessageInner) -> Self {
        static NEXT_MSG_ID: AtomicI64 = AtomicI64::new(1);
        Self {
            message_id: NEXT_MSG_ID.fetch_add(1, Ordering::Relaxed),
            inner,
        }
    }
}

/// This macro takes in the MessageInner enum definition, annotated with the message_type json value for each variant,
/// and produces the enum definition, and implements serialization for the Message type based on the variants.
macro_rules! message_inner_enum_with_name {
    (
        outer: $outer:ident
        $( #[$($meta:tt)*] )?
        $vis:vis enum $name:ident {
            $( $(#[$($variant_meta:tt)*])* $variant:ident { $( $field:ident : $field_ty:ty ),* $(,)? } = $variant_str:literal, )* $(,)?
        }
    ) => {
        $( #[$($meta)*] )?
        $vis enum $name {
            $( $(#[$($variant_meta)*])* $variant { $( $field : $field_ty ),* } , )*
        }
        impl $name {
            /// Function to get the message_type of a MessageInner.
            fn variant_name(&self) -> &'static str {
                use $name::*;
                match self {
                    $( $variant { .. } => $variant_str ),*
                }
            }
        }
        /// Array of all recognized message_type values.
        static MESSAGE_INNER_VARIANT_NAMES: &'static [&'static str] = &[
            $( $variant_str , )*
        ];
        /// Implementing serialization for the Message type.
        impl serde::ser::Serialize for $outer {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::ser::Serializer
            {
                use serde::ser::*;
                let make_map = || -> Result<HashMap::<&'static str, Box<RawValue>>, serde_json::Error> {
                    let mut map = HashMap::<&'static str, Box<RawValue>>::with_capacity(8);
                    // Insert the message_id and message_type before inserting variant-specific fields.
                    map.insert("message_id", to_raw_value(&self.message_id)?);
                    map.insert("message_type", to_raw_value(self.inner.variant_name())?);

                    use $name::*;
                    match &self.inner {
                        $(
                            $variant { $( $field ),* } => {
                                // Insert variant-specific fields into the map
                                $( map.insert(stringify!($field), to_raw_value(&$field)?); )*
                            }
                        ),*
                    };
                    Ok(map)
                };
                match make_map() {
                    Ok(map) => map.serialize(serializer), // Serialize the map if no errors occurred.
                    Err(e) => { // Else, propogate the error.
                        dbg!(&e);
                        Err(S::Error::custom(e))
                    },
                }
            }
        }
    };
}
message_inner_enum_with_name!{
outer: Message
#[derive(Debug)]
pub enum MessageInner {
    /// Machine description. Initial message sent to server.
    /// Contains the name of the client, and the functions, sensors, axes, and streams it supports (by name).
    MachineDescription {
        name: String,
        functions: HashMap<String, Function>,
        sensors: HashMap<String, Sensor>,
        axes: HashMap<String, Axis>,
        streams: HashMap<String, Stream>,
    } = "machine_description",
    /// Message from the server representing a request to call a function.
    FunctionCall {
        name: String,
        parameters: HashMap<String, Box<RawValue>>,
    } = "function_call",
    /// Message to the server representing a reply to a function call with the results.
    FunctionReturn {
        reply_to: i64,
        returns: HashMap<String, Box<RawValue>>,
    } = "function_return",
    /// Message from the server representing a request to read a sensor.
    SensorRead {
        name: String,
    } = "sensor_read",
    /// Message to the server representing a reply to a sensor read with the value.
    SensorReturn {
        reply_to: i64,
        value: Box<RawValue>,
    } = "sensor_return",
    /// Message from the server representing a request to change an axis.
    AxisChange {
        name: String,
        value: f64,
    } = "axis_change",
    /// Message to the server representing a reply to an axis change.
    AxisReturn {
        reply_to: i64,
    } = "axis_return",
    /// Message to/from the server representing that a previous message was unrecognized or unsupported for some reason.
    UnsupportedOperation {
        reply_to: i64,
        operation: String,
        reason: String
    } = "unsupported_operation",
    /// Message to/from the server representing that the sender has disconnected.
    Disconnect {} = "disconnect",
    /// TODO
    Other { data: Box<RawValue> } = "other",
}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub parameters: HashMap<String, String>,
    pub returns: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sensor {
    #[serde(rename = "type")]
    pub output_type: String,
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub max: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Axis {
    #[serde(rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub max: f64,
}

// TODO
#[derive(Debug, Serialize, Deserialize)]
pub struct Stream {
    pub format: String,
    pub address: String,
    pub port: u16,
}

#[allow(dead_code)]
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
    /// This map contains functions that finish deserializing a message, after it's message type and message id have been determined.
    /// The message_type is used as the key in this map, and the message_id is passed in as a parameter.
    static ref MESSAGE_INNER_DESERIALIZERS: HashMap<
        &'static str,
        for<'a> fn(message_id: i64, json: HashMap<&'a str, &'a RawValue>) -> Result<Message, DeserializeError>,
    > = {
        use serde::de::*;
        let mut map = HashMap::<
            &'static str,
            for<'a> fn(message_id: i64, json: HashMap<&'a str, &'a RawValue>) -> Result<Message, DeserializeError>,
        >::new();
        /// This macro takes in deserialization description for a message type, and adds a deserializer for that message type
        /// to the MESSAGE_INNER_DESERIALIZERS map.
        /// The message type variant and serialized "message_type" value are first, followed by (in parentheses)
        /// a comma-separated list of field descriptions.
        /// A field description is:
        /// * the name of the field in the variant,
        /// * the name of the field in the json,
        /// * a description of the field (used for error reporting when a field is not found)
        /// * the type of the field in the variant.
        macro_rules! make_inner_deserializer {
            (
                $variant:ident $variant_str:literal ( $( $field:ident $field_str:literal $field_desc:literal $field_ty:ty),* $(,)?)
            ) => {
                #[allow(unused_mut)] // (If the message has no fields, then rustc warns that json doesn't need to be mutable)
                map.insert( $variant_str , |message_id: i64, mut json: HashMap<&str, &RawValue>| {
                    // For each field in this message type
                    $(
                        // Get the field from the json, erroring if it does not exist.
                        let $field = json.remove($field_str)
                            .ok_or_else(|| DeserializeError::MissingField($field_str))?;
                        // Make a local variable with the name of the field.
                        // Deserialize the field into that variable.
                        let $field = serde_json::from_str::<$field_ty>($field.get())
                            .map_err(|_| DeserializeError::InvalidType(
                                Unexpected::Other("TODO: unknown"),
                                & $field_desc,
                            ))?;
                    )*
                    // After all fields have been read, ensure that no unrecognized fields are left. If so, error.
                    if let Some((field, _value)) = json.into_iter().next() {
                        Err(DeserializeError::UnknownField(
                            field.to_owned().into(),
                            &["message_id", "message_type", $( $field_str ),*],
                        ))?;
                    }
                    // Return a message with the given message id, and this variant message type, with all fields included.
                    Ok(Message {
                        message_id,
                        inner: MessageInner::$variant { $( $field ),* },
                    })
                });
            }
        }
        make_inner_deserializer!(MachineDescription "machine_description" (
            name        "name"      "a string"              String,
            functions   "functions" "function descriptors"  HashMap<String, Function>,
            sensors     "sensors"   "sensor descriptors"    HashMap<String, Sensor>,
            axes        "axes"      "axis descriptors"      HashMap<String, Axis>,
            streams     "streams"   "stream descriptors"    HashMap<String, Stream>,
        ));
        make_inner_deserializer!(FunctionCall "function_call" (
            name        "name"          "a string"              String,
            parameters  "parameters"    "function parameters"   HashMap<String, Box<RawValue>>,
        ));
        make_inner_deserializer!(FunctionReturn "function_return" (
            reply_to "reply_to" "a positive integer" i64,
            returns  "returns"  "function returns"   HashMap<String, Box<RawValue>>,
        ));
        make_inner_deserializer!(SensorRead "sensor_read" (
            name        "name"          "a string"              String,
        ));
        make_inner_deserializer!(SensorReturn "sensor_return" (
            reply_to "reply_to" "a positive integer" i64,
            value    "value"    "sensor value"       Box<RawValue>,
        ));
        make_inner_deserializer!(AxisChange "axis_change" (
            name     "name"     "a string"     String,
            value    "value"    "axis value"   f64,
        ));
        make_inner_deserializer!(AxisReturn "axis_return" (
            reply_to "reply_to" "a positive integer" i64,
        ));
        make_inner_deserializer!(UnsupportedOperation "unsupported_operation" (
            reply_to    "reply_to"  "a positive integer" i64,
            operation   "operation" "a string"           String,
            reason      "reason"    "a string"           String,
        ));
        make_inner_deserializer!(Other "other" (
            data "data" "json data" Box<RawValue>,
        ));
        make_inner_deserializer!(Disconnect "disconnect" (
        ));
        map
    };
}

impl<'de> serde::de::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        use serde::de::*;
        // Partially deserialize the message.
        let mut json = <HashMap<&'de str, &'de RawValue> as Deserialize<'de>>::deserialize(deserializer)?;
        // Get the message id, or error.
        let message_id = json.remove("message_id")
            .ok_or_else(|| D::Error::missing_field("message_id"))?;
        let message_id = match serde_json::from_str::<i64>(message_id.get()) {
            Ok(id @ 0..) => id,
            _ => Err(D::Error::invalid_type(
                Unexpected::Other("TODO: unknown"),
                &"a positive integer",
            ))?,
        };
        // Get the message type, or error.
        let message_type = json.remove("message_type")
            .ok_or_else(|| D::Error::missing_field("message_type"))?;
        let message_type = serde_json::from_str::<&'de str>(message_type.get())
            .map_err(|_| D::Error::invalid_type(
                Unexpected::Other("TODO: unknown"),
                &"a string",
            ))?;
        // Based on the message type, get the function that will complete deserialization, or error.
        let inner_deserializer = MESSAGE_INNER_DESERIALIZERS
            .get(message_type)
            .ok_or_else(|| D::Error::unknown_variant(
                message_type,
                MESSAGE_INNER_VARIANT_NAMES,
            ))?;
        // Complete deserialization.
        Ok(inner_deserializer(message_id, json).map_err(DeserializeError::into_serde)?)
    }
}


lazy_static::lazy_static! {
    static ref POLLER: Poller = Poller::new().unwrap_or_else(|e| panic!("Failed to create poller: {:?}", e));
    static ref KEY: AtomicUsize = AtomicUsize::new(0);
}

pub fn try_read_message(stream: &mut BufReader<TcpStream>) -> Result<Option<Message>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let poller: &Poller = &*POLLER;
    let key = KEY.fetch_add(1, Ordering::Relaxed);
    poller.add(stream.get_ref(), Event::readable(key))?;
    let mut events = Vec::with_capacity(1);
    poller.wait(&mut events, Some(std::time::Duration::from_secs(0)))?;
    poller.delete(stream.get_ref())?;
    if events.len() > 0 {
        let mut msg_buf = String::with_capacity(4096);
        stream.read_line(&mut msg_buf)?;
        Ok(Some(serde_json::from_str::<Message>(&msg_buf)?))
    } else {
        Ok(None)
    }
}

pub fn try_write_message(mut stream: impl Write, msg: &Message) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut data = Vec::with_capacity(4096);
    serde_json::to_writer(&mut data, msg)?;
    data.push(b'\n');
    stream.write_all(&data)?;
    Ok(())
}
