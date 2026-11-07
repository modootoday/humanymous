//! Reviewed synchronous Linux httpd bridge. Host pointer validity and callback
//! serialization are ABI premises, not conclusions from null checks.
use crate::ffi::*;
use humanymous_apache_adapter::{Cancellation, Child, Mpm, ReadMode, Replay};
use humanymous_authority::client::{BoundResponse, Verdict};
use humanymous_core::integrity::adapter::AdapterProfile;
use humanymous_framework_adapter::{
    Admission, Configuration, Observation, Pending, body_length, internal_header,
};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::OnceLock;

const FAILURE: c_int = 503;
const PASS: c_int = -1;
const FILTER: &CStr = c"HUMANYMOUS_REPLAY";
// Preserve the generated enum values while using Rust constant names in patterns.
const MODE_BYTES: ap_input_mode_t = ap_input_mode_t_AP_MODE_READBYTES;
const MODE_LINE: ap_input_mode_t = ap_input_mode_t_AP_MODE_GETLINE;
const MODE_SPECULATIVE: ap_input_mode_t = ap_input_mode_t_AP_MODE_SPECULATIVE;
struct Settings {
    path: Option<PathBuf>,
}
struct Process {
    child: Child,
    generation: u64,
    server: usize,
    config: Configuration,
}
static PROCESS: OnceLock<Process> = OnceLock::new();
fn running(process: &Process) -> Result<(), ()> {
    let mut state: c_int = 0;
    let mut generation: c_int = 0;
    // SAFETY: public MPM queries write only these initialized output integers.
    if unsafe { ap_mpm_query(AP_MPMQ_MPM_STATE as c_int, &mut state) } != 0
        || unsafe { ap_mpm_query(AP_MPMQ_GENERATION as c_int, &mut generation) } != 0
        || state != AP_MPMQ_RUNNING as c_int
        || u64::try_from(generation)
            .ok()
            .and_then(|value| value.checked_add(1))
            != Some(process.generation)
    {
        let _ = process.child.shutdown();
        return Err(());
    }
    Ok(())
}
// The HTTP/1 connection socket remains host-owned for this synchronous input
// callback. No pointer is sent to Tokio or retained after guard returns.
struct SocketDeadline {
    connection: *mut conn_rec,
    socket: *mut apr_socket_t,
    previous: apr_interval_time_t,
}
impl SocketDeadline {
    fn new(connection: *mut conn_rec) -> Result<Self, ()> {
        // SAFETY: caller supplies this live HTTP/1 request's host connection.
        let socket = unsafe { ap_get_conn_socket(connection) };
        if socket.is_null() {
            return Err(());
        }
        let mut previous = 0;
        if unsafe { apr_socket_timeout_get(socket, &mut previous) } != 0 {
            return Err(());
        }
        Ok(Self {
            connection,
            socket,
            previous,
        })
    }
    fn limit(&self, pending: &Pending) -> Result<(), ()> {
        let remaining = pending.remaining().map_err(|_| ())?;
        let timeout = i64::try_from(remaining.as_micros()).map_err(|_| ())?.max(1);
        // SAFETY: same live socket, and duration never exceeds original budget.
        if unsafe { apr_socket_timeout_set(self.socket, timeout) } != 0 {
            return Err(());
        }
        Ok(())
    }
}
impl Drop for SocketDeadline {
    fn drop(&mut self) {
        // SAFETY: callback still owns the live connection/socket. If restoration
        // fails, mark it aborted rather than reuse a socket with unknown limits.
        unsafe {
            if apr_socket_timeout_set(self.socket, self.previous) != 0 {
                (*self.connection).aborted = 1;
            }
        }
    }
}
struct Context {
    thread: std::thread::ThreadId,
    replay: Replay,
    original: Observation,
    uri: String,
    _admission: Admission,
    pending: Pending,
    response: Option<BoundResponse>,
}

