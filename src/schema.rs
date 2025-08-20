use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Inline style flags (expandable)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InlineStyle {
    #[serde(default)] pub bold: bool,
    #[serde(default)] pub italic: bool,
    #[serde(default)] pub underline: bool,
    #[serde(default)] pub strike: bool,
    #[serde(default)] pub code: bool,
    #[serde(default)] pub font: Option<String>,
    #[serde(default)] pub href: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BodyElement {
    #[serde(rename = "text")]      TextSpan { text: String, #[serde(default)] style: InlineStyle },
    #[serde(rename = "paragraph")] Paragraph(Vec<BodyElement>),
    #[serde(rename = "blockquote")] BlockQuote(Vec<BodyElement>),
    #[serde(rename = "codeblock")] CodeBlock { lang: Option<String>, code: String },
    #[serde(rename = "pagebreak")] PageBreak,
    #[serde(rename = "image")]     Image { src: String, #[serde(default)] alt: Option<String>, #[serde(default)] caption: Option<String> },
    #[serde(rename = "table")]     Table { headers: Option<Vec<String>>, rows: Vec<Vec<BodyElement>> },
    #[serde(rename = "list")]      List { ordered: bool, items: Vec<Vec<BodyElement>> },
    #[serde(rename = "footnote_ref")] FootnoteRef(String),
    #[serde(rename = "callout")]   Callout { kind: String, content: Vec<BodyElement> }, // e.g., "note", "warning", "tip"
    #[serde(rename = "toc")]       TableOfContents, // placeholder node → renderer builds ToC
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Section {
    pub level: u8,                       // 1..=6
    #[serde(default)] pub heading: String,
    #[serde(default)] pub body: Vec<BodyElement>,
    #[serde(default)] pub subsections: Vec<Section>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Attachment { pub filename: String, #[serde(default)] pub description: Option<String> }

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Signature { pub signer: String, #[serde(default)] pub title: Option<String>, #[serde(default)] pub date: Option<String> }

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Footnote { pub id: String, pub content: Vec<BodyElement> }

/// Document-level styling (expand as needed)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DocStyle {
    #[serde(default)] pub body_font: Option<String>,
    #[serde(default)] pub heading_font: Option<String>,
    #[serde(default)] pub margins_mm: Option<(f32, f32, f32, f32)>, // left, top, right, bottom
    #[serde(default)] pub header_text: Option<String>,
    #[serde(default)] pub footer_text: Option<String>,
    #[serde(default)] pub page_numbers: bool,
    #[serde(default)] pub logo_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Metadata {
    #[serde(default)] pub doc_type: Option<String>,
    #[serde(default)] pub title: Option<String>,
    #[serde(default)] pub date: Option<String>,
    #[serde(default)] pub author: Option<String>,
    #[serde(default, rename="from")] pub from_: Option<String>,
    #[serde(default)] pub to: Option<String>,
    #[serde(default)] pub subject: Option<String>,
    #[serde(default)] pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Document {
    #[serde(default)] pub metadata: Metadata,
    #[serde(default)] pub style: DocStyle,
    #[serde(default)] pub front_matter: Option<HashMap<String, serde_json::Value>>, // from YAML
    #[serde(default)] pub sections: Vec<Section>,
    #[serde(default)] pub attachments: Vec<Attachment>,
    #[serde(default)] pub signatures: Vec<Signature>,
    #[serde(default)] pub footnotes: Vec<Footnote>,
}
