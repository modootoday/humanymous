//! Reviewed Linux nginx ABI. All pointers are confined to the nginx thread.
//! Pool cleanup owns timer deletion and future cancellation; tasks own no nginx
//! memory. ngx bindings and runtime must be built from the same pinned ABI.
use crate::request::{self, Configuration, Ingress};
use humanymous_authority::client::{self, BoundResponse, ClientOptions, ExpectedBinding, Verdict};
use humanymous_integrity::adapter::AdapterProfile;
use ngx::core::Status;
use ngx::ffi::*;
use ngx::http::{self, HttpModule, HttpModuleLocationConf, HttpRequestHandler, MergeConfigError};
use ngx::ngx_string;
use std::cell::{RefCell, UnsafeCell};
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

struct Module;
impl HttpModule for Module {
    fn module() -> &'static ngx_module_t {
        // SAFETY: nginx initializes this static during master configuration;
        // request callbacks only read its fixed ctx_index on the owning thread.
        unsafe { &*ptr::addr_of!(ngx_http_humanymous_module) }
    }
    unsafe extern "C" fn postconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
        if cf.is_null() {
            return NGX_ERROR;
        }
        catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: nginx supplies a live exclusive configuration callback.
            http::add_phase_handler::<Gate>(unsafe { &mut *cf }).map_or(NGX_ERROR, |_| NGX_OK)
        }))
        .unwrap_or(NGX_ERROR)
    }
}
#[derive(Default)]
struct Location {
    config: Option<Arc<Configuration>>,
}
impl http::Merge for Location {
    fn merge(&mut self, parent: &Self) -> Result<(), MergeConfigError> {
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
        Ok(())
    }
}
// SAFETY: Location is exclusively pool-owned and initialized with Default by SDK.
unsafe impl HttpModuleLocationConf for Module {
    type LocationConf = Location;
}

static mut COMMANDS: [ngx_command_t; 2] = [
    ngx_command_t {
        name: ngx_string!("humanymous_guard"),
        type_: NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        set: Some(configure),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t::empty(),
];
static CONTEXT: ngx_http_module_t = ngx_http_module_t {
    preconfiguration: None,
    postconfiguration: Some(Module::postconfiguration),
    create_main_conf: None,
    init_main_conf: None,
    create_srv_conf: None,
    merge_srv_conf: None,
    create_loc_conf: Some(Module::create_loc_conf),
    merge_loc_conf: Some(Module::merge_loc_conf),
};
#[used]
#[allow(non_upper_case_globals)]
pub static mut ngx_http_humanymous_module: ngx_module_t = ngx_module_t {
    ctx: &raw const CONTEXT as _,
    commands: ptr::addr_of_mut!(COMMANDS).cast(),
    type_: NGX_HTTP_MODULE,
    ..ngx_module_t::default()
};
#[cfg(feature = "export-modules")]
ngx::ngx_modules!(ngx_http_humanymous_module);

extern "C" fn configure(
    cf: *mut ngx_conf_t,
    _: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if cf.is_null() || conf.is_null() {
            return Err(());
        }
        // SAFETY: nginx's TAKE1 callback supplies exactly two initialized args
        // and a Location created by the matching SDK location-conf allocator.
        let (args, location) = unsafe {
            (
                (*(*cf).args).as_slice::<ngx_str_t>(),
                &mut *conf.cast::<Location>(),
            )
        };
        if location.config.is_some() || args.len() != 2 {
            return Err(());
        }
        let path = args[1].to_str().map_err(|_| ())?;
        let config =
            request::read_private_configuration(std::path::Path::new(path)).map_err(|_| ())?;
        location.config = Some(Arc::new(config));
        Ok(())
    }));
    if matches!(result, Ok(Ok(()))) {
        ngx::core::NGX_CONF_OK
    } else {
        ngx::core::NGX_CONF_ERROR
    }
}

