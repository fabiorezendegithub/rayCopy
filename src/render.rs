//! Renders the code-image frame (window, gradient, lines, numbers, title) as SVG
//! and rasterizes it to PNG with resvg.

use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use crate::highlight;
use crate::model::{Snapshot, Token};
use crate::theme::{self, Palette};

const FONT_SIZE: f64 = 14.0;
const LINE_HEIGHT: f64 = 21.0; // 1.5 * 14
const CHAR_WIDTH: f64 = 0.6 * FONT_SIZE;
const CODE_PAD_X: f64 = 16.0;
const CODE_PAD_Y: f64 = 16.0;
const HEADER_HEIGHT: f64 = 40.0;
const FRAME_RADIUS: f64 = 12.0;
const TRAFFIC_RADIUS: f64 = 6.0;
const MIN_FRAME_WIDTH: f64 = 240.0;
const TAB: &str = "  ";
// Window chrome colours, measured from ray.so. The traffic lights are neutral in both modes.
// Light mode uses fixed greys; dark mode uses translucent white so it works on any theme.
const LIGHT_TRAFFIC: &str = "#C7C2BB";
const LIGHT_TITLE: &str = "#ABA59F";
const LIGHT_LINE_NUMBER: &str = "#C6C0B9";
const DARK_TRAFFIC: &str = "rgba(255,255,255,0.23)";
const DARK_TITLE: &str = "rgba(255,255,255,0.32)";
const DARK_LINE_NUMBER: &str = "rgba(255,255,255,0.22)";
const DARK_BORDER_OPACITY: f64 = 0.26;

/// Geometry shared by the SVG builder and the PNG size check.
struct Layout {
    pad: f64,
    frame_w: f64,
    frame_h: f64,
    header_h: f64,
    gutter_w: f64,
    svg_w: u32,
    svg_h: u32,
}

impl Layout {
    fn new(snap: &Snapshot, lines: &[String]) -> Self {
        let line_count = lines.len().max(1);
        let digits = line_count.to_string().len();
        let gutter_w = if snap.line_numbers {
            digits as f64 * CHAR_WIDTH + 24.0
        } else {
            0.0
        };
        let header_h = if snap.window_chrome { HEADER_HEIGHT } else { 0.0 };
        let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f64;
        let frame_w = match snap.width {
            Some(w) => (w as f64).max(1.0),
            None => (widest * CHAR_WIDTH + gutter_w + 2.0 * CODE_PAD_X).max(MIN_FRAME_WIDTH),
        };
        let frame_h = header_h + 2.0 * CODE_PAD_Y + line_count as f64 * LINE_HEIGHT;
        let pad = snap.padding as f64;
        Layout {
            pad,
            frame_w,
            frame_h,
            header_h,
            gutter_w,
            svg_w: (frame_w + 2.0 * pad).ceil() as u32,
            svg_h: (frame_h + 2.0 * pad).ceil() as u32,
        }
    }
}

/// A run of text with one colour, as drawn inside a line.
type Run = (String, String, bool);

