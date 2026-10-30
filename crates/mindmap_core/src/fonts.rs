//! The font catalogue. The UI and the SVG exporter both read from here, so
//! a font is defined in exactly one place. Font files live in `ui/fonts`.

pub struct FontInfo {
    /// Name stored in map files and shown in the picker.
    pub name: &'static str,
    /// CSS `font-family` value with generic fallbacks.
    pub stack: &'static str,
    /// Average glyph width as a fraction of the font size, used for layout.
    /// Measured in a browser on a mixed sample of words, plus about 7% headroom
    /// so labels never clip.
    pub width: f32,
    /// (weight, file name) pairs available in `ui/fonts`.
    pub files: &'static [(u16, &'static str)],
}

pub const DEFAULT_FONT: &str = "Open Sans";

pub const FONTS: &[FontInfo] = &[
    FontInfo {
        name: "Open Sans",
        stack: "'Open Sans', 'Segoe UI', sans-serif",
        width: 0.53,
        files: &[
            (400, "open-sans-latin-400-normal.woff2"),
            (700, "open-sans-latin-700-normal.woff2"),
        ],
    },
    FontInfo {
        name: "Montserrat",
        stack: "'Montserrat', 'Segoe UI', sans-serif",
        width: 0.57,
        files: &[
            (400, "montserrat-latin-400-normal.woff2"),
            (700, "montserrat-latin-700-normal.woff2"),
        ],
    },
    FontInfo {
        name: "Lora",
        stack: "'Lora', Georgia, serif",
        width: 0.53,
        files: &[
            (400, "lora-latin-400-normal.woff2"),
            (700, "lora-latin-700-normal.woff2"),
        ],
    },
    FontInfo {
        name: "Caveat",
        stack: "'Caveat', 'Segoe Print', cursive",
        width: 0.39,
        files: &[
            (400, "caveat-latin-400-normal.woff2"),
            (700, "caveat-latin-700-normal.woff2"),
        ],
    },
    FontInfo {
        name: "Patrick Hand",
        stack: "'Patrick Hand', 'Segoe Print', cursive",
        width: 0.41,
        files: &[(400, "patrick-hand-latin-400-normal.woff2")],
    },
    FontInfo {
        name: "Pacifico",
        stack: "'Pacifico', 'Brush Script MT', cursive",
        width: 0.49,
        files: &[(400, "pacifico-latin-400-normal.woff2")],
    },
    FontInfo {
        name: "Source Code Pro",
        stack: "'Source Code Pro', Consolas, monospace",
        width: 0.61,
        files: &[
            (400, "source-code-pro-latin-400-normal.woff2"),
            (700, "source-code-pro-latin-700-normal.woff2"),
        ],
    },
];

pub fn find(name: &str) -> Option<&'static FontInfo> {
    FONTS.iter().find(|f| f.name == name)
}

/// Unknown names fall back to the default font instead of failing, so a map
/// edited by hand or saved by a newer version still opens.
pub fn resolve(name: &str) -> &'static FontInfo {
    find(name).unwrap_or_else(|| find(DEFAULT_FONT).expect("default font exists"))
}
