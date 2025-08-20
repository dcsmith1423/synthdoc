use anyhow::Result;
use std::io::Cursor;

// Import ONLY what we need from docx-rs, to avoid name clashes.
use docx_rs::{Docx, Paragraph, Run, Table, TableCell, TableRow, NumberingId, IndentLevel};

// Bring in OUR AST types explicitly and alias them so they can't collide.
use crate::schema::{
    BodyElement,
    InlineStyle,
    Section as AstSection,
    Document as AstDocument,
};

fn add_run_from_span(run: Run, text: &str, style: &InlineStyle) -> Run {
    let mut r = run.add_text(text);
    if style.bold { r = r.bold(); }
    if style.italic { r = r.italic(); }
    if style.underline { r = r.underline("single"); }
    // TODO: strike/code/font/href mapping in DOCX if needed
    r
}

fn add_body_to_doc(mut doc: Docx, body: &[BodyElement]) -> Docx {
    for elem in body {
        doc = match elem {
            BodyElement::Paragraph(inlines) => {
                let mut para = Paragraph::new();
                let mut run = Run::new();
                for e in inlines {
                    match e {
                        BodyElement::TextSpan { text, style } => {
                            run = add_run_from_span(run, text, style);
                        }
                        BodyElement::Image { src, caption, .. } => {
                            // Placeholder image handling
                            let t = format!("[Image: {}] {}", src, caption.clone().unwrap_or_default());
                            run = run.add_text(&t);
                        }
                        _ => {}
                    }
                }
                para = para.add_run(run);
                doc.add_paragraph(para)
            }
            BodyElement::List { ordered, items } => {
                // Minimal numbering: define explicit numbering later if desired.
                let mut d = doc;
                for item in items {
                    let mut para = Paragraph::new();
                    let mut run = Run::new();
                    for e in item {
                        if let BodyElement::Paragraph(inlines) = e {
                            for el in inlines {
                                if let BodyElement::TextSpan { text, style } = el {
                                    run = add_run_from_span(run, text, style);
                                }
                            }
                        }
                    }
                    para = para.add_run(run);
                    let num_id = if *ordered { 1 } else { 2 };
                    para = para.numbering(NumberingId::new(num_id), IndentLevel::new(0));
                    d = d.add_paragraph(para);
                }
                d
            }
            BodyElement::Table { headers, rows } => {
                let mut tbl = Table::new(vec![]);
                if let Some(hs) = headers {
                    let head = TableRow::new(
                        hs.iter()
                          .map(|h| TableCell::new()
                                .add_paragraph(Paragraph::new()
                                    .add_run(Run::new().bold().add_text(h))))
                          .collect()
                    );
                    tbl = tbl.add_row(head);
                }
                for row in rows {
                    let cells = row.iter().map(|cell| {
                        match cell {
                            BodyElement::Paragraph(inlines) => {
                                let mut run = Run::new();
                                for el in inlines {
                                    if let BodyElement::TextSpan { text, style } = el {
                                        run = add_run_from_span(run, text, style);
                                    }
                                }
                                TableCell::new().add_paragraph(Paragraph::new().add_run(run))
                            }
                            _ => TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(" "))),
                        }
                    }).collect();
                    tbl = tbl.add_row(TableRow::new(cells));
                }
                doc.add_table(tbl)
            }
            BodyElement::BlockQuote(children) => {
                let mut d = doc;
                let mut para = Paragraph::new().indent(Some(720), None, None, None); // left indent
                let mut run = Run::new();
                for c in children {
                    if let BodyElement::TextSpan { text, style } = c {
                        run = add_run_from_span(run, text, style);
                    }
                }
                para = para.add_run(run);
                d = d.add_paragraph(para);
                d
            }
            BodyElement::CodeBlock { code, .. } => {
                let para = Paragraph::new().add_run(Run::new().add_text(code));
                doc.add_paragraph(para)
            }
            BodyElement::PageBreak => {
                doc.add_paragraph(Paragraph::new().page_break_before(true))
            }
            BodyElement::FootnoteRef(name) => {
                let para = Paragraph::new().add_run(Run::new().add_text(&format!("[^{}]", name)));
                doc.add_paragraph(para)
            }
            BodyElement::Callout { kind, content } => {
                let mut run = Run::new().add_text(&format!("[{}] ", kind.to_uppercase()));
                for c in content {
                    if let BodyElement::TextSpan { text, style } = c {
                        run = add_run_from_span(run, text, style);
                    }
                }
                doc.add_paragraph(Paragraph::new().add_run(run))
            }
            BodyElement::TableOfContents => {
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("[Table of Contents]")))
            }
            BodyElement::TextSpan { text, style } => {
                let para = Paragraph::new().add_run(add_run_from_span(Run::new(), text, style));
                doc.add_paragraph(para)
            }
            // Handle top-level images (not only inside paragraphs)
            BodyElement::Image { src, caption, .. } => {
                let t = format!("[Image: {}] {}", src, caption.clone().unwrap_or_default());
                let para = Paragraph::new().add_run(Run::new().add_text(&t));
                doc.add_paragraph(para)
            }
        };
    }
    doc
}

fn add_section(mut doc: Docx, section: &AstSection) -> Docx {
    if !section.heading.is_empty() {
        let run = Run::new().add_text(&section.heading).bold();
        let para = Paragraph::new().add_run(run);
        doc = doc.add_paragraph(para);
    }
    doc = add_body_to_doc(doc, &section.body);
    for sub in &section.subsections {
        doc = add_section(doc, sub);
    }
    doc
}

pub fn document_to_docx_bytes(document: &AstDocument) -> Result<Vec<u8>> {
    let mut doc = Docx::new();

    if let Some(title) = document.metadata.title.as_ref() {
        let title_run = Run::new().add_text(title).bold().size(48);
        let title_para = Paragraph::new().add_run(title_run);
        doc = doc.add_paragraph(title_para);
    }

    for s in &document.sections {
        doc = add_section(doc, s);
    }

    let mut buf = Cursor::new(Vec::new());
    doc.build().pack(&mut buf)?;
    Ok(buf.into_inner())
}
