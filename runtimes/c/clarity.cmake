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

# The webview draws with WebKitGTK on Linux, WebView2 on Windows and WebKit on macOS.
add_library(clarity_shell STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/shell.c ${CMAKE_CURRENT_LIST_DIR}/shell/webview.cc)
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
