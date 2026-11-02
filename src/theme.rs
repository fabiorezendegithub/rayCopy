//! Palettes for the two supported themes (Candy, Sand) in light and dark mode.
//! Values are copied from ray.so's theme store (app/(navigation)/(code)/store/themes.ts).

use crate::model::Theme;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_from: &'static str,
    pub bg_to: &'static str,
    pub foreground: &'static str,
    pub constant: &'static str,
    /// Colour of quoted strings (ray.so's "string expression").
    pub string_expression: &'static str,
    pub comment: &'static str,
    pub keyword: &'static str,
    pub parameter: &'static str,
    pub function: &'static str,
    pub punctuation: &'static str,
    pub number: &'static str,
    pub property: &'static str,
    pub highlight: &'static str,
    pub highlight_border: &'static str,
    /// Fill of the window panel (CSS colour, may be rgba).
    pub window: &'static str,
}

const CANDY_LIGHT: Palette = Palette {
    bg_from: "#A58EFB",
    bg_to: "#E9BFF8",
    foreground: "#434447",
    constant: "#2286A6",
    string_expression: "#B2762E",
    comment: "#8D949B",
    keyword: "#DC155E",
    parameter: "#009033",
    function: "#009033",
    punctuation: "#d15a8b",
    number: "#676DFF",
    property: "#2286A6",
    highlight: "rgba(220,21,94,0.10)",
    highlight_border: "#DC155E",
    window: "rgba(255,255,255,0.75)",
};

const CANDY_DARK: Palette = Palette {
    bg_from: "#A58EFB",
    bg_to: "#E9BFF8",
    foreground: "#FFFFFF",
    constant: "#1AC8FF",
    string_expression: "#DFD473",
    comment: "#807796",
    keyword: "#FF659C",
    parameter: "#1AC8FF",
    function: "#73DFA5",
    punctuation: "#FF659C",
    number: "#7A7FFD",
    property: "#1AC8FF",
    highlight: "rgba(255,101,156,0.14)",
    highlight_border: "#FF659C",
    window: "#322a3f",
};

const SAND_LIGHT: Palette = Palette {
    bg_from: "#EED5B6",
    bg_to: "#AF8856",
    foreground: "#262217",
    constant: "#A28C4E",
    string_expression: "#C57416",
    comment: "#C4B39C",
    keyword: "#906937",
    parameter: "#DA8744",
    function: "#DA8744",
    punctuation: "#DA8744",
    number: "#A28C4E",
    property: "#A28C4E",
    highlight: "rgba(218,135,68,0.10)",
    highlight_border: "#DA8744",
    window: "rgba(255,255,255,0.75)",
};

const SAND_DARK: Palette = Palette {
    bg_from: "#EED5B6",
    bg_to: "#AF8856",
    foreground: "#FFFFFF",
    constant: "#C2B181",
    string_expression: "#EED5B8",
    comment: "#837E77",
    keyword: "#D3B48C",
    parameter: "#F4A361",
    function: "#F4A361",
    punctuation: "#F4A361",
    number: "#C2B181",
    property: "#C2B181",
    highlight: "rgba(244,163,97,0.14)",
    highlight_border: "#F4A361",
    window: "rgba(0,0,0,0.75)",
};

/// Returns the palette for a theme family in light or dark mode.
pub fn palette(theme: Theme, dark: bool) -> &'static Palette {
    match (theme, dark) {
        (Theme::Candy, false) => &CANDY_LIGHT,
        (Theme::Candy, true) => &CANDY_DARK,
        (Theme::Sand, false) => &SAND_LIGHT,
        (Theme::Sand, true) => &SAND_DARK,
    }
}
