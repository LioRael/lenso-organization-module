//! Shared Organization policy. This library owns no resources or Plugin activation.

/// Admission compares the complete caller Instance key without normalization.
/// D1 evaluates the equivalent binary equality inside its atomic write batch.
pub fn exact_caller<'a>(caller: Option<&'a str>, allowed: &[String]) -> Option<&'a str> {
    caller.filter(|caller| allowed.iter().any(|entry| entry == *caller))
}

pub fn valid_organization_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value.len() <= 200 && !value.chars().any(char::is_control)
}

pub fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

pub fn valid_name(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
}

pub fn valid_membership_request(
    idempotency_key: &str,
    organization_id: &str,
    subject: &str,
) -> bool {
    valid_name(idempotency_key, 256) && valid_name(organization_id, 256) && valid_name(subject, 256)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_is_exact_and_missing_callers_fail_closed() {
        let allowed = vec!["app.admin:one".to_owned()];
        assert_eq!(
            exact_caller(Some("app.admin:one"), &allowed),
            Some("app.admin:one")
        );
        for caller in [
            None,
            Some("app.admin"),
            Some("app.admin:two"),
            Some("App.admin:one"),
            Some("app.admin:one "),
            Some("*"),
        ] {
            assert_eq!(exact_caller(caller, &allowed), None);
        }
        assert_eq!(exact_caller(Some("app.admin:one"), &[]), None);
    }
    #[test]
    fn input_rules_preserve_byte_limits_and_normalization() {
        assert!(valid_organization_name("  Example  "));
        assert!(!valid_organization_name("a\nb"));
        assert!(!valid_organization_name(&"é".repeat(101)));
        assert!(valid_slug("team-2"));
        for slug in ["Team", "-team", "team-", "team_2", ""] {
            assert!(!valid_slug(slug));
        }
        assert!(valid_membership_request("key:1", "org_1", "auth.user:1"));
        assert!(!valid_membership_request("key", "org", "subject *"));
    }
}