pub fn build_svg(snap: &Snapshot) -> String {
    let pal = theme::palette(snap.theme, snap.dark);
    let lines = source_lines(&snap.code);
    let lay = Layout::new(snap, &lines);
    let runs = line_runs(snap, &lines, pal);
    let family = xml_escape(&font_family_css(&snap.font));

    let fx = lay.pad;
    let fy = lay.pad;
    let (panel_fill, panel_opacity) = paint(pal.window);

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
        w = lay.svg_w,
        h = lay.svg_h
    );
    s.push_str("<defs>");
    if snap.background {
        let _ = write!(
            s,
            r#"<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient>"#,
            xml_escape(pal.bg_from),
            xml_escape(pal.bg_to)
        );
    }
    s.push_str(
        r#"<filter id="shadow" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur in="SourceAlpha" stdDeviation="12"/><feOffset dy="8" result="blur"/><feComponentTransfer in="blur" result="shadow"><feFuncA type="linear" slope="0.35"/></feComponentTransfer></filter>"#,
    );
    let _ = write!(
        s,
        r#"<clipPath id="frame-clip"><rect x="{fx}" y="{fy}" width="{fw}" height="{fh}" rx="{r}" ry="{r}"/></clipPath>"#,
        fw = lay.frame_w,
        fh = lay.frame_h,
        r = FRAME_RADIUS
    );
    // The shadow must not show through the translucent panel (CSS box-shadow doesn't), so
    // the panel's area is cut out of it with a mask.
    let _ = write!(
        s,
        r##"<mask id="shadow-cut" maskUnits="userSpaceOnUse" x="0" y="0" width="{w}" height="{h}"><rect x="0" y="0" width="{w}" height="{h}" fill="#FFFFFF"/><rect x="{fx}" y="{fy}" width="{fw}" height="{fh}" rx="{r}" ry="{r}" fill="#000000"/></mask>"##,
        w = lay.svg_w,
        h = lay.svg_h,
        fw = lay.frame_w,
        fh = lay.frame_h,
        r = FRAME_RADIUS
    );
    s.push_str("</defs>");

    if snap.background {
        let _ = write!(
            s,
            r#"<rect x="0" y="0" width="{}" height="{}" fill="url(#bg)"/>"#,
            lay.svg_w, lay.svg_h
        );
    }

    let _ = write!(
        s,
        r##"<rect x="{fx}" y="{fy}" width="{fw}" height="{fh}" rx="{r}" ry="{r}" fill="#000000" filter="url(#shadow)" mask="url(#shadow-cut)"/><rect x="{fx}" y="{fy}" width="{fw}" height="{fh}" rx="{r}" ry="{r}" fill="{panel_fill}" fill-opacity="{panel_opacity}"/>"##,
        fw = lay.frame_w,
        fh = lay.frame_h,
        r = FRAME_RADIUS
    );

    if snap.dark {
        // 1px light outline, drawn inside the frame edge (as on ray.so's dark windows).
        let _ = write!(
            s,
            r##"<rect x="{}" y="{}" width="{}" height="{}" rx="{r}" ry="{r}" fill="none" stroke="#FFFFFF" stroke-opacity="{DARK_BORDER_OPACITY}" stroke-width="1"/>"##,
            fx + 0.5,
            fy + 0.5,
            lay.frame_w - 1.0,
            lay.frame_h - 1.0,
            r = FRAME_RADIUS - 0.5
        );
    }

    s.push_str(r#"<g clip-path="url(#frame-clip)">"#);

    if snap.window_chrome {
        let cy = fy + lay.header_h / 2.0;
        let (dot_fill, dot_op) = if snap.dark {
            paint(DARK_TRAFFIC)
        } else {
            (LIGHT_TRAFFIC.to_string(), 1.0)
        };
        for i in 0..3 {
            let cx = fx + 20.0 + i as f64 * 20.0;
            let _ = write!(
                s,
                r#"<circle cx="{cx}" cy="{cy}" r="{TRAFFIC_RADIUS}" fill="{dot_fill}"{}/>"#,
                opacity_attr(dot_op)
            );
        }
        if !snap.title.is_empty() || !snap.subtitle.is_empty() {
            let (title_fill, title_op) = if snap.dark {
                paint(DARK_TITLE)
            } else {
                (LIGHT_TITLE.to_string(), 1.0)
            };
            let _ = write!(
                s,
                r#"<text x="{}" y="{}" text-anchor="middle" font-family="sans-serif" font-size="13" fill="{}"{} xml:space="preserve">"#,
                fx + lay.frame_w / 2.0,
                cy + 4.5,
                title_fill,
                opacity_attr(title_op)
            );
            s.push_str(&xml_escape(&snap.title));
            if !snap.subtitle.is_empty() {
                let sep = if snap.title.is_empty() { "" } else { "  " };
                let _ = write!(
                    s,
                    r#"<tspan fill-opacity="0.6" font-size="12">{}{}</tspan>"#,
                    sep,
                    xml_escape(&snap.subtitle)
                );
            }
            s.push_str("</text>");
        }
    }

    let code_x = fx + CODE_PAD_X + lay.gutter_w;
    let num_x = fx + CODE_PAD_X + lay.gutter_w - 12.0;
    for (i, line_runs) in runs.iter().enumerate() {
        let top = fy + lay.header_h + CODE_PAD_Y + i as f64 * LINE_HEIGHT;
        let baseline = top + LINE_HEIGHT / 2.0 + FONT_SIZE * 0.35;
        let line_no = i + 1;

        if snap.highlighted_lines.contains(&line_no) {
            let (band, band_op) = paint(pal.highlight);
            let (border, border_op) = paint(pal.highlight_border);
            let _ = write!(
                s,
                r#"<rect x="{fx}" y="{top}" width="{}" height="{LINE_HEIGHT}" fill="{band}" fill-opacity="{band_op}"/>"#,
                lay.frame_w
            );
            let _ = write!(
                s,
                r#"<rect x="{fx}" y="{top}" width="3" height="{LINE_HEIGHT}" fill="{border}" fill-opacity="{border_op}"/>"#
            );
        }

        if snap.line_numbers {
            let (num_fill, num_op) = if snap.dark {
                paint(DARK_LINE_NUMBER)
            } else {
                (LIGHT_LINE_NUMBER.to_string(), 1.0)
            };
            let _ = write!(
                s,
                r#"<text x="{num_x}" y="{baseline}" text-anchor="end" font-family="{family}" font-size="{FONT_SIZE}" fill="{num_fill}"{}>{}</text>"#,
                opacity_attr(num_op),
                line_no
            );
        }

        if line_runs.iter().all(|(text, _, _)| text.is_empty()) {
            continue;
        }
        let _ = write!(
            s,
            r#"<text x="{code_x}" y="{baseline}" font-family="{family}" font-size="{FONT_SIZE}" xml:space="preserve">"#
        );
        for (text, color, italic) in line_runs {
            if text.is_empty() {
                continue;
            }
            let (hex, op) = paint(color);
            let style = if *italic { r#" font-style="italic""# } else { "" };
            let _ = write!(
                s,
                r#"<tspan fill="{hex}"{}{style}>{}</tspan>"#,
                opacity_attr(op),
                xml_escape(text)
            );
        }
        s.push_str("</text>");
    }

    s.push_str("</g></svg>");
    s
}

pub fn render_png(snap: &Snapshot) -> Result<Vec<u8>, String> {
    let scale = if (2..=6).contains(&snap.scale) {
        snap.scale
    } else {
        2
    };
    let svg = build_svg(snap);

    let mut opt = usvg::Options::default();
    opt.fontdb = font_database();
    let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| format!("invalid svg: {e}"))?;

    let lines = source_lines(&snap.code);
    let lay = Layout::new(snap, &lines);
    let width = lay.svg_w * scale;
    let height = lay.svg_h * scale;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("could not allocate {width}x{height} pixmap"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| format!("png encode failed: {e}"))
}

