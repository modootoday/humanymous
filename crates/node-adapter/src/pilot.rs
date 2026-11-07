//! Rust-authored host process fixture. Upstream framework modules are loaded by
//! their real module loader; no foreign test source is generated or evaluated.
use crate::{
    envelope::{Configuration, error},
    host::*,
    stream,
};
use napi::{Env, Result, bindgen_prelude::*};
use napi_derive::napi;
use serde::Deserialize;
use std::io::Read;
mod origin;

#[derive(Deserialize)]
struct Pilot {
    config: serde_json::Value,
    framework: String,
    package_json: String,
    effect_file: Option<std::path::PathBuf>,
}

#[napi(module_exports)]
fn initialize(_exports: Object<'_>, env: Env) -> Result<()> {
    if std::env::var_os("HUMANYMOUS_NODE_HOST_PILOT").is_none() {
        return Ok(());
    }
    let mut input = String::new();
    std::io::stdin()
        .take(16384)
        .read_to_string(&mut input)
        .map_err(|_| error("private pilot input"))?;
    let options: Pilot = serde_json::from_str(&input).map_err(|_| error("pilot configuration"))?;
    let config = Configuration::parse(&options.config.to_string())?;
    let module = builtin(&env, "module")?;
    let require: Function<String, Object> = call(&module, "createRequire", options.package_json)?;
    let server = if options.framework == "express" {
        let express = require.call("express".to_owned())?;
        let factory: Function<(), Object> = {
            let mut h = object(&env)?;
            h.set_named_property("f", express)?;
            prop(&h, "f")?
        };
        let app = factory.call(())?;
        origin::express(&env, &app, &express, options.effect_file.clone())?;
        crate::server::express_server(&env, config, app)?
    } else if options.framework == "hono" {
        let module = require.call("hono".to_owned())?;
        let constructor: Function<(), Unknown> = prop(&module, "Hono")?;
        let app = constructor.new_instance(())?.coerce_to_object()?;
        origin::hono(&env, &app, options.effect_file.clone())?;
        let streams = builtin(&env, "stream")?;
        let handler = stream::fetch(&env, config, app, streams)?;
        let g = global(&env)?;
        if let Some(bun) = prop::<Option<Object>>(&g, "Bun")? {
            let mut options = object(&env)?;
            options.set_named_property("fetch", handler)?;
            options.set_named_property("port", 0u32)?;
            options.set_named_property("hostname", "127.0.0.1")?;
            let server: Object = call(&bun, "serve", options)?;
            println!(
                "{}",
                serde_json::json!({"kind":"rust-node-api-host","port":prop::<u32>(&server,"port")?,"pid":std::process::id(),"host":"Bun.serve"})
            );
            return Ok(());
        }
        let node = require.call("@hono/node-server".to_owned())?;
        let mut options = object(&env)?;
        options.set_named_property("fetch", handler)?;
        options.set_named_property("port", 0u32)?;
        options.set_named_property("hostname", "127.0.0.1")?;
        let diagnostic =
            env.create_function_from_closure::<(), Object, _>("pilotHonoError", move |ctx| {
                let value: Unknown = ctx.get(0)?;
                eprintln!(
                    "native Hono pilot error: {}",
                    value.coerce_to_string()?.into_utf8()?.as_str()?
                );
                response(ctx.env, 500)
            })?;
        options.set_named_property("errorHandler", diagnostic)?;
        let listener: Object = call(
            &node,
            "getRequestListener",
            FnArgs::from((handler, options)),
        )?;
        let http = builtin(&env, "http")?;
        call::<_, Object>(&http, "createServer", listener)?
    } else {
        return Err(error("unknown framework pilot"));
    };
    let server_ref = reference(&env, &server)?;
    let listening=env.create_function_from_closure::<(),(),_>("pilotListening",move |ctx|{
        let server=server_ref.get_value(ctx.env)?;let address:Object=call(&server,"address",())?;
        println!("{}",serde_json::json!({"kind":"rust-node-api-host","port":prop::<u32>(&address,"port")?,"pid":std::process::id()}));Ok(())
    })?;
    let _: Unknown = call(
        &server,
        "once",
        FnArgs::from(("listening".to_owned(), listening)),
    )?;
    if ["express", "hono"].contains(&options.framework.as_str()) {
        let _: Unknown = call(
            &server,
            "listen",
            FnArgs::from((0u32, "127.0.0.1".to_owned())),
        )?;
    }
    Ok(())
}
