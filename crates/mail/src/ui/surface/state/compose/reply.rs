use super::{
    mail_forward_attachments, mail_forward_subject, mail_latest_thread_message,
    mail_reply_recipients, mail_reply_subject, mail_thread_quote, mail_thread_references, Context,
    MailComposeMode, MailDraftRequest, MailIdentity, MailThread, SurfaceState,
};

impl SurfaceState {
    pub(crate) fn reply_mail_thread(&mut self, cx: &mut Context<Self>) -> bool {
        self.open_mail_reply_like(MailComposeMode::Reply, cx)
    }

    pub(crate) fn reply_all_mail_thread(&mut self, cx: &mut Context<Self>) -> bool {
        self.open_mail_reply_like(MailComposeMode::ReplyAll, cx)
    }

    pub(crate) fn forward_mail_thread(&mut self, cx: &mut Context<Self>) -> bool {
        self.open_mail_reply_like(MailComposeMode::Forward, cx)
    }

    pub(crate) fn open_mail_reply_like(
        &mut self,
        mode: MailComposeMode,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_active_account_can_submit() {
            return true;
        }
        let Some(identity) = self.mail_identity_or_error(cx) else {
            return true;
        };
        let Some(thread) = self.mail_active_thread() else {
            return false;
        };
        let Some(request) = self.mail_reply_like_request(mode, &identity, &thread) else {
            return false;
        };
        self.open_mail_compose_with_new_draft(mode, Some(thread.id.clone()), request, cx);
        true
    }

    fn mail_reply_like_request(
        &self,
        mode: MailComposeMode,
        identity: &MailIdentity,
        thread: &MailThread,
    ) -> Option<MailDraftRequest> {
        match mode {
            MailComposeMode::Reply | MailComposeMode::ReplyAll => {
                let (to, cc) = mail_reply_recipients(
                    thread,
                    identity.email.as_str(),
                    mode == MailComposeMode::ReplyAll,
                );
                Some(MailDraftRequest {
                    identity_id: identity.id.clone(),
                    from: self.mail_identity_from_address(identity),
                    reply_to: identity.reply_to.clone(),
                    to,
                    cc,
                    subject: mail_reply_subject(
                        mail_latest_thread_message(thread)
                            .map(|message| message.subject.as_str())
                            .unwrap_or_default(),
                    ),
                    body_text: String::new(),
                    attachments: Vec::new(),
                    in_reply_to: mail_latest_thread_message(thread)
                        .map(|message| message.message_id.clone())
                        .unwrap_or_default(),
                    references: mail_thread_references(thread),
                })
            }
            MailComposeMode::Forward => Some(MailDraftRequest {
                identity_id: identity.id.clone(),
                from: self.mail_identity_from_address(identity),
                reply_to: identity.reply_to.clone(),
                to: Vec::new(),
                cc: Vec::new(),
                subject: mail_forward_subject(
                    mail_latest_thread_message(thread)
                        .map(|message| message.subject.as_str())
                        .unwrap_or_default(),
                ),
                body_text: mail_thread_quote(thread),
                attachments: mail_forward_attachments(thread),
                in_reply_to: Vec::new(),
                references: mail_thread_references(thread),
            }),
            MailComposeMode::Closed | MailComposeMode::New => None,
        }
    }
}
