#include "client.h"
#include <string>
#include <cstdio>
#include <jni.h>

struct ClientHandle_t {
    JavaVM *jvm;
    JNIEnv *env;
    jclass client_class; // global reference
    jobject client; // global reference
    jclass native_callback_class; // global reference

    ClientHandle_t(JavaVM *jvm_, JNIEnv *env_, jclass client_class_, jobject client_, jclass native_callback_class_)
     : jvm(jvm_), env(env_), client_class(client_class_), client(client_), native_callback_class(native_callback_class_)
    {}

    jclass _parameter_class = nullptr; // global reference
    jclass _string_class = nullptr; // global reference

    jclass parameter_class(void) {
        if (_parameter_class) return _parameter_class;
        jclass parameter_class = env->FindClass("frontrow/client/Parameter");
        _parameter_class = (jclass)env->NewGlobalRef((jobject)parameter_class);
//        std::printf("parameter_class: %p\n", _parameter_class);
        return _parameter_class;
    }
    jclass string_class(void) {
        if (_string_class) return _string_class;
        jclass string_class = env->FindClass("java/lang/String");
        _string_class = (jclass)env->NewGlobalRef((jobject)string_class);
        return _string_class;
    }

    jmethodID parameter_constructor(void) {
        return env->GetMethodID(parameter_class(), "<init>", "(Ljava/lang/String;Ljava/lang/String;)V");
    }

    jobject new_parameter(const char *name, const char *type) {
        auto p = env->NewObject(parameter_class(), parameter_constructor(), new_string(name), new_string(type));
//        std::printf("%p\n", p);
        return p;
    }

    jmethodID native_callback_constructor(void) {
        return env->GetMethodID(native_callback_class, "<init>", "([Lfrontrow/client/Parameter;[Lfrontrow/client/Parameter;J)V");
    }

    jobject new_native_callback(jobjectArray parameters, jobjectArray returns, jlong callback) {
        return env->NewObject(native_callback_class, native_callback_constructor(), parameters, returns, callback);
    }

    jstring new_string(const char *string) {
        auto s = env->NewStringUTF(string);
//        std::printf("string %p (%s)\n", s, string);
        return s;
    }
};

extern const char __NativeClient_start[];
extern const char __NativeClient_end[];
extern const char __NativeCallback_start[];
extern const char __NativeCallback_end[];

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
/*
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
*/

    jclass native_callback_class = env->DefineClass(
        "frontrow/client/NativeCallback",
        system_loader,
        (const jbyte*)__NativeCallback_start,
        __NativeCallback_end - __NativeCallback_start
    );
    if (native_callback_class == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not load frontrow.client.NativeCallback.\n");
        auto size = __NativeCallback_end - __NativeCallback_start;

        std::fprintf(stderr, "%zu %zx\n", size, size);

        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }

    jclass native_callback_class_global = (jclass)env->NewGlobalRef(
        native_callback_class
    );
    if (native_callback_class_global == NULL) {
        // TODO: error handling
        std::fprintf(stderr, "Could not create global reference.\n");
        env->ExceptionDescribe();
        jvm->DestroyJavaVM();
        return nullptr;
    }


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
        client_global,
        native_callback_class_global
    };
}

/**
* Deinitialize and shut down the library.
*/
extern "C" void ShutdownLibrary(ClientHandle handle) {
    if (!handle) return;

    jmethodID shutdown_library_method = handle->env->GetMethodID(handle->client_class, "ShutdownLibrary", "()V");
    if (!shutdown_library_method) return; // TODO: ExceptionOccurred, etc. Handle java exceptions

    handle->env->CallVoidMethod(handle->client, shutdown_library_method);
    if (handle->env->ExceptionOccurred()) return; // TODO: Handle java exceptions

//    handle->env->ExceptionDescribe();
    handle->jvm->DestroyJavaVM();
    delete handle;
}

