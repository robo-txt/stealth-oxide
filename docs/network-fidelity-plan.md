# Chrome-native network fidelity plan

## Objective

Make stealth-oxide network behavior resemble the installed Chrome build as
closely as possible while keeping Chrome responsible for request scheduling,
connection reuse, redirects, caching, cookies, service workers, protocol
negotiation, and native transport retries.

The default path must configure and observe Chrome. It must not replace Chrome
with a Rust HTTP client, pause every request, reconstruct native headers, or
replay requests behind the application's back.

## Native-network contract

- Do not enable Fetch interception in the default stealth path.
- Do not install blanket extra HTTP headers.
- Do not reconstruct request headers from CDP event maps.
- Do not serialize or artificially batch browser requests.
- Do not implement Rust-side subresource fetching.
- Let Chrome generate cookies, `Sec-Fetch-*`, `Priority`, CORS, preflight,
  WebSocket, pseudo-header, compression, and cache-related behavior.
- Preserve Chrome's scheduler, connection pool, HTTP/2 and HTTP/3 multiplexing,
  Alt-Svc behavior, service workers, and browser cache.
- Keep request mutation through the interceptor feature explicit and outside
  the default stealth preset.

Chrome does not literally send every discovered resource simultaneously. Its
native scheduler determines concurrency and priority from parser discovery,
preload hints, resource type, connection limits, cache state, service workers,
and the negotiated HTTP protocol. Network fidelity requires leaving that
scheduler in control.

## Retry ownership

Chrome remains responsible for:

- connection establishment and connection reuse;
- safe native transport retries;
- DNS and address fallback;
- QUIC/HTTP/3 and HTTP/2 fallback behavior;
- redirects;
- authentication challenges;
- cache validation;
- service-worker handling;
- Alt-Svc processing.

An optional stealth-oxide navigation retry policy may handle application-level
responses with these constraints:

- Retry only explicitly eligible navigation failures and status codes.
- Honor `Retry-After` for eligible `429` and `503` responses.
- Use bounded exponential backoff with jitter when `Retry-After` is absent.
- Preserve the same browser process, user-data profile, cookies, cache, and
  session state.
- Do not automatically replay unsafe methods such as `POST`, `PATCH`, or
  `DELETE`.
- Permit unsafe-method replay only through explicit caller authorization and an
  application-provided idempotency guarantee.
- Do not automatically retry `403` or replace the browser profile.
- Record the retry reason, attempt number, delay source, and terminal result
  without retaining sensitive URLs or headers.

## Implementation stages

### 1. Define and test the native-network contract

- Document which components own scheduling, redirects, retries, cookies,
  caching, and proxy behavior.
- Add tests proving the default profile does not enable Fetch interception or
  blanket extra headers.
- Keep the existing interceptor feature opt-in.
- Distinguish Chrome transport retries from application-level HTTP retries in
  public types and documentation.

### 2. Build a controlled Chrome baseline harness

Create a controlled HTTPS test origin and compare unmodified Chrome 151 with
each stealth-oxide profile across:

- first navigation;
- a follow-up navigation after `Accept-CH`;
- parser-discovered and dynamically created subresources;
- same-origin and cross-origin requests;
- redirects;
- dedicated, shared, and service-worker requests;
- cached and revalidated resources;
- HTTP/1.1, HTTP/2, and HTTP/3 where available;
- direct and proxied connections.

Server-side capture is required for wire details CDP cannot represent, such as
HTTP/1 header order, HTTP/2 pseudo-header order and settings, HTTP/3 behavior,
and the TLS ClientHello.

### 3. Prefer native Chrome language configuration

- Support persistent user-data profiles with Chrome's preferred-language
  settings configured before launch.
- Align application locale, `intl.accept_languages`, `navigator.language`,
  `navigator.languages`, Intl locale, and the HTTP `Accept-Language` header.
- Let Chrome generate quality weights such as `en-US,en;q=0.9`.
- Retain CDP `acceptLanguage` as an explicit target-scoped fallback for
  temporary profiles.
- Detect and report when the CDP fallback fails to reach worker navigators.

### 4. Coordinate every browser target

Use target auto-attachment to cover:

- pages and popups;
- out-of-process iframes;
- dedicated workers;
- shared workers;
- service workers.

Apply UA, Client Hint metadata, platform, locale, timezone, and fallback
language configuration before target execution, then resume the target.
Avoid pausing ordinary network requests.

Current limitation: Chromiumoxide 0.9.1 cannot publicly route commands to an
arbitrary flattened CDP session. Chrome rejects its deprecated nested-session
path for service-worker targets, so the coordinator deliberately excludes
service workers rather than pausing and breaking them. Service-worker identity
remains a reported coverage gap until the browser client exposes safe session
routing.

### 5. Validate UA Client Hints against the runtime

- Compare the configured UA and metadata with the installed Chrome version.
- Validate brand ordering, GREASE brand/value, full-version lists, platform,
  platform version, architecture, bitness, model, mobile state, WOW64, and form
  factors.
