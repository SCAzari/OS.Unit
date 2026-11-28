#include "bus.h"

#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDir>
#include <QLoggingCategory>
#include <QProcessEnvironment>

#include <unistd.h>

Q_LOGGING_CATEGORY(lcBus, "unit.shell.bus")

namespace {
const QString kName = QStringLiteral("unit-session-bus");

// The address of the user's session bus: DBUS_SESSION_BUS_ADDRESS if set, otherwise the
// standard socket in the user's runtime folder. The compositor gives the shell its own runtime
// folder (for the Wayland socket), so XDG_RUNTIME_DIR cannot be used for this.
QString address()
{
    const QString set = qEnvironmentVariable("DBUS_SESSION_BUS_ADDRESS");
    if (!set.isEmpty())
        return set;
    return QStringLiteral("unix:path=/run/user/%1/bus").arg(getuid());
}

// Applications the Session service starts run under the user's service manager, which does not
// know the compositor's socket. Give it the socket as an absolute path (libwayland accepts one in
// WAYLAND_DISPLAY), so the applications the user opens reach the compositor.
void shareWaylandDisplay(const QDBusConnection &bus)
{
    QString display = qEnvironmentVariable("WAYLAND_DISPLAY");
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (display.isEmpty())
        return;
    if (!display.startsWith(u'/') && !runtime.isEmpty())
        display = QDir(runtime).filePath(display);
    auto call = QDBusMessage::createMethodCall(QStringLiteral("org.freedesktop.systemd1"),
                                               QStringLiteral("/org/freedesktop/systemd1"),
                                               QStringLiteral("org.freedesktop.systemd1.Manager"),
                                               QStringLiteral("SetEnvironment"));
    call << QStringList{QStringLiteral("WAYLAND_DISPLAY=") + display, QStringLiteral("XDG_SESSION_TYPE=wayland")};
    const QDBusMessage reply = bus.call(call, QDBus::Block, 5000);
    if (reply.type() == QDBusMessage::ErrorMessage)
        qCWarning(lcBus) << "could not share WAYLAND_DISPLAY with the service manager:" << reply.errorMessage();
    else
        qCInfo(lcBus) << "applications will use WAYLAND_DISPLAY" << display;
}
} // namespace

Bus::Bus(QObject *parent)
    : QObject(parent)
{
    m_retry.setInterval(500);
    connect(&m_retry, &QTimer::timeout, this, &Bus::tryConnect);
    tryConnect();
}

QDBusConnection Bus::connection() const
{
    return QDBusConnection(kName);
}

void Bus::tryConnect()
{
    QDBusConnection bus = QDBusConnection::connectToBus(address(), kName);
    if (!bus.isConnected()) {
        // A failed connection keeps its name; drop it so the next try starts afresh.
        QDBusConnection::disconnectFromBus(kName);
        if (++m_attempts == 1 || m_attempts % 20 == 0)
            qCWarning(lcBus) << "session bus not available yet at" << address() << "- retrying";
        if (!m_retry.isActive())
            m_retry.start();
        return;
    }
    m_retry.stop();
    qCInfo(lcBus) << "connected to the session bus";
    shareWaylandDisplay(bus);
    m_ready = true;
    emit ready();
}
