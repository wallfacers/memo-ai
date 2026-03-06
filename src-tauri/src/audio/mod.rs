pub mod capture;
pub mod encoder;

#[cfg(target_os = "windows")]
pub mod wasapi;
