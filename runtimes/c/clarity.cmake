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

# The service answers the commands of the web wire, and serves the web
# screen to a browser (the web target): only in a project whose runtime
# copy has it, so a command-line program builds without it.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/service/service.c)
  add_library(clarity_service STATIC ${CMAKE_CURRENT_LIST_DIR}/service/service.c ${CMAKE_CURRENT_LIST_DIR}/service/serve.c)
  target_include_directories(clarity_service PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_service PUBLIC clarity)
  if(WIN32)
    # Winsock for the server, the shell for links.
    target_link_libraries(clarity_service PUBLIC ws2_32 shell32)
  endif()
endif()

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
  set(CLARITY_UI "$ENV{CLARITY_UI}" CACHE PATH "The folder of Clarity.UI's library: include/clarity_ui.h and lib/")
  if(NOT CLARITY_UI AND CLARITY_UI_VERSION)
    include(${CMAKE_CURRENT_LIST_DIR}/clarity_ui.cmake)
    clarity_ui_rid(CLARITY_UI_RID "${CMAKE_SYSTEM_NAME}" "${CMAKE_SYSTEM_PROCESSOR}")
    if(CLARITY_UI_RID)
      clarity_ui_fetch(CLARITY_UI ${CLARITY_UI_RELEASES} ${CLARITY_UI_VERSION} ${CLARITY_UI_RID} ${CMAKE_BINARY_DIR}/clarity-ui)
    endif()
  endif()
  if(NOT CLARITY_UI OR NOT EXISTS "${CLARITY_UI}/include/clarity_ui.h")
    message(FATAL_ERROR "Clarity.UI's library is not found: set CLARITY_UI (the environment variable, or -DCLARITY_UI=...) to the folder with include/clarity_ui.h and lib/ (now '${CLARITY_UI}')")
  endif()
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

# The files a desktop program carries for Clarity.UI (DESIGN 7.9), put into
# the program as C arrays in the order given, by their names: the windows
# and app frames as markup (WINDOWS, APPS: windows/Main.cxaml is "Main"),
# the resolved themes (THEMES: themes/shop.light.json is "shop.light") and
# the resources (RESOURCES: resources/fonts/a.ttf is "fonts/a.ttf"). Paths
# are relative to the project; a change to one configures again.
function(clarity_carry target)
  cmake_parse_arguments(CARRY "" "" "WINDOWS;APPS;THEMES;RESOURCES" ${ARGN})
  set(source "/* The files the program carries for Clarity.UI, from the project's folders. */\n#include \"desktop/desktop.h\"\n\n")
  set(index 0)
  foreach(kind WINDOWS APPS THEMES RESOURCES)
    set(entries "")
    set(count 0)
    foreach(path IN LISTS CARRY_${kind})
      set(file ${CMAKE_CURRENT_SOURCE_DIR}/${path})
      set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${file})
      if(kind STREQUAL "RESOURCES")
        string(REGEX REPLACE "^resources/" "" name "${path}")
      else()
        get_filename_component(name "${path}" NAME)
        string(REGEX REPLACE "\\.(cxaml|json)$" "" name "${name}")
      endif()
      file(READ ${file} hex HEX)
      string(REGEX REPLACE "([0-9a-f][0-9a-f])" "0x\\1," bytes "${hex}")
      file(SIZE ${file} size)
      string(APPEND source "static const unsigned char carried_${index}[] = {${bytes}0x00};\n")
      string(APPEND entries "    {\"${name}\", carried_${index}, ${size}},\n")
      math(EXPR index "${index} + 1")
      math(EXPR count "${count} + 1")
    endforeach()
    string(TOLOWER ${kind} lower)
    if(count EQUAL 0)
      string(APPEND entries "    {\"\", 0, 0},\n")
    endif()
    string(APPEND source "static const ClarityCarried carried_${lower}[] = {\n${entries}};\n\n")
    set(${lower}_count ${count})
  endforeach()
  string(APPEND source "const ClarityCarriedFiles clarity_carried = {carried_windows, ${windows_count}, carried_apps, ${apps_count}, carried_themes, ${themes_count}, carried_resources, ${resources_count}};\n")
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
