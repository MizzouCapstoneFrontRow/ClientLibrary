#ifndef CLIENT_H
#define CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif // def __cplusplus

#include <stdbool.h>
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
bool SetName(ClientHandle, const char *name);

/**
* Set the reset function. This will be called when the server wants to reset the client
* to a known safe state (e.g. if the unity client disconnects).
* If this function pointer is NULL, or if the client never sets the function, then
* the client library will do nothing upon recieving a reset message, except posibly print a
* diagnostic message.
* On success: returns true
* On failure: returns false
*/
bool SetReset(ClientHandle, void (*reset)(void));

/**
* Registers a function.
* TODO: document how callback works
* @param handle     The client handle
* @param parameters Parameter descriptors for input parameters
* @param returns    Parameter descriptors for output parameters
* @param callback   The callback function to call when the server calls the function
* @returns bool success (Was the function registered successfully)
* Parameter descriptor: A parameter descriptor is an array of two const char*,
* the name and type, respectively, of the parameter.
* The arrays of parameter descritptors passed into this function should be
* terminated by {NULL, NULL}.
*/
bool RegisterFunction(
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
* @returns bool success (Was the sensor registered successfully)
* Type descriptor: A type descriptor is const char*, the type of the parameter.
*/
bool RegisterSensor(
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
* @returns bool success (Was the axis registered successfully)
* The callback must take a single `double` parameter
*/
bool RegisterAxis(
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
* @param address    The IPv4 address of the stream
* @param port       The TCP port
* @returns bool success (Was the axis registered successfully)
* Type descriptor: A type descriptor is const char*, the type of the parameter.
*/
bool RegisterStream(
    ClientHandle handle,
    const char *name,
    const char *format,
    const char *address,
    unsigned short port
);

/**
* Connects to a server
* @param server     String that is the domain name or IP address (v4 or v6) of the server.
* @param port       uint16_t that is the port to connect to on the server.
* @returns bool success (Did the client connect successfully)
*/
bool ConnectToServer(
    ClientHandle handle,
    const char *server,
    uint16_t port
);

/**
* Updates internal library state and calls any necessary callbacks.
*/
bool LibraryUpdate(ClientHandle);

/**
* Deinitialize and shut down the library.
*/
void ShutdownLibrary(ClientHandle);

#ifdef __cplusplus
} // extern "C"
#endif // def __cplusplus

#endif // ndef CLIENT_H
