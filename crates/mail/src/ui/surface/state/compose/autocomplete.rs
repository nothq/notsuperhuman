use super::{
    mail_address_summary, mail_parse_addresses, mail_replace_active_compose_address,
    mail_split_compose_addresses, offset_index, Context, HashSet, MailAddress,
    MailComposeAutocompleteItem, MailComposeField, MailMessage, SurfaceState,
};

struct MailAutocompleteMatchContext<'a> {
    query_lower: &'a str,
    current_recipients: &'a HashSet<String>,
    self_email: Option<&'a str>,
    seen: &'a mut HashSet<String>,
}

impl SurfaceState {
    pub(super) fn mail_compose_recipient_field_mut(&mut self) -> Option<&mut String> {
        match self.mail_compose_focused_field {
            MailComposeField::To => Some(&mut self.mail_compose_to),
            MailComposeField::Cc => Some(&mut self.mail_compose_cc),
            MailComposeField::Subject | MailComposeField::Body => None,
        }
    }

    fn mail_known_addresses(&self) -> Vec<MailAddress> {
        let mut addresses = Vec::new();
        if let Some(identity) = self.mail_identity.as_ref() {
            addresses.extend(identity.reply_to.clone());
        }
        if let Some(thread) = self.mail_active_thread() {
            for message in thread.messages.iter().rev() {
                Self::extend_mail_message_addresses(&mut addresses, message);
            }
        }
        if let Some(workspace) = self.mail_workspace() {
            for message in &workspace.messages {
                Self::extend_mail_message_addresses(&mut addresses, message);
            }
        }
        for thread in self.mail_thread_cache.values() {
            for message in thread.messages.iter().rev() {
                Self::extend_mail_message_addresses(&mut addresses, message);
            }
        }
        addresses
    }

    fn extend_mail_message_addresses(target: &mut Vec<MailAddress>, message: &MailMessage) {
        target.extend(message.sender.clone());
        target.extend(message.reply_to.clone());
        target.extend(message.from.clone());
        target.extend(message.to.clone());
        target.extend(message.cc.clone());
        target.extend(message.bcc.clone());
    }

    pub(crate) fn mail_compose_autocomplete_items(&self) -> Vec<MailComposeAutocompleteItem> {
        let Some(field_value) = self.mail_autocomplete_field_value() else {
            return Vec::new();
        };
        let Some(query_lower) = self.mail_autocomplete_query(field_value) else {
            return Vec::new();
        };
        let current_recipients = mail_parse_addresses(field_value)
            .into_iter()
            .map(|address| address.email.trim().to_ascii_lowercase())
            .filter(|email| !email.is_empty())
            .collect::<HashSet<_>>();
        let self_email = self
            .mail_compose_from
            .as_ref()
            .map(|address| address.email.trim().to_ascii_lowercase());
        self.mail_ranked_autocomplete_items(query_lower.as_str(), &current_recipients, self_email)
    }

    fn mail_autocomplete_field_value(&self) -> Option<&str> {
        match self.mail_compose_focused_field {
            MailComposeField::To => Some(&self.mail_compose_to),
            MailComposeField::Cc => Some(&self.mail_compose_cc),
            MailComposeField::Subject | MailComposeField::Body => None,
        }
    }

    fn mail_autocomplete_query(&self, field_value: &str) -> Option<String> {
        let (_, query) = mail_split_compose_addresses(field_value);
        let query = query.trim();
        (!query.is_empty()).then(|| query.to_ascii_lowercase())
    }

    fn mail_ranked_autocomplete_items(
        &self,
        query_lower: &str,
        current_recipients: &HashSet<String>,
        self_email: Option<String>,
    ) -> Vec<MailComposeAutocompleteItem> {
        let mut seen = HashSet::new();
        let mut prefix_matches = Vec::new();
        let mut contains_matches = Vec::new();
        for address in self.mail_known_addresses() {
            let mut context = MailAutocompleteMatchContext {
                query_lower,
                current_recipients,
                self_email: self_email.as_deref(),
                seen: &mut seen,
            };
            let Some((starts, item)) = self.mail_autocomplete_item(&address, &mut context) else {
                continue;
            };
            if starts {
                prefix_matches.push(item);
            } else {
                contains_matches.push(item);
            }
        }
        prefix_matches.extend(contains_matches);
        prefix_matches.truncate(6);
        prefix_matches
    }

    fn mail_autocomplete_item(
        &self,
        address: &MailAddress,
        context: &mut MailAutocompleteMatchContext<'_>,
    ) -> Option<(bool, MailComposeAutocompleteItem)> {
        let email = address.email.trim();
        if email.is_empty() {
            return None;
        }
        let email_lower = email.to_ascii_lowercase();
        if context.current_recipients.contains(&email_lower)
            || context.self_email == Some(email_lower.as_str())
            || !context.seen.insert(email_lower.clone())
        {
            return None;
        }
        let label = mail_address_summary(address);
        let label_lower = label.to_ascii_lowercase();
        let email_starts = email_lower.starts_with(context.query_lower);
        let label_starts = label_lower.starts_with(context.query_lower);
        let email_contains = email_lower.contains(context.query_lower);
        let label_contains = label_lower.contains(context.query_lower);
        if !email_contains && !label_contains {
            return None;
        }
        Some((
            email_starts || label_starts,
            MailComposeAutocompleteItem {
                label,
                detail: address.email.clone(),
                address: address.clone(),
            },
        ))
    }

    pub(crate) fn reset_mail_compose_autocomplete(&mut self) {
        self.mail_compose_autocomplete_selected_index = 0;
    }

    pub(crate) fn move_mail_compose_autocomplete(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let suggestions = self.mail_compose_autocomplete_items();
        if suggestions.is_empty() {
            return false;
        }
        self.mail_compose_autocomplete_selected_index = offset_index(
            self.mail_compose_autocomplete_selected_index,
            delta,
            suggestions.len(),
        );
        cx.notify();
        true
    }

    pub(crate) fn accept_mail_compose_autocomplete(&mut self, cx: &mut Context<Self>) -> bool {
        let suggestions = self.mail_compose_autocomplete_items();
        let Some(item) = suggestions.get(
            self.mail_compose_autocomplete_selected_index
                .min(suggestions.len().saturating_sub(1)),
        ) else {
            return false;
        };
        self.select_mail_compose_autocomplete_item(item.address.clone(), cx);
        true
    }

    pub(crate) fn select_mail_compose_autocomplete_item(
        &mut self,
        address: MailAddress,
        cx: &mut Context<Self>,
    ) {
        let Some(field_value) = self.mail_compose_recipient_field_mut() else {
            return;
        };
        mail_replace_active_compose_address(field_value, &address);
        self.mail_compose_error = None;
        self.reset_mail_compose_autocomplete();
        cx.notify();
        self.save_mail_draft_in_background(cx);
    }
}
