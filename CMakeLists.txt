cmake_minimum_required(VERSION 3.29)

project(
  fcitx5-windows-next
  VERSION 0.1.0
  DESCRIPTION "Native Fcitx5 platform frontend for Windows"
  LANGUAGES C CXX)

if(POLICY CMP0141)
  cmake_policy(SET CMP0141 NEW)
endif()
set(CMAKE_MSVC_DEBUG_INFORMATION_FORMAT
    "$<$<CONFIG:Debug,RelWithDebInfo>:Embedded>")
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)

include(CTest)

set(FCITX_WTL_INCLUDE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/wtl-10.1.0/lib/native/include"
    CACHE PATH "Path to the pinned WTL 10.1.0 headers")
if(NOT EXISTS "${FCITX_WTL_INCLUDE_DIR}/atlapp.h")
  message(FATAL_ERROR
          "Pinned WTL headers are missing. Run tools/prepare-wtl.ps1.")
endif()

set(FCITX_NLOHMANN_JSON_INCLUDE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/nlohmann-json-3.12.0"
    CACHE PATH "Path to the pinned nlohmann/json 3.12.0 header")
if(NOT EXISTS "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}/nlohmann/json.hpp")
  message(FATAL_ERROR
          "Pinned nlohmann/json header is missing. Run tools/prepare-package-dependencies.ps1.")
endif()

set(FCITX_MINIZ_SOURCE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/miniz-3.1.2"
    CACHE PATH "Path to the pinned miniz 3.1.2 sources")
if(NOT EXISTS "${FCITX_MINIZ_SOURCE_DIR}/miniz.c" OR
   NOT EXISTS "${FCITX_MINIZ_SOURCE_DIR}/miniz.h")
  message(FATAL_ERROR
          "Pinned miniz sources are missing. Run tools/prepare-package-dependencies.ps1.")
endif()

set(FCITX_TOMLPLUSPLUS_INCLUDE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/third_party/tomlplusplus/include"
    CACHE PATH "Path to the pinned tomlplusplus headers")

set(FCITX_MLDSA_NATIVE_SOURCE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/mldsa-native-2.0.0/mldsa-native-2.0.0"
    CACHE PATH "Path to the pinned mldsa-native 2.0.0 sources")
if(NOT EXISTS "${FCITX_MLDSA_NATIVE_SOURCE_DIR}/mldsa/mldsa_native.c" OR
   NOT EXISTS "${FCITX_MLDSA_NATIVE_SOURCE_DIR}/mldsa/mldsa_native.h")
  message(FATAL_ERROR
          "Pinned mldsa-native sources are missing. Run tools/prepare-package-dependencies.ps1.")
endif()

set(FCITX_BLAKE3_SOURCE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/blake3-1.8.7/BLAKE3-1.8.7/c"
    CACHE PATH "Path to the pinned BLAKE3 1.8.7 C sources")
if(NOT EXISTS "${FCITX_BLAKE3_SOURCE_DIR}/blake3.c" OR
   NOT EXISTS "${FCITX_BLAKE3_SOURCE_DIR}/blake3.h")
  message(FATAL_ERROR
          "Pinned BLAKE3 sources are missing. Run tools/prepare-package-dependencies.ps1.")
endif()

option(FCITX_WARNINGS_AS_ERRORS "Treat project warnings as errors" ON)
option(FCITX_ENABLE_MSVC_ANALYZE "Run MSVC static analysis" OFF)
option(FCITX_BUILD_BENCHMARKS "Build repeatable microbenchmarks" ON)
option(FCITX_BUILD_FUZZERS "Build security-boundary fuzz harnesses" ON)
option(FCITX_DEVELOPMENT_PEER_EXCEPTION
       "Allow explicitly marked same-user/session peers in development builds" OFF)
option(FCITX_ENABLE_PCH "Use project precompiled headers for C++ targets" ON)
set(FCITX_TARGET_ARCH
    ""
    CACHE STRING "Windows target architecture: x64, x86, or arm64")
set_property(CACHE FCITX_TARGET_ARCH PROPERTY STRINGS "" x64 x86 arm64)
set(FCITX_EFFECTIVE_TARGET_ARCH "${FCITX_TARGET_ARCH}")
if(NOT FCITX_EFFECTIVE_TARGET_ARCH)
  set(FCITX_CMAKE_PLATFORM "${CMAKE_VS_PLATFORM_NAME}")
  if(NOT FCITX_CMAKE_PLATFORM)
    set(FCITX_CMAKE_PLATFORM "${CMAKE_GENERATOR_PLATFORM}")
  endif()
  if(FCITX_CMAKE_PLATFORM MATCHES "^[Ww]in32$" OR CMAKE_SIZEOF_VOID_P EQUAL 4)
    set(FCITX_EFFECTIVE_TARGET_ARCH "x86")
  elseif(FCITX_CMAKE_PLATFORM MATCHES "^[Aa][Rr][Mm]64$" OR
         CMAKE_SYSTEM_PROCESSOR MATCHES "^(ARM64|AARCH64|arm64|aarch64)$")
    set(FCITX_EFFECTIVE_TARGET_ARCH "arm64")
  else()
    set(FCITX_EFFECTIVE_TARGET_ARCH "x64")
  endif()
endif()
message(STATUS "Fcitx Windows target architecture: ${FCITX_EFFECTIVE_TARGET_ARCH}")
if(FCITX_EFFECTIVE_TARGET_ARCH STREQUAL "x86")
  set(FCITX_RUST_TARGET "i686-pc-windows-msvc")
elseif(FCITX_EFFECTIVE_TARGET_ARCH STREQUAL "arm64")
  set(FCITX_RUST_TARGET "aarch64-pc-windows-msvc")
else()
  set(FCITX_RUST_TARGET "x86_64-pc-windows-msvc")
endif()

