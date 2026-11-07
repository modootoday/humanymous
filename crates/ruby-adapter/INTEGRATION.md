# Ruby integration frontier

Native lib uses explicit Init_humanymous_ruby export. Humanymous.wrap returns
a VM Rack object whose app/config live in Ruby ivars. No Ruby Value is stored
in Rust heap collections. Body copies use safe upstream Magnus bytes API.
IPC runs on a Rust-only worker; upstream thread_sleep yields GVL while awaiting
it. A request-owned guard closes cancellation and joins the worker, including
Ruby interrupt/error exits. Thread scheduling, interrupt and GC.compact require
actual probes. This latest native source has not compiled or executed.

Trusted server-owned humanymous.client_disconnected callable is mandatory before
input, during the IPC wait and before origin handoff. Only exact Ruby false is
connected; true rejects and absence/nil/truthy/error fails closed. Rack has no
universal standard callable; a real server bridge and its disconnect proof remain
unfinished. A header or local IO#closed? cannot establish remote liveness.

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
closed. The current contract expects four native groups/eight cases: five
liveness-negative cases plus three framework dependency-down503/noeffect cases.
Production excludes host-pilot/package-tool. Package-tool init reads at most64KiB
of private JSON from HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST and calls upstream
APIs, not eval or a generated Ruby executable string. Archive/install remain
NOT_RUN. CGI leading-zero/duplicate content header ambiguity must reject.
Root-approved host isolation gates Magnus behind native. host-pilot/package-tool
both enable native, while production native enables neither fixture nor packager.
Default0tests/empty host build is not native/product evidence. Shared pure and
native feature compile/actual host/install groups are all mandatory. Host source
is src/native.rs; explicit Init_humanymous_ruby symbol is unchanged.
Rack hijack capability advertisement no longer denies every ordinary HTTP request;
the app-visible copy disables rack.hijack? and removes rack.hijack. Actual upgrade
requests still reject. New source is NOT_RUN. Native capabilities report actual
fixture/package feature bits; read-only receipt mode requires no Ruby source.

Rust standard-layout binary humanymous-ruby-package provides build RUBY
PACKAGER_EXTENSION STAGE METADATA and install-test RUBY GEM. It calls unchanged
Ruby/RubyGems with closed stdin, bounded1MiB streams,120s deadline and kill/wait
ownership. Build selects six declarative metadata members and retains upstream
RubyGems/Digest archive SHA256 receipt. Install uses --local/--ignore-dependencies
in a fresh GEM_HOME/GEM_PATH then requires production and rejects fixture bits.
No publish command or authored Ruby script exists. The default Rust driver does
not enable Magnus; its compilation cannot qualify Ruby ABI. Default all-targets
Rust-driver check/Clippy passed as recorded in evidence:root/20261107075243
summary003. No tests, Ruby native build/import or package execution ran.
That earlier snapshot does not compile the later worker/liveness changes.
Two worker cancellation/join/panic pure definitions and two package-driver pure
definitions now require a separate four-case default test gate. These pure tests
do not establish native Ruby ABI or callback behavior. Root then allocated the
four pure cases: all passed, zero failed/ignored/filtered. All-targets Clippy
failed one items_after_test_module diagnostic because Drop followed the tests.
The unchanged Drop impl was moved before tests without suppression. This order
fix awaits retry. A conservatively tracked test-support Docker file changed
during the batch, so full-closure stability is not claimed. Exact source/log
hashes are in evidence:root/20261107075243 summary005; native probes remain NOT_RUN.

Local Windows/WSL actual execution is now user-authorized under root allocation;
the active benchmark quiet window still permits no new build/runtime command.
The historical HOSTED opt-in variable name is unchanged and is not provenance.
