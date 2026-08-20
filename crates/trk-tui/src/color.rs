use ratatui::prelude::Color;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TerminalColorMode {
    #[default]
    TrueColor,
    Indexed256,
    Ansi16,
    Monochrome,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HsbColor {
    pub hue_degrees: f32,
    pub saturation: f32,
    pub brightness: f32,
}

impl HsbColor {
    #[must_use]
    pub const fn new(hue_degrees: f32, saturation: f32, brightness: f32) -> Self {
        Self {
            hue_degrees,
            saturation,
            brightness,
        }
    }

    #[must_use]
    pub fn to_rgb(self) -> RgbColor {
        let hue = finite_or_zero(self.hue_degrees).rem_euclid(360.0);
        let saturation = unit(self.saturation);
        let brightness = unit(self.brightness);
        let chroma = brightness * saturation;
        let sector = hue / 60.0;
        let secondary = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
        let (red, green, blue) = match sector as u8 {
            0 => (chroma, secondary, 0.0),
            1 => (secondary, chroma, 0.0),
            2 => (0.0, chroma, secondary),
            3 => (0.0, secondary, chroma),
            4 => (secondary, 0.0, chroma),
            _ => (chroma, 0.0, secondary),
        };
        let offset = brightness - chroma;
        RgbColor::new(
            channel(red + offset),
            channel(green + offset),
            channel(blue + offset),
        )
    }
}

#[must_use]
pub fn rgb_gradient(stops: &[(f32, RgbColor)], intensity: f32) -> RgbColor {
    gradient_segment(stops, intensity).map_or_else(RgbColor::default, |(left, right, amount)| {
        let left = left.1;
        let right = right.1;
        RgbColor::new(
            lerp_channel(left.red, right.red, amount),
            lerp_channel(left.green, right.green, amount),
            lerp_channel(left.blue, right.blue, amount),
        )
    })
}

#[must_use]
pub fn hsb_gradient(stops: &[(f32, HsbColor)], intensity: f32) -> RgbColor {
    gradient_segment(stops, intensity).map_or_else(RgbColor::default, |(left, right, amount)| {
        let left = left.1;
        let right = right.1;
        let left_hue = finite_or_zero(left.hue_degrees).rem_euclid(360.0);
        let right_hue = finite_or_zero(right.hue_degrees).rem_euclid(360.0);
        let hue_delta = (right_hue - left_hue + 540.0).rem_euclid(360.0) - 180.0;
        HsbColor::new(
            (left_hue + hue_delta * amount).rem_euclid(360.0),
            lerp(unit(left.saturation), unit(right.saturation), amount),
            lerp(unit(left.brightness), unit(right.brightness), amount),
        )
        .to_rgb()
    })
}

#[must_use]
pub fn terminal_color(rgb: RgbColor, mode: TerminalColorMode) -> Option<Color> {
    match mode {
        TerminalColorMode::TrueColor => Some(Color::Rgb(rgb.red, rgb.green, rgb.blue)),
        TerminalColorMode::Indexed256 => Some(Color::Indexed(indexed_color(rgb))),
        TerminalColorMode::Ansi16 => Some(nearest_ansi_color(rgb)),
        TerminalColorMode::Monochrome => None,
    }
}

trait GradientColor: Copy {
    fn position(self) -> f32;
}

impl GradientColor for (f32, RgbColor) {
    fn position(self) -> f32 {
        self.0
    }
}

impl GradientColor for (f32, HsbColor) {
    fn position(self) -> f32 {
        self.0
    }
}

fn gradient_segment<T: GradientColor>(stops: &[T], intensity: f32) -> Option<(T, T, f32)> {
    let first = *stops.first()?;
    let last = *stops.last()?;
    let intensity = unit(intensity);
    if intensity <= unit(first.position()) {
        return Some((first, first, 0.0));
    }
    for pair in stops.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let left_position = unit(left.position());
        let right_position = unit(right.position()).max(left_position);
        if intensity <= right_position {
            let span = right_position - left_position;
            let amount = if span > f32::EPSILON {
                (intensity - left_position) / span
            } else {
                0.0
            };
            return Some((left, right, unit(amount)));
        }
    }
    Some((last, last, 0.0))
}

fn indexed_color(rgb: RgbColor) -> u8 {
    let red = cube_level(rgb.red);
    let green = cube_level(rgb.green);
    let blue = cube_level(rgb.blue);
    let cube_rgb = RgbColor::new(cube_channel(red), cube_channel(green), cube_channel(blue));
    let cube_error = color_error(rgb, cube_rgb);

    let average = (u16::from(rgb.red) + u16::from(rgb.green) + u16::from(rgb.blue)) / 3;
    let gray = ((i32::from(average) - 8 + 5) / 10).clamp(0, 23) as u8;
    let gray_channel = 8 + gray * 10;
    let gray_error = color_error(rgb, RgbColor::new(gray_channel, gray_channel, gray_channel));
    if gray_error < cube_error {
        232 + gray
    } else {
        16 + 36 * red + 6 * green + blue
    }
}

