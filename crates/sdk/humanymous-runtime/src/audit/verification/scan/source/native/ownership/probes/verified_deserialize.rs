use super::*;

fn rejects_deserialization<'op, 'cancel>() {
    fn requires_deserialize<T: serde::de::DeserializeOwned>() {}
    requires_deserialize::<VerifiedScan<'op, 'cancel>>();
}
