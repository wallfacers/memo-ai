/// Windows WASAPI loopback capture (system audio).
/// This is a stub — full WASAPI loopback implementation requires
/// careful COM initialization and is added in a follow-up sprint.
///
/// For now, cpal's default input (microphone) is used on all platforms.
/// To mix system audio on Windows, enable this module and blend channels.

#[allow(dead_code)]
pub struct WasapiLoopback;

#[allow(dead_code)]
impl WasapiLoopback {
    pub fn new() -> Self {
        WasapiLoopback
    }

    pub fn is_supported() -> bool {
        // Will be true once full implementation is in place
        false
    }
}
