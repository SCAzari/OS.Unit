#include "palette.h"

#include <algorithm>
#include <cmath>
#include <cstdio>

namespace unit {

namespace {

constexpr Rgb rgb(uint32_t v) { return Rgb{uint8_t(v >> 16), uint8_t(v >> 8), uint8_t(v)}; }

// VDS §3.1 (Daylight) and §4.5 (Night Shift, Pebble). Glass tints from §3.2 and §4.5; user
// limits from §3.2 and the §4.5 check table.
const Tokens kDaylightLight{rgb(0xF4F2EE), rgb(0xFFFFFF), rgb(0xF0EEEA), rgb(0x1E2626), rgb(0x566362),
                            rgb(0x11756B), rgb(0xFFFFFF), rgb(0xB3261E), rgb(0x8A5A00), rgb(0x2E7D4F),
                            rgb(0xFAF8F4), 66, 60};
const Tokens kDaylightDark{rgb(0x121616), rgb(0x1B2121), rgb(0x232A2A), rgb(0xECEFEE), rgb(0xA3AEAD),
                           rgb(0x4FC1B3), rgb(0x0B1514), rgb(0xFF8A80), rgb(0xF2C14E), rgb(0x7BD39A),
                           rgb(0x161C1C), 72, 68};
const Tokens kNightShiftLight{rgb(0xF2EADC), rgb(0xFBF6EC), rgb(0xEEE4D2), rgb(0x2A2017), rgb(0x665443),
                              rgb(0x8F5208), rgb(0xFFFFFF), rgb(0xA8321F), rgb(0x7D5200), rgb(0x3E6E2E),
                              rgb(0xFAF4E8), 66, 60};
const Tokens kNightShiftDark{rgb(0x16110C), rgb(0x211A13), rgb(0x2B2219), rgb(0xF2E6D5), rgb(0xB8A58E),
                             rgb(0xE2A45A), rgb(0x1B1209), rgb(0xFF9A85), rgb(0xF0C060), rgb(0x9CCB86),
                             rgb(0x1E1710), 72, 69};
const Tokens kPebbleLight{rgb(0xEDEEEF), rgb(0xFAFAFB), rgb(0xE8E9EB), rgb(0x202326), rgb(0x585D63),
                          rgb(0x46596E), rgb(0xFFFFFF), rgb(0xB3261E), rgb(0x855700), rgb(0x2E7346),
                          rgb(0xF6F7F8), 66, 60};
const Tokens kPebbleDark{rgb(0x17181A), rgb(0x212326), rgb(0x2A2D31), rgb(0xE8EAEC), rgb(0xA4A9AF),
                         rgb(0x9DB2CA), rgb(0x111417), rgb(0xFF8A80), rgb(0xF2C14E), rgb(0x86CFA0),
                         rgb(0x1B1D20), 72, 70};

double channel(uint8_t v)
{
    const double c = v / 255.0;
    return c <= 0.04045 ? c / 12.92 : std::pow((c + 0.055) / 1.055, 2.4);
}

// HLS as defined by Python's colorsys, which the VDS checker used.
struct Hls {
    double h, l, s;
};

Hls toHls(Rgb c)
{
    const double r = c.r / 255.0, g = c.g / 255.0, b = c.b / 255.0;
    const double maxc = std::max({r, g, b}), minc = std::min({r, g, b});
    const double l = (minc + maxc) / 2.0;
    if (maxc == minc)
        return {0.0, l, 0.0};
    const double s = l <= 0.5 ? (maxc - minc) / (maxc + minc) : (maxc - minc) / (2.0 - maxc - minc);
    const double rc = (maxc - r) / (maxc - minc), gc = (maxc - g) / (maxc - minc), bc = (maxc - b) / (maxc - minc);
    double h;
    if (r == maxc)
        h = bc - gc;
    else if (g == maxc)
        h = 2.0 + rc - bc;
    else
        h = 4.0 + gc - rc;
    h = std::fmod(h / 6.0, 1.0);
    if (h < 0)
        h += 1.0;
    return {h, l, s};
}

double hueValue(double m1, double m2, double hue)
{
    hue = std::fmod(hue, 1.0);
    if (hue < 0)
        hue += 1.0;
    if (hue < 1.0 / 6.0)
        return m1 + (m2 - m1) * hue * 6.0;
    if (hue < 0.5)
        return m2;
    if (hue < 2.0 / 3.0)
        return m1 + (m2 - m1) * (2.0 / 3.0 - hue) * 6.0;
    return m1;
}

uint8_t toByte(double v)
{
    // Round half to even, as the checker did, and keep within range.
    return uint8_t(std::clamp(std::nearbyint(v * 255.0), 0.0, 255.0));
}

Rgb fromHls(Hls c)
{
    if (c.s == 0.0)
        return Rgb{toByte(c.l), toByte(c.l), toByte(c.l)};
    const double m2 = c.l <= 0.5 ? c.l * (1.0 + c.s) : c.l + c.s - c.l * c.s;
    const double m1 = 2.0 * c.l - m2;
    return Rgb{toByte(hueValue(m1, m2, c.h + 1.0 / 3.0)), toByte(hueValue(m1, m2, c.h)),
               toByte(hueValue(m1, m2, c.h - 1.0 / 3.0))};
}

double worstContrast(Rgb c, const Tokens &t)
{
    return std::min(contrast(c, t.background), contrast(c, t.surface));
}

} // namespace

const std::array<const char *, 7> kAccents = {"#11756B", "#3D6FD8", "#7A5CF0", "#C2416F",
                                              "#D2512F", "#B7791F", "#2E7D4F"};

bool parseHex(const std::string &hex, Rgb &out)
{
    if (hex.size() != 7 || hex[0] != '#')
        return false;
    unsigned v = 0;
    for (size_t i = 1; i < 7; ++i) {
        const char ch = hex[i];
        unsigned d;
        if (ch >= '0' && ch <= '9')
            d = ch - '0';
        else if (ch >= 'a' && ch <= 'f')
            d = ch - 'a' + 10;
        else if (ch >= 'A' && ch <= 'F')
            d = ch - 'A' + 10;
        else
            return false;
        v = v * 16 + d;
    }
    out = rgb(v);
    return true;
}

std::string toHex(Rgb c)
{
    char buf[8];
    std::snprintf(buf, sizeof buf, "#%02X%02X%02X", c.r, c.g, c.b);
    return buf;
}

double luminance(Rgb c)
{
    return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
}

double contrast(Rgb a, Rgb b)
{
    const double la = luminance(a), lb = luminance(b);
    return (std::max(la, lb) + 0.05) / (std::min(la, lb) + 0.05);
}

const Tokens &tokens(ThemeId theme, Mode mode)
{
    const bool dark = mode == Mode::Dark;
    switch (theme) {
    case ThemeId::NightShift:
        return dark ? kNightShiftDark : kNightShiftLight;
    case ThemeId::Pebble:
        return dark ? kPebbleDark : kPebbleLight;
    case ThemeId::Daylight:
        break;
    }
    return dark ? kDaylightDark : kDaylightLight;
}

ThemeId themeFromName(const std::string &name)
{
    if (name == "night-shift")
        return ThemeId::NightShift;
    if (name == "pebble")
        return ThemeId::Pebble;
    return ThemeId::Daylight;
}

Rgb adjustAccent(Rgb accent, const Tokens &t, Mode mode)
{
    const Hls start = toHls(accent);
    Rgb c = accent;
    // 100 steps cover the whole lightness range; the loop always ends at black or white, which
    // pass against every theme's background and surface.
    for (int i = 1; i <= 100 && worstContrast(c, t) < 4.5; ++i) {
        Hls h = start;
        h.l = std::clamp(mode == Mode::Dark ? start.l + 0.01 * i : start.l - 0.01 * i, 0.0, 1.0);
        c = fromHls(h);
    }
    return c;
}

Rgb textOnAccent(Rgb accentFill, ThemeId theme)
{
    const Rgb white{255, 255, 255};
    const Rgb ink = tokens(theme, Mode::Dark).textOnAccent;
    return contrast(white, accentFill) >= contrast(ink, accentFill) ? white : ink;
}

int glassOpacity(int requested, const Tokens &t, bool reduceTransparency)
{
    if (reduceTransparency)
        return 100;
    return std::clamp(requested, t.glassUserLimit, 100);
}

} // namespace unit