struct State {
    body_ready: bool,
    waiting_body: bool,
    started: bool,
    finished: bool,
    released: bool,
    receiver: Option<mpsc::Receiver<Result<BoundResponse, ()>>>,
    task: Option<tokio::task::JoinHandle<()>>,
    snapshot: Option<Ingress>,
    verdict: Option<Result<BoundResponse, ()>>,
}
struct Context {
    // SAFETY: only nginx callbacks read/mutate this event on one thread.
    event: UnsafeCell<ngx_event_t>,
    request: *mut ngx_http_request_t,
    state: RefCell<State>,
    started: Instant,
    budget: Duration,
    deadline: u64,
}
impl Context {
    fn new(r: *mut ngx_http_request_t, timeout: u64) -> Result<Self, ()> {
        let deadline = humanymous_authority::now_ms()
            .map_err(|_| ())?
            .checked_add(timeout)
            .ok_or(())?;
        Ok(Self {
            // SAFETY: ngx_event_t is a C plain-data struct; zero is nginx's
            // documented initialization before setting handler/data/log.
            event: UnsafeCell::new(unsafe { std::mem::zeroed() }),
            request: r,
            state: RefCell::new(State {
                body_ready: false,
                waiting_body: false,
                started: false,
                finished: false,
                released: false,
                receiver: None,
                task: None,
                snapshot: None,
                verdict: None,
            }),
            started: Instant::now(),
            budget: Duration::from_millis(timeout),
            deadline,
        })
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        if let Some(task) = self.state.get_mut().task.take() {
            task.abort();
        }
        // SAFETY: pool cleanup runs on nginx thread before releasing event memory.
        // Timer dispatch and cleanup cannot run concurrently; no posted events.
        unsafe {
            if (*self.event.get()).timer_set() != 0 {
                ngx_del_timer(self.event.get());
            }
        }
    }
}

static INFLIGHT: AtomicUsize = AtomicUsize::new(0);
struct Slot;
impl Slot {
    fn acquire() -> Result<Self, ()> {
        INFLIGHT
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 16).then_some(n + 1)
            })
            .map(|_| Self)
            .map_err(|_| ())
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        INFLIGHT.fetch_sub(1, Ordering::AcqRel);
    }
}
fn runtime() -> Result<&'static Runtime, ()> {
    // First called in a request, never in the pre-fork nginx master.
    static RUNTIME: OnceLock<Result<Runtime, ()>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .max_blocking_threads(4)
                .enable_all()
                .build()
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| ())
}

