//! Syntax highlighting restricted to C#, HTML, CSS and JavaScript.
//!
//! Tokenization uses syntect's grammars (restricted to those four languages), but colors
//! never come from a syntect theme: each token's TextMate scope stack is mapped onto a
//! field of `crate::theme::palette(theme, dark)`.
//!
//! Public API (used by other modules):
//!   - `highlight_lines(snap)`   -> one `Vec<Token>` per source line
//!   - `detect_language(code)`   -> heuristic choice among the four supported languages
//!   - `format_code(code, lang)` -> conservative, idempotent re-indentation

use std::sync::OnceLock;

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

use crate::model::{Language, Snapshot, Token};
use crate::theme::{palette, Palette};

/// Grammar names (as they appear in syntect's default packs) that we use. Nothing else is
/// ever looked up, so no other grammar is reachable from this module.
///
/// Note: syntect cannot drop syntaxes from its precompiled defaults (cross-syntax context
/// indices would dangle, e.g. HTML embeds JavaScript/CSS), so the defaults are loaded once
/// and only these four names are exposed.
const KEPT_SYNTAXES: [&str; 4] = ["C#", "HTML", "CSS", "JavaScript"];

/// The syntax set, loaded once from syntect's defaults.
fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Looks up a grammar by name, but only if it is one of the kept four.
fn find_kept<'a>(set: &'a SyntaxSet, name: &str) -> Option<&'a SyntaxReference> {
    if KEPT_SYNTAXES.contains(&name) {
        set.find_syntax_by_name(name)
    } else {
        None
    }
}

fn syntax_name(language: Language) -> Option<&'static str> {
    match language {
        Language::CSharp => Some("C#"),
        Language::Html => Some("HTML"),
        Language::Css => Some("CSS"),
        Language::JavaScript => Some("JavaScript"),
        Language::Auto => None,
    }
}

