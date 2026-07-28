//! # PlatformLinker — Configuración del linker por plataforma
//!
//! Centraliza qué linker usar, qué flags, qué librerías,
//! y qué entry point según la plataforma.

/// Comando y flags del linker para una plataforma.
#[derive(Debug, Clone)]
pub struct PlatformLinker {
    /// Nombre del linker (o ruta)
    pub cmd: String,
    /// Flags estándar (siempre incluidos)
    pub flags: Vec<String>,
    /// Librerías a linkear
    pub libs: Vec<String>,
    /// Entry point
    pub entry: String,
    /// Objeto de objeto (ruta al trampolín si existe)
    pub extra_objs: Vec<String>,
    /// Ruta a la runtime library (falcato_runtime staticlib)
    pub runtime_lib: Option<String>,
}

impl PlatformLinker {
    // ============================================================
    // Windows — MSVC link.exe
    // ============================================================
    pub fn windows() -> Self {
        Self {
            cmd: "link.exe".to_string(),
            flags: vec![
                "/SUBSYSTEM:CONSOLE".to_string(),
                "/ENTRY:principal".to_string(),
                "/NOLOGO".to_string(),
            ],
            libs: vec![
                "libcmt.lib".to_string(),
                "ucrt.lib".to_string(),
                "legacy_stdio_definitions.lib".to_string(),
                "vcruntime.lib".to_string(),
                "kernel32.lib".to_string(),
                "user32.lib".to_string(),
                "gdi32.lib".to_string(),
                "ws2_32.lib".to_string(),
                "ntdll.lib".to_string(),
                "userenv.lib".to_string(),
            ],
            entry: "principal".to_string(),
            extra_objs: vec!["lib/trampolin_win32.obj".to_string()],
            runtime_lib: Some("lib/falcato_runtime/target/release/falcato_runtime.lib".to_string()),
        }
    }

    // ============================================================
    // Linux — GCC
    // ============================================================
    pub fn linux() -> Self {
        Self {
            cmd: "gcc".to_string(),
            flags: vec![],
            libs: vec![
                "-lpthread".to_string(),
                "-lm".to_string(),
            ],
            entry: "_start".to_string(), // el entry point lo maneja gcc
            extra_objs: vec![],
            runtime_lib: Some("lib/falcato_runtime/target/release/libfalcato_runtime.a".to_string()),
        }
    }

    // ============================================================
    // macOS — Clang (o gcc)
    // ============================================================
    pub fn macos() -> Self {
        Self {
            cmd: "clang".to_string(),
            flags: vec![],
            libs: vec![
                "-lpthread".to_string(),
                "-lm".to_string(),
            ],
            entry: "_main".to_string(), // macOS entry (manejado por clang)
            extra_objs: vec![],
            runtime_lib: Some("lib/falcato_runtime/target/release/libfalcato_runtime.a".to_string()),
        }
    }

    /// Retorna el linker para un target triple.
    pub fn for_target(target: &str) -> Self {
        if target.contains("windows") {
            Self::windows()
        } else if target.contains("darwin") || target.contains("apple") {
            Self::macos()
        } else {
            Self::linux()
        }
    }

    /// Retorna el linker para el OS actual.
    pub fn for_current_os() -> Self {
        #[cfg(target_os = "windows")]
        { Self::windows() }
        #[cfg(target_os = "linux")]
        { Self::linux() }
        #[cfg(target_os = "macos")]
        { Self::macos() }
    }
}
