#include "frontrow_client_NativeCallback.h"
#include <vector>
#include <string>
#include <any>
#include <memory>
#include <cstdio>
#include <cstring> // strcmp

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

//    env->PushLocalFrame();

    jclass native_callback_class = env->GetObjectClass(self);
    jclass parameter_type_class = env->FindClass("frontrow/client/Parameter");

    jfieldID parameters_field = env->GetFieldID(
        native_callback_class,
        "parameters",
        "[Lfrontrow/client/Parameter;"
    );
    jfieldID returns_field = env->GetFieldID(
        native_callback_class,
        "returns",
        "[Lfrontrow/client/Parameter;"
    );
    jfieldID function_pointer_field = env->GetFieldID(
        native_callback_class,
        "function_pointer",
        "J"
    );
    env->ExceptionDescribe();

//    jfieldID parameter_name_field = env->GetFieldID(
//        parameter_type_class,
//        "name",
//        "Ljava/lang/String;"
//    );
    jfieldID parameter_type_field = env->GetFieldID(
        parameter_type_class,
        "type",
        "Ljava/lang/String;"
    );
    std::printf("parameter_type_field: %p\n", parameter_type_field);
    env->ExceptionDescribe();

    jobjectArray parameter_descriptors = (jobjectArray)env->GetObjectField(
        self,
        parameters_field
    );
    env->ExceptionDescribe();
    jobjectArray return_descriptors = (jobjectArray)env->GetObjectField(
        self,
        returns_field
    );
    env->ExceptionDescribe();
    jlong function_pointer = env->GetLongField(
        self,
        function_pointer_field
    );
    env->ExceptionDescribe();

    const jsize parameter_count = env->GetArrayLength(parameter_descriptors);
    if (parameter_count != env->GetArrayLength(parameters)) {
        // TODO: error handling
        std::printf("Incorrect number of parameters\n");
        return nullptr;
    }
    const jsize return_count = env->GetArrayLength(return_descriptors);

    std::vector<const void*> parameters_c;
    parameters_c.reserve(parameter_count);

    std::vector<std::any> parameters_buffer;
    parameters_buffer.reserve(parameter_count*2);

    std::vector<void*> returns_c;
    returns_c.reserve(return_count);

    std::vector<std::any> returns_buffer;
    returns_buffer.reserve(return_count*2);

    for (jsize i = 0; i < parameter_count; ++i) {
        jobject parameter = env->GetObjectArrayElement(
            parameters,
            i
        );
    env->ExceptionDescribe();
        jobject parameter_descriptor = env->GetObjectArrayElement(
            parameter_descriptors,
            i
        );
    env->ExceptionDescribe();
    std::printf("type_j: (%p, %p)\n", parameter_descriptor, parameter_type_field);
        jstring type_j = (jstring)env->GetObjectField(
            parameter_descriptor,
            parameter_type_field
        );
    std::printf("type_j: %p\n", type_j);
    env->ExceptionDescribe();

        const char *type = env->GetStringUTFChars(type_j, nullptr);

        jclass parameter_class = env->GetObjectClass(parameter);

        if (!std::strcmp(type, "int")) {
            // Marshall an int from an Integer (or something with int intValue())
            jmethodID int_value_method = env->GetMethodID(
                parameter_class,
                "intValue",
                "()I"
            );
            if (!int_value_method) {
                // TODO: error handling (parameter was wrong type)
                std::printf("Incorrect parameter type (no intValue method)\n");
                env->ReleaseStringUTFChars(type_j, type);
                return nullptr;
            }

            jint value = env->CallIntMethod(
                parameter,
                int_value_method
            ); // TODO: error handling

            std::shared_ptr<int> marshalled = std::make_shared<int>(value);
            parameters_c.push_back((const void*)marshalled.get());
            parameters_buffer.push_back(marshalled);
        } else if (!std::strcmp(type, "string")) {
            // Marshall a std::string from a String (or something with String toString())
            jmethodID to_string_method = env->GetMethodID(
                parameter_class,
                "toString",
                "()Ljava/lang/String;"
            );
            if (!to_string_method) {
                // TODO: error handling (parameter was wrong type)
                std::printf("Incorrect parameter type (no toString method)\n");
                env->ReleaseStringUTFChars(type_j, type);
                return nullptr;
            }

            jstring value = (jstring)env->CallObjectMethod(
                parameter,
                to_string_method
            ); // TODO: error handling

            jsize value_size = env->GetStringLength(value);
            const char *value_c = env->GetStringUTFChars(value, nullptr);

            parameters_buffer.push_back(std::string(value_c, value_size));
            std::any &param_any = parameters_buffer.back();
            std::string &param_string = *std::any_cast<std::string>(&param_any);
            std::shared_ptr<const char *> marshalled = std::make_shared<const char*>(param_string.c_str());
            parameters_c.push_back((const void*)marshalled.get());

            env->ReleaseStringUTFChars(value, value_c);
        } else {
            // TODO: error handling (probably should check this in NativeMethod constructor)
            std::printf("Unrecognized parameter type: %s\n", type);
            env->ReleaseStringUTFChars(type_j, type);
            return nullptr;
        }

        env->ReleaseStringUTFChars(type_j, type);
    }
    for (jsize i = 0; i < return_count; ++i) {
        jobject return_descriptor = env->GetObjectArrayElement(
            return_descriptors,
            i
        );
    env->ExceptionDescribe();
    std::printf("type_j: (%p, %p)\n", return_descriptor, parameter_type_field);
        jstring type_j = (jstring)env->GetObjectField(
            return_descriptor,
            parameter_type_field
        );
    std::printf("type_j: %p\n", type_j);
    env->ExceptionDescribe();

        const char *type = env->GetStringUTFChars(type_j, nullptr);

        if (!std::strcmp(type, "int")) {
            // Marshall an int

            std::shared_ptr<int> marshalled = std::make_shared<int>();
            returns_c.push_back((void*)marshalled.get());
            returns_buffer.push_back(marshalled);
        } else if (!std::strcmp(type, "string")) {
            // Marshall a string TODO: freeing

            std::shared_ptr<const char *> marshalled = std::make_shared<const char*>(nullptr);
            returns_c.push_back((void*)marshalled.get());
            returns_buffer.push_back(marshalled);
        } else {
            // TODO: error handling (probably should check this in NativeMethod constructor)
            std::printf("Unrecognized return type: %s\n", type);
            env->ReleaseStringUTFChars(type_j, type);
            return nullptr;
        }

        env->ReleaseStringUTFChars(type_j, type);
    }

    void (*function)(const void *const *const, void *const *const) = (void(*)(const void*const*const , void*const*const))function_pointer;
    for (auto ptr : parameters_c) {
        std::printf("parameter: %p\n", ptr);
    }
    for (auto ptr : returns_c) {
        std::printf("Return: %p\n", ptr);
    }
    function(&parameters_c[0], &returns_c[0]);

    std::printf("TODO: return\n");

    jobjectArray returns_j = env->NewObjectArray(
        return_count,
        env->FindClass("java/lang/Object"),
        nullptr
    );

    for (jsize i = 0; i < return_count; ++i) {
        jobject return_descriptor = env->GetObjectArrayElement(
            return_descriptors,
            i
        );
    env->ExceptionDescribe();
    std::printf("type_j: (%p, %p)\n", return_descriptor, parameter_type_field);
        jstring type_j = (jstring)env->GetObjectField(
            return_descriptor,
            parameter_type_field
        );
    std::printf("type_j: %p\n", type_j);
    env->ExceptionDescribe();

        const char *type = env->GetStringUTFChars(type_j, nullptr);

        if (!std::strcmp(type, "int")) {
            // Marshall an int to an Integer
            int value = *(int*)returns_c[i];

            jclass integer_class = env->FindClass("java/lang/Integer");
            jmethodID integer_constructor = env->GetMethodID(
                integer_class,
                "<init>",
                "(I)V"
            );

            jobject integer = env->NewObject(
                integer_class,
                integer_constructor,
                value
            );

            env->SetObjectArrayElement(
                returns_j,
                i,
                integer
            );
        } else if (!std::strcmp(type, "string")) {
            // Marshall a string to a String TODO: freeing
            // Marshall an int to an Integer
            const char *value = *(const char**)returns_c[i];

            jstring string = env->NewStringUTF(
                value
            );

            env->SetObjectArrayElement(
                returns_j,
                i,
                string
            );
        } else {
            // TODO: error handling (probably should check this in NativeMethod constructor)
            std::printf("Unrecognized return type: %s\n", type);
            env->ReleaseStringUTFChars(type_j, type);
            return nullptr;
        }

        env->ReleaseStringUTFChars(type_j, type);
    }

    return returns_j;
}
