// Toolkit spike (UNIT-OS-WP-0010): Qt 6 QML version of the spike screens.
//   unit-spike-qt sign-in | settings
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>

int main(int argc, char *argv[])
{
    // Keep the AT-SPI bridge on even before a reader asks for it (the probe connects later).
    qputenv("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1");
    QGuiApplication app(argc, argv);
    app.setApplicationName("unit-spike-qt");
    const QString screen = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QStringLiteral("sign-in");
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("startScreen", screen);
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreationFailed, &app,
                     [] { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.loadFromModule("UnitSpike", "Main");
    return app.exec();
}
