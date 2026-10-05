/// Where a mail account lives. Caches and split preferences are scoped to it,
/// so two servers never share local state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailLiveConfig {
    pub server: String,
}
