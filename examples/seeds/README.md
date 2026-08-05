# Generated browser profiles

The `seeded_profile` example creates a unique Chromium user-data directory for
each run. It never reads a personal Chrome profile. Unless `--keep-profile` is
provided, the generated directory is removed after Chromium closes.

With no `--seed` arguments, the example creates three harmless seed documents
scoped to the requested URL:

1. An HTTP-only test cookie.
2. Two local-storage preferences.
3. One IndexedDB test record.

These values demonstrate reproducible application state. They do not create a
human identity or prove that the browser has a genuine browsing history.

## Run the defaults

```bash
cargo run --features seeding --example seeded_profile -- --url https://example.com
```

The equivalent container command is:

```bash
docker compose run --rm stealth-oxide \
  cargo run --features seeding --example seeded_profile -- \
  --url https://example.com
```

Use `--keep-profile` to retain the generated directory for inspection. Never
commit a generated directory because it can contain cookies and site data.

## Custom seed documents

A seed document is JSON with optional `cookies` and `origins` arrays. See
[`storage.json`](storage.json) for a complete
example.

Cookies require a full `http` or `https` URL. Storage requires an exact origin
containing only its scheme and authority:

```json
{
  "cookies": [
    {
      "name": "my_test_cookie",
      "value": "test-value",
      "url": "https://app.example.test/",
      "secure": true,
      "httpOnly": true
    }
  ],
  "origins": [
    {
      "origin": "https://app.example.test",
      "localStorage": {
        "theme": "dark"
      },
      "indexedDb": [
        {
          "database": "test-state",
          "store": "settings",
          "key": "onboarding",
          "value": { "complete": true }
        }
      ]
    }
  ]
}
```

Only seed origins and accounts you own or are authorized to test. Do not copy
authentication, clearance, or tracking cookies from third parties.

## Merge more than one seed

Repeat `--seed` in the desired order:

```bash
cargo run --features seeding --example seeded_profile -- \
  --url https://app.example.test \
  --seed examples/seeds/base.json \
  --seed examples/seeds/preferences.json \
  --seed examples/seeds/test-session.json
```

All cookies and origin records are installed. When local-storage or IndexedDB
keys overlap, later operations replace earlier values using the browser's normal
storage APIs.

## CreepJS diagnostic

For defensive comparison testing, run the example with and without seeds and
compare the reported ratings:

```bash
docker compose run --rm stealth-oxide \
  cargo run --features seeding --example seeded_profile -- \
  --url https://abrahamjuliot.github.io/creepjs/ \
  --wait 15
```

Add `--no-seeds` to the same command for the clean-profile baseline.

CreepJS does not treat arbitrary storage as evidence of a naturally aged user.
Its headless, stealth, and consistency signals primarily examine browser APIs,
workers, rendering, and environment coherence. Seeded storage should therefore
be expected to have little or no effect on those ratings.

On 2026-08-05, the repository container produced this controlled comparison
with Chromium 151. Both runs used a newly generated profile and the same six
recommended patches:

| Profile | Headless | Like headless | Stealth |
| --- | ---: | ---: | ---: |
| Clean (`--no-seeds`) | 33% | 38% | 0% |
| Three default seeds | 33% | 38% | 0% |

The seeded run confirmed the HTTP-only cookie through CDP and confirmed both
localStorage and IndexedDB from the page. The unchanged ratings are evidence
that these storage values do not affect the CreepJS rating surfaces measured by
this example. They are still useful for reproducible application-state tests.

## Technical boundaries

- Cookies are installed through Chromiumoxide's browser-level CDP API before
  navigation.
- The storage script is registered before navigation but checks
  `location.origin`; it writes only after the target origin's execution context
  exists. Writing on `about:blank` would seed the wrong origin or throw a
  security error.
- `StealthConfig` remains responsible only for its typed CDP patches.
- The example does not manufacture Chrome history, edit Chromium databases, or
  conceal extension resources.
