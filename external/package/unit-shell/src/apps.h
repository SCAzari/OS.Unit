// The installed applications, from the Session service (os.unit.Session1, version 1), for the
// home grid, the dock and the launcher. Opening one asks the Session service to start it: the
// Shell UI never starts programs itself.
#pragma once

#include <QAbstractListModel>
#include <QList>

class Bus;

struct AppEntry {
    QString id, name, icon, comment;
};

class AppsModel : public QAbstractListModel
{
    Q_OBJECT
    Q_PROPERTY(int count READ rowCount NOTIFY countChanged)
public:
    enum Role { IdRole = Qt::UserRole + 1, NameRole, IconRole, CommentRole };
    explicit AppsModel(Bus *bus, QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = {}) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    // The name of an application by its ID, or an empty string if it is not installed.
    Q_INVOKABLE QString nameOf(const QString &appId) const;
    // Asks the Session service to start the application. `failed` reports a refusal.
    Q_INVOKABLE void open(const QString &appId);
    Q_INVOKABLE void reload();

signals:
    void countChanged();
    void opened(const QString &appId);
    void failed(const QString &appId, const QString &reason);

private:
    Bus *m_bus;
    QList<AppEntry> m_apps;
};