- Ensure low-entropy hints appear naturally.
- Ensure high-entropy hints are sent only after the origin opts in through
  `Accept-CH` and according to Chrome's normal rules.
- Do not add `Sec-CH-UA-*` headers through extra-header APIs.
- Fail validation or warn when a static profile does not match the runtime.

### 6. Add a passive `NetworkAudit`

Observe without pausing or modifying requests. Correlate:

- `Network.requestWillBeSent`;
- `Network.requestWillBeSentExtraInfo`;
- `Network.responseReceived`;
- `Network.responseReceivedExtraInfo`;
- `Network.loadingFinished`;
- `Network.loadingFailed`;
- redirect request IDs and loader IDs;
- frame and target ownership.

Report:

- main-document, subresource, iframe, and worker scope;
- exposed request headers without claiming wire order;
- UA, Client Hint, language, and platform consistency;
- Client Hint opt-in and subsequent delivery;
- negotiated response protocol;
- cache and service-worker involvement;
- redirect chains;
- categorized DNS, proxy, connection, TLS, HTTP/2, HTTP/3, timeout, policy,
  cancellation, and unknown failures;
- target coverage gaps.

Keep observations bounded and redact credentials, cookies, query strings,
request bodies, and sensitive headers.

### 7. Add opt-in navigation retries

- Create typed retry eligibility and terminal-result types.
- Parse both delta-seconds and HTTP-date forms of `Retry-After`.
- Add caller-configured maximum attempts, maximum elapsed time, base delay,
  maximum delay, and jitter.
- Retry eligible top-level navigations without recreating the browser or
  profile.
- Do not retry subresources independently of Chrome.
- Do not infer that a failed navigation is safe to replay when its method or
  body is unknown.
- Expose retry observations without taking ownership of site-specific recovery
  workflows.

### 8. Add proxy and transport diagnostics

Distinguish:

- direct connections;
- SOCKS proxies;
- HTTP CONNECT tunnels that preserve Chrome's end-to-end TLS;
- TLS-terminating proxies that replace the browser's transport signature.

Measure at the controlled origin:

- TLS ClientHello and negotiated ALPN;
- HTTP protocol;
- HTTP/2 settings and pseudo-header order;
- HTTP/3 negotiation;
- request header wire order and casing where applicable;
- connection reuse;
- DNS ownership and address family;
- proxy authentication boundaries.

Do not claim Chrome-native transport fidelity through a TLS-terminating proxy.

### 9. Update examples and documentation

- Make the native-network path the default in examples.
- Explain that Chrome controls concurrency and priority.
- Explain the difference between transport retries and navigation retries.
- Document the limitations of CDP header maps and request interception.
- Show persistent-profile language configuration.
- Show passive auditing separately from mutation.
- Keep proxies and credentials under application control.

### 10. Verification and fidelity matrix

Run:

- unit tests for configuration, validation, redaction, correlation, and retry
  eligibility;
- real-browser tests for pages, iframes, popups, and all worker types;
- controlled-origin comparison against unmodified Chrome 151;
- first-request and post-`Accept-CH` comparisons;
- direct, CONNECT, SOCKS, and TLS-terminating proxy diagnostics;
- HTTP/1.1, HTTP/2, and HTTP/3 tests where supported;
- CreepJS and existing browser regressions.

Publish a matrix that separates:

- verified Chrome-native behavior;
- behavior configured through CDP;
- modeled but not native behavior;
- behavior observable only at the server or wire layer;
- unsupported or environment-dependent behavior.

Protected-site outcomes and fingerprint-only diagnostics are tracked in the
[site evaluation matrix](site-evaluation-matrix.md). Entries must identify the
tested commit and runtime, use repeated application-content assertions, and be
marked stale after material browser, profile, or target changes.

The evaluation workflow should enforce same-major runtime/profile compatibility,
use a host-matching profile, require application-owned destination assertions,
and retain only bounded, redacted `NetworkAudit` evidence for authorized test
origins.

## Initial delivery order

1. Native-network contract and regression tests.
2. Passive network audit and controlled-origin fixture.
3. Target coordinator for worker and popup consistency.
4. Runtime UA Client Hint validation.
5. Persistent-profile language configuration.
6. Safe navigation retry policy.
7. Proxy and wire-level diagnostics.
8. Documentation and fidelity matrix.

## Acceptance criteria

- Default stealth operation does not pause or reconstruct ordinary requests.
- Chrome remains responsible for resource concurrency and priority.
- Language, UA, Client Hints, locale, and platform agree across page and worker
  targets.
- High-entropy Client Hints follow normal `Accept-CH` negotiation.
- Redirect, cache, service-worker, connection reuse, and protocol behavior are
  preserved.
- Retry behavior is bounded, observable, and safe by default.
- Unsafe methods and `403` responses are not automatically retried.
- Direct and tunneled connections retain Chrome's native TLS and HTTP stack.
- Documentation never claims CDP event header order is wire order.
