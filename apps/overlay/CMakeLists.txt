cmake_minimum_required(VERSION 3.22)
project(substream-overlay VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.7 REQUIRED COMPONENTS Core Gui Quick Widgets WebSockets LinguistTools)
find_package(LayerShellQt 6.0 REQUIRED)

add_library(caption-client STATIC
    src/caption_model.cpp
    src/caption_model.h
    src/event_source.cpp
    src/event_source.h
    src/token_file.cpp
    src/token_file.h
)
target_include_directories(caption-client PUBLIC src)
target_link_libraries(caption-client PUBLIC Qt6::Core Qt6::WebSockets)
target_compile_options(caption-client PRIVATE -Wall -Wextra -Wpedantic)

add_library(desktop-session STATIC
    src/audio_devices.cpp src/audio_devices.h
    src/session_controller.cpp src/session_controller.h
    src/video_controller.cpp src/video_controller.h
)
target_link_libraries(desktop-session PUBLIC caption-client)
target_compile_options(desktop-session PRIVATE -Wall -Wextra -Wpedantic)

qt_add_executable(substream-overlay
    src/main.cpp
    src/main_window.cpp src/main_window.h
    src/video_page.cpp src/video_page.h
    src/widgets.cpp src/widgets.h
    src/translations.cpp src/translations.h
    src/kde_window.cpp
    src/kde_window.h
)
qt_add_resources(substream-overlay ui PREFIX "/" FILES qml/Overlay.qml)
target_link_libraries(substream-overlay PRIVATE desktop-session Qt6::Widgets Qt6::Quick LayerShellQt::Interface)
target_compile_options(substream-overlay PRIVATE -Wall -Wextra -Wpedantic)
install(TARGETS substream-overlay RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(FILES substream-overlay.desktop DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)

set(translation_targets substream-overlay)
include(CTest)
if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    qt_add_executable(caption-model-test tests/caption_model_test.cpp)
    target_link_libraries(caption-model-test PRIVATE caption-client Qt6::Test)
    add_test(NAME caption-model COMMAND caption-model-test)
    qt_add_executable(session-peer tests/session_peer.cpp)
    target_link_libraries(session-peer PRIVATE Qt6::Core)
    qt_add_executable(session-controller-test tests/session_controller_test.cpp)
    target_link_libraries(session-controller-test PRIVATE desktop-session Qt6::Test)
    add_dependencies(session-controller-test session-peer)
    add_test(NAME session-controller COMMAND session-controller-test)
    qt_add_executable(translations-test tests/translations_test.cpp src/translations.cpp src/translations.h)
    target_link_libraries(translations-test PRIVATE desktop-session Qt6::Test)
    list(APPEND translation_targets translations-test)
    add_test(NAME translations COMMAND translations-test)
endif()

qt_add_translations(
    TARGETS ${translation_targets}
    SOURCE_TARGETS substream-overlay caption-client desktop-session
    TS_FILES i18n/substream_zh_CN.ts
    PLURALS_TS_FILE i18n/substream_en.ts
    LUPDATE_OPTIONS -no-obsolete
)
