// SAFETY: We are only passing this to C, not getting it from C,
// so Rust's enum valididty requirements will not be violated.
#[repr(C)]
pub enum ErrorCode {
    /// Success
    NoError = 0,
    /// An invalid handle was passed (and the error was detected)
    InvalidHandle = 1,
    /// An operation that requires a connected handle was called
    /// with an unconencted handle.
    NotConnected = 2,
    /// An operation that requires an unconnected handle was called
    /// with a conencted handle.
    AlreadyConnected = 3,
    /// A required parameter was null.
    NullParameter = 4,
    /// A string was not UTF8.
    NonUtf8String = 5,
    /// A parameter was invalid.
    InvalidParameter = 6,
    /// An error occurred reading a message
    MessageReadError = 7,
    /// The server sent an invalid message
    InvalidMessageReceived = 8,
    /// An error occurred writing a message
    MessageWriteError = 9,
    /// Tried to register a axis/function/sensor/stream with the same name as an
    /// existing one of the same thing.
    DuplicateName = 10,
    /// The server disconnected.
    ServerDisconnected = 11,
    /// The operation is unsupported (e.g. streams on Windows).
    Unsupported = 12,
    /// The server rejected the connection.
    ConnectionRejected = 13,
    /// Failed to connect because a required value (e.g. name) was not set.
    MissingRequiredValue = 14,
    /// Error connecting to server.
    ConnectionError = 15,
    /// Other error (nonfatal), e.g. server sent a FunctionCall with invalid parameters,
    /// or a message that should never be sent to machine (e.g. AxisReturn)
    OtherError = 16,
}
