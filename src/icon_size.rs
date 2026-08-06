/// Explorer-aligned icon side length for sized extraction APIs.
///
/// Nominal pixel sizes below match Explorer view sizes at 96 DPI. On higher-DPI
/// displays, `Small` / `Medium` / `ExtraLarge` are taken from the shell image
/// lists (`SHIL_*`) and may scale with the system; `Large` is requested at an
/// exact 96×96 via `IShellItemImageFactory`.
///
/// `ExtraLarge` uses the jumbo image list (nominally 256×256). Icons that do not
/// ship a 256px glyph are often centered on a padded 256×256 canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSize {
    /// 16×16 — List / Small icons
    Small,
    /// 48×48 — Medium icons
    Medium,
    /// 96×96 — Large icons
    Large,
    /// 256×256 — Extra large / Super
    ExtraLarge,
}

impl IconSize {
    /// Nominal side length in pixels at 96 DPI.
    pub const fn pixels(self) -> u32 {
        match self {
            Self::Small => 16,
            Self::Medium => 48,
            Self::Large => 96,
            Self::ExtraLarge => 256,
        }
    }
}
