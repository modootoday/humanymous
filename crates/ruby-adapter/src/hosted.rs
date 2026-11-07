//! Hosted-only actual Rack/Sinatra/Rails dependency-down native callback probe.
use super::{call, wrap};
use magnus::{Error, RClass, RHash, RModule, Ruby, Value, method, prelude::*};
use std::sync::atomic::{AtomicUsize, Ordering};
static EFFECTS: AtomicUsize = AtomicUsize::new(0);
fn spy(ruby: &Ruby, receiver: Value, env: RHash) -> Result<Value, Error> {
    EFFECTS.fetch_add(1, Ordering::AcqRel);
    let inner: Value = receiver.ivar_get("@inner")?;
    if inner.is_nil() {
        return Err(super::failure(ruby, "unexpected protected origin call"));
    }
    inner.funcall("call", (env,))
}
fn liveness(ruby: &Ruby, receiver: Value) -> Result<Value, Error> {
    let state: String = receiver.ivar_get("@state")?;
    match state.as_str() {
        "connected" => Ok(ruby.qfalse().as_value()),
        "disconnected" => Ok(ruby.qtrue().as_value()),
        "nil" => Ok(ruby.qnil().as_value()),
        "truthy" => Ok(ruby.str_new("false").as_value()),
        _ => Err(super::failure(ruby, "owned liveness fixture error")),
    }
}
pub fn run_if_requested(ruby: &Ruby) -> Result<(), Error> {
    if std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref() != Ok("1") {
        return Ok(());
    }
    ruby.require("rack/lint")?;
    ruby.require("sinatra/base")?;
    ruby.require("rails/application")?;
    ruby.require("stringio")?;
    let module: RModule = ruby.class_object().const_get("Humanymous")?;
    let spy_class = module.define_class("HostedOriginSpy", ruby.class_object())?;
    spy_class.define_method("call", method!(spy, 1))?;
    let liveness_class = module.define_class("HostedLiveness", ruby.class_object())?;
    liveness_class.define_method("call", method!(liveness, 0))?;
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\humanymous-ruby-hosted-no-service"
    } else {
        "/tmp/humanymous-ruby-hosted-no-service.sock"
    };
    let config=serde_json::json!({"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,
        "credential":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":{"kind":"unix","uid":0},"routes":["/api"],
        "timeoutMs":250,"maxBody":8,"syncInputDeadlineEnforced":true}).to_string();
    for state in ["missing", "disconnected", "nil", "truthy", "error"] {
        let origin: Value = spy_class.new_instance(())?;
        let guarded = wrap(ruby, origin, config.clone())?;
        let env = ruby.hash_new();
        if state != "missing" {
            let check: Value = liveness_class.new_instance(())?;
            check.ivar_set("@state", state)?;
            env.aset("humanymous.client_disconnected", check)?;
        }
        let outcome = call(ruby, guarded, env);
        let rejected = if state == "disconnected" {
            outcome
                .and_then(|response| response.funcall::<_, _, u16>("[]", (0,)))
                .is_ok_and(|status| status == 503)
        } else {
            outcome.is_err()
        };
        if !rejected || EFFECTS.load(Ordering::Acquire) != 0 {
            return Err(super::failure(
                ruby,
                "unknown/disconnected liveness reached protected origin",
            ));
        }
    }
    println!("humanymous-ruby-hosted group=liveness-negative cases=5 origin_effects=0");
    for framework in ["rack", "sinatra", "rails"] {
        let app: Value = match framework {
            "rack" => {
                let rack: RModule = ruby.class_object().const_get("Rack")?;
                let lint: RClass = rack.const_get("Lint")?;
                let base: Value = spy_class.new_instance(())?;
                lint.new_instance((base,))?
            }
            "sinatra" => {
                let sinatra: RModule = ruby.class_object().const_get("Sinatra")?;
                let base: RClass = sinatra.const_get("Base")?;
                base.new_instance(())?
            }
            _ => {
                let rails: RModule = ruby.class_object().const_get("Rails")?;
                let base: RClass = rails.const_get("Application")?;
                base.new_instance(())?
            }
        };
        let observed: Value = spy_class.new_instance(())?;
        observed.ivar_set("@inner", app)?;
        let guarded = wrap(ruby, observed, config.clone())?;
        let env = ruby.hash_new();
        let check: Value = liveness_class.new_instance(())?;
        check.ivar_set("@state", "connected")?;
        env.aset("humanymous.client_disconnected", check)?;
        if super::liveness::disconnected(ruby, env)? {
            return Err(super::failure(
                ruby,
                "connected fixture must report exact false",
            ));
        }
        env.aset("REQUEST_METHOD", "POST")?;
        env.aset("REQUEST_URI", "/api?binary=1")?;
        env.aset("HTTP_HOST", "example.test")?;
        env.aset("SERVER_PROTOCOL", "HTTP/1.1")?;
        env.aset("CONTENT_LENGTH", "2")?;
        env.aset("rack.hijack?", false)?;
        let io: RClass = ruby.class_object().const_get("StringIO")?;
        let input: Value = io.new_instance((ruby.str_from_slice(&[0, 255]),))?;
        env.aset("rack.input", input)?;
        let response = call(ruby, guarded, env)?;
        let status: u16 = response.funcall("[]", (0,))?;
        if status != 503 || EFFECTS.load(Ordering::Acquire) != 0 {
            return Err(super::failure(
                ruby,
                "dependency-down invoked protected framework",
            ));
        }
        println!(
            "humanymous-ruby-hosted framework={framework} status=503 origin_effects=0 profile=dependency-unavailable"
        );
    }
    Ok(())
}
