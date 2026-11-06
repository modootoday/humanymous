# Adapter wire corpus

Experimental `humanymous.adapter/0.1`; internal deployment contract, independent
of public standards Draft v0. JSON Schema checks structure only. Implementations
must also enforce duplicate keys, UTF-8 validity, byte limits, body recomputation,
authority/framing, capabilities, authenticated peer scope, revision, deadline,
replay and one-request result binding described by adapter-service-contract SoT.

Schema validation alone never establishes authenticated provenance or an ALLOW.
All currently executable Node authority fixtures are mock conformance fixtures.
No real Rust service or full isolated Docker acceptance is claimed by this corpus.
