//! Schedule Python callbacks on their owning event loop and cancel on Rust Drop.
use crate::failure;
use pyo3::{
    BoundObject,
    prelude::*,
    types::{PyDict, PyTuple},
};
use pyo3_async_runtimes::TaskLocals;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::oneshot;

struct State {
    cancelled: bool,
    task: Option<Py<PyAny>>,
    result: Option<oneshot::Sender<PyResult<Py<PyAny>>>>,
}
#[pyclass]
struct Start {
    callback: Py<PyAny>,
    args: Py<PyTuple>,
    state: Arc<Mutex<State>>,
}
#[pymethods]
impl Start {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        if self
            .state
            .lock()
            .map_err(|_| failure("callback state unavailable"))?
            .cancelled
        {
            return Ok(());
        }
        let result = (|| {
            let awaitable = self.callback.bind(py).call1(self.args.bind(py))?;
            let task = py
                .import("asyncio")?
                .call_method1("ensure_future", (awaitable,))?;
            task.call_method1(
                "add_done_callback",
                (Py::new(
                    py,
                    Done {
                        state: Arc::downgrade(&self.state),
                    },
                )?,),
            )?;
            let mut state = self
                .state
                .lock()
                .map_err(|_| failure("callback state unavailable"))?;
            if state.cancelled {
                task.call_method0("cancel")?;
            }
            state.task = Some(task.unbind());
            Ok(())
        })();
        if let Err(e) = result
            && let Some(tx) = self
                .state
                .lock()
                .map_err(|_| failure("callback state unavailable"))?
                .result
                .take()
        {
            let _ = tx.send(Err(e));
        }
        Ok(())
    }
}
#[pyclass]
struct Done {
    state: Weak<Mutex<State>>,
}
#[pymethods]
impl Done {
    fn __call__(&self, task: &Bound<'_, PyAny>) -> PyResult<()> {
        let result = task.call_method0("result").map(Bound::unbind);
        let Some(state) = self.state.upgrade() else {
            return Ok(());
        };
        let mut state = state
            .lock()
            .map_err(|_| failure("callback state unavailable"))?;
        state.task.take();
        if let Some(tx) = state.result.take() {
            let _ = tx.send(result);
        }
        Ok(())
    }
}
#[pyclass]
struct Cancel {
    state: Arc<Mutex<State>>,
}
#[pymethods]
impl Cancel {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        let task = self
            .state
            .lock()
            .map_err(|_| failure("callback state unavailable"))?
            .task
            .as_ref()
            .map(|t| t.clone_ref(py));
        if let Some(task) = task {
            task.call_method0(py, "cancel")?;
        }
        Ok(())
    }
}
struct Guard {
    locals: TaskLocals,
    state: Arc<Mutex<State>>,
    complete: bool,
}
impl Drop for Guard {
    fn drop(&mut self) {
        if self.complete {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            state.cancelled = true;
        }
        // No blocking wait or foreign executable code on cleanup.
        Python::attach(|py| {
            if let Ok(cancel) = Py::new(
                py,
                Cancel {
                    state: self.state.clone(),
                },
            ) {
                let _ = self
                    .locals
                    .event_loop(py)
                    .call_method1("call_soon_threadsafe", (cancel,));
            }
        });
    }
}
pub async fn call<A>(locals: &TaskLocals, callback: &Py<PyAny>, args: A) -> PyResult<Py<PyAny>>
where
    A: Send + for<'py> IntoPyObject<'py, Target = PyTuple>,
{
    let (tx, rx) = oneshot::channel();
    let state = Arc::new(Mutex::new(State {
        cancelled: false,
        task: None,
        result: Some(tx),
    }));
    let mut guard = Guard {
        locals: locals.clone(),
        state: state.clone(),
        complete: false,
    };
    Python::attach(|py| -> PyResult<()> {
        let args = args
            .into_pyobject(py)
            .map_err(|e| -> PyErr { e.into() })?
            .unbind();
        let start = Py::new(
            py,
            Start {
                callback: callback.clone_ref(py),
                args,
                state,
            },
        )?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("context", locals.context(py))?;
        locals
            .event_loop(py)
            .call_method("call_soon_threadsafe", (start,), Some(&kwargs))?;
        Ok(())
    })?;
    let result = rx
        .await
        .map_err(|_| failure("callback completion unavailable"))?;
    guard.complete = true;
    result
}
