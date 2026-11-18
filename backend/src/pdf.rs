//! PDF text extraction via the system `pdftotext` (poppler-utils).
//!
//! Keeping this in a small module makes the dependency explicit: the runtime
//! image installs `poppler-utils`, and a missing binary produces a clear
//! actionable error rather than an opaque failure. Bytes are streamed through
//! stdin so no temporary files are needed.

use crate::error::{AppError, AppResult};
use std::io::Write;
use std::process::{Command, Stdio};

/// Extract readable text from PDF bytes using `pdftotext -layout - -`.
pub fn extract_text(bytes: &[u8]) -> AppResult<String> {
    if bytes.len() < 5 || &bytes[..4] != b"%PDF" {
        return Err(AppError::Parse(
            "uploaded file does not look like a PDF (missing %PDF header)".into(),
        ));
    }

    let mut child = Command::new("pdftotext")
        .arg("-layout")
        .arg("-")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Parse(format!(
            "PDF text extraction needs the `pdftotext` binary (poppler-utils): {e}"
        )))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(bytes).map_err(|e| AppError::Parse(format!("could not stream PDF: {e}")))?;
    }
    let output = child.wait_with_output()
        .map_err(|e| AppError::Parse(format!("pdftotext failed: {e}")))?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Parse(format!(
            "pdftotext could not read this PDF{}",
            if detail.is_empty() { String::new() } else { format!(": {detail}") }
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().len() < 80 {
        return Err(AppError::Parse(
            "no readable text found in the PDF (it may be a scanned image)".into(),
        ));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_pdf_bytes_before_spawning() {
        let err = extract_text(b"hello world").unwrap_err();
        assert!(err.to_string().contains("%PDF"));
    }
}
