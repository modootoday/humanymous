# Ruby integration frontier

Native lib uses explicit Init_humanymous_ruby export. Humanymous.wrap returns
a VM Rack object whose app/config live in Ruby ivars. No Ruby Value is stored
in Rust heap collections. Body copies use safe upstream Magnus bytes API.
IPC runs on a Rust-only worker; upstream thread_sleep yields GVL while awaiting
it. Watch sender Drop cancels the worker client future. Thread scheduling,
interrupt and GC.compact require actual hosted probes.

Feature package-tool exports Humanymous.build_native_gem(staging,metadata_json).
It calls upstream Gem::Specification/Requirement/Platform/Package methods,
requires cwd to equal explicit canonical staging and requires native
lib/humanymous_ruby.<runtime DLEXT>, README.md and LICENSE. Upstream RubyGems
generates metadata; no authored gemspec/extconf/Ruby loader or script string.
Single Ruby minor requirement only; source gem/multiABI selection unresolved.
Package-tool must be absent from production binary. Registry name unverified.

Root workspace/lock required; Magnus=0.9.3 bytes, transitive rb-sys lock audit
pending. CRuby3.3/3.4 LinuxGNU candidates; Windows UCRT/MinGW ABI is distinct
from RustMSVC. JRuby/TruffleRuby/Ractors unsupported.

Compiler/unit/runtime/package/install/require/framework/GC/GVL/ABI/IPC/audit
all NOT_RUN. Hosted must exercise Rack/Sinatra/Rails and preserve actual counts.

Feature host-pilot opts into unchanged ruby --require native-extension, stdin
closed; contract expects3unique dependency-down503/noeffects0 groups only.
Production excludes host-pilot/package-tool. Package-tool init reads at most64KiB
of private JSON from HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST and calls upstream
APIs, not eval or a generated Ruby executable string. Archive/install remain
NOT_RUN. CGI leading-zero/duplicate content header ambiguity must reject.
Root-approved host isolation gates Magnus behind native. host-pilot/package-tool
both enable native, while production native enables neither fixture nor packager.
Default0tests/empty host build is not native/product evidence. Shared pure and
native feature compile/actual host/install groups are all mandatory. Host source
is src/native.rs; explicit Init_humanymous_ruby symbol is unchanged.
