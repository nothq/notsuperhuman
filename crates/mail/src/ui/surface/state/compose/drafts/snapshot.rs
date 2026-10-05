use super::super::{
    mail_parse_addresses, MailComposeDraftSave, MailComposeDraftSaveSnapshot, MailDraftRequest,
    SurfaceState,
};

impl SurfaceState {
    fn mail_compose_request(&self) -> Result<MailDraftRequest, String> {
        let identity_id = self
            .mail_compose_identity_id
            .clone()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "missing mail identity".to_string())?;
        let from = self
            .mail_compose_from
            .clone()
            .filter(|address| !address.email.trim().is_empty())
            .ok_or_else(|| "missing From address".to_string())?;
        Ok(MailDraftRequest {
            identity_id,
            from,
            reply_to: self.mail_compose_reply_to.clone(),
            to: mail_parse_addresses(&self.mail_compose_to),
            cc: mail_parse_addresses(&self.mail_compose_cc),
            subject: self.mail_compose_subject.clone(),
            body_text: self.mail_compose_body.clone(),
            attachments: self.mail_compose_attachments.clone(),
            in_reply_to: self.mail_compose_in_reply_to.clone(),
            references: self.mail_compose_references.clone(),
        })
    }

    pub(super) fn mail_compose_autosave_snapshot(&self) -> Option<MailComposeDraftSaveSnapshot> {
        let draft_id = self.mail_compose_draft_id.clone()?;
        let request = self.mail_compose_request().ok()?;
        Some(MailComposeDraftSaveSnapshot { draft_id, request })
    }

    pub(super) fn next_mail_compose_autosave(
        &mut self,
        snapshot: MailComposeDraftSaveSnapshot,
    ) -> MailComposeDraftSave {
        self.mail_compose_autosave_generation =
            self.mail_compose_autosave_generation.wrapping_add(1);
        MailComposeDraftSave {
            generation: self.mail_compose_autosave_generation,
            draft_id: snapshot.draft_id,
            request: snapshot.request,
        }
    }

    pub(super) fn clear_mail_compose_autosave_queue(&mut self) {
        self.mail_compose_autosave_generation =
            self.mail_compose_autosave_generation.wrapping_add(1);
        self.mail_compose_autosave_scheduled_generation = None;
        self.mail_compose_autosave_in_flight = None;
        self.mail_compose_autosave_queued = None;
        self.mail_compose_autosave_last_saved = None;
        self.mail_compose_send_after_autosave = None;
    }
}