set(FCITX_COMPILER_IS_CLANG_CL OFF)
set(FCITX_COMPILER_IS_MSVC_CL OFF)
if(MSVC AND CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
  set(FCITX_COMPILER_IS_CLANG_CL ON)
  message(STATUS "Default Windows toolchain: clang-cl / Ninja / lld-link")
elseif(MSVC AND CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
  set(FCITX_COMPILER_IS_MSVC_CL ON)
  message(STATUS "Windows toolchain: MSVC cl.exe compatibility path")
endif()

find_program(
  FCITX_CARGO_EXECUTABLE cargo
  PATHS "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/rust/cargo-home/bin"
  NO_DEFAULT_PATH)
find_program(FCITX_CARGO_EXECUTABLE cargo REQUIRED)
find_program(
  FCITX_SCCACHE_EXECUTABLE sccache
  PATHS
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/fast/sccache-0.17.0/sccache-v0.17.0-x86_64-pc-windows-msvc"
  NO_DEFAULT_PATH)
find_program(FCITX_SCCACHE_EXECUTABLE sccache)
find_program(FCITX_PWSH_EXECUTABLE pwsh powershell REQUIRED)
file(TO_CMAKE_PATH "${FCITX_PWSH_EXECUTABLE}" FCITX_PWSH_COMMAND)
set(FCITX_RUSTUP_HOME "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/rust/rustup-home" CACHE PATH
    "Pinned Rustup home")
set(FCITX_CARGO_HOME "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/rust/cargo-home" CACHE PATH
    "Pinned Cargo home")
if(DEFINED ENV{FCITX_CARGO_TARGET_DIR} AND NOT "$ENV{FCITX_CARGO_TARGET_DIR}" STREQUAL "")
  set(_fcitx_default_cargo_target_dir "$ENV{FCITX_CARGO_TARGET_DIR}")
else()
  set(_fcitx_default_cargo_target_dir "${CMAKE_CURRENT_SOURCE_DIR}/out/cargo-target")
endif()
set(FCITX_CARGO_TARGET_DIR "${_fcitx_default_cargo_target_dir}" CACHE PATH
    "Worktree-local Cargo target directory")
unset(_fcitx_default_cargo_target_dir)
set(FCITX_RUST_BUILD_ENV)
if("$ENV{GITHUB_ACTIONS}" STREQUAL "true" AND
   NOT "$ENV{ACTIONS_RUNTIME_TOKEN}" STREQUAL "" AND
   (NOT "$ENV{ACTIONS_CACHE_URL}" STREQUAL "" OR
    NOT "$ENV{ACTIONS_RESULTS_URL}" STREQUAL ""))
  list(APPEND FCITX_RUST_BUILD_ENV "SCCACHE_GHA_ENABLED=true")
elseif(NOT "$ENV{SCCACHE_DIR}" STREQUAL "")
  list(APPEND FCITX_RUST_BUILD_ENV "--unset=SCCACHE_GHA_ENABLED")
else()
  list(APPEND FCITX_RUST_BUILD_ENV "--unset=SCCACHE_GHA_ENABLED")
endif()
list(APPEND FCITX_RUST_BUILD_ENV
     "RUSTUP_HOME=${FCITX_RUSTUP_HOME}"
     "CARGO_HOME=${FCITX_CARGO_HOME}"
     "CARGO_TARGET_DIR=${FCITX_CARGO_TARGET_DIR}")
if(NOT "$ENV{RUSTC_WRAPPER}" STREQUAL "")
  list(APPEND FCITX_RUST_BUILD_ENV "RUSTC_WRAPPER=$ENV{RUSTC_WRAPPER}")
elseif(FCITX_SCCACHE_EXECUTABLE)
  list(APPEND FCITX_RUST_BUILD_ENV "RUSTC_WRAPPER=${FCITX_SCCACHE_EXECUTABLE}")
endif()
if(NOT "$ENV{SCCACHE_DIR}" STREQUAL "")
  list(APPEND FCITX_RUST_BUILD_ENV "SCCACHE_DIR=$ENV{SCCACHE_DIR}")
endif()
set(FCITX_RELEASE_CHANNEL "stable" CACHE STRING "Release identity: stable, beta, or nightly")
set_property(CACHE FCITX_RELEASE_CHANNEL PROPERTY STRINGS stable beta nightly)
if(FCITX_RELEASE_CHANNEL STREQUAL "stable")
  set(FCITX_RELEASE_CHANNEL_INDEX 0)
  set(FCITX_RELEASE_DATA_DIRECTORY "Fcitx5")
elseif(FCITX_RELEASE_CHANNEL STREQUAL "beta")
  set(FCITX_RELEASE_CHANNEL_INDEX 1)
  set(FCITX_RELEASE_DATA_DIRECTORY "Fcitx5-Beta")
elseif(FCITX_RELEASE_CHANNEL STREQUAL "nightly")
  set(FCITX_RELEASE_CHANNEL_INDEX 2)
  set(FCITX_RELEASE_DATA_DIRECTORY "Fcitx5-Nightly")
else()
  message(FATAL_ERROR "FCITX_RELEASE_CHANNEL must be stable, beta, or nightly")
endif()

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
include(CompilerOptions)
set(FCITX_PROJECT_PCH_HEADER "${CMAKE_CURRENT_SOURCE_DIR}/src/pch/fcitx_windows_pch.h")

configure_file(
  cmake/version.h.in
  "${CMAKE_CURRENT_BINARY_DIR}/generated/fcitx5_windows/version.h"
  @ONLY)
configure_file(
  cmake/release_identity.h.in
  "${CMAKE_CURRENT_BINARY_DIR}/generated/fcitx5_windows/release_identity.h"
  @ONLY)

add_library(fcitx5_release_identity INTERFACE)
add_library(fcitx5::release_identity ALIAS fcitx5_release_identity)
target_include_directories(
  fcitx5_release_identity
  INTERFACE "$<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/generated>")

set(FCITX_RUST_WINDOWS_COMMON_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_windows_common_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_WINDOWS_COMMON_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_WINDOWS_VERSION=${PROJECT_VERSION}"
    "FCITX_RELEASE_CHANNEL_NAME=${FCITX_RELEASE_CHANNEL}"
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-windows-common-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/windows-common-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/windows-common-core/src/lib.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-windows-common-core staticlib")
add_custom_target(fcitx5_windows_common_rustlib
                  DEPENDS "${FCITX_RUST_WINDOWS_COMMON_STATICLIB}")
add_library(fcitx5_windows_common STATIC IMPORTED GLOBAL)
add_library(fcitx5::windows_common ALIAS fcitx5_windows_common)
set_target_properties(fcitx5_windows_common PROPERTIES
                      IMPORTED_LOCATION "${FCITX_RUST_WINDOWS_COMMON_STATICLIB}")
add_dependencies(fcitx5_windows_common fcitx5_windows_common_rustlib)
target_include_directories(
  fcitx5_windows_common
  INTERFACE "$<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/generated>")
target_link_libraries(fcitx5_windows_common INTERFACE ws2_32 userenv ntdll bcrypt advapi32 shell32 ole32 gdi32)

add_library(fcitx5_protocol STATIC protocol/protocol.cpp)
add_library(fcitx5::protocol ALIAS fcitx5_protocol)
target_compile_features(fcitx5_protocol PUBLIC cxx_std_20)
target_include_directories(fcitx5_protocol PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/protocol")
# E1 cutover: the codec is authoritative in Rust `fcitx5-protocol-core`; this
# static library only marshals over the C ABI, so every consumer links the Rust
# codec transitively.
target_link_libraries(fcitx5_protocol PUBLIC fcitx5_protocol_core_rust)
fcitx_apply_project_options(fcitx5_protocol)

set(FCITX_RUST_LAUNCHER_EXE
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/$<IF:$<CONFIG:Debug>,debug,release>/fcitx5-launcher.exe")
set(FCITX_LAUNCHER_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>")
set(FCITX_LAUNCHER_SHIPPING_EXE "${FCITX_LAUNCHER_OUTPUT_DIR}/fcitx5-launcher.exe")
add_custom_target(
  fcitx5_launcher ALL
  BYPRODUCTS "${FCITX_LAUNCHER_SHIPPING_EXE}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_WINDOWS_VERSION=${PROJECT_VERSION}"
    "FCITX_RELEASE_DATA_DIRECTORY=${FCITX_RELEASE_DATA_DIRECTORY}"
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-launcher-core --bin
    fcitx5-launcher --target "${FCITX_RUST_TARGET}" --profile
    "$<IF:$<CONFIG:Debug>,dev,release>"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${FCITX_LAUNCHER_OUTPUT_DIR}"
  COMMAND "${CMAKE_COMMAND}" -E copy_if_different
          "${FCITX_RUST_LAUNCHER_EXE}" "${FCITX_LAUNCHER_SHIPPING_EXE}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/launcher-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/launcher-core/build.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/launcher-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/launcher-core/src/main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/windows/launcher.rc"
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/windows/resource.h"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-launcher.exe")

set(FCITX_RUST_PROCESS_EXECUTION_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_process_execution_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_PROCESS_EXECUTION_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-process-execution-core --lib --target
    "${FCITX_RUST_TARGET}"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-process-execution-core staticlib")
add_custom_target(fcitx5_process_execution_core_rustlib
                  DEPENDS "${FCITX_RUST_PROCESS_EXECUTION_CORE_STATICLIB}")
add_library(fcitx5_process_execution_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(
  fcitx5_process_execution_core_rust
  PROPERTIES IMPORTED_LOCATION "${FCITX_RUST_PROCESS_EXECUTION_CORE_STATICLIB}")
add_dependencies(fcitx5_process_execution_core_rust fcitx5_process_execution_core_rustlib)

set(FCITX_RUST_CONTROL_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_control_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_CONTROL_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_WINDOWS_VERSION=${PROJECT_VERSION}"
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-control-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/control-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/control-core/src/lib.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-control-core staticlib")
add_custom_target(fcitx5_control_core_rustlib
                  DEPENDS "${FCITX_RUST_CONTROL_CORE_STATICLIB}")
add_library(fcitx5_control_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(fcitx5_control_core_rust
                      PROPERTIES IMPORTED_LOCATION "${FCITX_RUST_CONTROL_CORE_STATICLIB}")
add_dependencies(fcitx5_control_core_rust fcitx5_control_core_rustlib)
target_link_libraries(fcitx5_control_core_rust INTERFACE advapi32 bcrypt ntdll ole32 userenv ws2_32)

set(FCITX_RUST_REGISTER_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_register_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_REGISTER_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-register-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/register-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/register-core/src/lib.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-register-core staticlib")
add_custom_target(fcitx5_register_core_rustlib
                  DEPENDS "${FCITX_RUST_REGISTER_CORE_STATICLIB}")
add_library(fcitx5_register_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(fcitx5_register_core_rust
                      PROPERTIES IMPORTED_LOCATION "${FCITX_RUST_REGISTER_CORE_STATICLIB}")
add_dependencies(fcitx5_register_core_rust fcitx5_register_core_rustlib)

set(FCITX_RUST_CANDIDATE_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_candidate_core.lib")
set(FCITX_RUST_CANDIDATE_POC_EXE
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5-candidate-poc.exe")
add_custom_command(
  OUTPUT "${FCITX_RUST_CANDIDATE_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/candidate-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/candidate-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/candidate-core/src/qingfeng.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-candidate-core staticlib")
add_custom_target(fcitx5_candidate_core_rustlib
                  DEPENDS "${FCITX_RUST_CANDIDATE_CORE_STATICLIB}")
add_library(fcitx5_candidate_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(fcitx5_candidate_core_rust
                      PROPERTIES IMPORTED_LOCATION "${FCITX_RUST_CANDIDATE_CORE_STATICLIB}")
add_dependencies(fcitx5_candidate_core_rust fcitx5_candidate_core_rustlib)
add_custom_target(
  fcitx5_candidate_poc_rustbin
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
    fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-candidate-poc executable")

set(FCITX_RUST_PROTOCOL_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_protocol_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_PROTOCOL_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-protocol-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/src/capi.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/src/capi_tests.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/src/tests.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-protocol-core staticlib")
add_custom_target(fcitx5_protocol_core_rustlib
                  DEPENDS "${FCITX_RUST_PROTOCOL_CORE_STATICLIB}")
add_library(fcitx5_protocol_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(fcitx5_protocol_core_rust
                      PROPERTIES IMPORTED_LOCATION "${FCITX_RUST_PROTOCOL_CORE_STATICLIB}")
add_dependencies(fcitx5_protocol_core_rust fcitx5_protocol_core_rustlib)
target_link_libraries(fcitx5_protocol_core_rust INTERFACE ws2_32 userenv ntdll)

# E2 side-by-side: Engine context/composition/revision ledger is Rust
# authoritative in `fcitx5-engine-core`; `src/engine/fcitx_runtime.cpp` is not
# cut over yet (see docs/engine-boundary.md migration order).
set(FCITX_RUST_ENGINE_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_engine_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_ENGINE_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-engine-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/engine-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/engine-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/engine-core/src/capi.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/engine-core/src/capi_tests.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/engine-core/src/tests.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-engine-core staticlib")
add_custom_target(fcitx5_engine_core_rustlib
                  DEPENDS "${FCITX_RUST_ENGINE_CORE_STATICLIB}")
add_library(fcitx5_engine_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(fcitx5_engine_core_rust
                      PROPERTIES IMPORTED_LOCATION "${FCITX_RUST_ENGINE_CORE_STATICLIB}")
add_dependencies(fcitx5_engine_core_rust fcitx5_engine_core_rustlib)
target_link_libraries(fcitx5_engine_core_rust INTERFACE ws2_32 userenv ntdll)

set(FCITX_RUST_TSF_POC_DLL
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_tsf_poc.dll")
add_custom_target(
  fcitx5_tsf_poc_rustdll
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-tsf-poc --lib --target
    "${FCITX_RUST_TARGET}"
  BYPRODUCTS "${FCITX_RUST_TSF_POC_DLL}"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-tsf-poc DLL")

add_library(fcitx5_miniz STATIC "${FCITX_MINIZ_SOURCE_DIR}/miniz.c")
target_include_directories(fcitx5_miniz SYSTEM PUBLIC "${FCITX_MINIZ_SOURCE_DIR}")
target_compile_definitions(fcitx5_miniz PRIVATE MINIZ_NO_ZLIB_APIS)
if(MSVC)
  target_compile_options(fcitx5_miniz PRIVATE /W3)
  if(FCITX_COMPILER_IS_MSVC_CL)
    target_compile_options(fcitx5_miniz PRIVATE /sdl)
  endif()
endif()

add_library(
  fcitx5_mldsa65_verify STATIC
  "${FCITX_MLDSA_NATIVE_SOURCE_DIR}/mldsa/mldsa_native.c")
target_include_directories(
  fcitx5_mldsa65_verify
  SYSTEM PUBLIC "${FCITX_MLDSA_NATIVE_SOURCE_DIR}")
target_include_directories(
  fcitx5_mldsa65_verify
  PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/package")
target_compile_definitions(
  fcitx5_mldsa65_verify
  PRIVATE MLD_CONFIG_FILE=\"fcitx5_mldsa65_config.h\")
if(MSVC)
  target_compile_options(fcitx5_mldsa65_verify PRIVATE /W3)
  if(FCITX_COMPILER_IS_MSVC_CL)
    target_compile_options(fcitx5_mldsa65_verify PRIVATE /sdl)
  endif()
endif()

add_library(
  fcitx5_blake3 STATIC
  "${FCITX_BLAKE3_SOURCE_DIR}/blake3.c"
  "${FCITX_BLAKE3_SOURCE_DIR}/blake3_dispatch.c"
  "${FCITX_BLAKE3_SOURCE_DIR}/blake3_portable.c")
target_include_directories(fcitx5_blake3 SYSTEM PUBLIC "${FCITX_BLAKE3_SOURCE_DIR}")
target_compile_definitions(
  fcitx5_blake3
  PRIVATE BLAKE3_NO_SSE2 BLAKE3_NO_SSE41 BLAKE3_NO_AVX2 BLAKE3_NO_AVX512
          BLAKE3_USE_NEON=0)
if(MSVC)
  target_compile_options(fcitx5_blake3 PRIVATE /W3 /wd4244)
  if(FCITX_COMPILER_IS_MSVC_CL)
    target_compile_options(fcitx5_blake3 PRIVATE /sdl)
  endif()
endif()

add_library(
  fcitx5_package_core STATIC
  src/package/package_core.cpp
  )
add_library(fcitx5::package_core ALIAS fcitx5_package_core)
target_compile_features(fcitx5_package_core PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_package_core PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/package")
target_include_directories(
  fcitx5_package_core SYSTEM PUBLIC "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}")
target_include_directories(
  fcitx5_package_core SYSTEM PRIVATE "${FCITX_MLDSA_NATIVE_SOURCE_DIR}")
target_include_directories(
  fcitx5_package_core SYSTEM PRIVATE "${FCITX_BLAKE3_SOURCE_DIR}")
target_link_libraries(
  fcitx5_package_core
  PUBLIC bcrypt crypt32
  PRIVATE fcitx5_miniz fcitx5_mldsa65_verify fcitx5_blake3)
fcitx_apply_project_options(fcitx5_package_core)
if(FCITX_COMPILER_IS_MSVC_CL)
  target_compile_options(fcitx5_package_core PRIVATE /external:W0 /analyze:external-)
endif()

add_library(fcitx5_config STATIC src/config/config_parser.cpp)
add_library(fcitx5::config ALIAS fcitx5_config)
target_compile_features(fcitx5_config PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_config
  PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/config")
target_include_directories(
  fcitx5_config SYSTEM
  PRIVATE "${FCITX_TOMLPLUSPLUS_INCLUDE_DIR}")
target_link_libraries(fcitx5_config PUBLIC fcitx5_control_core_rust)
add_dependencies(fcitx5_config fcitx5_control_core_rustlib)
target_compile_definitions(fcitx5_config PRIVATE TOML_EXCEPTIONS=1)
fcitx_apply_project_options(fcitx5_config)
if(FCITX_COMPILER_IS_MSVC_CL)
  target_compile_options(fcitx5_config PRIVATE /external:W0 /analyze:external-)
endif()


add_library(
  fcitx5_platform STATIC
  src/platform/runtime_identity.cpp
  src/platform/pipe_security.cpp)
add_library(fcitx5::platform ALIAS fcitx5_platform)
target_compile_features(fcitx5_platform PUBLIC cxx_std_20)
target_include_directories(fcitx5_platform PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/platform")
target_link_libraries(fcitx5_platform PUBLIC fcitx5::release_identity fcitx5::windows_common
                                            advapi32 user32)
fcitx_apply_project_options(fcitx5_platform)

add_library(
  fcitx5_ipc_client STATIC
  src/ipc/launcher_client.cpp
  src/ipc/pipe_client.cpp
  src/ipc/peer_verification.cpp)
add_library(fcitx5::ipc_client ALIAS fcitx5_ipc_client)
target_compile_features(fcitx5_ipc_client PUBLIC cxx_std_20)
target_include_directories(fcitx5_ipc_client PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/ipc")
target_link_libraries(fcitx5_ipc_client PUBLIC fcitx5::protocol fcitx5::platform)
if(FCITX_DEVELOPMENT_PEER_EXCEPTION)
  target_compile_definitions(fcitx5_ipc_client PRIVATE FCITX_DEVELOPMENT_PEER_EXCEPTION=1)
endif()
fcitx_apply_project_options(fcitx5_ipc_client)

set(FCITX_RUST_TSF_SUPPORT_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_tsf_support_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_TSF_SUPPORT_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_RELEASE_DATA_DIRECTORY=${FCITX_RELEASE_DATA_DIRECTORY}"
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-tsf-support-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/tsf-support-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/tsf-support-core/src/lib.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-tsf-support-core staticlib")
add_custom_target(fcitx5_tsf_support_core_rustlib
                  DEPENDS "${FCITX_RUST_TSF_SUPPORT_CORE_STATICLIB}")
add_library(fcitx5_tsf_activation_guard STATIC IMPORTED GLOBAL)
add_library(fcitx5::tsf_activation_guard ALIAS fcitx5_tsf_activation_guard)
set_target_properties(fcitx5_tsf_activation_guard PROPERTIES
                      IMPORTED_LOCATION "${FCITX_RUST_TSF_SUPPORT_CORE_STATICLIB}")
add_dependencies(fcitx5_tsf_activation_guard fcitx5_tsf_support_core_rustlib)
target_link_libraries(fcitx5_tsf_activation_guard INTERFACE ws2_32 userenv ntdll bcrypt advapi32)

add_executable(fcitx5_mock_engine src/engine/mock_engine_main.cpp)
set_target_properties(fcitx5_mock_engine PROPERTIES OUTPUT_NAME "fcitx5-mock-engine")
target_link_libraries(
  fcitx5_mock_engine
  PRIVATE fcitx5::windows_common fcitx5::protocol fcitx5::ipc_client fcitx5::platform)
fcitx_apply_project_options(fcitx5_mock_engine)
fcitx_apply_binary_hardening(fcitx5_mock_engine)

add_executable(fcitx5_control src/control/control_main.cpp)
set_target_properties(fcitx5_control PROPERTIES OUTPUT_NAME "fcitx5-control")
target_link_libraries(
  fcitx5_control
  PRIVATE fcitx5::windows_common fcitx5::config fcitx5::protocol
          fcitx5::ipc_client fcitx5::platform fcitx5::package_core
          fcitx5::tsf_activation_guard
          fcitx5_package_core_rust fcitx5_control_core_rust
          fcitx5_process_execution_core_rust shell32 ole32 ws2_32 userenv ntdll bcrypt
          advapi32)
add_dependencies(
  fcitx5_control fcitx5_package_core_rustlib fcitx5_control_core_rustlib
  fcitx5_process_execution_core_rustlib)
fcitx_apply_project_options(fcitx5_control)
fcitx_apply_binary_hardening(fcitx5_control)

set(FCITX_RUST_CONFIG_SIDE_BY_SIDE_EXE
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5-config-rust.exe")
set(FCITX_CONFIG_APP_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>")
set(FCITX_CONFIG_SHIPPING_EXE "${FCITX_CONFIG_APP_OUTPUT_DIR}/fcitx5-config.exe")
set(FCITX_CONFIG_SIDE_BY_SIDE_EXE "${FCITX_CONFIG_APP_OUTPUT_DIR}/fcitx5-config-rust.exe")
add_custom_target(
  fcitx5_config_rust_side_by_side
  BYPRODUCTS "${FCITX_CONFIG_SIDE_BY_SIDE_EXE}" "${FCITX_CONFIG_SHIPPING_EXE}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-poc --bin
    fcitx5-config-rust --target "${FCITX_RUST_TARGET}"
  COMMAND
    "${CMAKE_COMMAND}" -E make_directory "${FCITX_CONFIG_APP_OUTPUT_DIR}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different
    "${FCITX_RUST_CONFIG_SIDE_BY_SIDE_EXE}"
    "${FCITX_CONFIG_SIDE_BY_SIDE_EXE}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different
    "${FCITX_RUST_CONFIG_SIDE_BY_SIDE_EXE}"
    "${FCITX_CONFIG_SHIPPING_EXE}"
  COMMAND ${CMAKE_COMMAND} -E copy_directory
          "${CMAKE_CURRENT_SOURCE_DIR}/locales"
          "${FCITX_CONFIG_APP_OUTPUT_DIR}/locales"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/config-poc/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/config-poc/src/main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/config-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/config-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/config.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/control-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/control-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/lib.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-config.exe and side-by-side fcitx5-config-rust executable")
add_custom_target(fcitx5_config_app ALL DEPENDS fcitx5_config_rust_side_by_side)

add_library(fcitx5_candidate_layout INTERFACE)
add_library(fcitx5::candidate_layout ALIAS fcitx5_candidate_layout)
target_compile_features(fcitx5_candidate_layout INTERFACE cxx_std_20)
target_link_libraries(fcitx5_candidate_layout INTERFACE fcitx5_candidate_core_rust ws2_32 userenv
                                                     ntdll)
add_dependencies(fcitx5_candidate_layout fcitx5_candidate_core_rustlib)

add_executable(
  fcitx5_ui WIN32
  src/ui/ui_main.cpp
  resources/windows/app.rc)
set_target_properties(fcitx5_ui PROPERTIES OUTPUT_NAME "fcitx5-ui")
  target_link_libraries(
  fcitx5_ui
  PRIVATE fcitx5::candidate_layout fcitx5::config
          fcitx5::windows_common fcitx5::protocol fcitx5::ipc_client fcitx5::platform
          d2d1 dwrite shell32 ole32)
fcitx_apply_project_options(fcitx5_ui)
fcitx_apply_binary_hardening(fcitx5_ui)
add_custom_command(
  TARGET fcitx5_ui POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_directory
          "${CMAKE_CURRENT_SOURCE_DIR}/resources"
          "$<TARGET_FILE_DIR:fcitx5_ui>/resources"
  VERBATIM)

add_custom_target(
  fcitx5_tsf
  ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-tsf.dll"
  COMMAND
    "${CMAKE_COMMAND}" -DFCFW_COPY_FROM=${FCITX_RUST_TSF_POC_DLL}
    -DFCFW_COPY_TO=${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-tsf.dll
    -DATTEMPTS=8 -DDELAY_SECONDS=1 -P
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/copy-with-retry.cmake"
  DEPENDS
    fcitx5_tsf_poc_rustdll
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/windows/tsf.rc"
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/windows/resource.h"
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/icons/fcitx5-tsf.ico"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/tsf-poc/build.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/tsf-poc/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/tsf-poc/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/protocol-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/windows-common-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/windows-common-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/copy-with-retry.cmake"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust shipping fcitx5-tsf.dll")
set(FCITX_SHIPPING_TSF_DLL "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-tsf.dll")

add_custom_target(
  fcitx5_register
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-register.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-register-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>" -Version "${PROJECT_VERSION}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/register-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/register-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/register-core/src/main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-register-cli.ps1"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-register CLI")

add_custom_target(
  fcitx5_bootstrap
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-bootstrap.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-bootstrap-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>" -Version "${PROJECT_VERSION}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/bootstrap_main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-bootstrap-cli.ps1"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-bootstrap CLI")

add_custom_target(
  fcitx5_release_pqc_signer ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-release-pqc-signer.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-release-pqc-signer --bin
    fcitx5-release-pqc-signer --target "${FCITX_RUST_TARGET}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5-release-pqc-signer.exe"
    "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-release-pqc-signer.exe"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/release-pqc-signer/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/release-pqc-signer/build.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/release-pqc-signer/src/main.rs"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust release-only PQC signer")

add_custom_target(
  fcitx5_package ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-package.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-package-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-package CLI")

set(FCITX_RUST_PACKAGE_CORE_STATICLIB
    "${FCITX_CARGO_TARGET_DIR}/${FCITX_RUST_TARGET}/debug/fcitx5_package_core.lib")
add_custom_command(
  OUTPUT "${FCITX_RUST_PACKAGE_CORE_STATICLIB}"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --lib --target
    "${FCITX_RUST_TARGET}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/build.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/c/miniz_archive.c"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-package-core staticlib")
add_custom_target(fcitx5_package_core_rustlib
                  DEPENDS "${FCITX_RUST_PACKAGE_CORE_STATICLIB}")
add_library(fcitx5_package_core_rust STATIC IMPORTED GLOBAL)
set_target_properties(fcitx5_package_core_rust PROPERTIES
                      IMPORTED_LOCATION "${FCITX_RUST_PACKAGE_CORE_STATICLIB}")
add_dependencies(fcitx5_package_core_rust fcitx5_package_core_rustlib)
target_link_libraries(
  fcitx5_package_core
  PUBLIC fcitx5_package_core_rust ole32 advapi32 ws2_32 userenv ntdll)
add_dependencies(fcitx5_package_core fcitx5_package_core_rustlib)

add_custom_target(
  fcitx5_downloader ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-downloader.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-downloader-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>" -Version "${PROJECT_VERSION}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/downloader_main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-downloader-cli.ps1"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-downloader CLI")

add_custom_target(
  fcitx5_deployer ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-deployer.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-deployer-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>" -Version "${PROJECT_VERSION}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/deployer_main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-deployer-cli.ps1"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-deployer CLI")

add_custom_target(
  fcitx5_provider ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-provider.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-provider-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-provider CLI")

add_custom_target(
  fcitx5_updater ALL
  BYPRODUCTS "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-updater.exe"
  COMMAND
    "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
    "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
    "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
    "${FCITX_PWSH_COMMAND}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-updater-cli.ps1"
    -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
    -OutputDirectory "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>" -Version "${PROJECT_VERSION}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/lib.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/package-core/src/updater_main.rs"
    "${CMAKE_CURRENT_SOURCE_DIR}/tools/build-rust-updater-cli.ps1"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  COMMENT "Building Rust fcitx5-updater CLI")

if(BUILD_TESTING)
  add_executable(fcitx5_version_test tests/unit/version_test.cpp)
  target_link_libraries(fcitx5_version_test PRIVATE fcitx5::windows_common)
  fcitx_apply_project_options(fcitx5_version_test)
  fcitx_apply_binary_hardening(fcitx5_version_test)
  add_test(NAME version-contract COMMAND fcitx5_version_test)

  add_executable(fcitx5_release_identity_test tests/unit/release_identity_test.cpp)
  target_link_libraries(
    fcitx5_release_identity_test PRIVATE fcitx5::windows_common fcitx5::release_identity ole32)
  fcitx_apply_project_options(fcitx5_release_identity_test)
  fcitx_apply_binary_hardening(fcitx5_release_identity_test)
  add_test(NAME release-identity-contract COMMAND fcitx5_release_identity_test)

  add_executable(fcitx5_brand_resource_test tests/unit/brand_resource_test.cpp)
  fcitx_apply_project_options(fcitx5_brand_resource_test)
  fcitx_apply_binary_hardening(fcitx5_brand_resource_test)
  add_test(NAME brand-resource-contract
           COMMAND fcitx5_brand_resource_test "${CMAKE_CURRENT_SOURCE_DIR}")

  add_test(NAME release-plugin-source-contract
           COMMAND "${FCITX_PWSH_EXECUTABLE}"
                   -NoProfile -File
                   "${CMAKE_CURRENT_SOURCE_DIR}/tools/test-release-plugin-artifacts.ps1")

  add_test(NAME signing-credential-policy
           COMMAND "${FCITX_PWSH_EXECUTABLE}"
                   -NoProfile -File
                   "${CMAKE_CURRENT_SOURCE_DIR}/tools/test-signing-credential-policy.ps1")

  add_library(
    fcitx5_mldsa65_test_sign STATIC
    "${FCITX_MLDSA_NATIVE_SOURCE_DIR}/mldsa/mldsa_native.c")
  target_include_directories(
    fcitx5_mldsa65_test_sign
    SYSTEM PUBLIC "${FCITX_MLDSA_NATIVE_SOURCE_DIR}")
  target_include_directories(
    fcitx5_mldsa65_test_sign
    PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_compile_definitions(
    fcitx5_mldsa65_test_sign
    PRIVATE MLD_CONFIG_FILE=\"fcitx5_mldsa65_test_config.h\")
  if(MSVC)
    target_compile_options(fcitx5_mldsa65_test_sign PRIVATE /W3)
    if(FCITX_COMPILER_IS_MSVC_CL)
      target_compile_options(fcitx5_mldsa65_test_sign PRIVATE /sdl)
    endif()
  endif()

  add_executable(fcitx5_pqc_fixture_signer tests/support/pqc_fixture_signer.cpp)
  set_target_properties(fcitx5_pqc_fixture_signer PROPERTIES
                        OUTPUT_NAME "fcitx5-pqc-fixture-signer")
  target_include_directories(
    fcitx5_pqc_fixture_signer SYSTEM PRIVATE "${FCITX_MLDSA_NATIVE_SOURCE_DIR}")
  target_include_directories(
    fcitx5_pqc_fixture_signer PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_link_libraries(fcitx5_pqc_fixture_signer PRIVATE fcitx5_mldsa65_test_sign crypt32)
  fcitx_apply_project_options(fcitx5_pqc_fixture_signer)
  fcitx_apply_binary_hardening(fcitx5_pqc_fixture_signer)

  add_executable(
    fcitx5_control_package_integration_test
    tests/integration/control_package_integration_test.cpp)
  target_include_directories(
    fcitx5_control_package_integration_test SYSTEM PRIVATE "${FCITX_MINIZ_SOURCE_DIR}")
  target_link_libraries(
    fcitx5_control_package_integration_test
    PRIVATE fcitx5::package_core fcitx5_miniz bcrypt crypt32)
  add_dependencies(
    fcitx5_control_package_integration_test fcitx5_control fcitx5_downloader
    fcitx5_pqc_fixture_signer)
  fcitx_apply_project_options(fcitx5_control_package_integration_test)
  fcitx_apply_binary_hardening(fcitx5_control_package_integration_test)
  add_test(
    NAME control-package-stopped-service-contract
    COMMAND fcitx5_control_package_integration_test
            "$<TARGET_FILE:fcitx5_control>"
            "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-downloader.exe"
            "$<TARGET_FILE:fcitx5_pqc_fixture_signer>")
  set_tests_properties(control-package-stopped-service-contract PROPERTIES TIMEOUT 30)
  add_test(NAME launcher-version COMMAND "${FCITX_LAUNCHER_SHIPPING_EXE}" --version)
  add_test(NAME mock-engine-version COMMAND fcitx5_mock_engine --version)
  add_test(NAME control-version COMMAND fcitx5_control --version)
  add_test(NAME control-schema COMMAND fcitx5_control --schema)
  add_test(NAME control-startup-query COMMAND fcitx5_control --get-startup)
  add_test(
    NAME register-version
    COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-register.exe" --version)
  add_executable(fcitx5_register_artifact_test tests/unit/register_artifact_test.cpp)
  add_dependencies(fcitx5_register_artifact_test fcitx5_register)
  fcitx_apply_project_options(fcitx5_register_artifact_test)
  fcitx_apply_binary_hardening(fcitx5_register_artifact_test)
  add_test(NAME register-artifact-validation
           COMMAND fcitx5_register_artifact_test
                   "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-register.exe")
  add_test(NAME package-version
           COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-package.exe" --version)
  add_test(
    NAME updater-version
    COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-updater.exe" --version)
  add_test(
    NAME downloader-version
    COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-downloader.exe" --version)
  add_test(
    NAME downloader-boundary-smoke
    COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-downloader.exe" --self-test)
  add_test(
    NAME deployer-version
    COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-deployer.exe" --version)
  add_test(
    NAME deployer-boundary-smoke
    COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-deployer.exe" --self-test)
  add_test(NAME provider-version
           COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-provider.exe" --version)
  add_test(NAME provider-boundary-smoke
           COMMAND "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/fcitx5-provider.exe" --self-test
                   --audit-self-pe)

  add_executable(fcitx5_control_repository_rollback_test
                 tests/unit/control_repository_rollback_test.cpp)
  target_include_directories(
    fcitx5_control_repository_rollback_test SYSTEM PRIVATE
    "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}" "${FCITX_MLDSA_NATIVE_SOURCE_DIR}")
  target_include_directories(
    fcitx5_control_repository_rollback_test PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_link_libraries(
    fcitx5_control_repository_rollback_test PRIVATE fcitx5::package_core
            fcitx5_mldsa65_test_sign crypt32)
  add_dependencies(fcitx5_control_repository_rollback_test fcitx5_control)
  fcitx_apply_project_options(fcitx5_control_repository_rollback_test)
  fcitx_apply_binary_hardening(fcitx5_control_repository_rollback_test)
  add_test(NAME control-repository-rollback
           COMMAND fcitx5_control_repository_rollback_test "$<TARGET_FILE:fcitx5_control>")

  add_test(NAME control-config-roundtrip
           COMMAND fcitx5_control --data-root
                   "${CMAKE_CURRENT_BINARY_DIR}/control-test-data"
                   --apply-config "${CMAKE_CURRENT_SOURCE_DIR}/resources/config.toml")
  add_test(NAME config-ui-headless-smoke COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --self-test)
  add_test(NAME config-ui-i18n-check COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --check-i18n)
  add_test(NAME config-ui-resource-check COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --check-resources)
  add_test(NAME config-ui-behavior-contract COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --ui-contract-test)
  add_test(NAME config-ui-visual-contract
           COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --ui-visual-contract-test)
  add_test(NAME config-ui-live-preview-contract
           COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --ui-live-preview-contract-test)
  add_test(NAME config-ui-interaction-coverage
           COMMAND "${FCITX_CONFIG_SHIPPING_EXE}" --ui-interaction-test)
  add_test(
    NAME config-ui-preview-fidelity-qa
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-qa --target
      "${FCITX_RUST_TARGET}" -- --config-exe "${FCITX_CONFIG_SHIPPING_EXE}"
      --candidate-ui-exe "$<TARGET_FILE:fcitx5_ui>" --out
      "${CMAKE_CURRENT_BINARY_DIR}/config-ui-preview-fidelity-qa")
  set_tests_properties(config-ui-preview-fidelity-qa PROPERTIES TIMEOUT 60)
  add_test(
    NAME rust-config-ui-preview-qa
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-qa --target
      "${FCITX_RUST_TARGET}" -- --config-exe
      "${FCITX_CONFIG_SHIPPING_EXE}"
      --candidate-ui-exe "$<TARGET_FILE:fcitx5_ui>" --out
      "${CMAKE_CURRENT_BINARY_DIR}/rust-config-ui-preview-qa")
  set_tests_properties(rust-config-ui-preview-qa PROPERTIES TIMEOUT 60)
  add_test(
    NAME rust-config-poc-contract
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-poc --bin
      fcitx5-config-poc --target "${FCITX_RUST_TARGET}" -- --self-check --report
      "${CMAKE_CURRENT_BINARY_DIR}/config-poc-report.json")
  set_tests_properties(rust-config-poc-contract PROPERTIES
                       WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
  add_test(
    NAME rust-config-core-transaction-contract
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-core --test
      transaction_contract --target "${FCITX_RUST_TARGET}")
  set_tests_properties(rust-config-core-transaction-contract PROPERTIES
                       WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
  add_test(
    NAME rust-config-core-cli-contract
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-poc --test
      config_core_cli_contract --target "${FCITX_RUST_TARGET}")
  set_tests_properties(rust-config-core-cli-contract PROPERTIES
                       WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
  add_test(
    NAME rust-config-side-by-side-contract
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-poc --bin
      fcitx5-config-rust --target "${FCITX_RUST_TARGET}" -- --self-check
      --report
      "${CMAKE_CURRENT_BINARY_DIR}/config-rust-side-by-side-report.json")
  set_tests_properties(rust-config-side-by-side-contract
                       PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
  add_test(
    NAME config-rust-shipping-lineage
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_PWSH_EXECUTABLE}" -NoLogo -NoProfile -ExecutionPolicy Bypass
      -File "${CMAKE_CURRENT_SOURCE_DIR}/tools/test-config-rust-shipping-lineage.ps1"
      -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget
      "${FCITX_RUST_TARGET}" -ShippingConfigExe "${FCITX_CONFIG_SHIPPING_EXE}" -OutputDirectory
      "${CMAKE_CURRENT_BINARY_DIR}/config-rust-shipping-lineage" -SideBySideConfigExe
      "${FCITX_CONFIG_SIDE_BY_SIDE_EXE}" -Report
      "${CMAKE_CURRENT_BINARY_DIR}/config-rust-shipping-lineage.json")
  set_tests_properties(config-rust-shipping-lineage
                       PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
                                  TIMEOUT 45)
  add_test(
    NAME config-rust-legacy-headless-cli
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_PWSH_EXECUTABLE}" -NoLogo -NoProfile -ExecutionPolicy Bypass
      -File "${CMAKE_CURRENT_SOURCE_DIR}/tools/test-config-rust-legacy-cli.ps1"
      -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget
      "${FCITX_RUST_TARGET}" -Report
      "${CMAKE_CURRENT_BINARY_DIR}/config-rust-legacy-headless-cli.json")
  set_tests_properties(config-rust-legacy-headless-cli
                       PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
                                  TIMEOUT 45)
  add_test(
    NAME rust-config-poc-window-smoke
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-config-poc --bin
      fcitx5-config-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke --report
      "${CMAKE_CURRENT_BINARY_DIR}/config-poc-window-smoke.json")
  set_tests_properties(rust-config-poc-window-smoke PROPERTIES
                       WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")

  add_executable(fcitx5_engine_core_contract_test tests/unit/engine_core_contract_test.cpp)
  target_include_directories(fcitx5_engine_core_contract_test
                             PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/engine")
  target_link_libraries(fcitx5_engine_core_contract_test PRIVATE fcitx5_engine_core_rust)
  fcitx_apply_project_options(fcitx5_engine_core_contract_test)
  fcitx_apply_binary_hardening(fcitx5_engine_core_contract_test)
  add_test(NAME engine-core-contract COMMAND fcitx5_engine_core_contract_test)

  find_program(FCITX_CARGO_EXECUTABLE cargo)
  if(FCITX_CARGO_EXECUTABLE)
    find_program(FCITX_PWSH_EXECUTABLE pwsh powershell)
    add_custom_target(
      fcitx5_rust_package_core_test
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --lib --target
        "${FCITX_RUST_TARGET}"
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
      DEPENDS fcitx5_pqc_fixture_signer
      COMMENT "Running Rust package-core differential corpus")
    add_custom_target(
      fcitx5_rust_package_core_artifact_smoke
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --bin
        fcitx5-package-core --target "${FCITX_RUST_TARGET}" -- --self-check --audit-self-pe --trusted-keys
        "${CMAKE_CURRENT_SOURCE_DIR}/security/trusted-keys.template.json"
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
      COMMENT "Running Rust package-core artifact smoke")
    add_test(
      NAME rust-package-core-differential
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --lib --target
        "${FCITX_RUST_TARGET}")
    set_tests_properties(
      rust-package-core-differential PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-package-core-fuzz-smoke
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --lib --target
        "${FCITX_RUST_TARGET}" fuzz_smoke_consumes_deterministic_random_package_bytes -- --exact)
    set_tests_properties(
      rust-package-core-fuzz-smoke PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-package-core-path-corpus-fuzz-smoke
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --lib --target
        "${FCITX_RUST_TARGET}" package_path_corpus_matches_frozen_cpp_policy -- --exact)
    set_tests_properties(
      rust-package-core-path-corpus-fuzz-smoke PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-package-core-updater-cli-contract
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --test
        updater_cli_contract --target "${FCITX_RUST_TARGET}")
    set_tests_properties(
      rust-package-core-updater-cli-contract PROPERTIES
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-package-core-artifact-smoke
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
        "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-package-core --bin
        fcitx5-package-core --target "${FCITX_RUST_TARGET}" -- --self-check --audit-self-pe --trusted-keys
        "${CMAKE_CURRENT_SOURCE_DIR}/security/trusted-keys.template.json")
    set_tests_properties(
      rust-package-core-artifact-smoke PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_custom_target(
      fcitx5_rust_candidate_poc_contract
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --self-check --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-report.json"
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
      COMMENT "Running Rust Candidate out-of-process PoC contract")
    add_test(
      NAME rust-candidate-poc-contract
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --self-check --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-report.json")
    set_tests_properties(
      rust-candidate-poc-contract PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-candidate-poc-window-smoke
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-window-smoke.json" --screenshot
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-window-smoke.bmp")
    set_tests_properties(
      rust-candidate-poc-window-smoke PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-candidate-poc-demo-snapshot
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke
        --demo-snapshot --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-demo-snapshot.json" --screenshot
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-demo-snapshot.bmp")
    set_tests_properties(
      rust-candidate-poc-demo-snapshot PROPERTIES
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    add_test(
      NAME rust-candidate-poc-scroll-demo-snapshot
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke
        --scroll-demo-snapshot --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-scroll-demo-snapshot.json" --screenshot
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-scroll-demo-snapshot.bmp")
    set_tests_properties(
      rust-candidate-poc-scroll-demo-snapshot PROPERTIES
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    foreach(FCITX_CANDIDATE_LABEL_SLOT_SNAPSHOT vertical horizontal grid vertical-dark horizontal-dark grid-dark)
      add_test(
        NAME "rust-candidate-poc-label-slot-${FCITX_CANDIDATE_LABEL_SLOT_SNAPSHOT}"
        COMMAND
          "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
          "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
          "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
          fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke
          --label-slot-snapshot "${FCITX_CANDIDATE_LABEL_SLOT_SNAPSHOT}" --dpi-scale 1.50 --report
          "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-label-slot-${FCITX_CANDIDATE_LABEL_SLOT_SNAPSHOT}.json"
          --screenshot
          "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-label-slot-${FCITX_CANDIDATE_LABEL_SLOT_SNAPSHOT}.bmp")
      set_tests_properties(
        "rust-candidate-poc-label-slot-${FCITX_CANDIDATE_LABEL_SLOT_SNAPSHOT}" PROPERTIES
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    endforeach()
    add_test(
      NAME rust-candidate-poc-typography-vertical
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke
        --typography-snapshot --dpi-scale 1.50 --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-typography-vertical.json" --screenshot
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-typography-vertical.bmp")
    set_tests_properties(
      rust-candidate-poc-typography-vertical PROPERTIES
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    foreach(FCITX_CANDIDATE_HOST_SNAPSHOT mock-notepad mock-word mock-chromium mock-emoji-host)
      add_test(
        NAME "rust-candidate-poc-host-snapshot-${FCITX_CANDIDATE_HOST_SNAPSHOT}"
        COMMAND
          "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
          "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
          "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
          fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke
          --host-snapshot "${FCITX_CANDIDATE_HOST_SNAPSHOT}" --report
          "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-host-${FCITX_CANDIDATE_HOST_SNAPSHOT}.json"
          --screenshot
          "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-host-${FCITX_CANDIDATE_HOST_SNAPSHOT}.bmp")
      set_tests_properties(
        "rust-candidate-poc-host-snapshot-${FCITX_CANDIDATE_HOST_SNAPSHOT}" PROPERTIES
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    endforeach()
    add_test(
      NAME rust-candidate-poc-dpi-smoke
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-candidate-core --bin
        fcitx5-candidate-poc --target "${FCITX_RUST_TARGET}" -- --window-smoke
        --dpi-scale 1.50 --report
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-dpi-smoke.json" --screenshot
        "${CMAKE_CURRENT_BINARY_DIR}/candidate-poc-dpi-smoke.bmp")
    set_tests_properties(
      rust-candidate-poc-dpi-smoke PROPERTIES
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    if(FCITX_PWSH_EXECUTABLE)
      add_custom_target(
        fcitx5_rust_package_core_packaged_artifact_smoke
        COMMAND
          "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
          "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
          "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
          "${FCITX_PWSH_EXECUTABLE}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
          "${CMAKE_CURRENT_SOURCE_DIR}/tools/test-rust-package-core-artifact.ps1"
          -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}"
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
        COMMENT "Running Rust package-core packaged artifact smoke")
      add_test(
        NAME rust-package-core-packaged-artifact-smoke
        COMMAND
          "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
          "FCITX_MLDSA_NATIVE_SOURCE_DIR=${FCITX_MLDSA_NATIVE_SOURCE_DIR}"
          "FCITX_MINIZ_SOURCE_DIR=${FCITX_MINIZ_SOURCE_DIR}"
          "${FCITX_PWSH_EXECUTABLE}" -NoLogo -NoProfile -ExecutionPolicy Bypass -File
          "${CMAKE_CURRENT_SOURCE_DIR}/tools/test-rust-package-core-artifact.ps1"
          -CargoExecutable "${FCITX_CARGO_EXECUTABLE}" -CargoTarget "${FCITX_RUST_TARGET}")
      set_tests_properties(
        rust-package-core-packaged-artifact-smoke PROPERTIES
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    endif()
    add_test(
      NAME rust-protocol-core-contract
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-protocol-core --lib --target
        "${FCITX_RUST_TARGET}")
    set_tests_properties(
      rust-protocol-core-contract PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    if(FCITX_BUILD_FUZZERS)
      add_test(
        NAME rust-protocol-fuzz-smoke
        COMMAND
          "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
          "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
          "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-protocol-core --lib --target
          "${FCITX_RUST_TARGET}" fuzz_smoke_consumes_deterministic_random_bytes -- --exact)
      set_tests_properties(
        rust-protocol-fuzz-smoke PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    endif()
  else()
    message(STATUS "Cargo was not found; Rust R1 package-core tests are not registered.")
  endif()


  add_test(NAME candidate-ui-device-smoke
           COMMAND fcitx5_ui --self-test --simulate-device-loss)
  add_test(NAME candidate-ui-safe-mode-smoke COMMAND fcitx5_ui --self-test --safe-mode)
  add_test(NAME candidate-ui-interaction-contract
           COMMAND fcitx5_ui --interaction-self-test)
  set_tests_properties(candidate-ui-interaction-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-uiless-presentation-contract
           COMMAND fcitx5_ui --uiless-presentation-self-test)
  set_tests_properties(candidate-ui-uiless-presentation-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-scroll-expansion-contract
           COMMAND fcitx5_ui --scroll-expansion-self-test)
  set_tests_properties(candidate-ui-scroll-expansion-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-locale-contract
           COMMAND fcitx5_ui --locale-self-test --safe-mode)
  set_tests_properties(candidate-ui-locale-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-ux-contract
           COMMAND fcitx5_ui --candidate-ux-self-test --safe-mode)
  set_tests_properties(candidate-ui-ux-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-live-config-reflow
           COMMAND fcitx5_ui --self-test --reload-test)

  add_executable(
    fcitx5_candidate_ui_config_integration_test
    tests/integration/candidate_ui_config_integration_test.cpp)
  add_dependencies(
    fcitx5_candidate_ui_config_integration_test fcitx5_ui fcitx5_control
    fcitx5_candidate_poc_rustbin)
  target_link_libraries(
    fcitx5_candidate_ui_config_integration_test PRIVATE fcitx5::config user32)
  fcitx_apply_project_options(fcitx5_candidate_ui_config_integration_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_ui_config_integration_test)
  add_test(
    NAME candidate-ui-live-presentation-contract
    COMMAND fcitx5_candidate_ui_config_integration_test
            "$<TARGET_FILE:fcitx5_ui>" "$<TARGET_FILE:fcitx5_control>"
            "$<TARGET_FILE_DIR:fcitx5_ui>/resources" "${FCITX_RUST_CANDIDATE_POC_EXE}")
  set_tests_properties(candidate-ui-live-presentation-contract PROPERTIES TIMEOUT 30)

  add_executable(fcitx5_runtime_identity_test tests/unit/runtime_identity_test.cpp)
  target_link_libraries(fcitx5_runtime_identity_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_runtime_identity_test)
  fcitx_apply_binary_hardening(fcitx5_runtime_identity_test)
  add_test(NAME runtime-identity-contract COMMAND fcitx5_runtime_identity_test)

  add_executable(fcitx5_ipc_roundtrip_test tests/integration/ipc_roundtrip_test.cpp)
  target_link_libraries(fcitx5_ipc_roundtrip_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_roundtrip_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_roundtrip_test)
  add_test(NAME ipc-key-commit-roundtrip
           COMMAND fcitx5_ipc_roundtrip_test $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_ipc_multi_client_test tests/integration/ipc_multi_client_test.cpp)
  target_link_libraries(fcitx5_ipc_multi_client_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_multi_client_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_multi_client_test)
  add_test(NAME ipc-multi-client
           COMMAND fcitx5_ipc_multi_client_test $<TARGET_FILE:fcitx5_mock_engine>)
  add_test(NAME ipc-multi-client-32
           COMMAND fcitx5_ipc_multi_client_test $<TARGET_FILE:fcitx5_mock_engine> 32)
  set_tests_properties(ipc-multi-client-32 PROPERTIES TIMEOUT 30)

  add_executable(fcitx5_ipc_idle_client_test tests/integration/ipc_idle_client_test.cpp)
  target_link_libraries(fcitx5_ipc_idle_client_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_idle_client_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_idle_client_test)
  add_test(NAME ipc-idle-client
           COMMAND fcitx5_ipc_idle_client_test $<TARGET_FILE:fcitx5_mock_engine>)
  set_tests_properties(ipc-idle-client PROPERTIES TIMEOUT 30)

  add_executable(fcitx5_ipc_generation_routing_test
                 tests/integration/ipc_generation_routing_test.cpp)
  target_link_libraries(fcitx5_ipc_generation_routing_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_generation_routing_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_generation_routing_test)
  add_test(NAME ipc-generation-routing
           COMMAND fcitx5_ipc_generation_routing_test $<TARGET_FILE:fcitx5_mock_engine>)
  set_tests_properties(ipc-generation-routing PROPERTIES TIMEOUT 30)

  add_executable(fcitx5_ipc_late_response_test
                 tests/integration/ipc_late_response_test.cpp)
  target_link_libraries(fcitx5_ipc_late_response_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_late_response_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_late_response_test)
  add_test(NAME ipc-late-response-reconnect
           COMMAND fcitx5_ipc_late_response_test $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_launcher_integration_test tests/integration/launcher_integration_test.cpp)
  target_link_libraries(fcitx5_launcher_integration_test PRIVATE fcitx5::ipc_client)
  add_dependencies(fcitx5_launcher_integration_test fcitx5_launcher)
  fcitx_apply_project_options(fcitx5_launcher_integration_test)
  fcitx_apply_binary_hardening(fcitx5_launcher_integration_test)
  add_test(NAME launcher-engine-lifecycle
           COMMAND fcitx5_launcher_integration_test
                   "${FCITX_LAUNCHER_SHIPPING_EXE}"
                   $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_crash_engine_fixture tests/fixtures/crashing_engine_main.cpp)
  set_target_properties(
    fcitx5_crash_engine_fixture PROPERTIES OUTPUT_NAME "fcitx5-crash-engine-fixture")
  fcitx_apply_project_options(fcitx5_crash_engine_fixture)
  fcitx_apply_binary_hardening(fcitx5_crash_engine_fixture)
  add_executable(fcitx5_launcher_crash_loop_test
                 tests/integration/launcher_crash_loop_test.cpp)
  target_link_libraries(fcitx5_launcher_crash_loop_test PRIVATE fcitx5::ipc_client)
  add_dependencies(fcitx5_launcher_crash_loop_test fcitx5_launcher)
  fcitx_apply_project_options(fcitx5_launcher_crash_loop_test)
  fcitx_apply_binary_hardening(fcitx5_launcher_crash_loop_test)
  add_test(NAME launcher-crash-loop-safe-mode
           COMMAND fcitx5_launcher_crash_loop_test
                   "${FCITX_LAUNCHER_SHIPPING_EXE}"
                   $<TARGET_FILE:fcitx5_crash_engine_fixture>)

  add_executable(fcitx5_tsf_module_test tests/unit/tsf_module_test.cpp)
  add_dependencies(fcitx5_tsf_module_test fcitx5_tsf)
  target_include_directories(fcitx5_tsf_module_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_link_libraries(fcitx5_tsf_module_test PRIVATE fcitx5::release_identity uuid ole32)
  fcitx_apply_project_options(fcitx5_tsf_module_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_module_test)
  add_test(NAME tsf-module-activation
           COMMAND fcitx5_tsf_module_test "${FCITX_SHIPPING_TSF_DLL}")

  add_executable(fcitx5_rust_tsf_poc_export_smoke
                 tests/unit/rust_tsf_poc_export_smoke.cpp)
  add_dependencies(fcitx5_rust_tsf_poc_export_smoke fcitx5_tsf_poc_rustdll)
  fcitx_apply_project_options(fcitx5_rust_tsf_poc_export_smoke)
  fcitx_apply_binary_hardening(fcitx5_rust_tsf_poc_export_smoke)
  add_test(NAME rust-tsf-poc-unit
           COMMAND
             "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
             "${FCITX_CARGO_EXECUTABLE}" test --locked --manifest-path
             "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-tsf-poc --target
             "${FCITX_RUST_TARGET}")
  set_tests_properties(rust-tsf-poc-unit PROPERTIES
                       WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
  add_test(NAME rust-tsf-poc-export-smoke
           COMMAND fcitx5_rust_tsf_poc_export_smoke "${FCITX_RUST_TSF_POC_DLL}")

  add_executable(fcitx5_rust_tsf_poc_artifact_audit
                 tests/unit/rust_tsf_poc_artifact_audit.cpp)
  add_dependencies(fcitx5_rust_tsf_poc_artifact_audit fcitx5_tsf_poc_rustdll)
  fcitx_apply_project_options(fcitx5_rust_tsf_poc_artifact_audit)
  fcitx_apply_binary_hardening(fcitx5_rust_tsf_poc_artifact_audit)
  add_test(NAME rust-tsf-poc-artifact-audit
           COMMAND fcitx5_rust_tsf_poc_artifact_audit "${FCITX_RUST_TSF_POC_DLL}")

  add_executable(fcitx5_tsf_key_commit_test tests/integration/tsf_key_commit_test.cpp)
  add_dependencies(fcitx5_tsf_key_commit_test fcitx5_tsf)
  target_include_directories(fcitx5_tsf_key_commit_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_link_libraries(fcitx5_tsf_key_commit_test PRIVATE fcitx5::release_identity uuid ole32 oleaut32)
  fcitx_apply_project_options(fcitx5_tsf_key_commit_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_key_commit_test)
  add_test(NAME tsf-key-commit-e2e
           COMMAND fcitx5_tsf_key_commit_test
                  "${FCITX_SHIPPING_TSF_DLL}"
                   $<TARGET_FILE:fcitx5_mock_engine>
                   "${CMAKE_CURRENT_SOURCE_DIR}/tests/fixtures/tsf_behavior_corpus.json")

  # Interactive desktop E2E. It is built with the test suite but intentionally
  # not registered with CTest because it requires installed TSF registration
  # and a foreground user desktop with Notepad.
  add_executable(fcitx5_tsf_notepad_e2e_test
                 tests/integration/tsf_notepad_e2e_test.cpp)
  add_dependencies(fcitx5_tsf_notepad_e2e_test fcitx5_tsf)
  target_include_directories(fcitx5_tsf_notepad_e2e_test
                             PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_link_libraries(fcitx5_tsf_notepad_e2e_test
                        PRIVATE fcitx5::release_identity ole32 user32)
  fcitx_apply_project_options(fcitx5_tsf_notepad_e2e_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_notepad_e2e_test)

  add_executable(fcitx5_engine_integration_test
                 tests/integration/fcitx_engine_integration_test.cpp)
  target_link_libraries(fcitx5_engine_integration_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_engine_integration_test)
  fcitx_apply_binary_hardening(fcitx5_engine_integration_test)
endif()

if(FCITX_BUILD_BENCHMARKS)
  add_executable(fcitx5_handle_leak_soak tests/perf/handle_leak_soak.cpp)
  target_include_directories(
    fcitx5_handle_leak_soak PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/tests/support")
  target_link_libraries(fcitx5_handle_leak_soak PRIVATE fcitx5::release_identity uuid ole32 user32)
  fcitx_apply_project_options(fcitx5_handle_leak_soak)
  fcitx_apply_binary_hardening(fcitx5_handle_leak_soak)

  add_custom_target(
    fcitx5_rust_ipc_codec_bench
    COMMAND
      "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
      "${FCITX_CARGO_EXECUTABLE}" build --locked --manifest-path
      "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-protocol-core --bin
      fcitx5-protocol-bench --target "${FCITX_RUST_TARGET}"
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Building Rust fcitx5-protocol-bench")
  if(BUILD_TESTING)
    add_test(
      NAME rust-ipc-codec-bench
      COMMAND
        "${CMAKE_COMMAND}" -E env ${FCITX_RUST_BUILD_ENV}
        "${FCITX_CARGO_EXECUTABLE}" run --locked --manifest-path
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p fcitx5-protocol-core --bin
        fcitx5-protocol-bench --target "${FCITX_RUST_TARGET}")
    set_tests_properties(rust-ipc-codec-bench PROPERTIES
                         WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
  endif()

  add_executable(fcitx5_key_roundtrip_bench tests/perf/key_roundtrip_bench.cpp)
  target_link_libraries(fcitx5_key_roundtrip_bench PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_key_roundtrip_bench)
  fcitx_apply_binary_hardening(fcitx5_key_roundtrip_bench)
endif()
