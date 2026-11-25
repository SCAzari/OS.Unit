# Toolkit spike (UNIT-OS-WP-0010): sign-in and settings screens in Qt 6 QML.
cmake_minimum_required(VERSION 3.21)
project(unit-spike-qt VERSION 0.1.0 LANGUAGES CXX)
find_package(Qt6 REQUIRED COMPONENTS Gui Quick QuickControls2)
qt_standard_project_setup(REQUIRES 6.5)

qt_add_executable(unit-spike-qt main.cpp)
set_source_files_properties(Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(unit-spike-qt
    URI UnitSpike
    VERSION 1.0
    QML_FILES
        Theme.qml Main.qml StatusBar.qml Glass.qml FocusRing.qml
        KeyButton.qml PillButton.qml SettingsRow.qml SearchField.qml
        Choice.qml Swatch.qml UnitSlider.qml NavBar.qml
        SignIn.qml SettingsHome.qml DisplayAppearance.qml ControlsCheck.qml
)
target_link_libraries(unit-spike-qt PRIVATE Qt6::Gui Qt6::Quick Qt6::QuickControls2)
install(TARGETS unit-spike-qt RUNTIME DESTINATION bin)
