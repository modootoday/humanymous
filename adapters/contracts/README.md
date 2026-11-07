# Adapter wire corpus

Experimental `humanymous.adapter/0.1`; internal deployment contract, independent
of public standards Draft v0. JSON Schema checks structure only. Implementations
must also enforce duplicate keys, UTF-8 validity, byte limits, body recomputation,
authority/framing, capabilities, authenticated peer scope, revision, deadline,
replay and one-request result binding described by adapter-service-contract SoT.

Schema validation alone never establishes authenticated provenance or an ALLOW.
All currently executable Node authority fixtures are mock conformance fixtures.
No real Rust service or full isolated Docker acceptance is claimed by this corpus.

Rust continuation uses the same byte/body/decision framing and a provisioned
producer profile. Node identity accepts only node-raw/fetch-normalized; nginx
identity accepts only nginx-raw/nginx-normalized. nginx modes report no original
TLS/H2, peer or connection claim. The integrity crate's parse_with_profile is
the semantic authority; choosing a producer from untrusted wire is forbidden.
The existing request JSON Schema remains the Node structural corpus until the
new nginx mode is independently exercised; do not advertise schema-only support.
