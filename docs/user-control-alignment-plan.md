# User-control and alignment plan

This plan turns the recent network-fidelity work into explicit, reviewable
configuration choices. The library should make coherent browser identities easy
to select while keeping advanced controls opt-in and refusing contradictory
combinations.

## Current change set

The recent commits add bounded navigation retry decisions (`b8dc75e`), passive
network auditing and transmitted-identity validation (`f6ce139`), native
Client-Hint negotiation checks (`ce13365`), native Chrome language startup
(`5e8ec4f`), Chrome-150 profile alignment and local network baselines
(`c7f0a47`), and content-asserted site evaluation with expanded target coverage
(`87cfffa`). The working tree also contains the reproducible evaluation lab and
the pinned Chrome-150 container update.

## User-facing controls

1. Keep typed platform profiles as the primary API. A profile owns coupled
   values such as user agent, Client Hints, locale, timezone, screen, device,
   and rendering assumptions.
2. Expose patch selection through the existing `StealthConfig` builder. Users
   may choose `native`, `configured`, or `disabled` behavior per surface, but
   defaults remain conservative and deterministic.
3. Validate overrides before launch. Reject cross-platform, locale-language,
   touch, version, and Client-Hint contradictions unless an explicit permissive
   mode is selected.
4. Keep network observation separate from mutation. `NetworkAudit` reports
   bounded, redacted evidence; it must not silently rewrite headers, retry
   blocked requests, rotate profiles, or change identity mid-session.
5. Make retry policy explicit. Callers select method, attempt, elapsed-time,
   and server-delay budgets; the library returns a decision and never performs
   an automatic revisit.
6. Make runtime compatibility a launch gate. The selected profile major must
   match the discovered Chrome major; diagnostics should identify the mismatch
   and the selected profile.

## Alignment checks

- Every enabled patch has a typed profile source and a validation rule.
- Page, iframe, dedicated worker, and shared worker values agree where the
  browser exposes the same identity.
- Native startup values and post-launch CDP values are compared rather than
  assumed equivalent.
- Accept-CH negotiation is recorded as an origin-scoped state transition.
- Tests cover native, configured, disabled, and intentionally permissive modes.
- Evaluation records assert destination content and session continuity instead
  of treating status codes or fingerprint scores as access proof.

## Next implementation slices

- Add a public configuration report that lists selected profile, enabled
  patches, derived values, and validation warnings without exposing secrets.
- Add table-driven tests for each user override and its dependent fields.
- Add a documented proxy/transport configuration boundary that preserves one
  identity and one profile for the whole session.
- Keep all external-site evaluations under `dev/evals/`, with authorization,
  redaction, repeat counts, and environment metadata recorded separately from
  library correctness tests.
