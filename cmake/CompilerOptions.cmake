function(fcitx_apply_project_options target)
  target_compile_features(${target} PRIVATE cxx_std_20)
  target_compile_definitions(
    ${target}
    PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0601)

  if(MSVC)
    target_compile_options(
      ${target}
      PRIVATE /W4 /permissive- /sdl /Zc:__cplusplus /utf-8 /EHsc)

    if(FCITX_WARNINGS_AS_ERRORS)
      target_compile_options(${target} PRIVATE /WX)
    endif()

    if(FCITX_ENABLE_MSVC_ANALYZE)
      target_compile_options(${target} PRIVATE /analyze)
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
    # /CETCOMPAT (CET shadow stack) is rejected by the ARM64 linker with
    # LNK1246. The Visual Studio generator reports the target platform through
    # CMAKE_VS_PLATFORM_NAME ("x64", "Win32", "ARM64", "ARM64EC"), which is
    # reliable at configure time, unlike CMAKE_SYSTEM_PROCESSOR.
    if(NOT CMAKE_VS_PLATFORM_NAME MATCHES "ARM64|ARM64EC")
      target_link_options(${target} PRIVATE /CETCOMPAT)
    endif()
  endif()
endfunction()
