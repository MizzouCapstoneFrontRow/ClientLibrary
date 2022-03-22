#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include "client.h"
#include "unistd.h"

// Signature: (String name) -> ()
// so parameters[0] points to a const char *name
// so parameters[1] is NULL
// so returns[0] is NULL
void print_callback(const void *const*const parameters, void *const*const returns) {
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
void multiply_callback(const void *const*const parameters, void *const*const returns) {
    int x = *(const int*)parameters[0];
    int y = *(const int*)parameters[1];
    int *product = (int*)returns[0];
    *product = x * y;
}
const char *multiply_parameters[][2] = {
    {"x", "int"},
    {"y", "int"},
    {NULL, NULL},
};
const char *multiply_returns[][2] = {
    {"product", "int"},
    {NULL, NULL},
};

// Signature: (double[] x) -> (double average)
// so parameters[0] points to a struct ArrayInputParameter_t {const int length; const void *data;}, and data points to a const double[length]
// so parameters[2] is NULL
// so returns[0] points to an int product
// so returns[1] is NULL
void average_callback(const void *const*const parameters, void *const*const returns) {
    const struct ArrayInputParameter_t x = *(const struct ArrayInputParameter_t*)parameters[0];
    const double *data = (const double*)x.data;
    double *average = (double*)returns[0];

    double acc = 0.0;
    for (int i = 0; i < x.length; ++i) acc += data[i];

    *average = x.length ? acc / x.length : 0.0;
}
const char *average_parameters[][2] = {
    {"x", "double[]"},
    {NULL, NULL},
};
const char *average_returns[][2] = {
    {"average", "double"},
    {NULL, NULL},
};

// Signature: (int n) -> (int[] seq)
// so parameters[0] points to a const int
// so parameters[1] is NULL
// so returns[0] points to a struct ArrayOutputParameter_t {int length; void *data;, void(*release)(int, void*)}
// so returns[1] is NULL
void sequence_free(int length, void *data) {
    (void)length;
    free(data);
}
void sequence_callback(const void *const*const parameters, void *const*const returns) {
    const int n = *(const int*)parameters[0];
    struct ArrayOutputParameter_t *seq = (struct ArrayOutputParameter_t*)returns[0];

    int *sequence = malloc(n * sizeof(int));
    seq->data = sequence;
    seq->release = sequence_free;
    if (sequence) {
        seq->length = n;
        for (int i = 0; i < n; ++i) {
            sequence[i] = i;
        }
    }
}
const char *sequence_parameters[][2] = {
    {"n", "int"},
    {NULL, NULL},
};
const char *sequence_returns[][2] = {
    {"seq", "int[]"},
    {NULL, NULL},
};

// Signature: (bool[] values) -> (int trues, int falses)
// so parameters[0] points to a struct ArrayInputParameter_t {const int length; const void *data;}, and data points to const bool[length]
// so parameters[1] is NULL
// so returns[0] points to an int
// so returns[1] points to an int
// so returns[2] is NULL
void count_bools_callback(const void *const*const parameters, void *const*const returns) {
    const struct ArrayInputParameter_t values_struct = *(const struct ArrayInputParameter_t*)parameters[0];
    const bool *values = (const bool *)values_struct.data;
    const int values_len = values_struct.length;
    int *trues = (int*)returns[0];
    int *falses = (int*)returns[1];

    for (int i = 0; i < values_len; ++i) {
        if (values[i]) {
            ++*trues;
        } else {
            ++*falses;
        }
    }
}
const char *count_bools_parameters[][2] = {
    {"values", "bool[]"},
    {NULL, NULL},
};
const char *count_bools_returns[][2] = {
    {"trues", "int"},
    {"falses", "int"},
    {NULL, NULL},
};

int main() {
    ClientHandle handle = InitializeLibrary("./ClientLibrary.jar");
    printf("handle: %p\n", handle);

    bool success;

    printf("setting name\n");
    success = SetName(handle, "Example");
    printf("success: %d\n", (int)success);

    printf("registering \"print\" function\n");
    success = RegisterFunction(handle, "print", print_parameters, print_returns, print_callback);
    printf("success: %d\n", (int)success);

    printf("registering \"multiply\" function\n");
    success = RegisterFunction(handle, "multiply", multiply_parameters, multiply_returns, multiply_callback);
    printf("success: %d\n", (int)success);

    printf("registering \"average\" function\n");
    success = RegisterFunction(handle, "average", average_parameters, average_returns, average_callback);
    printf("success: %d\n", (int)success);

    printf("registering \"sequence\" function\n");
    success = RegisterFunction(handle, "sequence", sequence_parameters, sequence_returns, sequence_callback);
    printf("success: %d\n", (int)success);

    printf("registering \"count_bools\" function\n");
    success = RegisterFunction(handle, "count_bools", count_bools_parameters, count_bools_returns, count_bools_callback);
    printf("success: %d\n", (int)success);

    printf("connecting\n");
    success = ConnectToServer(handle, "localhost", 8089);
    printf("success: %d\n", (int)success);

    for (int i = 0; i < 10; ++i) {
        sleep(1);

        printf("updating\n");
        success = LibraryUpdate(handle);
        printf("success: %d\n", (int)success);

    }

    printf("shutting down\n");
    ShutdownLibrary(handle);
}
