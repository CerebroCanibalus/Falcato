use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature};
use cranelift_codegen::ir::types;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::Configurable;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::*;
use crate::backend::BackendFalcato;
use crate::codegen_helpers::CFunctionCache;
use crate::platform::{self, CodegenCtx, BuiltinRegistry, PlatformRuntime};
use crate::error::{Errores, ErrorCompilador, CategoriaError};
use crate::span::Span;

use std::collections::HashMap;

// SubmÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulos ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â cada uno aÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â±ade funciones en impl Codegen { ... }
pub mod builtins;
pub mod expresiones;
pub mod funciones;
pub mod generics;
pub mod sentencias;
pub mod tipos;

/// InformaciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n de layout de un struct para codegen
#[derive(Debug, Clone)]
pub struct LayoutStruct {
    pub(crate) nombre: String,
    pub(crate) tamano: u32,
    pub(crate) alineacion: u32,
    /// Offset de cada campo en bytes
    pub(crate) offsets: HashMap<String, u32>,
    /// Tipo de cada campo
    pub(crate) tipos: HashMap<String, Tipo>,
    /// Fase 15B: campos de bits ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â nombre ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¾Ãƒâ€šÃ‚Â¢ (offset_bits, ancho_bits)
    pub(crate) bitfields: HashMap<String, (u32, u32)>,
    /// true si es un struct de bitfields (respaldado por un solo entero)
    pub(crate) es_bitfield: bool,
}

/// InformaciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n de layout de un enum para codegen
#[derive(Debug, Clone)]
pub struct LayoutEnum {
    pub(crate) nombre: String,
    pub(crate) tamano: u32,
    pub(crate) alineacion: u32,
    pub(crate) tag_tamano: u32,
    pub(crate) datos_offset: u32,
    /// Tag de cada variante
    pub(crate) variantes: HashMap<String, u32>,
    /// Tipos de datos de cada variante (si tiene)
    pub(crate) tipos_datos: HashMap<String, Vec<Tipo>>,
}

/// Generador de cÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³digo con Cranelift
pub struct Codegen {
    pub(crate) module: ObjectModule,
    pub(crate) funciones: HashMap<String, cranelift_module::FuncId>,
    pub(crate) funciones_genericas: HashMap<String, FuncionDecl>,
    pub(crate) instanciaciones: HashMap<(String, Vec<String>), cranelift_module::FuncId>,
    pub(crate) structs: HashMap<String, LayoutStruct>,
    pub(crate) enums: HashMap<String, LayoutEnum>,
    pub(crate) errores: Errores,
    /// Cache de funciones C externas (para no declarar repetido)
    pub(crate) cache: CFunctionCache,
    /// Registry de builtins segÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Âºn plataforma
    pub(crate) registry: BuiltinRegistry,
    /// Convención de llamada nativa (WindowsFastcall en Win, SystemV en POSIX)
    pub(crate) call_conv: CallConv,
    pub(crate) contador_strings: u32,
    pub(crate) contador_variables: u32,
    pub(crate) contador_closures: u32,
    pub(crate) closures_pendientes: Vec<ClosurePendiente>,
    pub(crate) hilos_pendientes: Vec<HiloPendiente>,
    pub(crate) executor_pool_var: Option<String>,
    pub(crate) executor_worker_generado: bool,
}

/// Info para compilar un closure diferidamente
pub(crate) struct ClosurePendiente {
    pub(crate) nombre: String,
    pub(crate) params: Vec<(String, Tipo)>,
    pub(crate) cuerpo: Expresion,
    pub(crate) capturas: Vec<(String, Tipo)>, // variables capturadas del scope externo
    pub(crate) retorno: Tipo,
}

/// Info para compilar un hilo (lanzar) diferidamente
pub(crate) struct HiloPendiente {
    pub(crate) nombre: String,       // __hilo_N
    pub(crate) llamada: Llamada,     // la llamada a la funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n target
    pub(crate) func_id_module: cranelift_module::FuncId, // FuncId ya declarada en el mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulo
    pub(crate) arg_types: Vec<cranelift_codegen::ir::Type>, // tipos Cranelift de cada arg
}

