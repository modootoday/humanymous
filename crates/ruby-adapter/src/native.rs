//! Native Rack callback. Ruby callback state lives in VM-managed ivars.
use humanymous_authority::client::Verdict;
use humanymous_framework_adapter::{
    Admission, Configuration, Observation, Pending, body_length, internal_header,
};
use humanymous_integrity::adapter::AdapterProfile;
use magnus::{Error, RClass, RHash, RString, Ruby, Value, function, method, prelude::*};
use std::sync::Arc;
mod liveness;
use crate::worker::DecisionWorker;
#[cfg(feature = "host-pilot")]
#[path = "hosted.rs"]
mod hosted;
#[cfg(feature = "package-tool")]
#[path = "package.rs"]
mod package;

fn failure(ruby: &Ruby, message: impl std::fmt::Display) -> Error {
    Error::new(ruby.exception_runtime_error(), message.to_string())
}
fn capabilities() -> String {
    serde_json::json!({"adapter":"humanymous.ruby","version":"0.1.0-experimental.1",
        "implementation":"rust-magnus","profiles":["rack-normalized"],
        "rawTls":false,"originalH2":false,"rawHeaderOrder":false,"capture":false,
        "requestStreaming":false,"websocket":false,"administration":false,
        "hijack":false,
        "disconnectCallbackRequired":true,
        "maximumBodyBytes":1048576,"maximumPending":64,
        "hostPilotEnabled":cfg!(feature="host-pilot"),"packageToolEnabled":cfg!(feature="package-tool"),
        "releaseQualifiedTuples":[]}).to_string()
}
fn wrap(ruby: &Ruby, app: Value, options: String) -> Result<Value, Error> {
    let c = Configuration::parse(&options).map_err(|e| failure(ruby, e))?;
    if !c.sync_input_deadline_enforced {
        return Err(failure(ruby, "server input deadline required"));
    }
    let module: magnus::RModule = ruby.class_object().const_get("Humanymous")?;
    let class: RClass = module.const_get("Rack")?;
    let wrapper: Value = class.new_instance(())?;
    // VM ivars root app/config across GC and compaction. No Ruby Value in Rust heap.
    wrapper.ivar_set("@app", app)?;
    wrapper.ivar_set("@options", options)?;
    Ok(wrapper)
}
fn call(ruby: &Ruby, wrapper: Value, original: RHash) -> Result<Value, Error> {
    let options: String = wrapper.ivar_get("@options")?;
    let c = Arc::new(Configuration::parse(&options).map_err(|e| failure(ruby, e))?);
    let _admission = Admission::acquire().map_err(|e| failure(ruby, e))?;
    let pending = Arc::new(Pending::new(&c).map_err(|e| failure(ruby, e))?);
    let env: RHash = original.funcall("dup", ())?;
    if liveness::disconnected(ruby, env)? {
        return reject(ruby, 503);
    }
    let method: String = env.fetch("REQUEST_METHOD")?;
    let target: String = env.fetch("REQUEST_URI")?;
    let authority: String = env.fetch("HTTP_HOST")?;
    let protocol: String = env.fetch("SERVER_PROTOCOL")?;
    if !["HTTP/1.0", "HTTP/1.1"].contains(&protocol.as_str()) || !c.permits_target(&target) {
        return reject(ruby, 503);
    }
    // Capability presence does not mean this request invoked hijack. Disable
    // the unsupported extension in the application-visible environment copy.
    env.aset("rack.hijack?", false)?;
    env.delete::<_, Value>("rack.hijack")?;
    let length = body_length(&env.fetch::<_, String>("CONTENT_LENGTH")?, c.max_body)
        .map_err(|e| failure(ruby, e))?;
    let mut headers = Vec::new();
    let mut remove = Vec::new();
    let mut unsupported = false;
    env.foreach(|key: String, value: Value| {
        if let Some(name) = key.strip_prefix("HTTP_") {
            let name = name.replace('_', "-").to_ascii_lowercase();
            if internal_header(&name) {
                remove.push(key);
            } else {
                if [
                    "upgrade",
                    "trailer",
                    "transfer-encoding",
                    "content-length",
                    "content-type",
                ]
                .contains(&name.as_str())
                {
                    unsupported = true;
                }
                headers.push([name, String::try_convert(value)?]);
            }
        }
        Ok(magnus::r_hash::ForEach::Continue)
    })?;
    if unsupported {
        return reject(ruby, 503);
    }
    for name in remove {
        env.delete::<_, Value>(name)?;
    }
    headers.retain(|[n, _]| n != "content-length" && n != "content-type");
    headers.push(["content-length".into(), length.to_string()]);
    if let Some(value) = env.get("CONTENT_TYPE") {
        headers.push(["content-type".into(), String::try_convert(value)?]);
    }
    let input: Value = env.fetch("rack.input")?;
    let mut body = Vec::with_capacity(length);
    while body.len() < length {
        pending.remaining().map_err(|e| failure(ruby, e))?;
        let part: RString = input.funcall("read", ((length - body.len()).min(65536),))?;
        let bytes = part.to_bytes();
        if bytes.is_empty() || bytes.len() > length - body.len() {
            return reject(ruby, 503);
        }
        body.extend_from_slice(&bytes);
    }
    let observation = Observation {
        method,
        target,
        authority,
        headers,
        peer: None,
        connection: None,
        raw: false,
    };
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
    let decision_pending = pending.clone();
    let decision_body = body.clone();
    // Worker owns Rust values only. Ruby waits through upstream thread_sleep,
    // yielding GVL without handwritten unsafe. The request cancels and joins it.
    let thread = std::thread::Builder::new().name("humanymous-rack-decision".into()).spawn(move || {
        let result=(||{
            let rt=tokio::runtime::Builder::new_current_thread().enable_all().build()
                .map_err(|_|humanymous_framework_adapter::Error("adapter runtime unavailable"))?;
            rt.block_on(async {tokio::select! {
                decision=decision_pending.decide(&c,&observation,&decision_body,AdapterProfile::RubyRack)=>decision,
                _=cancelled.changed()=>Err(humanymous_framework_adapter::Error("request cancelled")),
            }})
        })();
        let _=tx.send(result);
    }).map_err(|_|failure(ruby,"adapter worker unavailable"))?;
    let worker = DecisionWorker::new(cancel, thread);
    let result = loop {
        match rx.try_recv() {
            Ok(result) => break result,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => return reject(ruby, 503),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                if liveness::disconnected(ruby, env)? {
                    return reject(ruby, 503);
                }
                let remaining = pending.remaining().map_err(|e| failure(ruby, e))?;
                ruby.thread_sleep(remaining.min(std::time::Duration::from_millis(2)))?;
            }
        }
    };
    drop(worker);
    let bound = match result {
        Ok(v) => v,
        Err(_) => return reject(ruby, 503),
    };
    let expires = bound.expires();
    let verdict = match bound.consume() {
        Ok(v) => v,
        Err(_) => return reject(ruby, 503),
    };
    if verdict != Verdict::Allow {
        return reject(ruby, if verdict == Verdict::Deny { 403 } else { 409 });
    }
    ruby.require("stringio")?;
    let io: RClass = ruby.class_object().const_get("StringIO")?;
    let binary = ruby.str_from_slice(&body);
    let replay: Value = io.new_instance((binary,))?;
    env.aset("rack.input", replay)?;
    let app: Value = wrapper.ivar_get("@app")?;
    pending
        .check_handoff(expires)
        .map_err(|e| failure(ruby, e))?;
    if liveness::disconnected(ruby, env)? {
        return reject(ruby, 503);
    }
    app.funcall("call", (env,))
}
fn reject(ruby: &Ruby, status: u16) -> Result<Value, Error> {
    let headers = ruby.hash_new();
    headers.aset("content-type", "application/json")?;
    headers.aset("content-length", "2")?;
    headers.aset("cache-control", "no-store")?;
    let body = ruby.ary_new();
    body.push(ruby.str_new("{}"))?;
    let response = ruby.ary_new();
    response.push(status)?;
    response.push(headers)?;
    response.push(body)?;
    Ok(response.as_value())
}
#[magnus::init(name = "humanymous_ruby")]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Humanymous")?;
    let class = module.define_class("Rack", ruby.class_object())?;
    class.define_method("call", method!(call, 1))?;
    module.define_singleton_method("wrap", function!(wrap, 2))?;
    module.define_singleton_method("capabilities", function!(capabilities, 0))?;
    if std::env::var("HUMANYMOUS_NATIVE_CAPABILITY_RECEIPT").as_deref() == Ok("1") {
        println!("{}", capabilities());
    }
    #[cfg(feature = "package-tool")]
    module.define_singleton_method("build_native_gem", function!(package::build, 2))?;
    #[cfg(feature = "package-tool")]
    package::from_private_request(ruby)?;
    #[cfg(feature = "host-pilot")]
    hosted::run_if_requested(ruby)?;
    Ok(())
}
