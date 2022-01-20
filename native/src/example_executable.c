#include <stdio.h>
#include "client.h"

// Signature: (String name) -> ()
// so parameters[0] points to a const char *name
// so parameters[1] is NULL
// so returns[0] is NULL
void print_callback(const void **parameters, void **returns) {
    (void)returns;
    printf("Hello from callback, %s!\n", *((const char**)parameters[0]));
}
const char *print_parameters[][2] = {
    {"name", "string"},
    {NULL, NULL},
};
const char *print_returns[][2] = {
    {NULL, NULL},
};

// Signature: (int x, int y) -> (int product)
// so parameters[0] points to a const int x
// so parameters[1] points to a const int y
// so parameters[2] is NULL
// so returns[0] points to an int product
// so returns[1] is NULL
void multiply_callback(const void **parameters, void **returns) {
    int x = *(const int*)parameters[0];
    int y = *(const int*)parameters[1];
    int *product = (int*)returns[0];
    *product = x * y;
}
const char *multiply_parameters[3][2] = {
    {"x", "int"},
    {"y", "int"},
    {NULL, NULL},
};
const char *multiply_returns[3][2] = {
    {"product", "int"},
    {NULL, NULL},
};

int main() {
    ClientHandle handle = InitializeLibrary("./ClientLibrary.jar");
    printf("%p\n", handle);
    SetName(handle, "Example");
    RegisterFunction(handle, "print", print_parameters, print_returns, print_callback);
    RegisterFunction(handle, "multiply", multiply_parameters, multiply_returns, multiply_callback);
    LibraryUpdate(handle);
    ShutdownLibrary(handle);
}
