//! Complete ClientHello framing and same-transport cryptographic qualification.
mod hello;
mod observation;
mod records;
mod transport;
pub use hello::{CaptureError, ClientHelloCapture};
pub use transport::{Cancellation, Connection, PendingHandshake};

pub use observation::{H2HeadersObservation, IngressSnapshot};
