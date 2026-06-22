cmake_minimum_required(VERSION 2.8...4.3)
project(ffi_tests CXX)

add_executable(ffi_cpp_tests "${PROJECT_SOURCE_DIR}/tests/ffi.cpp")

set_target_properties(ffi_cpp_tests PROPERTIES CXX_STANDARD 11)

target_include_directories(ffi_cpp_tests PRIVATE "${PROJECT_SOURCE_DIR}/include")

target_link_libraries(ffi_cpp_tests PRIVATE
    "${PROJECT_SOURCE_DIR}/../target/debug/${CMAKE_STATIC_LIBRARY_PREFIX}loot_condition_interpreter_ffi${CMAKE_STATIC_LIBRARY_SUFFIX}")

if(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    target_link_libraries(ffi_cpp_tests PRIVATE pthread dl)
endif()

if(MSVC)
    target_link_libraries(ffi_cpp_tests PRIVATE ntdll ws2_32 Userenv bcrypt)
endif()

enable_testing()
add_test(ffi_cpp_tests ffi_cpp_tests)
