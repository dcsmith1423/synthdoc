use docx_rs::*;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;

// ---------- SCHEMA ----------

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BodyElement {
    #[serde(rename = "text")]
    TextSpan(TextSpan),
    #[serde(rename = "pagebreak")]
    PageBreak(PageBreak),
    #[serde(rename = "image")]
    Image(Image),
    #[serde(rename = "list")]
    List(ListElement),
    #[serde(rename = "table")]
    Table(TableElement),
    #[serde(rename = "footnote")]
    FootnoteRef(FootnoteRef),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub href: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageBreak {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub src: String,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListElement {
    #[serde(default)]
    pub ordered: bool,
    pub items: Vec<Vec<BodyElement>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableElement {
    #[serde(default)]
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<BodyElement>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FootnoteRef {
    pub ref_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Section {
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub body: Vec<BodyElement>,
    #[serde(default)]
    pub subsections: Vec<Section>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    pub signer: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Footnote {
    pub id: String,
    pub content: Vec<BodyElement>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    pub doc_type: String,
    pub title: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default, rename = "from")]
    pub from_: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub attachments: Option<Vec<Attachment>>,
    #[serde(default)]
    pub signatures: Option<Vec<Signature>>,
    #[serde(default)]
    pub footnotes: Option<Vec<Footnote>>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
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
            BodyElement::Image(img) => {
                let caption = img.caption.clone().unwrap_or_default();
                let text = format!("[Image: {}] {}", img.src, caption);
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(&text)))
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
            BodyElement::Table(table) => {
                let mut tbl = Table::new(vec![]);
                if let Some(headers) = &table.headers {
                    let head_row = TableRow::new(
                        headers.iter()
                            .map(|h| TableCell::new()
                                .add_paragraph(Paragraph::new()
                                    .add_run(Run::new().add_text(h))))
                            .collect()
                    );
                    tbl = tbl.add_row(head_row);
                }
                for row in &table.rows {
                    let row_cells = row.iter().map(|cell| {
                        if let BodyElement::TextSpan(t) = cell {
                            TableCell::new()
                              .add_paragraph(Paragraph::new().add_run(Run::new().add_text(&t.text)))
                        } else {
                            TableCell::new()
                              .add_paragraph(Paragraph::new().add_run(Run::new().add_text("[?]")))

                        }
                    }).collect();
                    tbl = tbl.add_row(TableRow::new(row_cells));
                }
                doc.add_table(tbl)
            }
            BodyElement::FootnoteRef(_) => {
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("[Footnote Ref]")))

            }
        };
    }
    doc
}

/// Append a section (with optional heading and nested subsections).
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

/// The main entry: build the doc, serialize to bytes.
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

// ---------- PYTHON BINDING ----------

#[pyfunction]
fn generate_docx_from_json(py: Python, doc_json: &str) -> PyResult<PyObject> {
    let document: Document = serde_json::from_str(doc_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let docx_bytes = document_to_docx_bytes(&document)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyBytes::new(py, &docx_bytes).into())
}

#[pymodule]
fn docgen(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_docx_from_json, m)?)?;
    Ok(())
}