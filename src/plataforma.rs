//! Integrações que mudam entre Windows e Linux.

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

#[cfg(windows)]
#[path = "windows.rs"]
mod imp;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("FOL-discord suporta somente Windows e Linux");

pub use imp::*;