/**
* Updates internal library state and calls any necessary callbacks.
*/
extern "C" bool LibraryUpdate(ClientHandle handle) {
    if (!handle) return std::printf("%d: error\n", __LINE__), false;
std::printf("ASDF\n");

    jmethodID library_update_method = handle->env->GetMethodID(handle->client_class, "LibraryUpdate", "()V");
    if (!library_update_method) return std::printf("%d: error\n", __LINE__), false; // TODO: ExceptionOccurred, etc. Handle java exceptions
std::printf("ASDF\n");

    handle->env->CallVoidMethod(handle->client, library_update_method);
    handle->env->ExceptionDescribe();
    if (handle->env->ExceptionOccurred()) return std::printf("%d: error\n", __LINE__), false;
std::printf("ASDF\n");
    return true;
}

/**
* Set the name of the client.
* On success: returns true
* On failure: returns false
*/
extern "C" bool SetName(ClientHandle handle, const char *name) {
    if (!handle || !name) return std::printf("%d: error\n", __LINE__), false;

    jstring name_string = handle->new_string(name);
    if (!name_string) return std::printf("%d: error\n", __LINE__), false; // TODO: ExceptionOccurred, etc. Handle java exceptions

    jmethodID set_name_method = handle->env->GetMethodID(handle->client_class, "SetName", "(Ljava/lang/String;)V");
    if (!set_name_method) return std::printf("%d: error\n", __LINE__), false; // TODO: ExceptionOccurred, etc. Handle java exceptions

    handle->env->CallVoidMethod(handle->client, set_name_method, name_string);
    if (handle->env->ExceptionOccurred()) return std::printf("%d: error\n", __LINE__), false;

    return true;
}

/**
* Register a function
*/
extern "C" bool RegisterFunction(
    ClientHandle handle,
    const char *name,
    const char *(*parameters)[2],
    const char *(*returns)[2],
    void (*callback)(const void *const*const, void *const*const)
) {
    if (!handle || !callback) return std::printf("%d: error\n", __LINE__), false;

    jstring name_j = handle->new_string(name);

    unsigned long parameter_count = 0;
    unsigned long return_count = 0;
    if (parameters) {
        while (parameters[parameter_count][0] != NULL) ++parameter_count;
    }
    if (returns) {
        while (returns[return_count][0] != NULL) ++return_count;
    }

    std::printf("function %s has %lu parameters and %lu returns\n", name, parameter_count, return_count);

    jobjectArray parameters_j = handle->env->NewObjectArray(
        parameter_count,
        handle->parameter_class(),
        nullptr
    );
    if (handle->env->ExceptionOccurred()) return std::printf("%d: error\n", __LINE__), false;

    jobjectArray returns_j = handle->env->NewObjectArray(
        return_count,
        handle->parameter_class(),
        nullptr
    );
    if (handle->env->ExceptionOccurred()) return std::printf("%d: error\n", __LINE__), false;

    for (unsigned long i = 0; i < parameter_count; ++i) {
        jobject parameter = handle->new_parameter(parameters[i][0], parameters[i][1]);
        std::printf("new parameter: %p\n", parameter);
        handle->env->SetObjectArrayElement(parameters_j, i, parameter);
    }

    for (unsigned long i = 0; i < return_count; ++i) {
        jobject return_ = handle->new_parameter(returns[i][0], returns[i][1]);
        handle->env->SetObjectArrayElement(returns_j, i, return_);
    }

    jobject callback_j = handle->new_native_callback(parameters_j, returns_j, (jlong)callback);

    jmethodID register_function_method = handle->env->GetMethodID(
        handle->client_class,
        "RegisterFunction",
        "(Ljava/lang/String;[Lfrontrow/client/Parameter;[Lfrontrow/client/Parameter;Lfrontrow/client/Callback;)V"
    );
    handle->env->ExceptionDescribe();
    if (!register_function_method) return std::printf("%d: error\n", __LINE__), false; // TODO: ExceptionOccurred, etc. Handle java exceptions

    handle->env->CallVoidMethod(
        handle->client,
        register_function_method,
        name_j,
        parameters_j,
        returns_j,
        callback_j
    );
    handle->env->ExceptionDescribe();
    if (handle->env->ExceptionOccurred()) return std::printf("%d: error\n", __LINE__), false; // TODO: Handle java exceptions
    return true;
}
