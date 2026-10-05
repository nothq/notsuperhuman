use super::{
    Context, MailAddress, MailComposeDraftTarget, MailComposeField, MailComposeMode,
    MailDraftRequest, MailIdentity, SurfaceState,
};

impl SurfaceState {
    pub(crate) fn open_new_mail_composer(&mut self, cx: &mut Context<Self>) {
        if !self.mail_active_account_can_submit() {
            return;
        }
        self.reset_mail_open_thread_read_session();
        self.mail_open_thread_id = None;
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_action_palette = None;
        self.mail_compose_mode = MailComposeMode::New;
        self.mail_compose_bound_thread_id = None;
        self.mail_compose_focused_field = MailComposeField::To;
        self.mail_shortcuts_focused = false;
        self.mail_compose_autocomplete_selected_index = 0;
        self.mail_compose_error = None;

        if self.mail_compose_draft_id.is_some() {
            cx.notify();
            return;
        }

        self.ensure_mail_identity_loaded(cx);
        let Some(identity) = self.mail_identity_or_error(cx) else {
            return;
        };
        let from = self.mail_identity_from_address(&identity);
        let request = MailDraftRequest {
            identity_id: identity.id,
            from,
            reply_to: identity.reply_to,
            to: Vec::new(),
            cc: Vec::new(),
            subject: String::new(),
            body_text: String::new(),
            attachments: Vec::new(),
            in_reply_to: Vec::new(),
            references: Vec::new(),
        };
        self.open_mail_compose_with_new_draft(MailComposeMode::New, None, request, cx);
    }

    pub(crate) fn edit_mail_draft(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.mail_active_account_can_submit() {
            return true;
        }
        let Some(identity) = self.mail_identity_or_error(cx) else {
            return true;
        };
        let Some(message) = self
            .mail_active_message()
            .filter(|message| message.is_draft)
        else {
            return false;
        };
        let from = self.mail_identity_from_address(&identity);
        let request = MailDraftRequest {
            identity_id: identity.id,
            from,
            reply_to: identity.reply_to,
            to: message.to.clone(),
            cc: message.cc.clone(),
            subject: message.subject.clone(),
            body_text: message.body_text.clone(),
            attachments: message.attachments.clone(),
            in_reply_to: message.in_reply_to.clone(),
            references: message.references.clone(),
        };
        self.apply_mail_compose_draft(
            MailComposeDraftTarget {
                mode: MailComposeMode::New,
                bound_thread_id: Some(message.thread_id.clone()),
            },
            request,
            message,
            cx,
        );
        true
    }

    pub(super) fn mail_identity_or_error(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<MailIdentity> {
        let identity = self.mail_identity.clone();
        if identity.is_some() {
            return identity;
        }
        self.mail_compose_error = Some(if self.mail_identity_loading {
            "mail identity is still loading".to_string()
        } else {
            "mail identity is not available yet".to_string()
        });
        cx.notify();
        None
    }

    pub(super) fn mail_identity_from_address(&self, identity: &MailIdentity) -> MailAddress {
        MailAddress {
            name: identity.name.clone(),
            email: identity.email.clone(),
        }
    }
}
