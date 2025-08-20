use crate::schema::*;

fn esc(s: &str) -> String {
    s.replace('#', "\\#").replace('[', "\\[").replace(']', "\\]")
}

fn render_inline(el: &BodyElement) -> String {
    match el {
        BodyElement::TextSpan { text, style } => {
            let mut t = esc(text);
            if style.code { t = format!("`{}`", t); }
            if style.bold { t = format!("**{}**", t); }
            if style.italic { t = format!("*{}*", t); }
            if style.underline { /* Typst lacks underline markdown; could style via show rules */ }
            if let Some(href) = &style.href {
                t = format!("[{}]({})", t, href);
            }
            t
        }
        _ => String::new(),
    }
}

fn render_para(inlines: &[BodyElement]) -> String {
    inlines.iter().map(render_inline).collect::<Vec<_>>().join("")
}

fn render_body(body: &[BodyElement]) -> String {
    let mut out = String::new();
    for el in body {
        match el {
            BodyElement::Paragraph(inlines) => {
                out.push_str(&render_para(inlines));
                out.push_str("\n\n");
            }
            BodyElement::List { ordered, items } => {
                for item in items {
                    let bullet = if *ordered { "1." } else { "-" };
                    let text = item.iter().map(|e| {
                        match e {
                            BodyElement::Paragraph(inl) => render_para(inl),
                            _ => String::new()
                        }
                    }).collect::<Vec<_>>().join(" ");
                    out.push_str(&format!("{} {}\n", bullet, text));
                }
                out.push('\n');
            }
            BodyElement::Image { src, caption, .. } => {
                if let Some(c) = caption {
                    out.push_str(&format!("figure(image(\"{}\"), caption: [{}])\n\n", src, esc(c)));
                } else {
                    out.push_str(&format!("image(\"{}\")\n\n", src));
                }
            }
            BodyElement::Table { headers, rows } => {
                // Simple grid table
                out.push_str("#table(\n");
                if let Some(hs) = headers {
                    let head_row = hs.iter().map(|h| format!("[{}]", esc(h))).collect::<Vec<_>>().join(", ");
                    out.push_str(&format!("  [{}],\n", head_row));
                }
                for r in rows {
                    let cells = r.iter().map(|c| {
                        if let BodyElement::Paragraph(inl) = c { format!("[{}]", esc(&render_para(inl))) }
                        else { "[]".to_string() }
                    }).collect::<Vec<_>>().join(", ");
                    out.push_str(&format!("  [{}],\n", cells));
                }
                out.push_str(")\n\n");
            }
            BodyElement::BlockQuote(children) => {
                let content = children.iter().map(render_inline).collect::<String>();
                out.push_str(&format!("quote([{}])\n\n", esc(&content)));
            }
            BodyElement::CodeBlock { lang, code } => {
                let lang = lang.clone().unwrap_or_else(|| "text".into());
                out.push_str(&format!("```{}\n{}\n```\n\n", lang, code));
            }
            BodyElement::PageBreak => out.push_str("#pagebreak()\n"),
            BodyElement::FootnoteRef(name) => out.push_str(&format!("[^{}]\n\n", name)),
            BodyElement::Callout { kind, content } => {
                let txt = content.iter().map(render_inline).collect::<String>();
                out.push_str(&format!("note[{}: {}]\n\n", kind, esc(&txt)));
            }
            BodyElement::TableOfContents => out.push_str("#outline()\n\n"),
            BodyElement::TextSpan { .. } => {
                out.push_str(&render_inline(el));
                out.push_str("\n\n");
            }
        }
    }
    out
}

fn render_section(sec: &Section) -> String {
    let mut out = String::new();
    if !sec.heading.is_empty() {
        out.push_str(&"#".repeat(sec.level as usize));
        out.push(' ');
        out.push_str(&sec.heading);
        out.push_str("\n\n");
    }
    out.push_str(&render_body(&sec.body));
    for s in &sec.subsections {
        out.push_str(&render_section(s));
    }
    out
}

pub fn document_to_typst(document: &Document) -> String {
    let mut preamble = String::from(
        r#"#set page(margin: 1in)
#set par(justify: true)
"#,
    );

    if let Some(title) = document.metadata.title.as_ref() {
        preamble.push_str(&format!("#set document(title: \"{}\")\n", title));
    }
    if let Some(author) = document.metadata.author.as_ref() {
        preamble.push_str(&format!("#set document(author: \"{}\")\n", author));
    }

    let mut body = String::new();
    for s in &document.sections {
        body.push_str(&render_section(s));
    }

    format!("{}{}\n", preamble, body)
}
