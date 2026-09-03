cmake_minimum_required(VERSION 3.28)

project(fcitx5-windows-native-engine VERSION 0.1.0 LANGUAGES C CXX)

include(CTest)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(FCITX_DEVELOPMENT_PEER_EXCEPTION
       "Allow explicitly marked development peers" OFF)

set(FCITX_RELEASE_CHANNEL "stable" CACHE STRING "Release identity: stable, beta, or nightly")
set_property(CACHE FCITX_RELEASE_CHANNEL PROPERTY STRINGS stable beta nightly)
if(FCITX_RELEASE_CHANNEL STREQUAL "stable")
  set(FCITX_RELEASE_CHANNEL_INDEX 0)
elseif(FCITX_RELEASE_CHANNEL STREQUAL "beta")
  set(FCITX_RELEASE_CHANNEL_INDEX 1)
elseif(FCITX_RELEASE_CHANNEL STREQUAL "nightly")
  set(FCITX_RELEASE_CHANNEL_INDEX 2)
else()
  message(FATAL_ERROR "FCITX_RELEASE_CHANNEL must be stable, beta, or nightly")
endif()

find_package(Fcitx5Core 5.1.14 REQUIRED CONFIG)

set(FCITX_NATIVE_RUNTIME_DLLS)
foreach(prefix IN LISTS CMAKE_PREFIX_PATH)
  if(EXISTS "${prefix}/bin")
    file(GLOB FCITX_NATIVE_PREFIX_DLLS CONFIGURE_DEPENDS "${prefix}/bin/*.dll")
    list(APPEND FCITX_NATIVE_RUNTIME_DLLS ${FCITX_NATIVE_PREFIX_DLLS})
  endif()
endforeach()
list(REMOVE_DUPLICATES FCITX_NATIVE_RUNTIME_DLLS)

function(fcitx_stage_native_runtime target)
  if(WIN32 AND FCITX_NATIVE_RUNTIME_DLLS)
    add_custom_command(
      TARGET ${target}
      POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E copy_if_different ${FCITX_NATIVE_RUNTIME_DLLS}
              "$<TARGET_FILE_DIR:${target}>"
      COMMAND_EXPAND_LISTS
      COMMENT "Staging native engine runtime DLLs for ${target}")
  endif()
endfunction()

set(REPOSITORY_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/..")
configure_file(
  "${REPOSITORY_ROOT}/cmake/release_identity.h.in"
  "${CMAKE_CURRENT_BINARY_DIR}/generated/fcitx5_windows/release_identity.h"
  @ONLY)

# ---------------------------------------------------------------------------
# Rust staticlib wiring (E1 protocol-core + E2 engine-core, GNU ABI).
#
# The MSYS2/clang64 lane links with the GNU ABI, so the Rust crates are built
# with the `x86_64-pc-windows-gnu` target into
# `out/toolchains/rust/target-gnu`. The Windows Rust toolchain provides cargo;
# rust-std for the GNU target is installed under the toolchain rustlib.
# Use the repository-pinned sccache by absolute path; an empty environment
# value does not override Cargo's configured wrapper.
# ---------------------------------------------------------------------------
set(FCITX_RUST_CARGO
    "${REPOSITORY_ROOT}/out/toolchains/rust/cargo-home/bin/cargo.exe")
set(FCITX_RUSTUP_HOME "${REPOSITORY_ROOT}/out/toolchains/rust/rustup-home")
set(FCITX_RUST_CARGO_HOME "${REPOSITORY_ROOT}/out/toolchains/rust/cargo-home")
set(FCITX_RUSTC_WRAPPER
    "${REPOSITORY_ROOT}/out/toolchains/fast/sccache-0.17.0/sccache-v0.17.0-x86_64-pc-windows-msvc/sccache.exe")
set(FCITX_RUST_TARGET_DIR "${REPOSITORY_ROOT}/out/toolchains/rust/target-gnu")
set(FCITX_RUST_TARGET "x86_64-pc-windows-gnu")
set(FCITX_RUST_CARGO_ENV
    "RUSTUP_HOME=${FCITX_RUSTUP_HOME}"
    "CARGO_HOME=${FCITX_RUST_CARGO_HOME}"
    "CARGO_TARGET_DIR=${FCITX_RUST_TARGET_DIR}"
    "CC_x86_64_pc_windows_gnu=${CMAKE_C_COMPILER}"
    "CXX_x86_64_pc_windows_gnu=${CMAKE_CXX_COMPILER}"
    "AR_x86_64_pc_windows_gnu=${CMAKE_AR}"
    "RUSTC_WRAPPER=${FCITX_RUSTC_WRAPPER}")

# Builds one Rust crate as a GNU staticlib and exposes it as
# `<target>_rust` (imported static library) plus `<target>_rustlib` (build
# step). `crate` is the Cargo package name; `crate_dir` the `rust/` directory.
function(fcitx_add_rust_staticlib target crate crate_dir)
  set(lib_name "${crate}")
  string(REPLACE "-" "_" lib_name "${lib_name}")
  set(staticlib
      "${FCITX_RUST_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/lib${lib_name}.a")
  set(rust_sources)
  file(GLOB rust_sources "${REPOSITORY_ROOT}/rust/${crate_dir}/src/*.rs")
  add_custom_command(
    OUTPUT "${staticlib}"
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_CARGO_ENV}
      "${FCITX_RUST_CARGO}" build --locked --manifest-path
      "${REPOSITORY_ROOT}/Cargo.toml" -p ${crate} --lib --target
      "${FCITX_RUST_TARGET}"
    DEPENDS
      "${REPOSITORY_ROOT}/Cargo.toml"
      "${REPOSITORY_ROOT}/Cargo.lock"
      "${REPOSITORY_ROOT}/rust/${crate_dir}/Cargo.toml"
      ${rust_sources}
    WORKING_DIRECTORY "${REPOSITORY_ROOT}"
    COMMENT "Building Rust ${crate} (GNU) staticlib")
  add_custom_target(${target}_rustlib DEPENDS "${staticlib}")
  add_library(${target}_rust STATIC IMPORTED GLOBAL)
  set_target_properties(${target}_rust
                        PROPERTIES IMPORTED_LOCATION "${staticlib}")
  add_dependencies(${target}_rust ${target}_rustlib)
