function(fcitx_apply_project_options target)
  target_compile_definitions(
    ${target}
    PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)

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

function(fcitx_apply_executable_hardening target)
  if(MSVC)
    target_compile_options(${target} PRIVATE /guard:cf)
    target_link_options(
      ${target}
      PRIVATE /DYNAMICBASE /NXCOMPAT /guard:cf /CETCOMPAT)
  endif()
endfunction()
