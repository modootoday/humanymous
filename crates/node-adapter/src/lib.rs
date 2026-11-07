//! Rust-owned Node-API framework lifecycle bridge to the shared Rust authority.
//! No scoring, credential verification or security receipt implementation lives here.
mod admission;
mod envelope;
mod host;
mod server;
mod stream;
mod template;

#[cfg(all(feature = "host-pilot", not(test)))]
mod pilot;

use napi::{Env, bindgen_prelude::*};
use napi_derive::napi;

/// Create root-mounted Express middleware before every body parser and effect.
#[napi(ts_return_type = "(request: object, response: object, next: () => void) => void")]
pub fn express(env: Env, options: String) -> Result<Function<'static, (), ()>> {
    stream::express(&env, envelope::Configuration::parse(&options)?)
}

/// Create the Node HTTP server around an existing Express app before its parsers
/// and reject unsupported upgrade/CONNECT events without origin effects. Do not
/// add another request/upgrade/connect listener or mutate protected routing later.
#[napi]
pub fn create_express_server(
    env: Env,
    options: String,
    app: Object<'_>,
) -> Result<Object<'static>> {
    server::express_server(&env, envelope::Configuration::parse(&options)?, app)
}

/// Create an outer Fetch handler around a Hono app with an explicit route inventory.
#[napi(ts_return_type = "(request: Request, env?: object) => Promise<Response>")]
pub fn hono(
    env: Env,
    options: String,
    app: Object<'_>,
    stream_module: Object<'_>,
) -> Result<Function<'static, (), Unknown<'static>>> {
    stream::fetch(
        &env,
        envelope::Configuration::parse(&options)?,
        app,
        stream_module,
    )
}

pub use template::{HtmlHook, html_hook};

/// Explicit supported observation/lifecycle contract; compilation does not
/// promote an OS/runtime tuple or manufacture original-client network evidence.
#[napi]
pub fn capabilities(env: Env) -> Result<String> {
    let bun = host::prop::<Option<Object>>(&host::global(&env)?, "Bun")?.is_some();
    Ok(serde_json::json!({"protocol":"humanymous.adapter/0.1","adapter":"humanymous.node","implementation":"rust-node-api","expressObservation":if bun {"fetch-normalized"} else {"node-raw"},"expressRawHeaderOrder":!bun,"expressRawHeaderCasing":!bun,"fetchRawHeaderOrder":false,"rawTls":false,"originalH2":false,"capture":false,"administration":false,"completeBodyLimit":1048576,"pendingLimit":64,"uploadStreaming":false,"websocket":false,"grpc":false,"responseStreaming":"handler-owned-unmodified","html":"explicit-template-only","releaseQualifiedTuples":[]}).to_string())
}
