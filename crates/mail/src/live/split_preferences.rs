use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    live::{cache::framed_mail_scope_key, cache::validate_cache_scope, MailLiveConfig},
    model::{
        MailSplitDefinition, MailSplitDraft, MailSplitEnabled, MailSplitId, MailSplitMove,
        MailSplitMutation, MailSplitOrder, MAIL_SPLIT_MAX_COUNT,
    },
};

const MAIL_SPLIT_PREFERENCES_SCHEMA_VERSION: u32 = 1;
const MAIL_SPLIT_PREFERENCES_MAX_BYTES: usize = 128 * 1024;
const MAIL_SPLIT_PREFERENCES_FILE_NAME: &str = "preferences-v1.bin";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredMailSplitScope {
    identity_subject: String,
    account_id: String,
    server_endpoint: String,
    api_endpoint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredMailSplitDefinition {
    id: String,
    name: String,
    query: String,
    order: u32,
    enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredMailSplitPreferences {
    schema_version: u32,
    scope: StoredMailSplitScope,
    splits: Vec<StoredMailSplitDefinition>,
}

struct MailSplitPreferencesState {
    definitions: Vec<MailSplitDefinition>,
    poisoned: bool,
}

#[derive(Clone)]
pub(crate) struct MailSplitPreferencesStore {
    path: PathBuf,
    key: [u8; 32],
    scope: StoredMailSplitScope,
    state: Arc<Mutex<MailSplitPreferencesState>>,
}

impl MailSplitPreferencesStore {
    pub(crate) fn open(
        config: &MailLiveConfig,
        api_endpoint: &Url,
        identity_subject: &str,
        account_id: &str,
    ) -> Result<Self, String> {
        validate_cache_scope("account identity", identity_subject)?;
        validate_cache_scope("JMAP mail account ID", account_id)?;
        let scope = StoredMailSplitScope {
            identity_subject: identity_subject.to_string(),
            account_id: account_id.to_string(),
            server_endpoint: normalize_endpoint("mail server endpoint", &config.server)?,
            api_endpoint: normalize_endpoint("JMAP API endpoint", api_endpoint.as_str())?,
        };
        let scope_key = mail_split_scope_key(&scope);
        let key =
            local_cache::load_or_create_cache_key(scope_key.as_str(), "mail split preferences")?;
        let path = app_model::app_data_dir()?
            .join("mail")
            .join("split-inboxes")
            .join(local_cache::cache_key_hash(scope_key.as_str()))
            .join(MAIL_SPLIT_PREFERENCES_FILE_NAME);
        let stored = local_cache::read_encrypted_json_strict_bounded::<StoredMailSplitPreferences>(
            &path,
            &key,
            MAIL_SPLIT_PREFERENCES_MAX_BYTES,
        )?;
        let (definitions, canonical_write_required) = match stored {
            Some(stored) => load_definitions(stored, &scope)?,
            None => (Vec::new(), false),
        };
        let store = Self {
            path,
            key,
            scope,
            state: Arc::new(Mutex::new(MailSplitPreferencesState {
                definitions,
                poisoned: false,
            })),
        };
        if canonical_write_required {
            let definitions = store.snapshot()?;
            store
                .persist(&definitions)
                .map_err(|error| error.to_string())?;
        }
        Ok(store)
    }

    pub(crate) fn snapshot(&self) -> Result<Vec<MailSplitDefinition>, String> {
        let state = self.lock()?;
        ensure_store_healthy(&state)?;
        Ok(state.definitions.clone())
    }

    pub(crate) fn mutate(
        &self,
        mutation: MailSplitMutation,
    ) -> Result<Vec<MailSplitDefinition>, String> {
        let mut state = self.lock()?;
        ensure_store_healthy(&state)?;
        let mut candidate = state.definitions.clone();
        apply_mutation(&mut candidate, mutation)?;
        normalize_definition_order(&mut candidate);
        validate_definition_set(&candidate)?;
        if candidate == state.definitions {
            return Ok(candidate);
        }
        if let Err(error) = self.persist(&candidate) {
            if error.commit_state_is_ambiguous() {
                state.poisoned = true;
            }
            return Err(error.to_string());
        }
        state.definitions = candidate.clone();
        Ok(candidate)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, MailSplitPreferencesState>, String> {
        self.state
            .lock()
            .map_err(|_| "Mail split preferences lock was poisoned".to_string())
    }

    fn persist(
        &self,
        definitions: &[MailSplitDefinition],
    ) -> Result<(), local_cache::DurableEncryptedWriteError> {
        let stored = StoredMailSplitPreferences::from_definitions(&self.scope, definitions);
        local_cache::write_encrypted_json_durable_bounded(
            &self.path,
            &self.key,
            &stored,
            MAIL_SPLIT_PREFERENCES_MAX_BYTES,
        )
    }
}

impl StoredMailSplitPreferences {
    fn from_definitions(scope: &StoredMailSplitScope, definitions: &[MailSplitDefinition]) -> Self {
        Self {
            schema_version: MAIL_SPLIT_PREFERENCES_SCHEMA_VERSION,
            scope: scope.clone(),
            splits: definitions
                .iter()
                .map(|definition| StoredMailSplitDefinition {
                    id: definition.id().as_str().to_string(),
                    name: definition.name().to_string(),
                    query: definition.query().as_str().to_string(),
                    order: definition.order().get(),
                    enabled: definition.is_enabled(),
                })
                .collect(),
        }
    }
}

fn ensure_store_healthy(state: &MailSplitPreferencesState) -> Result<(), String> {
    if state.poisoned {
        return Err(
            "Mail split preferences are unavailable after an ambiguous durable commit".to_string(),
        );
    }
    Ok(())
}

/// Loaded definitions, and whether the stored file must be rewritten in canonical form.
type LoadedDefinitions = (Vec<MailSplitDefinition>, bool);

fn load_definitions(
    stored: StoredMailSplitPreferences,
    expected_scope: &StoredMailSplitScope,
) -> Result<LoadedDefinitions, String> {
    if stored.schema_version != MAIL_SPLIT_PREFERENCES_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported Mail split preferences schema version {}; expected {}",
            stored.schema_version, MAIL_SPLIT_PREFERENCES_SCHEMA_VERSION
        ));
    }
    if &stored.scope != expected_scope {
        return Err("Mail split preferences scope does not match the active account".to_string());
    }
    if stored.splits.len() > MAIL_SPLIT_MAX_COUNT {
        return Err(format!(
            "Mail split preferences contain {} definitions; at most {MAIL_SPLIT_MAX_COUNT} are supported",
            stored.splits.len()
        ));
    }
    let original = stored.clone();
    let mut definitions = stored
        .splits
        .into_iter()
        .map(stored_definition)
        .collect::<Result<Vec<_>, _>>()?;
    normalize_definition_order(&mut definitions);
    validate_definition_set(&definitions)?;
    let canonical = StoredMailSplitPreferences::from_definitions(expected_scope, &definitions);
    Ok((definitions, original != canonical))
}

