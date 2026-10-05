use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSearchSnippet {
    subject: Option<MailSearchSnippetText>,
    preview: Option<MailSearchSnippetText>,
}

impl MailSearchSnippet {
    pub fn new(
        subject: Option<MailSearchSnippetText>,
        preview: Option<MailSearchSnippetText>,
    ) -> Self {
        Self { subject, preview }
    }

    pub fn subject(&self) -> Option<&MailSearchSnippetText> {
        self.subject.as_ref()
    }

    pub fn preview(&self) -> Option<&MailSearchSnippetText> {
        self.preview.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSearchSnippetText {
    segments: Vec<MailSearchSnippetSegment>,
}

impl MailSearchSnippetText {
    pub fn new(segments: Vec<MailSearchSnippetSegment>) -> Self {
        Self { segments }
    }

    pub fn segments(&self) -> &[MailSearchSnippetSegment] {
        self.segments.as_slice()
    }

    pub fn plain_text(&self) -> String {
        self.segments
            .iter()
            .map(MailSearchSnippetSegment::text)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSearchSnippetSegment {
    text: String,
    highlighted: bool,
}

impl MailSearchSnippetSegment {
    pub fn new(text: String, highlighted: bool) -> Self {
        Self { text, highlighted }
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub fn is_highlighted(&self) -> bool {
        self.highlighted
    }
}