/// System fonts are loaded once and shared across renders.
fn font_database() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

/// Splits the code into lines, expanding tabs to two spaces.
fn source_lines(code: &str) -> Vec<String> {
    code.split('\n')
        .map(|l| l.trim_end_matches('\r').replace('\t', TAB))
        .collect()
}

/// One list of runs per line. Uses the highlighter's colours when they cover every
/// line, otherwise plain foreground text.
fn line_runs(snap: &Snapshot, lines: &[String], pal: &Palette) -> Vec<Vec<Run>> {
    let tokens = highlight::highlight_lines(snap);
    if tokens.is_empty() || tokens.len() < lines.len() {
        return lines
            .iter()
            .map(|l| vec![(l.clone(), pal.foreground.to_string(), false)])
            .collect();
    }
    tokens
        .iter()
        .take(lines.len())
        .map(|line: &Vec<Token>| {
            line.iter()
                .map(|t| {
                    let text = t.text.replace('\t', TAB).replace(|c| c == '\r' || c == '\n', "");
                    (text, t.color.clone(), t.italic)
                })
                .collect()
        })
        .collect()
}

/// Maps a palette colour to an SVG fill value and opacity.
/// Accepts "#rrggbb" and "rgba(r,g,b,a)".
fn paint(color: &str) -> (String, f64) {
    let c = color.trim();
    if let Some(inner) = c.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<f64> = inner
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if parts.len() == 4 {
            let hex = format!(
                "#{:02X}{:02X}{:02X}",
                parts[0].clamp(0.0, 255.0).round() as u8,
                parts[1].clamp(0.0, 255.0).round() as u8,
                parts[2].clamp(0.0, 255.0).round() as u8
            );
            return (hex, parts[3].clamp(0.0, 1.0));
        }
    }
    (xml_escape(c), 1.0)
}