fn stored_definition(stored: StoredMailSplitDefinition) -> Result<MailSplitDefinition, String> {
    let id = MailSplitId::parse(stored.id).map_err(|error| error.to_string())?;
    let draft = MailSplitDraft::parse(stored.name, stored.query)
        .map_err(|error| format!("Invalid persisted Mail split {id}: {error}"))?;
    Ok(MailSplitDefinition::from_parts(
        id,
        draft,
        MailSplitOrder::new(stored.order),
        MailSplitEnabled::from(stored.enabled),
    ))
}

fn apply_mutation(
    definitions: &mut Vec<MailSplitDefinition>,
    mutation: MailSplitMutation,
) -> Result<(), String> {
    match mutation {
        MailSplitMutation::Create(draft) => {
            if definitions.len() == MAIL_SPLIT_MAX_COUNT {
                return Err(format!(
                    "At most {MAIL_SPLIT_MAX_COUNT} Mail splits can be configured"
                ));
            }
            let id = new_split_id(definitions)?;
            let order = MailSplitOrder::new(
                u32::try_from(definitions.len()).expect("bounded Mail split count must fit in u32"),
            );
            definitions.push(MailSplitDefinition::from_parts(
                id,
                draft,
                order,
                MailSplitEnabled::Enabled,
            ));
        }
        MailSplitMutation::Edit { id, draft } => {
            definition_mut(definitions, &id)?.replace_draft(draft);
        }
        MailSplitMutation::SetEnabled { id, enabled } => {
            definition_mut(definitions, &id)?.set_enabled(enabled);
        }
        MailSplitMutation::Move { id, direction } => {
            let index = definitions
                .iter()
                .position(|definition| definition.id() == &id)
                .ok_or_else(|| format!("Mail split {id} no longer exists"))?;
            let destination = match direction {
                MailSplitMove::Earlier => index.checked_sub(1),
                MailSplitMove::Later => (index + 1 < definitions.len()).then_some(index + 1),
            };
            if let Some(destination) = destination {
                definitions.swap(index, destination);
                for (index, definition) in definitions.iter_mut().enumerate() {
                    definition.set_order(MailSplitOrder::new(
                        u32::try_from(index).expect("bounded Mail split count must fit in u32"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn definition_mut<'a>(
    definitions: &'a mut [MailSplitDefinition],
    id: &MailSplitId,
) -> Result<&'a mut MailSplitDefinition, String> {
    definitions
        .iter_mut()
        .find(|definition| definition.id() == id)
        .ok_or_else(|| format!("Mail split {id} no longer exists"))
}

fn new_split_id(definitions: &[MailSplitDefinition]) -> Result<MailSplitId, String> {
    for _ in 0..4 {
        let entropy = local_cache::encode_cache_key(&local_cache::generate_cache_key());
        let id = MailSplitId::parse(local_cache::cache_key_hash(entropy.as_str()))
            .expect("SHA-256 hex output must be a valid Mail split ID");
        if definitions.iter().all(|definition| definition.id() != &id) {
            return Ok(id);
        }
    }
    Err("Failed to allocate a unique Mail split ID".to_string())
}

fn normalize_definition_order(definitions: &mut [MailSplitDefinition]) {
    definitions.sort_by(|left, right| {
        left.order()
            .cmp(&right.order())
            .then_with(|| left.id().cmp(right.id()))
    });
    for (index, definition) in definitions.iter_mut().enumerate() {
        definition.set_order(MailSplitOrder::new(
            u32::try_from(index).expect("bounded Mail split count must fit in u32"),
        ));
    }
}

fn validate_definition_set(definitions: &[MailSplitDefinition]) -> Result<(), String> {
    if definitions.len() > MAIL_SPLIT_MAX_COUNT {
        return Err(format!(
            "At most {MAIL_SPLIT_MAX_COUNT} Mail splits can be configured"
        ));
    }
    let mut ids = HashSet::with_capacity(definitions.len());
    for (index, definition) in definitions.iter().enumerate() {
        if !ids.insert(definition.id()) {
            return Err(format!(
                "Mail split preferences contain duplicate ID {}",
                definition.id()
            ));
        }
        let expected_order =
            u32::try_from(index).expect("bounded Mail split count must fit in u32");
        if definition.order().get() != expected_order {
            return Err("Mail split preferences are not in canonical order".to_string());
        }
    }
    Ok(())
}

fn mail_split_scope_key(scope: &StoredMailSplitScope) -> String {
    framed_mail_scope_key(
        "mail-split-preferences-v1",
        &[
            scope.identity_subject.as_str(),
            scope.account_id.as_str(),
            scope.server_endpoint.as_str(),
            scope.api_endpoint.as_str(),
        ],
    )
}

fn normalize_endpoint(label: &str, raw: &str) -> Result<String, String> {
    let mut endpoint = Url::parse(raw).map_err(|error| format!("Invalid {label}: {error}"))?;
    endpoint.set_fragment(None);
    let normalized_path = endpoint.path().trim_end_matches('/').to_string();
    endpoint.set_path(if normalized_path.is_empty() {
        "/"
    } else {
        normalized_path.as_str()
    });
    let mut normalized = endpoint.to_string();
    if endpoint.path() == "/" && endpoint.query().is_none() {
        let normalized_len = normalized.trim_end_matches('/').len();
        normalized.truncate(normalized_len);
    }
    Ok(normalized)
}
