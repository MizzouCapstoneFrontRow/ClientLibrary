#ifndef CLIENT_H
#define CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif // def __cplusplus

#include <stdint.h>

struct ClientHandle_t;
typedef struct ClientHandle_t *ClientHandle;

struct StringInputParameter_t {
    const char *const string;
};
struct StringOutputParameter_t {
    const char *string;
    void (*release)(const char*);
};

struct ArrayInputParameter_t {
    const int length;
    const void *const data;
};
struct ArrayOutputParameter_t {
    int length;
    void *data;
    void (*release)(int, void*);
};

enum ErrorCode {
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
    /// Other error (nonfatal) e.g. server sent a FunctionCall with invalid parameters
    /// or a message that should never be sent to machine (e.g. AxisReturn)
    Other = 16
};

/**
* Initialize the library and return a handle that will be passed to all library functions.
* On success: returns a non-null handle (pointer).
* On failure: returns ((ClientHandle)NULL)
*/
ClientHandle InitializeLibrary(void);

/**
* Set the name of the client.
* On success: returns true
* On failure: returns false
*/
enum ErrorCode SetName(ClientHandle, const char *name);

/**
* Set the reset function. This will be called when the server wants to reset the client
* to a known safe state (e.g. if the unity client disconnects).
* If this function pointer is NULL, or if the client never sets the function, then
* the client library will do nothing upon recieving a reset message, except posibly print a
* diagnostic message.
* On success: returns true
* On failure: returns false
*/
enum ErrorCode SetReset(ClientHandle, void (*reset)(void));

/**
* Registers a function.
* TODO: document how callback works
* @param handle     The client handle
* @param parameters Parameter descriptors for input parameters
* @param returns    Parameter descriptors for output parameters
* @param callback   The callback function to call when the server calls the function
* @returns enum ErrorCode success (Was the function registered successfully)
* Parameter descriptor: A parameter descriptor is an array of two const char*,
* the name and type, respectively, of the parameter.
* The arrays of parameter descritptors passed into this function should be
* terminated by {NULL, NULL}.
*/
enum ErrorCode RegisterFunction(
    ClientHandle handle,
    const char *name,
    const char *(*parameters)[2],
    const char *(*returns)[2],
    void (*callback)(const void *const*const, void *const*const)
);

/**
* Registers a sensor.
* TODO: document how callback works
* @param handle     The client handle
* @param min        The minimum value that this sensor can have (not enforced)
* @param max        The maximum value that this sensor can have (not enforced)
* @param callback   The callback function to call when the server reads the sensor
* @returns enum ErrorCode success (Was the sensor registered successfully)
* Type descriptor: A type descriptor is const char*, the type of the parameter.
*/
enum ErrorCode RegisterSensor(
    ClientHandle handle,
    const char *name,
    double min,
    double max,
    void (*callback)(double *const)
);

/**
* Registers an axis.
* TODO: document how callback works
* @param handle     The client handle
* @param min        The minimum value that this axis can have (not enforced)
* @param max        The maximum value that this axis can have (not enforced)
* @param group      The group that this axis is a member of
* @param direction  The direction that this axis is in
* @param callback   The callback function to call when the server moves the axis
* @returns enum ErrorCode success (Was the axis registered successfully)
* The callback must take a single `double` parameter
*/
enum ErrorCode RegisterAxis(
    ClientHandle handle,
    const char *name,
    double min,
    double max,
    const char *group,
    const char *direction,
    void (*callback)(const double)
);

/**
* Registers a stream.
* @param handle     The client handle
* @param name       The name of the stream
* @param format     The format of the stream (only mjpeg is currently supported)
* @param fd         The file descriptor to read stream data from.
* @returns enum ErrorCode success (Was the stream registered successfully)
*/
enum ErrorCode RegisterStream(
    ClientHandle handle,
    const char *name,
    const char *format,
    int fd
);

/**
* Connects to a server
* @param server     String that is the domain name or IP address (v4 or v6) of the server.
* @param port       uint16_t that is the port to connect to on the server.
* @returns enum ErrorCode success (Did the client connect successfully)
*/
enum ErrorCode ConnectToServer(
    ClientHandle handle,
    const char *server,
    uint16_t port,
    uint16_t stream_port
);

/**
* Updates internal library state and calls any necessary callbacks.
*/
enum ErrorCode LibraryUpdate(ClientHandle);

/**
* Deinitialize and shut down the library.
*/
void ShutdownLibrary(ClientHandle);

#ifdef __cplusplus
} // extern "C"
#endif // def __cplusplus

#endif // ndef CLIENT_H
