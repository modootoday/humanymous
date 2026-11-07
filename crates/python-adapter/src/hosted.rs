//! Explicit hosted-only real-framework callback smoke. Not product qualification.
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyModule, PyTuple},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[pyclass]
struct Spy {
    app: Py<PyAny>,
    effects: Arc<AtomicUsize>,
}
#[pymethods]
impl Spy {
    #[pyo3(signature=(*args,**kwargs))]
    fn __call__(
        &self,
        py: Python<'_>,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        self.effects.fetch_add(1, Ordering::AcqRel);
        self.app.bind(py).call(args, kwargs).map(Bound::unbind)
    }
}
#[pyclass]
struct Start {
    statuses: Arc<Mutex<Vec<String>>>,
}
#[pymethods]
impl Start {
    fn __call__(&self, status: String, _headers: Py<PyAny>) -> PyResult<()> {
        self.statuses.lock().unwrap().push(status);
        Ok(())
    }
}
fn config() -> String {
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\humanymous-python-hosted-no-service"
    } else {
        "/tmp/humanymous-python-hosted-no-service.sock"
    };
    // Non-secret unavailable dependency fixture; never valid production settings.
    serde_json::json!({"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,
        "credential":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":{"kind":"unix","uid":0},
        "routes":["/api"],"timeoutMs":250,"maxBody":8,"syncInputDeadlineEnforced":true}).to_string()
}
fn enabled() {
    assert_eq!(
        std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref(),
        Ok("1"),
        "actual interpreter/framework pilots must be explicitly provisioned on hosted"
    );
}
fn module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let module = PyModule::new(py, "humanymous_python")?;
    crate::humanymous_python(&module)?;
    Ok(module)
}
fn django_settings(py: Python<'_>) -> PyResult<()> {
    let settings = py.import("django.conf")?.getattr("settings")?;
    if !settings.getattr("configured")?.extract::<bool>()? {
        let kwargs = PyDict::new(py);
        kwargs.set_item("SECRET_KEY", "nonsecret-owned-fixture")?;
        kwargs.set_item("ALLOWED_HOSTS", vec!["example.test"])?;
        settings.call_method("configure", (), Some(&kwargs))?;
    }
    Ok(())
}

#[test]
#[ignore = "actual CPython/Flask/Django native callback; hosted only"]
fn wsgi_real_frameworks_dependency_failure_has_no_origin_effect() {
    enabled();
    Python::initialize();
    Python::attach(|py| -> PyResult<()> {
        let native = module(py)?;
        for framework in ["flask", "django"] {
            let app = if framework == "flask" {
                py.import("flask")?
                    .getattr("Flask")?
                    .call1(("humanymous-owned-pilot",))?
            } else {
                django_settings(py)?;
                py.import("django.core.wsgi")?
                    .call_method0("get_wsgi_application")?
            };
            let effects = Arc::new(AtomicUsize::new(0));
            let spy = Py::new(
                py,
                Spy {
                    app: app.unbind(),
                    effects: effects.clone(),
                },
            )?;
            let guarded = native.getattr("Wsgi")?.call1((spy, config()))?;
            let env = PyDict::new(py);
            env.set_item("REQUEST_METHOD", "POST")?;
            env.set_item("REQUEST_URI", "/api?binary=1")?;
            env.set_item("HTTP_HOST", "example.test")?;
            env.set_item("SERVER_PROTOCOL", "HTTP/1.1")?;
            env.set_item("CONTENT_LENGTH", "2")?;
            let input = py
                .import("io")?
                .getattr("BytesIO")?
                .call1((PyBytes::new(py, &[0, 255]),))?;
            env.set_item("wsgi.input", input)?;
            let statuses = Arc::new(Mutex::new(Vec::new()));
            let start = Py::new(
                py,
                Start {
                    statuses: statuses.clone(),
                },
            )?;
            let body = guarded.call1((env, start))?;
            let chunks: Vec<Vec<u8>> = body.extract()?;
            assert_eq!(chunks, vec![b"{}".to_vec()]);
            assert_eq!(*statuses.lock().unwrap(), ["503 Service Unavailable"]);
            assert_eq!(
                effects.load(Ordering::Acquire),
                0,
                "{framework} protected callback ran"
            );
            println!("humanymous-python-hosted framework={framework} status=503 origin_effects=0 profile=dependency-unavailable");
        }
        Ok(())
    })
    .unwrap();
}

