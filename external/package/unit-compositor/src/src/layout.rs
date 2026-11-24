//! Layout classes and display density (Requirement OS-52 - adaptive interface; Interface Design
//! §5.1). Sizes are in device-independent points; the class follows the layout width.
//!
//! Desktop Foundation - Increment 1 builds only the compact layout. The class is still worked out
//! from the display here, so medium, expanded and VR can be added later without changing how
//! the compositor sizes and scales the display.

use std::fmt;

/// Layout class by available width in points: compact under 600, medium 600 to 839,
/// expanded 840 and over. VR is handled by the VR shell, not by a width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    Compact,
    Medium,
    Expanded,
}

impl LayoutClass {
    pub fn for_width(points: i32) -> Self {
        if points < 600 {
            LayoutClass::Compact
        } else if points < 840 {
            LayoutClass::Medium
        } else {
            LayoutClass::Expanded
        }
    }
}

impl fmt::Display for LayoutClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LayoutClass::Compact => "compact",
            LayoutClass::Medium => "medium",
            LayoutClass::Expanded => "expanded",
        })
    }
}

/// Built-in handheld screens use 160 pixels per inch as one point; screens viewed from further
/// away (laptops, monitors, TVs) use 96.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    BuiltInHandheld,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Density {
    pub factor: f64,
    pub source: DensitySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensitySource {
    /// Exact factor from the Unit device description.
    DeviceDescription,
    /// Calculated from the reported size, rounded to 0.125.
    ReportedSize,
    /// Reported size missing or unreliable: 1.0 for external screens.
    Fallback,
}

/// Device-family factor for built-in screens whose size cannot be read: the phone-size Unit
/// screen, 1080 pixels at 412 points (Interface Design §5.1).
pub const UNIT_PHONE_FACTOR: f64 = 1080.0 / 412.0;

/// Pixels per inch accepted as a real screen density, from a reported size or a device description.
const PPI_RANGE: std::ops::RangeInclusive<f64> = 60.0..=600.0;
/// Pixels per inch that make one point on a built-in handheld screen.
const HANDHELD_PPI_PER_POINT: f64 = 160.0;

/// Whether a device description's density factor is within the densities accepted from a reported
/// size (60 to 600 pixels per inch on a handheld screen: 0.375 to 3.75). Anything else, including
/// infinity and NaN, is ignored, so a mistyped description cannot shrink the display to nothing.
pub fn valid_density_factor(factor: f64) -> bool {
    factor.is_finite()
        && (*PPI_RANGE.start() / HANDHELD_PPI_PER_POINT..=*PPI_RANGE.end() / HANDHELD_PPI_PER_POINT)
            .contains(&factor)
}

/// Density factor of a display (Interface Design §5.1).
pub fn density(
    width_px: i32,
    height_px: i32,
    size_mm: (u32, u32),
    kind: ScreenKind,
    device_description: Option<f64>,
) -> Density {
    if kind == ScreenKind::BuiltInHandheld {
        if let Some(factor) = device_description {
            return Density { factor, source: DensitySource::DeviceDescription };
        }
    }
    if let Some(ppi) = reliable_ppi(width_px, height_px, size_mm) {
        let reference = match kind {
            ScreenKind::BuiltInHandheld => HANDHELD_PPI_PER_POINT,
            ScreenKind::External => 96.0,
        };
        let factor = (ppi / reference * 8.0).round() / 8.0;
        return Density { factor: factor.max(0.125), source: DensitySource::ReportedSize };
    }
    let factor = match kind {
        ScreenKind::BuiltInHandheld => UNIT_PHONE_FACTOR,
        ScreenKind::External => 1.0,
    };
    Density { factor, source: DensitySource::Fallback }
}