struct Gate;
impl HttpRequestHandler for Gate {
    const PHASE: http::HttpPhase = http::HttpPhase::Preaccess;
    type Output = Status;
    fn handler(r: &mut http::Request) -> Status {
        catch_unwind(AssertUnwindSafe(|| handle(r))).unwrap_or_else(|_| reject(r, 503))
    }
}
fn reject(r: &mut http::Request, status: isize) -> Status {
    if let Some(ctx) = r.get_module_ctx::<Context>(Module::module()) {
        // SAFETY: still on the nginx thread; prevent late timer phase resumption
        // while an early rejection response is flushing to a slow client.
        unsafe {
            if (*ctx.event.get()).timer_set() != 0 {
                ngx_del_timer(ctx.event.get());
            }
        }
        if let Ok(mut state) = ctx.state.try_borrow_mut() {
            state.finished = true;
            if let Some(task) = state.task.take() {
                task.abort();
            }
        }
    }
    let _ = r.add_header_out("Cache-Control", "no-store");
    Status(status)
}
fn handle(r: &mut http::Request) -> Status {
    let Some(config) = Module::location_conf(r).and_then(|l| l.config.clone()) else {
        return Status::NGX_DECLINED;
    };
    if !r.is_main() || r.as_ref().http_version != NGX_HTTP_VERSION_11 {
        return reject(r, 400);
    }
    let raw: *mut ngx_http_request_t = r.into();
    let mut ctx = r
        .get_module_ctx::<Context>(Module::module())
        .map(|c| c as *const Context);
    if ctx.is_none() {
        let value = match Context::new(raw, config.timeout_ms) {
            Ok(v) => v,
            Err(()) => return reject(r, 503),
        };
        let p = r.pool().allocate(value);
        if p.is_null() {
            return reject(r, 503);
        }
        r.set_module_ctx(p.cast(), Module::module());
        // SAFETY: p is initialized pool memory, stable until cleanup. All
        // event users are this nginx thread and cleanup removes the timer.
        unsafe {
            (*(*p).event.get()).handler = Some(poll);
            (*(*p).event.get()).data = p.cast();
            (*(*p).event.get()).log = r.log();
            ngx_add_timer((*p).event.get(), 10);
        }
        ctx = Some(p);
        // Reject unknown routes/unsupported headers before reading the body.
        if head_allowed(r, &config).is_err() {
            return reject(r, 400);
        }
        // SAFETY: nginx buffered body API owns the request and invokes callback
        // once; callback releases its main count and resumes phases if needed.
        let rc = unsafe { ngx_http_read_client_request_body(raw, Some(body_ready)) };
        if rc >= NGX_HTTP_SPECIAL_RESPONSE {
            return reject(r, rc);
        }
        // SAFETY: body API did not finalize on the non-special path; pool lives.
        let context = unsafe { &*p };
        if !context.state.borrow().body_ready {
            context.state.borrow_mut().waiting_body = true;
            return Status::NGX_DONE;
        }
    }
    // SAFETY: Context is this module's initialized request pool object. All
    // mutation uses interior cells; no aliasing mutable reference is created.
    let context = unsafe { &*ctx.unwrap() };
    if context.started.elapsed() >= Duration::from_millis(config.timeout_ms) {
        return reject(r, 503);
    }
    {
        let mut state = context.state.borrow_mut();
        if state.released {
            return reject(r, 503);
        }
        if let Some(result) = state.verdict.take() {
            let unchanged = snapshot(r).ok().as_ref() == state.snapshot.as_ref();
            // The shared client rechecks expiry/monotonic budget at consumption.
            let verdict = result.and_then(|response| response.consume().map_err(|_| ()));
            state.released = true;
            drop(state);
            if !unchanged {
                return reject(r, 503);
            }
            return match verdict {
                Ok(Verdict::Allow) => Status::NGX_DECLINED,
                Ok(Verdict::Deny) => reject(r, 403),
                Ok(Verdict::Challenge) => reject(r, 409),
                Err(()) => reject(r, 503),
            };
        }
        if state.started || !state.body_ready {
            return Status::NGX_AGAIN;
        }
    }
    let input = match snapshot(r) {
        Ok(v) => v,
        Err(()) => return reject(r, 413),
    };
    let id = match request::nonce() {
        Ok(v) => v,
        Err(_) => return reject(r, 503),
    };
    let now = humanymous_authority::now_ms().unwrap_or(u64::MAX);
    let envelope = match input.envelope(&config, &id, context.deadline, now) {
        Ok(v) => v,
        Err(_) => return reject(r, 400),
    };
    let slot = match Slot::acquire() {
        Ok(v) => v,
        Err(()) => return reject(r, 503),
    };
    let rt = match runtime() {
        Ok(v) => v,
        Err(()) => return reject(r, 503),
    };
    // SAFETY: nginx's client-close observer runs on this owning thread; cleanup
    // aborts the future and timer before request pool memory can be released.
    unsafe {
        (*raw).read_event_handler = Some(ngx_http_test_reading);
        if ngx_handle_read_event((*(*raw).connection).read, 0) != NGX_OK {
            return reject(r, 503);
        }
    }
    let (tx, rx) = mpsc::channel();
    let deadline = context.deadline;
    let c = config.clone();
    let task = rt.spawn(async move {
        let _slot = slot;
        let options = ClientOptions {
            endpoint: &c.endpoint,
            credential: &c.credential,
            server: &c.server,
            timeout_ms: c.timeout_ms,
            adapter: AdapterProfile::Nginx,
        };
        let expected = ExpectedBinding {
            id: &id,
            tenant: &c.tenant,
            origin: &c.origin,
            revision: &c.revision,
            deadline,
        };
        let result = client::decide(&options, &envelope, expected)
            .await
            .map_err(|_| ());
        let _ = tx.send(result);
    });
    let mut state = context.state.borrow_mut();
    state.snapshot = Some(input);
    state.receiver = Some(rx);
    state.task = Some(task);
    state.started = true;
    Status::NGX_AGAIN
}

