use super::super::{MailStartup, SurfaceState};
use crate::model::MailAccountInfo;

impl SurfaceState {
    pub(crate) fn mail_account_catalog(&self) -> Vec<MailAccountInfo> {
        match &self.mail_startup {
            MailStartup::Ready(bootstrap) => bootstrap.accounts.clone(),
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => self
                .mail_workspace()
                .map(fixture_mail_account)
                .into_iter()
                .collect(),
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => Vec::new(),
        }
    }

    pub(crate) fn mail_account_palette_options(&self) -> Vec<(usize, MailAccountInfo)> {
        let query = self
            .mail_account_palette
            .as_ref()
            .map(|state| state.query.trim().to_ascii_lowercase())
            .unwrap_or_default();
        sorted_mail_accounts(self.mail_account_catalog())
            .into_iter()
            .enumerate()
            .filter_map(|(index, account)| {
                let matches = query.is_empty()
                    || account.name.to_ascii_lowercase().contains(&query)
                    || account
                        .address
                        .as_deref()
                        .is_some_and(|address| address.to_ascii_lowercase().contains(&query));
                matches.then_some((index + 1, account))
            })
            .collect()
    }

    pub(crate) fn mail_current_account_info(&self) -> Option<MailAccountInfo> {
        let account_id = self.mail_workspace()?.account_id.as_str();
        self.mail_account_catalog()
            .into_iter()
            .find(|account| account.id == account_id)
    }

    pub(crate) fn mail_displayed_account_info(&self) -> Option<MailAccountInfo> {
        self.mail_switching_account_id
            .as_deref()
            .and_then(|account_id| {
                self.mail_account_catalog()
                    .into_iter()
                    .find(|account| account.id == account_id)
            })
            .or_else(|| self.mail_current_account_info())
    }

    pub(crate) fn mail_active_account_can_submit(&self) -> bool {
        match self.mail_startup {
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => true,
            MailStartup::Ready(_) => {
                let account = self
                    .mail_current_account_info()
                    .expect("active Mail account must be present in its catalog");
                account.can_submit && !account.is_read_only
            }
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => false,
        }
    }

    pub(crate) fn mail_active_account_is_read_only(&self) -> bool {
        match self.mail_startup {
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => false,
            MailStartup::Ready(_) => {
                self.mail_current_account_info()
                    .expect("active Mail account must be present in its catalog")
                    .is_read_only
            }
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => true,
        }
    }
}

pub(super) fn sorted_mail_accounts(mut accounts: Vec<MailAccountInfo>) -> Vec<MailAccountInfo> {
    accounts.sort_by(|left, right| {
        (!left.is_primary)
            .cmp(&(!right.is_primary))
            .then_with(|| (!left.is_personal).cmp(&(!right.is_personal)))
            .then_with(|| account_sort_label(left).cmp(&account_sort_label(right)))
            .then_with(|| left.id.cmp(&right.id))
    });
    accounts
}

fn account_sort_label(account: &MailAccountInfo) -> String {
    account
        .address
        .as_deref()
        .unwrap_or(account.name.as_str())
        .to_ascii_lowercase()
}

#[cfg(any(test, feature = "test-support"))]
fn fixture_mail_account(workspace: &crate::ui::MailWorkspace) -> MailAccountInfo {
    MailAccountInfo {
        id: workspace.account_id.clone(),
        name: if workspace.display_name.is_empty() {
            workspace.owner_username.clone()
        } else {
            workspace.display_name.clone()
        },
        address: workspace.mailbox_email.clone(),
        is_primary: true,
        is_personal: true,
        is_read_only: false,
        can_submit: true,
    }
}
