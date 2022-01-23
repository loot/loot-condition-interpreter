cmake_minimum_required(VERSION 2.8)
project(ffi_tests CXX)

include_directories("${CMAKE_SOURCE_DIR}/include")

set(CMAKE_CXX_STANDARD 11)

if (CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    set (SYSTEM_LIBS pthread dl)
endif ()

if (MSVC)
    set (SYSTEM_LIBS ws2_32 Userenv bcrypt)
endif ()

set (FFI_LIBRARY "${CMAKE_SOURCE_DIR}/../target/debug/${CMAKE_STATIC_LIBRARY_PREFIX}loot_condition_interpreter_ffi${CMAKE_STATIC_LIBRARY_SUFFIX}")

add_executable(ffi_cpp_tests "${CMAKE_SOURCE_DIR}/tests/ffi.cpp")
target_link_libraries(ffi_cpp_tests ${FFI_LIBRARY} ${SYSTEM_LIBS})

enable_testing()
add_test(ffi_cpp_tests ffi_cpp_tests)
