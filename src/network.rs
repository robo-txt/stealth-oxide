//! Passive, redacted Chromium network diagnostics and failure classification.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestServedFromCache, EventRequestWillBeSent,
    EventRequestWillBeSentExtraInfo, EventResponseReceived, EventResponseReceivedExtraInfo,
    Headers, Response,
};

use crate::{
    environment::{Finding, FindingSeverity},
    profiles::IdentityConfig,
    redaction::{MAX_COLLECTION_ITEMS, bounded, safe_headers, url},
    topology::{Coverage, ResourceScope, classify_resource, sanitize_initiator_origin},
};

/// Compares network-stack identity headers with the configured browser identity.
///
/// Findings contain stable messages rather than observed header values. Missing
/// extra-info evidence is reported as informational instead of being treated as
/// a contradiction.
pub fn validate_request_identity(
    request: &NetworkRequestAudit,
    identity: &IdentityConfig,
) -> Vec<Finding> {
    let Some(headers) = request.request_identity_headers.last() else {
        return vec![Finding {
            severity: FindingSeverity::Info,
            code: "request-identity-headers-unobserved",
            message: "Chrome did not provide request extra-info identity headers",
        }];
    };
    let mut findings = Vec::new();
    if headers.get("user-agent") != Some(&identity.user_agent) {
        findings.push(Finding {
            severity: FindingSeverity::Contradiction,
            code: "request-user-agent-mismatch",
            message: "the transmitted User-Agent differs from the configured identity",
        });
    }
    match headers.get("accept-language") {
        Some(value) if language_tags(value) == identity.languages => {}
        Some(_) => findings.push(Finding {
            severity: FindingSeverity::Contradiction,
            code: "request-language-mismatch",
            message: "the transmitted language order differs from the configured identity",
        }),
        None => findings.push(Finding {
            severity: FindingSeverity::Warning,
            code: "request-language-unobserved",
            message: "the transmitted request did not expose an Accept-Language header",
        }),
    }
    if let Some(client_hints) = &identity.client_hints {
        let expected_platform = format!("\"{}\"", client_hints.platform);
        if headers.get("sec-ch-ua-platform") != Some(&expected_platform) {
            findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "request-client-hint-platform-mismatch",
                message: "the transmitted Client Hint platform differs from the configured identity",
            });
        }
        let expected_mobile = if client_hints.mobile { "?1" } else { "?0" };
        if headers.get("sec-ch-ua-mobile").map(String::as_str) != Some(expected_mobile) {
            findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "request-client-hint-mobile-mismatch",
                message: "the transmitted Client Hint mobile value differs from the configured identity",
            });
        }
        match headers.get("sec-ch-ua") {
            Some(value)
                if client_hints.brands.iter().all(|brand| {
                    value.contains(&format!("\"{}\"", brand.brand))
                        && value.contains(&format!("v=\"{}\"", brand.version))
                }) => {}
            Some(_) => findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "request-client-hint-brands-mismatch",
                message: "the transmitted Client Hint brands differ from the configured identity",
            }),
            None => findings.push(Finding {
                severity: FindingSeverity::Warning,
                code: "request-client-hint-brands-unobserved",
                message: "the transmitted request did not expose Client Hint brands",
            }),
        }
        check_quoted_hint(
            headers,
            "sec-ch-ua-arch",
            &client_hints.architecture,
            "request-client-hint-architecture-mismatch",
            "the transmitted Client Hint architecture differs from the configured identity",
            &mut findings,
        );
        check_quoted_hint(
            headers,
            "sec-ch-ua-bitness",
            &client_hints.bitness,
            "request-client-hint-bitness-mismatch",
            "the transmitted Client Hint bitness differs from the configured identity",
            &mut findings,
        );
        check_quoted_hint(
            headers,
            "sec-ch-ua-model",
            &client_hints.model,
            "request-client-hint-model-mismatch",
            "the transmitted Client Hint model differs from the configured identity",
            &mut findings,
        );
        check_quoted_hint(
            headers,
            "sec-ch-ua-platform-version",
            &client_hints.platform_version,
            "request-client-hint-platform-version-mismatch",
            "the transmitted Client Hint platform version differs from the configured identity",
            &mut findings,
        );
        if headers
            .get("sec-ch-ua-full-version-list")
            .is_some_and(|value| {
                !client_hints.full_version_list.iter().all(|brand| {
                    value.contains(&format!("\"{}\"", brand.brand))
                        && value.contains(&format!("v=\"{}\"", brand.version))
                })
            })
        {
            findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "request-client-hint-full-version-list-mismatch",
                message: "the transmitted full-version Client Hint brands differ from the configured identity",
            });
        }
        if headers.get("sec-ch-ua-form-factors").is_some_and(|value| {
            client_hints.form_factors.as_ref().is_some_and(|expected| {
                !expected
                    .iter()
                    .all(|factor| value.contains(&format!("\"{factor}\"")))
            })
        }) {
            findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "request-client-hint-form-factors-mismatch",
                message: "the transmitted Client Hint form factors differ from the configured identity",
            });
        }
        if headers.get("sec-ch-ua-wow64").is_some_and(|value| {
            client_hints
                .wow64
                .is_some_and(|expected| value != if expected { "?1" } else { "?0" })
        }) {
            findings.push(Finding {
                severity: FindingSeverity::Contradiction,
                code: "request-client-hint-wow64-mismatch",
                message: "the transmitted Client Hint WOW64 value differs from the configured identity",
            });
        }
    }
    findings
}

