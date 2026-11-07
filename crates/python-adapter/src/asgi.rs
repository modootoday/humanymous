use crate::{async_call::call, failure};
use humanymous_authority::client::Verdict;
use humanymous_framework_adapter::{
    Admission, Configuration, Observation, Pending, internal_header,
};
use humanymous_integrity::adapter::AdapterProfile;
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyList},
};
use pyo3_async_runtimes::TaskLocals;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// ASGI HTTP1 guard with transparent trusted-server lifespan forwarding.
#[pyclass(module = "humanymous_python")]
pub struct Asgi {
    app: Py<PyAny>,
    config: Arc<Configuration>,
}
#[pymethods]
impl Asgi {
    #[new]
    fn new(app: Py<PyAny>, options_json: &str) -> PyResult<Self> {
        Ok(Self {
            app,
            config: Arc::new(Configuration::parse(options_json).map_err(failure)?),
        })
    }
    fn __call__<'py>(
        &self,
        py: Python<'py>,
        scope: &Bound<'py, PyDict>,
        receive: Py<PyAny>,
        send: Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kind = scope
            .get_item("type")?
            .ok_or_else(|| failure("ASGI scope type missing"))?
            .extract::<String>()?;
        if kind == "lifespan" {
            // Trusted server lifecycle is not a protected HTTP request. Forward
            // the original scope/state and callbacks without inventing decisions.
            let app = self.app.clone_ref(py);
            let scope = scope.clone().into_any().unbind();
            let locals = pyo3_async_runtimes::tokio::get_current_locals(py)?;
            return pyo3_async_runtimes::tokio::future_into_py(py, async move {
                call(&locals, &app, (scope, receive, send)).await?;
                Ok(())
            });
        }
        let admission = Admission::acquire().map_err(failure)?;
        let pending = Pending::new(&self.config).map_err(failure)?;
        let scope = scope.copy()?;
        let get = |key: &str| {
            scope
                .get_item(key)?
                .ok_or_else(|| failure("required ASGI field missing"))
        };
        if kind != "http"
            || !["1.0", "1.1"].contains(&get("http_version")?.extract::<String>()?.as_str())
        {
            return Err(failure("unsupported ASGI scope"));
        }
        let method = get("method")?.extract::<String>()?;
        let raw = get("raw_path")?;
        let mut target = std::str::from_utf8(raw.cast::<PyBytes>()?.as_bytes())
            .map_err(|_| failure("ASCII raw target required"))?
            .to_owned();
        let query = get("query_string")?;
        let query = query.cast::<PyBytes>()?.as_bytes();
        if !query.is_empty() {
            target.push('?');
            target
                .push_str(std::str::from_utf8(query).map_err(|_| failure("ASCII query required"))?);
        }
        let mut headers = Vec::new();
        let safe = PyList::empty(py);
        let mut authority = None;
        for item in get("headers")?.try_iter()? {
            let pair = item?;
            let (name, value): (Vec<u8>, Vec<u8>) = pair.extract()?;
            let name = String::from_utf8(name).map_err(|_| failure("invalid ASGI header name"))?;
            let value =
                String::from_utf8(value).map_err(|_| failure("unsupported header encoding"))?;
            if internal_header(&name) {
                continue;
            }
            if ["upgrade", "trailer", "transfer-encoding"]
                .contains(&name.to_ascii_lowercase().as_str())
            {
                return Err(failure("unsupported ASGI stream"));
            }
            if name.eq_ignore_ascii_case("host") {
                if authority.replace(value.clone()).is_some() {
                    return Err(failure("duplicate host"));
                }
            }
            safe.append((
                PyBytes::new(py, name.as_bytes()),
                PyBytes::new(py, value.as_bytes()),
            ))?;
            headers.push([name, value]);
        }
        scope.set_item("headers", safe)?;
        let observation = Observation {
            method,
            target,
            authority: authority.ok_or_else(|| failure("host required"))?,
            headers,
            peer: None,
            connection: None,
            raw: false,
        };
        let locals = pyo3_async_runtimes::tokio::get_current_locals(py)?;
        let app = self.app.clone_ref(py);
        let scope = scope.into_any().unbind();
        let config = self.config.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let _admission = admission;
            let mut body = Vec::new();
            loop {
                let event = tokio::time::timeout(
                    pending.remaining().map_err(failure)?,
                    call(&locals, &receive, ()),
                )
                .await
                .map_err(|_| failure("ASGI input deadline expired"))??;
                let (part, more) = Python::attach(|py| -> PyResult<_> {
                    let event = event.bind(py).cast::<PyDict>()?;
                    let kind = event
                        .get_item("type")?
                        .ok_or_else(|| failure("ASGI event type missing"))?
                        .extract::<String>()?;
                    if kind != "http.request" {
                        return Err(failure("ASGI request incomplete/disconnected"));
                    }
                    let part = if let Some(part) = event.get_item("body")? {
                        let bytes = part.cast::<PyBytes>()?.as_bytes();
                        if bytes.len() > config.max_body.saturating_sub(body.len()) {
                            return Err(failure("body exceeds limit"));
                        }
                        bytes.to_vec()
                    } else {
                        Vec::new()
                    };
                    let more = event
                        .get_item("more_body")?
                        .map(|p| p.extract::<bool>())
                        .transpose()?
                        .unwrap_or(false);
                    Ok((part, more))
                })?;
                if part.len() > config.max_body.saturating_sub(body.len()) {
                    return respond(&locals, &send, 503).await;
                }
                body.extend_from_slice(&part);
                if !more {
                    break;
                }
                pending.remaining().map_err(failure)?;
            }
            let bound = match pending
                .decide(&config, &observation, &body, AdapterProfile::PythonAsgi)
                .await
            {
                Ok(b) => b,
                Err(_) => return respond(&locals, &send, 503).await,
            };
            let expires = bound.expires();
            let verdict = bound
                .consume()
                .map_err(|_| failure("authority result expired"))?;
            if verdict != Verdict::Allow {
                return respond(
                    &locals,
                    &send,
                    if verdict == Verdict::Deny { 403 } else { 409 },
                )
                .await;
            }
            let replay = Python::attach(|py| {
                Py::new(
                    py,
                    Replay {
                        body: body.into(),
                        sent: AtomicBool::new(false),
                        receive,
                        locals: locals.clone(),
                    },
                )
            })?;
            pending.check_handoff(expires).map_err(failure)?;
            call(&locals, &app, (scope, replay, send)).await?;
            Ok(())
        })
    }
}
#[pyclass]
struct Replay {
    body: Arc<[u8]>,
    sent: AtomicBool,
    receive: Py<PyAny>,
    locals: TaskLocals,
}
#[pymethods]
impl Replay {
    fn __call__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let first = !self.sent.swap(true, Ordering::AcqRel);
        let body = self.body.clone();
        let receive = self.receive.clone_ref(py);
        let locals = self.locals.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if !first {
                return call(&locals, &receive, ()).await;
            }
            Python::attach(|py| {
                let event = PyDict::new(py);
                event.set_item("type", "http.request")?;
                event.set_item("body", PyBytes::new(py, &body))?;
                event.set_item("more_body", false)?;
                Ok(event.into_any().unbind())
            })
        })
    }
}
async fn respond(locals: &TaskLocals, send: &Py<PyAny>, status: u16) -> PyResult<()> {
    let (head, body) = Python::attach(|py| -> PyResult<_> {
        let head = PyDict::new(py);
        head.set_item("type", "http.response.start")?;
        head.set_item("status", status)?;
        head.set_item(
            "headers",
            vec![
                (b"content-type".to_vec(), b"application/json".to_vec()),
                (b"content-length".to_vec(), b"2".to_vec()),
                (b"cache-control".to_vec(), b"no-store".to_vec()),
            ],
        )?;
        let body = PyDict::new(py);
        body.set_item("type", "http.response.body")?;
        body.set_item("body", PyBytes::new(py, b"{}"))?;
        body.set_item("more_body", false)?;
        Ok((head.into_any().unbind(), body.into_any().unbind()))
    })?;
    call(locals, send, (head,)).await?;
    call(locals, send, (body,)).await?;
    Ok(())
}
