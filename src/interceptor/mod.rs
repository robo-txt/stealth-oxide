//! Conservative building blocks for opt-in CDP request interception.
//!
//! This module does not enable interception by itself. Header mutation causes
//! Chromium to rebuild a complete request-header override, so native wire order
//! cannot be observed or guaranteed.

use crate::{Error, Result};

const DENIED_HEADERS: &[&str] = &[
    "authorization",
    "connection",
    "content-length",
    "cookie",
    "host",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "set-cookie",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// A request header exposed to or produced by the interception policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    /// Creates a header without applying mutation-policy restrictions.
    ///
    /// This constructor represents headers already supplied by Chromium. Use
    /// [`HeaderPolicyBuilder::set_header`] to configure validated mutations.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns the header name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the header value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Validated, deterministic request-header additions and replacements.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeaderPolicy {
    mutations: Vec<Header>,
}

impl HeaderPolicy {
    /// Starts a header-policy builder.
    pub fn builder() -> HeaderPolicyBuilder {
        HeaderPolicyBuilder::default()
    }

    /// Returns true when applying this policy would not override headers.
    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
    }

    /// Applies additions and replacements to Chromium's exposed header set.
    ///
    /// Names are matched case-insensitively. Existing headers retain their
    /// position and spelling; additions are appended in policy order. Because
    /// CDP does not expose native wire order, the result must not be described
    /// as preserving HTTP/1 ordering or HTTP/2 and HTTP/3 serialization.
    ///
    /// # Errors
    ///
    /// Returns an error if Chromium's exposed set contains multiple spellings
    /// of a header targeted by this policy. An interceptor should fail open by
    /// continuing that request without a header override.
    pub fn apply(&self, original: &[Header]) -> Result<Vec<Header>> {
        let mut result = original.to_vec();

        for mutation in &self.mutations {
            let matches = result
                .iter()
                .enumerate()
                .filter(|(_, header)| header.name.eq_ignore_ascii_case(&mutation.name))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();

            match matches.as_slice() {
                [] => result.push(mutation.clone()),
                [index] => result[*index].value.clone_from(&mutation.value),
                _ => {
                    return Err(Error::invalid_interceptor_header(format!(
                        "cannot safely replace duplicate header {:?}",
                        mutation.name
                    )));
                }
            }
        }

        Ok(result)
    }
}

/// Builder for a validated [`HeaderPolicy`].
#[derive(Clone, Debug, Default)]
pub struct HeaderPolicyBuilder {
    mutations: Vec<Header>,
}

impl HeaderPolicyBuilder {
    /// Adds or replaces one request header.
    ///
    /// Repeating the same name with different ASCII casing updates the earlier
    /// rule, so a policy can never emit duplicate configured mutations.
    pub fn set_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let mutation = Header::new(name, value);
        if let Some(existing) = self
            .mutations
            .iter_mut()
            .find(|existing| existing.name.eq_ignore_ascii_case(&mutation.name))
        {
            *existing = mutation;
        } else {
            self.mutations.push(mutation);
        }
        self
    }

    /// Validates and builds the policy.
    ///
    /// # Errors
    ///
    /// Rejects pseudo-headers, invalid HTTP token names, values containing
    /// unsafe control characters, hop-by-hop headers, framing headers, and
    /// credential-bearing headers.
    pub fn build(self) -> Result<HeaderPolicy> {
        for mutation in &self.mutations {
            validate_mutation(mutation)?;
        }
        Ok(HeaderPolicy {
            mutations: self.mutations,
        })
    }
}

fn validate_mutation(header: &Header) -> Result<()> {
    if !is_http_token(&header.name) {
        return Err(Error::invalid_interceptor_header(format!(
            "header name {:?} is not a valid HTTP token",
            header.name
        )));
    }

    if DENIED_HEADERS
        .iter()
        .any(|denied| header.name.eq_ignore_ascii_case(denied))
    {
        return Err(Error::invalid_interceptor_header(format!(
            "header {:?} is reserved and cannot be overridden",
            header.name
        )));
    }

    if header.value.bytes().any(|byte| {
        byte == b'\r'
            || byte == b'\n'
            || byte == 0
            || (byte < 0x20 && byte != b'\t')
            || byte == 0x7f
    }) {
        return Err(Error::invalid_interceptor_header(format!(
            "header {:?} contains an unsafe control character",
            header.name
        )));
    }

    Ok(())
}

fn is_http_token(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}
