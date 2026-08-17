cmake_minimum_required(VERSION 3.28)

project(fcitx5-windows-native-engine VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(FCITX_DEVELOPMENT_PEER_EXCEPTION
       "Allow explicitly marked development peers" OFF)

find_package(Fcitx5Core 5.1.14 REQUIRED CONFIG)

set(REPOSITORY_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/..")

add_executable(
  fcitx5_engine
  "${REPOSITORY_ROOT}/protocol/protocol.cpp"
  "${REPOSITORY_ROOT}/src/platform/runtime_identity.cpp"
  "${REPOSITORY_ROOT}/src/platform/pipe_security.cpp"
  "${REPOSITORY_ROOT}/src/ipc/peer_verification.cpp"
  "${REPOSITORY_ROOT}/src/engine/fcitx_runtime.cpp"
  "${REPOSITORY_ROOT}/src/engine/fcitx_dispatcher.cpp"
  "${REPOSITORY_ROOT}/src/engine/fcitx_engine_main.cpp")

set_target_properties(
  fcitx5_engine
  PROPERTIES OUTPUT_NAME "fcitx5-engine"
             RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

target_compile_definitions(fcitx5_engine PRIVATE UNICODE _UNICODE NOMINMAX WIN32_LEAN_AND_MEAN)
target_compile_options(fcitx5_engine PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_options(fcitx5_engine PRIVATE -municode)
target_include_directories(
  fcitx5_engine
  PRIVATE "${REPOSITORY_ROOT}/protocol"
          "${REPOSITORY_ROOT}/src/platform"
          "${REPOSITORY_ROOT}/src/ipc"
          "${REPOSITORY_ROOT}/src/engine")
target_link_libraries(fcitx5_engine PRIVATE Fcitx5::Core advapi32 user32)

if(FCITX_DEVELOPMENT_PEER_EXCEPTION)
  target_compile_definitions(fcitx5_engine PRIVATE FCITX_DEVELOPMENT_PEER_EXCEPTION=1)
endif()

install(TARGETS fcitx5_engine RUNTIME DESTINATION bin)