fn check_quoted_hint(
    headers: &BTreeMap<String, String>,
    name: &str,
    expected: &str,
    code: &'static str,
    message: &'static str,
    findings: &mut Vec<Finding>,
) {
    if headers
        .get(name)
        .is_some_and(|value| value != &format!("\"{expected}\""))
    {
        findings.push(Finding {
            severity: FindingSeverity::Contradiction,
            code,
            message,
        });
    }
}

fn language_tags(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.split(';').next().unwrap_or(item).trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Evaluates high-entropy Client Hint delivery against opt-ins observed earlier
/// in the same audit window.
///
/// Unexplained delivery is a warning rather than a contradiction because Chrome
/// can persist Client Hint preferences in a reused browser profile before the
/// audit begins.
pub fn validate_client_hint_negotiation(audit: &NetworkAudit) -> Vec<Finding> {
    let mut findings = Vec::new();
    for request in audit.requests() {
        if request.request_identity_headers.is_empty() {
            if !request.eligible_high_entropy_hints.is_empty() {
                push_unique_finding(
                    &mut findings,
                    Finding {
                        severity: FindingSeverity::Info,
                        code: "client-hint-request-headers-unobserved",
                        message: "request extra-info was unavailable for Client Hint negotiation validation",
                    },
                );
            }
            continue;
        }
        for hint in request
            .sent_high_entropy_hints
            .difference(&request.eligible_high_entropy_hints)
        {
            let _ = hint;
            push_unique_finding(
                &mut findings,
                Finding {
                    severity: FindingSeverity::Warning,
                    code: "high-entropy-client-hint-without-observed-opt-in",
                    message: "a high-entropy Client Hint was sent without an opt-in observed earlier in this audit",
                },
            );
        }
        for hint in request
            .eligible_high_entropy_hints
            .difference(&request.sent_high_entropy_hints)
        {
            let _ = hint;
            push_unique_finding(
                &mut findings,
                Finding {
                    severity: FindingSeverity::Warning,
                    code: "opted-in-client-hint-not-observed",
                    message: "a previously opted-in high-entropy Client Hint was not observed on a later request",
                },
            );
        }
    }
    findings
}

fn push_unique_finding(findings: &mut Vec<Finding>, finding: Finding) {
    if !findings.iter().any(|item| item.code == finding.code) {
        findings.push(finding);
    }
}

/// A redacted response observed before a redirect was followed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectHop {
    /// Redirect URL with credentials, query, and fragment removed.
    pub url: String,
    /// HTTP status reported by Chromium.
    pub status: i64,
    /// Negotiated application protocol, when Chromium supplied it.
    pub protocol: Option<String>,
}

