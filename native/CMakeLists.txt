cmake_minimum_required(VERSION 3.14)
project(frontrowclient LANGUAGES C CXX ASM)

find_package(Java 1.11 REQUIRED COMPONENTS Development Runtime)
find_package(JNI 1.11 REQUIRED)

set(CMAKE_C_FLAGS "-std=c11 -Wall -Wextra -Wshadow -Werror -g -pthread")
set(CMAKE_CXX_FLAGS "-std=c++17 -Wall -Wextra -Wshadow -Werror -g -pthread")

include_directories(include ${JNI_INCLUDE_DIRS} ${CMAKE_CURRENT_BINARY_DIR})

set(CLIENT_LIBRARY_JAR_PATH "${CMAKE_CURRENT_BINARY_DIR}/ClientLibrary.jar")

macro(make_generated_class SOURCE_FILE GENERATED_CLASS GENERATED_CLASS_HEADER GENERATED_CLASS_OBJECT SYMBOL_NAME)
    list(APPEND GENERATED_CLASSES "${GENERATED_CLASS}")
    add_custom_command(
        OUTPUT "${GENERATED_CLASS}" "${GENERATED_CLASS_HEADER}"
        COMMAND ${Java_JAVAC_EXECUTABLE} -classpath "${CLIENT_LIBRARY_JAR_PATH}" -source 11 -target 11 ${SOURCE_FILE} -d ${CMAKE_CURRENT_BINARY_DIR} -h ${CMAKE_CURRENT_BINARY_DIR}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        DEPENDS src/java/frontrow/client/NativeClient.java
    )

    list(APPEND GENERATED_CLASS_OBJECTS ${GENERATED_CLASS_OBJECT})
    add_custom_command(
        OUTPUT ${GENERATED_CLASS_OBJECT}
        COMMAND ${CMAKE_ASM_COMPILER} ARGS -c src/incbin.S -DFILE='"'${GENERATED_CLASS}'"' -DOBJECT_START=__${SYMBOL_NAME}_start -DOBJECT_END=__${SYMBOL_NAME}_end -o ${GENERATED_CLASS_OBJECT}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        DEPENDS ${GENERATED_CLASS} src/incbin.S
    )
endmacro()

make_generated_class(
    src/java/frontrow/client/NativeClient.java
    ${CMAKE_CURRENT_BINARY_DIR}/frontrow/client/NativeClient.class
    ${CMAKE_CURRENT_BINARY_DIR}/frontrow_client_NativeClient.h
    ${CMAKE_CURRENT_BINARY_DIR}/frontrow/client/NativeClient.class.o
    NativeClient
)

make_generated_class(
    src/java/frontrow/client/NativeCallback.java
    ${CMAKE_CURRENT_BINARY_DIR}/frontrow/client/NativeCallback.class
    ${CMAKE_CURRENT_BINARY_DIR}/frontrow_client_NativeCallback.h
    ${CMAKE_CURRENT_BINARY_DIR}/frontrow/client/NativeCallback.class.o
    NativeCallback
)

add_library(client SHARED
    src/client.cpp
    src/frontrow_client_NativeClient.cpp
    src/frontrow_client_NativeCallback.cpp
    ${GENERATED_CLASS_OBJECTS}
)
target_link_libraries(client ${JNI_LIBRARIES})

add_executable(example src/example_executable.c)
target_link_libraries(example client)
