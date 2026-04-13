/// Shared test utilities for GameSense Agent
///
/// Provides helpers for creating test fixtures, mock ADB outputs,
/// and temporary directories for file-based tests.

#[cfg(test)]
pub mod fixtures {
    use std::path::PathBuf;

    /// Returns a fake ADB path for testing command construction
    pub fn fake_adb_path() -> PathBuf {
        PathBuf::from("/usr/bin/adb")
    }

    /// Sample `adb devices` output with one connected device
    pub fn sample_devices_output() -> &'static str {
        "List of devices attached\nR5CT900XYZ1\tdevice\n\n"
    }

    /// Sample `adb devices` output with no devices
    pub fn empty_devices_output() -> &'static str {
        "List of devices attached\n\n"
    }

    /// Sample `adb devices` output with unauthorized device
    pub fn unauthorized_device_output() -> &'static str {
        "List of devices attached\nR5CT900XYZ1\tunauthorized\n\n"
    }

    /// Sample device serial for testing
    pub fn test_serial() -> &'static str {
        "R5CT900XYZ1"
    }
}
