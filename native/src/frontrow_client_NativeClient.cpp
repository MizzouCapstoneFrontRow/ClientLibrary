#include "frontrow_client_NativeClient.h"
#include <cstdio>

extern "C" jint JNI_OnLoad_NativeClient(JavaVM *vm, void *reserved) {
    (void)vm;
    (void)reserved;
    std::printf("AAAAAAAAA\n");
    return JNI_VERSION_1_8;
}

extern "C" void Java_frontrow_client_NativeClient_native_1method(JNIEnv *env, jclass clazz) {
    (void)env;
    (void)clazz;
    std::printf("Printing from a native method!\n");
}
