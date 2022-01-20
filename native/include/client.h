#ifndef CLIENT_H
#define CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif // def __cplusplus

#include <stdbool.h>

struct ClientHandle_t;
typedef struct ClientHandle_t *ClientHandle;

/**
* Initialize the library and return a handle that will be passed to all library functions.
* On success: returns a non-null handle (pointer).
* On failure: returns ((ClientHandle)NULL)
*/
ClientHandle InitializeLibrary(const char *jar_path);

/**
* Set the name of the client.
* On success: returns true
* On failure: returns false
*/
bool SetName(ClientHandle, const char *name);

/**
* Registers a function.
* TODO: document how callback works
* @param handle     The client handle
* @param parameters Parameter descriptors for input parameters
* @param returns    Parameter descriptors for output parameters
* @param callback   The callback function to call when the server calls the function
* @returns bool success (Was the function registered successfully)
*/
bool RegisterFunction(
    ClientHandle handle,
    const char *name,
    const char *(*parameters)[2],
    const char *(*returns)[2],
    void (*callback)(const void **, void **)
);

/**
* Updates internal library state and calls any necessary callbacks.
*/
void LibraryUpdate(ClientHandle);

/**
* Deinitialize and shut down the library.
*/
void ShutdownLibrary(ClientHandle);

#ifdef __cplusplus
} // extern "C"
#endif // def __cplusplus

#endif // ndef CLIENT_H
