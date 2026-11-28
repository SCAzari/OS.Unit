// Unit Shell UI (UNIT-OS-WP-0009): the status bar, navigation bar and home screen, drawn as
// layer surfaces of the Unit compositor. The compositor starts it with a private connection
// (WAYLAND_SOCKET) that alone may create layer surfaces, and restarts it if it fails
// (Executive decision, 26 Nov 2026). It is a view over the Unit services (Software Architecture
// r2.0 §4): it keeps no state that must survive a restart.
#include "apps.h"
#include "bus.h"
#include "settings.h"
#include "theme.h"

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QtQml>

int main(int argc, char *argv[])
{
    // Every window of the shell is a layer surface (layer-shell-qt), on the Wayland platform.
    qputenv("QT_QPA_PLATFORM", "wayland");
    qputenv("QT_WAYLAND_SHELL_INTEGRATION", "layer-shell");
    // Keep the AT-SPI bridge on before a screen reader asks for it. Readers and probes use
    // Properties.Get, never GetAll (UNIT-OS-DEC-0122 item 3).
    qputenv("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1");

    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("unit-shell"));
    app.setOrganizationName(QStringLiteral("Unit"));

    Bus bus;
    SettingsClient settings(&bus);
    Theme theme(&settings);
    AppsModel apps(&bus);

    qmlRegisterSingletonInstance("Unit.Services", 1, 0, "Settings", &settings);
    qmlRegisterSingletonInstance("Unit.Services", 1, 0, "Theme", &theme);
    qmlRegisterSingletonInstance("Unit.Services", 1, 0, "Apps", &apps);

    QQmlApplicationEngine engine;
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreationFailed, &app,
                     [] { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.loadFromModule("Unit.Shell", "Shell");
    return app.exec();
}
