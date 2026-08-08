//! Read-only classification helpers for network/frame topology diagnostics.

/// Evidence used to classify a network resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceScope {
    /// The document loaded in the inspected page's main frame.
    MainDocument,
    /// A resource requested by the inspected page's main frame.
    MainSubresource,
    /// A document loaded in a child frame.
    ChildDocument,
    /// A resource requested by a child frame.
    ChildSubresource,
    /// No frame evidence was available, including worker-originated requests.
    Unknown,
}

/// Target coverage state for a topology observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Coverage {
    /// The inspected page target supplied the event.
    Observed,
    /// The event may belong to an unattached worker or OOPIF target.
    UnobservedTarget,
    /// The event was missing the frame correlation fields.
    Incomplete,
}

/// Classifies a resource using only frame presence, main-frame identity, and type.
pub fn classify_resource(
    frame_id: Option<&str>,
    main_frame_id: Option<&str>,
    resource_type: &str,
) -> ResourceScope {
    let Some(frame_id) = frame_id else {
        return ResourceScope::Unknown;
    };
    let is_document = resource_type.eq_ignore_ascii_case("document");
    match main_frame_id {
        Some(main) if frame_id == main && is_document => ResourceScope::MainDocument,
        Some(main) if frame_id == main => ResourceScope::MainSubresource,
        Some(_) if is_document => ResourceScope::ChildDocument,
        Some(_) => ResourceScope::ChildSubresource,
        None => ResourceScope::Unknown,
    }
}

/// Sanitizes an initiator URL down to an origin, never retaining path/query data.
pub fn sanitize_initiator_origin(value: Option<&str>) -> Option<String> {
    let value = value?;
    let parsed = url::Url::parse(value).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Some(format!("<{}-origin>", parsed.scheme()));
    }
    let host = parsed.host_str()?;
    Some(match parsed.port() {
        Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
        None => format!("{}://{}", parsed.scheme(), host),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_main_and_child_resources() {
        assert_eq!(
            classify_resource(Some("main"), Some("main"), "Document"),
            ResourceScope::MainDocument
        );
        assert_eq!(
            classify_resource(Some("main"), Some("main"), "Script"),
            ResourceScope::MainSubresource
        );
        assert_eq!(
            classify_resource(Some("child"), Some("main"), "Document"),
            ResourceScope::ChildDocument
        );
        assert_eq!(
            classify_resource(Some("child"), Some("main"), "Image"),
            ResourceScope::ChildSubresource
        );
        assert_eq!(
            classify_resource(None, Some("main"), "Script"),
            ResourceScope::Unknown
        );
    }

    #[test]
    fn sanitizes_initiator_to_origin() {
        assert_eq!(
            sanitize_initiator_origin(Some("https://example.com/path?q=secret")),
            Some("https://example.com".into())
        );
        assert_eq!(
            sanitize_initiator_origin(Some("data:text/plain,secret")),
            Some("<data-origin>".into())
        );
    }
}