endfunction()

fcitx_add_rust_staticlib(fcitx5_protocol_core fcitx5-protocol-core protocol-core)
fcitx_add_rust_staticlib(fcitx5_engine_core fcitx5-engine-core engine-core)
fcitx_add_rust_staticlib(fcitx5_windows_common_core fcitx5-windows-common-core
                         windows-common-core)
fcitx_add_rust_staticlib(fcitx5_control_core fcitx5-control-core control-core)
fcitx_add_rust_staticlib(fcitx5_config_core fcitx5-config-core config-core)
fcitx_add_rust_staticlib(fcitx5_package_core fcitx5-package-core package-core)

add_executable(
  fcitx5_engine
  "${REPOSITORY_ROOT}/src/platform/runtime_identity.cpp"
  "${REPOSITORY_ROOT}/src/platform/pipe_security.cpp"
  "${REPOSITORY_ROOT}/src/ipc/peer_verification.cpp"
  "${REPOSITORY_ROOT}/src/engine/key_event.cpp"
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
          "${CMAKE_CURRENT_BINARY_DIR}/generated"
          "${REPOSITORY_ROOT}/src/platform"
          "${REPOSITORY_ROOT}/src/ipc"
          "${REPOSITORY_ROOT}/src/config"
          "${REPOSITORY_ROOT}/src/engine")
target_link_libraries(
  fcitx5_engine
  PRIVATE Fcitx5::Core advapi32 user32
          fcitx5_protocol_core_rust fcitx5_engine_core_rust
          fcitx5_windows_common_core_rust fcitx5_control_core_rust
          fcitx5_config_core_rust fcitx5_package_core_rust
          ntdll ws2_32 userenv bcrypt)
fcitx_stage_native_runtime(fcitx5_engine)

# Platform keyboard layer following the same design used by Fcitx5 Android:
# keep keyboard-us as the first group item while letting the OS handle keys.
add_library(
  windowskeyboard MODULE
  "${REPOSITORY_ROOT}/src/engine/windows_keyboard.cpp")
set_target_properties(
  windowskeyboard
  PROPERTIES PREFIX "lib"
             OUTPUT_NAME "windowskeyboard")
target_compile_options(windowskeyboard PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(windowskeyboard PRIVATE Fcitx5::Core)

if(FCITX_DEVELOPMENT_PEER_EXCEPTION)
  target_compile_definitions(fcitx5_engine PRIVATE FCITX_DEVELOPMENT_PEER_EXCEPTION=1)
endif()

install(TARGETS fcitx5_engine RUNTIME DESTINATION bin)
install(TARGETS windowskeyboard RUNTIME DESTINATION lib/fcitx5 LIBRARY DESTINATION lib/fcitx5)
install(FILES windowskeyboard.conf DESTINATION share/fcitx5/addon)

if(BUILD_TESTING)
  add_executable(
    fcitx5_key_event_test
    "${REPOSITORY_ROOT}/src/engine/key_event.cpp"
    "${REPOSITORY_ROOT}/tests/unit/key_event_test.cpp")
  target_compile_definitions(
    fcitx5_key_event_test PRIVATE UNICODE _UNICODE NOMINMAX WIN32_LEAN_AND_MEAN)
  target_compile_options(fcitx5_key_event_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
  target_include_directories(
    fcitx5_key_event_test
    PRIVATE "${REPOSITORY_ROOT}/protocol" "${REPOSITORY_ROOT}/src/engine")
  target_link_libraries(fcitx5_key_event_test PRIVATE Fcitx5::Core user32)
  fcitx_stage_native_runtime(fcitx5_key_event_test)
  add_test(NAME key-event-contract COMMAND fcitx5_key_event_test)
endif()