/// Resolves `Auto` through detection; never returns `Auto`.
fn resolve(language: Language, code: &str) -> Language {
    match language {
        Language::Auto => detect_language(code),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Highlighting
// ---------------------------------------------------------------------------

/// Highlights the snapshot's code. Returns one token list per source line.
pub fn highlight_lines(snap: &Snapshot) -> Vec<Vec<Token>> {
    let language = resolve(snap.language, &snap.code);
    let pal = palette(snap.theme, snap.dark);
    // Tabs become two spaces so that token text reproduces the line exactly.
    let code = snap.code.replace('\t', "  ");
    highlight_with(&code, language, pal)
}

fn highlight_with(code: &str, language: Language, pal: &'static Palette) -> Vec<Vec<Token>> {
    let set = syntax_set();
    let syntax: Option<&SyntaxReference> = syntax_name(language).and_then(|name| find_kept(set, name));
    let mut parser = syntax.map(ParseState::new);
    let mut stack = ScopeStack::new();

    let mut out = Vec::new();
    for raw in code.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let mut tokens: Vec<Token> = Vec::new();
        match parser.as_mut() {
            None => push_token(&mut tokens, line, pal.foreground, false),
            Some(state) => {
                // syntect expects newline-terminated lines; the newline is stripped again below.
                let text = format!("{line}\n");
                let ops = state.parse_line(&text, set).unwrap_or_default();
                let mut last = 0;
                for (pos, op) in ops {
                    let pos = pos.min(text.len()).max(last);
                    emit_segment(&mut tokens, &text[last..pos], &stack, pal);
                    last = pos;
                    let _ = stack.apply(&op);
                }
                emit_segment(&mut tokens, &text[last..], &stack, pal);
            }
        }
        out.push(tokens);
    }
    out
}

fn emit_segment(tokens: &mut Vec<Token>, segment: &str, stack: &ScopeStack, pal: &'static Palette) {
    let segment = segment.strip_suffix('\n').unwrap_or(segment);
    if segment.is_empty() {
        return;
    }
    let (color, italic) = classify(stack.as_slice(), pal);
    push_token(tokens, segment, color, italic);
}

/// Appends text, merging with the previous token when style is identical.
fn push_token(tokens: &mut Vec<Token>, text: &str, color: &str, italic: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = tokens.last_mut() {
        if last.color == color && last.italic == italic {
            last.text.push_str(text);
            return;
        }
    }
    tokens.push(Token {
        text: text.to_string(),
        color: color.to_string(),
        italic,
    });
}

/// Picks a palette color for a scope stack. The innermost scope with a known mapping wins.
fn classify(scopes: &[Scope], pal: &'static Palette) -> (&'static str, bool) {
    for scope in scopes.iter().rev() {
        let name = scope.build_string();
        if let Some(hit) = classify_scope(&name, pal) {
            return hit;
        }
    }
    (pal.foreground, false)
}

fn classify_scope(name: &str, pal: &'static Palette) -> Option<(&'static str, bool)> {
    let starts = |prefix: &str| name.starts_with(prefix);
    if starts("comment") || starts("punctuation.definition.comment") {
        // ray.so renders comments upright (not italic).
        return Some((pal.comment, false));
    }
    // Quote characters belong visually to the string they delimit.
    if starts("punctuation.definition.string") || starts("string") {
        return Some((pal.string_expression, false));
    }
    if starts("constant.numeric") {
        return Some((pal.number, false));
    }
    if starts("constant") {
        return Some((pal.constant, false));
    }
    if starts("keyword") || starts("storage") {
        return Some((pal.keyword, false));
    }
    if starts("entity.name.function") || starts("support.function") {
        return Some((pal.function, false));
    }
    if starts("variable.parameter") {
        // On ray.so, C# parameter names (`nome` in `Dizer(string nome)`) are plain text.
        // Other languages were not checked, so they keep the parameter colour.
        if name.ends_with(".cs") {
            return Some((pal.foreground, false));
        }
        return Some((pal.parameter, false));
    }
    if starts("entity.name.tag") {
        return Some((pal.keyword, false));
    }
    if starts("entity.other.attribute-name")
        || starts("support.type.property-name")
        || starts("meta.property-name")
        || starts("variable.other.property")
    {
        return Some((pal.property, false));
    }
    // Structural punctuation (`;`, `{ }`, `( )`) is plain text on ray.so.
    if starts("punctuation.terminator")
        || starts("punctuation.section")
        || starts("punctuation.definition.block")
        || starts("punctuation.definition.parameters")
    {
        return Some((pal.foreground, false));
    }
    if starts("punctuation") {
        return Some((pal.punctuation, false));
    }
    // Type and class names (System, Demo, Saudacao) take the function/type colour on ray.so.
    // entity.name.{type,class,struct,interface,enum,namespace} and qualified paths such as
    // `System` in `using System;`. Functions, tags and attributes were handled above.
    if starts("support.type") || starts("entity.name") || starts("meta.path") {
        return Some((pal.function, false));
    }
    None
}

// ---------------------------------------------------------------------------
// Language detection
// ---------------------------------------------------------------------------

/// Score-based guess among the four supported languages. Ties and no signals -> JavaScript.
pub fn detect_language(code: &str) -> Language {
    if code.trim().is_empty() {
        return Language::JavaScript;
    }

    let mut csharp = 0u32;
    let mut javascript = 0u32;
    let mut html = 0u32;
    let mut css = 0u32;

    // C#
    for (needle, weight) in [
        ("using System", 3),
        ("namespace ", 2),
        ("public class", 2),
        ("static void Main", 3),
        ("Console.Write", 2),
    ] {
        if code.contains(needle) {
            csharp += weight;
        }
    }
    if code.contains("var ") && code.contains(';') {
        csharp += 1;
    }
    if code.contains("=>") && code.contains(';') {
        csharp += 1;
    }

    // JavaScript
    for (needle, weight) in [
        ("function", 2),
        ("const ", 1),
        ("let ", 1),
        ("=>", 1),
        ("console.log", 2),
        ("require(", 2),
        ("module.exports", 2),
    ] {
        if code.contains(needle) {
            javascript += weight;
        }
    }

    // HTML
    let lower = code.to_ascii_lowercase();
    if lower.contains("<!doctype") {
        html += 3;
    }
    if lower.contains("<html") {
        html += 3;
    }
    if code.contains("</") && code.lines().any(|l| l.trim_start().starts_with('<')) {
        html += 2;
    }

    // CSS
    for (needle, weight) in [("@media", 3), ("@import", 3), (":root", 3)] {
        if code.contains(needle) {
            css += weight;
        }
    }
    if code.contains('{') && code.lines().any(is_css_declaration) {
        css += 2;
    }

    let scores = [
        (Language::CSharp, csharp),
        (Language::JavaScript, javascript),
        (Language::Html, html),
        (Language::Css, css),
    ];
    let max = scores.iter().map(|(_, s)| *s).max().unwrap_or(0);
    if max == 0 {
        return Language::JavaScript;
    }
    let winners: Vec<Language> = scores.iter().filter(|(_, s)| *s == max).map(|(l, _)| *l).collect();
    match winners.as_slice() {
        [single] => *single,
        _ => Language::JavaScript,
    }
}

/// True for a line shaped like `prop-name: value;` (lowercase property name).
fn is_css_declaration(line: &str) -> bool {
    let t = line.trim();
    if !t.ends_with(';') {
        return false;
    }
    match t.split_once(':') {
        Some((name, _)) => {
            !name.is_empty()
                && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && name.chars().all(|c| c.is_ascii_lowercase() || c == '-')
        }
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Conservative re-indenter. Only leading/trailing whitespace and blank-line runs change;
/// the content of each line is kept byte-for-byte. Idempotent.
pub fn format_code(code: &str, language: Language) -> String {
    let language = resolve(language, code);
    let code = code.replace('\t', "  ");

    let mut out: Vec<String> = Vec::new();
    let mut pending_blank = false;
    let mut depth: i64 = 0;

    match language {
        Language::Html => format_html(&code, &mut out, &mut pending_blank),
        _ => {
            let unit = if language == Language::CSharp { "    " } else { "  " };
            let css = language == Language::Css;
            let mut scan = ScanState::default();
            for raw in code.split('\n') {
                let line = raw.strip_suffix('\r').unwrap_or(raw);

                if scan.in_multiline() {
                    // Inside a block comment or template literal: keep the text untouched.
                    flush_blank(&mut out, &mut pending_blank);
                    out.push(line.to_string());
                    depth = (depth + scan.scan_line(line, css)).max(0);
                    continue;
                }

                let t = line.trim();
                if t.is_empty() {
                    pending_blank = !out.is_empty();
                    continue;
                }

                flush_blank(&mut out, &mut pending_blank);
                let closes_first = t.starts_with(['}', ']', ')']);
                let level = if closes_first { (depth - 1).max(0) } else { depth.max(0) };
                out.push(format!("{}{}", unit.repeat(level as usize), t));
                depth = (depth + scan.scan_line(t, css)).max(0);
            }
        }
    }

    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

fn flush_blank(out: &mut Vec<String>, pending_blank: &mut bool) {
    if *pending_blank {
        out.push(String::new());
        *pending_blank = false;
    }
}

/// Tracks constructs that span lines (block comments, template literals) and bracket depth
/// changes within a single line, ignoring brackets inside strings and comments.
#[derive(Default)]
struct ScanState {
    in_block_comment: bool,
    in_template: bool,
}

impl ScanState {
    fn in_multiline(&self) -> bool {
        self.in_block_comment || self.in_template
    }

    /// Returns the net bracket depth change of `line`, updating the multi-line state.
    fn scan_line(&mut self, line: &str, css: bool) -> i64 {
        let chars: Vec<char> = line.chars().collect();
        let mut delta = 0i64;
        let mut i = 0usize;
        while i < chars.len() {
            if self.in_block_comment {
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    self.in_block_comment = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if self.in_template {
                match chars[i] {
                    '\\' => i += 2,
                    '`' => {
                        self.in_template = false;
                        i += 1;
                    }
                    _ => i += 1,
                }
                continue;
            }
            let c = chars[i];
            match c {
                '/' if chars.get(i + 1) == Some(&'*') => {
                    self.in_block_comment = true;
                    i += 2;
                }
                '/' if !css && chars.get(i + 1) == Some(&'/') => break,
                '"' | '\'' => {
                    // Skip the string literal; unterminated strings end at the line end.
                    i += 1;
                    while i < chars.len() {
                        if chars[i] == '\\' {
                            i += 2;
                        } else if chars[i] == c {
                            i += 1;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }
                '`' if !css => {
                    self.in_template = true;
                    i += 1;
                }
                '{' | '[' | '(' => {
                    delta += 1;
                    i += 1;
                }
                '}' | ']' | ')' => {
                    delta -= 1;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        delta
    }
}

/// HTML: indents by tag depth when the markup is simple enough; otherwise keeps the
/// author's indentation (only trailing whitespace and blank runs are changed).
fn format_html(code: &str, out: &mut Vec<String>, pending_blank: &mut bool) {
    let simple = html_is_simple(code);
    let mut depth: i64 = 0;
    for raw in code.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let t = line.trim();
        if t.is_empty() {
            *pending_blank = !out.is_empty();
            continue;
        }
        flush_blank(out, pending_blank);
        if simple {
            let closes_first = t.starts_with("</");
            let level = if closes_first { (depth - 1).max(0) } else { depth.max(0) };
            out.push(format!("{}{}", "  ".repeat(level as usize), t));
            depth = (depth + html_tag_delta(t)).max(0);
        } else {
            out.push(line.trim_end().to_string());
        }
    }
}

/// Simple HTML: no raw-text elements or comments, and every line has balanced `<` / `>`.
fn html_is_simple(code: &str) -> bool {
    let lower = code.to_ascii_lowercase();
    if ["<pre", "<textarea", "<script", "<style", "<!--"].iter().any(|t| lower.contains(t)) {
        return false;
    }
    code.split('\n').all(|line| {
        line.matches('<').count() == line.matches('>').count()
    })
}

/// Net tag-depth change of one line of simple HTML.
fn html_tag_delta(line: &str) -> i64 {
    const VOID: [&str; 14] = [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
        "track", "wbr",
    ];
    let mut delta = 0i64;
    let mut rest = line;
    while let Some(lt) = rest.find('<') {
        let after = &rest[lt + 1..];
        let Some(gt) = after.find('>') else { break };
        let inner = &after[..gt];
        rest = &after[gt + 1..];
        if inner.starts_with('!') || inner.starts_with('?') {
            continue;
        }
        if inner.starts_with('/') {
            delta -= 1;
        } else if inner.ends_with('/') {
            // self-closing
        } else {
            let name = inner
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !VOID.contains(&name.as_str()) {
                delta += 1;
            }
        }
    }
    delta
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Theme;

    const CSHARP: &str = "using System;\nnamespace Demo\n{\n    // say hi\n    public class Program\n    {\n        static void Main()\n        {\n            var name = \"world\";\n            Console.WriteLine($\"hi {name}\");\n        }\n    }\n}";
    const HTML: &str = "<!DOCTYPE html>\n<html>\n<body>\n\t<p class=\"a\">Hi</p>\n\n\n<img src=\"x.png\">\n</body>\n</html>";
    const CSS: &str = ":root {\n  --accent: red;\n}\n\n.card {\n  color: blue;\n  margin: 0 auto;\n}\n@media (max-width: 600px) {\n  .card { padding: 4px; }\n}";
    const JS: &str = "const fs = require('fs');\nfunction greet(name) {\n  return `hi ${name}`;\n}\nconsole.log(greet('x'));\nmodule.exports = greet;";

    fn snap(code: &str, language: Language) -> Snapshot {
        Snapshot {
            code: code.to_string(),
            language,
            title: String::new(),
            subtitle: String::new(),
            theme: Theme::Candy,
            dark: false,
            background: true,
            line_numbers: false,
            padding: 64,
            font: "jetbrains-mono".to_string(),
            highlighted_lines: vec![],
            width: None,
            scale: 4,
            window_chrome: true,
        }
    }

    #[test]
    fn detects_each_language() {
        assert_eq!(detect_language(CSHARP), Language::CSharp);
        assert_eq!(detect_language(HTML), Language::Html);
        assert_eq!(detect_language(CSS), Language::Css);
        assert_eq!(detect_language(JS), Language::JavaScript);
    }

    #[test]
    fn no_signal_and_empty_default_to_javascript() {
        assert_eq!(detect_language(""), Language::JavaScript);
        assert_eq!(detect_language("hello world"), Language::JavaScript);
    }

    #[test]
    fn all_four_grammars_are_available() {
        for name in KEPT_SYNTAXES {
            assert!(syntax_set().find_syntax_by_name(name).is_some(), "missing grammar {name}");
        }
    }

    #[test]
    fn highlight_reproduces_every_line() {
        for (code, lang) in [
            (CSHARP, Language::CSharp),
            (HTML, Language::Html),
            (CSS, Language::Css),
            (JS, Language::JavaScript),
            (JS, Language::Auto),
            ("", Language::JavaScript),
        ] {
            let lines = highlight_lines(&snap(code, lang));
            let expected: Vec<String> = code
                .replace('\t', "  ")
                .split('\n')
                .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
                .collect();
            assert_eq!(lines.len(), expected.len());
            for (tokens, want) in lines.iter().zip(expected.iter()) {
                let got: String = tokens.iter().map(|t| t.text.as_str()).collect();
                assert_eq!(&got, want);
            }
        }
    }

    #[test]
    fn csharp_comment_is_upright() {
        let lines = highlight_lines(&snap(CSHARP, Language::CSharp));
        let comment = lines[3].iter().find(|t| t.text.contains("say hi"));
        let comment = comment.expect("comment token present");
        assert!(!comment.italic);
        assert_eq!(comment.color, palette(Theme::Candy, false).comment);
    }

    #[test]
    fn format_is_idempotent() {
        for (code, lang) in [
            (CSHARP, Language::CSharp),
            (HTML, Language::Html),
            (CSS, Language::Css),
            (JS, Language::JavaScript),
            ("if (a) {\n\t\tfoo();   \n\n\n\n   bar();\n}\n", Language::JavaScript),
            ("<div>\n<ul>\n<li>x</li>\n</ul>\n</div>", Language::Html),
        ] {
            let once = format_code(code, lang);
            assert_eq!(format_code(&once, lang), once);
        }
    }

    #[test]
    fn format_reindents_by_depth() {
        let out = format_code("function f() {\nreturn 1;\n}", Language::JavaScript);
        assert_eq!(out, "function f() {\n  return 1;\n}");
        let out = format_code("class A\n{\nvoid M()\n{\n}\n}", Language::CSharp);
        assert_eq!(out, "class A\n{\n    void M()\n    {\n    }\n}");
    }
}

