#include "client.h"
#include <string>
#include <cstdio>
#include <jni.h>

struct ClientHandle_t {
    JavaVM *jvm;
    JNIEnv *env;
    jclass client_class; // global reference
    jobject client; // global reference
};

extern const char __NativeClient_start[];
extern const char __NativeClient_end[];

/**
* Initialize the library and return a handle that will be passed to all library functions.
* On success: returns a non-null handle (pointer).
* On failure: returns ((ClientHandle)NULL)
*/
extern "C" ClientHandle InitializeLibrary(const char *jar_path) {
    if (!jar_path) {
        return nullptr;
    }

    JavaVM *jvm;
    JNIEnv *env;
    {
        JavaVMInitArgs vm_args; /* JDK/JRE 6 VM initialization arguments */
        JavaVMOption options[1];
        std::string option_string = "-Djava.class.path=";
        option_string += jar_path;
        // NOTE: requires C++11; std::string wasn't guaranteed to be nul-terminated until C++11
        options[0].optionString = &option_string[0];
        vm_args.version = JNI_VERSION_10;
        vm_args.nOptions = 1;
        vm_args.options = &options[0];
        vm_args.ignoreUnrecognized = false;
        /* load and initialize a Java VM, return a JNI interface
         * pointer in env */
        JNI_CreateJavaVM(&jvm, (void**)&env, &vm_args); // TODO: error checking
    }

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

    jclass native_client_class = env->DefineClass(
        "frontrow/client/NativeClient",
        system_loader,
        (const jbyte*)__NativeClient_start,
        __NativeClient_end - __NativeClient_start
    );
    if (native_client_class == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not load frontrow.client.NativeClient.\n");
        auto size = __NativeClient_end - __NativeClient_start;

        std::fprintf(stderr, "%zu %zx\n", size, size);

        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jmethodID test = env->GetStaticMethodID(native_client_class, "test", "()V");
    if (test == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find frontrow.client.NativeClient's test method.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    env->CallStaticVoidMethod(native_client_class, test);

    jclass client_class = env->FindClass(
        "frontrow/client/Client"
    );
    if (client_class == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find frontrow.client.Client.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jclass client_class_global = (jclass)env->NewGlobalRef(
        (jobject)client_class
    );
    if (client_class_global == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not create global reference.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jmethodID client_constructor = env->GetMethodID(
        client_class,
        "<init>",
        "()V"
    );
    if (client_constructor == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find frontrow.client.Client's default constructor.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jobject client = env->NewObject(
        client_class,
        client_constructor
    );
    if (client == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not construct frontrow.client.Client.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jobject client_global = env->NewGlobalRef(
        client
    );
    if (client_global == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not create global reference.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jmethodID client_initialize_method = env->GetMethodID(
        client_class,
        "InitializeLibrary",
        "()V"
    );
    if (client_initialize_method == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not find frontrow.client.Client's InitializeLibrary method.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    env->CallVoidMethod(client, client_initialize_method);
    if (env->ExceptionOccurred()) {
        // TODO: error handling
        std::fprintf(stderr, "Failed calling frontrow.client.Client's InitializeLibrary method.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    return new ClientHandle_t {
        jvm,
        env,
        client_class_global,
        client_global
    };
}

/**
* Deinitialize and shut down the library.
*/
void ShutdownLibrary(ClientHandle handle) {
    if (!handle) return;

    jmethodID set_name_method = handle->env->GetMethodID(handle->client_class, "ShutdownLibrary", "()V");
    if (!set_name_method) return; // TODO: ExceptionOccurred, etc. Handle java exceptions

    handle->env->CallVoidMethod(handle->client, set_name_method);
    if (handle->env->ExceptionOccurred()) return; // TODO: Handle java exceptions

//    handle->env->ExceptionDescribe();
    handle->jvm->DestroyJavaVM();
    delete handle;
}

/**
* Updates internal library state and calls any necessary callbacks.
*/
void LibraryUpdate(ClientHandle handle) {
    (void)handle;
    // TODO
}

/**
* Set the name of the client.
* On success: returns true
* On failure: returns false
*/
bool SetName(ClientHandle handle, const char *name) {
    if (!handle || !name) return false;

    jstring name_string = handle->env->NewStringUTF(name);
    if (!name_string) return false; // TODO: ExceptionOccurred, etc. Handle java exceptions

    jmethodID set_name_method = handle->env->GetMethodID(handle->client_class, "SetName", "(Ljava/lang/String;)V");
    if (!set_name_method) return false; // TODO: ExceptionOccurred, etc. Handle java exceptions

    handle->env->CallVoidMethod(handle->client, set_name_method, name_string);
    if (handle->env->ExceptionOccurred()) return false;

    return true;
}

/**
* Register a function
*/
bool RegisterFunction(
    ClientHandle handle,
    const char *name,
    const char *(*parameters)[2],
    const char *(*returns)[2],
    void (*callback)(const void **, void **)
) {
    (void)handle;
    (void)name;
    (void)parameters;
    (void)returns;
    (void)callback;
    return false;
}
