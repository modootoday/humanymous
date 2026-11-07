//! Safe runtime service contexts with explicit role selection.
#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "authority")]
pub mod authority;
#[cfg(feature = "capture")]
pub mod capture;
#[cfg(feature = "gateway")]
pub mod gateway;
#[cfg(feature = "native-security")]
pub mod native_security;
#[cfg(feature = "network")]
pub mod network;
