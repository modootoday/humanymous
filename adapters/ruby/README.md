# Experimental Ruby native adapter

The Rust extension exposes `Humanymous.wrap(app,options_json)` as a Rack callable.
No handwritten Ruby loader, gemspec, extconf or eval implementation is included.
The candidate distribution name is unverified. No gem/runtime tuple or publisher
is qualified, and no publish action is authorized.

Root registration must precede every parser/effect. Rack env requires REQUEST_URI,
CONTENT_LENGTH, HTTP_HOST and a server-enforced input deadline. CGI header loss
is explicit; raw TLS/H2/casing/order and browser capture are unavailable.
Hijack, upgrade and request streaming reject before protected application calls.
Private config is delivered by a trusted supervisor; no public management mount.

The intended first precompiled gem contains `lib/humanymous_ruby.<DLEXT>` directly,
one exact Ruby minor ABI and strict Ruby version requirement. A platform tag alone
does not qualify Ruby ABI or select a compatible implementation. MultiABI loading
and source gem build hooks remain unresolved. Upstream RubyGems generates archive
metadata from Specification/Package API calls made in Rust; clean install/require,
Rack/Sinatra/Rails, GC compaction, GVL scheduling and authenticated IPC remain
NOT_RUN. JRuby/TruffleRuby/Ractors are unsupported.
