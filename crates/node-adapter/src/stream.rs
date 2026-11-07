//! Bounded pre-origin body collection and host-owned response streams.
use crate::{
    envelope::{self, Configuration, Observation, error},
    host::*,
};
use napi::{Env, Result, bindgen_prelude::*};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct Pending {
    _admission: humanymous_framework_adapter::Admission,
    cancelled: Arc<AtomicBool>,
    request: Lease,
    response: Option<Lease>,
    next: Option<FunctionRef<(), Unknown<'static>>>,
    app: Option<Lease>,
    resolve: Option<FunctionRef<Object<'static>, Unknown<'static>>>,
    environment: Option<Lease>,
    source: Option<Lease>,
    bytes: Vec<u8>,
    observation: Observation,
    deadline: u64,
}
impl Pending {
    fn release(self, env: &Env) -> Result<()> {
        self.request.unref(env)?;
        if let Some(r) = self.response {
            r.unref(env)?;
        }
        if let Some(a) = self.app {
            a.unref(env)?;
        }
        Ok(())
    }
}
type State = Rc<RefCell<Option<Pending>>>;

pub(crate) fn express(env: &Env, config: Configuration) -> Result<Function<'static, (), ()>> {
    let config = Arc::new(config);
    // Bun1.3.14's Node HTTP shim lowercases rawHeaders (measured by real socket
    // corpus). Never feed those names into casing-reliable network evidence.
    let raw = prop::<Option<Object>>(&global(env)?, "Bun")?.is_none();
    let callback = env.create_function_from_closure("humanymousExpress", move |ctx| {
        trace("express:entry");
        let req: Object = ctx.get(0)?;
        let res: Object = ctx.get(1)?;
        let next: Function<(), Unknown> = ctx.get(2)?;
        let run = || -> Result<()> {
            let admission =
                humanymous_framework_adapter::Admission::acquire().map_err(|e| error(e.0))?;
            if prop::<bool>(&req, "readableEnded")?
                || req.has_named_property("body")?
                || prop::<Option<String>>(&req, "readableEncoding")?.is_some()
                || prop::<u32>(&req, "httpVersionMajor")? != 1
            {
                return Err(error("guard must precede parsers on HTTP1"));
            }
            let socket: Object = prop(&req, "socket")?;
            let connection = if socket.has_named_property("__humanymousConnection")? {
                text(&socket, "__humanymousConnection")?
            } else {
                let id = envelope::uuid()?;
                let mut socket = socket;
                socket.set_named_property("__humanymousConnection", id.clone())?;
                id
            };
            let headers: Object = prop(&req, "headers")?;
            let observation = Observation {
                method: text(&req, "method")?,
                target: text(&req, "url")?,
                authority: text(&headers, "host")?,
                headers: pairs(prop(&req, "rawHeaders")?)?,
                peer: if raw {
                    prop::<Option<String>>(&socket, "remoteAddress")?
                } else {
                    None
                },
                connection: if raw { Some(connection) } else { None },
                raw,
            };
            let pending = Pending {
                _admission: admission,
                cancelled: Arc::new(AtomicBool::new(false)),
                request: reference(ctx.env, &req)?,
                response: Some(reference(ctx.env, &res)?),
                next: Some(next.create_ref()?),
                app: None,
                resolve: None,
                environment: None,
                source: None,
                bytes: Vec::new(),
                observation,
                deadline: envelope::now()? + config.timeout_ms,
            };
            collect(ctx.env, &req, pending, config.clone())?;
            trace("express:collecting");
            Ok(())
        };
        if run().is_err() {
            fail(&res, 503)?;
        }
        Ok(())
    })?;
    function(env, callback)
}