/// One bounded request lifecycle correlated by a CDP request identifier.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkRequestAudit {
    /// Opaque CDP request identifier. It is local to the inspected session.
    pub request_id: String,
    /// Current URL with credentials, query, and fragment removed.
    pub url: Option<String>,
    /// HTTP method.
    pub method: Option<String>,
    /// Opaque loader identifier supplied by CDP.
    pub loader_id: Option<String>,
    /// CDP resource type.
    pub resource_type: Option<String>,
    /// Main/child-frame scope derived from the configured main frame ID.
    pub scope: ResourceScope,
    /// Whether this page-session event had enough frame evidence to classify.
    pub coverage: Coverage,
    /// Sanitized initiator origin, when Chromium supplied an initiator URL.
    pub initiator_origin: Option<String>,
    /// Redirect responses in observation order.
    pub redirects: Vec<RedirectHop>,
    /// Final HTTP status, when a response was observed.
    pub status: Option<i64>,
    /// Status observations from response extra-info events.
    ///
    /// These remain a sequence because CDP does not guarantee whether extra-info
    /// arrives before or after the corresponding redirect/response event.
    pub extra_statuses: Vec<i64>,
    /// Negotiated application protocol, for example `h2` or `h3`.
    pub protocol: Option<String>,
    /// Whether Chromium reported any cache source for this lifecycle.
    pub from_cache: bool,
    /// Whether Chromium reported a service-worker response.
    pub from_service_worker: bool,
    /// Whether Chrome reported reuse of the underlying connection.
    pub connection_reused: Option<bool>,
    /// Allowlisted response headers only. Cookie and authentication headers are excluded.
    pub response_headers: BTreeMap<String, String>,
    /// Ordered allowlisted request identity header sets from extra-info events.
    pub request_identity_headers: Vec<BTreeMap<String, String>>,
    /// High-entropy hints allowed by an `Accept-CH` observed before this request.
    pub eligible_high_entropy_hints: BTreeSet<String>,
    /// High-entropy hint header names Chrome transmitted on this request.
    pub sent_high_entropy_hints: BTreeSet<String>,
    /// Client Hint tokens requested by this response, including low-entropy tokens.
    pub response_accept_ch: BTreeSet<String>,
    /// Encoded bytes Chromium reported when loading completed.
    pub encoded_data_length: Option<f64>,
    /// Broad failure family, when loading failed.
    pub failure_category: Option<FailureCategory>,
    /// Bounded Chromium network error name without free-form detail text.
    pub failure_name: Option<String>,
    /// Whether a terminal loading-finished or loading-failed event was observed.
    pub finished: bool,
}

impl NetworkRequestAudit {
    fn placeholder(request_id: &str) -> Self {
        Self {
            request_id: bounded(request_id),
            url: None,
            method: None,
            loader_id: None,
            resource_type: None,
            scope: ResourceScope::Unknown,
            coverage: Coverage::Incomplete,
            initiator_origin: None,
            redirects: Vec::new(),
            status: None,
            extra_statuses: Vec::new(),
            protocol: None,
            from_cache: false,
            from_service_worker: false,
            connection_reused: None,
            response_headers: BTreeMap::new(),
            request_identity_headers: Vec::new(),
            eligible_high_entropy_hints: BTreeSet::new(),
            sent_high_entropy_hints: BTreeSet::new(),
            response_accept_ch: BTreeSet::new(),
            encoded_data_length: None,
            failure_category: None,
            failure_name: None,
            finished: false,
        }
    }
}

/// Aggregate counts derived from the retained audit window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetworkAuditSummary {
    /// Number of retained correlated request lifecycles.
    pub requests: usize,
    /// Number with a terminal event.
    pub finished: usize,
    /// Number that used any cache source.
    pub cache_hits: usize,
    /// Number served by a service worker.
    pub service_worker_responses: usize,
    /// Counts by negotiated protocol.
    pub protocols: BTreeMap<String, usize>,
    /// Counts by broad loading-failure family.
    pub failures: BTreeMap<String, usize>,
}

/// A passive, bounded collector for Chrome `Network` domain events.
///
/// This collector never enables Fetch interception, modifies requests, reads
/// bodies, or replays traffic. Call the matching `observe_*` method for events
/// received from the same page/session. The oldest lifecycle is evicted when
/// capacity is reached.
#[derive(Debug)]
pub struct NetworkAudit {
    capacity: usize,
    main_frame_id: Option<String>,
    order: VecDeque<String>,
    requests: BTreeMap<String, NetworkRequestAudit>,
    accepted_client_hints: BTreeMap<String, BTreeSet<String>>,
}

impl Default for NetworkAudit {
    fn default() -> Self {
        Self::new(MAX_COLLECTION_ITEMS)
    }
}

