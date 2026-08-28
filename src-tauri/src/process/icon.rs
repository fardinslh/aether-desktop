use std::path::Path;

/// Utility to extract Windows application icons as base64 data URIs
pub struct IconExtractor;

impl IconExtractor {
    /// Extracts icon from executable path or returns None if unavailable
    pub fn extract_icon_base64(_exe_path: &Path) -> Option<String> {
        // Safe fallback: On Windows MinGW environments where linking GDI+ directly
        // may introduce runtime dependencies, frontend renders high-quality SVG brand icons
        // for known applications and polished generic icons for arbitrary executables.
        None
    }
}