// Unit tests for the colour rules (palette.cpp), against the values the Visual Design
// Specification r1.20 publishes. Plain C++: g++ -std=c++17 palette.cpp tests/palette_test.cpp
#include "../palette.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>

using namespace unit;

static int failures = 0;

static void check(bool ok, const std::string &what)
{
    if (!ok) {
        std::printf("FAIL: %s\n", what.c_str());
        ++failures;
    }
}

static Rgb hex(const char *s)
{
    Rgb c;
    if (!parseHex(s, c)) {
        std::printf("bad colour in test: %s\n", s);
        std::exit(2);
    }
    return c;
}

static void contrastMatches(const char *fg, const char *bg, double expected)
{
    const double got = contrast(hex(fg), hex(bg));
    check(std::fabs(got - expected) < 0.005,
          std::string("contrast ") + fg + " on " + bg + " = " + std::to_string(got));
}

int main()
{
    // VDS §4.1, a sample of the published pairs.
    contrastMatches("#1E2626", "#F4F2EE", 13.81);
    contrastMatches("#566362", "#F4F2EE", 5.60);
    contrastMatches("#ECEFEE", "#1B2121", 14.11);
    contrastMatches("#A3AEAD", "#121616", 8.00);

    // VDS §4.2: every adjusted accent, light and dark, exactly as published.
    struct Row {
        const char *accent, *light, *dark;
    } rows[] = {
        {"#11756B", "#11756B", "#16998C"}, {"#3D6FD8", "#3569D6", "#5B85DE"},
        {"#7A5CF0", "#6F4EEF", "#8D73F2"}, {"#C2416F", "#C03E6C", "#CE688C"},
        {"#D2512F", "#BE4829", "#D76648"}, {"#B7791F", "#946219", "#BB7C20"},
        {"#2E7D4F", "#2E7D4F", "#399B62"},
    };
    for (const Row &r : rows) {
        const std::string light = toHex(adjustAccent(hex(r.accent), tokens(ThemeId::Daylight, Mode::Light), Mode::Light));
        const std::string dark = toHex(adjustAccent(hex(r.accent), tokens(ThemeId::Daylight, Mode::Dark), Mode::Dark));
        check(light == r.light, std::string(r.accent) + " light: " + light + ", expected " + r.light);
        check(dark == r.dark, std::string(r.accent) + " dark: " + dark + ", expected " + r.dark);
    }

    // Every theme, mode and accent (and the theme's own default) ends at 4.5:1 or better.
    for (ThemeId theme : {ThemeId::Daylight, ThemeId::NightShift, ThemeId::Pebble}) {
        for (Mode mode : {Mode::Light, Mode::Dark}) {
            const Tokens &t = tokens(theme, mode);
            auto readable = [&](Rgb a, const std::string &name) {
                const Rgb c = adjustAccent(a, t, mode);
                check(contrast(c, t.background) >= 4.5 && contrast(c, t.surface) >= 4.5, "accent " + name + " readable");
                // Text on the accent fill: the VDS checks reach 5:1 or better for the seven accents.
                check(contrast(textOnAccent(c, theme), c) >= 4.5, "text on accent " + name);
            };
            for (const char *a : kAccents)
                readable(hex(a), a);
            readable(t.defaultAccent, "default " + toHex(t.defaultAccent));
            // Text tokens on the theme's own surfaces.
            check(contrast(t.text, t.background) >= 4.5, "text on background");
            check(contrast(t.secondary, t.surface) >= 4.5, "secondary text on surface");
        }
    }
    // The theme default accents need no adjustment (they are designed to pass).
    check(adjustAccent(hex("#4FC1B3"), tokens(ThemeId::Daylight, Mode::Dark), Mode::Dark) == hex("#4FC1B3"),
          "Daylight dark default accent unchanged");

    // Glass: never below the theme's user limit; solid with Reduce transparency (VDS §3.2, §4.5).
    check(glassOpacity(50, tokens(ThemeId::Daylight, Mode::Light), false) == 60, "Daylight light limit 60");
    check(glassOpacity(68, tokens(ThemeId::Daylight, Mode::Dark), false) == 68, "Daylight dark limit 68");
    check(glassOpacity(68, tokens(ThemeId::NightShift, Mode::Dark), false) == 69, "Night Shift dark limit 69");
    check(glassOpacity(68, tokens(ThemeId::Pebble, Mode::Dark), false) == 70, "Pebble dark limit 70");
    check(glassOpacity(66, tokens(ThemeId::Daylight, Mode::Light), true) == 100, "Reduce transparency is solid");
    check(glassOpacity(150, tokens(ThemeId::Daylight, Mode::Light), false) == 100, "at most solid");

    check(themeFromName("night-shift") == ThemeId::NightShift && themeFromName("pebble") == ThemeId::Pebble &&
              themeFromName("daylight") == ThemeId::Daylight && themeFromName("neon") == ThemeId::Daylight,
          "theme names");
    Rgb dummy;
    check(!parseHex("11756B", dummy) && !parseHex("#11756G", dummy) && !parseHex("theme", dummy), "bad hex refused");

    if (failures == 0)
        std::printf("palette: all checks passed\n");
    return failures == 0 ? 0 : 1;
}
