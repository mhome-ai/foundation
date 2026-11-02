//! Markdown image references in Agent Profile documents. Code examples are not media.
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileImage {
    pub range: Range<usize>,
    pub uri: String,
    pub alt: String,
    pub title: String,
}

/// Select only actual Markdown images using the private Artifact scheme.
/// Reference-style images are supported and rewritten to canonical inline images.
pub fn images(content: &str) -> Vec<ProfileImage> {
    let mut result = Vec::new();
    let mut current: Option<ProfileImage> = None;
    let mut image_depth = 0usize;
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                if image_depth == 0 && dest_url.starts_with("meow-artifact:") {
                    current = Some(ProfileImage {
                        range,
                        uri: dest_url.into_string(),
                        title: title.into_string(),
                        alt: String::new(),
                    });
                }
                image_depth += 1;
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(image) = &mut current {
                    image.alt.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(image) = &mut current {
                    image.alt.push(' ');
                }
            }
            Event::End(TagEnd::Image) => {
                image_depth -= 1;
                if image_depth == 0 {
                    if let Some(image) = current.take() {
                        result.push(image);
                    }
                }
            }
            _ => {}
        }
    }
    result
}

/// Rewrite image nodes only; prose, code, other links and unrelated Markdown remain intact.
pub fn rewrite(
    content: &str,
    images: &[ProfileImage],
    replacements: &HashMap<String, String>,
) -> String {
    let mut result = content.to_owned();
    for image in images.iter().rev() {
        if let Some(uri) = replacements.get(&image.uri) {
            let alt = image
                .alt
                .replace('\\', "\\\\")
                .replace('[', "\\[")
                .replace(']', "\\]");
            let title = image.title.replace('\\', "\\\\").replace('"', "\\\"");
            let title = if title.is_empty() {
                String::new()
            } else {
                format!(" \"{title}\"")
            };
            result.replace_range(image.range.clone(), &format!("![{alt}]({uri}{title})"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_rewrite_preserves_prose_code_and_reference_meaning() {
        let text = "Ali.\n![Ali](meow-artifact://old \"photo\")\n`![code](meow-artifact://ignored)`\n\n![Other][photo]\n\n[photo]: meow-artifact://second\n";
        let found = images(text);
        assert_eq!(
            found.iter().map(|x| x.uri.as_str()).collect::<Vec<_>>(),
            ["meow-artifact://old", "meow-artifact://second"]
        );
        let result = rewrite(
            text,
            &found,
            &HashMap::from([
                (
                    "meow-artifact://old".into(),
                    "meow-artifact://durable".into(),
                ),
                (
                    "meow-artifact://second".into(),
                    "meow-artifact://other".into(),
                ),
            ]),
        );
        assert!(result.contains("![Ali](meow-artifact://durable \"photo\")"));
        assert!(result.contains("![Other](meow-artifact://other)"));
        assert!(result.contains("`![code](meow-artifact://ignored)`"));
    }
}

#[cfg(test)]
mod nested_tests {
    use super::*;
    #[test]
    fn unicode_multiline_and_nested_alt_have_one_visible_image() {
        let content = "猫。\n![阿狸 ![内层](meow-artifact://inner)\n尾巴](meow-artifact://outer)";
        let nodes = images(content);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].uri, "meow-artifact://outer");
        let rewritten = rewrite(
            content,
            &nodes,
            &HashMap::from([(
                "meow-artifact://outer".into(),
                "meow-artifact://owned".into(),
            )]),
        );
        assert_eq!(images(&rewritten)[0].uri, "meow-artifact://owned");
        assert!(rewritten.starts_with("猫。\n"));
        assert!(
            images("```md\n![x](meow-artifact://code)\n```\n[x](meow-artifact://link)").is_empty()
        );
    }
}
