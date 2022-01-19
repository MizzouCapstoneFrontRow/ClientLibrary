#include <stdio.h>
#include "client.h"

int main() {
    ClientHandle handle = InitializeLibrary();
    printf("%p\n", handle);
    ShutdownLibrary(handle);
}
