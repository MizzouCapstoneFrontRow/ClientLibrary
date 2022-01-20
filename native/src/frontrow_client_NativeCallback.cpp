#include "frontrow_client_NativeCallback.h"
#include <cstdio>

extern "C" jint JNI_OnLoad_NativeCallback(JavaVM *vm, void *reserved) {
    (void)vm;
    (void)reserved;
    return JNI_VERSION_1_8;
}

extern "C" jobjectArray Java_frontrow_client_NativeCallback_call(JNIEnv *env, jobject self, jobjectArray parameters) {
    (void)env;
    (void)self;
    (void)parameters;
    std::printf("Printing from a native method (NativeCallback.call)!\n");
    return nullptr;
}
