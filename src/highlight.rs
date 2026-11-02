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

/// Formats the supported languages with a small, dependency-free formatter.
///
/// This is intentionally not a full compiler/parser. It understands the constructs that
/// matter for a code image: strings, comments, brackets, statements and HTML tags. It can
/// therefore expand compact or minified input while avoiding changes inside strings/comments.
/// The result is idempotent, which is important when the button is pressed repeatedly.
pub fn format_code(code: &str, language: Language) -> String {
    let language = resolve(language, code);
    match language {
        Language::Html => format_html(&code.replace('\t', "  ")),
        Language::CSharp | Language::Css | Language::JavaScript => {
            format_braced(&code.replace('\t', "  "), language)
        }
        Language::Auto => unreachable!("format_code resolves Auto before formatting"),
    }
}

fn indent_for(language: Language, depth: usize) -> String {
    let unit = if language == Language::CSharp { "    " } else { "  " };
    unit.repeat(depth)
}

fn format_braced(code: &str, language: Language) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;

    let flush = |out: &mut Vec<String>, current: &mut String, depth: usize, language: Language| {
        let text = current.trim();
        if !text.is_empty() {
            out.push(format!("{}{}", indent_for(language, depth), text));
        }
        current.clear();
    };

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if line_comment {
            if c == '\n' {
                flush(&mut out, &mut current, depth, language);
                line_comment = false;
            } else {
                current.push(c);
            }
            i += 1;
            continue;
        }

        if block_comment {
            current.push(c);
            if c == '*' && chars.get(i + 1) == Some(&'/') {
                current.push('/');
                block_comment = false;
                i += 2;
                continue;
            }
            if c == '\n' {
                flush(&mut out, &mut current, depth, language);
            }
            i += 1;
            continue;
        }

        if let Some(delimiter) = quote {
            current.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == delimiter {
                quote = None;
            }
            i += 1;
            continue;
        }

        if c == '"' || c == '\'' || (c == '`' && language == Language::JavaScript) {
            quote = Some(c);
            current.push(c);
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            current.push_str("//");
            line_comment = true;
            i += 2;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            current.push_str("/*");
            block_comment = true;
            i += 2;
            continue;
        }

        match c {
            '{' => {
                if !current.is_empty() && !current.ends_with(char::is_whitespace) {
                    current.push(' ');
                }
                current.push('{');
                flush(&mut out, &mut current, depth, language);
                depth += 1;
            }
            '}' => {
                flush(&mut out, &mut current, depth, language);
                depth = depth.saturating_sub(1);
                out.push(format!("{}}}", indent_for(language, depth)));
            }
            ';' => {
                current.push(';');
                if parens == 0 && brackets == 0 {
                    flush(&mut out, &mut current, depth, language);
                }
            }
            '(' => {
                parens += 1;
                current.push(c);
            }
            ')' => {
                parens = parens.saturating_sub(1);
                current.push(c);
            }
            '[' => {
                brackets += 1;
                current.push(c);
            }
            ']' => {
                brackets = brackets.saturating_sub(1);
                current.push(c);
            }
            '\n' | '\r' => {
                if c == '\n' {
                    flush(&mut out, &mut current, depth, language);
                }
            }
            c if c.is_whitespace() => {
                if !current.ends_with(char::is_whitespace) {
                    current.push(' ');
                }
            }
            _ => current.push(c),
        }
        i += 1;
    }
    flush(&mut out, &mut current, depth, language);
    out.join("\n")
}

fn format_html(code: &str) -> String {
    const VOID: [&str; 14] = [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
        "track", "wbr",
    ];
    let mut out = Vec::new();
    let mut text = String::new();
    let mut tag = String::new();
    let mut in_tag = false;
    let mut quote = None;
    let mut depth = 0usize;

    let flush_text = |out: &mut Vec<String>, text: &mut String, depth: usize| {
        let value = text.trim();
        if !value.is_empty() {
            out.push(format!("{}{}", "  ".repeat(depth), value));
        }
        text.clear();
    };

    for c in code.chars() {
        if !in_tag {
            if c == '<' {
                flush_text(&mut out, &mut text, depth);
                in_tag = true;
                tag.push(c);
            } else {
                text.push(c);
            }
            continue;
        }

        tag.push(c);
        if let Some(delimiter) = quote {
            if c == delimiter {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == '>' {
            let value = tag.trim();
            let inner = value.trim_start_matches('<').trim_end_matches('>').trim();
            let closing = inner.starts_with('/');
            let special = inner.starts_with('!') || inner.starts_with('?');
            let self_closing = inner.ends_with('/');
            let name = inner
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();

            if closing {
                depth = depth.saturating_sub(1);
            }
            out.push(format!("{}{}", "  ".repeat(depth), value));
            if !closing && !special && !self_closing && !VOID.contains(&name.as_str()) {
                depth += 1;
            }
            tag.clear();
            in_tag = false;
        }
    }
    if in_tag {
        text.push_str(&tag);
    }
    flush_text(&mut out, &mut text, depth);
    out.join("\n")
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

    #[test]
    fn format_expands_compact_code_in_all_supported_languages() {
        let js = format_code("function greet(name){return `Hello, ${name}!`;}", Language::JavaScript);
        assert_eq!(js, "function greet(name) {\n  return `Hello, ${name}!`;\n}");

        let csharp = format_code("class A{void M(){return;}}", Language::CSharp);
        assert_eq!(csharp, "class A {\n    void M() {\n        return;\n    }\n}");

        let css = format_code(".card{color:red;margin:0 auto;}", Language::Css);
        assert_eq!(css, ".card {\n  color:red;\n  margin:0 auto;\n}");

        let html = format_code("<div><p>Hello</p><img src=\"x.png\"></div>", Language::Html);
        assert_eq!(html, "<div>\n  <p>\n    Hello\n  </p>\n  <img src=\"x.png\">\n</div>");

        for (code, lang) in [
            (js, Language::JavaScript),
            (csharp, Language::CSharp),
            (css, Language::Css),
            (html, Language::Html),
        ] {
            assert_eq!(format_code(&code, lang), code);
        }
    }
}

