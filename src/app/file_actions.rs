// Slint's text editor eagerly lays out its entire value. Keep the editor
// responsive and avoid renderer crashes when an otherwise valid TAP is huge.
const MAX_SOURCE_EDITOR_BYTES: usize = 10 * 1024;

/// Produce editor-safe text while preserving the fact that a source was cut.
pub(crate) fn source_gcode_for_editor(content: &str) -> (String, bool) {
    if content.len() <= MAX_SOURCE_EDITOR_BYTES {
        return (content.to_owned(), false);
    }

    let max_end = content.floor_char_boundary(MAX_SOURCE_EDITOR_BYTES);
    let end = content[..max_end].rfind('\n').unwrap_or(max_end);
    (
        format!(
            "{}\n\n; --- Display limited to the first {} KB of a large source file ---\n",
            &content[..end],
            MAX_SOURCE_EDITOR_BYTES / 1024
        ),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_source_is_truncated_at_a_character_boundary() {
        let content = format!("{}\nG1 X1", "Ж".repeat(MAX_SOURCE_EDITOR_BYTES));
        let (editor_text, truncated) = source_gcode_for_editor(&content);

        assert!(truncated);
        assert!(editor_text.is_char_boundary(editor_text.len()));
    }
}