pub(crate) fn fetch(
    env: &Env,
    config: Configuration,
    app: Object<'_>,
    stream_module: Object<'_>,
) -> Result<Function<'static, (), Unknown<'static>>> {
    let routes: Vec<Object> = prop(&app, "routes")?;
    for route in routes {
        let path = text(&route, "path")?;
        if path == "/_humanymous" || path.starts_with("/_humanymous/") {
            return Err(error("reserved Hono route collision"));
        }
    }
    let config = Arc::new(config);
    let app = Rc::new(reference(env, &app)?);
    let streams = Rc::new(reference(env, &stream_module)?);
    let callback = env.create_function_from_closure("humanymousFetch", move |ctx| {
        let result = (|| -> Result<Unknown> {
            let admission =
                humanymous_framework_adapter::Admission::acquire().map_err(|e| error(e.0))?;
            let request: Object = ctx.get(0)?;
            if prop::<bool>(&request, "bodyUsed")? {
                return Err(error("guard must precede Fetch parsers"));
            }
            let signal: Object = prop(&request, "signal")?;
            if prop::<bool>(&signal, "aborted")? {
                return Err(error("request cancelled"));
            }
            let global = global(ctx.env)?;
            let url_ctor: Function<String, Unknown> = prop(&global, "URL")?;
            let url = url_ctor
                .new_instance(text(&request, "url")?)?
                .coerce_to_object()?;
            let headers: Object = prop(&request, "headers")?;
            let mut pairs_vec = Vec::new();
            let entries: Object = call(&headers, "entries", ())?;
            loop {
                let entry: Object = call(&entries, "next", ())?;
                if prop::<bool>(&entry, "done")? {
                    break;
                }
                let pair: Vec<String> = prop(&entry, "value")?;
                if pair.len() != 2 {
                    return Err(error("invalid Fetch headers"));
                }
                if pairs_vec.len() >= 100
                    || pairs_vec
                        .iter()
                        .map(|p: &[String; 2]| p[0].len() + p[1].len())
                        .sum::<usize>()
                        + pair[0].len()
                        + pair[1].len()
                        > 32768
                {
                    return Err(error("Fetch header budget"));
                }
                pairs_vec.push([pair[0].clone(), pair[1].clone()]);
            }
            let observation = Observation {
                method: text(&request, "method")?,
                target: format!("{}{}", text(&url, "pathname")?, text(&url, "search")?),
                authority: text(&url, "host")?,
                headers: pairs_vec,
                peer: None,
                connection: None,
                raw: false,
            };
            let environment = if ctx.length() > 1 {
                ctx.get::<Option<Object>>(1)?
                    .map(|v| reference(ctx.env, &v))
                    .transpose()?
            } else {
                None
            };
            let pending = Pending {
                _admission: admission,
                cancelled: Arc::new(AtomicBool::new(false)),
                request: reference(ctx.env, &request)?,
                response: None,
                next: None,
                app: Some(reference(ctx.env, &app.get_value(ctx.env)?)?),
                resolve: None,
                environment,
                source: None,
                bytes: Vec::new(),
                observation,
                deadline: envelope::now()? + config.timeout_ms,
            };
            let streams = streams.get_value(ctx.env)?;
            let readable = callable(&streams, "Readable")?;
            let source: Object = match prop::<Option<Object>>(&request, "body")? {
                Some(body) => call(&readable, "fromWeb", body)?,
                None => call(&readable, "from", Vec::<Buffer>::new())?,
            };
            let slot = Rc::new(RefCell::new(Some(pending)));
            let config = config.clone();
            let executor = ctx.env.create_function_from_closure::<(), (), _>(
                "humanymousFetchPromise",
                move |ctx| {
                    let resolve: Function<Object, Unknown> = ctx.get(0)?;
                    let Some(mut pending) = slot.borrow_mut().take() else {
                        return Err(error("promise executor reused"));
                    };
                    pending.resolve = Some(resolve.create_ref()?);
                    collect(ctx.env, &source, pending, config.clone())
                },
            )?;
            let promise: Function<Function<(), ()>, Unknown> = prop(&global, "Promise")?;
            promise.new_instance(executor)
        })();
        match result {
            Ok(value) => Ok(value),
            Err(_) => {
                let g = global(ctx.env)?;
                let promise = callable(&g, "Promise")?;
                call(&promise, "resolve", response(ctx.env, 503)?)
            }
        }
    })?;
    function(env, callback)
}