impl Codegen {
    pub fn nuevo(nombre_modulo: &str) -> Result<Self, String> {
        let mut flag_builder = cranelift_codegen::settings::builder();
        flag_builder.set("use_colocated_libcalls", "false")
            .map_err(|e| format!("Error en flags: {}", e))?;
        flag_builder.set("is_pic", "true")
            .map_err(|e| format!("Error en flags: {}", e))?;
        
        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("No se pudo detectar ISA nativo: {}", e))?;
        
        let isa = isa_builder.finish(
            cranelift_codegen::settings::Flags::new(flag_builder)
        ).map_err(|e| format!("Error al crear ISA: {}", e))?;

        let mut builder = ObjectBuilder::new(
            isa,
            nombre_modulo.as_bytes().to_vec(),
            cranelift_module::default_libcall_names(),
        ).map_err(|e| format!("Error al crear builder: {}", e))?;

        builder.per_function_section(true);

        let module = ObjectModule::new(builder);

        Ok(Self {
            module,
            funciones: HashMap::new(),
            funciones_genericas: HashMap::new(),
            instanciaciones: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            errores: Errores::nuevo(),
            cache: CFunctionCache::nuevo(),
            registry: platform::current_registry(),
            call_conv: platform::current_runtime().call_conv_default(),
            contador_strings: 0,
            contador_variables: 0,
            contador_closures: 0,
            closures_pendientes: Vec::new(),
            hilos_pendientes: Vec::new(),
            executor_pool_var: None,
            executor_worker_generado: false,
        }.registrar_builtins_codegen())
    }

    /// Registra enums built-in (Resultado<T,E>)
    fn registrar_builtins_codegen(mut self) -> Self {
        // Resultado<T, E>: tag (I32) + datos (max de T y E)
        // Por ahora, asumimos que T y E son Entero32 (4 bytes cada uno)
        // En monomorfizaciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n se especializarÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡
        let tag_tamano = 4u32;
        let datos_offset = tag_tamano;
        let max_tamano_datos = 4u32; // Asumimos Entero32 por defecto
        let tamano_total = datos_offset + max_tamano_datos;
        let alineacion = 4u32;
        let padding = (alineacion - (tamano_total % alineacion)) % alineacion;
        let tamano_alineado = tamano_total + padding;

        let mut variantes = HashMap::new();
        variantes.insert("Exito".to_string(), 0);
        variantes.insert("Error".to_string(), 1);

        let mut tipos_datos = HashMap::new();
        tipos_datos.insert("Exito".to_string(), vec![Tipo::Entero32]);
        tipos_datos.insert("Error".to_string(), vec![Tipo::Entero32]);

        self.enums.insert("Resultado".to_string(), LayoutEnum {
            nombre: "Resultado".to_string(),
            tamano: tamano_alineado,
            alineacion,
            tag_tamano,
            datos_offset,
            variantes,
            tipos_datos,
        });

        self
    }

    /// ConvenciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n de llamada por defecto segÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Âºn el target nativo.
    /// En Windows x64 se usa WindowsFastcall; en otros, SystemV.
    fn call_conv_default(&self) -> CallConv {
        self.call_conv
    }

    /// Genera un ID ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Âºnico de variable SSA para el builder actual.
    fn nueva_variable(&mut self) -> Variable {
        let id = self.contador_variables;
        self.contador_variables += 1;
        Variable::from_u32(id)
    }

    /// Crea un CodegenCtx para llamar builtins de plataforma.
    /// La codegen NUNCA debe hacer #[cfg(target_os)] ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â usa esta API.
    fn ctx(&mut self) -> CodegenCtx<'_> {
        CodegenCtx::new(&mut self.cache, &mut self.module, &self.registry)
    }

    /// Helper para builtins simples (via registry).
    fn platform_call_void(&mut self, name: &str, builder: &mut FunctionBuilder, args: &[cranelift_codegen::ir::Value]) {
        self.ctx().call_void(name, builder, args);
    }

    fn platform_call_ret(&mut self, name: &str, builder: &mut FunctionBuilder, args: &[cranelift_codegen::ir::Value]) -> cranelift_codegen::ir::Value {
        self.ctx().call_ret(name, builder, args)
    }

    pub fn compilar_programa(&mut self,
        programa: &Programa,
    ) -> Result<(), Errores> {
        // Obtener todas las declaraciones con prefijo de mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulo
        let todas: Vec<(String, &Declaracion)> = programa.declaraciones.iter()
            .flat_map(|d| self.aplanar_con_prefijo("", d))

            .collect();

        // Primera pasada: registrar structs y enums
        for (_prefijo, decl) in &todas {
            match decl {
                Declaracion::Estructural(s) => self.registrar_struct(s),
                Declaracion::Enumeracion(e) => self.registrar_enum(e),
                _ => {}
            }
        }

        // Segunda pasada: declarar funciones (no genÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©ricas)
        for (_prefijo, decl) in &todas {
            if let Declaracion::Funcion(func) = decl {
                if func.parametros_genericos.is_empty() {
                    self.declarar_funcion(func);
                } else {
                    // Almacenar funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n genÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©rica para monomorfizaciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n
                    self.funciones_genericas.insert(func.nombre.clone(), func.clone());
                }
            }
        }

        // Registrar alias cualificados (modulo::funcion ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¾Ãƒâ€šÃ‚Â¢ FuncId)
        // ANTES de compilar cuerpos, para que las llamadas cualificadas funcionen
        let alias: Vec<(String, String)> = todas.iter()
            .filter(|(prefijo, _)| !prefijo.is_empty())
            .filter_map(|(prefijo, decl)| {
                if let Declaracion::Funcion(func) = decl {
                    let nombre_cualif = format!("{}::{}", prefijo.trim_end_matches("::"), func.nombre)
                        .trim_start_matches("::").to_string();
                    Some((nombre_cualif, func.nombre.clone()))
                } else {
                    None
                }
            }).collect();

        for (nombre_cualif, nombre_simple) in &alias {
            if let Some(func_id) = self.funciones.get(nombre_simple).copied() {
                self.funciones.entry(nombre_cualif.clone()).or_insert(func_id);
            }
        }

        // Tercera pasada: compilar cuerpos (solo funciones no genÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©ricas)
        for (_prefijo, decl) in &todas {
            if let Declaracion::Funcion(func) = decl {
                if func.parametros_genericos.is_empty() {
                    if let Err(_) = self.compilar_funcion(func) {
                        // Error ya agregado a self.errores
                    }
                }
            }
        }

        // Cuarta pasada: compilar closures pendientes (funciones anÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³nimas)
        self.compilar_closures_pendientes();

        // Quinta pasada: compilar wrappers de hilos (lanzar)
        self.compilar_hilos_pendientes();

        if self.errores.hay_errores() {
            Err(self.errores.clone())
        } else {
            Ok(())
        }
    }

    /// Compila una declaraciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n `prueba` como una funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n void sin parÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡metros
    /// Compila closures pendientes como funciones independientes en el mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulo
    fn compilar_closures_pendientes(&mut self) {
        // Tomar ownership de la lista para evitar borrow conflict
        let closures = std::mem::take(&mut self.closures_pendientes);

        for closure in closures {
            let func_id = match self.funciones.get(&closure.nombre).copied() {
                Some(id) => id,
                None => continue,
            };

            let mut ctx = self.module.make_context();
            let mut func_ctx = FunctionBuilderContext::new();

            // Reconstruir firma (SIEMPRE env_ptr como primer param)
            let mut sig = Signature::new(self.call_conv_default());
            sig.params.push(AbiParam::new(types::I64)); // env_ptr siempre presente
            for (_, tipo) in &closure.params {
                sig.params.push(AbiParam::new(self.tipo_a_cranelift(tipo)));
            }
            sig.returns.push(AbiParam::new(self.tipo_a_cranelift(&closure.retorno)));
            ctx.func.signature = sig;

            {
                let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
                let entry_block = builder.create_block();
                builder.append_block_params_for_function_params(entry_block);
                builder.switch_to_block(entry_block);
                builder.seal_block(entry_block);

                // Crear variables para parÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡metros
                let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

                // env_ptr SIEMPRE es el primer parÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡metro (offset 0 en la firma)
                let env_ptr_val = builder.block_params(entry_block)[0];

                // Si hay capturas, cargarlas desde env_ptr
                if !closure.capturas.is_empty() {
                    for (i, (nombre_cap, tipo_cap)) in closure.capturas.iter().enumerate() {
                        let tam = self.tamano_tipo(tipo_cap);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tam,
                                0,
                            )
                        );
                        // env_ptr contiene punteros a las variables capturadas
                        let offset = (i * 8) as i32;
                        let cap_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), env_ptr_val, offset);
                        let cranelift_tipo = self.tipo_a_cranelift(tipo_cap);
                        let cap_val = builder.ins().load(cranelift_tipo, cranelift_codegen::ir::MemFlags::new(), cap_ptr, 0);
                        builder.ins().stack_store(cap_val, slot, 0);
                        variables.insert(nombre_cap.clone(), (slot, tipo_cap.clone(), crate::ast::Articulo::La));
                    }
                }

                // ParÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡metros del closure (empiezan en index 1, despuÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©s de env_ptr)
                let mut param_idx = 1;
                for (nombre_param, tipo_param) in &closure.params {
                    let tam = self.tamano_tipo(tipo_param);
                    let slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            tam,
                            0,
                        )
                    );
                    let val = builder.block_params(entry_block)[param_idx];
                    builder.ins().stack_store(val, slot, 0);
                    variables.insert(nombre_param.clone(), (slot, tipo_param.clone(), crate::ast::Articulo::La));
                    param_idx += 1;
                }

                // Compilar cuerpo del closure
                let _span_dummy = crate::span::Span::vacio();
                match self.compilar_expresion(&closure.cuerpo, &mut builder, &variables) {
                    Ok(resultado) => {
                        builder.ins().return_(&[resultado]);
                    }
                    Err(_) => {
                        // Error ya reportado
                        let cero = builder.ins().iconst(types::I32, 0);
                        builder.ins().return_(&[cero]);
                    }
                }

                builder.finalize();
            }

            // Definir la funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n en el mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulo
            let _ = self.module.define_function(func_id, &mut ctx);
        }
    }

    fn registrar_struct(&mut self, s: &EstructuralDecl) {
        let mut offsets = HashMap::new();
        let mut tipos = HashMap::new();
        let mut bitfields = HashMap::new();
        let mut offset_actual = 0u32;
        let mut alineacion_max = 1u32;

        // Fase 15B: struct de bitfields ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â respaldado por un solo entero
        if !s.campos_bits.is_empty() && s.campos.is_empty() {
            let total_bits: u32 = s.campos_bits.iter().map(|c| c.ancho_bits).sum();
            // Determinar tipo de respaldo: u8/u16/u32/u64
            let (tamano, alineacion) = if total_bits <= 8 {
                (1u32, 1u32)
            } else if total_bits <= 16 {
                (2, 2)
            } else if total_bits <= 32 {
                (4, 4)
            } else {
                (8, 8)
            };
            for campo_bit in &s.campos_bits {
                bitfields.insert(campo_bit.nombre.clone(), (campo_bit.offset_bits, campo_bit.ancho_bits));
            }
            self.structs.insert(s.nombre.clone(), LayoutStruct {
                nombre: s.nombre.clone(),
                tamano,
                alineacion,
                offsets,
                tipos,
                bitfields,
                es_bitfield: true,
            });
            return;
        }

        for campo in &s.campos {
            let tamano = self.tamano_tipo(&campo.tipo);
            let alineacion = tamano; // C ABI: alineaciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n = tamaÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â±o (simplificado)
            
            // Alinear offset_actual
            let padding = (alineacion - (offset_actual % alineacion)) % alineacion;
            offset_actual += padding;
            
            offsets.insert(campo.nombre.clone(), offset_actual);
            tipos.insert(campo.nombre.clone(), campo.tipo.clone());
            offset_actual += tamano;
            
            if alineacion > alineacion_max {
                alineacion_max = alineacion;
            }
        }

        // Alinear tamaÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â±o total del struct
        let padding_final = (alineacion_max - (offset_actual % alineacion_max)) % alineacion_max;
        let tamano_total = offset_actual + padding_final;

        self.structs.insert(s.nombre.clone(), LayoutStruct {
            nombre: s.nombre.clone(),
            tamano: tamano_total,
            alineacion: alineacion_max,
            offsets,
            tipos,
            bitfields,
            es_bitfield: false,
        });
    }

    fn registrar_enum(&mut self, e: &EnumeracionDecl) {
        let mut variantes = HashMap::new();
        let mut tipos_datos = HashMap::new();
        let mut max_tamano_datos = 0u32;
        let mut tag: u32 = 0;

        for variante in &e.variantes {
            variantes.insert(variante.nombre.clone(), tag);
            
            let tamano = if let Some(ref campos) = variante.datos {
                let tipos: Vec<Tipo> = campos.iter().map(|(_, t)| t.clone()).collect();
                let tam = campos.iter().map(|(_, t)| self.tamano_tipo(t)).sum();
                tipos_datos.insert(variante.nombre.clone(), tipos);
                tam
            } else {
                0
            };
            
            if tamano > max_tamano_datos {
                max_tamano_datos = tamano;
            }
            
            tag += 1;
        }

        // Layout: tag (I32, 4 bytes) + datos (max tamaÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â±o de variantes)
        let tag_tamano = 4u32;
        let datos_offset = tag_tamano;
        let tamano_total = datos_offset + max_tamano_datos;
        // Alinear a 4 bytes
        let alineacion = 4u32;
        let padding = (alineacion - (tamano_total % alineacion)) % alineacion;
        let tamano_alineado = tamano_total + padding;

        self.enums.insert(e.nombre.clone(), LayoutEnum {
            nombre: e.nombre.clone(),
            tamano: tamano_alineado,
            alineacion,
            tag_tamano,
            datos_offset,
            variantes,
            tipos_datos,
        });
    }

    fn declarar_funcion_externa(
        &mut self,
        nombre: &str,
        params: &[cranelift_codegen::ir::Type],
        retorno: Option<cranelift_codegen::ir::Type>,
    ) -> cranelift_module::FuncId {
        let mut sig = Signature::new(self.call_conv_default());
        for &p in params {
            sig.params.push(AbiParam::new(p));
        }
        if let Some(r) = retorno {
            sig.returns.push(AbiParam::new(r));
        }
        
        match self.module.declare_function(nombre, Linkage::Import, &sig) {
            Ok(id) => {
                self.funciones.insert(nombre.to_string(), id);
                // TambiÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©n registrar en cache para compatibilidad con platform layer
                id
            }
            Err(_) => {
                *self.funciones.get(nombre).expect("funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n externa no encontrada")
            }
        }
    }

    fn asegurar_funcion_c(
        &mut self,
        nombre: &str,
        params: &[cranelift_codegen::ir::Type],
        retorno: Option<cranelift_codegen::ir::Type>,
    ) -> cranelift_module::FuncId {
        // Buscar primero en funciones internas
        if let Some(&id) = self.funciones.get(nombre) {
            return id;
        }
        // Buscar en cache de funciones C externas
        if let Some(id) = self.cache.obtener(nombre) {
            return id;
        }

        // Intentar remapear nombre Windows ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¾Ãƒâ€šÃ‚Â¢ nombre correcto segÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Âºn plataforma
        let (c_name, c_params, c_ret) = self.registry.remap(nombre, params, retorno);

        // Comprobar si el nombre remapeado ya estÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡ en funciones
        if c_name != nombre {
            if let Some(&id) = self.funciones.get(&c_name) {
                return id;
            }
        }

        self.declarar_funcion_externa(&c_name, &c_params, c_ret)
    }

    // ============================================================
    // Helpers FFI (malloc/free/realloc/memcpy/strlen)
    // ============================================================

    fn llamar_malloc(
        &mut self,
        builder: &mut FunctionBuilder,
        tamano: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        let func_id = self.asegurar_funcion_c("malloc", &[types::I64], Some(types::I64));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let call = builder.ins().call(func_ref, &[tamano]);
        builder.inst_results(call)[0]
    }

    fn llamar_free(
        &mut self,
        builder: &mut FunctionBuilder,
        ptr: cranelift_codegen::ir::Value,
    ) {
        let func_id = self.asegurar_funcion_c("free", &[types::I64], None);
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        builder.ins().call(func_ref, &[ptr]);
    }

    fn llamar_realloc(
        &mut self,
        builder: &mut FunctionBuilder,
        ptr: cranelift_codegen::ir::Value,
        tamano: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        let func_id = self.asegurar_funcion_c("realloc", &[types::I64, types::I64], Some(types::I64));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let call = builder.ins().call(func_ref, &[ptr, tamano]);
        builder.inst_results(call)[0]
    }

    fn llamar_memcpy(
        &mut self,
        builder: &mut FunctionBuilder,
        dest: cranelift_codegen::ir::Value,
        src: cranelift_codegen::ir::Value,
        n: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        let func_id = self.asegurar_funcion_c("memcpy", &[types::I64, types::I64, types::I64], Some(types::I64));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let call = builder.ins().call(func_ref, &[dest, src, n]);
        builder.inst_results(call)[0]
    }

    fn llamar_strlen(
        &mut self,
        builder: &mut FunctionBuilder,
        ptr: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        let func_id = self.asegurar_funcion_c("strlen", &[types::I64], Some(types::I64));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let call = builder.ins().call(func_ref, &[ptr]);
        builder.inst_results(call)[0]
    }

    // ============================================================
    // Helpers para descriptor Texto/Vector: { ptr, len, cap }
    // ============================================================

    const OFFSET_PTR: i32 = 0;
    const OFFSET_LEN: i32 = 8;
    const OFFSET_CAP: i32 = 16;
    const TAMANO_DESCRIPTOR: i64 = 24;

    fn descriptor_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
    ) -> cranelift_codegen::ir::Value {
        let tamano = builder.ins().iconst(types::I64, Self::TAMANO_DESCRIPTOR);
        let ptr = self.llamar_malloc(builder, tamano);
        let cero = builder.ins().iconst(types::I64, 0);
        let flags = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags, cero, ptr, Self::OFFSET_PTR);
        builder.ins().store(flags, cero, ptr, Self::OFFSET_LEN);
        builder.ins().store(flags, cero, ptr, Self::OFFSET_CAP);
        ptr
    }

    fn cargar_campo_descriptor(
        &self,
        builder: &mut FunctionBuilder,
        ptr: cranelift_codegen::ir::Value,
        offset: i32,
    ) -> cranelift_codegen::ir::Value {
        builder.ins().load(
            types::I64,
            cranelift_codegen::ir::MemFlags::new(),
            ptr,
            offset,
        )
    }

    fn guardar_campo_descriptor(
        &self,
        builder: &mut FunctionBuilder,
        ptr: cranelift_codegen::ir::Value,
        offset: i32,
        valor: cranelift_codegen::ir::Value,
    ) {
        builder.ins().store(
            cranelift_codegen::ir::MemFlags::new(),
            valor,
            ptr,
            offset,
        );
    }

    fn declarar_funcion(
        &mut self,
        func: &FuncionDecl,
    ) {
        let mut sig = Signature::new(self.call_conv_default());

        // Tipo de retorno
        if let Some(ref ret) = func.retorno {
            let tipo = self.tipo_a_cranelift(ret);
            sig.returns.push(AbiParam::new(tipo));
        }

        // ParÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¡metros
        for param in &func.parametros {
            let tipo = self.tipo_a_cranelift(&param.tipo);
            sig.params.push(AbiParam::new(tipo));
        }

        let linkage = if func.es_insegura && func.cuerpo.sentencias.is_empty() {
            Linkage::Import
        } else {
            Linkage::Export
        };

        let func_id = self.module.declare_function(
            &func.nombre,
            linkage,
            &sig,
        ).unwrap_or_else(|_| {
            panic!("No se pudo declarar funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n '{}'", func.nombre)
        });
        
        self.funciones.insert(func.nombre.clone(), func_id);
    }