fn head_allowed(r: &mut http::Request, config: &Configuration) -> Result<(), ()> {
    // SAFETY: nginx owns the initialized request and its string/list storage.
    unsafe {
        let method = text(r.as_ref().method_name)?;
        let target = text(r.as_ref().unparsed_uri)?;
        let path = text(r.as_ref().uri)?;
        if target.split('?').next() != Some(path.as_str())
            || request::reserved(&path)
            || !config
                .routes
                .iter()
                .any(|route| route.method == method && route.path == path)
            || r.as_ref().headers_in.content_length_n > request::BODY_LIMIT as i64
        {
            return Err(());
        }
        let headers = headers(r, true)?;
        for [name, value] in headers {
            let n = name.to_ascii_lowercase();
            if [
                "upgrade",
                "trailer",
                "transfer-encoding",
                "te",
                "expect",
                "proxy-connection",
            ]
            .contains(&n.as_str())
                || (n == "connection"
                    && !["close", "keep-alive"].contains(&value.to_ascii_lowercase().as_str()))
                || (n == "content-type"
                    && value.to_ascii_lowercase().starts_with("application/grpc"))
                || (n == "host" && value != config.authority)
            {
                return Err(());
            }
        }
    }
    Ok(())
}
unsafe fn text(value: ngx_str_t) -> Result<String, ()> {
    if value.len == 0 {
        return Ok(String::new());
    }
    if value.data.is_null() {
        return Err(());
    }
    // SAFETY: caller supplies a live nginx string with exactly len readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(value.data, value.len) };
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| ())
}
unsafe fn headers(r: &mut http::Request, sanitize: bool) -> Result<Vec<[String; 2]>, ()> {
    if sanitize {
        // nginx proxy does not skip hash=0 entries. Rebuild its generic list,
        // retaining original storage for nginx's typed header pointers.
        // SAFETY: all entries and strings remain live in the request pool;
        // copying C table entries preserves ordinary duplicate order/casing.
        unsafe {
            let list = ngx_list_create(r.as_ref().pool, 4, std::mem::size_of::<ngx_table_elt_t>());
            if list.is_null() {
                return Err(());
            }
            let mut part: *mut ngx_list_part_t = &raw mut r.as_mut().headers_in.headers.part;
            let mut visited = 0;
            let mut parts = 0;
            while !part.is_null() {
                parts += 1;
                if parts > 101 {
                    return Err(());
                }
                let p = &*part;
                if p.nelts > 100 || visited + p.nelts > 100 || (p.nelts > 0 && p.elts.is_null()) {
                    return Err(());
                }
                visited += p.nelts;
                for i in 0..p.nelts {
                    let h = p.elts.cast::<ngx_table_elt_t>().add(i);
                    if request::strip_header(&text((*h).key)?) {
                        continue;
                    }
                    let out = ngx_list_push(list).cast::<ngx_table_elt_t>();
                    if out.is_null() {
                        return Err(());
                    }
                    ptr::copy_nonoverlapping(h, out, 1);
                }
                part = p.next;
            }
            let first = (*list).last == ptr::addr_of_mut!((*list).part);
            r.as_mut().headers_in.headers = ptr::read(list);
            if first {
                r.as_mut().headers_in.headers.last = &raw mut r.as_mut().headers_in.headers.part;
            }
        }
    }
    let mut out = Vec::new();
    let mut part: *mut ngx_list_part_t = &raw mut r.as_mut().headers_in.headers.part;
    let mut parts = 0;
    while !part.is_null() {
        parts += 1;
        if parts > 101 {
            return Err(());
        }
        // SAFETY: nginx owns a valid list of table elements; bounded to100 total.
        let p = unsafe { &mut *part };
        if p.nelts > 100 || out.len() + p.nelts > 100 || (p.nelts > 0 && p.elts.is_null()) {
            return Err(());
        }
        for i in 0..p.nelts {
            let h = unsafe { &mut *p.elts.cast::<ngx_table_elt_t>().add(i) };
            if h.hash == 0 {
                continue;
            }
            let name = unsafe { text(h.key) }?;
            if request::strip_header(&name) {
                continue;
            }
            out.push([name, unsafe { text(h.value) }?]);
        }
        part = p.next;
    }
    Ok(out)
}
fn snapshot(r: &mut http::Request) -> Result<Ingress, ()> {
    // SAFETY: body_ready guarantees complete buffered data; no nginx pointer
    // escapes. File-backed buffers reject rather than hashing incomplete memory.
    unsafe {
        let mut body = Vec::new();
        let rb = r.as_ref().request_body;
        if rb.is_null() || !(*rb).temp_file.is_null() {
            return Err(());
        }
        let mut chain = (*rb).bufs;
        let mut count = 0;
        while !chain.is_null() {
            count += 1;
            if count > 256 {
                return Err(());
            }
            let b = (*chain).buf;
            if b.is_null() || (*b).in_file() != 0 {
                return Err(());
            }
            let (start, end) = ((*b).pos, (*b).last);
            if start != end {
                if start.is_null() || end.is_null() || end.addr() < start.addr() {
                    return Err(());
                }
                // nginx defines pos..last as one initialized buffer allocation.
                let len = end.offset_from_unsigned(start);
                if body.len() + len > request::BODY_LIMIT {
                    return Err(());
                }
                body.extend_from_slice(std::slice::from_raw_parts(start, len));
            }
            chain = (*chain).next;
        }
        Ok(Ingress {
            method: text(r.as_ref().method_name)?,
            target: text(r.as_ref().unparsed_uri)?,
            path: text(r.as_ref().uri)?,
            headers: headers(r, false)?,
            body,
        })
    }
}

