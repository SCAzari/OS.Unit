# Toolkit spike (UNIT-OS-WP-0010): sign-in and settings screens in Qt 6 QML.
cmake_minimum_required(VERSION 3.21)
project(unit-spike-qt VERSION 0.1.0 LANGUAGES CXX)
find_package(Qt6 REQUIRED COMPONENTS Gui Quick)
qt_standard_project_setup(REQUIRES 6.5)

qt_add_executable(unit-spike-qt main.cpp)
set_source_files_properties(qml/Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(unit-spike-qt
    URI UnitSpike
    VERSION 1.0
    QML_FILES
        qml/Theme.qml qml/Main.qml qml/StatusBar.qml qml/Glass.qml qml/FocusRing.qml
        qml/KeyButton.qml qml/PillButton.qml qml/SettingsRow.qml qml/SearchField.qml
        qml/Choice.qml qml/Swatch.qml qml/UnitSlider.qml qml/NavBar.qml
        qml/SignIn.qml qml/SettingsHome.qml qml/DisplayAppearance.qml
)
target_link_libraries(unit-spike-qt PRIVATE Qt6::Gui Qt6::Quick)
install(TARGETS unit-spike-qt RUNTIME DESTINATION bin)
