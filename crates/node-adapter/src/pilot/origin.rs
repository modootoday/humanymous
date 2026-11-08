//! Unchanged upstream frameworks with Rust-authored application fixtures.
use crate::{envelope::error, host::*};
use napi::{Env, Result, bindgen_prelude::*};
use std::{cell::Cell, io::Write, path::PathBuf, rc::Rc};

fn effect(path: &Option<PathBuf>) -> Result<()> {
    if let Some(path) = path {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(b"effect\n"))
            .map_err(|_| error("fixture origin receipt"))?;
    }
    Ok(())
}
pub(super) fn express(
    env: &Env,
    app: &Object<'_>,
    module: &Object<'_>,
    effects: Option<PathBuf>,
) -> Result<()> {
    let middleware =
        env.create_function_from_closure::<(), Unknown, _>("fixtureEffect", move |ctx| {
            effect(&effects)?;
            let next: Function<(), Unknown> = ctx.get(2)?;
            next.call(())
        })?;
    let _: Unknown = call(app, "use", middleware)?;
    let iter =
        env.create_function_from_closure::<(), (), _>("fixtureAsyncIterator", move |ctx| {
            let req: Object = ctx.get(0)?;
            let res: Object = ctx.get(1)?;
            let res = reference(ctx.env, &res)?;
            let consumers = builtin(ctx.env, "stream/consumers")?;
            let promise: PromiseRaw<Buffer> = call(&consumers, "buffer", req)?;
            promise.then(move |ctx| {
                let res = res.get_value(&ctx.env)?;
                let _: Unknown = call(&res, "writeHead", 201u32)?;
                let _: Unknown = call(&res, "end", ctx.value)?;
                Ok(())
            })?;
            Ok(())
        })?;
    let _: Unknown = call(app, "post", FnArgs::from(("/iter".to_owned(), iter)))?;
    let mut options = object(env)?;
    options.set_named_property("type", "*/*")?;
    let parser: Object = call(module, "raw", options)?;
    let _: Unknown = call(app, "use", parser)?;
    let echo = env.create_function_from_closure::<(), (), _>("fixtureEcho", move |ctx| {
        let req: Object = ctx.get(0)?;
        let res: Object = ctx.get(1)?;
        let body: Buffer = prop(&req, "body")?;
        let mut headers = object(ctx.env)?;
        headers.set_named_property("x-origin-effect", "one")?;
        headers.set_named_property("set-cookie", vec!["a=1; Path=/", "b=2; Path=/"])?;
        let _: Unknown = call(&res, "writeHead", FnArgs::from((201u32, headers)))?;
        let _: Unknown = call(&res, "end", body)?;
        Ok(())
    })?;
    let _: Unknown = call(app, "post", FnArgs::from(("/api".to_owned(), echo)))?;
    for (path, status) in [("/redirect", 302u32), ("/empty", 204u32)] {
        let handler =
            env.create_function_from_closure::<(), (), _>("fixtureStatus", move |ctx| {
                let req: Object = ctx.get(0)?;
                let res: Object = ctx.get(1)?;
                let status = if status == 204 && text(&req, "url")?.contains("not-modified") {
                    304
                } else {
                    status
                };
                if status == 302 {
                    let _: Unknown = call(
                        &res,
                        "setHeader",
                        FnArgs::from(("location".to_owned(), "/api?q=kept".to_owned())),
                    )?;
                }
                if status == 304 {
                    let _: Unknown = call(
                        &res,
                        "setHeader",
                        FnArgs::from(("etag".to_owned(), "\"unchanged\"".to_owned())),
                    )?;
                }
                let _: Unknown = call(&res, "writeHead", status)?;
                let _: Unknown = call(&res, "end", ())?;
                Ok(())
            })?;
        let _: Unknown = call(app, "post", FnArgs::from((path.to_owned(), handler)))?;
        if status == 204 {
            let _: Unknown = call(app, "head", FnArgs::from((path.to_owned(), handler)))?;
        }
    }
    let sse = env.create_function_from_closure::<(), (), _>("fixtureSse", move |ctx| {
        let res: Object = ctx.get(1)?;
        let _: Unknown = call(
            &res,
            "setHeader",
            FnArgs::from(("content-type".to_owned(), "text/event-stream".to_owned())),
        )?;
        let _: Unknown = call(&res, "writeHead", 200u32)?;
        let _: Unknown = call(&res, "write", "data: first\n\n".to_owned())?;
        let cancelled = Rc::new(Cell::new(false));
        let closed = cancelled.clone();
        let callback =
            ctx.env
                .create_function_from_closure::<(), (), _>("fixtureSseClosed", move |_| {
                    closed.set(true);
                    Ok(())
                })?;
        let _: Unknown = call(&res, "once", FnArgs::from(("close".to_owned(), callback)))?;
        let res = reference(ctx.env, &res)?;
        let finish =
            ctx.env
                .create_function_from_closure::<(), (), _>("fixtureSseEnd", move |ctx| {
                    if !cancelled.get() {
                        let _: Unknown = call(
                            &res.get_value(ctx.env)?,
                            "end",
                            "data: second\n\n".to_owned(),
                        )?;
                    }
                    Ok(())
                })?;
        let _: Unknown = call(
            &global(ctx.env)?,
            "setTimeout",
            FnArgs::from((finish, 500u32)),
        )?;
        Ok(())
    })?;
    let _: Unknown = call(app, "post", FnArgs::from(("/sse".to_owned(), sse)))?;
    Ok(())
}
pub(super) fn hono(env: &Env, app: &Object<'_>, effects: Option<PathBuf>) -> Result<()> {
    let middleware =
        env.create_function_from_closure::<(), Unknown, _>("fixtureEffect", move |ctx| {
            effect(&effects)?;
            let next: Function<(), Unknown> = ctx.get(1)?;
            next.call(())
        })?;
    let _: Unknown = call(app, "use", FnArgs::from(("*".to_owned(), middleware)))?;
    for path in ["/api", "/iter"] {
        let echo =
            env.create_function_from_closure::<(), Object, _>("fixtureFetchEcho", move |ctx| {
                let c: Object = ctx.get(0)?;
                let req: Object = prop(&c, "req")?;
                let raw: Object = prop(&req, "raw")?;
                let body: Unknown = prop(&raw, "body")?;
                web_response(ctx.env, body, 201)
            })?;
        let _: Unknown = call(app, "post", FnArgs::from((path.to_owned(), echo)))?;
    }
    for (path, status) in [("/redirect", 302u32), ("/empty", 204u32)] {
        let handler =
            env.create_function_from_closure::<(), Object, _>("fixtureStatus", move |ctx| {
                let c: Object = ctx.get(0)?;
                let req: Object = prop(&c, "req")?;
                let status = if status == 204 && text(&req, "url")?.contains("not-modified") {
                    304
                } else {
                    status
                };
                if status == 302 {
                    call(
                        &c,
                        "redirect",
                        FnArgs::from(("/api?q=kept".to_owned(), 302u32)),
                    )
                } else {
                    let g = global(ctx.env)?;
                    let ctor: Function<FnArgs<((), Object)>, Unknown> = prop(&g, "Response")?;
                    let mut init = object(ctx.env)?;
                    init.set_named_property("status", status)?;
                    if status == 304 {
                        let mut headers = object(ctx.env)?;
                        headers.set_named_property("etag", "\"unchanged\"")?;
                        init.set_named_property("headers", headers)?;
                    }
                    ctor.new_instance(FnArgs::from(((), init)))?
                        .coerce_to_object()
                }
            })?;
        let _: Unknown = call(app, "post", FnArgs::from((path.to_owned(), handler)))?;
        if status == 204 {
            // Hono rewrites HEAD to GET before route matching, so app.on("HEAD") never
            // runs (honojs/hono#5506). Register GET so HEAD /empty reaches this 204 fixture.
            let _: Unknown = call(app, "get", FnArgs::from((path.to_owned(), handler)))?;
        }
    }
    let sse = env.create_function_from_closure::<(), Object, _>("fixtureFetchSse", move |ctx| {
        let state = Rc::new(Cell::new(0u32));
        let cancelled = Rc::new(Cell::new(false));
        let cancel = cancelled.clone();
        let mut source = object(ctx.env)?;
        let pull =
            ctx.env
                .create_function_from_closure::<(), (), _>("fixtureSsePull", move |ctx| {
                    let controller: Object = ctx.get(0)?;
                    match state.get() {
                        0 => {
                            state.set(1);
                            let _: Unknown = call(
                                &controller,
                                "enqueue",
                                Buffer::from(b"data: first\n\n".to_vec()),
                            )?;
                        }
                        1 => {
                            state.set(2);
                            let controller = reference(ctx.env, &controller)?;
                            let cancelled = cancelled.clone();
                            let finish = ctx.env.create_function_from_closure::<(), (), _>(
                                "fixtureSseEnd",
                                move |ctx| {
                                    if !cancelled.get() {
                                        let controller = controller.get_value(ctx.env)?;
                                        let _: Unknown = call(
                                            &controller,
                                            "enqueue",
                                            Buffer::from(b"data: second\n\n".to_vec()),
                                        )?;
                                        let _: Unknown = call(&controller, "close", ())?;
                                    }
                                    Ok(())
                                },
                            )?;
                            let _: Unknown = call(
                                &global(ctx.env)?,
                                "setTimeout",
                                FnArgs::from((finish, 500u32)),
                            )?;
                        }
                        _ => {}
                    }
                    Ok(())
                })?;
        source.set_named_property("pull", pull)?;
        let stop =
            ctx.env
                .create_function_from_closure::<(), (), _>("fixtureSseCancel", move |_| {
                    cancel.set(true);
                    Ok(())
                })?;
        source.set_named_property("cancel", stop)?;
        let g = global(ctx.env)?;
        let ctor: Function<Object, Unknown> = prop(&g, "ReadableStream")?;
        let stream = ctor.new_instance(source)?;
        let ctor: Function<FnArgs<(Unknown, Object)>, Unknown> = prop(&g, "Response")?;
        let mut init = object(ctx.env)?;
        let mut headers = object(ctx.env)?;
        headers.set_named_property("content-type", "text/event-stream")?;
        init.set_named_property("headers", headers)?;
        ctor.new_instance(FnArgs::from((stream, init)))?
            .coerce_to_object()
    })?;
    let _: Unknown = call(app, "post", FnArgs::from(("/sse".to_owned(), sse)))?;
    Ok(())
}
fn web_response(env: &Env, body: Unknown<'_>, status: u32) -> Result<Object<'static>> {
    let g = global(env)?;
    let ctor: Function<FnArgs<(Unknown, Object)>, Unknown> = prop(&g, "Response")?;
    let mut init = object(env)?;
    init.set_named_property("status", status)?;
    let mut headers = object(env)?;
    headers.set_named_property("x-origin-effect", "one")?;
    init.set_named_property("headers", headers)?;
    ctor.new_instance(FnArgs::from((body, init)))?
        .coerce_to_object()
}
