use std::fmt::Write;

use matrix_sdk::ruma::events::room::message::MessageEventContent;
use url::Url;

const SEPARATOR: &str = "⋅";

enum Style {
    Bold,
    Code,
    Span,
}

impl Style {
    fn close(&self) -> &'static str {
        match self {
            Self::Bold => "</b>",
            Self::Code => "</code>",
            Self::Span => "</span>",
        }
    }
}

#[derive(Default)]
pub struct MessageBuilder {
    pub(crate) html: String,
    pub(crate) plain: String,
    style_stack: Vec<Style>,
    pub(crate) url: Option<Url>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn build(mut self) -> MessageEventContent {
        // Append main URL to plain text message, if we have one
        if let Some(url) = self.url {
            write!(self.plain, " {} {}", SEPARATOR, url).unwrap();
        }

        MessageEventContent::text_html(self.plain, self.html)
    }

    pub fn bold(&mut self) {
        self.html.push_str("<b>");
        self.style_stack.push(Style::Bold);
    }

    pub fn code(&mut self) {
        self.html.push_str("<code>");
        self.style_stack.push(Style::Code);
    }

    pub fn color(&mut self, color: &str) {
        write!(self.html, r#"<span style="color: {}">"#, color).unwrap();
        self.style_stack.push(Style::Span);
    }

    pub fn tag(&mut self, tag: &str, emoji: Option<char>) {
        self.bold();
        write!(self, "[").unwrap();
        if let Some(emoji) = emoji {
            write!(self, "{} ", emoji).unwrap()
        }
        write!(self, "{}]", tag).unwrap();
        self.close_last();
    }

    pub fn link(&mut self, text: &str, href: &Url) {
        // NOTE: we consider that the URL is bonus information, not needed in plain text mode to
        // understand the message
        self.plain.push_str(text);

        write!(self.html, r#"<a href="{}">{}</a>"#, href, text).unwrap();
    }

    /// Format the provided text as an anchor tag, and set the URL to be appended at the end of the
    /// plain text message
    pub fn main_link(&mut self, text: &str, href: &Url) {
        self.link(text, href);
        self.url = Some(href.clone());
    }

    /// Panics if called with no style in the stack
    pub fn close_last(&mut self) {
        let style = self.style_stack.pop().expect("cannot be empty");
        self.html.push_str(style.close());
    }

    pub fn close_styles(&mut self) {
        while !self.style_stack.is_empty() {
            self.close_last();
        }
    }
}

impl std::fmt::Write for MessageBuilder {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.plain.push_str(s);

        let mut last = 0;
        for (i, c) in s.char_indices() {
            // NOTE: escape characters that have a special meaning in HTML. Shamelessly adapted from
            // rustdoc/html/escape.rs
            let escaped = match c {
                '>' => "&gt;",
                '<' => "&lt;",
                '&' => "&amp;",
                '\'' => "&#39;",
                '"' => "&quot;",
                _ => continue,
            };

            self.html.push_str(&s[last..i]);
            self.html.push_str(escaped);
            last = i + 1;
        }

        if last < s.len() {
            self.html.push_str(&s[last..]);
        }

        Ok(())
    }
}

impl std::convert::From<MessageBuilder> for MessageEventContent {
    fn from(msg: MessageBuilder) -> Self {
        msg.build()
    }
}

#[cfg(test)]
mod tests {
    use matrix_sdk::ruma::events::room::message::{
        FormattedBody, MessageFormat, TextMessageEventContent,
    };

    use super::*;

    #[test]
    fn html_escape() {
        let mut msgbld = MessageBuilder::new();

        msgbld.color("#ff0000");
        msgbld.bold();
        write!(&mut msgbld, "These should be escaped: < > & \" '").unwrap();
        msgbld.close_styles();

        assert_eq!(msgbld.html, "<span style=\"color: #ff0000\"><b>These should be escaped: &lt; &gt; &amp; &quot; &#39;</b></span>");
        assert_eq!(msgbld.plain, "These should be escaped: < > & \" '");
    }

    #[test]
    fn test_append_main_url() {
        let mut msgbld = MessageBuilder::new();

        msgbld.main_link("test", &Url::parse("https://prologin.org").unwrap());

        match msgbld.build().msgtype {
            matrix_sdk::ruma::events::room::message::MessageType::Text(
                TextMessageEventContent {
                    body: plain,
                    formatted:
                        Some(FormattedBody {
                            format: MessageFormat::Html,
                            body: html,
                        }),
                    ..
                },
            ) => {
                assert_eq!(plain, "test ⋅ https://prologin.org/");
                assert_eq!(html, r#"<a href="https://prologin.org/">test</a>"#);
            }
            _ => panic!("shouldn't happen"),
        }
    }
}
