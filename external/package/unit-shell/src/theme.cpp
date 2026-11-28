#include "theme.h"

#include "palette.h"
#include "settings.h"

#include <QTime>

namespace {
QColor qcolor(unit::Rgb c, qreal alpha = 1.0)
{
    QColor q(c.r, c.g, c.b);
    q.setAlphaF(alpha);
    return q;
}

} // namespace

Theme::Theme(SettingsClient *settings, QObject *parent)
    : QObject(parent)
    , m_settings(settings)
{
    connect(settings, &SettingsClient::valuesChanged, this, &Theme::update);
    // Automatic light/dark changes with the time: look again on every minute.
    m_minute.setInterval(60 * 1000);
    connect(&m_minute, &QTimer::timeout, this, [this] {
        if (darkNow() != m_dark)
            update();
    });
    m_minute.start();
    update();
}

QString Theme::greeting(int hour) const
{
    switch (unit::greetingFor(hour)) {
    case 0:
        return tr("Good morning");
    case 1:
        return tr("Good afternoon");
    }
    return tr("Good evening");
}

bool Theme::darkNow() const
{
    // Automatic: dark between two times the user can change, 19:00 to 07:00 by default
    // (UNIT-OS-DEC-0155).
    const QString mode = m_settings->value(QStringLiteral("appearance.mode"), QStringLiteral("light")).toString();
    if (mode != QLatin1String("automatic"))
        return mode == QLatin1String("dark");
    const QTime now = QTime::currentTime();
    const int from = m_settings->value(QStringLiteral("appearance.automatic-dark-from"), 19 * 60).toInt();
    const int until = m_settings->value(QStringLiteral("appearance.automatic-dark-until"), 7 * 60).toInt();
    return unit::automaticIsDark(now.hour() * 60 + now.minute(), from, until);
}

void Theme::update()
{
    // Defaults are the service's own (Daylight, light), used until the service answers.
    const auto str = [this](const char *key, const char *fallback) {
        return m_settings->value(QLatin1String(key), QLatin1String(fallback)).toString();
    };
    const auto num = [this](const char *key, int fallback) {
        return m_settings->value(QLatin1String(key), fallback).toInt();
    };
    m_themeName = str("appearance.theme", "daylight");
    m_dark = darkNow();
    m_reduceTransparency = m_settings->value(QStringLiteral("appearance.reduce-transparency"), false).toBool();

    const unit::ThemeId id = unit::themeFromName(m_themeName.toStdString());
    const unit::Mode mode = m_dark ? unit::Mode::Dark : unit::Mode::Light;
    const unit::Tokens &t = unit::tokens(id, mode);

    unit::Rgb chosen = t.defaultAccent;
    unit::Rgb parsed;
    if (unit::parseHex(str("appearance.accent", "theme").toStdString(), parsed))
        chosen = parsed;
    const unit::Rgb accent = unit::adjustAccent(chosen, t, mode);

    m_background = qcolor(t.background);
    m_surface = qcolor(t.surface);
    m_surface2 = qcolor(t.surface2);
    m_text = qcolor(t.text);
    m_secondary = qcolor(t.secondary);
    m_accent = qcolor(accent);
    m_accentSoft = qcolor(unit::accentSoft(accent, t, mode));
    m_textOnAccent = qcolor(unit::textOnAccent(accent, id));
    m_danger = qcolor(t.danger);
    m_warning = qcolor(t.warning);
    m_success = qcolor(t.success);

    const int requested = num(m_dark ? "appearance.glass-opacity-dark" : "appearance.glass-opacity-light", t.glassDefault);
    m_glassOpacity = unit::glassOpacity(requested, t, m_reduceTransparency);
    m_glass = m_glassOpacity >= 100 ? qcolor(t.surface) : qcolor(t.glassTint, m_glassOpacity / 100.0);
    m_glassEdge = QColor::fromRgbF(1, 1, 1, m_dark ? 0.12 : 0.55);

    m_radius = qBound(0, num("appearance.corner-radius", 20), 32);
    m_iconShape = str("appearance.icon-shape", "rounded");
    // Inter, falling back to Noto Sans CJK JP for Japanese (fontconfig does the fallback). The
    // image installs Inter's variable font, whose family name is "Inter Variable".
    const QString font = str("appearance.font", "Inter");
    m_fontFamily = font == QLatin1String("Inter") ? QStringLiteral("Inter Variable") : font;
    emit changed();
}