// SAFETY: httpd initializes these exports during single-threaded master setup;
// subsequent callbacks read the fixed index. No Rust references alias mutation.
/// Export required by httpd's LoadModule directive for the pinned host ABI.
#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
pub static mut humanymous_module: module = module {
    version: MODULE_MAGIC_NUMBER_MAJOR as _,
    minor_version: MODULE_MAGIC_NUMBER_MINOR as _,
    module_index: -1,
    name: c"humanymous_rust".as_ptr(),
    dynamic_load_handle: ptr::null_mut(),
    next: ptr::null_mut(),
    magic: MODULE_MAGIC_COOKIE as _,
    rewrite_args: None,
    create_dir_config: None,
    merge_dir_config: None,
    create_server_config: Some(create_settings),
    merge_server_config: None,
    cmds: ptr::addr_of!(COMMANDS).cast(),
    register_hooks: Some(register),
    flags: 0,
};
static mut COMMANDS: [command_rec; 2] = [
    command_rec {
        name: c"HumanymousConfig".as_ptr(),
        func: cmd_func {
            take1: Some(configure),
        },
        cmd_data: ptr::null_mut(),
        req_override: RSRC_CONF as _,
        args_how: cmd_how_TAKE1,
        errmsg: c"absolute private child-owned configuration path".as_ptr(),
    },
    command_rec {
        name: ptr::null(),
        func: cmd_func { take1: None },
        cmd_data: ptr::null_mut(),
        req_override: 0,
        args_how: cmd_how_RAW_ARGS,
        errmsg: ptr::null(),
    },
];

