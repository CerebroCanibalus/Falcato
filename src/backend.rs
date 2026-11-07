use crate::ast::Programa;
use crate::error::Errores;

/// Trait abstracto para backends de codegen.
///
/// Falcato soporta múltiples backends a través de este trait.
/// Hoy: Cranelift (nativo x86_64). Mañana: WASM, LLVM, backend propio.
///
/// Estrategia: el backend es intercambiable. El CLI y el resolver
/// trabajan contra este trait, no contra Codegen directamente.
pub trait BackendFalcato {
    /// Crea una nueva instancia del backend para un módulo con nombre.
    fn nuevo(nombre_modulo: &str) -> Result<Self, String>
    where
        Self: Sized;

    /// Compila un programa completo (AST → código objeto).
    fn compilar_programa(&mut self, nombre_modulo: &str, programa: &Programa) -> Result<(), Errores>;

    /// Finaliza el módulo y escribe el código objeto a un archivo `.o`.
    fn escribir_objeto(&mut self, ruta: &str) -> Result<(), String>;
}
