# Network module 0.1

An optional Rust-owned HTTPS text reader, independent of BLE, data and security.
Install during `ferrule new reader_app --modules network` or later with
`ferrule add network`, then run `ferrule sync`.

## Boundaries

- Domain response/error types, an `HttpPort`, a fetch use case, reqwest infrastructure,
  a composition function and a worker-pool FRB API. Flutter owns input and display.
- Explicit manual GET only. HTTPS URLs up to 2048 bytes, without credentials or fragments.
  Query strings and arbitrary HTTPS hosts/ports are allowed. This is a client-side reader,
  not a server-side URL-fetching/SSRF sandbox. Do not put secrets in query strings.
- TLS certificate and hostname verification remain enabled. Redirects are rejected, including
  same-origin redirects. Cookies, authentication, user headers and automatic retry are absent.
- reqwest 0.12.28 with default features off and `blocking,rustls-tls` uses bundled WebPKI roots
  and ring fallback. No system proxy or native TLS. Device-installed private CAs and enterprise
  proxy settings are not adopted. Certificate-root/dependency updates require a reviewed rebuild.
  No global crypto provider is installed. A custom app installing a different global provider
  must validate its combined TLS configuration; the stock generated modules do not do so.
- Connect timeout 5 seconds, request deadline 10 seconds (including response-body transfer),
  and a separate blocking wait bound. Host DNS/OS scheduling can add overhead to wall-clock
  cancellation. This is not a hard real-time guarantee.
- Only successful 2xx responses are shown. A maximum of 65,537 bytes is read to enforce
  the 64 KiB cap even without Content-Length; oversized content is rejected, not truncated.
  Invalid UTF-8 is rejected. Compressed responses are not automatically decompressed.
- Response text is displayed as text, including HTML/Markdown. No WebView or HTML execution.
  Connection errors omit raw URLs/query strings and native diagnostics.
- The page starts no request on load and permits only one in-flight request. Navigation/disposal
  suppresses late UI updates but does not cancel the Rust request. No background service,
  stream subscriptions, persistent response cache, HTTP upload, WebSocket or QUIC support.
  Other pages/bridge callers can submit independent requests; this is not a global concurrency quota.
- Android adds INTERNET permission. No global ATS exceptions or Android cleartext opt-ins
  are added. Existing custom platform policies are preserved rather than silently tightened.

## Reproducible checks

```sh
sh scripts/check.sh
sh scripts/network-smoke.sh
# Repeat transport/host checks without regenerating a synchronized app:
sh scripts/network-smoke.sh --check /absolute/path/to/generated/combined_app
sh scripts/platform-smoke.sh /absolute/path/to/generated/combined_app both
```

The host smoke creates a disposable CA and separately signed localhost server certificate with
OpenSSL, then starts a private Python HTTPS fixture. Fixture keys never enter generated apps.
Only an explicitly invoked ignored Rust test adds that test CA to a local client. Production
constructor/bridge trust is unchanged. The fixture checks trusted success, rejection of an unknown
CA and wrong hostname, redirect/status rejection, announced/unannounced oversize responses,
invalid UTF-8, header stalls, body stalls and slow-drip deadlines. The real Dart/Rust bridge rejects
insecure URLs and the untrusted fixture. Fake-fetcher widgets check single-flight/retry and disposal.

The smoke first generates a network-only app and then checks the four optional modules together.
An optional absolute project argument adds network to an existing **disposable** BLE/data/security
fixture to reuse native build caches. Mobile compilation is separate from host transport checks.
Physical-device networking, captive portals, app suspension and network handover remain unverified.
No production endpoint or device is contacted by these tests.

Reference: [reqwest blocking client](https://docs.rs/reqwest/0.12.28/reqwest/blocking/struct.ClientBuilder.html)
and [request deadline](https://docs.rs/reqwest/0.12.28/reqwest/blocking/struct.RequestBuilder.html#method.timeout).
