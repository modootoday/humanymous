use crate::failure;
use humanymous_authority::client::Verdict;
use humanymous_framework_adapter::{
    Admission, Configuration, Observation, Pending, body_length, internal_header,
};
use humanymous_integrity::adapter::AdapterProfile;
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyList},
};
use std::sync::Arc;

/// Root WSGI middleware. Install before every parser and protected effect.
#[pyclass(module = "humanymous_python")]
pub struct Wsgi {
    app: Py<PyAny>,
    config: Arc<Configuration>,
}
#[pymethods]
impl Wsgi {
    #[new]
    fn new(app: Py<PyAny>, options_json: &str) -> PyResult<Self> {
        let config = Configuration::parse(options_json).map_err(failure)?;
        if !config.sync_input_deadline_enforced {
            return Err(failure("server input deadline required"));
        }
        Ok(Self {
            app,
            config: Arc::new(config),
        })
    }
    fn __call__<'py>(
        &self,
        py: Python<'py>,
        environ: &Bound<'py, PyDict>,
        start_response: Py<PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let _admission = Admission::acquire().map_err(failure)?;
        let pending = Pending::new(&self.config).map_err(failure)?;
        let env = environ.copy()?;
        let get = |key: &str| -> PyResult<String> {
            env.get_item(key)?
                .ok_or_else(|| failure("required WSGI field missing"))?
                .extract()
        };
        let method = get("REQUEST_METHOD")?;
        let target = env
            .get_item("RAW_URI")?
            .or(env.get_item("REQUEST_URI")?)
            .ok_or_else(|| failure("server raw target extension required"))?
            .extract::<String>()?;
        if !self.config.permits_target(&target) {
            return reject(py, &start_response, 503);
        }
        let protocol = get("SERVER_PROTOCOL")?;
        if !["HTTP/1.0", "HTTP/1.1"].contains(&protocol.as_str()) {
            return reject(py, &start_response, 503);
        }
        let authority = get("HTTP_HOST")?;
        let length = body_length(&get("CONTENT_LENGTH")?, self.config.max_body).map_err(failure)?;
        let mut headers = Vec::new();
        let mut remove = Vec::new();
        for (key, value) in env.iter() {
            let key: String = key.extract()?;
            if let Some(name) = key.strip_prefix("HTTP_") {
                let name = name.replace('_', "-").to_ascii_lowercase();
                if internal_header(&name) {
                    remove.push(key);
                    continue;
                }
                if ["upgrade", "transfer-encoding", "trailer"].contains(&name.as_str()) {
                    return reject(py, &start_response, 503);
                }
                headers.push([name, value.extract::<String>()?]);
            }
        }
        for key in remove {
            env.del_item(key)?;
        }
        headers.retain(|[n, _]| n != "content-length" && n != "content-type");
        headers.push(["content-length".into(), length.to_string()]);
        if let Some(ct) = env.get_item("CONTENT_TYPE")? {
            headers.push(["content-type".into(), ct.extract()?]);
        }
        let input = env
            .get_item("wsgi.input")?
            .ok_or_else(|| failure("WSGI input missing"))?;
        let mut body = Vec::with_capacity(length);
        while body.len() < length {
            pending.remaining().map_err(failure)?;
            let part = input.call_method1("read", ((length - body.len()).min(65536),))?;
            let bytes = part.cast::<PyBytes>()?.as_bytes();
            if bytes.is_empty() || bytes.len() > length - body.len() {
                return reject(py, &start_response, 503);
            }
            body.extend_from_slice(bytes);
        }
        let observation = Observation {
            method,
            target,
            authority,
            headers,
            peer: None,
            connection: None,
            raw: false,
        };
        let config = self.config.clone();
        let result = py.detach(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| humanymous_framework_adapter::Error("adapter runtime unavailable"))?;
            rt.block_on(pending.decide(&config, &observation, &body, AdapterProfile::PythonWsgi))
        });
        let bound = match result {
            Ok(v) => v,
            Err(_) => return reject(py, &start_response, 503),
        };
        let expires = bound.expires();
        let verdict = match bound.consume() {
            Ok(v) => v,
            Err(_) => return reject(py, &start_response, 503),
        };
        if verdict != Verdict::Allow {
            return reject(
                py,
                &start_response,
                if verdict == Verdict::Deny { 403 } else { 409 },
            );
        }
        let replay = py
            .import("io")?
            .getattr("BytesIO")?
            .call1((PyBytes::new(py, &body),))?;
        env.set_item("wsgi.input", replay)?;
        pending.check_handoff(expires).map_err(failure)?;
        self.app.call1(py, (env, start_response))
    }
}
fn reject(py: Python<'_>, start: &Py<PyAny>, status: u16) -> PyResult<Py<PyAny>> {
    let text = match status {
        403 => "403 Forbidden",
        409 => "409 Conflict",
        _ => "503 Service Unavailable",
    };
    start.call1(
        py,
        (
            text,
            vec![
                ("Content-Type", "application/json"),
                ("Content-Length", "2"),
                ("Cache-Control", "no-store"),
            ],
        ),
    )?;
    Ok(PyList::new(py, [PyBytes::new(py, b"{}")])?
        .into_any()
        .unbind())
}
