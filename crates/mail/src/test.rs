mod profile;

pub(crate) fn mail_profile_count(env_var: &str, default: usize) -> usize {
    match std::env::var(env_var) {
        Ok(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or_else(|| panic!("{env_var} must be a positive integer")),
        Err(std::env::VarError::NotPresent) => default,
        Err(err) => panic!("{env_var} is not valid UTF-8: {err}"),
    }
}
