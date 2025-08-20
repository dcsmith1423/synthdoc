use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::{render_template_to_markdown, markdown_to_document, document_to_docx_bytes, document_to_typst};
use crate::pdf::typst_to_pdf_bytes;

#[pyfunction]
fn generate_from_markdown(py: Python, template_markdown: &str, context_json: &str, output_format: &str) -> PyResult<PyObject> {
    let rendered_md = render_template_to_markdown(template_markdown, context_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Template render error: {}", e)))?;

    let doc = markdown_to_document(&rendered_md)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Markdown parse error: {}", e)))?;

    match output_format {
        "docx" => {
            let bytes = document_to_docx_bytes(&doc)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("DOCX render error: {}", e)))?;
            Ok(PyBytes::new(py, &bytes).into())
        }
        "pdf" => {
            let typst_src = document_to_typst(&doc);
            let bytes = typst_to_pdf_bytes(&typst_src)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("PDF render error: {}", e)))?;
            Ok(PyBytes::new(py, &bytes).into())
        }
        other => Err(pyo3::exceptions::PyValueError::new_err(format!("Unsupported output_format: {}", other))),
    }
}

#[pymodule]
fn docgen(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_from_markdown, m)?)?;
    Ok(())
}
