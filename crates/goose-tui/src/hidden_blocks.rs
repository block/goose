pub const CWD_ANALYSIS_TAG: &str = "cwd_analysis_goose_tui";
pub const ATTACHED_FILES_TAG: &str = "attached_files_goose_tui";

pub fn strip_hidden_blocks(text: &str, is_first_user_message: bool) -> String {
    let mut result = text.to_string();
    if is_first_user_message {
        result = strip_block(&result, CWD_ANALYSIS_TAG);
    }
    strip_block(&result, ATTACHED_FILES_TAG)
}

fn strip_block(text: &str, tag: &str) -> String {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");

    let Some(start) = text.find(&start_tag) else {
        return text.to_string();
    };
    let Some(end_offset) = text[start + start_tag.len()..].find(&end_tag) else {
        return text.to_string();
    };

    let end = start + start_tag.len() + end_offset + end_tag.len();
    let before = text[..start].trim_end();
    let after = text[end..].trim_start();

    match (before.is_empty(), after.is_empty()) {
        (_, true) => before.to_string(),
        (true, _) => after.to_string(),
        _ => format!("{before}\n\n{after}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_hidden_blocks_appropriately() {
        let with_both = "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage\n\n<attached_files_goose_tui>\nfiles\n</attached_files_goose_tui>";
        assert_eq!(strip_hidden_blocks(with_both, true), "Message");
        assert_eq!(
            strip_hidden_blocks(with_both, false),
            "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage"
        );

        assert_eq!(strip_hidden_blocks("plain text", true), "plain text");
    }
}
