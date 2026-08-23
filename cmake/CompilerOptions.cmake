function(fcitx_apply_project_options target)
  target_compile_features(${target} PRIVATE cxx_std_20)
  target_compile_definitions(
    ${target}
    PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0601)

  set(FCITX_TARGET_USES_PCH ON)
  if("${target}" MATCHES "(_test|_bench|_fuzz|fixture|mock)")
    set(FCITX_TARGET_USES_PCH OFF)
  endif()
  if(FCITX_ENABLE_PCH AND FCITX_TARGET_USES_PCH AND
     DEFINED FCITX_PROJECT_PCH_HEADER AND EXISTS "${FCITX_PROJECT_PCH_HEADER}")
    target_precompile_headers(
      ${target} PRIVATE "$<$<COMPILE_LANGUAGE:CXX>:${FCITX_PROJECT_PCH_HEADER}>")
  endif()

  if(MSVC)
    target_compile_options(
      ${target}
      PRIVATE /W4 /permissive- /Zc:__cplusplus /utf-8 /EHsc)
    if(FCITX_COMPILER_IS_MSVC_CL)
      target_compile_options(${target} PRIVATE /sdl)
    endif()

    if(FCITX_WARNINGS_AS_ERRORS)
      target_compile_options(${target} PRIVATE /WX)
    endif()

    if(FCITX_ENABLE_MSVC_ANALYZE AND FCITX_COMPILER_IS_MSVC_CL)
      target_compile_options(${target} PRIVATE /analyze)
    endif()

    if(FCITX_COMPILER_IS_CLANG_CL)
      target_compile_options(
        ${target} PRIVATE -Wno-return-type-c-linkage -Wno-missing-field-initializers)
    endif()

  else()
    target_compile_options(
      ${target}
      PRIVATE -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion)
    if(FCITX_WARNINGS_AS_ERRORS)
      target_compile_options(${target} PRIVATE -Werror)
    endif()
  endif()
endfunction()

function(fcitx_apply_binary_hardening target)
  if(MSVC)
    target_compile_options(${target} PRIVATE /guard:cf)
    target_link_options(
      ${target}
      PRIVATE /DYNAMICBASE /NXCOMPAT /guard:cf)
    if(FCITX_COMPILER_IS_CLANG_CL)
      target_link_options(${target} PRIVATE "$<$<CONFIG:Debug,RelWithDebInfo>:/DEBUG:GHASH>")
    endif()
    # /CETCOMPAT (CET shadow stack) is rejected by the ARM64 linker with
    # LNK1246. Prefer the explicit FCITX_TARGET_ARCH used by the default
    # Ninja/clang-cl presets, and keep the Visual Studio platform fallback for
    # compatibility presets.
    if(NOT FCITX_EFFECTIVE_TARGET_ARCH STREQUAL "arm64" AND
       NOT CMAKE_VS_PLATFORM_NAME MATCHES "ARM64|ARM64EC")
      target_link_options(${target} PRIVATE /CETCOMPAT)
    endif()
  endif()
endfunction()