impl NetworkAudit {
    /// Creates an audit retaining at most `capacity` request lifecycles.
    /// Capacity is clamped to the library diagnostic limit and to at least one.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.clamp(1, MAX_COLLECTION_ITEMS),
            main_frame_id: None,
            order: VecDeque::new(),
            requests: BTreeMap::new(),
            accepted_client_hints: BTreeMap::new(),
        }
    }

    /// Sets the main frame used to classify documents and subresources.
    pub fn set_main_frame_id(&mut self, frame_id: impl Into<String>) {
        self.main_frame_id = Some(frame_id.into());
    }

    /// Returns retained lifecycles in first-observation order.
    pub fn requests(&self) -> impl Iterator<Item = &NetworkRequestAudit> {
        self.order.iter().filter_map(|id| self.requests.get(id))
    }

    /// Returns one retained lifecycle by CDP request identifier.
    pub fn get(&self, request_id: &str) -> Option<&NetworkRequestAudit> {
        self.requests.get(request_id)
    }

    /// Derives aggregate counts from the currently retained window.
    pub fn summary(&self) -> NetworkAuditSummary {
        let mut summary = NetworkAuditSummary::default();
        for request in self.requests() {
            summary.requests += 1;
            summary.finished += usize::from(request.finished);
            summary.cache_hits += usize::from(request.from_cache);
            summary.service_worker_responses += usize::from(request.from_service_worker);
            if let Some(protocol) = &request.protocol {
                *summary.protocols.entry(protocol.clone()).or_default() += 1;
            }
            if let Some(category) = request.failure_category {
                increment_failure_count(&mut summary.failures, category);
            }
        }
        summary
    }

    /// Observes a request and any redirect response attached to it.
    pub fn observe_request_will_be_sent(&mut self, event: &EventRequestWillBeSent) {
        let request_id = event.request_id.as_ref();
        let main_frame_id = self.main_frame_id.clone();
        if let Some(response) = &event.redirect_response {
            self.observe_accept_ch(&response.url, &response.headers);
        }
        let eligible_hints = origin(&event.request.url)
            .and_then(|origin| self.accepted_client_hints.get(&origin).cloned())
            .unwrap_or_default();
        let request = self.entry(request_id);
        if let Some(response) = &event.redirect_response {
            push_bounded(&mut request.redirects, redirect_hop(response));
            apply_response(request, response);
            request.status = None;
        }
        request.url = Some(url(&event.request.url));
        request.eligible_high_entropy_hints = eligible_hints
            .into_iter()
            .filter(|hint| is_high_entropy_hint(hint))
            .collect();
        request.method = Some(bounded(&event.request.method));
        request.loader_id = Some(bounded(event.loader_id.as_ref()));
        request.resource_type = event
            .r#type
            .as_ref()
            .map(|value| value.as_ref().to_string());
        request.scope = classify_resource(
            event.frame_id.as_ref().map(AsRef::as_ref),
            main_frame_id.as_deref(),
            request.resource_type.as_deref().unwrap_or("unknown"),
        );
        request.coverage = if event.frame_id.is_some() {
            Coverage::Observed
        } else {
            Coverage::Incomplete
        };
        request.initiator_origin = sanitize_initiator_origin(event.initiator.url.as_deref());
    }

    /// Observes request headers actually supplied by Chrome's network stack.
    pub fn observe_request_extra_info(&mut self, event: &EventRequestWillBeSentExtraInfo) {
        let headers = identity_headers(&event.headers);
        let sent_hints = headers
            .keys()
            .filter(|name| is_high_entropy_hint(name))
            .cloned()
            .collect();
        let request = self.entry(event.request_id.as_ref());
        request.sent_high_entropy_hints = sent_hints;
        push_bounded(&mut request.request_identity_headers, headers);
    }

    /// Observes a response without reading its body.
    pub fn observe_response_received(&mut self, event: &EventResponseReceived) {
        {
            let request = self.entry(event.request_id.as_ref());
            request.loader_id = Some(bounded(event.loader_id.as_ref()));
            request.resource_type = Some(event.r#type.as_ref().to_string());
            apply_response(request, &event.response);
            request.response_accept_ch = accept_ch_tokens(&event.response.headers);
        }
        self.observe_accept_ch(&event.response.url, &event.response.headers);
    }

    /// Observes status and allowlisted headers from network-stack extra info.
    pub fn observe_response_extra_info(&mut self, event: &EventResponseReceivedExtraInfo) {
        let response_url = {
            let request = self.entry(event.request_id.as_ref());
            push_bounded(&mut request.extra_statuses, event.status_code);
            request
                .response_headers
                .extend(safe_headers(event.headers.inner()));
            let accept_ch = accept_ch_tokens(&event.headers);
            if header_value(&event.headers, "accept-ch").is_some() {
                request.response_accept_ch = accept_ch;
            }
            request.url.clone()
        };
        if let Some(response_url) = response_url {
            self.observe_accept_ch(&response_url, &event.headers);
        }
    }

    /// Marks a lifecycle as served from Chrome's cache.
    pub fn observe_request_served_from_cache(&mut self, event: &EventRequestServedFromCache) {
        self.entry(event.request_id.as_ref()).from_cache = true;
    }

    /// Records Chrome's terminal byte count.
    pub fn observe_loading_finished(&mut self, event: &EventLoadingFinished) {
        let request = self.entry(event.request_id.as_ref());
        request.encoded_data_length = Some(event.encoded_data_length);
        request.finished = true;
    }

    /// Classifies and records a terminal Chrome loading failure.
    pub fn observe_loading_failed(&mut self, event: &EventLoadingFailed) {
        let request = self.entry(event.request_id.as_ref());
        let blocked_reason = event.blocked_reason.as_ref().map(|_| "reported");
        request.failure_category = Some(classify_failure(
            &event.error_text,
            event.canceled.unwrap_or(false),
            blocked_reason,
            event.cors_error_status.is_some(),
        ));
        request.failure_name = Some(bounded(&sanitize_error_name(&event.error_text)));
        request.finished = true;
    }

    fn entry(&mut self, request_id: &str) -> &mut NetworkRequestAudit {
        if !self.requests.contains_key(request_id) {
            while self.requests.len() >= self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    self.requests.remove(&oldest);
                }
            }
            self.order.push_back(request_id.to_string());
            self.requests.insert(
                request_id.to_string(),
                NetworkRequestAudit::placeholder(request_id),
            );
        }
        self.requests.get_mut(request_id).expect("entry inserted")
    }

    fn observe_accept_ch(&mut self, response_url: &str, headers: &Headers) {
        let Some(value) = header_value(headers, "accept-ch") else {
            return;
        };
        let Some(origin) = origin(response_url) else {
            return;
        };
        self.accepted_client_hints
            .insert(origin, parse_client_hint_tokens(value));
    }
}

