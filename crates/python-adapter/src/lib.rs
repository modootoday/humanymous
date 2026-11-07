//! Rust-native WSGI/ASGI callables. No Python implementation or eval strings.
mod asgi;
mod async_call;
#[cfg(test)]
mod hosted;
mod wsgi;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
fn failure(e: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

#[pymodule]
fn humanymous_python(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<wsgi::Wsgi>()?;
    module.add_class::<asgi::Asgi>()?;
    module.add("__version__", "0.1.0-experimental.1")?;
    Ok(())
}
