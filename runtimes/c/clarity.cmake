# The runtime's libraries, for the runtime's own build and for generated
# projects, which include this file: `clarity` (records, storage, the
# binding to the screen, with cJSON), `clarity_service` (the commands of the
# web wire, and the web server) and `clarity_desktop` (the windows, drawn
# by Clarity.UI's library).
find_package(SQLite3 REQUIRED)

file(GLOB CLARITY_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/clarity/*.c)
add_library(clarity STATIC ${CLARITY_SOURCES} ${CMAKE_CURRENT_LIST_DIR}/third_party/cjson/cJSON.c)
target_include_directories(clarity PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
target_link_libraries(clarity PUBLIC SQLite::SQLite3 m)
if(MINGW)
  # The C99 printf, so %zu and %lld print as they do elsewhere.
  target_compile_definitions(clarity PUBLIC __USE_MINGW_ANSI_STDIO=1)
endif()
if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(clarity PRIVATE $<$<COMPILE_LANGUAGE:C>:-Wall -Wextra>)
endif()

# The service answers Clarity.UI's commands, and serves the browser files
# to a browser (the web target): only in a project whose runtime copy has
# it, so a command-line program builds without it.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/service/service.c)
  add_library(clarity_service STATIC ${CMAKE_CURRENT_LIST_DIR}/service/service.c ${CMAKE_CURRENT_LIST_DIR}/service/wire.c ${CMAKE_CURRENT_LIST_DIR}/service/watches.c ${CMAKE_CURRENT_LIST_DIR}/service/serve.c)
  target_include_directories(clarity_service PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_service PUBLIC clarity)
  # Requests side by side, each on a thread; long calls watched beside them.
  find_package(Threads REQUIRED)
  target_link_libraries(clarity_service PUBLIC Threads::Threads)
  if(WIN32)
    # Winsock for the server, the shell for links.
    target_link_libraries(clarity_service PUBLIC ws2_32 shell32)
  endif()
endif()

# The folder of Clarity.UI's library and the libraries beside it (CLARITY_UI):
# named by the environment variable or -DCLARITY_UI, else fetched from the
# release of CLARITY_UI_VERSION into the build folder. The desktop needs it,
# and so does any program that reads or writes a format (Clarity.Formats lies there).
set(CLARITY_RUNTIME_DIR ${CMAKE_CURRENT_LIST_DIR})
macro(clarity_ui_folder)
  set(CLARITY_UI "$ENV{CLARITY_UI}" CACHE PATH "The folder of Clarity.UI's library: include/clarity_ui.h and lib/")
  if(NOT CLARITY_UI AND CLARITY_UI_VERSION)
    include(${CLARITY_RUNTIME_DIR}/clarity_ui.cmake)
    clarity_ui_rid(CLARITY_UI_RID "${CMAKE_SYSTEM_NAME}" "${CMAKE_SYSTEM_PROCESSOR}")
    if(CLARITY_UI_RID)
      clarity_ui_fetch(CLARITY_UI ${CLARITY_UI_RELEASES} ${CLARITY_UI_VERSION} ${CLARITY_UI_RID} ${CMAKE_BINARY_DIR}/clarity-ui)
    endif()
  endif()
  if(NOT CLARITY_UI OR NOT EXISTS "${CLARITY_UI}/include/clarity_ui.h")
    message(FATAL_ERROR "Clarity.UI's library is not found: set CLARITY_UI (the environment variable, or -DCLARITY_UI=...) to the folder with include/clarity_ui.h and lib/ (now '${CLARITY_UI}')")
  endif()
  # The libraries are looked up in the folder CLARITY_UI names, and CMake keeps what it found:
  # when that folder changes (a new Clarity.UI version fetched), what was found in the old one goes.
  if(NOT "${CLARITY_UI}" STREQUAL "${CLARITY_UI_LOOKED_IN}")
    foreach(found CLARITY_UI_SHARED CLARITY_UI_IMPORT CLARITY_AUDIO_IMPORT CLARITY_IMAGE_IMPORT CLARITY_VIDEO_IMPORT CLARITY_FORMATS_IMPORT)
      unset(${found} CACHE)
    endforeach()
    set(CLARITY_UI_LOOKED_IN "${CLARITY_UI}" CACHE INTERNAL "The Clarity.UI folder the libraries were looked up in")
  endif()
endmacro()

# The desktop (DESIGN 7.9): Clarity.UI's library draws the windows, takes
# the person's input and runs the window; the program answers it. The
# library comes from the folder CLARITY_UI names (by default the environment
# variable of that name): its include/clarity_ui.h, and in lib/ the library
# for this system (libclarity_ui.so, libclarity_ui.dylib, or clarity_ui.dll
# with its import library). Without it, a generated project (which names
# CLARITY_UI_VERSION) fetches the library from its release into the build
# folder (clarity_ui.cmake). The program finds it beside itself: every
# program that links it gets a copy next to it (clarity_beside below).
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/desktop/desktop.c)
  clarity_ui_folder()
  add_library(clarity_ui SHARED IMPORTED GLOBAL)
  set_target_properties(clarity_ui PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
  if(WIN32)
    find_library(CLARITY_UI_IMPORT NAMES clarity_ui.dll.lib libclarity_ui.dll.a clarity_ui.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
    set_target_properties(clarity_ui PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_ui.dll" IMPORTED_IMPLIB "${CLARITY_UI_IMPORT}")
  elseif(APPLE)
    set_target_properties(clarity_ui PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_ui.dylib")
  else()
    # The library names no soname: linked by its name, so the program looks for it by that name.
    set_target_properties(clarity_ui PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_ui.so" IMPORTED_NO_SONAME TRUE)
  endif()

  add_library(clarity_desktop STATIC ${CMAKE_CURRENT_LIST_DIR}/desktop/desktop.c)
  target_include_directories(clarity_desktop PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_desktop PUBLIC clarity_service clarity_ui)
  find_package(Threads REQUIRED)
  target_link_libraries(clarity_desktop PUBLIC Threads::Threads)
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_desktop PRIVATE -Wall -Wextra)
  endif()
endif()

# Puts Clarity.UI's library beside `target` after every build, and has the
# program look for it there: on Linux and macOS through the program's rpath,
# on Windows, where a program looks beside itself first anyway.
function(clarity_beside target)
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_ui> $<TARGET_FILE_DIR:${target}>)
  if(APPLE)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "@loader_path" INSTALL_RPATH "@loader_path")
  elseif(UNIX)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "$ORIGIN" INSTALL_RPATH "$ORIGIN")
  endif()
endfunction()

# Clarity.Audio (docs/AUDIO-PLAN.md) for a program whose sound parts play
# songs: its library lies beside Clarity.UI's in the folder CLARITY_UI names
# (include/clarity_audio.h and lib/). Linked to `target` and put beside it.
function(clarity_use_audio target)
  if(NOT TARGET clarity_audio)
    if(NOT EXISTS "${CLARITY_UI}/include/clarity_audio.h")
      message(FATAL_ERROR "The program plays songs, but Clarity.Audio's library is not in '${CLARITY_UI}': use a Clarity.UI folder with include/clarity_audio.h and its library in lib/")
    endif()
    add_library(clarity_audio SHARED IMPORTED GLOBAL)
    set_target_properties(clarity_audio PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
    if(WIN32)
      find_library(CLARITY_AUDIO_IMPORT NAMES clarity_audio.dll.lib libclarity_audio.dll.a clarity_audio.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
      set_target_properties(clarity_audio PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_audio.dll" IMPORTED_IMPLIB "${CLARITY_AUDIO_IMPORT}")
    elseif(APPLE)
      set_target_properties(clarity_audio PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_audio.dylib")
    else()
      set_target_properties(clarity_audio PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_audio.so" IMPORTED_NO_SONAME TRUE)
    endif()
  endif()
  target_link_libraries(${target} PRIVATE clarity_audio)
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_audio> $<TARGET_FILE_DIR:${target}>)
endfunction()

# Clarity.Formats (docs/FORMATS-PLAN.md) for a program whose code reads or
# writes a format, on any target: its library lies beside Clarity.UI's in
# the folder CLARITY_UI names (include/clarity_formats.h and lib/), and the
# runtime's formats/formats.c calls it, and formats/file_source.c reads a
# file data source through it. Linked to `target` and put beside it,
# where the program finds it.
function(clarity_use_formats target)
  clarity_ui_folder()
  if(NOT TARGET clarity_formats)
    if(NOT EXISTS "${CLARITY_UI}/include/clarity_formats.h")
      message(FATAL_ERROR "The program's code reads or writes a format, but Clarity.Formats' library is not in '${CLARITY_UI}': use a Clarity.UI folder with include/clarity_formats.h and its library in lib/")
    endif()
    add_library(clarity_formats SHARED IMPORTED GLOBAL)
    set_target_properties(clarity_formats PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
    if(WIN32)
      find_library(CLARITY_FORMATS_IMPORT NAMES clarity_formats.dll.lib libclarity_formats.dll.a clarity_formats.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
      set_target_properties(clarity_formats PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_formats.dll" IMPORTED_IMPLIB "${CLARITY_FORMATS_IMPORT}")
    elseif(APPLE)
      set_target_properties(clarity_formats PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_formats.dylib")
    else()
      set_target_properties(clarity_formats PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_formats.so" IMPORTED_NO_SONAME TRUE)
    endif()
  endif()
  target_sources(${target} PRIVATE ${CLARITY_RUNTIME_DIR}/formats/formats.c ${CLARITY_RUNTIME_DIR}/formats/file_source.c)
  target_link_libraries(${target} PRIVATE clarity_formats)
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_formats> $<TARGET_FILE_DIR:${target}>)
  if(APPLE)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "@loader_path" INSTALL_RPATH "@loader_path")
  elseif(UNIX)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "$ORIGIN" INSTALL_RPATH "$ORIGIN")
  endif()
endfunction()

# Clarity.AppMcp (docs/ledger/app-mcp.md), for a program whose commands
# agents may call (`<program> mcp`). Linked to `target` and put beside it,
# where the program finds it.
function(clarity_use_agents target)
  clarity_ui_folder()
  if(NOT TARGET clarity_app_mcp)
    if(NOT EXISTS "${CLARITY_UI}/include/clarity_app_mcp.h")
      message(FATAL_ERROR "The program offers commands to agents, but Clarity.AppMcp's library is not in '${CLARITY_UI}': use a Clarity.UI folder with include/clarity_app_mcp.h and its library in lib/")
    endif()
    add_library(clarity_app_mcp SHARED IMPORTED GLOBAL)
    set_target_properties(clarity_app_mcp PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
    if(WIN32)
      find_library(CLARITY_APP_MCP_IMPORT NAMES clarity_app_mcp.dll.lib libclarity_app_mcp.dll.a clarity_app_mcp.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
      set_target_properties(clarity_app_mcp PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_app_mcp.dll" IMPORTED_IMPLIB "${CLARITY_APP_MCP_IMPORT}")
    elseif(APPLE)
      set_target_properties(clarity_app_mcp PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_app_mcp.dylib")
    else()
      set_target_properties(clarity_app_mcp PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_app_mcp.so" IMPORTED_NO_SONAME TRUE)
    endif()
  endif()
  target_sources(${target} PRIVATE ${CLARITY_RUNTIME_DIR}/agents/agents.c)
  target_link_libraries(${target} PRIVATE clarity_app_mcp)
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_app_mcp> $<TARGET_FILE_DIR:${target}>)
  if(APPLE)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "@loader_path" INSTALL_RPATH "@loader_path")
  elseif(UNIX)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "$ORIGIN" INSTALL_RPATH "$ORIGIN")
  endif()
endfunction()

# Clarity.Auth (docs/APP-CAPABILITIES-PLAN.md, step 4) for a program whose
# project signs people in, on any target: its library lies beside
# Clarity.UI's in the folder CLARITY_UI names (include/clarity_auth.h and
# lib/), and the runtime's auth/auth.c calls it. Linked to `target` and
# put beside it, where the program finds it.
function(clarity_use_auth target)
  clarity_ui_folder()
  if(NOT TARGET clarity_auth)
    if(NOT EXISTS "${CLARITY_UI}/include/clarity_auth.h")
      message(FATAL_ERROR "The program signs people in, but Clarity.Auth's library is not in '${CLARITY_UI}': use a Clarity.UI folder with include/clarity_auth.h and its library in lib/")
    endif()
    add_library(clarity_auth SHARED IMPORTED GLOBAL)
    set_target_properties(clarity_auth PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
    if(WIN32)
      find_library(CLARITY_AUTH_IMPORT NAMES clarity_auth.dll.lib libclarity_auth.dll.a clarity_auth.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
      set_target_properties(clarity_auth PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_auth.dll" IMPORTED_IMPLIB "${CLARITY_AUTH_IMPORT}")
    elseif(APPLE)
      set_target_properties(clarity_auth PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_auth.dylib")
    else()
      set_target_properties(clarity_auth PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_auth.so" IMPORTED_NO_SONAME TRUE)
    endif()
  endif()
  target_sources(${target} PRIVATE ${CLARITY_RUNTIME_DIR}/auth/auth.c)
  target_link_libraries(${target} PRIVATE clarity_auth)
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_auth> $<TARGET_FILE_DIR:${target}>)
  if(APPLE)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "@loader_path" INSTALL_RPATH "@loader_path")
  elseif(UNIX)
    set_target_properties(${target} PROPERTIES BUILD_RPATH "$ORIGIN" INSTALL_RPATH "$ORIGIN")
  endif()
endfunction()

# Clarity.Image (docs/IMAGE-PLAN.md) for a program whose code calls a
# picture function: its library lies beside Clarity.UI's in the folder
# CLARITY_UI names (include/clarity_image.h and lib/), and the runtime's
# pictures/pictures.c calls it. Linked to `target` and put beside it.
function(clarity_use_image target)
  if(NOT TARGET clarity_image)
    if(NOT EXISTS "${CLARITY_UI}/include/clarity_image.h")
      message(FATAL_ERROR "The program's code calls picture functions, but Clarity.Image's library is not in '${CLARITY_UI}': use a Clarity.UI folder with include/clarity_image.h and its library in lib/")
    endif()
    add_library(clarity_image SHARED IMPORTED GLOBAL)
    set_target_properties(clarity_image PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
    if(WIN32)
      find_library(CLARITY_IMAGE_IMPORT NAMES clarity_image.dll.lib libclarity_image.dll.a clarity_image.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
      set_target_properties(clarity_image PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_image.dll" IMPORTED_IMPLIB "${CLARITY_IMAGE_IMPORT}")
    elseif(APPLE)
      set_target_properties(clarity_image PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_image.dylib")
    else()
      set_target_properties(clarity_image PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_image.so" IMPORTED_NO_SONAME TRUE)
    endif()
  endif()
  target_sources(${target} PRIVATE ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/pictures/pictures.c)
  target_link_libraries(${target} PRIVATE clarity_image)
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_image> $<TARGET_FILE_DIR:${target}>)
endfunction()

# Clarity.Video (docs/VIDEO-PLAN.md) for a program with video parts: its
# library and Cisco's openh264 lie beside Clarity.UI's in the folder CLARITY_UI
# names. Linked to `target` and put beside it, openh264 too, so videos play
# offline.
function(clarity_use_video target)
  if(NOT TARGET clarity_video)
    if(NOT EXISTS "${CLARITY_UI}/include/clarity_video.h")
      message(FATAL_ERROR "The program plays videos, but Clarity.Video's library is not in '${CLARITY_UI}': use a Clarity.UI folder with include/clarity_video.h and its library in lib/")
    endif()
    add_library(clarity_video SHARED IMPORTED GLOBAL)
    set_target_properties(clarity_video PROPERTIES INTERFACE_INCLUDE_DIRECTORIES "${CLARITY_UI}/include")
    if(WIN32)
      find_library(CLARITY_VIDEO_IMPORT NAMES clarity_video.dll.lib libclarity_video.dll.a clarity_video.lib PATHS "${CLARITY_UI}/lib" NO_DEFAULT_PATH REQUIRED)
      set_target_properties(clarity_video PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/clarity_video.dll" IMPORTED_IMPLIB "${CLARITY_VIDEO_IMPORT}")
    elseif(APPLE)
      set_target_properties(clarity_video PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_video.dylib")
    else()
      set_target_properties(clarity_video PROPERTIES IMPORTED_LOCATION "${CLARITY_UI}/lib/libclarity_video.so" IMPORTED_NO_SONAME TRUE)
    endif()
  endif()
  target_link_libraries(${target} PRIVATE clarity_video)
  file(GLOB CLARITY_OPENH264 "${CLARITY_UI}/lib/*openh264-*")
  add_custom_command(TARGET ${target} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:clarity_video> ${CLARITY_OPENH264} $<TARGET_FILE_DIR:${target}>)
  # The codec libraries the project adds go in a codecs folder beside the program.
  file(GLOB CLARITY_CODECS "${CMAKE_SOURCE_DIR}/codecs/*")
  if(CLARITY_CODECS)
    add_custom_command(TARGET ${target} POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E make_directory $<TARGET_FILE_DIR:${target}>/codecs
      COMMAND ${CMAKE_COMMAND} -E copy_if_different ${CLARITY_CODECS} $<TARGET_FILE_DIR:${target}>/codecs)
  endif()
endfunction()

# The bytes of `file` as a C initializer in `out`: a string literal where
# the compiler takes a long one cheaply (GCC, Clang), else a list of bytes
# (MSVC limits string literals). Either ends with a zero byte past the file.
function(clarity_bytes out file)
  file(READ ${file} hex HEX)
  if(MSVC)
    string(REGEX REPLACE "([0-9a-f][0-9a-f])" "0x\\1," bytes "${hex}")
    set(${out} "{${bytes}0x00}" PARENT_SCOPE)
  else()
    string(REGEX REPLACE "([0-9a-f][0-9a-f])" "\\\\x\\1" bytes "${hex}")
    set(${out} "\"${bytes}\"" PARENT_SCOPE)
  endif()
endfunction()

# The files a program carries (DESIGN 7.9), put into the program as C
# arrays in the order given, by their names: for Clarity.UI on the desktop
# the windows and app frames as markup (WINDOWS, APPS: windows/Main.cxaml
# is "Main"), the resolved themes (THEMES: themes/shop.light.json is
# "shop.light") and the resources (RESOURCES: resources/fonts/a.ttf is
# "fonts/a.ttf"); for the web target every file below the folder BROWSER
# names, by its path there (browser/clarity/files.json is "clarity/files.json"),
# which the server serves to the browser. Paths are relative to the
# project; a change to one configures again.
function(clarity_carry target)
  cmake_parse_arguments(CARRY "" "BROWSER" "WINDOWS;APPS;THEMES;RESOURCES" ${ARGN})
  set(CARRY_BROWSER_FILES "")
  if(CARRY_BROWSER)
    file(GLOB_RECURSE found RELATIVE ${CMAKE_CURRENT_SOURCE_DIR} ${CMAKE_CURRENT_SOURCE_DIR}/${CARRY_BROWSER}/*)
    list(SORT found)
    set(CARRY_BROWSER_FILES ${found})
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/${CARRY_BROWSER})
  endif()
  set(source "/* The files the program carries, from the project's folders. */\n#include \"service/carried.h\"\n\n")
  set(index 0)
  foreach(kind WINDOWS APPS THEMES RESOURCES BROWSER_FILES)
    set(entries "")
    set(count 0)
    foreach(path IN LISTS CARRY_${kind})
      set(file ${CMAKE_CURRENT_SOURCE_DIR}/${path})
      set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${file})
      if(kind STREQUAL "RESOURCES")
        string(REGEX REPLACE "^resources/" "" name "${path}")
      elseif(kind STREQUAL "BROWSER_FILES")
        file(RELATIVE_PATH name ${CMAKE_CURRENT_SOURCE_DIR}/${CARRY_BROWSER} ${file})
      else()
        get_filename_component(name "${path}" NAME)
        string(REGEX REPLACE "\\.(cxaml|json)$" "" name "${name}")
      endif()
      clarity_bytes(bytes ${file})
      file(SIZE ${file} size)
      string(APPEND source "static const unsigned char carried_${index}[] = ${bytes};\n")
      string(APPEND entries "    {\"${name}\", carried_${index}, ${size}},\n")
      math(EXPR index "${index} + 1")
      math(EXPR count "${count} + 1")
    endforeach()
    if(kind STREQUAL "BROWSER_FILES")
      set(lower browser)
    else()
      string(TOLOWER ${kind} lower)
    endif()
    if(count EQUAL 0)
      string(APPEND entries "    {\"\", 0, 0},\n")
    endif()
    string(APPEND source "static const ClarityCarried carried_${lower}[] = {\n${entries}};\n\n")
    set(${lower}_count ${count})
  endforeach()
  string(APPEND source "const ClarityCarriedFiles clarity_carried = {carried_windows, ${windows_count}, carried_apps, ${apps_count}, carried_themes, ${themes_count}, carried_resources, ${resources_count}, carried_browser, ${browser_count}};\n")
  file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/carried.c.in "${source}")
  configure_file(${CMAKE_CURRENT_BINARY_DIR}/carried.c.in ${CMAKE_CURRENT_BINARY_DIR}/carried.c COPYONLY)
  target_sources(${target} PRIVATE ${CMAKE_CURRENT_BINARY_DIR}/carried.c)
