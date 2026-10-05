use super::{
    mail_correspondent_address, Context, MailContactAddress, MailContactHistoryPage,
    MailContactHistoryRequest, MailContactHistoryRow, MailContactHistoryState,
    MailContactHistoryStatus, SurfaceState,
};
use crate::ui::format_mail_received_at;

#[cfg(any(test, feature = "test-support"))]
use super::super::{mail_contact_history_rows, MailStartup};

/// The contact history load a completed result must still match to apply.
struct MailContactHistoryLoad {
    account_generation: u64,
    request_generation: u64,
    thread_id: String,
    contact: MailContactAddress,
}

impl SurfaceState {
    pub(crate) fn ensure_mail_contact_history(&mut self, cx: &mut Context<Self>) {
        let Some(thread_id) = self.mail_open_thread_id.clone() else {
            if self.deselect_mail_contact_history() {
                cx.notify();
            }
            return;
        };
        let contact = self.mail_selected_correspondent_address();
        self.select_mail_contact_address(contact.clone(), cx);
        let Some(contact) = contact else {
            return;
        };
        match self.mail_contact_histories.get(&contact) {
            Some(MailContactHistoryState {
                status: MailContactHistoryStatus::Ready(_) | MailContactHistoryStatus::Error(_),
                ..
            }) => return,
            Some(MailContactHistoryState {
                thread_id: requested_thread_id,
                status: MailContactHistoryStatus::Loading,
                ..
            }) if requested_thread_id == &thread_id => return,
            Some(_) | None => {}
        }
        self.start_mail_contact_history_load(contact, thread_id, cx);
    }

    pub(crate) fn retry_mail_contact_history(&mut self, cx: &mut Context<Self>) {
        let (Some(contact), Some(thread_id)) = (
            self.mail_selected_contact_address.clone(),
            self.mail_open_thread_id.clone(),
        ) else {
            return;
        };
        self.start_mail_contact_history_load(contact, thread_id, cx);
    }

    pub(crate) fn clear_mail_contact_histories(&mut self) {
        self.mail_selected_contact_address = None;
        self.mail_contact_history_request_generation =
            self.mail_contact_history_request_generation.wrapping_add(1);
        self.mail_contact_histories.clear();
    }

    pub(crate) fn deselect_mail_contact_history(&mut self) -> bool {
        let had_selection = self.mail_selected_contact_address.take().is_some();
        let loading_count = self
            .mail_contact_histories
            .values()
            .filter(|state| matches!(&state.status, MailContactHistoryStatus::Loading))
            .count();
        if !had_selection && loading_count == 0 {
            return false;
        }
        self.mail_contact_history_request_generation =
            self.mail_contact_history_request_generation.wrapping_add(1);
        self.mail_contact_histories
            .retain(|_, state| !matches!(&state.status, MailContactHistoryStatus::Loading));
        true
    }

    fn mail_selected_correspondent_address(&self) -> Option<MailContactAddress> {
        let workspace = self.mail_workspace()?;
        let message = self.mail_active_message()?;
        mail_correspondent_address(workspace, &message).map(|(_, contact)| contact)
    }

    fn select_mail_contact_address(
        &mut self,
        contact: Option<MailContactAddress>,
        cx: &mut Context<Self>,
    ) {
        if self.mail_selected_contact_address == contact {
            return;
        }
        self.mail_contact_history_request_generation =
            self.mail_contact_history_request_generation.wrapping_add(1);
        self.mail_contact_histories
            .retain(|_, state| !matches!(&state.status, MailContactHistoryStatus::Loading));
        self.mail_selected_contact_address = contact;
        cx.notify();
    }

