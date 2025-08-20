use anyhow::{Context, Result};
use std::process::Command;
use tempfile::NamedTempFile;
use std::fs;

#[cfg(feature = "pdf-typst-cli")]
pub fn typst_to_pdf_bytes(typst_src: &str) -> Result<Vec<u8>> {
    let typ_file = NamedTempFile::new()?;
    fs::write(typ_file.path(), typst_src)?;

    let pdf_file = NamedTempFile::new()?;
    let status = Command::new("typst")
        .args(["compile", typ_file.path().to_str().unwrap(), pdf_file.path().to_str().unwrap()])
        .status()
        .context("Failed to spawn typst CLI (is it installed and in PATH?)")?;

    if !status.success() {
        anyhow::bail!("typst CLI did not succeed");
    }

    let bytes = fs::read(pdf_file.path())?;
    Ok(bytes)
}

#[cfg(not(feature = "pdf-typst-cli"))]
pub fn typst_to_pdf_bytes(_typst_src: &str) -> Result<Vec<u8>> {
    anyhow::bail!("PDF generation disabled: build with feature `pdf-typst-cli` or provide another PDF backend.");
}
