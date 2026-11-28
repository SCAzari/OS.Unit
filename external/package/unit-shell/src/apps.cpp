#include "apps.h"

#include "bus.h"

#include <QDBusArgument>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(lcApps, "unit.shell.apps")

namespace {
const QString kService = QStringLiteral("os.unit.Session1");
const QString kPath = QStringLiteral("/os/unit/Session1");
const QString kInterface = QStringLiteral("os.unit.Session1");
} // namespace

AppsModel::AppsModel(Bus *bus, QObject *parent)
    : QAbstractListModel(parent)
    , m_bus(bus)
{
    bus->whenReady(this, [this] { reload(); });
}

int AppsModel::rowCount(const QModelIndex &parent) const
{
    return parent.isValid() ? 0 : int(m_apps.size());
}

QVariant AppsModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() >= m_apps.size())
        return {};
    const AppEntry &a = m_apps.at(index.row());
    switch (role) {
    case IdRole:
        return a.id;
    case Qt::DisplayRole:
    case NameRole:
        return a.name;
    case IconRole:
        return a.icon;
    case CommentRole:
        return a.comment;
    }
    return {};
}

QHash<int, QByteArray> AppsModel::roleNames() const
{
    return {{IdRole, "appId"}, {NameRole, "name"}, {IconRole, "icon"}, {CommentRole, "comment"}};
}

QString AppsModel::nameOf(const QString &appId) const
{
    for (const AppEntry &a : m_apps)
        if (a.id == appId)
            return a.name;
    return {};
}

void AppsModel::reload()
{
    if (!m_bus->isReady())
        return;
    auto call = QDBusMessage::createMethodCall(kService, kPath, kInterface, QStringLiteral("ListApps"));
    auto *watcher = new QDBusPendingCallWatcher(m_bus->connection().asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this](QDBusPendingCallWatcher *w) {
        w->deleteLater();
        const QDBusMessage reply = w->reply();
        if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
            qCWarning(lcApps) << "Session service did not list the applications:" << reply.errorMessage();
            return;
        }
        // a(ssss): ID, name, icon name, comment, already sorted by name.
        QList<AppEntry> apps;
        const QDBusArgument arg = reply.arguments().constFirst().value<QDBusArgument>();
        arg.beginArray();
        while (!arg.atEnd()) {
            AppEntry a;
            arg.beginStructure();
            arg >> a.id >> a.name >> a.icon >> a.comment;
            arg.endStructure();
            apps.append(a);
        }
        arg.endArray();
        beginResetModel();
        m_apps = apps;
        endResetModel();
        emit countChanged();
        qCInfo(lcApps) << m_apps.size() << "applications installed";
    });
}

void AppsModel::open(const QString &appId)
{
    if (!m_bus->isReady()) {
        emit failed(appId, QStringLiteral("not connected to the Session service"));
        return;
    }
    auto call = QDBusMessage::createMethodCall(kService, kPath, kInterface, QStringLiteral("Launch"));
    call << appId;
    auto *watcher = new QDBusPendingCallWatcher(m_bus->connection().asyncCall(call), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, appId](QDBusPendingCallWatcher *w) {
        w->deleteLater();
        QDBusPendingReply<uint> reply = *w;
        if (reply.isError()) {
            qCWarning(lcApps) << "could not open" << appId << ":" << reply.error().message();
            emit failed(appId, reply.error().message());
            return;
        }
        qCInfo(lcApps) << "opened" << appId << "pid" << reply.value();
        emit opened(appId);
    });
}