endfunction()

# HTTP calls from code (DESIGN 5.23), through libcurl: only in a project
# whose runtime copy has them, that is, one whose code calls out.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/http/http.c)
  find_package(CURL REQUIRED)
  add_library(clarity_http STATIC ${CMAKE_CURRENT_LIST_DIR}/http/http.c)
  target_include_directories(clarity_http PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_http PUBLIC clarity CURL::libcurl)
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_http PRIVATE -Wall -Wextra)
  endif()
endif()

# Secret settings (DESIGN 5.27) in the system's protected store: only in a
# project whose runtime copy has them, that is, one with a secret setting.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.c)
  add_library(clarity_secrets STATIC ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.c)
  target_include_directories(clarity_secrets PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_secrets PUBLIC clarity)
  if(WIN32)
    target_link_libraries(clarity_secrets PUBLIC advapi32)
  elseif(APPLE)
    target_link_libraries(clarity_secrets PUBLIC "-framework Security" "-framework CoreFoundation")
  else()
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(CLARITY_GIO REQUIRED IMPORTED_TARGET gio-2.0)
    target_link_libraries(clarity_secrets PUBLIC PkgConfig::CLARITY_GIO)
  endif()
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_secrets PRIVATE -Wall -Wextra)
  endif()
endif()

# Engines (DESIGN 5.11), hosted by wasm3: only in a project whose runtime
# copy has them, that is, one that declares an engine.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/engine/engine.c)
  file(GLOB WASM3_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/third_party/wasm3/*.c)
  add_library(clarity_engine STATIC ${CMAKE_CURRENT_LIST_DIR}/engine/engine.c ${WASM3_SOURCES})
  target_include_directories(clarity_engine PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
  target_link_libraries(clarity_engine PUBLIC clarity m)
  # wasm3 is taken as released; its own warnings are not the runtime's.
  set_source_files_properties(${WASM3_SOURCES} PROPERTIES COMPILE_OPTIONS "-w")
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    set_source_files_properties(${CMAKE_CURRENT_LIST_DIR}/engine/engine.c PROPERTIES COMPILE_OPTIONS "-Wall;-Wextra")
  endif()
  # Engines beside the screen (DESIGN 5.12): one instance per canvas, held by number.
  if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/engine/screen.c)
    target_sources(clarity_engine PRIVATE ${CMAKE_CURRENT_LIST_DIR}/engine/screen.c)
    find_package(Threads REQUIRED)
    target_link_libraries(clarity_engine PUBLIC Threads::Threads)
    if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
      set_source_files_properties(${CMAKE_CURRENT_LIST_DIR}/engine/screen.c PROPERTIES COMPILE_OPTIONS "-Wall;-Wextra")
    endif()
  endif()
endif()
