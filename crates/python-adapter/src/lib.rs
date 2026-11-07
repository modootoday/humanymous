//! Rust-native WSGI/ASGI callables. No Python implementation or eval strings.
#[cfg(feature = "native")]
mod asgi;
#[cfg(feature = "native")]
mod async_call;
#[cfg(all(test, feature = "host-pilot"))]
mod hosted;
#[cfg(feature = "native")]
mod wsgi;
#[cfg(feature = "native")]
use pyo3::{exceptions::PyRuntimeError, prelude::*};
#[cfg(feature = "native")]
fn failure(e: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

#[cfg(feature = "native")]
#[pymodule]
fn humanymous_python(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<wsgi::Wsgi>()?;
    module.add_class::<asgi::Asgi>()?;
    module.add("__version__", "0.1.0-experimental.1")?;
    Ok(())
}
