//! Trusted server callback; absent or non-boolean state never means connected.
use magnus::{
    Error, RHash, Ruby, Value,
    prelude::*,
    value::{Qfalse, Qtrue},
};

pub(super) fn disconnected(ruby: &Ruby, env: RHash) -> Result<bool, Error> {
    let callback: Value = env
        .get("humanymous.client_disconnected")
        .ok_or_else(|| super::failure(ruby, "trusted server disconnect callback required"))?;
    let value: Value = callback.funcall("call", ())?;
    if Qfalse::from_value(value).is_some() {
        Ok(false)
    } else if Qtrue::from_value(value).is_some() {
        Ok(true)
    } else {
        Err(super::failure(
            ruby,
            "server disconnect callback must return a boolean",
        ))
    }
}
