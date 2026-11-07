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
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\humanymous-ruby-hosted-no-service"
    } else {
        "/tmp/humanymous-ruby-hosted-no-service.sock"
    };
    let config=serde_json::json!({"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,
        "credential":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":{"kind":"unix","uid":0},"routes":["/api"],
        "timeoutMs":250,"maxBody":8,"syncInputDeadlineEnforced":true}).to_string();
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