/// Pixels per inch from the reported size, or None when the size is missing or unreliable:
/// zero; density below 60 or above 600 pixels per inch; diagonal under 4 or over 120 inches;
/// width-to-height ratio more than 10% from the pixel ratio; or a recognised placeholder.
fn reliable_ppi(width_px: i32, height_px: i32, (w_mm, h_mm): (u32, u32)) -> Option<f64> {
    if w_mm == 0 || h_mm == 0 || width_px <= 0 || height_px <= 0 {
        return None;
    }
    if is_placeholder_size(w_mm, h_mm) {
        return None;
    }
    let (w, h) = (w_mm as f64, h_mm as f64);
    let pixel_ratio = width_px as f64 / height_px as f64;
    if ((w / h) / pixel_ratio - 1.0).abs() > 0.10 {
        return None;
    }
    let diagonal_in = (w * w + h * h).sqrt() / 25.4;
    if !(4.0..=120.0).contains(&diagonal_in) {
        return None;
    }
    let diagonal_px = ((width_px as f64).powi(2) + (height_px as f64).powi(2)).sqrt();
    let ppi = diagonal_px / diagonal_in;
    PPI_RANGE.contains(&ppi).then_some(ppi)
}

/// Recognised placeholder sizes (Issue UNIT-OS-ISS-0019, confirmed by the Senior User on
/// 22 November 2026). Some displays report their aspect ratio in whole centimetres instead of
/// their physical size. A reported size, in either orientation, of exactly 16 x 9, 16 x 10,
/// 21 x 9, 32 x 9 or 4 x 3 cm is treated as missing, and sizing follows the fallback. Any other
/// size, including genuine widescreen and ultrawide sizes, is checked by the other rules.
pub const PLACEHOLDER_SIZES_MM: &[(u32, u32)] = &[(160, 90), (160, 100), (210, 90), (320, 90), (40, 30)];

pub fn is_placeholder_size(w_mm: u32, h_mm: u32) -> bool {
    PLACEHOLDER_SIZES_MM
        .iter()
        .any(|&(a, b)| (w_mm, h_mm) == (a, b) || (w_mm, h_mm) == (b, a))
}

/// Layout width in points: pixels / (density factor x display scaling), rounded down.
pub fn layout_width(width_px: i32, factor: f64, scaling: f64) -> i32 {
    // Whole points, rounded down; the small allowance stops a factor given to limited decimal
    // places (720/412 written as 1.74757281553) losing a point to floating-point error.
    (width_px as f64 / (factor * scaling) + 1e-6).floor() as i32
}

/// Settings read from /etc/unit/display.conf (key=value lines): `density_factor` is the device
/// description's exact factor for the built-in screen (ignored outside 0.375 to 3.75);
/// `display_scaling` is the user's scaling, 1.0 to 2.0 (ignored if not a number).
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayConfig {
    pub density_factor: Option<f64>,
    pub display_scaling: f64,
    /// Connector types that are built-in handheld screens (default eDP, DSI and LVDS). The
    /// emulation image adds Virtual, because its virtual display stands for the Unit screen.
    pub built_in_connectors: Vec<String>,
}

impl DisplayConfig {
    pub fn screen_kind(&self, connector_type: &str) -> ScreenKind {
        if self.built_in_connectors.iter().any(|c| c.eq_ignore_ascii_case(connector_type)) {
            ScreenKind::BuiltInHandheld
        } else {
            ScreenKind::External
        }
    }

