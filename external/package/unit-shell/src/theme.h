// The design tokens the QML draws with (Visual Design Specification r2.0 §3), worked out from
// the user's appearance settings with the readability rules applied (palette.h). Every colour,
// radius and opacity in the Shell UI comes from here; QML never writes a colour of its own.
#pragma once

#include <QColor>
#include <QObject>
#include <QTimer>

class SettingsClient;

class Theme : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool dark READ dark NOTIFY changed)
    Q_PROPERTY(QString themeName READ themeName NOTIFY changed)
    Q_PROPERTY(QColor background READ background NOTIFY changed)
    Q_PROPERTY(QColor surface READ surface NOTIFY changed)
    Q_PROPERTY(QColor surface2 READ surface2 NOTIFY changed)
    Q_PROPERTY(QColor text READ text NOTIFY changed)
    Q_PROPERTY(QColor secondary READ secondary NOTIFY changed)
    Q_PROPERTY(QColor accent READ accent NOTIFY changed)
    Q_PROPERTY(QColor accentSoft READ accentSoft NOTIFY changed)
    Q_PROPERTY(QColor textOnAccent READ textOnAccent NOTIFY changed)
    Q_PROPERTY(QColor danger READ danger NOTIFY changed)
    Q_PROPERTY(QColor warning READ warning NOTIFY changed)
    Q_PROPERTY(QColor success READ success NOTIFY changed)
    // Glass (VDS §3.2): the tint at the opacity actually drawn, and the hairline edge.
    Q_PROPERTY(QColor glass READ glass NOTIFY changed)
    Q_PROPERTY(QColor glassEdge READ glassEdge NOTIFY changed)
    Q_PROPERTY(int glassOpacity READ glassOpacity NOTIFY changed)
    // Shape (VDS §3.3): the corner rounding and its small, medium and extra large steps.
    Q_PROPERTY(qreal radius READ radius NOTIFY changed)
    Q_PROPERTY(qreal radiusSmall READ radiusSmall NOTIFY changed)
    Q_PROPERTY(qreal radiusMedium READ radiusMedium NOTIFY changed)
    Q_PROPERTY(qreal radiusLarge READ radiusLarge NOTIFY changed)
    Q_PROPERTY(QString iconShape READ iconShape NOTIFY changed)
    Q_PROPERTY(QString fontFamily READ fontFamily NOTIFY changed)
    Q_PROPERTY(bool reduceTransparency READ reduceTransparency NOTIFY changed)
public:
    explicit Theme(SettingsClient *settings, QObject *parent = nullptr);

    // The home greeting for an hour of the day (UNIT-OS-DEC-0157). The user's name is added once
    // a profile service exists.
    Q_INVOKABLE QString greeting(int hour) const;

    bool dark() const { return m_dark; }
    QString themeName() const { return m_themeName; }
    QColor background() const { return m_background; }
    QColor surface() const { return m_surface; }
    QColor surface2() const { return m_surface2; }
    QColor text() const { return m_text; }
    QColor secondary() const { return m_secondary; }
    QColor accent() const { return m_accent; }
    QColor accentSoft() const { return m_accentSoft; }
    QColor textOnAccent() const { return m_textOnAccent; }
    QColor danger() const { return m_danger; }
    QColor warning() const { return m_warning; }
    QColor success() const { return m_success; }
    QColor glass() const { return m_glass; }
    QColor glassEdge() const { return m_glassEdge; }
    int glassOpacity() const { return m_glassOpacity; }
    qreal radius() const { return m_radius; }
    qreal radiusSmall() const { return m_radius * 0.5; }
    qreal radiusMedium() const { return m_radius * 0.75; }
    qreal radiusLarge() const { return m_radius * 1.4; }
    QString iconShape() const { return m_iconShape; }
    QString fontFamily() const { return m_fontFamily; }
    bool reduceTransparency() const { return m_reduceTransparency; }

signals:
    void changed();

private:
    void update();
    bool darkNow() const;
    SettingsClient *m_settings;
    QTimer m_minute;
    bool m_dark = false;
    QString m_themeName;
    QColor m_background, m_surface, m_surface2, m_text, m_secondary, m_accent, m_accentSoft, m_textOnAccent;
    QColor m_danger, m_warning, m_success, m_glass, m_glassEdge;
    int m_glassOpacity = 66;
    qreal m_radius = 20;
    QString m_iconShape;
    QString m_fontFamily;
    bool m_reduceTransparency = false;
};
