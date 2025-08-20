import tempfile
from pathlib import Path

import synthdoc  # your PyO3 module name from #[pymodule] fn synthdoc(...)

fp_basic_template = "/home/dansmith1423/code/python/synthdoc/examples/fake_proposal_template_basic.md"

TEMPLATE_MD = Path(fp_basic_template).read_text(encoding="utf-8")

def main():
    # No template variables — pass an empty JSON context.
    ctx_json = "{}"

    # DOCX
    docx_bytes = synthdoc.generate_from_markdown(TEMPLATE_MD, ctx_json, "docx")
    with tempfile.NamedTemporaryFile(prefix="doc_", suffix=".docx", delete=False) as fdocx:
        fdocx.write(docx_bytes)
        docx_path = Path(fdocx.name)

    # PDF (requires Typst CLI available in PATH for your current backend)
    pdf_bytes = synthdoc.generate_from_markdown(TEMPLATE_MD, ctx_json, "pdf")
    with tempfile.NamedTemporaryFile(prefix="doc_", suffix=".pdf", delete=False) as fpdf:
        fpdf.write(pdf_bytes)
        pdf_path = Path(fpdf.name)

    print(f"DOCX written to: {docx_path}")
    print(f"PDF  written to: {pdf_path}")

if __name__ == "__main__":
    main()

