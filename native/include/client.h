#ifndef CLIENT_H
#define CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif // def __cplusplus

#include <stdbool.h>

struct ClientHandle_t;
typedef struct ClientHandle_t *ClientHandle;

/*
 * Initialize the library and return a handle that will be passed to all library functions.
 * On success: returns a non-null handle (pointer).
 * On failure: returns ((ClientHandle)NULL)
 */
ClientHandle InitializeLibrary(void);

/*
 * Set the name of the client.
 * On success: returns true
 * On failure: returns false
 */
bool SetName(ClientHandle, const char *name);

/*
 * Deinitialize and shut down the library.
 */
void ShutdownLibrary(ClientHandle);

#ifdef __cplusplus
} // extern "C"
#endif // def __cplusplus

#endif // ndef CLIENT_H
