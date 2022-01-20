cmake_minimum_required(VERSION 3.14)
project(frontrowclient LANGUAGES C CXX ASM)

find_package(Java 1.11 REQUIRED COMPONENTS Development Runtime)
find_package(JNI 1.11 REQUIRED)

set(CMAKE_C_FLAGS "-std=c11 -Wall -Wextra -Wshadow -Werror -g")
set(CMAKE_CXX_FLAGS "-std=c++11 -Wall -Wextra -Wshadow -Werror -g")

include_directories(include ${JNI_INCLUDE_DIRS} ${CMAKE_CURRENT_BINARY_DIR})

set(GENERATED_CLASS ${CMAKE_CURRENT_BINARY_DIR}/frontrow/client/NativeClient.class)
set(GENERATED_CLASS_HEADER ${CMAKE_CURRENT_BINARY_DIR}/frontrow_client_NativeClient.h)
add_custom_command(
    OUTPUT ${GENERATED_CLASS} ${GENERATED_CLASS_HEADER}
    COMMAND ${Java_JAVAC_EXECUTABLE} -source 11 -target 11 frontrow/client/NativeClient.java -d ${CMAKE_CURRENT_BINARY_DIR} -h ${CMAKE_CURRENT_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/src/java
    DEPENDS src/java/frontrow/client/NativeClient.java
)

set(GENERATED_CLASS_OBJECT ${CMAKE_CURRENT_BINARY_DIR}/frontrow/client/NativeClient.class.o)
add_custom_command(
    OUTPUT ${GENERATED_CLASS_OBJECT}
    COMMAND ${CMAKE_ASM_COMPILER} ARGS -c src/incbin.S -DFILE='"'${GENERATED_CLASS}'"' -DOBJECT_START=__NativeClient_start -DOBJECT_END=__NativeClient_end -o ${GENERATED_CLASS_OBJECT}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    DEPENDS ${GENERATED_CLASS} src/incbin.S
)

add_library(client SHARED
    src/client.cpp
    src/frontrow_client_NativeClient.cpp
    ${GENERATED_CLASS_OBJECT}
)
target_link_libraries(client ${JNI_LIBRARIES})

add_executable(example src/example_executable.c)
target_link_libraries(example client)
