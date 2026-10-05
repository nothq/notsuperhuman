mod style;
mod text_style;

pub use style::*;
pub use text_style::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MailDocument {
    pub blocks: Vec<MailBlock>,
    pub image_urls: Vec<String>,
    pub is_rich_layout: bool,
    pub clipped: bool,
    /// The `<body>` margin, which insets the whole document.
    ///
    /// Every browser's user agent stylesheet sets it to 8px. Most emails reset
    /// it and the ones that do not are laid out inside that inset, so it is
    /// carried here and applied once by the renderer rather than becoming a box
    /// in the block tree.
    pub body_margin: MailEdgeInsets,
    /// The `<body>` background colour, which CSS propagates to the canvas so it
    /// fills the viewport — margin area included — rather than only the body box.
    pub body_background: Option<u32>,
}

impl MailDocument {
    pub fn from_blocks(blocks: Vec<MailBlock>) -> Self {
        let mut document = Self {
            blocks,
            image_urls: Vec::new(),
            is_rich_layout: false,
            clipped: false,
            body_margin: MailEdgeInsets::default(),
            body_background: None,
        };
        document.refresh_metadata();
        document
    }

    pub fn from_text_paragraphs(paragraphs: Vec<Vec<MailTextRun>>) -> Self {
        Self::from_blocks(
            paragraphs
                .into_iter()
                .filter(|runs| runs.iter().any(|run| !run.text.trim().is_empty()))
                .map(|runs| {
                    MailBlock::Paragraph(MailParagraph {
                        runs,
                        style: MailStyle::default(),
                    })
                })
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn refresh_metadata(&mut self) {
        self.image_urls = mail_block_image_urls(&self.blocks);
        self.is_rich_layout = self.blocks.iter().any(MailBlock::is_rich_layout);
    }

    pub fn text_paragraphs(&self) -> Vec<Vec<MailTextRun>> {
        let mut paragraphs = Vec::new();
        collect_text_paragraphs(&self.blocks, &mut paragraphs);
        paragraphs
    }

    pub fn plain_text(&self) -> String {
        self.text_paragraphs()
            .into_iter()
            .map(|paragraph| {
                paragraph
                    .into_iter()
                    .map(|run| run.text)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MailBlock {
    Paragraph(MailParagraph),
    Container(MailContainer),
    Table(MailTable),
    Image(MailImage),
    Rule(MailStyle),
    Spacer(f32),
}

impl MailBlock {
    pub fn plain_text(&self) -> String {
        match self {
            MailBlock::Paragraph(paragraph) => paragraph
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            MailBlock::Container(container) => container
                .children
                .iter()
                .map(MailBlock::plain_text)
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            MailBlock::Table(table) => table
                .rows
                .iter()
                .flat_map(|row| row.cells.iter())
                .map(|cell| {
                    cell.children
                        .iter()
                        .map(MailBlock::plain_text)
                        .filter(|text| !text.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            MailBlock::Image(image) => image.alt.clone(),
            MailBlock::Rule(_) | MailBlock::Spacer(_) => String::new(),
        }
    }

    pub(crate) fn is_rich_layout(&self) -> bool {
        match self {
            MailBlock::Table(_) | MailBlock::Image(_) | MailBlock::Rule(_) => true,
            MailBlock::Container(container) => {
                container.style.has_box_style()
                    || container.children.iter().any(MailBlock::is_rich_layout)
            }
            MailBlock::Paragraph(paragraph) => paragraph.style.has_box_style(),
            MailBlock::Spacer(_) => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailContainer {
    pub children: Vec<MailBlock>,
    pub style: MailStyle,
    pub link_target: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailParagraph {
    pub runs: Vec<MailTextRun>,
    pub style: MailStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailTable {
    pub rows: Vec<MailTableRow>,
    pub style: MailStyle,
    pub cell_spacing: f32,
    pub row_spacing: f32,
    pub cell_padding: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailTableRow {
    pub cells: Vec<MailTableCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailTableCell {
    pub children: Vec<MailBlock>,
    pub style: MailStyle,
    pub colspan: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailImage {
    pub src: String,
    pub alt: String,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub style: MailStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailTextRun {
    pub text: String,
    pub href: Option<String>,
    pub style: MailInlineStyle,
}

impl MailTextRun {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            href: None,
            style: MailInlineStyle::default(),
        }
    }

    pub fn link(text: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            href: Some(href.into()),
            style: MailInlineStyle::default(),
        }
    }
}

fn collect_text_paragraphs(blocks: &[MailBlock], paragraphs: &mut Vec<Vec<MailTextRun>>) {
    for block in blocks {
        match block {
            MailBlock::Paragraph(paragraph) => paragraphs.push(paragraph.runs.clone()),
            MailBlock::Container(container) => {
                collect_text_paragraphs(&container.children, paragraphs);
            }
            MailBlock::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_text_paragraphs(&cell.children, paragraphs);
                    }
                }
            }
            MailBlock::Image(_) | MailBlock::Rule(_) | MailBlock::Spacer(_) => {}
        }
    }
}

fn mail_block_image_urls(blocks: &[MailBlock]) -> Vec<String> {
    let mut urls = Vec::new();
    collect_mail_block_image_urls(blocks, &mut urls);
    urls
}

fn collect_mail_block_image_urls(blocks: &[MailBlock], urls: &mut Vec<String>) {
    for block in blocks {
        match block {
            MailBlock::Image(image) => push_unique_image_url(&image.src, urls),
            MailBlock::Container(container) => {
                collect_mail_style_image_urls(&container.style, urls);
                collect_mail_block_image_urls(&container.children, urls);
            }
            MailBlock::Table(table) => {
                collect_mail_style_image_urls(&table.style, urls);
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_mail_style_image_urls(&cell.style, urls);
                        collect_mail_block_image_urls(&cell.children, urls);
                    }
                }
            }
            MailBlock::Paragraph(paragraph) => {
                collect_mail_style_image_urls(&paragraph.style, urls)
            }
            MailBlock::Rule(style) => collect_mail_style_image_urls(style, urls),
            MailBlock::Spacer(_) => {}
        }
    }
}

fn collect_mail_style_image_urls(style: &MailStyle, urls: &mut Vec<String>) {
    if let Some(url) = &style.background_image_url {
        push_unique_image_url(url, urls);
    }
}

fn push_unique_image_url(url: &str, urls: &mut Vec<String>) {
    if urls.iter().all(|existing| existing != url) {
        urls.push(url.to_string());
    }
}
