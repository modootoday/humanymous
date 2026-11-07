//! Node server ownership for protected Express applications. Unsupported upgrade
//! and CONNECT events are rejected before any application request callback.
use crate::{envelope::Configuration, host::*, stream};
use napi::{Env, Result, bindgen_prelude::*};
use std::rc::Rc;
type ExpressGuard<'a> = Function<'a, FnArgs<(Object<'a>, Object<'a>, Function<'a, (), ()>)>, ()>;

pub(crate) fn express_server(
    env: &Env,
    config: Configuration,
    app: Object<'_>,
) -> Result<Object<'static>> {
    let guard = stream::express(env, config)?;
    let mut holder = object(env)?;
    holder.set_named_property("guard", guard)?;
    let guard: ExpressGuard = holder.get_named_property("guard")?;
    let guard = guard.create_ref()?;
    let app = Rc::new(reference(env, &app)?);
    let request =
        env.create_function_from_closure::<(), (), _>("humanymousExpressServer", move |ctx| {
            let req: Object = ctx.get(0)?;
            let res: Object = ctx.get(1)?;
            let result = (|| -> Result<()> {
                let req_ref = reference(ctx.env, &req)?;
                let res_ref = reference(ctx.env, &res)?;
                let app = app.clone();
                let next = ctx.env.create_function_from_closure::<(), (), _>(
                    "humanymousOriginOnce",
                    move |ctx| {
                        let req = req_ref.get_value(ctx.env)?;
                        let res = res_ref.get_value(ctx.env)?;
                        let app = app.get_value(ctx.env)?;
                        let _: Unknown = call(&app, "call", FnArgs::from(((), req, res)))?;
                        Ok(())
                    },
                )?;
                guard
                    .borrow_back(ctx.env)?
                    .call(FnArgs::from((req, res, next)))?;
                Ok(())
            })();
            if result.is_err() {
                fail(&res, 503)?;
            }
            Ok(())
        })?;
    let http = builtin(env, "http")?;
    let server: Object = call(&http, "createServer", request)?;
    for event in ["upgrade", "connect"] {
        let reject=env.create_function_from_closure::<(),(),_>("humanymousUnsupportedProtocol",move |ctx|{
            let socket:Object=ctx.get(1)?;let _:Unknown=call(&socket,"end","HTTP/1.1 503 Service Unavailable\r\nCache-Control: no-store\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned())?;Ok(())
        })?;
        let _: Unknown = call(&server, "on", FnArgs::from((event.to_owned(), reject)))?;
    }
    Ok(server)
}