#[test]
#[ignore = "actual CPython/Starlette/FastAPI ASGI native async callback; hosted only"]
fn asgi_real_frameworks_dependency_failure_has_no_origin_effect() {
    enabled();
    Python::initialize();
    Python::attach(|py| -> PyResult<()> {
        let native = module(py)?;
        for (name, class) in [
            ("starlette.applications", "Starlette"),
            ("fastapi", "FastAPI"),
            ("django.core.asgi", "get_asgi_application"),
        ] {
            if name=="django.core.asgi" {django_settings(py)?;}
            let app = py.import(name)?.getattr(class)?.call0()?;
            let effects = Arc::new(AtomicUsize::new(0));
            let spy = Py::new(
                py,
                Spy {
                    app: app.unbind(),
                    effects: effects.clone(),
                },
            )?;
            let guard = native.getattr("Asgi")?.call1((spy, config()))?;
            let scope = PyDict::new(py);
            scope.set_item("type", "http")?;
            scope.set_item("http_version", "1.1")?;
            scope.set_item("method", "POST")?;
            scope.set_item("raw_path", PyBytes::new(py, b"/api"))?;
            scope.set_item("query_string", PyBytes::new(py, b"binary=1"))?;
            scope.set_item(
                "headers",
                vec![
                    (b"host".to_vec(), b"example.test".to_vec()),
                    (b"content-length".to_vec(), b"2".to_vec()),
                ],
            )?;
            let asyncio = py.import("asyncio")?;
            let loop_ = asyncio.call_method0("new_event_loop")?;
            asyncio.call_method1("set_event_loop", (&loop_,))?;
            let inbound = asyncio.getattr("Queue")?.call0()?;
            let event = PyDict::new(py);
            event.set_item("type", "http.request")?;
            event.set_item("body", PyBytes::new(py, &[0, 255]))?;
            event.set_item("more_body", false)?;
            inbound.call_method1("put_nowait", (event,))?;
            let outbound = asyncio.getattr("Queue")?.call0()?;
            // Schedule guard creation on its owning running loop; no Python script.
            let factory = Py::new(
                py,
                AsgiInvoke {
                    guard: guard.unbind(),
                    scope: scope.into_any().unbind(),
                    receive: inbound.getattr("get")?.unbind(),
                    send: outbound.getattr("put")?.unbind(),
                    loop_: loop_.clone().unbind(),
                },
            )?;
            loop_.call_method1("call_soon", (factory,))?;
            loop_.call_method1(
                "call_later",
                (
                    5.0,
                    Py::new(
                        py,
                        Stop {
                            loop_: loop_.clone().unbind(),
                        },
                    )?,
                    py.None(),
                ),
            )?;
            loop_.call_method0("run_forever")?;
            let task = loop_.getattr("_humanymous_pilot_task")?;
            task.call_method0("result")?;
            let head = outbound.call_method0("get_nowait")?;
            assert_eq!(head.get_item("status")?.extract::<u16>()?, 503);
            assert_eq!(
                effects.load(Ordering::Acquire),
                0,
                "{name} protected callback ran"
            );
            println!("humanymous-python-hosted framework={name} status=503 origin_effects=0 profile=dependency-unavailable");
            loop_.call_method0("close")?;
        }
        Ok(())
    })
    .unwrap();
}
#[pyclass]
struct AsgiInvoke {
    guard: Py<PyAny>,
    scope: Py<PyAny>,
    receive: Py<PyAny>,
    send: Py<PyAny>,
    loop_: Py<PyAny>,
}
#[pymethods]
impl AsgiInvoke {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        let future = self.guard.call1(
            py,
            (
                self.scope.clone_ref(py),
                self.receive.clone_ref(py),
                self.send.clone_ref(py),
            ),
        )?;
        self.loop_
            .setattr(py, "_humanymous_pilot_task", future.clone_ref(py))?;
        future.call_method1(
            py,
            "add_done_callback",
            (Py::new(
                py,
                Stop {
                    loop_: self.loop_.clone_ref(py),
                },
            )?,),
        )?;
        Ok(())
    }
}
#[pyclass]
struct Stop {
    loop_: Py<PyAny>,
}
#[pymethods]
impl Stop {
    fn __call__(&self, py: Python<'_>, _task: Py<PyAny>) -> PyResult<()> {
        self.loop_.call_method0(py, "stop")?;
        Ok(())
    }
}
