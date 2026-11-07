//! Real-framework ASGI factory; compiled only into the explicit host pilot.
use crate::{async_call::call, failure, pilot};
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict},
};
use std::path::PathBuf;

#[pyclass]
struct Origin {
    effect_file: PathBuf,
}
#[pymethods]
impl Origin {
    fn __call__<'py>(
        &self,
        py: Python<'py>,
        scope: &Bound<'py, PyDict>,
        receive: Py<PyAny>,
        send: Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if scope
            .get_item("type")?
            .ok_or_else(|| failure("origin scope missing"))?
            .extract::<String>()?
            != "http"
        {
            return Err(failure("native pilot origin accepts HTTP only"));
        }
        let locals = pyo3_async_runtimes::tokio::get_current_locals(py)?;
        let effect_file = self.effect_file.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut body = Vec::new();
            loop {
                let event = call(&locals, &receive, ()).await?;
                let more = Python::attach(|py| -> PyResult<bool> {
                    let event = event.bind(py).cast::<PyDict>()?;
                    if event
                        .get_item("type")?
                        .ok_or_else(|| failure("origin event missing"))?
                        .extract::<String>()?
                        != "http.request"
                    {
                        return Err(failure("origin request disconnected or unsupported"));
                    }
                    if let Some(part) = event.get_item("body")? {
                        let part = part.cast::<PyBytes>()?.as_bytes();
                        if part.len() > 1048576_usize.saturating_sub(body.len()) {
                            return Err(failure("pilot origin body exceeds bound"));
                        }
                        body.extend_from_slice(part);
                    }
                    event
                        .get_item("more_body")?
                        .map(|value| value.extract::<bool>())
                        .transpose()
                        .map(|value| value.unwrap_or(false))
                })?;
                if !more {
                    break;
                }
            }
            let (head, event) = Python::attach(|py| -> PyResult<_> {
                pilot::effect(&effect_file, &body)?;
                let head = PyDict::new(py);
                head.set_item("type", "http.response.start")?;
                head.set_item("status", 201)?;
                head.set_item(
                    "headers",
                    vec![
                        (
                            b"content-type".to_vec(),
                            b"application/octet-stream".to_vec(),
                        ),
                        (
                            b"content-length".to_vec(),
                            body.len().to_string().into_bytes(),
                        ),
                        (
                            b"set-cookie".to_vec(),
                            b"one=1; Path=/; SameSite=lax".to_vec(),
                        ),
                        (
                            b"set-cookie".to_vec(),
                            b"two=2; Path=/; SameSite=lax".to_vec(),
                        ),
                    ],
                )?;
                let event = PyDict::new(py);
                event.set_item("type", "http.response.body")?;
                event.set_item("body", PyBytes::new(py, &body))?;
                event.set_item("more_body", false)?;
                Ok((head.into_any().unbind(), event.into_any().unbind()))
            })?;
            call(&locals, &send, (head,)).await?;
            call(&locals, &send, (event,)).await?;
            Ok(())
        })
    }
}

/// Upstream ASGI server --factory entry point with bounded private stdin.
#[pyfunction]
pub fn pilot_asgi(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let request = pilot::request()?;
    let app = match request.framework.as_str() {
        "starlette" | "fastapi" => {
            let (module, class) = if request.framework == "starlette" {
                ("starlette.applications", "Starlette")
            } else {
                ("fastapi", "FastAPI")
            };
            let app = py.import(module)?.getattr(class)?.call0()?;
            app.call_method1(
                "mount",
                (
                    "/",
                    Py::new(
                        py,
                        Origin {
                            effect_file: request.effect_file,
                        },
                    )?,
                ),
            )?;
            app
        }
        "django" => pilot::django(py, request.effect_file, true)?,
        _ => return Err(failure("unsupported ASGI pilot framework")),
    };
    py.import("humanymous_python")?
        .getattr("Asgi")?
        .call1((app, request.options))
        .map(Bound::unbind)
}