// Movido a codegen/sentencias.rs

    // Movido a codegen/expresiones.rs
    fn crear_modulo_dummy() -> ObjectModule {
        // Crear un mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulo dummy temporal
        let mut flag_builder = cranelift_codegen::settings::builder();
        let _ = flag_builder.set("use_colocated_libcalls", "false");
        let _ = flag_builder.set("is_pic", "true");
        
        let isa_builder = cranelift_native::builder().unwrap();
        let isa = isa_builder.finish(
            cranelift_codegen::settings::Flags::new(flag_builder)
        ).unwrap();

        let builder = ObjectBuilder::new(
            isa,
            b"dummy".to_vec(),
            cranelift_module::default_libcall_names(),
        ).unwrap();

        ObjectModule::new(builder)
    }

    /// Aplana declaraciones recursivamente (desciende en mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulos).
    /// Devuelve (prefijo_de_nombre, declaracion).
    /// Ej: un mÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³dulo "matematicas" con funciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n "suma" ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¾Ãƒâ€šÃ‚Â¢ ("matematicas::", Funcion("suma"))
    fn aplanar_con_prefijo<'a>(&self, prefijo: &str, decl: &'a Declaracion) -> Vec<(String, &'a Declaracion)> {
        match decl {
            Declaracion::Modulo(modulo) => {
                let mut resultado = Vec::new();
                let nuevo_prefijo = format!("{}::", modulo.nombre);
                for d in &modulo.contenido {
                    resultado.extend(self.aplanar_con_prefijo(&nuevo_prefijo, d));
                }
                resultado
            }
            _ => vec![(prefijo.to_string(), decl)],
        }
    }

    pub fn escribir_objeto(&mut self, ruta: &str) -> Result<(), String> {
        let dummy = Self::crear_modulo_dummy();
        let object = std::mem::replace(
            &mut self.module,
            dummy
        ).finish();

        let bytes = object.emit()
            .map_err(|e| format!("Error emitiendo objeto: {}", e))?;

        std::fs::write(ruta, bytes)
            .map_err(|e| format!("Error escribiendo archivo: {}", e))?;

        Ok(())
    }
}

/// ImplementaciÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â³n del trait BackendFalcato para el backend Cranelift.
impl BackendFalcato for Codegen {
    fn nuevo(nombre_modulo: &str) -> Result<Self, String> {
        Codegen::nuevo(nombre_modulo)
    }

    fn compilar_programa(&mut self, programa: &Programa) -> Result<(), Errores> {
        self.compilar_programa(programa)
    }

    fn escribir_objeto(&mut self, ruta: &str) -> Result<(), String> {
        self.escribir_objeto(ruta)
    }
}