fn nearest_ansi_color(rgb: RgbColor) -> Color {
    const PALETTE: [(Color, RgbColor); 16] = [
        (Color::Black, RgbColor::new(0, 0, 0)),
        (Color::Red, RgbColor::new(128, 0, 0)),
        (Color::Green, RgbColor::new(0, 128, 0)),
        (Color::Yellow, RgbColor::new(128, 128, 0)),
        (Color::Blue, RgbColor::new(0, 0, 128)),
        (Color::Magenta, RgbColor::new(128, 0, 128)),
        (Color::Cyan, RgbColor::new(0, 128, 128)),
        (Color::Gray, RgbColor::new(192, 192, 192)),
        (Color::DarkGray, RgbColor::new(128, 128, 128)),
        (Color::LightRed, RgbColor::new(255, 0, 0)),
        (Color::LightGreen, RgbColor::new(0, 255, 0)),
        (Color::LightYellow, RgbColor::new(255, 255, 0)),
        (Color::LightBlue, RgbColor::new(0, 0, 255)),
        (Color::LightMagenta, RgbColor::new(255, 0, 255)),
        (Color::LightCyan, RgbColor::new(0, 255, 255)),
        (Color::White, RgbColor::new(255, 255, 255)),
    ];
    PALETTE
        .into_iter()
        .min_by_key(|(_, candidate)| color_error(rgb, *candidate))
        .map_or(Color::White, |(color, _)| color)
}

fn cube_level(channel: u8) -> u8 {
    ((u16::from(channel) * 5 + 127) / 255) as u8
}

fn cube_channel(level: u8) -> u8 {
    if level == 0 {
        0
    } else {
        55 + level * 40
    }
}

fn color_error(left: RgbColor, right: RgbColor) -> u32 {
    let red = i32::from(left.red) - i32::from(right.red);
    let green = i32::from(left.green) - i32::from(right.green);
    let blue = i32::from(left.blue) - i32::from(right.blue);
    (red * red + green * green + blue * blue) as u32
}

fn lerp_channel(start: u8, end: u8, amount: f32) -> u8 {
    channel(lerp(
        f32::from(start) / 255.0,
        f32::from(end) / 255.0,
        amount,
    ))
}

fn channel(value: f32) -> u8 {
    (unit(value) * 255.0).round() as u8
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

fn unit(value: f32) -> f32 {
    finite_or_zero(value).clamp(0.0, 1.0)
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_gradient_clamps_and_interpolates_finite_channels() {
        let stops = [
            (0.0, RgbColor::new(0, 20, 40)),
            (1.0, RgbColor::new(200, 220, 240)),
        ];

        assert_eq!(rgb_gradient(&stops, -1.0), stops[0].1);
        assert_eq!(rgb_gradient(&stops, f32::NAN), stops[0].1);
        assert_eq!(rgb_gradient(&stops, 2.0), stops[1].1);
        assert_eq!(rgb_gradient(&stops, 0.5), RgbColor::new(100, 120, 140));
        assert_eq!(rgb_gradient(&[], 0.5), RgbColor::default());
    }

    #[test]
    fn hsb_gradient_uses_the_shortest_hue_path() {
        let stops = [
            (0.0, HsbColor::new(350.0, 1.0, 1.0)),
            (1.0, HsbColor::new(10.0, 1.0, 1.0)),
        ];

        assert_eq!(hsb_gradient(&stops, 0.5), RgbColor::new(255, 0, 0));
        assert_eq!(
            HsbColor::new(120.0, 1.0, 1.0).to_rgb(),
            RgbColor::new(0, 255, 0)
        );
        assert_eq!(
            HsbColor::new(f32::NAN, f32::NAN, f32::NAN).to_rgb(),
            RgbColor::default()
        );
    }

    #[test]
    fn terminal_modes_emit_only_their_supported_color_shapes() {
        let rgb = RgbColor::new(86, 224, 212);

        assert_eq!(
            terminal_color(rgb, TerminalColorMode::TrueColor),
            Some(Color::Rgb(86, 224, 212))
        );
        assert!(matches!(
            terminal_color(rgb, TerminalColorMode::Indexed256),
            Some(Color::Indexed(_))
        ));
        assert!(
            matches!(terminal_color(rgb, TerminalColorMode::Ansi16), Some(color) if !matches!(color, Color::Rgb(..) | Color::Indexed(_)))
        );
        assert_eq!(terminal_color(rgb, TerminalColorMode::Monochrome), None);
    }
}