fn collect(
    env: &Env,
    source: &Object<'_>,
    mut pending: Pending,
    config: Arc<Configuration>,
) -> Result<()> {
    let _: Unknown = call(source, "pause", ())?;
    pending.source = Some(reference(env, source)?);
    let cancelled = pending.cancelled.clone();
    let state: State = Rc::new(RefCell::new(Some(pending)));
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let data_state = state.clone();
    let max = config.max_body;
    let data = env.create_function_from_closure::<(), (), _>("humanymousBody", move |ctx| {
        let chunk: Buffer = ctx.get(0)?;
        trace("body:data");
        let overflow = {
            let mut borrow = data_state.borrow_mut();
            if let Some(p) = borrow.as_mut() {
                if p.bytes.len() + chunk.len() > max {
                    true
                } else {
                    p.bytes.extend_from_slice(&chunk);
                    false
                }
            } else {
                false
            }
        };
        if overflow {
            terminate(ctx.env, &data_state, 503)?;
        }
        Ok(())
    })?;
    let _: Unknown = call(source, "on", FnArgs::from(("data".to_owned(), data)))?;
    for event in ["aborted", "error"] {
        let event_state = state.clone();
        let cancel_tx = cancel_tx.clone();
        let cancelled = cancelled.clone();
        let callback =
            env.create_function_from_closure::<(), (), _>("humanymousAbort", move |ctx| {
                cancelled.store(true, Ordering::Release);
                let _ = cancel_tx.send(true);
                terminate(ctx.env, &event_state, 503)
            })?;
        let _: Unknown = call(source, "once", FnArgs::from((event.to_owned(), callback)))?;
    }
    if let Some(p) = state.borrow().as_ref() {
        let cancel_tx = cancel_tx.clone();
        let cancelled = cancelled.clone();
        let closed =
            env.create_function_from_closure::<(), (), _>("humanymousClientClose", move |_| {
                cancelled.store(true, Ordering::Release);
                let _ = cancel_tx.send(true);
                Ok(())
            })?;
        if let Some(res) = &p.response {
            let _: Unknown = call(
                &res.get_value(env)?,
                "once",
                FnArgs::from(("close".to_owned(), closed)),
            )?;
        } else {
            let request = p.request.get_value(env)?;
            let signal: Object = prop(&request, "signal")?;
            let _: Unknown = call(
                &signal,
                "addEventListener",
                FnArgs::from(("abort".to_owned(), closed)),
            )?;
        }
    }
    let timeout_state = state.clone();
    let timeout_tx = cancel_tx.clone();
    let timeout_cancelled = cancelled.clone();
    let timer =
        env.create_function_from_closure::<(), (), _>("humanymousDeadline", move |ctx| {
            timeout_cancelled.store(true, Ordering::Release);
            let _ = timeout_tx.send(true);
            terminate(ctx.env, &timeout_state, 503)
        })?;
    let g = global(env)?;
    let _: Unknown = call(
        &g,
        "setTimeout",
        FnArgs::from((timer, config.timeout_ms as u32)),
    )?;
    let end_state = state.clone();
    let end =
        env.create_function_from_closure::<(), (), _>("humanymousBodyComplete", move |ctx| {
            let Some(mut pending) = end_state.borrow_mut().take() else {
                return Ok(());
            };
            let request = pending.request.get_value(ctx.env)?;
            trace("body:end");
            if pending.response.is_some()
                && !prop::<Vec<String>>(&request, "rawTrailers")?.is_empty()
            {
                return finish(ctx.env, pending, 503);
            }
            let wire = match envelope::encode(
                &config,
                &mut pending.observation,
                &pending.bytes,
                pending.deadline,
            ) {
                Ok(wire) => wire,
                Err(_) => return finish(ctx.env, pending, 503),
            };
            start_decision(ctx.env, pending, config.clone(), wire, cancel_rx.clone())?;
            Ok(())
        })?;
    let _: Unknown = call(source, "once", FnArgs::from(("end".to_owned(), end)))?;
    let _: Unknown = call(source, "resume", ())?;
    Ok(())
}
fn terminate(env: &Env, state: &State, status: u32) -> Result<()> {
    let pending = state.borrow_mut().take();
    if let Some(p) = pending {
        p.cancelled.store(true, Ordering::Release);
        if let Some(source) = &p.source {
            let _: Unknown = call(&source.get_value(env)?, "destroy", ())?;
        }
        finish(env, p, status)?;
    }
    Ok(())
}
fn finish(env: &Env, p: Pending, status: u32) -> Result<()> {
    if let Some(res) = &p.response {
        fail(&res.get_value(env)?, status)?;
    }
    if let Some(resolve) = &p.resolve {
        resolve.borrow_back(env)?.call(response(env, status)?)?;
    }
    p.release(env)
}
fn start_decision(
    env: &Env,
    pending: Pending,
    config: Arc<Configuration>,
    wire: Vec<u8>,
    mut cancellation: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let deadline = pending.deadline;
    trace("decision:scheduled");
    let _promise=env.spawn_future_with_callback(async move {
        if *cancellation.borrow(){return Ok(Err(error("cancelled")));}
        let future=async {
            use humanymous_authority::client::{ClientOptions,ExpectedBinding};
            let parsed=humanymous_integrity::adapter::parse(&wire,envelope::now()?).map_err(error)?;
            let (origin,revision,_)=parsed.origin_binding();
            let options=ClientOptions{endpoint:&config.endpoint,credential:config.credential(),server:&config.server,timeout_ms:config.timeout_ms,adapter:humanymous_integrity::adapter::AdapterProfile::Node};
            let expected=ExpectedBinding{id:parsed.id(),tenant:parsed.tenant(),origin,revision,deadline};
            humanymous_authority::client::decide(&options,&wire,expected).await.map_err(|_|error("authority unavailable"))
        };
        let result=tokio::select!{value=future=>value,_=cancellation.changed()=>Err(error("cancelled"))};
        Ok(result)
    },move |env,result| {
        trace("decision:callback");
        let expires=result.as_ref().map(|bound|bound.expires()).unwrap_or(0).min(deadline);
        let status=match result.and_then(|bound|bound.consume().map_err(|_|error("expired decision"))) {Ok(verdict)=>match verdict{humanymous_authority::client::Verdict::Allow=>200,humanymous_authority::client::Verdict::Deny=>403,humanymous_authority::client::Verdict::Challenge=>409},Err(_)=>503};
        if status==200 && envelope::now()?<expires && !pending.cancelled.load(Ordering::Acquire) {
            let req=pending.request.get_value(env)?;
            if let Some(res)=&pending.response && prop::<Option<bool>>(&res.get_value(env)?,"destroyed")?.unwrap_or(false) {return pending.release(env);}
            if pending.response.is_some() {
                replay(env,&req,&pending.bytes,&pending.observation.headers)?;
                if envelope::now()?>=expires||pending.cancelled.load(Ordering::Acquire){return finish(env,pending,503);}
                if let Some(next)=&pending.next {next.borrow_back(env)?.call(())?;}
            }else if let (Some(app),Some(resolve))=(&pending.app,&pending.resolve) {
                let g=global(env)?;let ctor:Function<FnArgs<(Object,Object)>,Unknown>=prop(&g,"Request")?;
                let mut init=object(env)?;init.set_named_property("method",pending.observation.method.clone())?;init.set_named_property("headers",pending.observation.headers.iter().map(|h|h.to_vec()).collect::<Vec<_>>())?;
                init.set_named_property("signal",prop::<Object>(&req,"signal")?)?;
                if !["GET","HEAD"].contains(&pending.observation.method.as_str()){init.set_named_property("body",Buffer::from(pending.bytes.clone()))?;}
                let authorized=ctor.new_instance(FnArgs::from((req,init)))?.coerce_to_object()?;
                if prop::<bool>(&prop::<Object>(&req,"signal")?,"aborted")? || envelope::now()?>=expires{return finish(env,pending,503);}
                let app=app.get_value(env)?;
                let value:Object=if let Some(environment)=&pending.environment {call(&app,"fetch",FnArgs::from((authorized,environment.get_value(env)?)))?}else{call(&app,"fetch",authorized)?};
                resolve.borrow_back(env)?.call(value)?;
            }
        }else {return finish(env,pending,if status==200 {503}else{status});}
        pending.release(env)
    })?;
    Ok(())
}
fn replay(env: &Env, request: &Object<'_>, body: &[u8], headers: &[[String; 2]]) -> Result<()> {
    let streams = builtin(env, "stream")?;
    let readable = callable(&streams, "Readable")?;
    let replay: Object = call(&readable, "from", vec![Buffer::from(body.to_vec())])?;
    let mut request = *request;
    for name in [
        "pipe",
        "unpipe",
        "read",
        "resume",
        "pause",
        "setEncoding",
        "isPaused",
    ] {
        let function = callable(&replay, name)?;
        let bound: Unknown = call(&function, "bind", replay)?;
        request.set_named_property(name, bound)?;
    }
    // Body events use the bounded replay. All other events retain the original
    // IncomingMessage emitter, and chaining still returns the original request.
    // Capture unbound original methods, not the request, to avoid native-root
    // cycles request -> callback -> request.
    for name in ["on", "once", "addListener", "removeListener", "off"] {
        let original: Function<FnArgs<(String, Unknown)>, Unknown> = prop(&request, name)?;
        let original = original.create_ref()?;
        let replay_ref = reference(env, &replay)?;
        let method = name.to_owned();
        let dispatch = env.create_function_from_closure::<(), Object, _>(
            "humanymousBodyEvents",
            move |ctx| {
                let event: String = ctx.get(0)?;
                let listener: Unknown = ctx.get(1)?;
                let this: Object = ctx.this()?;
                if ["data", "readable", "end"].contains(&event.as_str()) {
                    let replay = replay_ref.get_value(ctx.env)?;
                    let _: Unknown = call(&replay, &method, FnArgs::from((event, listener)))?;
                } else {
                    original
                        .borrow_back(ctx.env)?
                        .apply(this, FnArgs::from((event, listener)))?;
                }
                Ok(this)
            },
        )?;
        request.set_named_property(name, dispatch)?;
    }
    let mut normalized: Object = prop(&request, "headers")?;
    for key in Object::keys(&normalized)? {
        if envelope::internal(&key) {
            normalized.delete_named_property(&key)?;
        }
    }
    request.set_named_property(
        "rawHeaders",
        headers.iter().flatten().cloned().collect::<Vec<_>>(),
    )?;
    let global = global(env)?;
    let object_ctor = callable(&global, "Object")?;
    let symbol = callable(&global, "Symbol")?;
    let key: Unknown = prop(&symbol, "asyncIterator")?;
    let iterator: Unknown = replay.get_property(key)?;
    let iterator = iterator.coerce_to_object()?;
    let bound: Unknown = call(&iterator, "bind", replay)?;
    let key: Unknown = prop(&symbol, "asyncIterator")?;
    request.set_property(key, bound)?;
    for name in [
        "readable",
        "readableEnded",
        "readableFlowing",
        "readableLength",
    ] {
        let replay_ref = reference(env, &replay)?;
        let key = name.to_owned();
        let getter = env.create_function_from_closure::<(), Unknown, _>(
            "humanymousReplayState",
            move |ctx| {
                let object = replay_ref.get_value(ctx.env)?;
                object.get_named_property(&key)
            },
        )?;
        let mut descriptor = object(env)?;
        descriptor.set_named_property("get", getter)?;
        descriptor.set_named_property("configurable", true)?;
        let _: Unknown = call(
            &object_ctor,
            "defineProperty",
            FnArgs::from((request, name.to_owned(), descriptor)),
        )?;
    }
    Ok(())
}
