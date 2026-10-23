# The runtime's libraries, for the runtime's own build and for generated
# projects, which include this file: `clarity` (records, storage, the
# binding to the screen, with cJSON) and `clarity_shell` (the window and
# webview). The webview's own code is C++, so a project needs a C++
# compiler for that one file.
find_package(SQLite3 REQUIRED)

file(GLOB CLARITY_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/clarity/*.c)
add_library(clarity STATIC ${CLARITY_SOURCES} ${CMAKE_CURRENT_LIST_DIR}/third_party/cjson/cJSON.c)
target_include_directories(clarity PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
target_link_libraries(clarity PUBLIC SQLite::SQLite3 m)
if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(clarity PRIVATE $<$<COMPILE_LANGUAGE:C>:-Wall -Wextra>)
endif()

# The shell, which shows the windows: only in a project whose runtime copy
# has it, so a command-line program builds without a webview.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/shell.c)
  # The webview draws with WebKitGTK on Linux, WebView2 on Windows and WebKit on macOS.
  add_library(clarity_shell STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/shell.c ${CMAKE_CURRENT_LIST_DIR}/shell/parts.c ${CMAKE_CURRENT_LIST_DIR}/shell/webview.cc)
  target_include_directories(clarity_shell PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
  target_compile_definitions(clarity_shell PUBLIC WEBVIEW_STATIC)
  target_compile_features(clarity_shell PRIVATE cxx_std_11)
  target_link_libraries(clarity_shell PUBLIC clarity)
  if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(WEBKIT REQUIRED IMPORTED_TARGET gtk+-3.0 webkit2gtk-4.1)
    target_link_libraries(clarity_shell PUBLIC PkgConfig::WEBKIT dl)
  elseif(APPLE)
    target_link_libraries(clarity_shell PUBLIC "-framework WebKit")
  elseif(WIN32)
    target_link_libraries(clarity_shell PUBLIC advapi32 ole32 shell32 shlwapi user32 version)
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
endif()