fn origin(value: &str) -> Option<String> {
    let parsed = url::Url::parse(value).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed.host_str()?;
    Some(match parsed.port() {
        Some(port) => format!("{}://{}:{port}", parsed.scheme(), host),
        None => format!("{}://{}", parsed.scheme(), host),
    })
}

fn header_value<'a>(headers: &'a Headers, target: &str) -> Option<&'a str> {
    headers
        .inner()
        .as_object()?
        .iter()
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case(target)
                .then(|| value.as_str())
                .flatten()
        })
}

fn accept_ch_tokens(headers: &Headers) -> BTreeSet<String> {
    header_value(headers, "accept-ch")
        .map(parse_client_hint_tokens)
        .unwrap_or_default()
}

fn parse_client_hint_tokens(value: &str) -> BTreeSet<String> {
    value
        .split(',')
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.starts_with("sec-ch-ua-") || token == "sec-ch-ua")
        .take(MAX_COLLECTION_ITEMS)
        .collect()
}

fn is_high_entropy_hint(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "sec-ch-ua-arch"
            | "sec-ch-ua-bitness"
            | "sec-ch-ua-form-factors"
            | "sec-ch-ua-full-version"
            | "sec-ch-ua-full-version-list"
            | "sec-ch-ua-model"
            | "sec-ch-ua-platform-version"
            | "sec-ch-ua-wow64"
    )
}

fn redirect_hop(response: &Response) -> RedirectHop {
    RedirectHop {
        url: url(&response.url),
        status: response.status,
        protocol: response.protocol.as_deref().map(bounded),
    }
}

fn apply_response(request: &mut NetworkRequestAudit, response: &Response) {
    if request.url.is_none() {
        request.url = Some(url(&response.url));
    }
    request.status = Some(response.status);
    request.protocol = response.protocol.as_deref().map(bounded);
    request.from_cache |= response.from_disk_cache.unwrap_or(false)
        || response.from_prefetch_cache.unwrap_or(false)
        || response.from_early_hints.unwrap_or(false);
    request.from_service_worker |= response.from_service_worker.unwrap_or(false);
    request.connection_reused = Some(response.connection_reused);
    request
        .response_headers
        .extend(safe_headers(response.headers.inner()));
}

