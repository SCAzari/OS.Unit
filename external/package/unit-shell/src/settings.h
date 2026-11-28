// Client of the Settings service (os.unit.Settings1, version 1). The Shell UI holds no settings
// of its own: it shows the service's values and asks the service to change them, and follows
// the service's Changed signal, so a change made anywhere shows everywhere.
#pragma once

#include <QDBusVariant>
#include <QObject>
#include <QVariantMap>

class Bus;

class SettingsClient : public QObject
{
    Q_OBJECT
    // Every setting by key, as the service last reported it. Empty until the service answers.
    Q_PROPERTY(QVariantMap values READ values NOTIFY valuesChanged)
    Q_PROPERTY(bool loaded READ loaded NOTIFY loadedChanged)
public:
    explicit SettingsClient(Bus *bus, QObject *parent = nullptr);
    QVariantMap values() const { return m_values; }
    bool loaded() const { return m_loaded; }
    QVariant value(const QString &key, const QVariant &fallback = {}) const { return m_values.value(key, fallback); }

    // Asks the service to change a setting. The value shown changes when the service confirms
    // it (Changed); a refused value (out of range, not an option) is logged and nothing changes.
    Q_INVOKABLE void set(const QString &key, const QVariant &value);

signals:
    void valuesChanged();
    void loadedChanged();

private slots:
    void onChanged(const QString &key, const QDBusVariant &value);

private:
    void load();
    Bus *m_bus;
    QVariantMap m_values;
    bool m_loaded = false;
};
