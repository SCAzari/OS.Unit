// Unit tests for the colour rules (palette.cpp), against the values the Visual Design
// Specification r2.0 publishes. Plain C++: g++ -std=c++17 palette.cpp tests/palette_test.cpp
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

    // Accent soft (UNIT-OS-DEC-0156): the default accent's chip is within one step of the published
    // #E3F0EE, dark is the published 16% mix, and every accent's glyph reaches 3:1 on its chip.
    {
        const Rgb soft = accentSoft(hex("#11756B"), tokens(ThemeId::Daylight, Mode::Light), Mode::Light);
        const Rgb published = hex("#E3F0EE");
        check(std::abs(soft.r - published.r) <= 1 && std::abs(soft.g - published.g) <= 1 && std::abs(soft.b - published.b) <= 1,
              "Daylight light accent soft " + toHex(soft) + " near #E3F0EE");
        for (ThemeId theme : {ThemeId::Daylight, ThemeId::NightShift, ThemeId::Pebble})
            for (Mode mode : {Mode::Light, Mode::Dark}) {
                const Tokens &tk = tokens(theme, mode);
                for (const char *a : kAccents) {
                    const Rgb acc = adjustAccent(hex(a), tk, mode);
                    check(contrast(acc, accentSoft(acc, tk, mode)) >= 3.0, std::string("accent glyph on chip ") + a);
                }
            }
    }

    // Automatic light/dark (UNIT-OS-DEC-0155), default 19:00 to 07:00.
    check(automaticIsDark(19 * 60, 1140, 420) && automaticIsDark(23 * 60, 1140, 420) && automaticIsDark(0, 1140, 420)
              && automaticIsDark(6 * 60 + 59, 1140, 420),
          "dark from 19:00 to 06:59");
    check(!automaticIsDark(7 * 60, 1140, 420) && !automaticIsDark(12 * 60, 1140, 420) && !automaticIsDark(18 * 60 + 59, 1140, 420),
          "light from 07:00 to 18:59");
    check(automaticIsDark(10 * 60, 9 * 60, 17 * 60) && !automaticIsDark(8 * 60, 9 * 60, 17 * 60), "period within a day");
    check(!automaticIsDark(10 * 60, 600, 600), "equal times: never dark");

    // Greeting (UNIT-OS-DEC-0157).
    check(greetingFor(4) == 2 && greetingFor(5) == 0 && greetingFor(11) == 0 && greetingFor(12) == 1
              && greetingFor(17) == 1 && greetingFor(18) == 2 && greetingFor(23) == 2,
          "greeting times");

    check(themeFromName("night-shift") == ThemeId::NightShift && themeFromName("pebble") == ThemeId::Pebble &&
              themeFromName("daylight") == ThemeId::Daylight && themeFromName("neon") == ThemeId::Daylight,
          "theme names");
    Rgb dummy;
    check(!parseHex("11756B", dummy) && !parseHex("#11756G", dummy) && !parseHex("theme", dummy), "bad hex refused");

    if (failures == 0)
        std::printf("palette: all checks passed\n");
    return failures == 0 ? 0 : 1;
}
