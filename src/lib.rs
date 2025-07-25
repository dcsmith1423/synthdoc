use docx_rs::*;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pulldown_cmark::{Parser, Event, Tag};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use genpdf::Element;

// ---------- SCHEMA ----------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BodyElement {
    TextSpan(TextSpan),
    PageBreak(PageBreak),
    List(ListElement),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font: Option<String>,
    pub href: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageBreak {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListElement {
    pub ordered: bool,
    pub items: Vec<Vec<BodyElement>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Section {
    pub heading: Option<String>,
    pub body: Vec<BodyElement>,
    pub subsections: Vec<Section>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    pub doc_type: String,
    pub title: String,
    pub date: Option<String>,
    pub author: Option<String>,
    pub from_: Option<String>,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub sections: Vec<Section>,
    pub attachments: Option<Vec<()>>,
    pub signatures: Option<Vec<()>>,
    pub footnotes: Option<Vec<()>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ---------- MARKDOWN PARSING ----------

fn markdown_to_document(md: &str, title: &str) -> Document {
    let parser = Parser::new(md);

    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;
    let mut current_body: Vec<BodyElement> = Vec::new();
    let mut in_list = false;
    let mut list_items: Vec<Vec<BodyElement>> = Vec::new();
    let mut list_ordered = false;
    let mut text_style = (false, false, false); // bold, italic, underline

    for event in parser {
        match event {
            Event::Start(Tag::Heading(_level, ..)) => {
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }
                current_body = Vec::new();
            }
            Event::End(Tag::Heading(_level, ..)) => {
                if let Some(heading_text) = current_body.iter().filter_map(|e| {
                    if let BodyElement::TextSpan(t) = e {
                        Some(t.text.clone())
                    } else {
                        None
                    }
                }).next() {
                    current_section = Some(Section {
                        heading: Some(heading_text),
                        body: Vec::new(),
                        subsections: Vec::new(),
                    });
                }
                current_body = Vec::new();
            }
            Event::Start(Tag::List(Some(1))) => {
                in_list = true;
                list_ordered = true;
                list_items = Vec::new();
            }
            Event::Start(Tag::List(None)) => {
                in_list = true;
                list_ordered = false;
                list_items = Vec::new();
            }
            Event::End(Tag::List(_)) => {
                current_body.push(BodyElement::List(ListElement {
                    ordered: list_ordered,
                    items: list_items.clone(),
                }));
                in_list = false;
            }
            Event::Start(Tag::Item) => {
                list_items.push(Vec::new());
            }
            Event::End(Tag::Item) => {}
            Event::Text(text) => {
                if in_list {
                    if let Some(last) = list_items.last_mut() {
                        last.push(BodyElement::TextSpan(TextSpan {
                            text: text.to_string(),
                            bold: false,
                            italic: false,
                            underline: false,
                            font: None,
                            href: None,
                        }));
                    }
                } else {
                    let trimmed = text.trim();
                    if trimmed == "[[PAGEBREAK]]" {
                        current_body.push(BodyElement::PageBreak(PageBreak {}));
                    } else {
                        current_body.push(BodyElement::TextSpan(TextSpan {
                            text: text.to_string(),
                            bold: text_style.0,
                            italic: text_style.1,
                            underline: text_style.2,
                            font: None,
                            href: None,
                        }));
                    }
                }
            }
            Event::Start(Tag::Emphasis) => { text_style.1 = true; }
            Event::End(Tag::Emphasis) => { text_style.1 = false; }
            Event::Start(Tag::Strong) => { text_style.0 = true; }
            Event::End(Tag::Strong) => { text_style.0 = false; }
            Event::SoftBreak | Event::HardBreak => {
                current_body.push(BodyElement::TextSpan(TextSpan {
                    text: "\n".to_string(),
                    bold: false,
                    italic: false,
                    underline: false,
                    font: None,
                    href: None,
                }));
            }
            Event::End(Tag::Paragraph) => {}
            _ => {}
        }
    }

    if let Some(section) = current_section.take() {
        sections.push(section);
    }
    if sections.is_empty() && !current_body.is_empty() {
        sections.push(Section {
            heading: None,
            body: current_body.clone(),
            subsections: vec![],
        });
    }

    Document {
        doc_type: "testdoc".to_string(),
        title: title.to_string(),
        date: None,
        author: None,
        from_: None,
        to: None,
        subject: None,
        sections,
        attachments: None,
        signatures: None,
        footnotes: None,
        metadata: None,
    }
}


// ---------- DOCX GENERATION ----------

fn add_body_to_doc(mut doc: Docx, body: &[BodyElement]) -> Docx {
    for elem in body {
        doc = match elem {
            BodyElement::TextSpan(t) => {
                let mut run = Run::new().add_text(&t.text);
                if t.bold     { run = run.bold(); }
                if t.italic   { run = run.italic(); }
                if t.underline{ run = run.underline("single"); }
                let para = Paragraph::new().add_run(run);
                doc.add_paragraph(para)
            }
            BodyElement::PageBreak(_) => {
                doc.add_paragraph(Paragraph::new().page_break_before(true))
            }
            BodyElement::List(list) => {
                let mut d = doc;
                for item in &list.items {
                    let mut para = Paragraph::new();
                    for sub in item.iter().flat_map(|e| {
                        if let BodyElement::TextSpan(t) = e { Some(&t.text) }
                        else { None }
                    }) {
                        para = para.add_run(Run::new().add_text(sub));
                    }
                    let num_id = if list.ordered { 1 } else { 2 };
                    para = para.numbering(NumberingId::new(num_id), IndentLevel::new(0));
                    d = d.add_paragraph(para);
                }
                d
            }
        };
    }
    doc
}

fn add_section(mut doc: Docx, section: &Section) -> Docx {
    if let Some(h) = &section.heading {
        doc = doc.add_paragraph(Paragraph::new()
                                  .add_run(Run::new().add_text(h))
                                  .bold());
    }
    doc = add_body_to_doc(doc, &section.body);
    for sub in &section.subsections {
        doc = add_section(doc, sub);
    }
    doc
}

fn document_to_docx_bytes(document: &Document)
  -> Result<Vec<u8>, Box<dyn std::error::Error>>
{
    let mut doc = Docx::new();

    doc = doc.add_paragraph(
        Paragraph::new()
          .add_run(Run::new().add_text(&document.title))
          .bold()
          .size(48)
    );

    if let Some(a) = &document.author {
        let p = Paragraph::new().add_run(Run::new().add_text(&format!("Author: {}", a)));
        doc = doc.add_paragraph(p);
    }
    if let Some(d) = &document.date {
        let p = Paragraph::new().add_run(Run::new().add_text(&format!("Date: {}", d)));
        doc = doc.add_paragraph(p);
    }

    for section in &document.sections {
        doc = add_section(doc, section);
    }

    let mut buf = Cursor::new(Vec::new());
    doc.build().pack(&mut buf)?;
    Ok(buf.into_inner())
}


// ---------- PDF GENERATION ----------

fn document_to_pdf_bytes(document: &Document) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use genpdf::{elements, Alignment};

    let mut doc = genpdf::Document::new(genpdf::fonts::from_files(
        "./fonts", // provide a fonts dir or use the default ones
        "LiberationSans", // fallback
        None,
    )?);

    doc.set_title(document.title.clone());

    doc.push(elements::Paragraph::new(document.title.clone()).aligned(Alignment::Center).styled(genpdf::style::Style::new().bold().with_font_size(24)));
    if let Some(a) = &document.author {
        doc.push(elements::Paragraph::new(format!("Author: {}", a)).aligned(Alignment::Center));
    }
    if let Some(d) = &document.date {
        doc.push(elements::Paragraph::new(format!("Date: {}", d)).aligned(Alignment::Center));
    }
    doc.push(elements::Break::new(1));

    for section in &document.sections {
        if let Some(h) = &section.heading {
            let heading_style = genpdf::style::Style::new().bold().with_font_size(18);
            doc.push(elements::Paragraph::new(h.clone()).styled(heading_style));
        }
        for elem in &section.body {
            match elem {
                BodyElement::TextSpan(t) => {
                    let mut style = genpdf::style::Style::new();
                    if t.bold { style = style.bold(); }
                    if t.italic { style = style.italic(); }
                    let p = elements::Paragraph::new(t.text.clone()).styled(style);
                    doc.push(p);
                }
                BodyElement::PageBreak(_) => {
                    doc.push(elements::PageBreak::new());
                }
                BodyElement::List(list) => {
                    for (i, item) in list.items.iter().enumerate() {
                        let bullet = if list.ordered { format!("{}. ", i + 1) } else { "- ".to_string() };
                        let text: String = item.iter().filter_map(|e| {
                            if let BodyElement::TextSpan(t) = e {
                                Some(t.text.clone())
                            } else {
                                None
                            }
                        }).collect();
                        doc.push(elements::Paragraph::new(format!("{}{}", bullet, text)));
                    }
                }
            }
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    doc.render(&mut buf)?;
    Ok(buf)
}

// ---------- PYTHON BINDING ----------

#[pyfunction]
fn generate_docx_from_markdown(py: Python, markdown: &str, title: Option<&str>) -> PyResult<PyObject> {
    let doc_title = title.unwrap_or("Document");
    let document = markdown_to_document(markdown, doc_title);
    let docx_bytes = document_to_docx_bytes(&document)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBytes::new(py, &docx_bytes).into())
}

#[pyfunction]
fn generate_pdf_from_markdown(py: Python, markdown: &str, title: Option<&str>) -> PyResult<PyObject> {
    let doc_title = title.unwrap_or("Document");
    let document = markdown_to_document(markdown, doc_title);
    let pdf_bytes = document_to_pdf_bytes(&document)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyBytes::new(py, &pdf_bytes).into())
}

#[pymodule]
fn synthdoc(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_docx_from_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(generate_pdf_from_markdown, m)?)?;
    Ok(())
}