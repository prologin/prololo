use matrix_sdk::ruma::events::room::message::MessageEventContent;

struct MessageBuilder {
    html: String,
    plain: String,
}

impl MessageBuilder {
    fn build(self) -> MessageEventContent {
        MessageEventContent::text_html(self.plain, self.html)
    }
}
