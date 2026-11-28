// Colour tokens of the Visual Design Specification (VDS) r2.0, and the readability rules that
// apply to the colours a user chooses (VDS §3 and §4; Decision UNIT-OS-DEC-0063). Plain C++ with
// no Qt, so the rules are unit-tested on any machine (tests/palette_test.cpp).
#pragma once

#include <array>
#include <cstdint>
#include <string>

namespace unit {

struct Rgb {
    uint8_t r = 0, g = 0, b = 0;
    bool operator==(const Rgb &o) const { return r == o.r && g == o.g && b == o.b; }
};

// "#RRGGBB" to a colour. Returns false for anything else.
bool parseHex(const std::string &hex, Rgb &out);
std::string toHex(Rgb c);

// WCAG 2.2 relative luminance and contrast ratio.
double luminance(Rgb c);
double contrast(Rgb a, Rgb b);

enum class ThemeId { Daylight, NightShift, Pebble };
enum class Mode { Light, Dark };

// One theme in one mode: the token names are the same for every theme (VDS §4.5).
struct Tokens {
    Rgb background, surface, surface2, text, secondary;
    Rgb defaultAccent, textOnAccent, danger, warning, success;
    Rgb glassTint;
    int glassDefault;   // default glass opacity, percent (VDS gives Daylight's only; used for all)
    int glassUserLimit; // lowest opacity the user can choose (worst case 5:1), percent
};

const Tokens &tokens(ThemeId theme, Mode mode);
ThemeId themeFromName(const std::string &name); // "daylight", "night-shift", "pebble"

// The seven accent choices (VDS §3.1); the first is Daylight's default.
extern const std::array<const char *, 7> kAccents;

// VDS §4.2: accent text must reach 4.5:1 on the background and on solid surfaces. If it does not,
// its HSL lightness is lowered (light mode) or raised (dark mode) in steps of 1%, keeping the hue
// and saturation, until it does. This reproduces every adjusted value in the VDS table.
Rgb adjustAccent(Rgb accent, const Tokens &t, Mode mode);

// Accent soft, the chip behind accent glyphs (UNIT-OS-DEC-0156): the accent mixed onto the solid
// surface, 11.5% in light mode (within one step per channel of the published #E3F0EE for the
// default accent; no simple mix reproduces it exactly) and 16% in dark mode (as published). Then
// the contrast rule: the accent glyph on its chip must reach 3:1 (VDS §4.3), so the mix is
// lessened until it does.
Rgb accentSoft(Rgb accent, const Tokens &t, Mode mode);

// Text on an accent fill: white or the theme's dark ink (its dark-mode text on accent), whichever
// contrasts more (VDS §4.2).
Rgb textOnAccent(Rgb accentFill, ThemeId theme);

// The glass opacity actually drawn: never below the theme's user limit (VDS §3.2, §4.5), and
// solid when Reduce transparency is on.
int glassOpacity(int requested, const Tokens &t, bool reduceTransparency);

// Automatic light/dark (UNIT-OS-DEC-0155): dark from `from` until `until`, in minutes after
// midnight (default 19:00 to 07:00); the period may cross midnight. Equal times mean never dark.
bool automaticIsDark(int minuteOfDay, int from, int until);

// Home greeting (UNIT-OS-DEC-0157): "Good morning" 05:00-11:59, "Good afternoon" 12:00-17:59,
// "Good evening" otherwise. Returns 0 (morning), 1 (afternoon) or 2 (evening).
int greetingFor(int hour);

} // namespace unit
