//! Complete ClientHello framing and same-transport cryptographic qualification.
mod hello;
mod transport;
pub use hello::{CaptureError, ClientHelloCapture};
pub use transport::{Cancellation, Connection};
