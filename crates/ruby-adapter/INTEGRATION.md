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
