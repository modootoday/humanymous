//! Safe host calls: no eval or authored foreign-language source.
use crate::envelope::error;
use napi::{Env, Result, bindgen_prelude::*};
pub(crate) fn trace(message: &str) {
    #[cfg(feature = "host-pilot")]
    if std::env::var_os("HUMANYMOUS_NODE_TRACE").is_some() {
        eprintln!("native-host-phase:{message}");
    }
    let _ = message;
}

pub(crate) fn call<'a, A: JsValuesTupleIntoVec, R: FromNapiValue>(
    object: &Object<'a>,
    name: &str,
    args: A,
) -> Result<R> {
    let method: Function<'_, A, R> = object.get_named_property(name)?;
    method.apply(object, args)
}
pub(crate) fn object(env: &Env) -> Result<Object<'_>> {
    Object::new(env)
}
pub(crate) fn prop<T: FromNapiValue + ValidateNapiValue>(
    object: &Object<'_>,
    name: &str,
) -> Result<T> {
    object.get_named_property(name)
}
pub(crate) fn global(env: &Env) -> Result<Object<'_>> {
    env.get_global()?.get_named_property("globalThis")
}
pub(crate) fn text(object: &Object<'_>, name: &str) -> Result<String> {
    prop(object, name)
}
pub(crate) fn callable<'a>(object: &Object<'a>, name: &str) -> Result<Object<'a>> {
    let value: Unknown = prop(object, name)?;
    if value.get_type()? != napi::ValueType::Function {
        return Err(error("host callable required"));
    }
    value.coerce_to_object()
}
pub(crate) fn builtin<'a>(env: &'a Env, name: &str) -> Result<Object<'a>> {
    let g = global(env)?;
    let process: Object = prop(&g, "process")?;
    call(&process, "getBuiltinModule", name.to_owned())
}
pub(crate) fn response(env: &Env, status: u32) -> Result<Object<'static>> {
    let g = global(env)?;
    let constructor: Function<FnArgs<(String, Object)>, Unknown> = prop(&g, "Response")?;
    let mut init = object(env)?;
    init.set_named_property("status", status)?;
    let mut headers = object(env)?;
    headers.set_named_property("cache-control", "no-store")?;
    headers.set_named_property("content-type", "application/json")?;
    init.set_named_property("headers", headers)?;
    constructor
        .new_instance((error_body(status), init).into())?
        .coerce_to_object()
}
pub(crate) fn fail(res: &Object<'_>, status: u32) -> Result<()> {
    trace("reject:attempt");
    if prop::<bool>(res, "headersSent").unwrap_or(true) {
        return Ok(());
    }
    trace("reject:headers-open");
    let _: Unknown = call(
        res,
        "setHeader",
        FnArgs::from(("cache-control".to_owned(), "no-store".to_owned())),
    )?;
    trace("reject:cache-set");
    let _: Unknown = call(
        res,
        "setHeader",
        FnArgs::from(("content-type".to_owned(), "application/json".to_owned())),
    )?;
    let _: Unknown = call(res, "writeHead", FnArgs::from((status,)))?;
    trace("reject:head-written");
    let _: Unknown = call(res, "end", error_body(status))?;
    trace("reject:ended");
    Ok(())
}
fn error_body(status: u32) -> String {
    match status {
        403 => "{\"verdict\":\"DENY\"}",
        409 => "{\"verdict\":\"CHALLENGE\"}",
        _ => "{\"error\":\"authority_unavailable\"}",
    }
    .to_owned()
}
pub(crate) struct Lease {
    inner: Option<ObjectRef>,
    env: Env,
}
impl Lease {
    pub fn get_value<'a>(&self, env: &'a Env) -> Result<Object<'a>> {
        self.inner
            .as_ref()
            .ok_or_else(|| error("released host lease"))?
            .get_value(env)
    }
    pub fn unref(self, _: &Env) -> Result<()> {
        drop(self);
        Ok(())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            let _ = inner.unref(&self.env);
        }
    }
}
pub(crate) fn reference(env: &Env, object: &Object<'_>) -> Result<Lease> {
    Ok(Lease {
        inner: Some(object.create_ref()?),
        env: *env,
    })
}
pub(crate) fn function<A: JsValuesTupleIntoVec, R>(
    env: &Env,
    f: Function<'_, A, R>,
) -> Result<Function<'static, A, R>> {
    let mut holder = Object::new(env)?;
    holder.set_named_property("callback", f)?;
    holder.get_named_property("callback")
}
pub(crate) fn pairs(flat: Vec<String>) -> Result<Vec<[String; 2]>> {
    if !flat.len().is_multiple_of(2) {
        return Err(error("invalid raw header pairs"));
    }
    Ok(flat
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| [v[0].clone(), v[1].clone()])
        .collect())
}
