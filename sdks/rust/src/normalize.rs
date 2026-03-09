/// Normalize an event type to AllSource's `lowercase.dot.separated` convention.
///
/// Converts common naming conventions:
/// - **PascalCase**: `VerificationCreated` → `verification.created`
/// - **camelCase**: `userCreated` → `user.created`
/// - **snake_case**: `user_created` → `user.created`
/// - **SCREAMING_SNAKE**: `USER_CREATED` → `user.created`
/// - **kebab-case**: `user-created` → `user.created`
/// - **Already valid**: `user.created` → `user.created` (no-op)
///
/// # Examples
///
/// ```
/// use allsource::normalize_event_type;
///
/// assert_eq!(normalize_event_type("VerificationCreated"), "verification.created");
/// assert_eq!(normalize_event_type("UserCreated"), "user.created");
/// assert_eq!(normalize_event_type("user_created"), "user.created");
/// assert_eq!(normalize_event_type("user.created"), "user.created");
/// assert_eq!(normalize_event_type("auth.user.created"), "auth.user.created");
/// ```
pub fn normalize_event_type(event_type: &str) -> String {
    // Already contains dots → assume it's in the correct format, just lowercase it
    if event_type.contains('.') {
        return event_type.to_lowercase();
    }

    // Contains underscores or hyphens → split on those separators
    if event_type.contains('_') || event_type.contains('-') {
        return event_type
            .replace('-', ".")
            .replace('_', ".")
            .to_lowercase();
    }

    // PascalCase or camelCase → split on uppercase boundaries
    // Algorithm: collect chars, insert dots at boundaries:
    //   - lowercase → uppercase: "userCreated" → "user.Created"
    //   - uppercase → uppercase + lowercase: "APIKey" → "API.Key"
    let chars: Vec<char> = event_type.chars().collect();
    let mut result = String::with_capacity(chars.len() + 4);

    for i in 0..chars.len() {
        if i > 0 && chars[i].is_uppercase() {
            let prev_upper = chars[i - 1].is_uppercase();
            let prev_lower = chars[i - 1].is_lowercase();
            let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());

            // Insert dot when:
            // 1. Previous was lowercase (e.g., "user|C|reated")
            // 2. Previous was uppercase AND next is lowercase (e.g., "AP|I|K|ey" → dot before K)
            if prev_lower || (prev_upper && next_lower) {
                result.push('.');
            }
        }
        result.push(chars[i].to_lowercase().next().unwrap_or(chars[i]));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case() {
        assert_eq!(
            normalize_event_type("VerificationCreated"),
            "verification.created"
        );
        assert_eq!(normalize_event_type("UserCreated"), "user.created");
        assert_eq!(normalize_event_type("SessionDeleted"), "session.deleted");
        assert_eq!(
            normalize_event_type("TwoFactorCreated"),
            "two.factor.created"
        );
        assert_eq!(normalize_event_type("ApiKeyDeleted"), "api.key.deleted");
    }

    #[test]
    fn test_camel_case() {
        assert_eq!(normalize_event_type("userCreated"), "user.created");
        assert_eq!(normalize_event_type("sessionDeleted"), "session.deleted");
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(normalize_event_type("user_created"), "user.created");
        assert_eq!(
            normalize_event_type("verification_deleted"),
            "verification.deleted"
        );
        assert_eq!(normalize_event_type("USER_CREATED"), "user.created");
    }

    #[test]
    fn test_kebab_case() {
        assert_eq!(normalize_event_type("user-created"), "user.created");
    }

    #[test]
    fn test_already_valid() {
        assert_eq!(normalize_event_type("user.created"), "user.created");
        assert_eq!(
            normalize_event_type("auth.user.created"),
            "auth.user.created"
        );
        assert_eq!(
            normalize_event_type("auth.verification.created"),
            "auth.verification.created"
        );
    }

    #[test]
    fn test_uppercase_dots() {
        assert_eq!(
            normalize_event_type("Auth.User.Created"),
            "auth.user.created"
        );
    }

    #[test]
    fn test_acronym_handling() {
        assert_eq!(normalize_event_type("APIKeyCreated"), "api.key.created");
        assert_eq!(normalize_event_type("HTTPRequestSent"), "http.request.sent");
    }

    #[test]
    fn test_single_word() {
        assert_eq!(normalize_event_type("Created"), "created");
        assert_eq!(normalize_event_type("created"), "created");
    }
}
