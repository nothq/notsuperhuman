use crate::ui::*;
use base64::prelude::BASE64_STANDARD;
use time::{
    format_description::well_known::Rfc3339, macros::format_description, OffsetDateTime, UtcOffset,
};

pub(crate) use remote_image_model::RemoteImageData;

pub(crate) fn parse_data_image_url(url: &str) -> Option<RemoteImageData> {
    let payload = url.strip_prefix("data:")?;
    let (header, base64) = payload.split_once(',')?;
    let mimetype = header.strip_suffix(";base64")?;
    Some(RemoteImageData {
        bytes: BASE64_STANDARD.decode(base64.trim()).ok()?,
        mimetype: mimetype.to_string(),
    })
}

pub(crate) fn mail_image_from_data(data: RemoteImageData) -> Option<Image> {
    let format = ImageFormat::from_mime_type(&data.mimetype)?;
    Some(Image::from_bytes(format, data.bytes))
}

pub(crate) fn render_mail_remote_image<T: 'static>(
    image: Arc<Image>,
    cx: &mut Context<T>,
) -> Option<Arc<RenderImage>> {
    if image.format() == ImageFormat::Svg {
        return cx
            .svg_renderer()
            .render_single_frame(image.bytes(), 1.0)
            .ok();
    }
    image
        .to_image_data(cx.svg_renderer())
        .ok()
        .and_then(padded_image)
}

/// The ring of pixels round every frame of a decoded image, each a copy of
/// the nearest edge pixel.
///
/// GPUI packs sprites edge to edge in its atlas and samples them with a linear
/// filter, so when the window scale magnifies an image its outermost pixels
/// blend with whatever sprite sits beside it: a gray hairline round a white
/// header image. With the ring, the neighbour the sampler sees is the edge
/// itself. The renderer draws each frame inset by the ring, so nothing moves.
pub(crate) const IMAGE_PAD: u32 = 1;

pub(crate) fn padded_image(image: Arc<RenderImage>) -> Option<Arc<RenderImage>> {
    let frames = (0..image.frame_count())
        .map(|index| {
            let size = image.size(index);
            let (width, height) = (u32::from(size.width), u32::from(size.height));
            let bytes = image.as_bytes(index)?;
            let padded = (width + 2 * IMAGE_PAD, height + 2 * IMAGE_PAD);
            let mut buffer = image::RgbaImage::new(padded.0, padded.1);
            for (x, y, pixel) in buffer.enumerate_pixels_mut() {
                let source_x = x.saturating_sub(IMAGE_PAD).min(width - 1);
                let source_y = y.saturating_sub(IMAGE_PAD).min(height - 1);
                let offset = ((source_y * width + source_x) * 4) as usize;
                pixel.0.copy_from_slice(&bytes[offset..offset + 4]);
            }
            Some(image::Frame::from_parts(buffer, 0, 0, image.delay(index)))
        })
        .collect::<Option<Vec<image::Frame>>>()?;
    Some(Arc::new(RenderImage::new(frames)))
}

pub(crate) fn offset_index(current: usize, delta: isize, item_count: usize) -> usize {
    if item_count == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        (current + delta as usize).min(item_count - 1)
    }
}

pub(crate) fn mailbox_display_label(mailbox: &Mailbox) -> String {
    match mailbox.role.as_deref() {
        Some("inbox") => "Inbox".to_string(),
        Some("drafts") => "Drafts".to_string(),
        Some("sent") => "Sent".to_string(),
        Some("archive") => "Archive".to_string(),
        Some("trash") => "Trash".to_string(),
        Some("junk") => "Junk".to_string(),
        _ if mailbox.name.trim().is_empty() => "Mailbox".to_string(),
        _ => mailbox.name.clone(),
    }
}

pub(crate) fn mail_address_display(address: Option<&MailAddress>) -> String {
    let Some(address) = address else {
        return "Unknown sender".to_string();
    };
    if address.name.trim().is_empty() {
        address.email.clone()
    } else {
        address.name.clone()
    }
}

pub(crate) fn format_mail_address_list(addresses: &[MailAddress]) -> String {
    addresses
        .iter()
        .map(|address| {
            if address.name.trim().is_empty() {
                address.email.clone()
            } else {
                format!("{} <{}>", address.name, address.email)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn avatar_initials(label: &str) -> String {
    let mut initials = label
        .split(|character: char| character.is_whitespace() || character == '@' || character == '.')
        .filter(|segment| !segment.is_empty())
        .take(2)
        .filter_map(|segment| segment.chars().next())
        .collect::<String>();
    if initials.is_empty() {
        initials.push('?');
    }
    initials.to_uppercase()
}

pub(crate) fn format_mail_received_at(value: &str) -> String {
    let Some(timestamp) = mail_local_timestamp(value) else {
        return value.to_string();
    };
    let now = mail_local_now();
    if now.date() == timestamp.date() {
        let format = format_description!("[hour repr:12]:[minute] [period case:upper]");
        return timestamp
            .format(&format)
            .unwrap_or_else(|_| value.to_string());
    }
    let format = format_description!("[month repr:short] [day]");
    timestamp
        .format(&format)
        .unwrap_or_else(|_| value.to_string())
}

pub(crate) fn mail_local_timestamp(value: &str) -> Option<OffsetDateTime> {
    let timestamp = OffsetDateTime::parse(value, &Rfc3339).ok()?;
    Some(timestamp.to_offset(mail_local_offset_at(timestamp)))
}

pub(crate) fn mail_local_now() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    now.to_offset(mail_local_offset_at(now))
}

fn mail_local_offset_at(timestamp: OffsetDateTime) -> UtcOffset {
    UtcOffset::local_offset_at(timestamp)
        .expect("notsuperhuman Mail requires the macOS local timezone offset")
}

pub(crate) fn format_attachment_size(size: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    if size >= MIB {
        format!("{:.1} MB", size as f64 / MIB as f64)
    } else if size >= KIB {
        format!("{:.0} KB", size as f64 / KIB as f64)
    } else {
        format!("{size} B")
    }
}
