# Windows environment discovery contract

The existing Rust maintenance binary exposes `windows-doctor` and an explicit
`windows-doctor --probe` mode.
It emits `humanymous.windows-doctor.v1` JSON containing explicit executable paths,
the VS2022 developer-environment script, and proposed child `PYO3_PYTHON` setting.
Discovery mode never executes discovered programs. Neither mode installs tools, mutates environment state,
starts containers, invokes WSL, or changes Docker backend. Exit zero means the
inventory was emitted; it does not mean prerequisites are complete.

Missing paths remain null. Every tool has `runtime_validated: false`. Python312
discovery uses its explicit per-user installation candidate rather than silently
accepting an unrelated PATH Python. File presence does not prove version, Python
headers/import library validity, x64 target, authenticity, or ABI compatibility. Discovery
uses common VS2022 edition paths and reports the installer `vswhere.exe` candidate;
it does not claim arbitrary custom VS installation discovery.

The Python SDK observations check the adjacent `include/Python.h` and
`libs/python312.lib` files. They do not parse headers or link the library.
Probe mode on Windows executes fixed version arguments only: `rustc -Vv` and
`--version` for Cargo, Python, Node, Ruby, Docker, CMake and Ninja. It does not
execute vswhere, compiler builds, Python imports, daemon queries or shell input.
Version matching records Rust1.99.1/MSVC host, Cargo1.99.1, Python3.12, Node24 and
Ruby3.3/mingw-ucrt separately from full runtime validation. Probe failures remain
explicit; non-Windows does not execute these Windows tool probes.

Each child has a three-second polling budget under one 30-second command clock;
stdout/stderr use private scratch files, a 16KiB combined accepted capture and
output-size polling. An owned child is killed/reaped on observed timeout/overflow.
Polling and OS spawn/kill/filesystem calls are not hard resource/deadline guarantees;
transient disk output can exceed the accepted capture before polling. Probes are
for fixed trusted local tools, not an untrusted process containment interface.
Windows children use CREATE_NO_WINDOW. No global environment is changed.

MSVC commands require a separately controlled child developer environment from
the discovered `VsDevCmd.bat`. This inventory does not execute that environment
or provide an arbitrary shell-command runner. The report supplies fixed x64 host
and target setup arguments. Proposed PyO3 child configuration is emitted only
after a matching Python3.12 version probe plus both adjacent SDK file observations.
It still requires an actual SDK/link/import gate. Ruby mingw-ucrt and
an MSVC Rust extension retain an explicit missing ABI qualification profile.

Local unit/process validation uses native Windows. Linux container gates use the
explicit Windows Docker executable with `--context desktop-linux`, following
rule96 endpoint validation. Genuine CI OS gates remain required. No global PATH,
power, affinity, Docker backend, or WSL settings are changed by this command.

Status: source preparation only. Runtime, compiler, version and SDK checks are
not executed or qualified by this checkpoint.
