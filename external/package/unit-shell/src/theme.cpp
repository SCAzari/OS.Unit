#include "theme.h"

#include "palette.h"
#include "settings.h"

namespace {
QColor qcolor(unit::Rgb c, qreal alpha = 1.0)
{
    QColor q(c.r, c.g, c.b);
    q.setAlphaF(alpha);
    return q;
}

// Accent soft (VDS §3.1): a light tint of the accent behind accent glyphs. The VDS gives it for
// the default accent only (#E3F0EE light, the accent at 16% dark); for every accent it is the
// accent at 12% over the solid surface in light mode, and at 16% in dark mode.
QColor softAccent(unit::Rgb accent, unit::Rgb surface, bool dark)
{
    if (dark)
        return qcolor(accent, 0.16);
    const double a = 0.12;
    auto mix = [a](uint8_t fg, uint8_t bg) { return int(fg * a + bg * (1 - a) + 0.5); };
    return QColor(mix(accent.r, surface.r), mix(accent.g, surface.g), mix(accent.b, surface.b));
}
} // namespace

Theme::Theme(SettingsClient *settings, QObject *parent)
    : QObject(parent)
    , m_settings(settings)
{
    connect(settings, &SettingsClient::valuesChanged, this, &Theme::update);
    update();
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
    // "automatic" is not yet defined in the design (when it switches); until it is, it shows light.
    m_dark = str("appearance.mode", "light") == QLatin1String("dark");
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
    m_accentSoft = softAccent(accent, t.surface, m_dark);
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
