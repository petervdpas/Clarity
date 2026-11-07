# The runtime's libraries, for the runtime's own build and for generated
# projects, which include this file: `clarity` (records, storage, the
# binding to the screen), `clarity_service` (the screen's commands and the
# web server) and `clarity_desktop` (the desktop program on Clarity.UI).
find_package(SQLite3 REQUIRED)
find_package(nlohmann_json 3 REQUIRED)

file(GLOB CLARITY_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/clarity/*.cpp)
add_library(clarity STATIC ${CLARITY_SOURCES})
target_include_directories(clarity PUBLIC ${CMAKE_CURRENT_LIST_DIR})
target_link_libraries(clarity PUBLIC SQLite::SQLite3 nlohmann_json::nlohmann_json)
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(clarity PRIVATE -Wall -Wextra)
endif()

# The service, which answers Clarity.UI's commands and serves the browser
# files to a browser (the web target): nothing of the desktop.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/service.cpp)
  add_library(clarity_service STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/service.cpp ${CMAKE_CURRENT_LIST_DIR}/shell/serve.cpp)
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

# The desktop program on Clarity.UI (DESIGN 7.9): only in a project whose
# runtime copy has it, so a command-line program and a web server build
# without it. Clarity.UI is one library (libclarity_ui.so, clarity_ui.dll,
# libclarity_ui.dylib) with its header, in the folder CLARITY_UI names:
# include/clarity_ui.h and lib/ with the library for this system. Without
# it, a generated project (which names CLARITY_UI_VERSION) fetches the
# library from its release into the build folder (clarity_ui.cmake). The
# program finds the shared library beside it, where clarity_ui_beside
# copies it after each build.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/desktop.cpp)
  set(CLARITY_UI "$ENV{CLARITY_UI}" CACHE PATH "The folder of Clarity.UI: include/clarity_ui.h and lib/ with the library")
  if(NOT CLARITY_UI AND CLARITY_UI_VERSION)
    include(${CMAKE_CURRENT_LIST_DIR}/clarity_ui.cmake)
    clarity_ui_rid(CLARITY_UI_RID "${CMAKE_SYSTEM_NAME}" "${CMAKE_SYSTEM_PROCESSOR}")
    if(CLARITY_UI_RID)
      clarity_ui_fetch(CLARITY_UI ${CLARITY_UI_RELEASES} ${CLARITY_UI_VERSION} ${CLARITY_UI_RID} ${CMAKE_BINARY_DIR}/clarity-ui)
    endif()
  endif()
  if(NOT CLARITY_UI OR NOT EXISTS ${CLARITY_UI}/include/clarity_ui.h)
    message(FATAL_ERROR "Clarity.UI is not found: set CLARITY_UI (the environment variable, or -DCLARITY_UI=...) to its folder, which holds include/clarity_ui.h and lib/ with the library")
  endif()
  if(WIN32)
    find_file(CLARITY_UI_SHARED NAMES clarity_ui.dll PATHS ${CLARITY_UI}/lib ${CLARITY_UI}/bin NO_DEFAULT_PATH)
    find_file(CLARITY_UI_IMPORT NAMES clarity_ui.dll.lib clarity_ui.lib libclarity_ui.dll.a PATHS ${CLARITY_UI}/lib NO_DEFAULT_PATH)
  elseif(APPLE)
    find_file(CLARITY_UI_SHARED NAMES libclarity_ui.dylib PATHS ${CLARITY_UI}/lib NO_DEFAULT_PATH)
  else()
    find_file(CLARITY_UI_SHARED NAMES libclarity_ui.so PATHS ${CLARITY_UI}/lib NO_DEFAULT_PATH)
  endif()
  if(NOT CLARITY_UI_SHARED OR (WIN32 AND NOT CLARITY_UI_IMPORT))
    message(FATAL_ERROR "Clarity.UI's library for this system is not in ${CLARITY_UI}/lib")
  endif()
  add_library(clarity_ui SHARED IMPORTED GLOBAL)
  set_target_properties(clarity_ui PROPERTIES IMPORTED_LOCATION ${CLARITY_UI_SHARED} INTERFACE_INCLUDE_DIRECTORIES ${CLARITY_UI}/include)
  if(WIN32)
    set_target_properties(clarity_ui PROPERTIES IMPORTED_IMPLIB ${CLARITY_UI_IMPORT})
  elseif(NOT APPLE)
    # The library names no soname: the program asks for it by its file name, found beside it.
    set_target_properties(clarity_ui PROPERTIES IMPORTED_NO_SONAME TRUE)
  endif()
  add_library(clarity_desktop STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/desktop.cpp ${CMAKE_CURRENT_LIST_DIR}/ui/ui.cpp)
  target_include_directories(clarity_desktop PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_desktop PUBLIC clarity_service clarity_ui)
  # Handlers of long calls run on the library's worker thread (DESIGN 5.12).
  find_package(Threads REQUIRED)
  target_link_libraries(clarity_desktop PUBLIC Threads::Threads)
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_desktop PRIVATE -Wall -Wextra)
  endif()
endif()

# A program on Clarity.UI finds the library beside it: copied there after
# each build, and looked for there (rpath $ORIGIN, @loader_path; Windows
# looks beside the program by itself).
function(clarity_ui_beside target)
  add_custom_command(TARGET ${target} POST_BUILD COMMAND ${CMAKE_COMMAND} -E copy_if_different ${CLARITY_UI_SHARED} $<TARGET_FILE_DIR:${target}>)
  if(APPLE)
    set_target_properties(${target} PROPERTIES BUILD_WITH_INSTALL_RPATH TRUE INSTALL_RPATH "@loader_path")
  elseif(NOT WIN32)
    set_target_properties(${target} PROPERTIES BUILD_WITH_INSTALL_RPATH TRUE INSTALL_RPATH "$ORIGIN")
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

# The bytes of `file` as a C++ initializer in `out`: a string literal where
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

# The files a browser takes from the web server (the web target, DESIGN
# 7.9), compiled into `target`: every file below `folder` (browser/: the page,
# Clarity.UI's browser files, clarity/files.json, the resources), named by
# its path there, as the list clarity_browser_files the program hands to
# the server (shell::Options::browser_files).
function(clarity_browser_files target folder)
  file(GLOB_RECURSE found ${folder}/*)
  list(SORT found)
  set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${found} ${folder})
  set(arrays "")
  set(entries "")
  set(n 0)
  foreach(path IN LISTS found)
    file(RELATIVE_PATH name ${folder} ${path})
    clarity_bytes(bytes ${path})
    file(SIZE ${path} size)
    string(APPEND arrays "static const unsigned char browser${n}[] = ${bytes};\n")
    string(APPEND entries "    {\"browser\", \"${name}\", browser${n}, ${size}},\n")
    math(EXPR n "${n} + 1")
  endforeach()
  file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/clarity_browser_files.cpp.in "/* The files a browser takes from the web server, from ${folder}. */\n#include \"shell/shell.hpp\"\n\n${arrays}\nextern const clarity::shell::UiFile clarity_browser_files[] = {\n${entries}    {nullptr, nullptr, nullptr, 0},\n};\n")
  configure_file(${CMAKE_CURRENT_BINARY_DIR}/clarity_browser_files.cpp.in ${CMAKE_CURRENT_BINARY_DIR}/clarity_browser_files.cpp COPYONLY)
  target_sources(${target} PRIVATE ${CMAKE_CURRENT_BINARY_DIR}/clarity_browser_files.cpp)
endfunction()

# The files Clarity.UI draws from, compiled into `target`: the windows and
# app frames as markup (windows/*.cxaml, apps/*.cxaml), the themes
# (themes/*.json) and the resources (resources/...) under `folder`, as the
# list clarity_ui_files the program hands to the desktop (shell::UiFile).
# The themes' names after `folder` give their order, the one the program
# starts with first; without them, every theme in the folder.
function(clarity_ui_files target folder)
  file(GLOB windows ${folder}/windows/*.cxaml)
  file(GLOB apps ${folder}/apps/*.cxaml)
  if(ARGN)
    set(themes "")
    foreach(theme IN LISTS ARGN)
      list(APPEND themes ${folder}/themes/${theme}.json)
    endforeach()
  else()
    file(GLOB themes ${folder}/themes/*.json)
  endif()
  file(GLOB_RECURSE resources ${folder}/resources/*)
  set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${windows} ${apps} ${themes} ${resources} ${folder}/windows ${folder}/apps ${folder}/themes)
  set(arrays "")
  set(entries "")
  set(n 0)
  foreach(kind window app theme resource)
    if(kind STREQUAL "window")
      set(found ${windows})
    elseif(kind STREQUAL "app")
      set(found ${apps})
    elseif(kind STREQUAL "theme")
      set(found ${themes})
    else()
      set(found ${resources})
    endif()
    foreach(path IN LISTS found)
      if(kind STREQUAL "resource")
        file(RELATIVE_PATH name ${folder}/resources ${path})
      else()
        get_filename_component(name ${path} NAME_WLE)
      endif()
      clarity_bytes(bytes ${path})
      file(SIZE ${path} size)
      string(APPEND arrays "static const unsigned char file${n}[] = ${bytes};\n")
      string(APPEND entries "    {\"${kind}\", \"${name}\", file${n}, ${size}},\n")
      math(EXPR n "${n} + 1")
    endforeach()
  endforeach()
  file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/clarity_ui_files.cpp "/* The files Clarity.UI draws from, from ${folder}. */\n#include \"shell/shell.hpp\"\n\n${arrays}\nextern const clarity::shell::UiFile clarity_ui_files[] = {\n${entries}    {nullptr, nullptr, nullptr, 0},\n};\n")
  target_sources(${target} PRIVATE ${CMAKE_CURRENT_BINARY_DIR}/clarity_ui_files.cpp)
endfunction()

# HTTP calls from code (DESIGN 5.23), through libcurl: only in a project
# whose runtime copy has them, that is, one whose code calls out.
# Secret settings (DESIGN 5.27) in the system's protected store: only in a
# project whose runtime copy has them, that is, one with a secret setting.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.cpp)
  add_library(clarity_secrets STATIC ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.cpp)
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
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_secrets PRIVATE -Wall -Wextra)
  endif()
endif()

if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/http/http.cpp)
  find_package(CURL REQUIRED)
  add_library(clarity_http STATIC ${CMAKE_CURRENT_LIST_DIR}/http/http.cpp)
  target_include_directories(clarity_http PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_http PUBLIC clarity CURL::libcurl)
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_http PRIVATE -Wall -Wextra)
  endif()
