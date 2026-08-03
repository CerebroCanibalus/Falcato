//! # Platform Runtime Abstraction Layer
//!
//! Centraliza TODO el conocimiento de plataforma en un solo módulo.
//! La codegen NUNCA debe hacer `#[cfg(target_os)]` — consume esta API.
//!
//! ## Arquitectura
//!
//! Tres niveles de abstracción:
//!
//! 1. **BuiltinRegistry** (registry.rs): Tabla declarativa de nombre abstracto → función C.
//!    Para builtins simples: malloc, puts, socket, sleep, etc.
//!    Código: `registry.lookup("sleep")` → función C correcta según plataforma.
//!
//! 2. **PlatformRuntime** (traits.rs): Trait con primitivas de sincronización + builtins complejos.
//!    Para builtins con lógica diferente por plataforma: timestamp, channels, threads.
//!    Código: `platform.mutex_lock(ctx, builder, ptr)` → llamada correcta según plataforma.
//!
//! 3. **PlatformLinker** (linker.rs): Configuración del linker por plataforma.
//!
//! ## Cómo usar desde codegen.rs
//!
//! ```rust,ignore
//! use crate::platform::{self, CodegenCtx, BuiltinRegistry};
//!
//! // 1. Crear registry y contexto
//! let registry = BuiltinRegistry::for_current_os();
//! let mut ctx = CodegenCtx::new(&mut cache, &mut module, &registry);
//!
//! // 2. Builtins simples (via registry + CodegenCtx)
//! ctx.call_void("sleep", builder, &[ms_val]);
//!
//! // 3. Builtins complejos (via PlatformRuntime)
//! let runtime = platform::current_runtime();
//! let ts = runtime.timestamp(&mut ctx, builder);
//! runtime.mutex_lock(&mut ctx, builder, canal_ptr);
//! ```
//!
//! ## Agregar una nueva plataforma
//!
//! 1. Crear `src/platform/mi_os.rs`
//! 2. Implementar `PlatformRuntime` para `MiOsRuntime`
//! 3. Agregar entradas en `BuiltinRegistry::mi_os()`
//! 4. Agregar `#[cfg(target_os = "mi_os")]` en este archivo
//! 5. Agregar `pub use self::mi_os::MiOsRuntime;`
//!
//! **No tocar codegen.rs ni los otros archivos de plataforma.**

mod registry;
mod linker;
mod traits;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub use registry::BuiltinRegistry;
pub use traits::{PlatformRuntime, CodegenCtx};

/// Retorna el runtime de la plataforma actual.
pub fn current_runtime() -> impl PlatformRuntime {
    #[cfg(target_os = "windows")]
    { windows::WindowsRuntime }
    #[cfg(target_os = "linux")]
    { linux::LinuxRuntime }
    #[cfg(target_os = "macos")]
    { macos::MacOsRuntime }
}

/// Retorna el registry de builtins para el OS actual.
pub fn current_registry() -> BuiltinRegistry {
    #[cfg(target_os = "windows")]
    { BuiltinRegistry::windows() }
    #[cfg(target_os = "linux")]
    { BuiltinRegistry::linux() }
    #[cfg(target_os = "macos")]
    { BuiltinRegistry::macos() }
}
