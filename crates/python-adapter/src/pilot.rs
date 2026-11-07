//! Hosted server factory and native framework views; never shipped in production.
use crate::failure;
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyList, PyModule},
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

pub(super) fn effect(path: &PathBuf, body: &[u8]) -> PyResult<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| failure("owned pilot effect receipt unavailable"))?;
    writeln!(
        file,
        "{}",
        serde_json::json!({"bodyLength":body.len(),"bodySha256":hex::encode(Sha256::digest(body))})
    )
    .map_err(|_| failure("owned pilot effect receipt write failed"))
}
#[pyclass]
struct FlaskView {
    effect_file: PathBuf,
}
#[pymethods]
impl FlaskView {
    fn __call__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let flask = py.import("flask")?;
        let body = flask.getattr("request")?.call_method0("get_data")?;
        effect(&self.effect_file, body.cast::<PyBytes>()?.as_bytes())?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("status", 201)?;
        kwargs.set_item("content_type", "application/octet-stream")?;
        let response = flask.getattr("Response")?.call((body,), Some(&kwargs))?;
        response.call_method1("set_cookie", ("one", "1"))?;
        response.call_method1("set_cookie", ("two", "2"))?;
        Ok(response.unbind())
    }
}
#[pyclass]
struct DjangoView {
    effect_file: PathBuf,
}
#[pymethods]
impl DjangoView {
    fn __call__(&self, request: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let py = request.py();
        let body = request.getattr("body")?;
        effect(&self.effect_file, body.cast::<PyBytes>()?.as_bytes())?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("status", 201)?;
        kwargs.set_item("content_type", "application/octet-stream")?;
        let response = py
            .import("django.http")?
            .getattr("HttpResponse")?
            .call((body,), Some(&kwargs))?;
        response.call_method1("set_cookie", ("one", "1"))?;
        response.call_method1("set_cookie", ("two", "2"))?;
        Ok(response.unbind())
    }
}

pub(super) struct Request {
    pub options: String,
    pub effect_file: PathBuf,
    pub framework: String,
}
pub(super) fn request() -> PyResult<Request> {
    if std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref() != Ok("1") {
        return Err(failure(
            "explicit hosted native factory provisioning required",
        ));
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| failure("private native pilot request unavailable"))?;
    if bytes.len() > 65536 {
        return Err(failure("private native pilot request exceeds limit"));
    }
    // Only fixture orchestration uses this map. The configuration remains a JSON
    // STRING so the shared strict parser still sees duplicate/unknown members.
    let request: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| failure("invalid native pilot request"))?;
    let options = request["configuration"]
        .as_str()
        .ok_or_else(|| failure("strict config string required"))?;
    let effect_file = PathBuf::from(
        request["effect_file"]
            .as_str()
            .ok_or_else(|| failure("owned effect file required"))?,
    );
    if !effect_file.is_absolute() {
        return Err(failure("absolute owned effect path required"));
    }
    Ok(Request {
        options: options.into(),
        effect_file,
        framework: request["framework"]
            .as_str()
            .ok_or_else(|| failure("framework required"))?
            .into(),
    })
}

pub(super) fn django<'py>(
    py: Python<'py>,
    effect_file: PathBuf,
    asgi: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let urls = PyModule::new(py, "humanymous_owned_pilot_urls")?;
    let route = py
        .import("django.urls")?
        .call_method1("path", ("api", Py::new(py, DjangoView { effect_file })?))?;
    urls.add("urlpatterns", PyList::new(py, [route])?)?;
    let settings = py.import("django.conf")?.getattr("settings")?;
    if settings.getattr("configured")?.extract::<bool>()? {
        return Err(failure("fresh Django pilot process required"));
    }
    let kwargs = PyDict::new(py);
    kwargs.set_item("SECRET_KEY", "nonsecret-owned-native-fixture")?;
    kwargs.set_item("ALLOWED_HOSTS", vec!["example.test"])?;
    kwargs.set_item("ROOT_URLCONF", urls)?;
    settings.call_method("configure", (), Some(&kwargs))?;
    if asgi {
        py.import("django.core.asgi")?
            .call_method0("get_asgi_application")
    } else {
        py.import("django.core.wsgi")?
            .call_method0("get_wsgi_application")
    }
}

/// Upstream WSGI server --call factory, configured through bounded private stdin.
#[pyfunction]
pub fn pilot_wsgi(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let request = request()?;
    let effect_file = request.effect_file;
    let app = match request.framework.as_str() {
        "flask" => {
            let app = py
                .import("flask")?
                .getattr("Flask")?
                .call1(("humanymous_owned_native_pilot",))?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("endpoint", "owned_binary")?;
            kwargs.set_item("view_func", Py::new(py, FlaskView { effect_file })?)?;
            kwargs.set_item("methods", vec!["POST"])?;
            app.call_method("add_url_rule", ("/api",), Some(&kwargs))?;
            app
        }
        "django" => django(py, effect_file, false)?,
        _ => return Err(failure("unsupported WSGI pilot framework")),
    };
    py.import("humanymous_python")?
        .getattr("Wsgi")?
        .call1((app, request.options))
        .map(Bound::unbind)
}
