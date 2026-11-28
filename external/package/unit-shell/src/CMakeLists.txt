# Unit Shell UI (UNIT-OS-WP-0009): Qt 6 QML (UNIT-OS-DEC-0122) with C++ glue to the Unit
# services over D-Bus (Software Architecture r2.0 §5, option B).
cmake_minimum_required(VERSION 3.21)
project(unit-shell VERSION 0.1.0 LANGUAGES CXX)

find_package(Qt6 6.9 REQUIRED COMPONENTS Core Gui Qml Quick DBus)
qt_standard_project_setup(REQUIRES 6.9)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# The colour rules, without Qt, so they are tested on any machine.
add_library(unit-palette STATIC palette.cpp)
target_include_directories(unit-palette PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

qt_add_executable(unit-shell main.cpp bus.cpp settings.cpp theme.cpp apps.cpp)
target_compile_options(unit-shell PRIVATE -Wall -Wextra)
# QML files sit beside the C++ (not in a subfolder), so the module's own singleton is not shadowed
# by an implicit import of the subfolder, as found in the toolkit spike.
set_source_files_properties(Simulated.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(unit-shell
    URI Unit.Shell
    VERSION 1.0
    QML_FILES
        Shell.qml
        StatusBar.qml
        NavigationBar.qml
        Home.qml
        AppTile.qml
        FocusRing.qml
        Glyph.qml
        Simulated.qml
)
target_link_libraries(unit-shell PRIVATE unit-palette Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::DBus)
install(TARGETS unit-shell RUNTIME DESTINATION bin)

include(CTest)
if(BUILD_TESTING)
    add_executable(palette_test tests/palette_test.cpp)
    target_link_libraries(palette_test PRIVATE unit-palette)
    add_test(NAME palette COMMAND palette_test)
endif()
