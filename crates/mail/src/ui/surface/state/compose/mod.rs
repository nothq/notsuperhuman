use super::{
    mail_address_summary, mail_format_addresses, mail_forward_attachments, mail_forward_subject,
    mail_mailbox_id_for_role, mail_parse_addresses, mail_replace_active_compose_address,
    mail_reply_recipients, mail_reply_subject, mail_split_compose_addresses, mail_thread_quote,
    mail_thread_references, Context, Duration, HashSet, MailComposeAutocompleteItem,
    MailComposeDraftSave, MailComposeDraftSaveSnapshot, MailComposeField, MailComposeMode,
    MailComposeSendIntent, MailDraftRequest, MailMessage, MailSendRecipients, MailSendResult,
    MailThread, MailUploadFile, SurfaceState, Window,
};
use crate::model::{MailAddress, MailIdentity};
use crate::ui::offset_index;
use crate::ui::surface::mail_latest_thread_message;

mod attachments;
mod autocomplete;
mod commands;
mod drafts;
mod entities;
mod reply;

use self::drafts::MailComposeDraftTarget;

impl SurfaceState {
    pub(crate) fn focus_mail_compose_field(
        &mut self,
        field: MailComposeField,
        cx: &mut Context<Self>,
    ) {
        let shortcuts_were_focused = self.mail_shortcuts_focused;
        self.mail_shortcuts_focused = false;
        if self.mail_compose_focused_field == field {
            if shortcuts_were_focused {
                cx.notify();
            }
            return;
        }
        self.mail_compose_focused_field = field;
        self.reset_mail_compose_autocomplete();
        cx.notify();
    }

    pub(super) fn cycle_mail_compose_field(
        &mut self,
        backward: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        self.mail_shortcuts_focused = false;
        self.mail_compose_focused_field = match (self.mail_compose_focused_field, backward) {
            (MailComposeField::To, false) => MailComposeField::Cc,
            (MailComposeField::Cc, false) => MailComposeField::Subject,
            (MailComposeField::Subject, false) => MailComposeField::Body,
            (MailComposeField::Body, false) => MailComposeField::To,
            (MailComposeField::To, true) => MailComposeField::Body,
            (MailComposeField::Cc, true) => MailComposeField::To,
            (MailComposeField::Subject, true) => MailComposeField::Cc,
            (MailComposeField::Body, true) => MailComposeField::Subject,
        };
        self.reset_mail_compose_autocomplete();
        cx.notify();
        true
    }

    fn mail_compose_field_value_mut(&mut self, field: MailComposeField) -> &mut String {
        match field {
            MailComposeField::To => &mut self.mail_compose_to,
            MailComposeField::Cc => &mut self.mail_compose_cc,
            MailComposeField::Subject => &mut self.mail_compose_subject,
            MailComposeField::Body => &mut self.mail_compose_body,
        }
    }

    pub(crate) fn mail_compose_field_value(&self, field: MailComposeField) -> &str {
        match field {
            MailComposeField::To => &self.mail_compose_to,
            MailComposeField::Cc => &self.mail_compose_cc,
            MailComposeField::Subject => &self.mail_compose_subject,
            MailComposeField::Body => &self.mail_compose_body,
        }
    }

    pub(super) fn set_mail_compose_field_value(
        &mut self,
        field: MailComposeField,
        value: String,
        cx: &mut Context<Self>,
    ) {
        if self.mail_compose_field_value(field) == value.as_str() {
            return;
        }
        *self.mail_compose_field_value_mut(field) = value;
        self.reset_mail_compose_autocomplete();
        self.mail_compose_error = None;
        if field == MailComposeField::Body {
            self.remeasure_mail_open_thread_body_list_state();
        }
        cx.notify();
        self.schedule_mail_draft_autosave(cx);
    }
}