fn identity_headers(headers: &Headers) -> BTreeMap<String, String> {
    const ALLOWED: &[&str] = &[
        "accept-language",
        "sec-ch-ua",
        "sec-ch-ua-arch",
        "sec-ch-ua-bitness",
        "sec-ch-ua-form-factors",
        "sec-ch-ua-full-version",
        "sec-ch-ua-full-version-list",
        "sec-ch-ua-mobile",
        "sec-ch-ua-model",
        "sec-ch-ua-platform",
        "sec-ch-ua-platform-version",
        "sec-ch-ua-wow64",
        "user-agent",
    ];
    headers
        .inner()
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, _)| {
            ALLOWED
                .iter()
                .any(|allowed| name.eq_ignore_ascii_case(allowed))
        })
        .filter_map(|(name, value)| {
            value
                .as_str()
                .map(|value| (name.to_ascii_lowercase(), bounded(value)))
        })
        .take(ALLOWED.len())
        .collect()
}

fn push_bounded<T>(values: &mut Vec<T>, value: T) {
    if values.len() < MAX_COLLECTION_ITEMS {
        values.push(value);
    }
}

/// Broad, intentionally non-exhaustive families of loading failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum FailureCategory {
    /// The request was cancelled before completion.
    Cancelled,
    /// Chromium blocked the request due to browser policy, content blocking, or CORS.
    BrowserPolicy,
    /// DNS/name resolution failed.
    Dns,
    /// A proxy or CONNECT tunnel failed.
    Proxy,
    /// The peer connection was refused, reset, closed, or otherwise unavailable.
    Connection,
    /// TLS, certificate, or secure-channel negotiation failed.
    Tls,
    /// HTTP/2 framing or protocol negotiation failed.
    Http2,
    /// HTTP/3, QUIC, or QUIC transport negotiation failed.
    Http3,
    /// The request exceeded a network timeout.
    Timeout,
    /// Chromium reported a failure that does not fit a known family.
    Unknown,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Cancelled => "cancelled",
            Self::BrowserPolicy => "browser-policy",
            Self::Dns => "dns",
            Self::Proxy => "proxy",
            Self::Connection => "connection",
            Self::Tls => "tls",
            Self::Http2 => "http2",
            Self::Http3 => "http3",
            Self::Timeout => "timeout",
            Self::Unknown => "unknown",
        };
        formatter.write_str(name)
    }
}

/// Classifies a Chromium `Network.loadingFailed` error without suppressing its name.
pub fn classify_failure(
    error_text: &str,
    cancelled: bool,
    blocked_reason: Option<&str>,
    cors_error: bool,
) -> FailureCategory {
    if cancelled {
        return FailureCategory::Cancelled;
    }
    if blocked_reason.is_some() || cors_error {
        return FailureCategory::BrowserPolicy;
    }

    let error = error_text.to_ascii_uppercase();
    let error = error.strip_prefix("NET::").unwrap_or(&error);
    if error.contains("NAME_NOT_RESOLVED")
        || error.contains("DNS_")
        || error.contains("ADDRESS_UNREACHABLE")
    {
        FailureCategory::Dns
    } else if error.contains("PROXY") || error.contains("TUNNEL") {
        FailureCategory::Proxy
    } else if error.contains("CERT_")
        || error.contains("SSL")
        || error.contains("TLS")
        || error.contains("BAD_SECURITY")
    {
        FailureCategory::Tls
    } else if error.contains("HTTP2") || error.contains("SPDY") {
        FailureCategory::Http2
    } else if error.contains("QUIC") || error.contains("HTTP3") {
        FailureCategory::Http3
    } else if error.contains("TIMED_OUT") || error.contains("TIMEOUT") {
        FailureCategory::Timeout
    } else if error.contains("CONNECTION_")
        || error.contains("CONNECTION_REFUSED")
        || error.contains("CONNECTION_RESET")
        || error.contains("CONNECTION_CLOSED")
        || error.contains("NETWORK_CHANGED")
    {
        FailureCategory::Connection
    } else {
        FailureCategory::Unknown
    }
}

/// Removes transport prefixes and detail text while retaining a useful error name.
pub fn sanitize_error_name(error_text: &str) -> String {
    let trimmed = error_text.trim();
    let name = trimmed
        .strip_prefix("net::")
        .or_else(|| trimmed.strip_prefix("NET::"))
        .unwrap_or(trimmed);
    name.split_whitespace()
        .next()
        .unwrap_or("UNKNOWN")
        .to_string()
}

