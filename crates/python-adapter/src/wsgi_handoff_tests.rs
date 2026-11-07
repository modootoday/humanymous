//! Actual embedded CPython WSGI boundary; frozen evidence is component-only.
#[path = "../../framework-adapter/tests/support/authority.rs"]
mod authority_fixture;
#[expect(
    dead_code,
    reason = "Shared server fixture also provides unused empty-WAL checks."
)]
#[path = "../tests/support/wsgi_authority.rs"]
mod wsgi_authority;
#[path = "../tests/support/authority_environment.rs"]
mod wsgi_host;

use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict, PyList, PyModule},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[pyclass]
struct Connected {
    calls: Arc<AtomicUsize>,
    observed: authority_fixture::Observations,
    final_delay: Duration,
}
#[pymethods]
impl Connected {
    fn __call__(&self) -> bool {
        let call = self.calls.fetch_add(1, Ordering::AcqRel);
        assert!(
            call < 2,
            "WSGI must invoke the initial and final callbacks exactly once"
        );
        if call == 1 {
            // This callback is reached only after actual bound ALLOW and replay
            // construction. Never delay the initial pre-input callback: that
            // would test an earlier timeout rather than the handoff regression.
            assert_eq!(self.observed.lock().unwrap().len(), 1);
            std::thread::sleep(self.final_delay);
        }
        false
    }
}
#[pyclass]
struct Origin {
    calls: Arc<AtomicUsize>,
}
#[pymethods]
impl Origin {
    fn __call__(
        &self,
        py: Python<'_>,
        env: &Bound<'_, PyDict>,
        start: Py<PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        let input = env.get_item("wsgi.input")?.unwrap().call_method0("read")?;
        assert_eq!(input.cast::<PyBytes>()?.as_bytes(), &[0, 255, 42]);
        start.call1(py, ("201 Created", vec![("Content-Length", "3")]))?;
        Ok(PyList::new(py, [input])?.into_any().unbind())
    }
}
#[pyclass]
struct Start {
    statuses: Arc<Mutex<Vec<String>>>,
}
#[pymethods]
impl Start {
    fn __call__(&self, status: String, _headers: Py<PyAny>) {
        self.statuses.lock().unwrap().push(status);
    }
}

fn scenario(late: bool) {
    assert_eq!(
        std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref(),
        Ok("1")
    );
    let directory = wsgi_host::Directory::new();
    let engine =
        wsgi_authority::Running::start(&directory, "wsgi-final-consume", None, "python-wsgi");
    let config = serde_json::json!({"installation":"i","tenant":"t","origin":"o","revision":"r",
        "endpoint":engine.endpoint,"credential":engine.credential,
        "server":{"kind":"unix","uid":humanymous_runtime::native_security::process::current_uid()},
        "routes":["/api"],"timeoutMs":2000,"maxBody":8,"syncInputDeadlineEnforced":true})
    .to_string();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let origins = Arc::new(AtomicUsize::new(0));
    let statuses = Arc::new(Mutex::new(Vec::new()));
    Python::initialize();
    Python::attach(|py| -> PyResult<()> {
        let native = PyModule::new(py, "humanymous_python")?;
        crate::humanymous_python(&native)?;
        let app = Py::new(
            py,
            Origin {
                calls: origins.clone(),
            },
        )?;
        let guarded = native.getattr("Wsgi")?.call1((app, config))?;
        let env = PyDict::new(py);
        for (key, value) in [
            ("REQUEST_METHOD", "POST"),
            ("REQUEST_URI", "/api"),
            ("HTTP_HOST", "example.test"),
            ("SERVER_PROTOCOL", "HTTP/1.1"),
            ("CONTENT_LENGTH", "3"),
        ] {
            env.set_item(key, value)?;
        }
        let input = py
            .import("io")?
            .getattr("BytesIO")?
            .call1((PyBytes::new(py, &[0, 255, 42]),))?;
        env.set_item("wsgi.input", input)?;
        env.set_item(
            "humanymous.client_disconnected",
            Py::new(
                py,
                Connected {
                    calls: callbacks.clone(),
                    observed: engine.observed.clone(),
                    final_delay: if late {
                        Duration::from_millis(2100)
                    } else {
                        Duration::ZERO
                    },
                },
            )?,
        )?;
        let start = Py::new(
            py,
            Start {
                statuses: statuses.clone(),
            },
        )?;
        let result = guarded.call1((env, start));
        if late {
            let error = result.expect_err("late connected callback must refuse origin");
            assert!(
                error.to_string().contains("deadline expired"),
                "unexpected refusal: {error}"
            );
            assert_eq!(origins.load(Ordering::Acquire), 0);
            assert!(statuses.lock().unwrap().is_empty());
        } else {
            assert_eq!(result?.extract::<Vec<Vec<u8>>>()?, vec![vec![0, 255, 42]]);
            assert_eq!(origins.load(Ordering::Acquire), 1);
            assert_eq!(*statuses.lock().unwrap(), ["201 Created"]);
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        callbacks.load(Ordering::Acquire),
        2,
        "both trusted callbacks must actually execute"
    );
    let observations = engine.observed.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].mode, "wsgi-normalized");
    assert!(
        observations[0]
            .headers
            .iter()
            .any(|[name, value]| name == "content-length" && value == "3")
    );
    drop(observations);
    let records = engine.finish(&directory, "wsgi-final-consume");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].verdict, "ALLOW");
    println!(
        "wsgi-final-consume late={late} bound_allow=1 trusted_callbacks=2 origin_calls={} whole_product_qualified=false",
        origins.load(Ordering::Acquire)
    );
}

#[test]
#[ignore = "allocated actual embedded CPython WSGI and component Authority required"]
fn late_connected_callback_after_bound_allow_has_no_origin() {
    scenario(true);
}

#[test]
#[ignore = "allocated actual embedded CPython WSGI and component Authority required"]
fn timely_connected_callback_after_bound_allow_invokes_origin_once() {
    scenario(false);
}