fn boundary(action: impl FnOnce() -> Result<c_int, ()>) -> c_int {
    catch_unwind(AssertUnwindSafe(action))
        .ok()
        .and_then(Result::ok)
        .unwrap_or(FAILURE)
}
/// Read only a host-provided initialized NUL-terminated allocation. The host
/// guarantees allocation validity; the scan bound only limits copied metadata.
unsafe fn text(value: *const c_char, limit: usize) -> Result<String, ()> {
    if value.is_null() {
        return Err(());
    }
    let mut n = 0;
    // SAFETY: caller guarantees a live host string through its NUL terminator.
    while n <= limit && unsafe { *value.add(n) } != 0 {
        n += 1;
    }
    if n > limit {
        return Err(());
    }
    // SAFETY: same initialized host string allocation, copied before return.
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(value.cast(), n) })
        .map(str::to_owned)
        .map_err(|_| ())
}
unsafe fn index() -> Result<usize, ()> {
    // SAFETY: index was initialized by httpd before callbacks.
    usize::try_from(unsafe { (*ptr::addr_of!(humanymous_module)).module_index }).map_err(|_| ())
}
unsafe fn settings(server: *mut server_rec) -> Result<*mut Settings, ()> {
    if server.is_null() {
        return Err(());
    }
    // SAFETY: host server owns a module vector indexed by this loaded module.
    let vector = unsafe { (*server).module_config.cast::<*mut c_void>() };
    if vector.is_null() {
        return Err(());
    }
    let value = unsafe { *vector.add(index()?) }.cast::<Settings>();
    if value.is_null() { Err(()) } else { Ok(value) }
}
extern "C" fn create_settings(pool: *mut apr_pool_t, _: *mut server_rec) -> *mut c_void {
    catch_unwind(AssertUnwindSafe(|| {
        if pool.is_null() {
            return ptr::null_mut();
        }
        let value = Box::into_raw(Box::new(Settings { path: None }));
        // SAFETY: pool owns exactly this Box until its single cleanup callback.
        unsafe {
            apr_pool_cleanup_register(
                pool,
                value.cast(),
                Some(drop_settings),
                Some(apr_pool_cleanup_null),
            )
        };
        value.cast()
    }))
    .unwrap_or(ptr::null_mut())
}
extern "C" fn drop_settings(value: *mut c_void) -> apr_status_t {
    boundary(|| {
        if value.is_null() {
            return Err(());
        }
        // SAFETY: registered once by create_settings, callback invoked once.
        drop(unsafe { Box::from_raw(value.cast::<Settings>()) });
        Ok(0)
    }) as _
}
extern "C" fn configure(
    parms: *mut cmd_parms,
    _: *mut c_void,
    path: *const c_char,
) -> *const c_char {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), ()> {
        if parms.is_null() {
            return Err(());
        }
        // SAFETY: httpd invokes TAKE1 with live cmd_parms/server/string.
        let value = unsafe { &mut *settings((*parms).server)? };
        let path = PathBuf::from(unsafe { text(path, 4096)? });
        if !path.is_absolute() || value.path.is_some() {
            return Err(());
        }
        value.path = Some(path);
        Ok(())
    }));
    if matches!(result, Ok(Ok(()))) {
        ptr::null()
    } else {
        c"invalid HumanymousConfig".as_ptr()
    }
}
extern "C" fn child_init(pool: *mut apr_pool_t, server: *mut server_rec) {
    let _ = boundary(|| {
        if pool.is_null() || server.is_null() {
            return Err(());
        }
        // SAFETY: child_init owns live child pool and immutable server config.
        let path = unsafe { (&*settings(server)?).path.as_deref().ok_or(())? };
        // Initial target is a dedicated listener, not a merged virtualhost tree.
        if unsafe { !(*server).next.is_null() } {
            return Err(());
        }
        let config = read_configuration(path)?;
        if !config.sync_input_deadline_enforced {
            return Err(());
        }
        let mut threaded = 0;
        let mut generation = 0;
        let mut threads = 0;
        // SAFETY: query writes only the provided initialized output integers.
        unsafe {
            if ap_mpm_query(AP_MPMQ_IS_THREADED as _, &mut threaded) != 0
                || ap_mpm_query(AP_MPMQ_GENERATION as _, &mut generation) != 0
                || ap_mpm_query(AP_MPMQ_MAX_THREADS as _, &mut threads) != 0
            {
                return Err(());
            }
        }
        // Worker/prefork return APR_ENOTIMPL for IS_ASYNC. Select only an
        // explicit pinned identity, never infer a known model from that error.
        // SAFETY: the live child host returns a static NUL-terminated MPM name.
        let name = unsafe { text(ap_show_mpm(), 16)? };
        let threaded = match threaded {
            value if value == AP_MPMQ_STATIC as c_int => true,
            value if value == AP_MPMQ_NOT_SUPPORTED as c_int => false,
            _ => return Err(()),
        };
        let asynchronous = if name == "event" {
            let mut value: c_int = 0;
            // SAFETY: this supported event query writes the initialized local
            // integer only; no borrowed host memory crosses the call.
            if unsafe { ap_mpm_query(AP_MPMQ_IS_ASYNC as c_int, &mut value) } != 0 {
                return Err(());
            }
            match value {
                0 => Some(false),
                1 => Some(true),
                _ => return Err(()),
            }
        } else {
            None
        };
        let threads = usize::try_from(threads).map_err(|_| ())?;
        let mpm = Mpm::identify(&name, threaded, asynchronous, threads).map_err(|_| ())?;
        let generation = u64::try_from(generation)
            .map_err(|_| ())?
            .checked_add(1)
            .ok_or(())?;
        let capacity = if mpm == Mpm::Prefork {
            1
        } else {
            threads.min(64)
        };
        let child = Child::initialize(generation, mpm, capacity).map_err(|_| ())?;
        PROCESS
            .set(Process {
                child,
                generation,
                server: server as usize,
                config,
            })
            .map_err(|_| ())?;
        // SAFETY: no parent runtime exists; child cleanup only invalidates state.
        unsafe {
            apr_pool_cleanup_register(
                pool,
                ptr::null(),
                Some(child_cleanup),
                Some(apr_pool_cleanup_null),
            )
        };
        Ok(0)
    });
}
extern "C" fn child_cleanup(_: *mut c_void) -> apr_status_t {
    boundary(|| {
        PROCESS.get().ok_or(())?.child.shutdown().map_err(|_| ())?;
        Ok(0)
    }) as _
}
extern "C" fn register(_: *mut apr_pool_t) {
    let _ = boundary(|| {
        // SAFETY: httpd master invokes registration once per configuration epoch;
        // callbacks have exact generated signatures and static names.
        unsafe {
            ap_hook_child_init(Some(child_init), ptr::null(), ptr::null(), -10);
            ap_hook_post_read_request(Some(guard), ptr::null(), ptr::null(), -10);
            ap_hook_quick_handler(Some(quick), ptr::null(), ptr::null(), -10);
            ap_hook_fixups(Some(final_check), ptr::null(), ptr::null(), 10);
            ap_hook_handler(Some(handoff), ptr::null(), ptr::null(), -10);
            ap_register_input_filter(
                FILTER.as_ptr(),
                Some(input),
                None,
                ap_filter_type_AP_FTYPE_RESOURCE,
            );
        }
        Ok(0)
    });
}