    pub fn load() -> Self {
        let text = std::fs::read_to_string("/etc/unit/display.conf").unwrap_or_default();
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Self {
        let mut config = DisplayConfig {
            density_factor: None,
            display_scaling: 1.0,
            built_in_connectors: vec!["eDP".into(), "DSI".into(), "LVDS".into()],
        };
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else { continue };
            if key.trim() == "built_in_connectors" {
                config.built_in_connectors =
                    value.split(',').map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).collect();
                continue;
            }
            let Ok(value) = value.trim().parse::<f64>() else { continue };
            if !value.is_finite() {
                continue;
            }
            match key.trim() {
                "density_factor" if valid_density_factor(value) => config.density_factor = Some(value),
                "display_scaling" => config.display_scaling = value.clamp(1.0, 2.0),
                _ => {}
            }
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints() {
        assert_eq!(LayoutClass::for_width(412), LayoutClass::Compact);
        assert_eq!(LayoutClass::for_width(599), LayoutClass::Compact);
        assert_eq!(LayoutClass::for_width(600), LayoutClass::Medium);
        assert_eq!(LayoutClass::for_width(839), LayoutClass::Medium);
        assert_eq!(LayoutClass::for_width(840), LayoutClass::Expanded);
    }

    /// Interface Design §5.1: the phone-size Unit screen is 1080 pixels wide at 2.621, 412 points.
    #[test]
    fn unit_phone() {
        let d = density(1080, 2400, (70, 155), ScreenKind::BuiltInHandheld, Some(UNIT_PHONE_FACTOR));
        assert_eq!(d.source, DensitySource::DeviceDescription);
        assert_eq!(layout_width(1080, d.factor, 1.0), 412);
        assert_eq!(LayoutClass::for_width(412), LayoutClass::Compact);
    }

    /// Breakpoint check 1: the 13.3-inch 2560 x 1600 laptop at 100%, 150% and 200% gives
    /// expanded, medium and compact, at a density factor of 2.375.
    #[test]
    fn laptop_13_3_inch() {
        // 13.3 inches at 16:10 is about 286.5 x 179.1 mm.
        let d = density(2560, 1600, (287, 179), ScreenKind::External, None);
        assert_eq!(d.source, DensitySource::ReportedSize);
        assert_eq!(d.factor, 2.375);
        let class = |s| LayoutClass::for_width(layout_width(2560, d.factor, s));
        assert_eq!(class(1.0), LayoutClass::Expanded);
        assert_eq!(class(1.5), LayoutClass::Medium);
        assert_eq!(class(2.0), LayoutClass::Compact);
    }

    /// Breakpoint check 2: a monitor whose size cannot be read, and one reporting 16 x 9 cm,
    /// both use factor 1.0.
    #[test]
    fn monitor_without_reliable_size() {
        for size in [(0, 0), (160, 90)] {
            let d = density(1920, 1080, size, ScreenKind::External, None);
            assert_eq!(d.source, DensitySource::Fallback);
            assert_eq!(d.factor, 1.0);
        }
    }

    /// Breakpoint check 3: a built-in panel reporting 0 x 0 uses the device-family value.
    #[test]
    fn built_in_without_size() {
        let d = density(1080, 2400, (0, 0), ScreenKind::BuiltInHandheld, None);
        assert_eq!(d.source, DensitySource::Fallback);
        assert_eq!(layout_width(1080, d.factor, 1.0), 412);
    }

    /// Issue UNIT-OS-ISS-0019: a 16:9 monitor reporting the placeholder 16 x 9 cm falls back to
    /// 1.0, while a genuine 16:9 monitor keeps its reported size.
    #[test]
    fn iss_0019_placeholder_16_9() {
        let placeholder = density(1920, 1080, (160, 90), ScreenKind::External, None);
        assert_eq!(placeholder.source, DensitySource::Fallback);
        assert_eq!(placeholder.factor, 1.0);
        // 24-inch 1920 x 1080 monitor, 531 x 299 mm: about 92 pixels per inch, factor 1.0.
        let genuine = density(1920, 1080, (531, 299), ScreenKind::External, None);
        assert_eq!(genuine.source, DensitySource::ReportedSize);
        assert_eq!(genuine.factor, 1.0);
        // 27-inch 3840 x 2160 monitor, 597 x 336 mm: about 163 pixels per inch, factor 1.75.
        let uhd = density(3840, 2160, (597, 336), ScreenKind::External, None);
        assert_eq!(uhd.source, DensitySource::ReportedSize);
        assert_eq!(uhd.factor, 1.75);
    }

    /// Issue UNIT-OS-ISS-0019: a 32:9 ultrawide reporting the placeholder 32 x 9 cm falls back,
    /// while a genuine 49-inch 32:9 ultrawide keeps its reported size.
    #[test]
    fn iss_0019_placeholder_32_9() {
        let placeholder = density(5120, 1440, (320, 90), ScreenKind::External, None);
        assert_eq!(placeholder.source, DensitySource::Fallback);
        assert_eq!(placeholder.factor, 1.0);
        // 49-inch 5120 x 1440 ultrawide, 1196 x 336 mm: about 109 pixels per inch, factor 1.125.
        let genuine = density(5120, 1440, (1196, 336), ScreenKind::External, None);
        assert_eq!(genuine.source, DensitySource::ReportedSize);
        assert_eq!(genuine.factor, 1.125);
        assert_eq!(layout_width(5120, genuine.factor, 1.0), 4551);
    }

    /// The other placeholders, in both orientations.
    #[test]
    fn iss_0019_other_placeholders() {
        for (w, h) in [(160, 100), (210, 90), (40, 30), (90, 160), (90, 320)] {
            assert!(is_placeholder_size(w, h), "{w} x {h} mm");
        }
        for (w, h) in [(161, 90), (520, 320), (800, 340), (1196, 336)] {
            assert!(!is_placeholder_size(w, h), "{w} x {h} mm");
        }
    }

    /// The layout class reflects the calculated width after scaling, even though Increment 1
    /// renders every class with the compact layout.
    #[test]
    fn class_follows_scaled_width() {
        let d = density(5120, 1440, (1196, 336), ScreenKind::External, None);
        assert_eq!(LayoutClass::for_width(layout_width(5120, d.factor, 1.0)), LayoutClass::Expanded);
        let d = density(1920, 1080, (531, 299), ScreenKind::External, None);
        assert_eq!(LayoutClass::for_width(layout_width(1920, d.factor, 2.0)), LayoutClass::Expanded);
        assert_eq!(LayoutClass::for_width(layout_width(1440, d.factor, 2.0)), LayoutClass::Medium);
        assert_eq!(LayoutClass::for_width(layout_width(1080, d.factor, 2.0)), LayoutClass::Compact);
    }

    #[test]
    fn unreliable_sizes() {
        // Ratio more than 10% from the pixel ratio.
        assert!(reliable_ppi(1920, 1080, (500, 400)).is_none());
        // Diagonal over 120 inches.
        assert!(reliable_ppi(1920, 1080, (3000, 1700)).is_none());
        // 24-inch monitor is fine.
        assert!(reliable_ppi(1920, 1080, (531, 299)).is_some());
    }

    #[test]
    fn config() {
        let c = DisplayConfig::parse(
            "# emulation\ndensity_factor = 1.7476\ndisplay_scaling=3\nbuilt_in_connectors=eDP, Virtual\n",
        );
        assert_eq!(c.density_factor, Some(1.7476));
        assert_eq!(c.display_scaling, 2.0);
        assert_eq!(c.screen_kind("Virtual"), ScreenKind::BuiltInHandheld);
        assert_eq!(c.screen_kind("HDMI-A"), ScreenKind::External);
        // Emulated Unit screen: 720 pixels at 720/412 is 412 points, compact. Uses the factor as
        // written in the emulation image's /etc/unit/display.conf.
        let e = DisplayConfig::parse("density_factor = 1.74757281553\nbuilt_in_connectors = Virtual\n");
        let d = density(720, 1560, (0, 0), e.screen_kind("Virtual"), e.density_factor);
        assert_eq!(layout_width(720, d.factor, 1.0), 412);
        assert_eq!(LayoutClass::for_width(layout_width(720, d.factor, 1.0)), LayoutClass::Compact);
        assert_eq!(DisplayConfig::parse("").display_scaling, 1.0);
    }

    /// Review finding 5 (PR #5): density factors outside 0.375 to 3.75, infinity and NaN are
    /// ignored, and a non-number scaling keeps 1.0, so the display can never become 0 points.
    #[test]
    fn config_out_of_range() {
        for bad in ["inf", "-inf", "NaN", "0", "-1", "0.3", "3.76", "1500", "1e308"] {
            let c = DisplayConfig::parse(&format!("density_factor = {bad}\n"));
            assert_eq!(c.density_factor, None, "density_factor = {bad}");
        }
        for good in ["0.375", "1.74757281553", "2.621", "3.75"] {
            let c = DisplayConfig::parse(&format!("density_factor = {good}\n"));
            assert!(c.density_factor.is_some(), "density_factor = {good}");
        }
        for bad in ["NaN", "inf", "-inf"] {
            let c = DisplayConfig::parse(&format!("display_scaling = {bad}\n"));
            assert_eq!(c.display_scaling, 1.0, "display_scaling = {bad}");
        }
    }
}