unsafe extern "C" fn body_ready(raw: *mut ngx_http_request_t) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if raw.is_null() {
            return;
        }
        // SAFETY: invoked only by nginx's body API with its retained request.
        let r = unsafe { http::Request::from_ngx_http_request(raw) };
        // Release the extra main reference acquired by read_client_request_body.
        unsafe {
            let count = (*(*raw).main).count();
            (*(*raw).main).set_count(count - 1);
        }
        let Some(ctx) = r.get_module_ctx::<Context>(Module::module()) else {
            return;
        };
        let resume = {
            let mut s = ctx.state.borrow_mut();
            s.body_ready = true;
            s.waiting_body && !s.finished
        };
        if resume {
            unsafe {
                (*raw).write_event_handler = Some(ngx_http_core_run_phases);
                ngx_http_core_run_phases(raw);
            }
        }
    }));
    if result.is_err() && !raw.is_null() {
        unsafe {
            ngx_http_finalize_request(raw, 503);
        }
    }
}
unsafe extern "C" fn poll(event: *mut ngx_event_t) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if event.is_null() {
            return;
        }
        // SAFETY: only armed after pool allocation; cleanup removes the timer.
        let ctx = unsafe { &*(*event).data.cast::<Context>() };
        let raw = ctx.request;
        let now = humanymous_authority::now_ms().unwrap_or(u64::MAX);
        let mut resume = false;
        {
            let mut state = ctx.state.borrow_mut();
            if now >= ctx.deadline || ctx.started.elapsed() >= ctx.budget {
                state.verdict = Some(Err(()));
                resume = true;
            } else if let Some(rx) = &state.receiver {
                match rx.try_recv() {
                    Ok(v) => {
                        state.verdict = Some(v);
                        resume = true;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        state.verdict = Some(Err(()));
                        resume = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
            if resume {
                state.finished = true;
            }
        }
        if resume {
            // SAFETY: callback runs on the owning nginx thread and no cell
            // borrow remains live across phase dispatch or request finalization.
            let body_ready = ctx.state.borrow().body_ready;
            unsafe {
                if !body_ready {
                    let r = http::Request::from_ngx_http_request(raw);
                    let _ = r.add_header_out("Cache-Control", "no-store");
                    ngx_http_finalize_request(raw, 503);
                } else {
                    (*raw).write_event_handler = Some(ngx_http_core_run_phases);
                    ngx_http_core_run_phases(raw);
                }
            }
        } else {
            unsafe {
                ngx_add_timer(event, 10);
            }
        }
    }));
    if result.is_err() && !event.is_null() {
        unsafe {
            let ctx = &*(*event).data.cast::<Context>();
            ngx_http_finalize_request(ctx.request, 503);
        }
    }
}