endif()

# Engines (DESIGN 5.11), hosted by wasm3: only in a project whose runtime
# copy has them, that is, one that declares an engine. wasm3 is C; the
# runtime's own build takes it from the C runtime beside it.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/engine/engine.cpp)
  enable_language(C)
  set(WASM3_DIR ${CMAKE_CURRENT_LIST_DIR}/third_party/wasm3)
  if(NOT EXISTS ${WASM3_DIR})
    set(WASM3_DIR ${CMAKE_CURRENT_LIST_DIR}/../c/third_party/wasm3)
  endif()
  file(GLOB WASM3_SOURCES CONFIGURE_DEPENDS ${WASM3_DIR}/*.c)
  add_library(clarity_engine STATIC ${CMAKE_CURRENT_LIST_DIR}/engine/engine.cpp ${CMAKE_CURRENT_LIST_DIR}/engine/screen.cpp ${WASM3_SOURCES})
  target_include_directories(clarity_engine PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${WASM3_DIR})
  target_link_libraries(clarity_engine PUBLIC clarity m)
  # wasm3 is taken as released; its own warnings are not the runtime's.
  set_source_files_properties(${WASM3_SOURCES} PROPERTIES COMPILE_OPTIONS "-w")
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    set_source_files_properties(${CMAKE_CURRENT_LIST_DIR}/engine/engine.cpp ${CMAKE_CURRENT_LIST_DIR}/engine/screen.cpp PROPERTIES COMPILE_OPTIONS "-Wall;-Wextra")
  endif()
endif()
