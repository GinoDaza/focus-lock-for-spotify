//! Windows-only parts of the service. `acl` and `user` are the only
//! modules that call Win32 directly.

pub mod acl;
pub mod desktop;
pub mod scm;
pub mod user;
