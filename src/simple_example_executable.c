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

// Sensor (double count)
void count_sensor(double *const value) {
    static int count = 0;
    ++count;
    *value = count;
}

// Axis (double count)
// so value points to a const double
void example_axis(const double value) {
    printf("Axis got %lf.\n", value);
}


int main() {
    ClientHandle handle = InitializeLibrary();
    printf("handle: %p\n", handle);

    enum ErrorCode result;

    printf("setting name\n");
    result = SetName(handle, "Example");
    printf("result: %d\n", (int)result);

    printf("registering \"example\" axis\n");
    result = RegisterAxis(handle, "example", -1.0, 1.0, "example_group", "z", example_axis);
    printf("result: %d\n", (int)result);

    printf("connecting\n");
    result = ConnectToServer(handle, "192.168.1.3", 45575);
    printf("result: %d\n", (int)result);

    //for (int i = 0; i < 10; ++i) {
    while(true) {
        sleep(1);

        printf("updating\n");
        result = LibraryUpdate(handle);
        printf("result: %d\n", (int)result);

    }

    printf("shutting down\n");
    ShutdownLibrary(handle);
}
