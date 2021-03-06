pub(crate) fn shorten_content(content: &str) -> String {
    let max_length = 72;
    if content.len() <= max_length {
        content.to_owned()
    } else {
        content.chars().take(max_length).collect::<String>() + "…"
    }
}

pub(crate) fn shorten_content_length(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_owned()
    } else {
        content.chars().take(max_length).collect::<String>() + "…"
    }
}
