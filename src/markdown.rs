use crate::schema::*;
use anyhow::Result;
use handlebars::Handlebars;
use pulldown_cmark::{
    Options, Parser, Event, Tag, TagEnd, CodeBlockKind,
    HeadingLevel,
};
use serde_json::Value;
use serde_yaml;
use std::collections::HashMap;

pub fn render_template_to_markdown(template_md: &str, context_json: &str) -> Result<String> {
    let ctx: Value = serde_json::from_str(context_json)?;
    let mut hbs = Handlebars::new();
    hbs.register_escape_fn(handlebars::no_escape);
    let rendered = hbs.render_template(template_md, &ctx)?;
    Ok(rendered)
}

/// Optional YAML front matter:
/// ---\nkey: val\n---\n# Title ...
fn split_front_matter(md: &str) -> (Option<HashMap<String, Value>>, &str) {
    let s = md.trim_start();
    if s.starts_with("---\n") {
        if let Some(idx) = s[4..].find("\n---") {
            let yaml = &s[4..4 + idx];
            let rest = &s[4 + idx + 4..];
            let map = serde_yaml::from_str::<HashMap<String, Value>>(yaml).ok();
            return (map, rest);
        }
    }
    (None, md)
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn markdown_to_document(md: &str) -> Result<Document> {
    let (front, body) = split_front_matter(md);
    let mut doc = Document::default();
    doc.front_matter = front;

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(body, opts);

    // Build nested sections by tracking heading levels
    let mut section_stack: Vec<Section> = vec![Section { level: 0, ..Default::default() }];
    let mut current_para: Vec<BodyElement> = vec![];
    let mut list_stack: Vec<(bool, Vec<Vec<BodyElement>>)> = vec![]; // (ordered, items)

    let mut current_text_style = InlineStyle::default();

    // For headings
    let mut pending_heading_level: Option<u8> = None;
    let mut heading_fragments: String = String::new();

    // For code blocks
    let mut in_codeblock: bool = false;
    let mut code_lang: Option<String> = None;
    let mut code_buf: String = String::new();

    fn flush_para_to(section: &mut Section, para: &mut Vec<BodyElement>) {
        if !para.is_empty() {
            section.body.push(BodyElement::Paragraph(std::mem::take(para)));
        }
    }

    for ev in parser {
        match ev {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    // Finish any open paragraph in current section
                    let top = section_stack.last_mut().unwrap();
                    flush_para_to(top, &mut current_para);

                    pending_heading_level = Some(heading_level_to_u8(level));
                    heading_fragments.clear();
                }
                Tag::Paragraph => { /* content will go into current_para */ }
                Tag::Emphasis => { current_text_style.italic = true; }
                Tag::Strong => { current_text_style.bold = true; }
                Tag::Strikethrough => { current_text_style.strike = true; }
                Tag::CodeBlock(kind) => {
                    in_codeblock = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                        CodeBlockKind::Indented => None,
                    };
                    code_buf.clear();
                }
                Tag::BlockQuote => {
                    // (Optional) implement full blockquote capture; ignore for now.
                }
                Tag::List(start) => {
                    // ordered if a start number exists
                    let ordered = start.is_some();
                    list_stack.push((ordered, vec![]));
                }
                Tag::Item => {
                    current_para.clear();
                }
                Tag::Link { dest_url, .. } => {
                    current_text_style.href = Some(dest_url.to_string());
                }
                Tag::Image { dest_url, title, .. } => {
                    let img = BodyElement::Image {
                        src: dest_url.to_string(),
                        alt: if title.is_empty() { None } else { Some(title.to_string()) },
                        caption: None,
                    };
                    current_para.push(img);
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_level) => {
                    // Create and attach a new section with this heading
                    let lvl = pending_heading_level.take().unwrap_or(1);
                    let mut new_sec = Section {
                        level: lvl,
                        heading: heading_fragments.clone(),
                        ..Default::default()
                    };

                    // Attach under the nearest parent with lower level
                    while section_stack.last().map(|s| s.level >= lvl).unwrap_or(false) {
                        let child = section_stack.pop().unwrap();
                        if let Some(parent) = section_stack.last_mut() {
                            parent.subsections.push(child);
                        }
                    }
                    if let Some(parent) = section_stack.last_mut() {
                        parent.subsections.push(std::mem::take(&mut new_sec));
                    }
                    // Push a work section at this level to collect upcoming body content
                    section_stack.push(Section { level: lvl, ..Default::default() });
                }
                TagEnd::Paragraph => {
                    let top = section_stack.last_mut().unwrap();
                    flush_para_to(top, &mut current_para);
                }
                TagEnd::Emphasis => { current_text_style.italic = false; }
                TagEnd::Strong => { current_text_style.bold = false; }
                TagEnd::Strikethrough => { current_text_style.strike = false; }
                TagEnd::List(_is_tight) => {
                    let (ordered, items) = list_stack.pop().unwrap();
                    let top = section_stack.last_mut().unwrap();
                    top.body.push(BodyElement::List { ordered, items });
                }
                TagEnd::Item => {
                    // One list item ends: turn current_para into a Vec<BodyElement>
                    let item: Vec<BodyElement> = if current_para.is_empty() {
                        vec![]
                    } else {
                        vec![BodyElement::Paragraph(std::mem::take(&mut current_para))]
                    };
                    let (ordered, mut items) = list_stack.pop().unwrap();
                    items.push(item);
                    list_stack.push((ordered, items));
                }
                TagEnd::Link => {
                    current_text_style.href = None;
                }
                TagEnd::BlockQuote => { /* ignored for now */ }
                TagEnd::CodeBlock => {
                    in_codeblock = false;
                    let top = section_stack.last_mut().unwrap();
                    top.body.push(BodyElement::CodeBlock {
                        lang: code_lang.take(),
                        code: std::mem::take(&mut code_buf),
                    });
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_codeblock {
                    code_buf.push_str(&t);
                } else if pending_heading_level.is_some() {
                    heading_fragments.push_str(&t);
                } else {
                    let style = current_text_style.clone();
                    current_para.push(BodyElement::TextSpan { text: t.to_string(), style });
                }
            }
            Event::Code(t) => {
                if in_codeblock {
                    code_buf.push_str(&t);
                } else {
                    let mut style = current_text_style.clone();
                    style.code = true;
                    current_para.push(BodyElement::TextSpan { text: t.to_string(), style });
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_codeblock {
                    code_buf.push('\n');
                } else {
                    current_para.push(BodyElement::TextSpan { text: "\n".into(), style: current_text_style.clone() });
                }
            }
            Event::FootnoteReference(name) => {
                current_para.push(BodyElement::FootnoteRef(name.to_string()));
            }
            _ => {}
        }
    }

    // Flush trailing paragraph content
    if let Some(top) = section_stack.last_mut() {
        if !current_para.is_empty() {
            top.body.push(BodyElement::Paragraph(std::mem::take(&mut current_para)));
        }
    }

    // Collapse stack into root
    let mut root = Section { level: 0, ..Default::default() };
    while let Some(mut s) = section_stack.pop() {
        if s.level == 0 {
            root = s;
            break;
        } else {
            if let Some(parent) = section_stack.last_mut() {
                parent.subsections.push(s);
            }
        }
    }

    doc.sections = root.subsections;
    Ok(doc)
}
