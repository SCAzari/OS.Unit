// The session bus the Unit services run on (Software Architecture r2.0 §11.1). The compositor
// starts the Shell UI before the user's service manager may have opened the bus, so the shell
// keeps trying until it connects, then tells the clients (settings.h, apps.h) it is ready.
#pragma once

#include <QDBusConnection>
#include <QObject>
#include <QTimer>

class Bus : public QObject
{
    Q_OBJECT
public:
    explicit Bus(QObject *parent = nullptr);
    bool isReady() const { return m_ready; }
    QDBusConnection connection() const;
    // Runs `f` once the bus is connected (at once if it already is).
    template<typename F>
    void whenReady(QObject *context, F f)
    {
        if (m_ready)
            f();
        else
            connect(this, &Bus::ready, context, f, Qt::SingleShotConnection);
    }

signals:
    void ready();

private:
    void tryConnect();
    QTimer m_retry;
    bool m_ready = false;
    int m_attempts = 0;
};
