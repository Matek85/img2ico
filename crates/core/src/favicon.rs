// The text files of a website icon package: the web app manifest and the
// lines to paste into a page's <head>. (The pictures come from the conversion;
// this is only what goes with them.)

/// A string as JSON text, in quotes, with everything that needs escaping.
pub fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The text with the characters that matter in HTML attributes escaped.
fn html_attribute(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `site.webmanifest`: what a browser needs to offer the site as an app.
/// `name` may be empty; the colors are `#rrggbb`.
pub fn manifest(name: &str, theme_color: &str, background_color: &str) -> String {
    let name = if name.trim().is_empty() {
        "My site"
    } else {
        name.trim()
    };
    format!(
        "{{\n  \"name\": {name},\n  \"short_name\": {name},\n  \"icons\": [\n    {{ \"src\": \"icon-192.png\", \"sizes\": \"192x192\", \"type\": \"image/png\" }},\n    {{ \"src\": \"icon-512.png\", \"sizes\": \"512x512\", \"type\": \"image/png\" }}\n  ],\n  \"theme_color\": {theme},\n  \"background_color\": {background},\n  \"display\": \"standalone\"\n}}\n",
        name = json_string(name),
        theme = json_string(theme_color),
        background = json_string(background_color),
    )
}

/// The lines for the `<head>` of a page, for a package that was put at the
/// root of the site. `has_svg` adds the SVG icon that modern browsers prefer.
pub fn head_snippet(has_svg: bool, theme_color: &str) -> String {
    let mut lines = vec![r#"<link rel="icon" href="/favicon.ico" sizes="48x48">"#.to_string()];
    if has_svg {
        lines.push(r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml">"#.to_string());
    }
    lines.push(
        r#"<link rel="icon" href="/favicon-32x32.png" type="image/png" sizes="32x32">"#.to_string(),
    );
    lines.push(
        r#"<link rel="icon" href="/favicon-16x16.png" type="image/png" sizes="16x16">"#.to_string(),
    );
    lines.push(r#"<link rel="apple-touch-icon" href="/apple-touch-icon.png">"#.to_string());
    lines.push(r#"<link rel="manifest" href="/site.webmanifest">"#.to_string());
    lines.push(format!(
        r#"<meta name="theme-color" content="{}">"#,
        html_attribute(theme_color)
    ));
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_string("plain"), "\"plain\"");
        assert_eq!(json_string("a \"b\" \\ c"), "\"a \\\"b\\\" \\\\ c\"");
        assert_eq!(
            json_string("line\nbreak\t\u{1}"),
            "\"line\\nbreak\\t\\u0001\""
        );
        assert_eq!(json_string("Ünïcode ✓"), "\"Ünïcode ✓\"");
    }

    #[test]
    fn the_manifest_names_the_icons_and_the_colors() {
        let text = manifest("My \"App\"", "#112233", "#ffffff");
        assert!(text.contains("\"name\": \"My \\\"App\\\"\""));
        assert!(text.contains("icon-192.png"));
        assert!(text.contains("icon-512.png"));
        assert!(text.contains("\"theme_color\": \"#112233\""));
        assert!(text.contains("\"display\": \"standalone\""));
        // A name is never left empty.
        assert!(manifest("  ", "#000000", "#ffffff").contains("\"name\": \"My site\""));
    }

    #[test]
    fn the_head_lines_match_the_files_of_the_package() {
        let with = head_snippet(true, "#336699");
        for file in [
            "favicon.ico",
            "favicon.svg",
            "favicon-32x32.png",
            "favicon-16x16.png",
            "apple-touch-icon.png",
            "site.webmanifest",
        ] {
            assert!(with.contains(file), "{file}");
        }
        assert!(with.contains("content=\"#336699\""));
        assert!(!head_snippet(false, "#336699").contains("favicon.svg"));
        assert!(head_snippet(false, "\"><x").contains("&quot;&gt;&lt;x"));
    }
}