fn read_configuration(path: &Path) -> Result<Configuration, ()> {
    use humanymous_authority::configuration::Peer;
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let uid = rustix::process::geteuid().as_raw();
    if path.canonicalize().map_err(|_| ())? != path {
        return Err(());
    }
    humanymous_authority::transport::private_directory(
        path.parent().ok_or(())?,
        &Peer::Unix { uid },
    )
    .map_err(|_| ())?;
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| ())?;
    let file = std::fs::File::from(fd);
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.permissions().mode() & 0o777 != 0o600
        || metadata.len() > 65536
    {
        return Err(());
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    file.take(65537).read_to_end(&mut bytes).map_err(|_| ())?;
    let config =
        Configuration::parse(std::str::from_utf8(&bytes).map_err(|_| ())?).map_err(|_| ())?;
    if !matches!(config.server, Peer::Unix { uid: expected } if expected == uid) {
        return Err(());
    }
    Ok(config)
}

unsafe fn observation(request: *mut request_rec) -> Result<Observation, ()> {
    if request.is_null() {
        return Err(());
    }
    // SAFETY: caller supplies the current live request and initialized table.
    let r = unsafe { &*request };
    if !r.main.is_null()
        || !r.prev.is_null()
        || ![0, 2].contains(&r.proxyreq)
        || r.headers_in.is_null()
        || ![1000, 1001].contains(&r.proto_num)
    {
        return Err(());
    }
    let method = unsafe { text(r.method, 32)? };
    let target = unsafe { text(r.unparsed_uri, 8192)? };
    let table = unsafe { apr_table_elts(r.headers_in) };
    if table.is_null() {
        return Err(());
    }
    let table = unsafe { &*table };
    let count = usize::try_from(table.nelts).map_err(|_| ())?;
    if count > 256
        || table.elt_size as usize != std::mem::size_of::<apr_table_entry_t>()
        || (count != 0 && table.elts.is_null())
    {
        return Err(());
    }
    let mut headers = Vec::new();
    let mut authority = None;
    let mut total = 0usize;
    for n in 0..count {
        // SAFETY: host array owns nelts initialized apr_table_entry_t records.
        let entry = unsafe { &*table.elts.cast::<apr_table_entry_t>().add(n) };
        let name = unsafe { text(entry.key, 256)? }.to_ascii_lowercase();
        let value = unsafe { text(entry.val, 8192)? };
        total = total.checked_add(name.len() + value.len()).ok_or(())?;
        if total > 65536 {
            return Err(());
        }
        if name == "host" && authority.replace(value.clone()).is_some() {
            return Err(());
        }
        headers.push([name, value]);
    }
    Ok(Observation {
        method,
        target,
        authority: authority.ok_or(())?,
        headers,
        peer: None,
        connection: None,
        raw: false,
    })
}
unsafe fn context(request: *mut request_rec) -> Result<*mut Context, ()> {
    if request.is_null() {
        return Err(());
    }
    // SAFETY: live host request vector owns an initialized slot for every module.
    let vector = unsafe { (*request).request_config.cast::<*mut c_void>() };
    if vector.is_null() {
        return Err(());
    }
    let value = unsafe { *vector.add(index()?) }.cast::<Context>();
    if value.is_null() {
        return Err(());
    }
    // SAFETY: this vector slot contains only our live pool-owned context.
    if unsafe { (*value).thread } != std::thread::current().id() {
        return Err(());
    }
    Ok(value)
}
extern "C" fn cleanup_cancel(value: *mut c_void) -> apr_status_t {
    boundary(|| {
        if value.is_null() {
            return Err(());
        }
        // SAFETY: one Box registered once in the matching request pool.
        let signal = unsafe { Box::from_raw(value.cast::<Cancellation>()) };
        signal.cancel();
        Ok(0)
    }) as _
}
extern "C" fn cleanup_context(value: *mut c_void) -> apr_status_t {
    boundary(|| {
        if value.is_null() {
            return Err(());
        }
        // SAFETY: one context registered once; filter no longer runs after pool cleanup.
        drop(unsafe { Box::from_raw(value.cast::<Context>()) });
        Ok(0)
    }) as _
}
extern "C" fn guard(request: *mut request_rec) -> c_int {
    boundary(|| {
        let process = PROCESS.get().ok_or(())?;
        running(process)?;
        let original = unsafe { observation(request)? };
        // SAFETY: the host promises a live request. Avoid holding a Rust
        // reference across native calls that can mutate its allocation.
        let (server, connection, pool, headers, vector) = unsafe {
            (
                (*request).server,
                (*request).connection,
                (*request).pool,
                (*request).headers_in,
                (*request).request_config,
            )
        };
        if server as usize != process.server
            || connection.is_null()
            || pool.is_null()
            || vector.is_null()
            || unsafe { (*connection).aborted } != 0
            || unsafe { (*request).proxyreq } != 0
            || !process.config.permits_target(&original.target)
        {
            return Err(());
        }
        // Cache quick handlers are outside this initial module's supported graph.
        if unsafe { !ap_find_linked_module(c"mod_cache.c".as_ptr()).is_null() } {
            return Err(());
        }
        // The initial pinned tuple is plain HTTP/1. TLS close-notify handling
        // needs its own filter-aware cancellation contract and actual pilot.
        if unsafe { !ap_find_linked_module(c"mod_ssl.c".as_ptr()).is_null() } {
            return Err(());
        }
        let mut length = None;
        for [name, value] in &original.headers {
            if ["transfer-encoding", "upgrade", "trailer", "expect"].contains(&name.as_str()) {
                return Err(());
            }
            if name == "content-length"
                && length
                    .replace(body_length(value, process.config.max_body).map_err(|_| ())?)
                    .is_some()
            {
                return Err(());
            }
        }
        let length = length.ok_or(())?;
        let pending = Pending::new(&process.config).map_err(|_| ())?;
        let admission = Admission::acquire().map_err(|_| ())?;
        let mut owned = process
            .child
            .begin(
                process.generation,
                true,
                length,
                process.config.max_body,
                pending.remaining().map_err(|_| ())?,
            )
            .map_err(|_| ())?;
        let signal = Box::into_raw(Box::new(owned.cancellation()));
        // SAFETY: same request pool invokes exactly one cleanup for this signal.
        unsafe {
            apr_pool_cleanup_register(
                pool,
                signal.cast(),
                Some(cleanup_cancel),
                Some(apr_pool_cleanup_null),
            )
        };
        let input_timeout = SocketDeadline::new(connection)?;
        input_timeout.limit(&pending)?;
        if unsafe { ap_setup_client_block(request, REQUEST_CHUNKED_ERROR as _) } != 0 {
            return Err(());
        }
        // Preserve the host input cursor for the original content handler.
        // Expect/transfer coding were rejected above; do not invoke the
        // one-shot ap_should_client_block on behalf of that handler.
        let (remaining, read_length) = unsafe { ((*request).remaining, (*request).read_length) };
        if remaining != i64::try_from(length).map_err(|_| ())? || read_length != 0 {
            return Err(());
        }
        if length != 0 {
            let mut buffer = [0u8; 8192];
            loop {
                input_timeout.limit(&pending)?;
                let n = unsafe {
                    ap_get_client_block(request, buffer.as_mut_ptr().cast(), buffer.len())
                };
                if n < 0 {
                    return Err(());
                }
                if n == 0 {
                    break;
                }
                let n = usize::try_from(n).map_err(|_| ())?;
                if n > buffer.len() {
                    return Err(());
                }
                owned.append(&buffer[..n]).map_err(|_| ())?;
            }
        }
        owned.finish().map_err(|_| ())?;
        drop(input_timeout);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ())?;
        let response = runtime
            .block_on(pending.decide(
                &process.config,
                &original,
                owned.body().map_err(|_| ())?,
                AdapterProfile::Apache,
            ))
            .map_err(|_| ())?;
        pending.check_handoff(response.expires()).map_err(|_| ())?;
        match response.verdict() {
            Verdict::Allow => {}
            Verdict::Deny | Verdict::Challenge => return Ok(403),
        }
        owned.remaining().map_err(|_| ())?;
        // Strip spoofed authority headers from the application view too.
        for [name, _] in &original.headers {
            if internal_header(name) {
                let key = std::ffi::CString::new(name.as_str()).map_err(|_| ())?;
                unsafe { apr_table_unset(headers, key.as_ptr()) };
            }
        }
        let mut original = original;
        original.headers.retain(|[name, _]| !internal_header(name));
        let uri = unsafe { text((*request).uri, 8192)? };
        if uri != original.target.split('?').next().ok_or(())? {
            return Err(());
        }
        let value = Box::into_raw(Box::new(Context {
            thread: std::thread::current().id(),
            replay: Replay::new(owned).map_err(|_| ())?,
            original,
            uri,
            _admission: admission,
            pending,
            response: Some(response),
        }));
        unsafe {
            apr_pool_cleanup_register(
                pool,
                value.cast(),
                Some(cleanup_context),
                Some(apr_pool_cleanup_null),
            );
            *vector.cast::<*mut c_void>().add(index()?) = value.cast();
            // Only the bounded replay filter may now supply this complete body.
            // Restore the exact pre-read cursor; lower native input is never
            // consulted by our supported filter modes.
            (*request).remaining = remaining;
            (*request).read_length = read_length;
            if ap_add_input_filter(FILTER.as_ptr(), value.cast(), request, connection).is_null() {
                return Err(());
            }
        }
        Ok(PASS)
    })
}
extern "C" fn quick(request: *mut request_rec, lookup: c_int) -> c_int {
    if lookup != 0 {
        return FAILURE;
    }
    final_check(request)
}
extern "C" fn final_check(request: *mut request_rec) -> c_int {
    boundary(|| {
        running(PROCESS.get().ok_or(())?)?;
        let current = unsafe { observation(request)? };
        // SAFETY: current HTTP/1 callback owns the live connection. APR probes
        // receive EOF without blocking or consuming application bytes.
        let connection = unsafe { (*request).connection };
        if connection.is_null() || unsafe { (*connection).aborted } != 0 {
            return Err(());
        }
        let socket = unsafe { ap_get_conn_socket(connection) };
        if socket.is_null() {
            return Err(());
        }
        let mut eof = 0;
        if unsafe { apr_socket_atreadeof(socket, &mut eof) } != 0 || eof != 0 {
            return Err(());
        }
        let saved = unsafe { &mut *context(request)? };
        if current.method != saved.original.method
            || current.target != saved.original.target
            || current.authority != saved.original.authority
            || current.headers != saved.original.headers
            || unsafe { text((*request).uri, 8192)? } != saved.uri
        {
            return Err(());
        }
        saved.replay.eof().map_err(|_| ())?;
        saved
            .pending
            .check_handoff(saved.response.as_ref().ok_or(())?.expires())
            .map_err(|_| ())?;
        Ok(PASS)
    })
}
extern "C" fn handoff(request: *mut request_rec) -> c_int {
    if final_check(request) != PASS {
        return FAILURE;
    }
    boundary(|| {
        let probe = {
            // SAFETY: handler callback owns this live request's matching context.
            let saved = unsafe { &mut *context(request)? };
            let response = saved.response.take().ok_or(())?;
            saved
                .pending
                .check_handoff(response.expires())
                .map_err(|_| ())?;
            if response.consume().map_err(|_| ())? != Verdict::Allow {
                return Err(());
            }
            saved.replay.eof().map_err(|_| ())?;
            #[cfg(feature = "pilot-probes")]
            let probe = match saved.original.target.as_str() {
                "/probe-subrequest" => 1,
                "/probe-redirect" => 2,
                _ => 0,
            };
            #[cfg(not(feature = "pilot-probes"))]
            let probe = 0;
            probe
        };
        // No Context reference may survive calls that can re-enter host hooks.
        #[cfg(feature = "pilot-probes")]
        {
            // Test-only actual host lifecycle probes; never enabled implicitly
            // by a client header, production configuration or runtime setting.
            if probe == 1 {
                // SAFETY: this callback owns the live parent; lookup creates
                // one host-owned subrequest pool, destroyed once below.
                unsafe {
                    let subrequest =
                        ap_sub_req_lookup_uri(c"/echo".as_ptr(), request, ptr::null_mut());
                    if subrequest.is_null() {
                        return Err(());
                    }
                    // NULL next_filter requests lookup-only processing. Its
                    // real fixups must reject this nested request before a
                    // handler or response protocol can touch the parent.
                    // SAFETY: non-null lookup result remains live until the
                    // matching destroy call; copy the scalar before destruction.
                    let rejected = (*subrequest).status == FAILURE;
                    ap_destroy_sub_req(subrequest);
                    if !rejected {
                        return Err(());
                    }
                    apr_table_set(
                        (*request).err_headers_out,
                        c"X-Pilot-Probe-Ran".as_ptr(),
                        c"subrequest".as_ptr(),
                    );
                }
                return Ok(FAILURE);
            }
            if probe == 2 {
                unsafe {
                    apr_table_set(
                        (*request).err_headers_out,
                        c"X-Pilot-Probe-Ran".as_ptr(),
                        c"redirect".as_ptr(),
                    );
                    ap_internal_redirect(c"/echo".as_ptr(), request);
                }
                return Ok(0);
            }
        }
        #[cfg(not(feature = "pilot-probes"))]
        let _ = probe;
        Ok(PASS)
    })
}
extern "C" fn input(
    filter: *mut ap_filter_t,
    brigade: *mut apr_bucket_brigade,
    mode: ap_input_mode_t,
    _: apr_read_type_e,
    maximum: apr_off_t,
) -> apr_status_t {
    boundary(|| {
        if filter.is_null() || brigade.is_null() || maximum < 0 {
            return Err(());
        }
        let mode = match mode {
            MODE_BYTES => ReadMode::Bytes,
            MODE_LINE => ReadMode::Line,
            MODE_SPECULATIVE => ReadMode::Speculative,
            _ => return Err(()),
        };
        // SAFETY: matching request filter owns the live pool context on this thread.
        let context = unsafe { context((*filter).r)? };
        if unsafe { (*filter).ctx } != context.cast() {
            return Err(());
        }
        let value = unsafe { &mut *context };
        let bytes = value
            .replay
            .read(mode, usize::try_from(maximum).map_err(|_| ())?)
            .map_err(|_| ())?;
        if !bytes.is_empty()
            && unsafe {
                apr_brigade_write(
                    brigade,
                    None,
                    ptr::null_mut(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                )
            } != 0
        {
            return Err(());
        }
        if value.replay.eof().map_err(|_| ())? {
            // SAFETY: APR-created EOS bucket and current brigade share this host
            // allocator. APR_RING sentinel aliases the initial bucket link only.
            unsafe {
                let bucket = apr_bucket_eos_create((*brigade).bucket_alloc);
                if bucket.is_null() {
                    return Err(());
                }
                let sentinel = ptr::addr_of_mut!((*brigade).list).cast::<apr_bucket>();
                let previous = (*brigade).list.prev;
                (*bucket).link.next = sentinel;
                (*bucket).link.prev = previous;
                (*previous).link.next = bucket;
                (*brigade).list.prev = bucket;
            }
        }
        Ok(0)
    }) as _
}
