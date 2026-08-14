# Site evaluation matrix

This ledger records reproducible browser evaluations. It does not promise that
a site can always be accessed: protection rules, browser builds, IP reputation,
profiles, and site behavior can change independently of this crate.

Only evaluate origins you own or are authorized to test. Use a stable target
label instead of a sensitive URL, and never record credentials, cookies, proxy
secrets, query strings, or captured personal data.

## Status vocabulary

| Status | Meaning |
| --- | --- |
| `verified` | The expected destination content loaded and remained usable for every recorded attempt. |
| `blocked` | The expected content did not load because a challenge or access-denied response remained. |
| `inconclusive` | The run failed for an unrelated or ambiguous reason, such as DNS, proxy, timeout, or fixture drift. |
| `diagnostic-only` | Fingerprint observations were collected, but access to protected destination content was not tested. |
| `stale` | The evidence predates the current Chrome major, profile identity, or a material patch change. |

An HTTP `200` alone is not `verified`: a challenge page can also return `200`.
Verification requires an application-specific content assertion, no challenge
loop, and preserved session state on a follow-up navigation.

## Current results

| Target | Protection or purpose | Last run | Commit | Runtime and host | Mode | Attempts | Status | Evidence |
| --- | --- | --- | --- | --- | --- | ---: | --- | --- |
| Local network fixture | Redirect, cache, concurrency, `Accept-CH`, and `503` observation | 2026-08-14 | `c7f0a47` | Chrome 150.0.7871.128, Linux | Native | 1 | `verified` | 8/8 requests finished, one cache hit, expected redirect and Client Hint opt-in |
| Local network fixture | Same fixture with the Linux stealth profile | 2026-08-14 | `c7f0a47` | Chrome 150.0.7871.128, Linux | Configured | 1 | `verified` | Compatible runtime; 8/8 requests finished with the same lifecycle summary |
| CreepJS | Browser fingerprint diagnostics | Not recorded | — | — | Configured | — | `diagnostic-only` | Browser diagnostics exist under `tests/bypass/`; they are not a protected-site acceptance test |
| SannySoft | Browser fingerprint diagnostics | Not recorded | — | — | Configured | — | `diagnostic-only` | Screenshot example exists; it is not a protected-site acceptance test |
| Authorized Anubis target | End-to-end challenge and destination-content test | Not run | — | — | — | 0 | `inconclusive` | Add a result only after testing an origin the operator is authorized to evaluate |
| Axios home | Public content assertion | 2026-08-14 | `c7f0a47` | Chromium 151.0.7922.137, Linux host with Windows profile | Headless, fresh container | 1 | `inconclusive` | Content assertion passed; HTTP 200; 261456 HTML bytes; repeat after runtime/profile alignment |
| Zillow home | Public content assertion | 2026-08-14 | `c7f0a47` | Chromium 151.0.7922.137, Linux host with Windows profile | Headless, fresh container | 1 | `inconclusive` | Content assertion passed; HTTP 200; 408166 HTML bytes; repeat after runtime/profile alignment |
| Ticketmaster home | Public content assertion | 2026-08-14 | `c7f0a47` | Chromium 151.0.7922.137, Linux host with Windows profile | Headless, fresh container | 1 | `inconclusive` | Content assertion passed; HTTP 200; 486418 HTML bytes; repeat after runtime/profile alignment |
| Reddit home | JavaScript challenge observation | 2026-08-14 | `c7f0a47` | Chromium 151.0.7922.137, Linux host with Windows profile | Headless, fresh container | 1 | `blocked` | HTTP 200 challenge URL remained with `js_challenge=1`; challenge detector fired |
| G2 home | Access-denied observation | 2026-08-14 | `c7f0a47` | Chromium 151.0.7922.137, Linux host with Windows profile | Headless, fresh container | 1 | `blocked` | HTTP 403; 2611 HTML bytes; challenge detector fired |
| Axios home | Aligned public content assertion | 2026-08-14 | working tree after `c7f0a47` | Chrome 150.0.7871.128, Linux host and profile | Headless, fresh process | 2 | `inconclusive` | 2/2 content assertions passed with HTTP 200 and no blocking challenge evidence |
| Zillow home | Aligned public content assertion | 2026-08-14 | working tree after `c7f0a47` | Chrome 150.0.7871.128, Linux host and profile | Headless, fresh process | 2 | `inconclusive` | 2/2 content assertions passed with HTTP 200 and no challenge evidence |
| Ticketmaster home | Aligned public content assertion | 2026-08-14 | working tree after `c7f0a47` | Chrome 150.0.7871.128, Linux host and profile | Headless, fresh process | 2 | `inconclusive` | 2/2 content assertions passed; dormant CAPTCHA element did not replace destination content |
| Reddit home | Aligned JavaScript challenge observation | 2026-08-14 | working tree after `c7f0a47` | Chrome 150.0.7871.128, Linux host and profile | Headless, fresh process | 2 | `inconclusive` | One run remained on `js_challenge=1`; one later run loaded destination content, so the outcome is unstable |
| G2 home | Aligned access-denied observation | 2026-08-14 | working tree after `c7f0a47` | Chrome 150.0.7871.128, Linux host and profile | Headless, fresh process | 2 | `blocked` | 2/2 runs returned HTTP 403 with the destination assertion absent |

## Recording a run

Record one row per materially different environment. At minimum capture:

- UTC date and tested commit;
- exact Chrome product version and host operating system;
- built-in profile and whether Chrome was headless or headful;
- direct or proxy connection class, without recording proxy credentials;
- fresh or persistent profile and the number of attempts;
- final status and an application-specific content assertion;
- challenge status or redirect loop, if observed;
- a redacted `NetworkAudit` summary and relevant diagnostic artifact paths.

Use at least three fresh-profile attempts and three persistent-profile attempts
before describing a protected target as `verified`. A single success should be
recorded as `inconclusive` until repeated. Mark a row `stale` after a Chrome
major change, profile identity update, or material target-side change.

## Evaluation procedure

1. Confirm the runtime/profile major versions are compatible.
2. Capture an unconfigured Chrome baseline from the same host and network.
3. Run the configured browser without request interception.
4. Assert expected destination content rather than relying on the status code.
5. Navigate again in the same profile and assert session continuity.
6. Save only redacted audit summaries and screenshots without sensitive data.
7. Repeat with fresh and persistent profiles, then update the matrix.

Fingerprint diagnostics are supporting evidence, not proof of access. Likewise,
successful access does not prove that every fingerprint surface is consistent.
Keep both forms of evidence separate.
