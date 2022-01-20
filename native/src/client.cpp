#include "client.h"
#include "cstdio"
#include <jni.h>

struct ClientHandle_t {
    JavaVM *jvm;
    JNIEnv *env;
    jclass *client; // global reference
};

extern const char __NativeClient_start[];
extern const char __NativeClient_end[];

/*
 * Initialize the library and return a handle that will be passed to all library functions.
 * On success: returns a non-null handle (pointer).
 * On failure: returns ((ClientHandle)NULL)
 */
extern "C" ClientHandle InitializeLibrary(void) {
    JavaVM *jvm;
    JNIEnv *env;
    JavaVMInitArgs vm_args; /* JDK/JRE 6 VM initialization arguments */
    JavaVMOption* options = new JavaVMOption[2];
    options[0].optionString = const_cast<char*>("-Djava.class.path=/usr/lib/java");
    options[1].optionString = const_cast<char*>("-verbose:jni,gc,class");
    vm_args.version = JNI_VERSION_1_6;
    vm_args.nOptions = 1;
    vm_args.options = options;
    vm_args.ignoreUnrecognized = false;
    /* load and initialize a Java VM, return a JNI interface
     * pointer in env */
    JNI_CreateJavaVM(&jvm, (void**)&env, &vm_args);
    delete options;

    jclass class_loader = env->FindClass("java/lang/ClassLoader");
    if (class_loader == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find java.lang.ClassLoader.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jmethodID get_system_loader = env->GetStaticMethodID(
        class_loader,
        "getSystemClassLoader",
        "()Ljava/lang/ClassLoader;"
    );
    if (get_system_loader == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find java.lang.ClassLoader's getSystemClassLoader method.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jobject system_loader = env->CallStaticObjectMethod(
        class_loader,
        get_system_loader
    );
    if (system_loader == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "java.lang.ClassLoader.getSystemClassLoader() returned null.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    // Load library class, instantiate it, and store global ref to instance

    jclass clazz = env->DefineClass(
        "frontrow/client/NativeClient",
        system_loader,
        (const jbyte*)__NativeClient_start,
        __NativeClient_end - __NativeClient_start
    );
    if (clazz == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not load frontrow.client.NativeClient.\n");
        auto size = __NativeClient_end - __NativeClient_start;

        std::fprintf(stderr, "%zu %zx\n", size, size);

        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jmethodID test = env->GetStaticMethodID(clazz, "test", "()V");
    if (test == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find frontrow.client.NativeClient's test method.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    env->CallStaticVoidMethod(clazz, test);

    /* invoke the Main.test method using the JNI */
//    jclass cls = env->FindClass("Main");
//    jmethodID mid = env->GetStaticMethodID(cls, "test", "(I)V");
//    env->CallStaticVoidMethod(cls, mid, 100);
    /* We are done. */
//    jvm->DestroyJavaVM();
    return new ClientHandle_t {
        jvm,
        env,
        nullptr,
//        env->NewGlobalRef(clazz),
    };
}



/*
 * Set the name of the client.
 * On success: returns true
 * On failure: returns false
 */
//bool SetName(ClientHandle, const char *name);

/*
 * Deinitialize and shut down the library.
 */
void ShutdownLibrary(ClientHandle handle) {
    if (handle) {
        // TODO: call some sort of finalize method on client
//        handle->env->DeleteGlobalRef(handle->client);
        handle->env->ExceptionDescribe();
        handle->jvm->DestroyJavaVM();
        delete handle;
    }
}
