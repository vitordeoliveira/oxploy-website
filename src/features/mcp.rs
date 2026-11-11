//! Who may reach the owner's MCP server: oxploy, which turns it on in the app's MCP tab,
//! gives the site a token in `OXPLOY_MCP_TOKEN` and sends it with each call as
//! `Authorization: Bearer <token>`. Without the token, nobody gets in.

use topcoat::{
    context::Cx,
    router::{header::AUTHORIZATION, request::headers},
};

use crate::features::admin::same;

/// The variable oxploy gives the site its token in.
const TOKEN_VARIABLE: &str = "OXPLOY_MCP_TOKEN";

/// The token, if the environment has one.
fn token() -> Option<String> {
    std::env::var(TOKEN_VARIABLE).ok().filter(|token| !token.is_empty())
}

/// Whether the current request carries the token.
pub fn is_authorized(cx: &Cx) -> bool {
    let given = headers(cx)
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(bearer);
    matches(token().as_deref(), given)
}

/// Whether `given` is the token. Without a token, nothing is.
fn matches(token: Option<&str>, given: Option<&str>) -> bool {
    match (token, given) {
        (Some(token), Some(given)) => same(token, given),
        _ => false,
    }
}

/// The token in an `Authorization` header's value, if it uses the Bearer scheme, whose
/// name is case-insensitive.
fn bearer(value: &str) -> Option<&str> {
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("Bearer").then(|| token.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_only_the_exact_token() {
        assert!(matches(Some("3f9a1c"), Some("3f9a1c")));
        assert!(!matches(Some("3f9a1c"), Some("3f9a1")));
        assert!(!matches(Some("3f9a1c"), Some("3F9A1C")));
        assert!(!matches(Some("3f9a1c"), None));
        assert!(!matches(None, Some("")));
        assert!(!matches(None, None));
    }

    #[test]
    fn reads_the_token_of_a_bearer_header() {
        assert_eq!(bearer("Bearer 3f9a1c"), Some("3f9a1c"));
        assert_eq!(bearer("bearer 3f9a1c"), Some("3f9a1c"));
        assert_eq!(bearer("Basic YWRtaW46"), None);
        assert_eq!(bearer("3f9a1c"), None);
    }
}
