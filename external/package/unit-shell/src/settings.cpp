#include "settings.h"

#include "bus.h"

#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcSettings, "unit.shell.settings")

namespace {
const QString kService = QStringLiteral("os.unit.Settings1");
const QString kPath = QStringLiteral("/os/unit/Settings1");
const QString kInterface = QStringLiteral("os.unit.Settings1");
} // namespace

SettingsClient::SettingsClient(Bus *bus, QObject *parent)
    : QObject(parent)
    , m_bus(bus)
{
    bus->whenReady(this, [this] {
        m_bus->connection().connect(kService, kPath, kInterface, QStringLiteral("Changed"), this,
                                    SLOT(onChanged(QString, QDBusVariant)));
        load();
    });
}

void SettingsClient::load()
{
    auto call = QDBusMessage::createMethodCall(kService, kPath, kInterface, QStringLiteral("GetAll"));
    auto *watcher = new QDBusPendingCallWatcher(m_bus->connection().asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this](QDBusPendingCallWatcher *w) {
        w->deleteLater();
        QDBusPendingReply<QVariantMap> reply = *w;
        if (reply.isError()) {
            qCWarning(lcSettings) << "Settings service did not answer:" << reply.error().message();
            return;
        }
        m_values = reply.value();
        qCInfo(lcSettings) << "loaded" << m_values.size() << "settings";
        emit valuesChanged();
        if (!m_loaded) {
            m_loaded = true;
            emit loadedChanged();
        }
    });
}

void SettingsClient::set(const QString &key, const QVariant &value)
{
    if (!m_bus->isReady())
        return;
    auto call = QDBusMessage::createMethodCall(kService, kPath, kInterface, QStringLiteral("Set"));
    call << key << QVariant::fromValue(QDBusVariant(value));
    auto *watcher = new QDBusPendingCallWatcher(m_bus->connection().asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [key](QDBusPendingCallWatcher *w) {
        w->deleteLater();
        QDBusPendingReply<> reply = *w;
        if (reply.isError())
            qCWarning(lcSettings) << "setting" << key << "refused:" << reply.error().name() << reply.error().message();
    });
}

void SettingsClient::onChanged(const QString &key, const QDBusVariant &value)
{
    m_values.insert(key, value.variant());
    emit valuesChanged();
}
