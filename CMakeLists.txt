cmake_minimum_required(VERSION 3.14)
project(frontrowclient LANGUAGES C CXX ASM)

set(CMAKE_C_FLAGS "-std=c11 -Wall -Wextra -Wshadow -Werror -g -pthread")
set(CMAKE_CXX_FLAGS "-std=c++17 -Wall -Wextra -Wshadow -Werror -g -pthread")

include_directories(include "${CMAKE_CURRENT_BINARY_DIR}")

message(    OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/rs/debug/libclient.so"
)
add_custom_command(
    OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/rs/debug/libclient.so" FAKE_FILE # FAKE_FILE to make this target always be out-of-date
    COMMAND "${CMAKE_COMMAND}" -E env ${CARGO_ENV} "CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/rs" cargo build
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/rs/"
    DEPENDS "${GENERATED_CLASSES}"
    COMMENT "libclient.so (Rust)"
    BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/rs/"
)
add_custom_target(_client DEPENDS "${CMAKE_CURRENT_BINARY_DIR}/rs/debug/libclient.so")

add_library(client SHARED IMPORTED)
add_dependencies(client _client)
set_target_properties(client
    PROPERTIES
    IMPORTED_LOCATION "${CMAKE_CURRENT_BINARY_DIR}/rs/debug/libclient.so"
)

add_executable(simple_example src/simple_example_executable.c)
target_link_libraries(simple_example client)

add_executable(example src/example_executable.c)
target_link_libraries(example client)

add_executable(example_pi src/example_pi.c)
target_link_libraries(example_pi client )

find_library(WIRINGPI_LIBRARIES NAMES wiringPi)
target_link_libraries(example_pi ${WIRINGPI_LIBRARIES})

add_executable(demo_pi src/demo_pi.c)
target_link_libraries(demo_pi client )

find_library(WIRINGPI_LIBRARIES NAMES wiringPi)
target_link_libraries(demo_pi ${WIRINGPI_LIBRARIES})
target_link_libraries(demo_pi m)
