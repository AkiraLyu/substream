cmake_minimum_required(VERSION 3.22)
project(substream-overlay VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Quick WebSockets)
find_package(LayerShellQt 6.0 REQUIRED)

add_library(caption-client STATIC
    src/caption_model.cpp
    src/caption_model.h
    src/event_source.cpp
    src/event_source.h
)
target_include_directories(caption-client PUBLIC src)
target_link_libraries(caption-client PUBLIC Qt6::Core Qt6::WebSockets)
target_compile_options(caption-client PRIVATE -Wall -Wextra -Wpedantic)

qt_add_executable(substream-overlay
    src/main.cpp
    src/kde_window.cpp
    src/kde_window.h
)
qt_add_resources(substream-overlay ui PREFIX "/" FILES qml/Overlay.qml)
target_link_libraries(substream-overlay PRIVATE caption-client Qt6::Quick LayerShellQt::Interface)
target_compile_options(substream-overlay PRIVATE -Wall -Wextra -Wpedantic)
install(TARGETS substream-overlay RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(FILES substream-overlay.desktop DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)

include(CTest)
if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    qt_add_executable(caption-model-test tests/caption_model_test.cpp)
    target_link_libraries(caption-model-test PRIVATE caption-client Qt6::Test)
    add_test(NAME caption-model COMMAND caption-model-test)
endif()
