use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, AtomicI64, Ordering};
use std::net::TcpStream;
use std::io::{Write, BufRead, BufReader};
use std::time::Duration;
use serde_json::value::{RawValue, to_raw_value};
use polling::{Poller, Event};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
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
/// and produces the enum definition, and implements serialization and deserialization for the Message type based on the variants,
/// as well as adding some helper functions.
/// This macro takes in deserialization description for a message type, and adds a deserializer for that message type
/// to the MESSAGE_INNER_DESERIALIZERS map.
/// The message type variant is are first, followed by (in braces) a comma-separated list of field descriptions, where
/// a field description is:
/// * the name of the field in the variant AND the json (must be the same),
/// * the type of the field in the variant
/// * a description of the field (used for error reporting when a field is not found).
/// After the braces are the serialized "message_type" value, whether or not a reply is expected,
/// and (if it exists) the variant field that contains the message_id of the message this message is a reply to
macro_rules! message_inner_enum_with_metadata {
    (
        no_reply: $no_reply:ident,
        expects_reply: $expects_reply:ident,
        reply_to: $reply_to:ident,
        outer: $outer:ident
        $( #[$($meta:tt)*] )?
        $vis:vis enum $name:ident {
            $(
                $(#[$($variant_meta:tt)*])*
                $variant:ident {
                    $( $field:ident : $field_ty:ty : $field_desc:literal ),* $(,)?
                } = $variant_str:literal $variant_expects_reply:ident,
            )* $(,)?
        }
    ) => {
        $( #[$($meta)*] )?
        $vis enum $name {
            $( $(#[$($variant_meta)*])* $variant { $( $field : $field_ty ),* } , )*
        }
        impl $name {
            /// Function to get the message_type of a MessageInner.
            /// Internal use only (for serialization)
            fn variant_name(&self) -> &'static str {
                use $name::*;
                match self {
                    $( $variant { .. } => $variant_str ),*
                }
            }
            /// Does this message expect a reply? I.e. should the server
            /// keep track of its message_id to forward a reply back?
            fn expects_reply(&self) -> bool {
                use $name::*;
                let $no_reply = false;
                let $expects_reply = true;
                match self {
                    $( $variant { .. } => $variant_expects_reply ),*
                }
            }
            /// What message is this message a reply to?
            /// None if this message is not a reply
            pub fn reply_to(&self) -> Option<i64> {
                use $name::*;
                let $reply_to = &None::<i64>;
                match self {
                    $( $variant { $($field),* } => {
                        // If this variant has reply_to, .into() will be i64 -> Option<i64>
                        // else it will use the above local variable, and .into() will be a no-op
                        (*$reply_to).into()
                    } ),*
                }
            }
        }
        impl $outer {
            /// Does this message expect a reply? I.e. should the server
            /// keep track of its message_id to forward a reply back?
            pub fn expects_reply(&self) -> bool {
                self.inner.expects_reply()
            }
            /// What message is this message a reply to?
            /// None if this message is not a reply
            pub fn reply_to(&self) -> Option<i64> {
                self.inner.reply_to()
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

                $(
                    {
                        #[allow(unused_mut)] // (If the message has no fields, then rustc warns that json doesn't need to be mutable)
                        map.insert( $variant_str , |message_id: i64, mut json: HashMap<&str, &RawValue>| {
                            // For each field in this message type
                            $(
                                // Get the field from the json, erroring if it does not exist.
                                let $field = json.remove(stringify!($field))
                                    .ok_or_else(|| DeserializeError::MissingField(stringify!($field)))?;
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
                                    &["message_id", "message_type", $( stringify!($field) ),*],
                                ))?;
                            }
                            // Return a message with the given message id, and this variant message type, with all fields included.
                            Ok(Message {
                                message_id,
                                inner: MessageInner::$variant { $( $field ),* },
                            })
                        });
                    }
                )*
                map
            };
        }


    };
}

message_inner_enum_with_metadata!{
no_reply: no_reply,
expects_reply: expects_reply,
reply_to: reply_to,
outer: Message
#[derive(Debug, Clone)]
pub enum MessageInner {
    /// Machine description. Initial message sent to server.
    /// Contains the name of the client, and the functions, sensors, axes, and streams it supports (by name).
    MachineDescription {
        name: String: "the name of the machine",
        functions: HashMap<String, Function>: "function names and descriptors",
        sensors: HashMap<String, Sensor>: "sensor names and descriptors",
        axes: HashMap<String, Axis>: "axis names and descriptors",
        streams: HashMap<String, Stream>: "stream names and descriptors",
    } = "machine_description" no_reply,
    /// Message from the server representing a request to call a function.
    FunctionCall {
        name: String: "the name of the function",
        parameters: HashMap<String, Box<RawValue>>: "function parameters",
    } = "function_call" expects_reply,
    /// Message to the server representing a reply to a function call with the results.
    FunctionReturn {
        reply_to: i64: "message_id of the message this is a return of",
        returns: HashMap<String, Box<RawValue>>: "function returns",
    } = "function_return" no_reply,
    /// Message from the server representing a request to read a sensor.
    SensorRead {
        name: String: "the name of the sensor",
    } = "sensor_read" expects_reply,
    /// Message to the server representing a reply to a sensor read with the value.
    SensorReturn {
        reply_to: i64: "message_id of the message this is a return of",
        value: Box<RawValue>: "the value of the sensor",
    } = "sensor_return" no_reply,
    /// Message from the server representing a request to change an axis.
    AxisChange {
        name: String: "the name of the axis",
        value: f64: "the value of the axis",
    } = "axis_change" expects_reply,
    /// Message to the server representing a reply to an axis change.
    AxisReturn {
        reply_to: i64: "message_id of the message this is a return of",
    } = "axis_return" no_reply,
    /// Message to/from the server representing that a previous message was unrecognized or unsupported for some reason.
    UnsupportedOperation {
        reply_to: i64: "message_id of the message this is a return of",
        operation: String: "the operation that was unsupported",
        reason: String: "why the operation was unsupported"
    } = "unsupported_operation" no_reply,
    /// Message from the server representing that the client should reset to a safe state
    /// (e.g. because unity has disconnected).
    Reset {} = "reset" no_reply,
    /// Message to/from the server representing that the sender has disconnected.
    Disconnect {} = "disconnect" no_reply,
    /// Message to the server on a stream connection to identify the stream
    StreamDescription {
        machine: String: "the name of the machine",
        stream: String: "the name of the stream",
    } = "stream_descriptor" no_reply,
    /// Message to/from the server representing a keepalive/"heartbeat" request/reply
    Heartbeat {
        is_reply: bool: "is this heartbeat a reply",
    } = "heartbeat" no_reply,
    /// TODO
    Other { data: Box<RawValue>: "data" } = "other" no_reply,
}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub parameters: HashMap<String, String>,
    pub returns: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    #[serde(rename = "type")]
    pub output_type: String,
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axis {
    #[serde(rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub max: f64,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BufferMethod {
    /// Buffer a certain number of frames, then discard
    Frames,
    /// Buffer a certain number of bytes, then discard
    Bytes,
    /// Do not discard any stream contents.
    /// Implies that the stream may only be connected to once.
    NoDiscard,
}

impl Default for BufferMethod {
    fn default() -> Self { Self::NoDiscard }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub format: String,
    #[serde(default)]
    pub buffer_method: BufferMethod,
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

impl<'de> serde::de::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        use serde::de::*;
        // Partially deserialize the message.
        let mut json = <HashMap<&'de str, &'de RawValue> as Deserialize<'de>>::deserialize(deserializer)?;
        // Get the message id, or error.
        let message_id = json.remove("message_id");
        let message_id = message_id.map(|message_id|
            serde_json::from_str::<i64>(message_id.get()).or_else(|_| Err(D::Error::invalid_type(
                Unexpected::Other("TODO: unknown"),
                &"an integer",
            )))
        ).unwrap_or(Ok(-1))?;
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

pub fn try_read_message(stream: &mut BufReader<TcpStream>, timeout: Option<Duration>) -> Result<Option<Message>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let poller: &Poller = &*POLLER;
    let key = KEY.fetch_add(1, Ordering::Relaxed);
    poller.add(stream.get_ref(), Event::readable(key))?;
    let mut events = Vec::with_capacity(1);
    poller.wait(&mut events, timeout)?;
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