fn opacity_attr(op: f64) -> String {
    if op < 1.0 {
        format!(r#" fill-opacity="{op}""#)
    } else {
        String::new()
    }
}

fn font_family_css(id: &str) -> String {
    let family = match id {
        "jetbrains-mono" => "JetBrains Mono",
        "geist-mono" => "Geist Mono",
        "ibm-plex-mono" => "IBM Plex Mono",
        "fira-code" => "Fira Code",
        "soehne-mono" => "Söhne Mono",
        "roboto-mono" => "Roboto Mono",
        "commit-mono" => "Commit Mono",
        "space-mono" => "Space Mono",
        "source-code-pro" => "Source Code Pro",
        "google-sans-code" => "Google Sans Code",
        _ => "JetBrains Mono",
    };
    format!("'{family}', monospace")
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Language, Theme};

    fn snap(code: &str) -> Snapshot {
        Snapshot {
            code: code.to_string(),
            language: Language::JavaScript,
            title: "demo.js".into(),
            subtitle: "sub".into(),
            theme: Theme::Candy,
            dark: false,
            background: true,
            line_numbers: true,
            padding: 64,
            font: "jetbrains-mono".into(),
            highlighted_lines: vec![2],
            width: None,
            scale: 2,
            window_chrome: true,
        }
    }

    #[test]
    fn gradient_uses_palette_colors() {
        let s = snap("const a = 1;");
        let pal = theme::palette(s.theme, s.dark);
        let svg = build_svg(&s);
        assert!(svg.contains(pal.bg_from), "missing bg_from");
        assert!(svg.contains(pal.bg_to), "missing bg_to");
        assert!(svg.contains("linearGradient"));
    }

    #[test]
    fn escapes_angle_brackets_in_code() {
        let svg = build_svg(&snap("x<y & z"));
        assert!(svg.contains("&lt;"));
        assert!(svg.contains("&amp;"));
        assert!(!svg.contains("<y"));
    }

    #[test]
    fn no_gradient_without_background() {
        let mut s = snap("let x = 1;");
        s.background = false;
        let svg = build_svg(&s);
        assert!(!svg.contains("linearGradient"));
        assert!(!svg.contains("url(#bg)"));
    }

    #[test]
    fn png_dimensions_scale_exactly() {
        let s2 = snap("const a = 1;\nconst b = 2;");
        let mut s4 = s2.clone();
        s4.scale = 4;

        let b2 = render_png(&s2).expect("scale 2 render");
        let b4 = render_png(&s4).expect("scale 4 render");
        assert!(b4.len() > b2.len());

        let lines = source_lines(&s2.code);
        let lay = Layout::new(&s2, &lines);
        let dims = |bytes: &[u8]| {
            let reader = png::Decoder::new(bytes).read_info().expect("png header");
            let info = reader.info();
            (info.width, info.height)
        };
        assert_eq!(dims(&b2), (lay.svg_w * 2, lay.svg_h * 2));
        assert_eq!(dims(&b4), (lay.svg_w * 4, lay.svg_h * 4));
    }
}