    fn start_mail_contact_history_load(
        &mut self,
        contact: MailContactAddress,
        thread_id: String,
        cx: &mut Context<Self>,
    ) {
        self.mail_contact_history_request_generation =
            self.mail_contact_history_request_generation.wrapping_add(1);
        let request_generation = self.mail_contact_history_request_generation;

        #[cfg(any(test, feature = "test-support"))]
        if self.start_fixture_mail_contact_history(
            contact.clone(),
            thread_id.clone(),
            request_generation,
            cx,
        ) {
            return;
        }

        let Some(workspace_api) = self.mail_workspace_api() else {
            self.mail_contact_histories.insert(
                contact,
                MailContactHistoryState {
                    request_generation,
                    thread_id,
                    status: MailContactHistoryStatus::Error(
                        "Mail workspace is unavailable".to_string(),
                    ),
                },
            );
            cx.notify();
            return;
        };
        let account_generation = self.mail_account_generation;
        self.mail_contact_histories.insert(
            contact.clone(),
            MailContactHistoryState {
                request_generation,
                thread_id: thread_id.clone(),
                status: MailContactHistoryStatus::Loading,
            },
        );
        cx.notify();
        let load = MailContactHistoryLoad {
            account_generation,
            request_generation,
            thread_id,
            contact: contact.clone(),
        };
        self.spawn_background_task(
            MailContactHistoryRequest::new(contact),
            cx,
            move |request| workspace_api.load_mail_contact_history(request),
            move |this, result, cx| {
                this.complete_mail_contact_history_load(&load, result, cx);
            },
        );
    }

    #[cfg(any(test, feature = "test-support"))]
    fn start_fixture_mail_contact_history(
        &mut self,
        contact: MailContactAddress,
        thread_id: String,
        request_generation: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        if !matches!(&self.mail_startup, MailStartup::Fixture) {
            return false;
        }
        let rows = self
            .mail_workspace()
            .map(|workspace| {
                mail_contact_history_rows(workspace, contact.as_str(), thread_id.as_str())
            })
            .unwrap_or_default();
        self.mail_contact_histories.insert(
            contact,
            MailContactHistoryState {
                request_generation,
                thread_id,
                status: MailContactHistoryStatus::Ready(rows.into()),
            },
        );
        cx.notify();
        true
    }

    fn complete_mail_contact_history_load(
        &mut self,
        load: &MailContactHistoryLoad,
        result: Result<MailContactHistoryPage, String>,
        cx: &mut Context<Self>,
    ) {
        let account_generation = load.account_generation;
        let request_generation = load.request_generation;
        let thread_id = load.thread_id.as_str();
        let contact = &load.contact;
        if self.mail_account_generation != account_generation
            || self.mail_open_thread_id.as_deref() != Some(thread_id)
            || self.mail_selected_contact_address.as_ref() != Some(contact)
            || self
                .mail_contact_histories
                .get(contact)
                .is_none_or(|state| state.request_generation != request_generation)
        {
            return;
        }
        let status = match result {
            Ok(page) => match contact_history_rows_from_page(page, contact) {
                Ok(rows) => MailContactHistoryStatus::Ready(rows.into()),
                Err(error) => MailContactHistoryStatus::Error(error),
            },
            Err(error) => MailContactHistoryStatus::Error(error),
        };
        let state = self
            .mail_contact_histories
            .get_mut(contact)
            .expect("guarded Mail contact history request must retain state");
        state.status = status;
        cx.notify();
    }
}

fn contact_history_rows_from_page(
    page: MailContactHistoryPage,
    requested_contact: &MailContactAddress,
) -> Result<Vec<MailContactHistoryRow>, String> {
    if page.contact() != requested_contact {
        return Err(format!(
            "mail contact history returned {} for requested {requested_contact}",
            page.contact()
        ));
    }
    Ok(page
        .into_entries()
        .into_iter()
        .map(|entry| MailContactHistoryRow {
            thread_id: entry.thread_id,
            subject: entry.subject,
            preview: entry.preview,
            timestamp: format_mail_received_at(&entry.received_at),
            selected: false,
        })
        .collect())
}