/// Adds one failure to a category-count map.
pub fn increment_failure_count(counts: &mut BTreeMap<String, usize>, category: FailureCategory) {
    *counts.entry(category.to_string()).or_default() += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_known_families() {
        assert_eq!(
            classify_failure("net::ERR_NAME_NOT_RESOLVED", false, None, false),
            FailureCategory::Dns
        );
        assert_eq!(
            classify_failure("net::ERR_PROXY_CONNECTION_FAILED", false, None, false),
            FailureCategory::Proxy
        );
        assert_eq!(
            classify_failure("net::ERR_CERT_AUTHORITY_INVALID", false, None, false),
            FailureCategory::Tls
        );
        assert_eq!(
            classify_failure("net::ERR_HTTP2_PROTOCOL_ERROR", false, None, false),
            FailureCategory::Http2
        );
        assert_eq!(
            classify_failure("net::ERR_QUIC_PROTOCOL_ERROR", false, None, false),
            FailureCategory::Http3
        );
        assert_eq!(
            classify_failure("net::ERR_TIMED_OUT", false, None, false),
            FailureCategory::Timeout
        );
    }

    #[test]
    fn policy_and_cancellation_take_precedence() {
        assert_eq!(
            classify_failure("net::ERR_FAILED", true, None, false),
            FailureCategory::Cancelled
        );
        assert_eq!(
            classify_failure("net::ERR_FAILED", false, Some("blockedByClient"), false),
            FailureCategory::BrowserPolicy
        );
        assert_eq!(
            classify_failure("net::ERR_FAILED", false, None, true),
            FailureCategory::BrowserPolicy
        );
    }

    #[test]
    fn unknowns_remain_observable() {
        assert_eq!(
            classify_failure("net::ERR_NEW_CHROMIUM_ERROR", false, None, false),
            FailureCategory::Unknown
        );
        assert_eq!(
            sanitize_error_name("net::ERR_FAILED details omitted"),
            "ERR_FAILED"
        );
    }

    #[test]
    fn audit_correlates_out_of_order_events_and_redacts_secrets() {
        let extra: EventRequestWillBeSentExtraInfo = serde_json::from_value(json!({
            "requestId": "request-1",
            "associatedCookies": [],
            "headers": {
                "Authorization": "Bearer secret",
                "Cookie": "session=secret",
                "Accept-Language": "en-US,en;q=0.9",
                "Sec-CH-UA-Platform": "\"Windows\"",
                "User-Agent": "Chrome"
            },
            "connectTiming": { "requestTime": 1.0 }
        }))
        .unwrap();
        let sent: EventRequestWillBeSent = serde_json::from_value(json!({
            "requestId": "request-1",
            "loaderId": "loader-1",
            "documentURL": "https://example.com/page?document_secret=yes",
            "request": {
                "url": "https://user:password@example.com/api?token=secret#fragment",
                "method": "GET",
                "headers": { "Cookie": "secret" },
                "initialPriority": "High",
                "referrerPolicy": "strict-origin-when-cross-origin"
            },
            "timestamp": 1.0,
            "wallTime": 1.0,
            "initiator": {
                "type": "parser",
                "url": "https://example.com/source/path?secret=yes"
            },
            "redirectHasExtraInfo": false,
            "type": "Document",
            "frameId": "main"
        }))
        .unwrap();
        let finished: EventLoadingFinished = serde_json::from_value(json!({
            "requestId": "request-1",
            "timestamp": 2.0,
            "encodedDataLength": 321.0
        }))
        .unwrap();

        let mut audit = NetworkAudit::new(10);
        audit.set_main_frame_id("main");
        audit.observe_request_extra_info(&extra);
        audit.observe_request_will_be_sent(&sent);
        audit.observe_loading_finished(&finished);

        let request = audit.get("request-1").unwrap();
        assert_eq!(request.url.as_deref(), Some("https://example.com/api"));
        assert_eq!(
            request.initiator_origin.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(request.scope, ResourceScope::MainDocument);
        assert_eq!(request.encoded_data_length, Some(321.0));
        assert!(request.finished);
        let headers = &request.request_identity_headers[0];
        assert_eq!(headers["accept-language"], "en-US,en;q=0.9");
        assert!(!headers.contains_key("authorization"));
        assert!(!headers.contains_key("cookie"));
        assert!(!format!("{request:?}").contains("secret"));
    }

    #[test]
    fn audit_is_bounded_and_summarizes_failures() {
        let mut audit = NetworkAudit::new(1);
        audit.entry("old");
        audit.entry("new").failure_category = Some(FailureCategory::Dns);
        audit.entry("new").finished = true;

        assert!(audit.get("old").is_none());
        let summary = audit.summary();
        assert_eq!(summary.requests, 1);
        assert_eq!(summary.finished, 1);
        assert_eq!(summary.failures["dns"], 1);
    }

    #[test]
    fn validates_transmitted_identity_without_echoing_values() {
        let identity = crate::PlatformProfile::Linux.profile().navigator;
        let hints = identity.client_hints.as_ref().unwrap();
        let brands = hints
            .brands
            .iter()
            .map(|brand| format!("\"{}\";v=\"{}\"", brand.brand, brand.version))
            .collect::<Vec<_>>()
            .join(", ");
        let mut request = NetworkRequestAudit::placeholder("identity");
        request.request_identity_headers.push(BTreeMap::from([
            ("user-agent".into(), identity.user_agent.clone()),
            ("accept-language".into(), "en-US,en;q=0.9".into()),
            ("sec-ch-ua".into(), brands),
            ("sec-ch-ua-mobile".into(), "?0".into()),
            ("sec-ch-ua-platform".into(), "\"Linux\"".into()),
        ]));
        assert!(validate_request_identity(&request, &identity).is_empty());

        request.request_identity_headers[0].insert("sec-ch-ua-platform".into(), "\"Other\"".into());
        let findings = validate_request_identity(&request, &identity);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "request-client-hint-platform-mismatch");
        assert!(!format!("{findings:?}").contains("Other"));

        request.request_identity_headers[0].insert("sec-ch-ua-platform".into(), "\"Linux\"".into());
        request.request_identity_headers[0].insert("sec-ch-ua-arch".into(), "\"arm\"".into());
        let findings = validate_request_identity(&request, &identity);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].code,
            "request-client-hint-architecture-mismatch"
        );
    }

    #[test]
    fn validates_accept_ch_only_for_subsequent_same_origin_requests() {
        let mut audit = NetworkAudit::new(10);
        audit.observe_accept_ch(
            "https://example.com/first",
            &Headers::new(json!({
                "Accept-CH": "Sec-CH-UA-Arch, Sec-CH-UA-Full-Version-List"
            })),
        );

        let sent = request_event("subsequent", "https://example.com/second");
        audit.observe_request_will_be_sent(&sent);
        let extra = request_extra_info(
            "subsequent",
            json!({
                "Sec-CH-UA-Arch": "\"x86\"",
                "Sec-CH-UA-Full-Version-List": "\"Chromium\";v=\"150.0.0.0\""
            }),
        );
        audit.observe_request_extra_info(&extra);

        assert!(validate_client_hint_negotiation(&audit).is_empty());
        let request = audit.get("subsequent").unwrap();
        assert!(
            request
                .eligible_high_entropy_hints
                .contains("sec-ch-ua-arch")
        );
    }

    #[test]
    fn warns_for_unexplained_hints_and_honors_accept_ch_clear() {
        let mut audit = NetworkAudit::new(10);
        audit.observe_accept_ch(
            "https://example.com/",
            &Headers::new(json!({ "Accept-CH": "Sec-CH-UA-Arch" })),
        );
        audit.observe_accept_ch(
            "https://example.com/clear",
            &Headers::new(json!({ "Accept-CH": "" })),
        );
        let sent = request_event("cleared", "https://example.com/next");
        audit.observe_request_will_be_sent(&sent);
        let extra = request_extra_info("cleared", json!({ "Sec-CH-UA-Arch": "\"x86\"" }));
        audit.observe_request_extra_info(&extra);

        let findings = validate_client_hint_negotiation(&audit);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].code,
            "high-entropy-client-hint-without-observed-opt-in"
        );
    }

    fn request_event(request_id: &str, request_url: &str) -> EventRequestWillBeSent {
        serde_json::from_value(json!({
            "requestId": request_id,
            "loaderId": "loader",
            "documentURL": request_url,
            "request": {
                "url": request_url,
                "method": "GET",
                "headers": {},
                "initialPriority": "High",
                "referrerPolicy": "strict-origin-when-cross-origin"
            },
            "timestamp": 1.0,
            "wallTime": 1.0,
            "initiator": { "type": "other" },
            "redirectHasExtraInfo": false,
            "type": "Document",
            "frameId": "main"
        }))
        .unwrap()
    }

    fn request_extra_info(
        request_id: &str,
        headers: serde_json::Value,
    ) -> EventRequestWillBeSentExtraInfo {
        serde_json::from_value(json!({
            "requestId": request_id,
            "associatedCookies": [],
            "headers": headers,
            "connectTiming": { "requestTime": 1.0 }
        }))
        .unwrap()
    }
}
