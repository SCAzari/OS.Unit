# Shell UI toolkit spike (UNIT-OS-WP-0010)

Spike branch only (`spike/toolkit`), never merged (issue #8, answer 8i). It builds two Shell UI
screens in Qt 6 QML and in Slint and tests the four pass conditions of Software Architecture
r1.12 §5.2 (UNIT-OS-DEC-0058, UNIT-OS-DEC-0117): screen reader, Japanese input through Fcitx5,
touch response (measured, D-06 not set) and switch-scanning order.

- `screens.json`: the shared screen specification. Both toolkit versions implement it, and the
  probe checks each version against it: every element with its accessibility role, name and
  value, and the declared switch-scanning order.
- Toolkit versions: `external/package/unit-spike-qt` and `external/package/unit-spike-slint`.
- Test tools (reference reader, scanner): `external/package/unit-spike-probe`.
- Image: `make UNIT_IMAGE=unit_qemu_x86_64_spike`; tests: `make UNIT_IMAGE=unit_qemu_x86_64_spike spike`.

Screens (Interface Design r1.20, Visual Design Specification r1.5):
- `sign-in`: SC13 Sign in, used as the lock screen (8a), after mock-up P01.
- `settings`: SC04 Settings home from the wireframe, with a test-only search field for the
  Japanese input test (8c: the wireframe has none; flagged in the results), and the Display and
  appearance page after mock-up P07 (8b).

Content is illustrative (no real sign-in; settings change only the screen). Icons are
placeholders (VDS §10: the icon set is not drawn yet).
