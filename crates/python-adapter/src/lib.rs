//! Rust-native WSGI/ASGI callables. No Python implementation or eval strings.
#[cfg(feature = "native")]
mod asgi;
#[cfg(feature = "native")]
mod async_call;
#[cfg(all(test, feature = "host-pilot"))]
mod hosted;
#[cfg(feature = "host-pilot")]
mod pilot;
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
    module.add_function(wrap_pyfunction!(capabilities, module)?)?;
    #[cfg(feature = "host-pilot")]
    module.add_function(wrap_pyfunction!(pilot::pilot_wsgi, module)?)?;
    Ok(())
}

#[cfg(feature = "native")]
#[pyfunction]
fn capabilities() -> String {
    serde_json::json!({"adapter":"humanymous.python","version":"0.1.0-experimental.1",
        "implementation":"rust-pyo3","profiles":["asgi-normalized","wsgi-normalized"],
        "rawTls":false,"originalH2":false,"rawHeaderOrder":false,"capture":false,
        "requestStreaming":false,"websocket":false,"administration":false,
        "maximumBodyBytes":1048576,"maximumPending":64,
        "wsgiDisconnectCallbackRequired":true,"asgiServerLifespan":"transparent",
        "hostPilotEnabled":cfg!(feature="host-pilot"),"packageToolEnabled":cfg!(feature="package-tool"),
        "releaseQualifiedTuples":[]}).to_string()
}
