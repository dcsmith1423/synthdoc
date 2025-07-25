import json

import docgen

doc = {
    "doc_type": "memo",
    "title": "Test Document",
    "author": "Jane Doe",
    "sections": [
        {
            "heading": "Introduction",
            "body": [
                {"type": "text", "text": "This is a test paragraph.", "bold": True},
                {"type": "pagebreak"},
                {"type": "text", "text": "Second page."},
            ],
            "subsections": [],
        }
    ],
}

docx_bytes = docgen.generate_docx_from_json(json.dumps(doc))
with open("out.docx", "wb") as f:
    f.write(docx_bytes)

pdf_bytes = docgen.generate_pdf_from_json(json.dumps(doc))
with open("pdf_out.pdf", "wb") as fo:
    fo.write(pdf_bytes)
