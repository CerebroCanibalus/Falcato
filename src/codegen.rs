use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature};
use cranelift_codegen::ir::types;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::Configurable;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::*;
use crate::backend::BackendFalcato;
use crate::error::{Errores, ErrorCompilador, CategoriaError};
use crate::span::Span;

use std::collections::HashMap;

/// Información de layout de un struct para codegen
#[derive(Debug, Clone)]
pub struct LayoutStruct {
    pub nombre: String,
    pub tamano: u32,
    pub alineacion: u32,
    /// Offset de cada campo en bytes
    pub offsets: HashMap<String, u32>,
    /// Tipo de cada campo
    pub tipos: HashMap<String, Tipo>,
    /// Fase 15B: campos de bits — nombre → (offset_bits, ancho_bits)
    pub bitfields: HashMap<String, (u32, u32)>,
    /// true si es un struct de bitfields (respaldado por un solo entero)
    pub es_bitfield: bool,
}

/// Información de layout de un enum para codegen
#[derive(Debug, Clone)]
pub struct LayoutEnum {
    pub nombre: String,
    pub tamano: u32,
    pub alineacion: u32,
    pub tag_tamano: u32,
    pub datos_offset: u32,
    /// Tag de cada variante
    pub variantes: HashMap<String, u32>,
    /// Tipos de datos de cada variante (si tiene)
    pub tipos_datos: HashMap<String, Vec<Tipo>>,
}

/// Generador de código con Cranelift
pub struct Codegen {
    module: ObjectModule,
    funciones: HashMap<String, cranelift_module::FuncId>,
    funciones_genericas: HashMap<String, FuncionDecl>, // funciones genéricas sin compilar
    instanciaciones: HashMap<(String, Vec<String>), cranelift_module::FuncId>, // (nombre, [valores genéricos]) -> func_id
    structs: HashMap<String, LayoutStruct>,
    enums: HashMap<String, LayoutEnum>,
    errores: Errores,
    contador_strings: u32,  // Contador para evitar colisión de strings
    contador_variables: u32, // Contador para IDs únicos de variables SSA
    contador_closures: u32,  // Contador para funciones anónimas de closures
    closures_pendientes: Vec<ClosurePendiente>, // Closures por compilar después
    hilos_pendientes: Vec<HiloPendiente>, // Hilos (lanzar) por compilar después
    executor_pool_var: Option<String>, // Variable con pool ptr si estamos dentro de con_executor
    executor_worker_generado: bool, // __executor_worker ya fue generado
}

/// Info para compilar un closure diferidamente
struct ClosurePendiente {
    nombre: String,
    params: Vec<(String, Tipo)>,
    cuerpo: Expresion,
    capturas: Vec<(String, Tipo)>, // variables capturadas del scope externo
    retorno: Tipo,
}

/// Info para compilar un hilo (lanzar) diferidamente
struct HiloPendiente {
    nombre: String,       // __hilo_N
    llamada: Llamada,     // la llamada a la función target
    func_id_module: cranelift_module::FuncId, // FuncId ya declarada en el módulo
    arg_types: Vec<cranelift_codegen::ir::Type>, // tipos Cranelift de cada arg
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
        // En monomorfización se especializará
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

    /// Convención de llamada por defecto según el target nativo.
    /// En Windows x64 se usa WindowsFastcall; en otros, SystemV.
    fn call_conv_default(&self) -> CallConv {
        #[cfg(target_os = "windows")]
        { CallConv::WindowsFastcall }
        #[cfg(not(target_os = "windows"))]
        { CallConv::SystemV }
    }

    /// Genera un ID único de variable SSA para el builder actual.
    fn nueva_variable(&mut self) -> Variable {
        let id = self.contador_variables;
        self.contador_variables += 1;
        Variable::from_u32(id)
    }

    pub fn compilar_programa(&mut self,
        programa: &Programa,
    ) -> Result<(), Errores> {
        // Obtener todas las declaraciones con prefijo de módulo
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

        // Segunda pasada: declarar funciones (no genéricas)
        for (prefijo, decl) in &todas {
            if let Declaracion::Funcion(func) = decl {
                if func.parametros_genericos.is_empty() {
                    self.declarar_funcion(func);
                } else {
                    // Almacenar función genérica para monomorfización
                    self.funciones_genericas.insert(func.nombre.clone(), func.clone());
                }
            }
        }

        // Registrar alias cualificados (modulo::funcion → FuncId)
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

        // Tercera pasada: compilar cuerpos (solo funciones no genéricas)
        for (_prefijo, decl) in &todas {
            if let Declaracion::Funcion(func) = decl {
                if func.parametros_genericos.is_empty() {
                    if let Err(_) = self.compilar_funcion(func) {
                        // Error ya agregado a self.errores
                    }
                }
            }
        }

        // Cuarta pasada: compilar closures pendientes (funciones anónimas)
        self.compilar_closures_pendientes();

        // Quinta pasada: compilar wrappers de hilos (lanzar)
        self.compilar_hilos_pendientes();

        if self.errores.hay_errores() {
            Err(self.errores.clone())
        } else {
            Ok(())
        }
    }

    /// Compila una declaración `prueba` como una función void sin parámetros
    /// Compila closures pendientes como funciones independientes en el módulo
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

                // Crear variables para parámetros
                let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

                // env_ptr SIEMPRE es el primer parámetro (offset 0 en la firma)
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

                // Parámetros del closure (empiezan en index 1, después de env_ptr)
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
                let span_dummy = crate::span::Span::vacio();
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

            // Definir la función en el módulo
            let _ = self.module.define_function(func_id, &mut ctx);
        }
    }

    fn registrar_struct(&mut self, s: &EstructuralDecl) {
        let mut offsets = HashMap::new();
        let mut tipos = HashMap::new();
        let mut bitfields = HashMap::new();
        let mut offset_actual = 0u32;
        let mut alineacion_max = 1u32;

        // Fase 15B: struct de bitfields — respaldado por un solo entero
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
            let alineacion = tamano; // C ABI: alineación = tamaño (simplificado)
            
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

        // Alinear tamaño total del struct
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

        // Layout: tag (I32, 4 bytes) + datos (max tamaño de variantes)
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
                id
            }
            Err(_) => {
                // Si ya existe, recuperar el ID existente
                *self.funciones.get(nombre).expect("función externa no encontrada")
            }
        }
    }

    fn asegurar_funcion_c(
        &mut self,
        nombre: &str,
        params: &[cranelift_codegen::ir::Type],
        retorno: Option<cranelift_codegen::ir::Type>,
    ) -> cranelift_module::FuncId {
        if let Some(&id) = self.funciones.get(nombre) {
            id
        } else {
            self.declarar_funcion_externa(nombre, params, retorno)
        }
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

        // Parámetros
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
            panic!("No se pudo declarar función '{}'", func.nombre)
        });
        
        self.funciones.insert(func.nombre.clone(), func_id);
    }

    fn compilar_funcion(&mut self,
        func: &FuncionDecl,
    ) -> Result<(), ()> {
        // Si es FFI sin cuerpo, no compilar
        if func.es_insegura && func.cuerpo.sentencias.is_empty() {
            return Ok(());
        }

        // Fase 18E: fut función → state machine transform
        if func.es_futuro {
            return self.compilar_funcion_futuro(func);
        }

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        // Crear firma para la función
        let mut sig = Signature::new(self.call_conv_default());
        if let Some(ref ret) = func.retorno {
            sig.returns.push(AbiParam::new(self.tipo_a_cranelift(ret)));
        }
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }

        // Asignar firma al contexto
        ctx.func.signature = sig.clone();

        // Crear builder
        let mut builder = FunctionBuilder::new(
            &mut ctx.func,
            &mut func_ctx
        );

        let entry_block = builder.create_block();

        // Añadir parámetros al bloque de entrada ANTES de cualquier instrucción
        for param in &func.parametros {
            let tipo = self.tipo_a_cranelift(&param.tipo);
            builder.append_block_param(entry_block, tipo);
        }

        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Variables locales: nombre → (slot, tipo, artículo)
        let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

        // Parámetros como variables
        for (i, param) in func.parametros.iter().enumerate() {
            let val = builder.block_params(entry_block)[i];
            let tamano = self.tamano_tipo(&param.tipo);
            let slot = builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    tamano,
                    0,
                )
            );
            builder.ins().stack_store(val, slot, 0);
            variables.insert(param.nombre.clone(), (slot, param.tipo.clone(), param.articulo));
        }

        // Compilar sentencias
        for sentencia in &func.cuerpo.sentencias {
            self.compilar_sentencia(
                sentencia,
                &mut builder,
                &mut variables,
                &func.span,
            )?;
        }

        // TODO: Drop automático (Fase 12B)
        // El drop automático requiere análisis de flujo de control para insertar
        // free() antes de cada retorno. Por ahora, el programador debe llamar
        // a texto_liberar() / vector_liberar() manualmente.
        // 
        // Limitación conocida de Fase 12A:
        // - Variables owned de tipos heap (Texto, Vector) no se liberan automáticamente
        // - El programador debe llamar a liberar() manualmente
        // - Esto se resolverá en Fase 12B con análisis de CFG

        // Si no hay retorno explícito, añadir retorno void
        if func.retorno.is_none() {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        // Definir función en el módulo (usar func_id existente o declarar nueva)
        let func_id = match self.funciones.get(&func.nombre).copied() {
            Some(id) => id,
            None => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    10,
                    func.span.clone(),
                    format!("Función '{}' no declarada previamente", func.nombre),
                ));
                return Err(());
            }
        };

        self.module.define_function(func_id, &mut ctx)
            .map_err(|e| {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    10,
                    func.span.clone(),
                    format!("Error definiendo función: {}", e),
                ));
            })?;

        Ok(())
    }

    // ============================================================
    // Fase 18E: State machine transform para fut función
    // ============================================================

    /// Compila una `fut función` como state machine poll-based.
    /// Genera:
    /// - `__init_NOMBRE(args...) -> i64`: malloc struct + init state=0 + store params
    /// - `__poll_NOMBRE(ptr: i64) -> i64`: switch on state, returns 0=Pending, 1=Ready
    /// - `NOMBRE(args...) -> T`: wrapper que hace init + poll loop (para uso sync)
    fn compilar_funcion_futuro(&mut self, func: &FuncionDecl) -> Result<(), ()> {
        use crate::futuros;

        let analisis = futuros::analizar_futuro(func);
        let tamano_struct = futuros::tamano_struct_futuro(&analisis);
        let off_deadline = futuros::offset_deadline(&analisis);

        // Si no hay puntos de suspensión, compilar normal (no necesita state machine)
        if analisis.num_estados <= 1 {
            return self.compilar_funcion_normal(func);
        }

        // --- 1. Generar __init_NOMBRE ---
        self.generar_init_futuro(func, &analisis, tamano_struct)?;

        // --- 2. Generar __poll_NOMBRE ---
        self.generar_poll_futuro(func, &analisis, tamano_struct, off_deadline)?;

        // --- 3. Generar wrapper sync NOMBRE (init + poll loop con Sleep(1)) ---
        self.generar_wrapper_sync_futuro(func, &analisis)?;

        Ok(())
    }

    /// Genera `__init_NOMBRE(args...) -> i64`
    fn generar_init_futuro(
        &mut self,
        func: &FuncionDecl,
        analisis: &crate::futuros::AnalisisFuturo,
        tamano_struct: u32,
    ) -> Result<(), ()> {
        let nombre_init = format!("__init_{}", func.nombre);

        // Declarar la función en el módulo
        let mut sig = Signature::new(self.call_conv_default());
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }
        sig.returns.push(AbiParam::new(types::I64)); // retorna ptr

        let func_id = self.module.declare_function(&nombre_init, Linkage::Local, &sig)
            .expect("declarar __init");
        self.funciones.insert(nombre_init.clone(), func_id);

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry = builder.create_block();

        for param in &func.parametros {
            builder.append_block_param(entry, self.tipo_a_cranelift(&param.tipo));
        }

        builder.switch_to_block(entry);
        builder.seal_block(entry);

        // malloc(tamano_struct)
        let tam = builder.ins().iconst(types::I64, tamano_struct as i64);
        let ptr = self.llamar_malloc(&mut builder, tam);

        // state = 0 (offset 0, i32)
        let cero = builder.ins().iconst(types::I32, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, ptr, 0);

        // deadline = 0 (offset off_deadline, i64)
        let cero64 = builder.ins().iconst(types::I64, 0);
        let off_dl = crate::futuros::offset_deadline(analisis);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero64, ptr, off_dl as i32);

        // Store params en el struct
        let block_params = builder.block_params(entry).to_vec();
        for (i, param) in func.parametros.iter().enumerate() {
            if let Some(offset) = crate::futuros::offset_var(analisis, &param.nombre) {
                let val = block_params[i];
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, ptr, offset as i32);
            }
        }

        builder.ins().return_(&[ptr]);
        builder.finalize();

        self.module.define_function(func_id, &mut ctx).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo __init: {}", e),
            ));
        })?;

        Ok(())
    }

    /// Genera `__poll_NOMBRE(ptr: i64) -> i64`
    /// Returns 0 = Pending, 1 = Ready
    fn generar_poll_futuro(
        &mut self,
        func: &FuncionDecl,
        analisis: &crate::futuros::AnalisisFuturo,
        _tamano_struct: u32,
        off_deadline: u32,
    ) -> Result<(), ()> {
        let nombre_poll = format!("__poll_{}", func.nombre);

        let mut sig = Signature::new(self.call_conv_default());
        sig.params.push(AbiParam::new(types::I64)); // ptr
        sig.returns.push(AbiParam::new(types::I64)); // 0=Pending, 1=Ready

        let func_id = self.module.declare_function(&nombre_poll, Linkage::Local, &sig)
            .expect("declarar __poll");
        self.funciones.insert(nombre_poll.clone(), func_id);

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry = builder.create_block();
        builder.append_block_param(entry, types::I64); // ptr
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let ptr = builder.block_params(entry)[0];

        // Load state (offset 0, i32)
        let state = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), ptr, 0);

        // Crear bloques para cada estado + bloque de retorno
        let num_estados = analisis.num_estados;
        let mut bloques_estado: Vec<cranelift_codegen::ir::Block> = Vec::new();
        for _ in 0..num_estados {
            bloques_estado.push(builder.create_block());
        }
        let bloque_ready = builder.create_block();

        // Switch on state: cadena de if/else con sellado inmediato
        // Cada bloque tiene exactly 1 predecesor → sellar inmediato es seguro
        // NO sellar ningún bloque dos veces.
        let cero_i32 = builder.ins().iconst(types::I32, 0);
        let es_cero = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, state, cero_i32);
        let bloque_check1 = builder.create_block();
        builder.ins().brif(es_cero, bloques_estado[0], &[], bloque_check1, &[]);

        // Checks para estados 1..N-1
        builder.switch_to_block(bloque_check1);
        builder.seal_block(bloque_check1); // 1 predecesor: entry
        for i in 1..num_estados {
            let val_i = builder.ins().iconst(types::I32, i as i64);
            let es_i = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, state, val_i);
            if i < num_estados - 1 {
                let siguiente_check = builder.create_block();
                builder.ins().brif(es_i, bloques_estado[i], &[], siguiente_check, &[]);
                builder.switch_to_block(siguiente_check);
                builder.seal_block(siguiente_check); // 1 predecesor: check anterior
            } else {
                builder.ins().brif(es_i, bloques_estado[i], &[], bloque_ready, &[]);
            }
        }

        // GetTickCount64 para timers
        let get_tick_id = self.asegurar_funcion_c("GetTickCount64", &[], Some(types::I64));

        // Compilar cada estado
        for (estado_idx, segmento) in analisis.segmentos.iter().enumerate() {
            builder.switch_to_block(bloques_estado[estado_idx]);
            builder.seal_block(bloques_estado[estado_idx]); // 1 predecesor: dispatch chain

            // Para estados > 0: verificar timer (deadline)
            if estado_idx > 0 {
                let now = {
                    let func_ref = self.module.declare_func_in_func(get_tick_id, builder.func);
                    let call = builder.ins().call(func_ref, &[]);
                    builder.inst_results(call)[0]
                };
                let deadline = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), ptr, off_deadline as i32);
                let listo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual, now, deadline);
                let bloque_continuar = builder.create_block();
                let bloque_pending = builder.create_block();
                builder.ins().brif(listo, bloque_continuar, &[], bloque_pending, &[]);

                // Pending: return 0 (1 predecesor)
                builder.switch_to_block(bloque_pending);
                builder.seal_block(bloque_pending);
                let ret_cero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[ret_cero]);

                // Continuar (1 predecesor)
                builder.switch_to_block(bloque_continuar);
                builder.seal_block(bloque_continuar);
            }

            // Cargar variables del struct a stack slots locales
            let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

            for var in &analisis.vars_struct {
                let offset = crate::futuros::offset_var(analisis, &var.nombre).unwrap_or(0);
                let tipo_cranelift = self.tipo_a_cranelift(&var.tipo);
                let tamano = self.tamano_tipo(&var.tipo);
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        tamano,
                        0,
                    )
                );
                let val = builder.ins().load(tipo_cranelift, cranelift_codegen::ir::MemFlags::new(), ptr, offset as i32);
                builder.ins().stack_store(val, slot, 0);
                variables.insert(var.nombre.clone(), (slot, var.tipo.clone(), Articulo::El));
            }

            // Compilar sentencias del segmento (filtrar Retornar — el poll maneja sus propios returns)
            for sentencia in segmento {
                if matches!(sentencia, Sentencia::Retornar(_, _)) {
                    continue;
                }
                self.compilar_sentencia(sentencia, &mut builder, &mut variables, &func.span)?;
            }

            // Si hay una suspensión después de este estado:
            if estado_idx < analisis.suspensiones.len() {
                let susp = &analisis.suspensiones[estado_idx];

                // Extraer el valor de ms de dormir(ms)
                let ms_val = self.extraer_ms_de_suspension(&susp.expresion, &mut builder, &mut variables)?;

                // deadline = GetTickCount64() + ms
                let now = {
                    let func_ref = self.module.declare_func_in_func(get_tick_id, builder.func);
                    let call = builder.ins().call(func_ref, &[]);
                    builder.inst_results(call)[0]
                };
                let deadline_val = builder.ins().iadd(now, ms_val);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), deadline_val, ptr, off_deadline as i32);

                // Guardar variables locales de vuelta al struct
                for var in &analisis.vars_struct {
                    if let Some((slot, _, _)) = variables.get(&var.nombre) {
                        let slot = *slot;
                        let offset = crate::futuros::offset_var(analisis, &var.nombre).unwrap_or(0);
                        let tipo_cranelift = self.tipo_a_cranelift(&var.tipo);
                        let val = builder.ins().stack_load(tipo_cranelift, slot, 0);
                        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, ptr, offset as i32);
                    }
                }

                // state = estado_idx + 1
                let nuevo_state = builder.ins().iconst(types::I32, (estado_idx + 1) as i64);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), nuevo_state, ptr, 0);

                // return 0 (Pending)
                let ret_cero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[ret_cero]);
            } else {
                // Último estado: return 1 (Ready)
                let ret_uno = builder.ins().iconst(types::I64, 1);
                builder.ins().return_(&[ret_uno]);
            }
        }

        // Bloque ready (default) — 1 predecesor: último check
        builder.switch_to_block(bloque_ready);
        builder.seal_block(bloque_ready);
        let ret_uno = builder.ins().iconst(types::I64, 1);
        builder.ins().return_(&[ret_uno]);

        builder.finalize();

        self.module.define_function(func_id, &mut ctx).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo __poll: {}", e),
            ));
        })?;

        Ok(())
    }

    /// Genera wrapper sync: `NOMBRE(args) -> T` que hace init + poll loop
    fn generar_wrapper_sync_futuro(
        &mut self,
        func: &FuncionDecl,
        _analisis: &crate::futuros::AnalisisFuturo,
    ) -> Result<(), ()> {
        let nombre_init = format!("__init_{}", func.nombre);
        let nombre_poll = format!("__poll_{}", func.nombre);

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut sig = Signature::new(self.call_conv_default());
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }
        if let Some(ref ret) = func.retorno {
            sig.returns.push(AbiParam::new(self.tipo_a_cranelift(ret)));
        }
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry = builder.create_block();

        for param in &func.parametros {
            builder.append_block_param(entry, self.tipo_a_cranelift(&param.tipo));
        }

        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let block_params = builder.block_params(entry).to_vec();

        // Llamar __init(args...)
        let init_id = *self.funciones.get(&nombre_init).unwrap();
        let init_ref = self.module.declare_func_in_func(init_id, builder.func);
        let init_call = builder.ins().call(init_ref, &block_params);
        let fut_ptr = builder.inst_results(init_call)[0];

        // Poll loop: while __poll(ptr) == 0 { Sleep(1); }
        let poll_id = *self.funciones.get(&nombre_poll).unwrap();
        let sleep_id = self.asegurar_funcion_c("Sleep", &[types::I32], None);

        let bloque_loop = builder.create_block();
        let bloque_check = builder.create_block();
        let bloque_done = builder.create_block();

        builder.ins().jump(bloque_check, &[]);

        // Check: result = __poll(ptr); if result == 0 goto loop else done
        builder.switch_to_block(bloque_check);
        let poll_ref = self.module.declare_func_in_func(poll_id, builder.func);
        let poll_call = builder.ins().call(poll_ref, &[fut_ptr]);
        let poll_result = builder.inst_results(poll_call)[0];
        let cero64 = builder.ins().iconst(types::I64, 0);
        let es_pending = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, poll_result, cero64);
        builder.ins().brif(es_pending, bloque_loop, &[], bloque_done, &[]);
        // NO sellar bloque_check aquí — tiene 2 predecesores (entry + loop back-edge)

        // Loop body: Sleep(1) + jump back to check
        builder.switch_to_block(bloque_loop);
        let sleep_ref = self.module.declare_func_in_func(sleep_id, builder.func);
        let uno32 = builder.ins().iconst(types::I32, 1);
        builder.ins().call(sleep_ref, &[uno32]);
        builder.ins().jump(bloque_check, &[]);
        builder.seal_block(bloque_loop); // 1 predecesor: check

        // Ahora sí sellar check (2 predecesores: entry + loop)
        builder.seal_block(bloque_check);

        // Done: free(ptr) + return
        builder.switch_to_block(bloque_done);
        builder.seal_block(bloque_done);
        self.llamar_free(&mut builder, fut_ptr);

        if func.retorno.is_some() {
            // TODO: cargar resultado del struct antes de free
            let dummy = builder.ins().iconst(types::I32, 0);
            builder.ins().return_(&[dummy]);
        } else {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let func_id = *self.funciones.get(&func.nombre).unwrap();
        self.module.define_function(func_id, &mut ctx).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo wrapper sync: {}", e),
            ));
        })?;

        Ok(())
    }

    /// Extrae el valor de ms de una expresión de suspensión (dormir(ms))
    // ============================================================
    // Fase 15A: Métodos bitwise en enteros
    // ============================================================

    fn compilar_metodo(
        &mut self,
        receptor: &Expresion,
        nombre: &str,
        args: &[Expresion],
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        use crate::ast::Tipo;
        
        // Inferir tipo del receptor
        let tipo_receptor = self.inferir_tipo(receptor, variables);
        
        // Si es método de tipo (Texto, Vector), desugar a llamada built-in
        match &tipo_receptor {
            Tipo::Texto => {
                let builtin = match nombre {
                    "agregar" => Some("texto_agregar"),
                    "tam" => Some("texto_longitud"),
                    "liberar" => Some("texto_liberar"),
                    "obtener" => Some("texto_obtener_byte"),
                    "concatenar" => Some("texto_concatenar"),
                    "subtexto" => Some("texto_subtexto"),
                    "comparar" => Some("texto_comparar"),
                    _ => None,
                };
                if let Some(func) = builtin {
                    let mut argumentos = vec![receptor.clone()];
                    argumentos.extend(args.iter().cloned());
                    let llamada = Llamada {
                        funcion: func.to_string(),
                        tipo_args: vec![],
                        argumentos,
                        span: receptor.span().clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                // Fallback a bitwise
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
            Tipo::Vector(_) => {
                let builtin = match nombre {
                    "agregar" => Some("vector_agregar"),
                    "tam" => Some("vector_longitud"),
                    "obtener" => Some("vector_obtener"),
                    "liberar" => Some("vector_liberar"),
                    _ => None,
                };
                if let Some(func) = builtin {
                    let mut argumentos = vec![receptor.clone()];
                    argumentos.extend(args.iter().cloned());
                    let llamada = Llamada {
                        funcion: func.to_string(),
                        tipo_args: vec![],
                        argumentos,
                        span: receptor.span().clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                // Fallback a bitwise
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
            _ => {
                // Bitwise methods u otros
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
        }
    }

    fn compilar_metodo_bitwise(
        &mut self,
        receptor: &Expresion,
        nombre: &str,
        args: &[Expresion],
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(receptor, builder, variables)?;
        let tipo_x = builder.func.dfg.value_type(x);

        match nombre {
            // x.poner_bit(n) → x | (1 << n)
            "poner_bit" => {
                let n = self.compilar_expresion(&args[0], builder, variables)?;
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, n);
                Ok(builder.ins().bor(x, mask))
            }
            // x.quitar_bit(n) → x & ~(1 << n)
            "quitar_bit" => {
                let n = self.compilar_expresion(&args[0], builder, variables)?;
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, n);
                let not_mask = builder.ins().bnot(mask);
                Ok(builder.ins().band(x, not_mask))
            }
            // x.alternar_bit(n) → x ^ (1 << n)
            "alternar_bit" => {
                let n = self.compilar_expresion(&args[0], builder, variables)?;
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, n);
                Ok(builder.ins().bxor(x, mask))
            }
            // x.extraer_bits(offset, cantidad) → (x >> offset) & ((1 << cantidad) - 1)
            "extraer_bits" => {
                let offset = self.compilar_expresion(&args[0], builder, variables)?;
                let cantidad = self.compilar_expresion(&args[1], builder, variables)?;
                let shifted = builder.ins().ushr(x, offset);
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, cantidad);
                let menos_uno = builder.ins().iconst(tipo_x, -1);
                let mask_menos1 = builder.ins().iadd(mask, menos_uno);
                Ok(builder.ins().band(shifted, mask_menos1))
            }
            // x.ceros_izquierda() → clz
            "ceros_izquierda" => {
                Ok(builder.ins().clz(x))
            }
            // x.unos() → popcount
            "unos" => {
                Ok(builder.ins().popcnt(x))
            }
            _ => {
                // No debería llegar aquí (semantic lo filtra)
                Ok(x)
            }
        }
    }

    fn extraer_ms_de_suspension(
        &mut self,
        expr: &Expresion,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // La expresión es la llamada a dormir(ms)
        // Extraer solo el argumento ms (no compilar la llamada Sleep completa)
        if let Expresion::Llamada(llamada) = expr {
            if llamada.funcion == "dormir" && !llamada.argumentos.is_empty() {
                let ms_val = self.compilar_expresion(&llamada.argumentos[0], builder, variables)?;
                // Extender a i64 si es i32
                let ms_i64 = builder.ins().uextend(types::I64, ms_val);
                return Ok(ms_i64);
            }
        }
        // Fallback: compilar como expresión y extender
        let val = self.compilar_expresion(expr, builder, variables)?;
        let val_i64 = builder.ins().uextend(types::I64, val);
        Ok(val_i64)
    }

    /// Compila una función normal (no-futuro) — extraído de compilar_funcion para reuso
    fn compilar_funcion_normal(&mut self, func: &FuncionDecl) -> Result<(), ()> {
        // Misma lógica que compilar_funcion original
        if func.es_insegura && func.cuerpo.sentencias.is_empty() {
            return Ok(());
        }

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut sig = Signature::new(self.call_conv_default());
        if let Some(ref ret) = func.retorno {
            sig.returns.push(AbiParam::new(self.tipo_a_cranelift(ret)));
        }
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry_block = builder.create_block();

        for param in &func.parametros {
            let tipo = self.tipo_a_cranelift(&param.tipo);
            builder.append_block_param(entry_block, tipo);
        }

        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

        for (i, param) in func.parametros.iter().enumerate() {
            let val = builder.block_params(entry_block)[i];
            let tamano = self.tamano_tipo(&param.tipo);
            let slot = builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    tamano,
                    0,
                )
            );
            builder.ins().stack_store(val, slot, 0);
            variables.insert(param.nombre.clone(), (slot, param.tipo.clone(), param.articulo));
        }

        for sentencia in &func.cuerpo.sentencias {
            self.compilar_sentencia(sentencia, &mut builder, &mut variables, &func.span)?;
        }

        if func.retorno.is_none() {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let func_id = match self.funciones.get(&func.nombre).copied() {
            Some(id) => id,
            None => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno, 10, func.span.clone(),
                    format!("Función '{}' no declarada previamente", func.nombre),
                ));
                return Err(());
            }
        };

        self.module.define_function(func_id, &mut ctx).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo función: {}", e),
            ));
        })?;

        Ok(())
    }

    fn compilar_sentencia(
        &mut self,
        sentencia: &Sentencia,
        builder: &mut FunctionBuilder,
        variables: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _func_span: &Span,  // Span de la función padre para contexto
    ) -> Result<(), ()> {
        match sentencia {
            Sentencia::Expresion(expr) => {
                let _ = self.compilar_expresion(expr, builder, variables)?;
            }
            Sentencia::DeclaracionVariable(decl) => {
                let tipo = decl.tipo.clone().unwrap_or_else(||
                    self.inferir_tipo(&decl.valor, variables)
                );
                
                // Arrays: stack slot grande
                let (slot, tamano) = match &tipo {
                    Tipo::Array(tipo_elem, longitud) => {
                        let tamano_elem = self.tamano_tipo(tipo_elem);
                        let tamano_total = tamano_elem * (*longitud as u32);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano_total,
                                0,
                            )
                        );
                        
                        // Inicializar array
                        let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                        let tamano_elem_i64 = tamano_elem as i64;
                        
                        match &decl.valor {
                            Expresion::ArrayRelleno(elem, _, _) => {
                                // Caso "todos expr": inicializar todos con el mismo valor
                                let val = self.compilar_expresion(elem, builder, variables)?;
                                for i in 0..*longitud {
                                    let offset = (i as i64 * tamano_elem_i64) as i32;
                                    let elem_ptr = builder.ins().iadd_imm(base_ptr, offset as i64);
                                    builder.ins().store(
                                        cranelift_codegen::ir::MemFlags::new(),
                                        val,
                                        elem_ptr,
                                        0
                                    );
                                }
                            }
                            Expresion::LiteralArray(elementos, _) => {
                                // Inicializar con valores explícitos
                                for (i, elem) in elementos.iter().enumerate() {
                                    let val = self.compilar_expresion(elem, builder, variables)?;
                                    let offset = (i as i64 * tamano_elem_i64) as i32;
                                    let elem_ptr = builder.ins().iadd_imm(base_ptr, offset as i64);
                                    builder.ins().store(
                                        cranelift_codegen::ir::MemFlags::new(),
                                        val,
                                        elem_ptr,
                                        0
                                    );
                                }
                            }
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Interno,
                                    21,
                                    decl.span.clone(),
                                    "Expresión no válida para inicialización de arreglo".to_string(),
                                ));
                            }
                        }
                        
                        (slot, tamano_total)
                    }
                    Tipo::Nombre(nombre_tipo) => {
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        
                        match &decl.valor {
                            Expresion::InicializacionStruct(_, campos, _) => {
                                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                                let layout = match self.structs.get(nombre_tipo) {
                                    Some(l) => l.clone(),
                                    None => {
                                        self.errores.agregar(ErrorCompilador::nuevo(
                                            CategoriaError::Interno,
                                            30,
                                            decl.span.clone(),
                                            format!("Struct '{}' no registrado", nombre_tipo),
                                        ));
                                        return Err(());
                                    }
                                };

                                // Fase 15B: bitfield struct
                                if layout.es_bitfield {
                                    let backing_type = match layout.tamano {
                                        1 => types::I8,
                                        2 => types::I16,
                                        4 => types::I32,
                                        _ => types::I64,
                                    };
                                    let cero = builder.ins().iconst(backing_type, 0);
                                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, base_ptr, 0);
                                    for (nombre_campo, valor_expr) in campos {
                                        if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                                            let val = self.compilar_expresion(valor_expr, builder, variables)?;
                                            let raw = builder.ins().load(backing_type, cranelift_codegen::ir::MemFlags::new(), base_ptr, 0);
                                            let cur_i32 = if backing_type != types::I32 {
                                                builder.ins().uextend(types::I32, raw)
                                            } else {
                                                raw
                                            };
                                            let uno = builder.ins().iconst(types::I32, 1);
                                            let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                                            let field_mask = builder.ins().ishl(uno, ancho_val);
                                            let menos_uno = builder.ins().iconst(types::I32, -1);
                                            let field_mask = builder.ins().iadd(field_mask, menos_uno);
                                            let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                                            let shifted_mask = builder.ins().ishl(field_mask, offset_val);
                                            let not_mask = builder.ins().bnot(shifted_mask);
                                            let cleared = builder.ins().band(cur_i32, not_mask);
                                            let valor_masked = builder.ins().band(val, field_mask);
                                            let valor_shifted = builder.ins().ishl(valor_masked, offset_val);
                                            let nuevo = builder.ins().bor(cleared, valor_shifted);
                                            let store_val = if backing_type != types::I32 {
                                                builder.ins().ireduce(backing_type, nuevo)
                                            } else {
                                                nuevo
                                            };
                                            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), store_val, base_ptr, 0);
                                        }
                                    }
                                } else {
                                    for (nombre_campo, valor) in campos {
                                        let val = self.compilar_expresion(valor, builder, variables)?;
                                        let offset = match layout.offsets.get(nombre_campo) {
                                            Some(o) => *o as i64,
                                            None => {
                                                self.errores.agregar(ErrorCompilador::nuevo(
                                                    CategoriaError::Interno,
                                                    31,
                                                    decl.span.clone(),
                                                    format!("Campo '{}' no encontrado en '{}'", nombre_campo, nombre_tipo),
                                                ));
                                                return Err(());
                                            }
                                        };
                                        let campo_ptr = builder.ins().iadd_imm(base_ptr, offset);
                                        builder.ins().store(
                                            cranelift_codegen::ir::MemFlags::new(),
                                            val,
                                            campo_ptr,
                                            0,
                                        );
                                    }
                                }
                            }
                            Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, _) => {
                                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                                let layout = match self.enums.get(enum_nombre) {
                                    Some(l) => l.clone(),
                                    None => {
                                        self.errores.agregar(ErrorCompilador::nuevo(
                                            CategoriaError::Interno,
                                            50,
                                            decl.span.clone(),
                                            format!("Enum '{}' no registrado", enum_nombre),
                                        ));
                                        return Err(());
                                    }
                                };
                                
                                // Almacenar tag
                                let tag = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                                let tag_val = builder.ins().iconst(types::I32, tag as i64);
                                builder.ins().store(
                                    cranelift_codegen::ir::MemFlags::new(),
                                    tag_val,
                                    base_ptr,
                                    0,
                                );
                                
                                // Almacenar datos si hay argumentos
                                if !argumentos.is_empty() {
                                    let datos_ptr = builder.ins().iadd_imm(base_ptr, layout.datos_offset as i64);
                                    let mut offset = 0i64;
                                    for arg in argumentos {
                                        let val = self.compilar_expresion(arg, builder, variables)?;
                                        let arg_ptr = builder.ins().iadd_imm(datos_ptr, offset);
                                        builder.ins().store(
                                            cranelift_codegen::ir::MemFlags::new(),
                                            val,
                                            arg_ptr,
                                            0,
                                        );
                                        offset += 4;
                                    }
                                }
                            }
                            _ => {
                                let valor = self.compilar_expresion(&decl.valor, builder, variables)?;
                                builder.ins().stack_store(valor, slot, 0);
                            }
                        }
                        
                        (slot, tamano)
                    }
                    Tipo::Resultado(_, _) => {
                        // Manejar Resultado<T, E> similar a enums normales
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        
                        if let Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, _) = &decl.valor {
                            let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                            let layout = match self.enums.get(enum_nombre) {
                                Some(l) => l.clone(),
                                None => {
                                    self.errores.agregar(ErrorCompilador::nuevo(
                                        CategoriaError::Interno,
                                        50,
                                        decl.span.clone(),
                                        format!("Enum '{}' no registrado", enum_nombre),
                                    ));
                                    return Err(());
                                }
                            };
                            
                            // Almacenar tag
                            let tag = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                            let tag_val = builder.ins().iconst(types::I32, tag as i64);
                            builder.ins().store(
                                cranelift_codegen::ir::MemFlags::new(),
                                tag_val,
                                base_ptr,
                                0,
                            );
                            
                            // Almacenar datos si hay argumentos
                            if !argumentos.is_empty() {
                                let datos_ptr = builder.ins().iadd_imm(base_ptr, layout.datos_offset as i64);
                                let mut offset = 0i64;
                                for arg in argumentos {
                                    let val = self.compilar_expresion(arg, builder, variables)?;
                                    let arg_ptr = builder.ins().iadd_imm(datos_ptr, offset);
                                    builder.ins().store(
                                        cranelift_codegen::ir::MemFlags::new(),
                                        val,
                                        arg_ptr,
                                        0,
                                    );
                                    offset += 4;
                                }
                            }
                        } else {
                            let valor = self.compilar_expresion(&decl.valor, builder, variables)?;
                            builder.ins().stack_store(valor, slot, 0);
                        }
                        
                        (slot, tamano)
                    }
                    _ => {
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        let valor = self.compilar_expresion(&decl.valor, builder, variables)?;
                        builder.ins().stack_store(valor, slot, 0);
                        (slot, tamano)
                    }
                };
                
                variables.insert(decl.nombre.clone(), (slot, tipo, decl.articulo));
            }
            Sentencia::Asignacion(asig) => {
                let valor = self.compilar_expresion(&asig.valor, builder, variables)?;
                
                match &asig.lugar {
                    crate::ast::Lugar::Identificador(nombre) => {
                        if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                            builder.ins().stack_store(valor, *slot, 0);
                        } else {
                            self.errores.agregar(ErrorCompilador::nuevo(
                                CategoriaError::Interno,
                                15,
                                asig.span.clone(),
                                format!("Variable '{}' no encontrada para asignación", nombre),
                            ));
                        }
                    }
                    crate::ast::Lugar::Array(array_expr, indice_expr) => {
                        let array_val = self.compilar_expresion(array_expr, builder, variables)?;
                        let idx_val = self.compilar_expresion(indice_expr, builder, variables)?;
                        
                        let tipo_array = self.inferir_tipo(array_expr, variables);
                        let tamano_elem = match tipo_array {
                            Tipo::Array(ref t, _) => self.tamano_tipo(t) as i64,
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Interno,
                                    20,
                                    asig.span.clone(),
                                    "Asignación a arreglo en tipo no-arreglo".to_string(),
                                ));
                                return Err(());
                            }
                        };
                        
                        let idx_i64 = if builder.func.dfg.value_type(idx_val) == types::I32 {
                            builder.ins().sextend(types::I64, idx_val)
                        } else {
                            idx_val
                        };
                        
                        let offset = builder.ins().imul_imm(idx_i64, tamano_elem);
                        let elem_ptr = builder.ins().iadd(array_val, offset);
                        builder.ins().store(
                            cranelift_codegen::ir::MemFlags::new(),
                            valor,
                            elem_ptr,
                            0
                        );
                    }
                    // Fase 15B: bitfield write — reg.campo = valor
                    crate::ast::Lugar::Campo(base_expr, nombre_campo) => {
                        let struct_ptr = self.compilar_expresion(base_expr, builder, variables)?;
                        let tipo_base = self.inferir_tipo(base_expr, variables);
                        let nombre_struct = match tipo_base {
                            Tipo::Nombre(n) => n,
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Interno, 32, asig.span.clone(),
                                    format!("Asignación a campo en tipo no-struct '{:?}'", tipo_base),
                                ));
                                return Err(());
                            }
                        };
                        let layout = self.structs.get(&nombre_struct).cloned().unwrap();
                        if layout.es_bitfield {
                            if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                                let backing_type = match layout.tamano {
                                    1 => types::I8,
                                    2 => types::I16,
                                    4 => types::I32,
                                    _ => types::I64,
                                };
                                // Cargar entero de respaldo
                                let raw_val = builder.ins().load(backing_type, cranelift_codegen::ir::MemFlags::new(), struct_ptr, 0);
                                let val_i32 = if backing_type != types::I32 {
                                    builder.ins().uextend(types::I32, raw_val)
                                } else {
                                    raw_val
                                };
                                // mask = ((1 << ancho) - 1) << offset
                                let uno = builder.ins().iconst(types::I32, 1);
                                let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                                let field_mask = builder.ins().ishl(uno, ancho_val);
                                let menos_uno = builder.ins().iconst(types::I32, -1);
                                let field_mask = builder.ins().iadd(field_mask, menos_uno);
                                let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                                let shifted_mask = builder.ins().ishl(field_mask, offset_val);
                                // Limpiar bits: reg & ~shifted_mask
                                let not_mask = builder.ins().bnot(shifted_mask);
                                let cleared = builder.ins().band(val_i32, not_mask);
                                // Insertar valor: (valor & field_mask) << offset
                                let valor_masked = builder.ins().band(valor, field_mask);
                                let valor_shifted = builder.ins().ishl(valor_masked, offset_val);
                                let nuevo_val = builder.ins().bor(cleared, valor_shifted);
                                // Truncar y almacenar
                                let store_val = if backing_type != types::I32 {
                                    builder.ins().ireduce(backing_type, nuevo_val)
                                } else {
                                    nuevo_val
                                };
                                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), store_val, struct_ptr, 0);
                            }
                        } else {
                            // Struct normal: store directo al offset del campo
                            if let Some(offset) = layout.offsets.get(nombre_campo) {
                                let campo_ptr = builder.ins().iadd_imm(struct_ptr, *offset as i64);
                                let tipo_campo = self.buscar_tipo_campo(&nombre_struct, nombre_campo);
                                let cranelift_type = self.tipo_a_cranelift(&tipo_campo);
                                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), valor, campo_ptr, 0);
                            }
                        }
                    }
                }
            }
            Sentencia::Retornar(expr, span) => {
                if let Some(expr) = expr {
                    let val = self.compilar_expresion(expr, builder, variables)?;
                    builder.ins().return_(&[val]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            Sentencia::Condicional(cond) => {
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();

                // Compilar condición
                let cond_val = self.compilar_expresion(&cond.condicion, builder, variables)?;
                
                // Branch condicional
                builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

                match cond.modo {
                    crate::ast::ModoVerbal::Subjuntivo => {
                        // SUBJUNTIVO: condición improbable → cold path
                        // Construir ELSE primero (hot path, en línea)
                        builder.switch_to_block(else_block);
                        builder.seal_block(else_block);
                        let mut else_terminado = false;
                        if let Some(ref bloque_sino) = cond.bloque_sino {
                            for sentencia in &bloque_sino.sentencias {
                                if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                    else_terminado = true;
                                }
                                self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                            }
                        }
                        if !else_terminado {
                            builder.ins().jump(merge_block, &[]);
                        }

                        // Construir THEN después (cold path, fuera de línea)
                        builder.switch_to_block(then_block);
                        builder.seal_block(then_block);
                        let mut then_terminado = false;
                        for sentencia in &cond.bloque_entonces.sentencias {
                            if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                then_terminado = true;
                            }
                            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                        }
                        if !then_terminado {
                            builder.ins().jump(merge_block, &[]);
                        }
                    }
                    _ => {
                        // INDICATIVO / ESTATIVO: flujo normal
                        // Construir THEN primero (hot path)
                        builder.switch_to_block(then_block);
                        builder.seal_block(then_block);
                        let mut then_terminado = false;
                        for sentencia in &cond.bloque_entonces.sentencias {
                            if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                then_terminado = true;
                            }
                            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                        }
                        if !then_terminado {
                            builder.ins().jump(merge_block, &[]);
                        }

                        // Construir ELSE después
                        builder.switch_to_block(else_block);
                        builder.seal_block(else_block);
                        let mut else_terminado = false;
                        if let Some(ref bloque_sino) = cond.bloque_sino {
                            for sentencia in &bloque_sino.sentencias {
                                if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                    else_terminado = true;
                                }
                                self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                            }
                        }
                        if !else_terminado {
                            builder.ins().jump(merge_block, &[]);
                        }
                    }
                }

                // Bloque de unión
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
            }
            Sentencia::BucleMientras(bucle) => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                // Saltar al header inicialmente
                builder.ins().jump(header_block, &[]);

                // Header: evaluar condición
                builder.switch_to_block(header_block);
                let cond_val = self.compilar_expresion(&bucle.condicion, builder, variables)?;
                builder.ins().brif(cond_val, body_block, &[], exit_block, &[]);
                // NO sellar header todavía — el body puede saltar de vuelta

                // Body: ejecutar sentencias y volver al header
                builder.switch_to_block(body_block);
                let mut body_terminado = false;
                for sentencia in &bucle.bloque.sentencias {
                    if matches!(sentencia, Sentencia::Retornar(_, _)) {
                        body_terminado = true;
                    }
                    self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                }
                if !body_terminado {
                    builder.ins().jump(header_block, &[]);
                }
                builder.seal_block(body_block);

                // Ahora que todos los saltos al header están declarados, sellarlo
                builder.seal_block(header_block);

                // Exit: continuar después del bucle
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
            }
            Sentencia::BuclePara(bucle) => {
                // Detectar si el iterable es un rango
                if let Expresion::Rango(inicio_expr, fin_expr, inclusivo, _) = &bucle.iterable {
                    // === PARA SOBRE RANGO: para i en 0..10 { ... } ===
                    let header_block = builder.create_block();
                    let body_block = builder.create_block();
                    let exit_block = builder.create_block();

                    // Slot para la variable de iteración
                    let var_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            4, // I32
                            0,
                        )
                    );

                    // Compilar inicio y guardar en variable
                    let inicio_val = self.compilar_expresion(inicio_expr, builder, variables)?;
                    builder.ins().stack_store(inicio_val, var_slot, 0);

                    // Compilar fin y guardar en slot temporal
                    let fin_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            4,
                            0,
                        )
                    );
                    let fin_val = self.compilar_expresion(fin_expr, builder, variables)?;
                    builder.ins().stack_store(fin_val, fin_slot, 0);

                    // Registrar variable de iteración
                    let tipo_elem = self.inferir_tipo(inicio_expr, variables);
                    variables.insert(bucle.variable.clone(), (var_slot, tipo_elem, crate::ast::Articulo::La));

                    // Saltar al header
                    builder.ins().jump(header_block, &[]);

                    // Header: evaluar i < fin (o i <= fin si inclusivo)
                    builder.switch_to_block(header_block);
                    let cur_val = builder.ins().stack_load(types::I32, var_slot, 0);
                    let fin_loaded = builder.ins().stack_load(types::I32, fin_slot, 0);
                    let cc = if *inclusivo {
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual
                    } else {
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThan
                    };
                    let cond = builder.ins().icmp(cc, cur_val, fin_loaded);
                    builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                    // Body: ejecutar bloque
                    builder.switch_to_block(body_block);
                    let mut body_terminado = false;
                    for sentencia in &bucle.bloque.sentencias {
                        if matches!(sentencia, Sentencia::Retornar(_, _)) {
                            body_terminado = true;
                        }
                        self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                    }

                    if !body_terminado {
                        // i = i + 1
                        let cur = builder.ins().stack_load(types::I32, var_slot, 0);
                        let uno = builder.ins().iconst(types::I32, 1);
                        let nuevo = builder.ins().iadd(cur, uno);
                        builder.ins().stack_store(nuevo, var_slot, 0);
                        builder.ins().jump(header_block, &[]);
                    }
                    builder.seal_block(body_block);
                    builder.seal_block(header_block);

                    // Exit
                    builder.switch_to_block(exit_block);
                    builder.seal_block(exit_block);

                    // Limpiar variable de iteración
                    variables.remove(&bucle.variable);
                } else {
                    // === PARA SOBRE ARRAY (existente) ===
                    let header_block = builder.create_block();
                    let body_block = builder.create_block();
                    let exit_block = builder.create_block();

                    // Crear slot para índice (i = 0)
                    let idx_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            4, // I32
                            0,
                        )
                    );
                    let cero = builder.ins().iconst(types::I32, 0);
                    builder.ins().stack_store(cero, idx_slot, 0);

                    // Compilar iterable (obtener puntero al array)
                    let array_ptr = self.compilar_expresion(&bucle.iterable, builder, variables)?;

                    // Obtener tipo y longitud del array
                    let tipo_iterable = self.inferir_tipo(&bucle.iterable, variables);
                    let (tipo_elem, longitud, tamano_elem) = match tipo_iterable {
                        Tipo::Array(ref t, n) => {
                            let tam = self.tamano_tipo(t);
                            ((*t).clone(), n as i64, tam as i64)
                        }
                        _ => {
                            self.errores.agregar(ErrorCompilador::nuevo(
                                CategoriaError::Interno,
                                40,
                                bucle.span.clone(),
                                "'para' requiere arreglo o rango en codegen".to_string(),
                            ));
                            return Err(());
                        }
                    };

                    // Crear slot para variable de iteración
                    let elem_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            tamano_elem as u32,
                            0,
                        )
                    );

                    // Añadir variables al entorno
                    variables.insert(bucle.variable.clone(), (elem_slot, (*tipo_elem).clone(), crate::ast::Articulo::La));
                    let idx_name = format!("__idx_{}", bucle.variable);
                    variables.insert(idx_name.clone(), (idx_slot, Tipo::Entero32, crate::ast::Articulo::La));

                    // Saltar al header
                    builder.ins().jump(header_block, &[]);

                    // Header: evaluar i < longitud
                    builder.switch_to_block(header_block);
                    let idx_val = builder.ins().stack_load(types::I32, idx_slot, 0);
                    let len_val = builder.ins().iconst(types::I32, longitud);
                    let cond = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
                        idx_val,
                        len_val,
                    );
                    builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                    // Body: cargar elemento, ejecutar bloque, i++, volver
                    builder.switch_to_block(body_block);

                    // Calcular offset = i * tamano_elem
                    let idx_i64 = builder.ins().sextend(types::I64, idx_val);
                    let offset = builder.ins().imul_imm(idx_i64, tamano_elem);
                    let elem_ptr = builder.ins().iadd(array_ptr, offset);

                    // Cargar elemento y guardar en variable de iteración
                    let cranelift_type = self.tipo_a_cranelift(&tipo_elem);
                    let elem_val = builder.ins().load(
                        cranelift_type,
                        cranelift_codegen::ir::MemFlags::new(),
                        elem_ptr,
                        0,
                    );
                    builder.ins().stack_store(elem_val, elem_slot, 0);

                    // Ejecutar cuerpo
                    let mut body_terminado = false;
                    for sentencia in &bucle.bloque.sentencias {
                        if matches!(sentencia, Sentencia::Retornar(_, _)) {
                            body_terminado = true;
                        }
                        self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                    }

                    if !body_terminado {
                        // i = i + 1
                        let idx_val = builder.ins().stack_load(types::I32, idx_slot, 0);
                        let uno = builder.ins().iconst(types::I32, 1);
                        let nuevo_idx = builder.ins().iadd(idx_val, uno);
                        builder.ins().stack_store(nuevo_idx, idx_slot, 0);

                        builder.ins().jump(header_block, &[]);
                    }
                    builder.seal_block(body_block);
                    builder.seal_block(header_block);

                    // Exit: continuar
                    builder.switch_to_block(exit_block);
                    builder.seal_block(exit_block);

                    // Limpiar variables del bucle
                    variables.remove(&bucle.variable);
                    variables.remove(&idx_name);
                }
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                // Región: nuevo scope léxico (arena allocation)
                // Guardar variables actuales para restaurar después
                let variables_antes: Vec<String> = variables.keys().cloned().collect();
                
                // Compilar cuerpo de la región
                for sentencia in cuerpo {
                    self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                }
                
                // Limpiar variables declaradas en la región (LIFO)
                // En el futuro: insertar free() para heap allocations
                let variables_despues: Vec<String> = variables.keys().cloned().collect();
                for var in &variables_despues {
                    if !variables_antes.contains(var) {
                        variables.remove(var);
                    }
                }
            }
            Sentencia::Seleccionar(seleccionar) => {
                // Desugar a cadena if/else con canal_intentar
                // seleccionar { c como v => { A }, _ => { D } }
                // → let __sel = canal_intentar(c); si __sel != MIN { v = __sel; A } sino { D }
                let bloque_fin = builder.create_block();
                self.compilar_seleccionar_cadena(
                    &seleccionar.ramas,
                    0,
                    builder,
                    variables,
                    bloque_fin,
                    _func_span,
                )?;
                builder.switch_to_block(bloque_fin);
                builder.seal_block(bloque_fin);
            }
            Sentencia::ConExecutor { hilos, cuerpo, span: _ } => {
                // con_executor(N) { body }
                // 1. Crear pool en heap
                // 2. Spawn N workers (__executor_worker)
                // 3. Compilar body (lanzar encola al pool)
                // 4. Esperar completitud + shutdown
                self.compilar_con_executor(hilos, cuerpo, builder, variables, _func_span)?;
            }
        }
        Ok(())
    }

    /// Compila la cadena if/else de un `seleccionar` recursivamente
    fn compilar_seleccionar_cadena(
        &mut self,
        ramas: &[RamaSeleccionar],
        indice: usize,
        builder: &mut FunctionBuilder,
        variables: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        bloque_fin: cranelift_codegen::ir::Block,
        _func_span: &Span,
    ) -> Result<(), ()> {
        if indice >= ramas.len() {
            // Sin más ramas: jump al fin (no-op si no hay default)
            builder.ins().jump(bloque_fin, &[]);
            return Ok(());
        }

        let rama = &ramas[indice];

        // Rama default `_`: ejecutar cuerpo directamente
        if rama.variable.is_none() {
            for sentencia in &rama.cuerpo.sentencias {
                self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
            }
            builder.ins().jump(bloque_fin, &[]);
            return Ok(());
        }

        // Rama con canal: canal_intentar(canal) != i32::MIN → ejecutar
        let canal_ptr = self.compilar_expresion(&rama.canal, builder, variables)?;

        // Llamar canal_intentar inline (WaitForSingleObject con timeout=0)
        let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 8);
        let wfs_id = self.asegurar_funcion_c("WaitForSingleObject", &[types::I64, types::I32], Some(types::I32));
        let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);
        let zero_timeout = builder.ins().iconst(types::I32, 0);
        let call_wfs = builder.ins().call(wfs_ref, &[sem_handle, zero_timeout]);
        let wait_result = builder.inst_results(call_wfs)[0];

        let bloque_hay_dato = builder.create_block();
        let bloque_siguiente = builder.create_block();

        let wait_object_0 = builder.ins().iconst(types::I32, 0);
        let es_dato = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, wait_result, wait_object_0);
        builder.ins().brif(es_dato, bloque_hay_dato, &[], bloque_siguiente, &[]);

        // Bloque hay dato: lock, read, unlock, bind variable, ejecutar cuerpo
        builder.switch_to_block(bloque_hay_dato);
        builder.seal_block(bloque_hay_dato);

        let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 0);
        let wfs_ref2 = self.module.declare_func_in_func(wfs_id, builder.func);
        let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
        builder.ins().call(wfs_ref2, &[mutex_handle, infinite]);

        // Leer del ring buffer
        let head = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 16);
        let capacity = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 28);
        let head_i64 = builder.ins().sextend(types::I64, head);
        let four = builder.ins().iconst(types::I64, 4);
        let offset_buf = builder.ins().imul(head_i64, four);
        let base_offset = builder.ins().iconst(types::I64, 32);
        let read_offset = builder.ins().iadd(base_offset, offset_buf);
        let read_addr = builder.ins().iadd(canal_ptr, read_offset);
        let valor = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), read_addr, 0);

        // head = (head + 1) % capacity
        let one = builder.ins().iconst(types::I32, 1);
        let head_plus1 = builder.ins().iadd(head, one);
        let new_head = builder.ins().urem(head_plus1, capacity);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), new_head, canal_ptr, 16);

        // count--
        let count = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 24);
        let neg_one = builder.ins().iconst(types::I32, -1i64);
        let count_minus1 = builder.ins().iadd(count, neg_one);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), count_minus1, canal_ptr, 24);

        // ReleaseMutex
        let rel_mutex_id = self.asegurar_funcion_c("ReleaseMutex", &[types::I64], Some(types::I32));
        let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
        builder.ins().call(rel_mutex_ref, &[mutex_handle]);

        // Bind variable y ejecutar cuerpo
        if let Some(ref var_nombre) = rama.variable {
            let slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                4,
                0,
            ));
            builder.ins().stack_store(valor, slot, 0);
            variables.insert(var_nombre.clone(), (slot, Tipo::Entero32, crate::ast::Articulo::La));
        }

        for sentencia in &rama.cuerpo.sentencias {
            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
        }

        // Limpiar variable de la rama
        if let Some(ref var_nombre) = rama.variable {
            variables.remove(var_nombre);
        }

        builder.ins().jump(bloque_fin, &[]);

        // Bloque siguiente: intentar próxima rama
        builder.switch_to_block(bloque_siguiente);
        builder.seal_block(bloque_siguiente);
        self.compilar_seleccionar_cadena(ramas, indice + 1, builder, variables, bloque_fin, _func_span)?;

        Ok(())
    }

    /// con_executor(N) { body } — thread pool con work queue
    /// Pool layout (heap):
    ///   0: HANDLE mutex | 8: HANDLE semaphore | 16: HANDLE done_event
    ///   24: i64* worker_handles | 32: i32 head | 36: i32 tail
    ///   40: i32 count | 44: i32 capacity | 48: i32 shutdown
    ///   52: i32 active_tasks | 56: i32 num_workers | 60: pad
    ///   64: Task queue[capacity] (each task = {fn_ptr: i64, args_ptr: i64} = 16 bytes)
    fn compilar_con_executor(
        &mut self,
        hilos_expr: &Expresion,
        cuerpo: &[Sentencia],
        builder: &mut FunctionBuilder,
        variables: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _func_span: &Span,
    ) -> Result<(), ()> {
        let capacidad: i64 = 256;
        let pool_size: i64 = 64 + capacidad * 16;

        // 1. Evaluar N (número de workers)
        let num_workers = self.compilar_expresion(hilos_expr, builder, variables)?;
        let num_workers_i64 = builder.ins().sextend(types::I64, num_workers);

        // 2. malloc(pool_size)
        let malloc_id = self.asegurar_funcion_c("malloc", &[types::I64], Some(types::I64));
        let malloc_ref = self.module.declare_func_in_func(malloc_id, builder.func);
        let pool_size_val = builder.ins().iconst(types::I64, pool_size);
        let call_malloc = builder.ins().call(malloc_ref, &[pool_size_val]);
        let pool_ptr = builder.inst_results(call_malloc)[0];

        // 3. CreateMutexW(NULL, FALSE, NULL)
        let create_mutex_id = self.asegurar_funcion_c("CreateMutexW", &[types::I64, types::I32, types::I64], Some(types::I64));
        let cm_ref = self.module.declare_func_in_func(create_mutex_id, builder.func);
        let null_ptr = builder.ins().iconst(types::I64, 0);
        let false_val = builder.ins().iconst(types::I32, 0);
        let call_cm = builder.ins().call(cm_ref, &[null_ptr, false_val, null_ptr]);
        let mutex_handle = builder.inst_results(call_cm)[0];
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), mutex_handle, pool_ptr, 0);

        // 4. CreateSemaphoreW(NULL, 0, capacity, NULL)
        let create_sem_id = self.asegurar_funcion_c("CreateSemaphoreW", &[types::I64, types::I32, types::I32, types::I64], Some(types::I64));
        let cs_ref = self.module.declare_func_in_func(create_sem_id, builder.func);
        let cap_val = builder.ins().iconst(types::I32, capacidad);
        let zero_val = builder.ins().iconst(types::I32, 0);
        let call_cs = builder.ins().call(cs_ref, &[null_ptr, zero_val, cap_val, null_ptr]);
        let sem_handle = builder.inst_results(call_cs)[0];
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), sem_handle, pool_ptr, 8);

        // 5. CreateEventW(NULL, TRUE, FALSE, NULL) — manual reset, non-signaled
        let create_event_id = self.asegurar_funcion_c("CreateEventW", &[types::I64, types::I32, types::I32, types::I64], Some(types::I64));
        let ce_ref = self.module.declare_func_in_func(create_event_id, builder.func);
        let true_val = builder.ins().iconst(types::I32, 1);
        let call_ce = builder.ins().call(ce_ref, &[null_ptr, true_val, zero_val, null_ptr]);
        let done_event = builder.inst_results(call_ce)[0];
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), done_event, pool_ptr, 16);

        // 6. Init campos: head=0, tail=0, count=0, capacity, shutdown=0, active=0, num_workers
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_val, pool_ptr, 32); // head
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_val, pool_ptr, 36); // tail
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_val, pool_ptr, 40); // count
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cap_val, pool_ptr, 44); // capacity
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_val, pool_ptr, 48); // shutdown
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_val, pool_ptr, 52); // active_tasks
        let nw_i32 = builder.ins().iconst(types::I32, 0); // placeholder, usaremos num_workers real
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), num_workers, pool_ptr, 56); // num_workers

        // 7. malloc(N * 8) para worker handles
        let eight_i64 = builder.ins().iconst(types::I64, 8);
        let handles_size = builder.ins().imul(num_workers_i64, eight_i64);
        let malloc_ref2 = self.module.declare_func_in_func(malloc_id, builder.func);
        let call_malloc2 = builder.ins().call(malloc_ref2, &[handles_size]);
        let handles_ptr = builder.inst_results(call_malloc2)[0];
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), handles_ptr, pool_ptr, 24);

        // 8. Asegurar __executor_worker y spawn N threads
        self.asegurar_executor_worker(builder);
        let worker_func_id = self.funciones.get("__executor_worker").copied().unwrap();

        let create_thread_id = self.asegurar_funcion_c(
            "CreateThread",
            &[types::I64, types::I64, types::I64, types::I64, types::I32, types::I64],
            Some(types::I64),
        );

        // Loop: for i in 0..N { CreateThread(worker, pool_ptr) }
        let bloque_loop = builder.create_block();
        let bloque_body = builder.create_block();
        let bloque_fin_loop = builder.create_block();

        // Slot para contador i
        let slot_i = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 4, 0,
        ));
        builder.ins().stack_store(zero_val, slot_i, 0);
        builder.ins().jump(bloque_loop, &[]);

        // Condición: i < N
        builder.switch_to_block(bloque_loop);
        let i_val = builder.ins().stack_load(types::I32, slot_i, 0);
        let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedLessThan, i_val, num_workers);
        builder.ins().brif(cmp, bloque_body, &[], bloque_fin_loop, &[]);

        // Body: CreateThread + store handle
        builder.switch_to_block(bloque_body);
        builder.seal_block(bloque_body);
        let i_val2 = builder.ins().stack_load(types::I32, slot_i, 0);

        let worker_func_ref = self.module.declare_func_in_func(worker_func_id, builder.func);
        let worker_fn_ptr = builder.ins().func_addr(types::I64, worker_func_ref);

        let ct_ref = self.module.declare_func_in_func(create_thread_id, builder.func);
        let stack_size = builder.ins().iconst(types::I64, 0);
        let call_ct = builder.ins().call(ct_ref, &[
            null_ptr,           // lpThreadAttributes
            stack_size,         // dwStackSize
            worker_fn_ptr,      // lpStartAddress
            pool_ptr,           // lpParameter
            zero_val,           // dwCreationFlags
            null_ptr,           // lpThreadId
        ]);
        let thread_handle = builder.inst_results(call_ct)[0];

        // handles[i] = thread_handle
        let i_i64 = builder.ins().sextend(types::I64, i_val2);
        let eight_ct = builder.ins().iconst(types::I64, 8);
        let offset = builder.ins().imul(i_i64, eight_ct);
        let handle_addr = builder.ins().iadd(handles_ptr, offset);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), thread_handle, handle_addr, 0);

        // i++
        let one = builder.ins().iconst(types::I32, 1);
        let i_next = builder.ins().iadd(i_val2, one);
        builder.ins().stack_store(i_next, slot_i, 0);
        builder.ins().jump(bloque_loop, &[]);

        // Sellar loop header AFTER del back-edge
        builder.seal_block(bloque_loop);
        builder.switch_to_block(bloque_fin_loop);
        builder.seal_block(bloque_fin_loop);

        // 9. Guardar pool_ptr en variable __executor_pool y compilar body
        let slot_pool = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 8, 0,
        ));
        builder.ins().stack_store(pool_ptr, slot_pool, 0);
        let pool_var_name = format!("__executor_pool_{}", self.contador_closures);
        self.contador_closures += 1;
        variables.insert(pool_var_name.clone(), (slot_pool, Tipo::Entero64, crate::ast::Articulo::La));
        self.executor_pool_var = Some(pool_var_name.clone());

        // Compilar cuerpo
        for sentencia in cuerpo {
            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
        }

        self.executor_pool_var = None;
        variables.remove(&pool_var_name);

        // 10. Esperar completitud: spin-wait con Sleep(1) hasta count==0 && active==0
        let bloque_wait = builder.create_block();
        let bloque_check = builder.create_block();
        let bloque_done = builder.create_block();
        builder.ins().jump(bloque_wait, &[]);

        builder.switch_to_block(bloque_wait);
        // NO sellar aquí — back-edge desde bloque_check pendiente
        // Sleep(1)
        let sleep_id = self.asegurar_funcion_c("Sleep", &[types::I32], None);
        let sleep_ref = self.module.declare_func_in_func(sleep_id, builder.func);
        let one_ms = builder.ins().iconst(types::I32, 1);
        builder.ins().call(sleep_ref, &[one_ms]);
        builder.ins().jump(bloque_check, &[]);

        builder.switch_to_block(bloque_check);
        builder.seal_block(bloque_check);
        let count_val = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 40);
        let active_val = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 52);
        let cancelled_check = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 60);
        let count_zero = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, count_val, zero_val);
        let active_zero = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, active_val, zero_val);
        let both_zero = builder.ins().band(count_zero, active_zero);
        let is_cancelled = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, cancelled_check, zero_val);
        let should_done = builder.ins().bor(both_zero, is_cancelled);
        builder.ins().brif(should_done, bloque_done, &[], bloque_wait, &[]);

        // Sellar loop header AFTER del back-edge
        builder.seal_block(bloque_wait);
        builder.switch_to_block(bloque_done);
        builder.seal_block(bloque_done);

        // 11. Shutdown: shutdown=1, ReleaseSemaphore(N)
        let one_i32 = builder.ins().iconst(types::I32, 1);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), one_i32, pool_ptr, 48);
        let rel_sem_id = self.asegurar_funcion_c("ReleaseSemaphore", &[types::I64, types::I32, types::I64], Some(types::I32));
        let rs_ref = self.module.declare_func_in_func(rel_sem_id, builder.func);
        let sem_loaded = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 8);
        builder.ins().call(rs_ref, &[sem_loaded, num_workers, null_ptr]);

        // 12. Sleep(50) para dar tiempo a workers de salir
        let sleep_ref2 = self.module.declare_func_in_func(sleep_id, builder.func);
        let fifty_ms = builder.ins().iconst(types::I32, 50);
        builder.ins().call(sleep_ref2, &[fifty_ms]);

        // 13. CloseHandle: mutex, semaphore, done_event, worker handles
        let close_handle_id = self.asegurar_funcion_c("CloseHandle", &[types::I64], Some(types::I32));
        let ch_ref = self.module.declare_func_in_func(close_handle_id, builder.func);
        builder.ins().call(ch_ref, &[mutex_handle]);
        let ch_ref2 = self.module.declare_func_in_func(close_handle_id, builder.func);
        builder.ins().call(ch_ref2, &[sem_handle]);
        let ch_ref3 = self.module.declare_func_in_func(close_handle_id, builder.func);
        builder.ins().call(ch_ref3, &[done_event]);

        // CloseHandle para cada worker handle
        let bloque_ch_loop = builder.create_block();
        let bloque_ch_body = builder.create_block();
        let bloque_ch_fin = builder.create_block();
        let slot_j = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 4, 0,
        ));
        builder.ins().stack_store(zero_val, slot_j, 0);
        builder.ins().jump(bloque_ch_loop, &[]);

        builder.switch_to_block(bloque_ch_loop);
        let j_val = builder.ins().stack_load(types::I32, slot_j, 0);
        let cmp_j = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedLessThan, j_val, num_workers);
        builder.ins().brif(cmp_j, bloque_ch_body, &[], bloque_ch_fin, &[]);

        builder.switch_to_block(bloque_ch_body);
        builder.seal_block(bloque_ch_body);
        let j_val2 = builder.ins().stack_load(types::I32, slot_j, 0);
        let j_i64 = builder.ins().sextend(types::I64, j_val2);
        let eight_ch = builder.ins().iconst(types::I64, 8);
        let j_offset = builder.ins().imul(j_i64, eight_ch);
        let j_addr = builder.ins().iadd(handles_ptr, j_offset);
        let j_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), j_addr, 0);
        let ch_ref4 = self.module.declare_func_in_func(close_handle_id, builder.func);
        builder.ins().call(ch_ref4, &[j_handle]);
        let one_ch = builder.ins().iconst(types::I32, 1);
        let j_next = builder.ins().iadd(j_val2, one_ch);
        builder.ins().stack_store(j_next, slot_j, 0);
        builder.ins().jump(bloque_ch_loop, &[]);

        // Sellar loop header AFTER del back-edge
        builder.seal_block(bloque_ch_loop);
        builder.switch_to_block(bloque_ch_fin);
        builder.seal_block(bloque_ch_fin);

        // 14. free(handles_ptr), free(pool_ptr)
        let free_id = self.asegurar_funcion_c("free", &[types::I64], None);
        let free_ref = self.module.declare_func_in_func(free_id, builder.func);
        builder.ins().call(free_ref, &[handles_ptr]);
        let free_ref2 = self.module.declare_func_in_func(free_id, builder.func);
        builder.ins().call(free_ref2, &[pool_ptr]);

        Ok(())
    }

    /// Genera la función __executor_worker(pool_ptr: i64) -> i32
    /// Loop: WaitSem → Lock → check shutdown → dequeue → unlock → call → lock → active-- → unlock
    fn asegurar_executor_worker(&mut self, _caller_builder: &FunctionBuilder) {
        if self.executor_worker_generado {
            return;
        }
        self.executor_worker_generado = true;

        let mut sig = cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::WindowsFastcall);
        sig.params.push(cranelift_codegen::ir::AbiParam::new(types::I64)); // pool_ptr
        sig.returns.push(cranelift_codegen::ir::AbiParam::new(types::I32));

        let func_id = self.module.declare_function("__executor_worker", cranelift_module::Linkage::Local, &sig)
            .expect("declarar __executor_worker");
        self.funciones.insert("__executor_worker".to_string(), func_id);

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        {
            let mut fb = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fb);

            let bloque_entry = builder.create_block();
            builder.append_block_param(bloque_entry, types::I64);
            builder.switch_to_block(bloque_entry);
            builder.seal_block(bloque_entry);
            let pool_ptr = builder.block_params(bloque_entry)[0];

            let null_ptr = builder.ins().iconst(types::I64, 0);
            let zero_i32 = builder.ins().iconst(types::I32, 0);
            let one_i32 = builder.ins().iconst(types::I32, 1);
            let neg_one_i32 = builder.ins().iconst(types::I32, -1i64);
            let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);

            // Declarar funciones Win32
            let wfs_sig = {
                let mut s = cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::WindowsFastcall);
                s.params.push(cranelift_codegen::ir::AbiParam::new(types::I64));
                s.params.push(cranelift_codegen::ir::AbiParam::new(types::I32));
                s.returns.push(cranelift_codegen::ir::AbiParam::new(types::I32));
                s
            };
            let wfs_id = self.module.declare_function("WaitForSingleObject", cranelift_module::Linkage::Import, &wfs_sig).unwrap();
            let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);

            let rel_mutex_sig = {
                let mut s = cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::WindowsFastcall);
                s.params.push(cranelift_codegen::ir::AbiParam::new(types::I64));
                s.returns.push(cranelift_codegen::ir::AbiParam::new(types::I32));
                s
            };
            let rel_mutex_id = self.module.declare_function("ReleaseMutex", cranelift_module::Linkage::Import, &rel_mutex_sig).unwrap();
            let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);

            // Load handles del pool
            let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 0);
            let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 8);

            // === LOOP principal ===
            let bloque_loop = builder.create_block();
            let bloque_check_shutdown = builder.create_block();
            let bloque_dequeue = builder.create_block();
            let bloque_execute = builder.create_block();
            let bloque_post_exec = builder.create_block();
            let bloque_exit = builder.create_block();

            builder.ins().jump(bloque_loop, &[]);

            // WaitSem
            builder.switch_to_block(bloque_loop);
            // NO sellar aquí — back-edge desde bloque_post_exec pendiente
            let wfs_ref2 = self.module.declare_func_in_func(wfs_id, builder.func);
            builder.ins().call(wfs_ref2, &[sem_handle, infinite]);
            builder.ins().jump(bloque_check_shutdown, &[]);

            // Lock mutex + check shutdown/cancelled
            builder.switch_to_block(bloque_check_shutdown);
            builder.seal_block(bloque_check_shutdown);
            let wfs_ref3 = self.module.declare_func_in_func(wfs_id, builder.func);
            builder.ins().call(wfs_ref3, &[mutex_handle, infinite]);
            let shutdown_val = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 48);
            let cancelled_val = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 60);
            let should_exit = builder.ins().bor(shutdown_val, cancelled_val);
            let is_exit = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, should_exit, zero_i32);
            builder.ins().brif(is_exit, bloque_exit, &[], bloque_dequeue, &[]);

            // Dequeue: fn_ptr = queue[head*16 + 64], args_ptr = queue[head*16 + 64 + 8]
            builder.switch_to_block(bloque_dequeue);
            builder.seal_block(bloque_dequeue);
            let head = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 32);
            let capacity = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 44);
            let head_i64 = builder.ins().sextend(types::I64, head);
            let sixteen = builder.ins().iconst(types::I64, 16);
            let task_offset = builder.ins().imul(head_i64, sixteen);
            let base_64 = builder.ins().iconst(types::I64, 64);
            let task_addr_offset = builder.ins().iadd(base_64, task_offset);
            let task_addr = builder.ins().iadd(pool_ptr, task_addr_offset);
            let fn_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), task_addr, 0);
            let args_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), task_addr, 8);

            // head = (head + 1) % capacity
            let head_plus1 = builder.ins().iadd(head, one_i32);
            let new_head = builder.ins().urem(head_plus1, capacity);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), new_head, pool_ptr, 32);

            // count--
            let count = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 40);
            let count_minus1 = builder.ins().iadd(count, neg_one_i32);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), count_minus1, pool_ptr, 40);

            // active_tasks++
            let active = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 52);
            let active_plus1 = builder.ins().iadd(active, one_i32);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), active_plus1, pool_ptr, 52);

            // Unlock mutex
            let rel_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
            builder.ins().call(rel_ref, &[mutex_handle]);
            builder.ins().jump(bloque_execute, &[]);

            // Execute: call_indirect fn_ptr(args_ptr)
            builder.switch_to_block(bloque_execute);
            builder.seal_block(bloque_execute);
            // Firma del wrapper: fn(i64) -> i32
            let task_sig = {
                let mut s = cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::WindowsFastcall);
                s.params.push(cranelift_codegen::ir::AbiParam::new(types::I64));
                s.returns.push(cranelift_codegen::ir::AbiParam::new(types::I32));
                s
            };
            let task_sig_ref = builder.import_signature(task_sig);
            builder.ins().call_indirect(task_sig_ref, fn_ptr, &[args_ptr]);
            builder.ins().jump(bloque_post_exec, &[]);

            // Post-exec: lock, active--, unlock, loop
            builder.switch_to_block(bloque_post_exec);
            builder.seal_block(bloque_post_exec);
            let wfs_ref4 = self.module.declare_func_in_func(wfs_id, builder.func);
            builder.ins().call(wfs_ref4, &[mutex_handle, infinite]);
            let active2 = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 52);
            let active2_minus1 = builder.ins().iadd(active2, neg_one_i32);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), active2_minus1, pool_ptr, 52);
            let rel_ref2 = self.module.declare_func_in_func(rel_mutex_id, builder.func);
            builder.ins().call(rel_ref2, &[mutex_handle]);
            builder.ins().jump(bloque_loop, &[]);

            // Sellar loop header AFTER del back-edge
            builder.seal_block(bloque_loop);

            // Exit: unlock mutex, return 0
            builder.switch_to_block(bloque_exit);
            builder.seal_block(bloque_exit);
            let rel_ref3 = self.module.declare_func_in_func(rel_mutex_id, builder.func);
            builder.ins().call(rel_ref3, &[mutex_handle]);
            let ret_zero = builder.ins().iconst(types::I32, 0);
            builder.ins().return_(&[ret_zero]);

            builder.finalize();
        }

        let mut ctx2 = self.module.make_context();
        ctx2.func = ctx.func;
        let _ = self.module.define_function(func_id, &mut ctx2);
    }

    fn compilar_expresion(
        &mut self,
        expr: &Expresion,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        match expr {
            Expresion::LiteralArray(elementos, span) => {
                // Arrays literales: creamos stack slot grande y llenamos
                if elementos.is_empty() {
                    return Ok(builder.ins().iconst(types::I64, 0)); // null pointer
                }
                
                let tipo_elem = self.inferir_tipo(&elementos[0], variables);
                let tamano_elem = self.tamano_tipo(&tipo_elem) as i64;
                let longitud = elementos.len() as i64;
                let tamano_total = (tamano_elem * longitud) as u32;
                
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        tamano_total,
                        0,
                    )
                );
                
                // Obtener dirección base del array
                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                
                // Almacenar cada elemento
                for (i, elem) in elementos.iter().enumerate() {
                    let val = self.compilar_expresion(elem, builder, variables)?;
                    let offset = (i as i64 * tamano_elem) as i32;
                    let elem_ptr = builder.ins().iadd_imm(base_ptr, offset as i64);
                    builder.ins().store(
                        cranelift_codegen::ir::MemFlags::new(),
                        val,
                        elem_ptr,
                        0
                    );
                }
                
                Ok(base_ptr)
            }
            Expresion::AccesoArray(array, indice, span) => {
                let tipo_array = self.inferir_tipo(array, variables);
                
                // Texto[i] → builtin texto_obtener_byte
                // Texto[inicio..fin] → builtin texto_subtexto
                if tipo_array == Tipo::Texto {
                    // Verificar si el índice es un rango (slicing)
                    if let Expresion::Rango(inicio, fin, _inclusivo, _) = indice.as_ref() {
                        let llamada = Llamada {
                            funcion: "texto_subtexto".to_string(),
                            tipo_args: vec![],
                            argumentos: vec![
                                array.as_ref().clone(),
                                *inicio.clone(),
                                *fin.clone(),
                            ],
                            span: span.clone(),
                        };
                        return self.compilar_llamada(&llamada, builder, variables);
                    } else {
                        let llamada = Llamada {
                            funcion: "texto_obtener_byte".to_string(),
                            tipo_args: vec![],
                            argumentos: vec![
                                array.as_ref().clone(),
                                indice.as_ref().clone(),
                            ],
                            span: span.clone(),
                        };
                        return self.compilar_llamada(&llamada, builder, variables);
                    }
                }
                
                // Vector<T>[i] → builtin vector_obtener
                if let Tipo::Vector(_) = &tipo_array {
                    let llamada = Llamada {
                        funcion: "vector_obtener".to_string(),
                        tipo_args: vec![],
                        argumentos: vec![
                            array.as_ref().clone(),
                            indice.as_ref().clone(),
                        ],
                        span: span.clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                
                let array_val = self.compilar_expresion(array, builder, variables)?;
                let idx_val = self.compilar_expresion(indice, builder, variables)?;
                
                let (tipo_elem, tamano_elem) = match tipo_array {
                    Tipo::Array(ref t, _) => {
                        let tam = self.tamano_tipo(t);
                        (t.clone(), tam as i64)
                    }
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            20,
                            span.clone(),
                            "Acceso a arreglo en tipo no-arreglo".to_string(),
                        ));
                        return Err(());
                    }
                };
                
                // Asegurar que índice sea I64 para aritmética de punteros
                let idx_i64 = if builder.func.dfg.value_type(idx_val) == types::I32 {
                    builder.ins().sextend(types::I64, idx_val)
                } else {
                    idx_val
                };
                
                // Calcular offset = índice * tamaño_elemento
                let offset = builder.ins().imul_imm(idx_i64, tamano_elem);
                
                // Calcular dirección = array_ptr + offset
                let elem_ptr = builder.ins().iadd(array_val, offset);
                
                // Cargar elemento
                let cranelift_type = self.tipo_a_cranelift(&tipo_elem);
                let val = builder.ins().load(
                    cranelift_type,
                    cranelift_codegen::ir::MemFlags::new(),
                    elem_ptr,
                    0
                );
                Ok(val)
            }
            Expresion::Literal(lit) => {
                match lit {
                    Literal::Entero(n, span) => {
                        Ok(builder.ins().iconst(types::I32, *n as i64))
                    }
                    Literal::Palabra(s, span) => {
                        // Para strings, creamos un global data con ID único
                        self.contador_strings += 1;
                        let data_id = self.module.declare_data(
                            &format!("str_{}_{}", self.contador_strings, s.len()),
                            Linkage::Local,
                            false,
                            false,
                        ).map_err(|_| ())?;
                        
                        // Escribir datos incluyendo terminador nulo para compatibilidad C
                        let mut bytes = s.as_bytes().to_vec();
                        bytes.push(0);
                        let mut desc = cranelift_module::DataDescription::new();
                        desc.define(bytes.into_boxed_slice());
                        self.module.define_data(data_id, &desc)
                            .map_err(|_| ())?;
                        
                        // Crear puntero al string
                        let global = self.module.declare_data_in_func(data_id, builder.func);
                        let ptr = builder.ins().global_value(types::I64, global);
                        Ok(ptr)
                    }
                    Literal::Flotante(n, _span) => {
                        Ok(builder.ins().f64const(*n))
                    }
                    Literal::Booleano(v, _span) => {
                        let val = if *v { 1i64 } else { 0i64 };
                        Ok(builder.ins().iconst(types::I8, val))
                    }
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            5,
                            lit.span().clone(),
                            "Literal no soportado".to_string(),
                        ));
                        Err(())
                    }
                }
            }
            Expresion::Identificador(nombre, span) => {
                let (slot, tipo, _articulo) = match variables.get(nombre) {
                    Some(v) => v.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            6,
                            span.clone(),
                            format!("Variable '{}' no encontrada", nombre),
                        ));
                        return Err(());
                    }
                };
                
                // Si es array, struct o enum con datos, devolvemos puntero (dirección base)
                let val = if matches!(tipo, Tipo::Array(_, _) | Tipo::Nombre(_) | Tipo::Resultado(_, _)) {
                    builder.ins().stack_addr(types::I64, slot, 0)
                } else {
                    builder.ins().stack_load(
                        self.tipo_a_cranelift(&tipo),
                        slot,
                        0,
                    )
                };
                Ok(val)
            }
            Expresion::Binaria(izq, op, der, span) => {
                // Texto + Texto → concatenación via builtin
                if *op == OperadorBinario::Suma {
                    let tipo_izq = self.inferir_tipo(izq, variables);
                    if tipo_izq == Tipo::Texto {
                        let llamada = Llamada {
                            funcion: "texto_concatenar".to_string(),
                            tipo_args: vec![],
                            argumentos: vec![izq.as_ref().clone(), der.as_ref().clone()],
                            span: span.clone(),
                        };
                        return self.compilar_llamada(&llamada, builder, variables);
                    }
                }
                let val_izq = self.compilar_expresion(izq, builder, variables)?;
                let val_der = self.compilar_expresion(der, builder, variables)?;
                self.compilar_operacion_binaria(*op, val_izq, val_der, builder)
            }
            Expresion::Unaria(op, expr, span) => {
                // Manejar referencias de forma especial (necesitan puntero al stack slot)
                match op {
                    OperadorUnario::Referencia | OperadorUnario::ReferenciaMut => {
                        // &x o &mut x: obtener puntero al stack slot
                        if let Expresion::Identificador(nombre, _) = expr.as_ref() {
                            if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                                let ptr = builder.ins().stack_addr(types::I64, *slot, 0);
                                return Ok(ptr);
                            }
                        }
                        // &punto.x o &mut punto.x: obtener puntero al campo
                        if let Expresion::AccesoCampo(base, campo, _) = expr.as_ref() {
                            if let Expresion::Identificador(nombre, _) = base.as_ref() {
                                if let Some((slot, tipo, _articulo)) = variables.get(nombre) {
                                    // Obtener el offset del campo
                                    if let Tipo::Nombre(nombre_struct) = tipo {
                                        if let Some(layout) = self.structs.get(nombre_struct) {
                                            if let Some(offset) = layout.offsets.get(campo) {
                                                let base_ptr = builder.ins().stack_addr(types::I64, *slot, 0);
                                                let campo_ptr = builder.ins().iadd_imm(base_ptr, *offset as i64);
                                                return Ok(campo_ptr);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Si no es un identificador o acceso a campo, compilar la expresión y retornar su dirección
                        // (esto es una simplificación, no funciona para expresiones complejas)
                        let val = self.compilar_expresion(expr, builder, variables)?;
                        Ok(val)
                    }
                    OperadorUnario::Desreferencia => {
                        // *expr: cargar valor desde puntero
                        let ptr = self.compilar_expresion(expr, builder, variables)?;
                        // Asumimos I32 por ahora; en v2 inferir tipo desde el contexto
                        let val = builder.ins().load(
                            types::I32,
                            cranelift_codegen::ir::MemFlags::new(),
                            ptr,
                            0,
                        );
                        Ok(val)
                    }
                    _ => {
                        let val = self.compilar_expresion(expr, builder, variables)?;
                        self.compilar_operacion_unaria(*op, val, builder, span)
                    }
                }
            }
            Expresion::Llamada(llamada) => {
                self.compilar_llamada(llamada, builder, variables)
            }
            Expresion::InicializacionStruct(nombre, campos, span) => {
                let layout = match self.structs.get(nombre) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            30,
                            span.clone(),
                            format!("Struct '{}' no registrado en codegen", nombre),
                        ));
                        return Err(());
                    }
                };

                // Crear stack slot para el struct
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        layout.tamano,
                        0,
                    )
                );

                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                // Fase 15B: bitfield struct — inicializar entero de respaldo
                if layout.es_bitfield {
                    let backing_type = match layout.tamano {
                        1 => types::I8,
                        2 => types::I16,
                        4 => types::I32,
                        _ => types::I64,
                    };
                    // Inicializar a 0
                    let cero = builder.ins().iconst(backing_type, 0);
                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, base_ptr, 0);

                    // Escribir cada campo con shift+mask
                    for (nombre_campo, valor_expr) in campos {
                        if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                            let val = self.compilar_expresion(valor_expr, builder, variables)?;
                            // Cargar entero actual
                            let raw = builder.ins().load(backing_type, cranelift_codegen::ir::MemFlags::new(), base_ptr, 0);
                            let cur_i32 = if backing_type != types::I32 {
                                builder.ins().uextend(types::I32, raw)
                            } else {
                                raw
                            };
                            // mask = (1 << ancho) - 1
                            let uno = builder.ins().iconst(types::I32, 1);
                            let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                            let field_mask = builder.ins().ishl(uno, ancho_val);
                            let menos_uno = builder.ins().iconst(types::I32, -1);
                            let field_mask = builder.ins().iadd(field_mask, menos_uno);
                            let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                            let shifted_mask = builder.ins().ishl(field_mask, offset_val);
                            // Limpiar + insertar
                            let not_mask = builder.ins().bnot(shifted_mask);
                            let cleared = builder.ins().band(cur_i32, not_mask);
                            let valor_masked = builder.ins().band(val, field_mask);
                            let valor_shifted = builder.ins().ishl(valor_masked, offset_val);
                            let nuevo = builder.ins().bor(cleared, valor_shifted);
                            let store_val = if backing_type != types::I32 {
                                builder.ins().ireduce(backing_type, nuevo)
                            } else {
                                nuevo
                            };
                            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), store_val, base_ptr, 0);
                        }
                    }
                    return Ok(base_ptr);
                }

                // Struct normal: almacenar cada campo
                for (nombre_campo, valor) in campos {
                    let val = self.compilar_expresion(valor, builder, variables)?;
                    let offset = match layout.offsets.get(nombre_campo) {
                        Some(o) => *o as i64,
                        None => {
                            self.errores.agregar(ErrorCompilador::nuevo(
                                CategoriaError::Interno,
                                31,
                                span.clone(),
                                format!("Campo '{}' no encontrado en layout de '{}'", nombre_campo, nombre),
                            ));
                            return Err(());
                        }
                    };

                    let campo_ptr = builder.ins().iadd_imm(base_ptr, offset);
                    builder.ins().store(
                        cranelift_codegen::ir::MemFlags::new(),
                        val,
                        campo_ptr,
                        0,
                    );
                }

                Ok(base_ptr)
            }
            Expresion::AccesoCampo(expr, nombre_campo, span) => {
                let struct_ptr = self.compilar_expresion(expr, builder, variables)?;
                let tipo_expr = self.inferir_tipo(expr, variables);

                let nombre_struct = match tipo_expr {
                    Tipo::Nombre(n) => n,
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            32,
                            span.clone(),
                            format!("Acceso a campo en tipo no-struct '{:?}'", tipo_expr),
                        ));
                        return Err(());
                    }
                };

                let layout = match self.structs.get(&nombre_struct) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            30,
                            span.clone(),
                            format!("Struct '{}' no registrado en codegen", nombre_struct),
                        ));
                        return Err(());
                    }
                };

                // Fase 15B: bitfield read → (val >> offset) & mask
                if layout.es_bitfield {
                    if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                        // Cargar el entero de respaldo
                        let backing_type = match layout.tamano {
                            1 => types::I8,
                            2 => types::I16,
                            4 => types::I32,
                            _ => types::I64,
                        };
                        let raw_val = builder.ins().load(
                            backing_type,
                            cranelift_codegen::ir::MemFlags::new(),
                            struct_ptr,
                            0,
                        );
                        // Extender a I32 para operaciones
                        let val_i32 = if backing_type != types::I32 {
                            builder.ins().uextend(types::I32, raw_val)
                        } else {
                            raw_val
                        };
                        // (val >> offset) & ((1 << ancho) - 1)
                        let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                        let shifted = builder.ins().ushr(val_i32, offset_val);
                        let uno = builder.ins().iconst(types::I32, 1);
                        let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                        let mask = builder.ins().ishl(uno, ancho_val);
                        let menos_uno = builder.ins().iconst(types::I32, -1);
                        let mask_final = builder.ins().iadd(mask, menos_uno);
                        let resultado = builder.ins().band(shifted, mask_final);
                        return Ok(resultado);
                    }
                }

                let offset = match layout.offsets.get(nombre_campo) {
                    Some(o) => *o as i64,
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            31,
                            span.clone(),
                            format!("Campo '{}' no encontrado en layout de '{}'", nombre_campo, nombre_struct),
                        ));
                        return Err(());
                    }
                };

                let campo_ptr = builder.ins().iadd_imm(struct_ptr, offset);

                // Inferir tipo del campo para saber cómo cargar
                let tipo_campo = self.buscar_tipo_campo(&nombre_struct, nombre_campo);
                let cranelift_type = self.tipo_a_cranelift(&tipo_campo);
                let val = builder.ins().load(
                    cranelift_type,
                    cranelift_codegen::ir::MemFlags::new(),
                    campo_ptr,
                    0,
                );
                Ok(val)
            }
            Expresion::ArrayRelleno(_, _, span) => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    22,
                    span.clone(),
                    "'todos' solo puede usarse en inicialización de variable con tipo explícito".to_string(),
                ));
                Err(())
            }
            Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, span) => {
                let layout = match self.enums.get(enum_nombre) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            50,
                            span.clone(),
                            format!("Enum '{}' no registrado en codegen", enum_nombre),
                        ));
                        return Err(());
                    }
                };

                // Crear stack slot para el enum
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        layout.tamano,
                        0,
                    )
                );

                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                // Almacenar tag
                let tag = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                let tag_val = builder.ins().iconst(types::I32, tag as i64);
                builder.ins().store(
                    cranelift_codegen::ir::MemFlags::new(),
                    tag_val,
                    base_ptr,
                    0,
                );

                // Almacenar datos si hay argumentos
                if !argumentos.is_empty() {
                    let datos_ptr = builder.ins().iadd_imm(base_ptr, layout.datos_offset as i64);
                    let mut offset = 0i64;
                    for arg in argumentos {
                        let val = self.compilar_expresion(arg, builder, variables)?;
                        let arg_ptr = builder.ins().iadd_imm(datos_ptr, offset);
                        builder.ins().store(
                            cranelift_codegen::ir::MemFlags::new(),
                            val,
                            arg_ptr,
                            0,
                        );
                        // Asumimos I32 para simplificar; en v2 calcular tamaño real
                        offset += 4;
                    }
                }

                Ok(base_ptr)
            }
            Expresion::EsVariante(expr, enum_nombre, variante_nombre, _binding, span) => {
                let layout = match self.enums.get(enum_nombre) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            50,
                            span.clone(),
                            format!("Enum '{}' no registrado en codegen", enum_nombre),
                        ));
                        return Err(());
                    }
                };

                let enum_ptr = self.compilar_expresion(expr, builder, variables)?;
                
                // Cargar tag (I32 en offset 0)
                let tag_val = builder.ins().load(
                    types::I32,
                    cranelift_codegen::ir::MemFlags::new(),
                    enum_ptr,
                    0,
                );

                let tag_esperado = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                let esperado_val = builder.ins().iconst(types::I32, tag_esperado as i64);
                
                let resultado = builder.ins().icmp(
                    cranelift_codegen::ir::condcodes::IntCC::Equal,
                    tag_val,
                    esperado_val,
                );

                Ok(resultado)
            }
            Expresion::Ruta(path, span) => {
                // Ruta cualificada sin llamada (ej: pasar función como valor)
                // Por ahora: error, ya que no soportamos funciones como valores
                eprintln!("[Falcato] Error: ruta '{}' no es una expresión válida sin llamada",
                    path.join("::"));
                Err(())
            }
            Expresion::Propagacion(expr, span) => {
                // Operador ?: propaga errores
                // Si la expresión es Resultado.Error, retorna inmediatamente
                // Si es Resultado.Exito, extrae el valor
                
                // Por ahora: implementación simplificada
                // Extrae el valor del campo de datos (asumiendo Exito)
                // TODO: Implementar early return real con CFG restructuring
                
                let enum_ptr = self.compilar_expresion(expr, builder, variables)?;
                
                // Cargar el valor del campo de datos (offset 4, después del tag)
                let datos_ptr = builder.ins().iadd_imm(enum_ptr, 4);
                let valor = builder.ins().load(
                    types::I32,
                    cranelift_codegen::ir::MemFlags::new(),
                    datos_ptr,
                    0,
                );
                
                Ok(valor)
            }
            Expresion::Mover(nombre, _destino, span) => {
                // TODO: Implementar transferencia de ownership
                // Por ahora: compilar como identificador (sin verificación)
                let (slot, tipo, _articulo) = match variables.get(nombre) {
                    Some(v) => v.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            52,
                            span.clone(),
                            format!("Variable '{}' no encontrada en 'mover'", nombre),
                        ));
                        return Err(());
                    }
                };
                let val = if matches!(tipo, Tipo::Array(_, _) | Tipo::Nombre(_) | Tipo::Resultado(_, _)) {
                    builder.ins().stack_addr(types::I64, slot, 0)
                } else {
                    builder.ins().stack_load(self.tipo_a_cranelift(&tipo), slot, 0)
                };
                Ok(val)
            }
            Expresion::Copiar(expr, _span) => {
                // TODO: Implementar clonación profunda
                // Por ahora: compilar la expresión interna (copia superficial)
                self.compilar_expresion(expr, builder, variables)
            }
            Expresion::Rango(_, _, _, span) => {
                // Los rangos solo son válidos dentro de 'para'
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Tipo,
                    41,
                    span.clone(),
                    "Los rangos (..) solo pueden usarse dentro de un bucle 'para'".to_string(),
                ));
                Err(())
            }
            Expresion::Closure(params, cuerpo, span) => {
                // Generar nombre único para la función anónima
                self.contador_closures += 1;
                let nombre_closure = format!("__closure_{}", self.contador_closures);

                // Inferir tipos de parámetros (default Entero32 si no se especifica)
                let params_tipos: Vec<(String, Tipo)> = params.iter().map(|(n, t)| {
                    (n.clone(), t.clone().unwrap_or(Tipo::Entero32))
                }).collect();

                // Detectar capturas: variables usadas en el cuerpo que no son params
                let mut capturas: Vec<(String, Tipo)> = Vec::new();
                for (nombre_var, (_, tipo_var, _)) in variables.iter() {
                    if !params_tipos.iter().any(|(pn, _)| pn == nombre_var) {
                        if self.expresion_usa_variable(cuerpo, nombre_var) {
                            capturas.push((nombre_var.clone(), tipo_var.clone()));
                        }
                    }
                }

                // Firma: SIEMPRE env_ptr como primer parámetro (simplifica llamadas)
                let mut sig = Signature::new(self.call_conv_default());
                sig.params.push(AbiParam::new(types::I64)); // env_ptr (0 si no hay capturas)
                for (_, tipo) in &params_tipos {
                    sig.params.push(AbiParam::new(self.tipo_a_cranelift(tipo)));
                }
                let tipo_retorno = self.inferir_tipo(cuerpo, variables);
                sig.returns.push(AbiParam::new(self.tipo_a_cranelift(&tipo_retorno)));

                // Declarar la función closure en el módulo
                let func_id = self.module.declare_function(
                    &nombre_closure,
                    Linkage::Local,
                    &sig,
                ).map_err(|_| ())?;

                self.funciones.insert(nombre_closure.clone(), func_id);

                // Guardar para compilación diferida
                self.closures_pendientes.push(ClosurePendiente {
                    nombre: nombre_closure.clone(),
                    params: params_tipos,
                    cuerpo: *cuerpo.clone(),
                    capturas: capturas.clone(),
                    retorno: tipo_retorno,
                });

                // Obtener function pointer
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                let fn_ptr = builder.ins().func_addr(types::I64, func_ref);

                // Crear closure object: 16 bytes {fn_ptr: I64, env_ptr: I64}
                let closure_slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        16, // fn_ptr (8) + env_ptr (8)
                        0,
                    )
                );
                let closure_base = builder.ins().stack_addr(types::I64, closure_slot, 0);

                // Guardar fn_ptr en offset 0
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), fn_ptr, closure_base, 0);

                // Crear env struct si hay capturas
                if !capturas.is_empty() {
                    // Env struct: array de punteros a las variables capturadas (8 bytes cada uno)
                    let env_size = (capturas.len() * 8) as u32;
                    let env_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            env_size,
                            0,
                        )
                    );
                    let env_base = builder.ins().stack_addr(types::I64, env_slot, 0);

                    // Guardar punteros a cada variable capturada
                    for (i, (nombre_cap, _)) in capturas.iter().enumerate() {
                        if let Some((cap_slot, _, _)) = variables.get(nombre_cap) {
                            let cap_addr = builder.ins().stack_addr(types::I64, *cap_slot, 0);
                            let offset = (i * 8) as i32;
                            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cap_addr, env_base, offset);
                        }
                    }

                    // Guardar env_ptr en offset 8 del closure object
                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), env_base, closure_base, 8);
                } else {
                    // Sin capturas: env_ptr = 0
                    let cero = builder.ins().iconst(types::I64, 0);
                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, closure_base, 8);
                }

                // Retornar puntero al closure object
                Ok(closure_base)
            }
            Expresion::Coincidir(sujeto, brazos, _span) => {
                // Compilar sujeto
                let val_sujeto = self.compilar_expresion(sujeto, builder, variables)?;
                let tipo_sujeto = self.inferir_tipo(sujeto, variables);
                let cranelift_tipo = self.tipo_a_cranelift(&tipo_sujeto);

                // Slot para el resultado del match
                let resultado_slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        cranelift_tipo.bytes(),
                        0,
                    )
                );

                let bloque_fin = builder.create_block();
                builder.append_block_param(bloque_fin, cranelift_tipo);

                for brazo in brazos {
                    let bloque_brazo = builder.create_block();
                    let bloque_siguiente = builder.create_block();

                    match &brazo.patron {
                        crate::ast::PatronMatch::Comodin(_) => {
                            // Wildcard: siempre matchea, saltar directo al brazo
                            builder.ins().jump(bloque_brazo, &[]);
                        }
                        crate::ast::PatronMatch::Literal(lit) => {
                            // Comparar sujeto con literal
                            let val_lit = self.compilar_literal(lit, builder)?;
                            let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, val_sujeto, val_lit);
                            builder.ins().brif(cmp, bloque_brazo, &[], bloque_siguiente, &[]);
                        }
                        crate::ast::PatronMatch::VarianteEnum(enum_nombre, variante, binding, _span_pat) => {
                            // Para enums: comparar tag (primer campo I32)
                            // El sujeto es un puntero al struct del enum
                            let tag_offset = 0i32;
                            let tag_val = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), val_sujeto, tag_offset);

                            // Obtener índice de la variante
                            let tag_idx = self.indice_variante_enum(enum_nombre, variante).unwrap_or(0) as i64;
                            let tag_const = builder.ins().iconst(types::I32, tag_idx);
                            let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, tag_val, tag_const);

                            // Si hay binding, necesitamos pasar el dato al bloque del brazo
                            if let Some(_nombre_binding) = binding {
                                // Cargar dato del enum (offset 8, después del tag + padding)
                                let dato_val = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), val_sujeto, 8);
                                builder.ins().brif(cmp, bloque_brazo, &[dato_val], bloque_siguiente, &[]);
                            } else {
                                builder.ins().brif(cmp, bloque_brazo, &[], bloque_siguiente, &[]);
                            }
                        }
                    }

                    // Bloque del brazo: compilar cuerpo
                    builder.switch_to_block(bloque_brazo);
                    builder.seal_block(bloque_brazo);

                    // Si hay binding, declararlo como variable
                    if let crate::ast::PatronMatch::VarianteEnum(_, _, Some(nombre_binding), _) = &brazo.patron {
                        let dato_param = builder.block_params(bloque_brazo)[0];
                        let binding_slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                8,
                                0,
                            )
                        );
                        builder.ins().stack_store(dato_param, binding_slot, 0);
                        let mut vars_con_binding = variables.clone();
                        vars_con_binding.insert(nombre_binding.clone(), (binding_slot, Tipo::Entero64, crate::ast::Articulo::La));
                        let val_cuerpo = self.compilar_expresion(&brazo.cuerpo, builder, &vars_con_binding)?;
                        builder.ins().jump(bloque_fin, &[val_cuerpo]);
                    } else {
                        let val_cuerpo = self.compilar_expresion(&brazo.cuerpo, builder, variables)?;
                        builder.ins().jump(bloque_fin, &[val_cuerpo]);
                    }

                    // Bloque siguiente (para el próximo brazo)
                    builder.switch_to_block(bloque_siguiente);
                    builder.seal_block(bloque_siguiente);
                }

                // Después de todos los brazos, saltar al fin (caso no-exhaustivo: valor default 0)
                let default_val = builder.ins().iconst(cranelift_tipo, 0);
                builder.ins().jump(bloque_fin, &[default_val]);

                // Bloque fin: recibir el resultado
                builder.switch_to_block(bloque_fin);
                builder.seal_block(bloque_fin);
                let resultado = builder.block_params(bloque_fin)[0];
                Ok(resultado)
            }

            // Async (Fase 18A): esperar expr — MVP: compila la expresión interna
            // TODO: poll loop + waker cuando el runtime esté listo
            Expresion::Esperar(expr_interno, _span) => {
                self.compilar_expresion(expr_interno, builder, variables)
            }

            // Async (Fase 18A): lanzar expr — MVP: crea thread real del OS
            Expresion::Lanzar(expr_interno, _span) => {
                if let Expresion::Llamada(llamada) = expr_interno.as_ref() {
                    self.compilar_lanzar_hilo(llamada, builder, variables)
                } else {
                    // Fallback: compilar inline (secuencial)
                    self.compilar_expresion(expr_interno, builder, variables)
                }
            }

            // Async (Fase 18A): bloquear(expr) — MVP: compila la expresión interna
            // TODO: bridge sync→async con runtime
            Expresion::Bloquear(expr_interno, _span) => {
                self.compilar_expresion(expr_interno, builder, variables)
            }

            // Fase 15A: métodos bitwise en enteros
            Expresion::Metodo(receptor, nombre, args, _span) => {
                self.compilar_metodo(receptor, nombre, args, builder, variables)
            }
        }
    }

    /// Compila un literal a un valor Cranelift (para patrones de match)
    fn compilar_literal(&mut self, lit: &Literal, builder: &mut FunctionBuilder) -> Result<cranelift_codegen::ir::Value, ()> {
        match lit {
            Literal::Entero(n, _) => Ok(builder.ins().iconst(types::I32, *n as i64)),
            Literal::Booleano(b, _) => Ok(builder.ins().iconst(types::I8, if *b { 1 } else { 0 })),
            Literal::Caracter(c, _) => Ok(builder.ins().iconst(types::I32, *c as i64)),
            Literal::Flotante(f, _) => Ok(builder.ins().f64const(*f)),
            Literal::Palabra(_, _) => {
                // Strings en patrones no soportados por ahora
                Ok(builder.ins().iconst(types::I64, 0))
            }
        }
    }

    /// Obtiene el índice (tag) de una variante de enum
    fn indice_variante_enum(&self, enum_nombre: &str, variante: &str) -> Option<u32> {
        self.enums.get(enum_nombre).and_then(|layout| layout.variantes.get(variante).copied())
    }

    /// Verifica si una expresión usa una variable por nombre (para detectar capturas)
    fn expresion_usa_variable(&self, expr: &Expresion, nombre: &str) -> bool {
        match expr {
            Expresion::Identificador(n, _) => n == nombre,
            Expresion::Binaria(izq, _, der, _) => {
                self.expresion_usa_variable(izq, nombre) || self.expresion_usa_variable(der, nombre)
            }
            Expresion::Unaria(_, inner, _) => self.expresion_usa_variable(inner, nombre),
            Expresion::Llamada(llamada) => {
                llamada.argumentos.iter().any(|a| self.expresion_usa_variable(a, nombre))
            }
            Expresion::AccesoArray(base, idx, _) => {
                self.expresion_usa_variable(base, nombre) || self.expresion_usa_variable(idx, nombre)
            }
            Expresion::AccesoCampo(base, _, _) => self.expresion_usa_variable(base, nombre),
            Expresion::Rango(inicio, fin, _, _) => {
                self.expresion_usa_variable(inicio, nombre) || self.expresion_usa_variable(fin, nombre)
            }
            Expresion::Closure(params, cuerpo, _) => {
                // No contar si el closure shadowea la variable
                if params.iter().any(|(pn, _)| pn == nombre) {
                    false
                } else {
                    self.expresion_usa_variable(cuerpo, nombre)
                }
            }
            _ => false,
        }
    }

    fn buscar_tipo_campo(&self, nombre_struct: &str, nombre_campo: &str) -> Tipo {
        match self.structs.get(nombre_struct) {
            Some(layout) => {
                layout.tipos.get(nombre_campo).cloned().unwrap_or(Tipo::Entero32)
            }
            None => Tipo::Entero32,
        }
    }

    fn compilar_llamada(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Verificar si es llamada a función built-in (Texto / Vector<T>)
        if self.es_llamada_builtin(llamada) {
            return self.compilar_llamada_builtin(llamada, builder, variables);
        }

        // Verificar si es llamada a función genérica
        if self.funciones_genericas.contains_key(&llamada.funcion) {
            return self.compilar_llamada_generica(llamada, builder, variables);
        }

        let func_id = match self.funciones.get(&llamada.funcion).copied() {
            Some(func_id) => func_id,
            None => {
                // Verificar si es una llamada indirecta (variable con closure object)
                if let Some((slot, _tipo, _)) = variables.get(&llamada.funcion) {
                    let slot = *slot;
                    // Cargar puntero al closure object desde la variable
                    let closure_ptr = builder.ins().stack_load(types::I64, slot, 0);

                    // Cargar fn_ptr (offset 0) y env_ptr (offset 8) del closure object
                    let fn_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), closure_ptr, 0);
                    let env_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), closure_ptr, 8);

                    // Compilar argumentos
                    let mut args = vec![env_ptr]; // env_ptr siempre primero
                    for arg in &llamada.argumentos {
                        let val = self.compilar_expresion(arg, builder, variables)?;
                        args.push(val);
                    }

                    // Crear firma para la llamada indirecta
                    let mut sig = Signature::new(self.call_conv_default());
                    sig.params.push(AbiParam::new(types::I64)); // env_ptr
                    for _ in &llamada.argumentos {
                        sig.params.push(AbiParam::new(types::I32)); // default I32
                    }
                    sig.returns.push(AbiParam::new(types::I32)); // default retorno I32

                    let sig_ref = builder.import_signature(sig);
                    let call = builder.ins().call_indirect(sig_ref, fn_ptr, &args);
                    let result = builder.inst_results(call);
                    if result.is_empty() {
                        return Ok(builder.ins().iconst(types::I32, 0));
                    } else {
                        return Ok(result[0]);
                    }
                }

                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::FFI,
                    1,
                    llamada.span.clone(),
                    format!("Función '{}' no encontrada", llamada.funcion),
                ));
                return Err(());
            }
        };
        
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);

        let mut args = Vec::new();
        for arg in &llamada.argumentos {
            let val = self.compilar_expresion(arg, builder, variables)?;
            args.push(val);
        }

        let call = builder.ins().call(func_ref, &args);
        let result = builder.inst_results(call);
        
        if result.is_empty() {
            // Función void - devolver dummy
            Ok(builder.ins().iconst(types::I32, 0))
        } else {
            Ok(result[0])
        }
    }

    // ============================================================
    // Built-ins: Texto y Vector<T>
    // ============================================================

    fn es_llamada_builtin(&self, llamada: &Llamada) -> bool {
        matches!(llamada.funcion.as_str(),
            "imprimir" | "imprimir_linea" | "decir" | "tamaño_de" | "afirmar" |
            "texto_nuevo" | "texto_desde" | "texto_agregar" | "texto_longitud" | "texto_tam" | "texto_liberar" |
            "texto_concatenar" | "texto_subtexto" | "texto_comparar" | "texto_obtener_byte" |
            "archivo_leer" | "archivo_escribir" | "archivo_existe" |
            "abs" | "max" | "min" | "raiz" | "potencia" |
            "vector_nuevo" | "vector_agregar" | "vector_obtener" | "vector_longitud" | "vector_tam" | "vector_liberar" |
            "dormir" |
            "tcp_vincular" | "tcp_aceptar" | "tcp_leer" | "tcp_escribir" | "tcp_cerrar" |
            "canal_nuevo" | "canal_enviar" | "canal_recibir" | "canal_cerrar" | "canal_intentar" |
            "cancelar"
        )
    }

    fn compilar_llamada_builtin(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        match llamada.funcion.as_str() {
            "imprimir" => self.builtin_imprimir(builder, variables, &llamada.argumentos, false),
            "imprimir_linea" | "decir" => self.builtin_imprimir(builder, variables, &llamada.argumentos, true),
            "tamaño_de" => self.builtin_tamano_de(builder, &llamada.tipo_args),
            "afirmar" => self.builtin_afirmar(builder, variables, &llamada.argumentos, &llamada.span),
            "texto_nuevo" => self.builtin_texto_nuevo(builder),
            "texto_desde" => self.builtin_texto_desde(builder, variables, &llamada.argumentos),
            "texto_agregar" => self.builtin_texto_agregar(builder, variables, &llamada.argumentos),
            "texto_longitud" | "texto_tam" => self.builtin_texto_longitud(builder, variables, &llamada.argumentos),
            "texto_liberar" => self.builtin_texto_liberar(builder, variables, &llamada.argumentos),
            "texto_concatenar" => self.builtin_texto_concatenar(builder, variables, &llamada.argumentos),
            "texto_subtexto" => self.builtin_texto_subtexto(builder, variables, &llamada.argumentos),
            "texto_comparar" => self.builtin_texto_comparar(builder, variables, &llamada.argumentos),
            "texto_obtener_byte" => self.builtin_texto_obtener_byte(builder, variables, &llamada.argumentos),
            "archivo_leer" => self.builtin_archivo_leer(builder, variables, &llamada.argumentos),
            "archivo_escribir" => self.builtin_archivo_escribir(builder, variables, &llamada.argumentos),
            "archivo_existe" => self.builtin_archivo_existe(builder, variables, &llamada.argumentos),
            "abs" => self.builtin_abs(builder, variables, &llamada.argumentos),
            "max" => self.builtin_max(builder, variables, &llamada.argumentos),
            "min" => self.builtin_min(builder, variables, &llamada.argumentos),
            "raiz" => self.builtin_raiz(builder, variables, &llamada.argumentos),
            "potencia" => self.builtin_potencia(builder, variables, &llamada.argumentos),
            "vector_nuevo" => self.builtin_vector_nuevo(builder, &llamada.tipo_args),
            "vector_agregar" => self.builtin_vector_agregar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "vector_obtener" => self.builtin_vector_obtener(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "vector_longitud" | "vector_tam" => self.builtin_vector_longitud(builder, variables, &llamada.argumentos),
            "vector_liberar" => self.builtin_vector_liberar(builder, variables, &llamada.argumentos),
            "dormir" => self.builtin_dormir(builder, variables, &llamada.argumentos),
            "tcp_vincular" => self.builtin_tcp_vincular(builder, variables, &llamada.argumentos),
            "tcp_aceptar" => self.builtin_tcp_aceptar(builder, variables, &llamada.argumentos),
            "tcp_leer" => self.builtin_tcp_leer(builder, variables, &llamada.argumentos),
            "tcp_escribir" => self.builtin_tcp_escribir(builder, variables, &llamada.argumentos),
            "tcp_cerrar" => self.builtin_tcp_cerrar(builder, variables, &llamada.argumentos),
            "canal_nuevo" => self.builtin_canal_nuevo(builder, variables, &llamada.argumentos),
            "canal_enviar" => self.builtin_canal_enviar(builder, variables, &llamada.argumentos),
            "canal_recibir" => self.builtin_canal_recibir(builder, variables, &llamada.argumentos),
            "canal_cerrar" => self.builtin_canal_cerrar(builder, variables, &llamada.argumentos),
            "cancelar" => self.builtin_cancelar(builder, variables),
            "canal_intentar" => self.builtin_canal_intentar(builder, variables, &llamada.argumentos),
            _ => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    80,
                    llamada.span.clone(),
                    format!("Función built-in '{}' no implementada", llamada.funcion),
                ));
                Err(())
            }
        }
    }

    fn builtin_imprimir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        con_newline: bool,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Verificar si hay interpolación: literal con {variable}
        if let Expresion::Literal(Literal::Palabra(texto, _)) = &argumentos[0] {
            if texto.contains('{') {
                return self.builtin_imprimir_interpolado(builder, variables, texto, con_newline);
            }
        }

        // Inferir tipo del argumento para dispatch
        let tipo_arg = self.inferir_tipo(&argumentos[0], variables);

        match tipo_arg {
            Tipo::Texto => {
                // Texto: extraer ptr y usar puts/printf %s
                let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
                if con_newline {
                    let func_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[ptr]);
                } else {
                    let fmt_ptr = self.crear_string_literal(builder, "%s");
                    let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[fmt_ptr, ptr]);
                }
            }
            Tipo::Entero32 | Tipo::Entero64 | Tipo::Entero8 | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 => {
                // Enteros: printf %d — en Windows x64, args variádicos se pasan como I64
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let fmt = if con_newline { "%d\n" } else { "%d" };
                let fmt_ptr = self.crear_string_literal(builder, fmt);
                // Extender a I64 para passing variádico correcto en Windows x64
                let val_i64 = match tipo_arg {
                    Tipo::Entero8 | Tipo::Natural8 | Tipo::Booleano | Tipo::Caracter => {
                        builder.ins().uextend(types::I64, val)
                    }
                    Tipo::Entero16 | Tipo::Natural16 => {
                        builder.ins().uextend(types::I64, val)
                    }
                    Tipo::Entero32 | Tipo::Natural32 => {
                        builder.ins().sextend(types::I64, val)
                    }
                    _ => val, // Ya es I64
                };
                let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                builder.ins().call(func_ref, &[fmt_ptr, val_i64]);
            }
            Tipo::Booleano => {
                // Booleano: imprimir "verdadero"/"falso"
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let val_i32 = builder.ins().uextend(types::I32, val);
                let cero = builder.ins().iconst(types::I32, 0);
                let es_falso = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, val_i32, cero);
                let bloque_true = builder.create_block();
                let bloque_false = builder.create_block();
                let bloque_fin = builder.create_block();
                builder.ins().brif(es_falso, bloque_false, &[], bloque_true, &[]);

                builder.switch_to_block(bloque_true);
                if con_newline {
                    let msg_true = self.crear_string_literal(builder, "verdadero");
                    let puts_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let puts_ref = self.module.declare_func_in_func(puts_id, builder.func);
                    builder.ins().call(puts_ref, &[msg_true]);
                } else {
                    let msg_true = self.crear_string_literal(builder, "verdadero");
                    let fmt_ptr = self.crear_string_literal(builder, "%s");
                    let printf_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let printf_ref = self.module.declare_func_in_func(printf_id, builder.func);
                    builder.ins().call(printf_ref, &[fmt_ptr, msg_true]);
                }
                builder.ins().jump(bloque_fin, &[]);
                builder.seal_block(bloque_true);

                builder.switch_to_block(bloque_false);
                if con_newline {
                    let msg_false = self.crear_string_literal(builder, "falso");
                    let puts_id2 = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let puts_ref2 = self.module.declare_func_in_func(puts_id2, builder.func);
                    builder.ins().call(puts_ref2, &[msg_false]);
                } else {
                    let msg_false = self.crear_string_literal(builder, "falso");
                    let fmt_ptr2 = self.crear_string_literal(builder, "%s");
                    let printf_id2 = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let printf_ref2 = self.module.declare_func_in_func(printf_id2, builder.func);
                    builder.ins().call(printf_ref2, &[fmt_ptr2, msg_false]);
                }
                builder.ins().jump(bloque_fin, &[]);
                builder.seal_block(bloque_false);

                builder.switch_to_block(bloque_fin);
                builder.seal_block(bloque_fin);
            }
            Tipo::Flotante32 | Tipo::Flotante64 => {
                // Floats: printf %f
                // Windows x64 variadic ABI: doubles se pasan como bit pattern en reg entero
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let fmt = if con_newline { "%f\n" } else { "%f" };
                let fmt_ptr = self.crear_string_literal(builder, fmt);
                // Bitcast F64 → I64 para passing variádico correcto
                let val_bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), val);
                let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                builder.ins().call(func_ref, &[fmt_ptr, val_bits]);
            }
            _ => {
                // Palabra u otro puntero: camino original
                let msg_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
                if con_newline {
                    let func_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[msg_ptr]);
                } else {
                    let fmt_ptr = self.crear_string_literal(builder, "%s");
                    let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[fmt_ptr, msg_ptr]);
                }
            }
        }

        Ok(builder.ins().iconst(types::I64, 0))
    }

    /// afirmar(condición) — aborta si la condición es falsa
    fn builtin_afirmar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        span: &crate::span::Span,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if argumentos.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                75,
                span.clone(),
                "'afirmar' requiere un argumento booleano".to_string(),
            ));
            return Err(());
        }

        let cond = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Si condición es falsa → imprimir error y terminar
        let bloque_fallo = builder.create_block();
        let bloque_ok = builder.create_block();

        // Extender condición a I32 para comparación segura
        let cond_i32 = builder.ins().uextend(types::I32, cond);
        let cero = builder.ins().iconst(types::I32, 0);
        let es_falso = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, cond_i32, cero);
        builder.ins().brif(es_falso, bloque_fallo, &[], bloque_ok, &[]);

        // Bloque fallo: puts("  FALLO: afirmación fallida") + ExitProcess(1)
        builder.switch_to_block(bloque_fallo);
        builder.seal_block(bloque_fallo);

        let msg = self.crear_string_literal(builder, "  FALLO: afirmación fallida");
        let puts_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
        let puts_ref = self.module.declare_func_in_func(puts_id, builder.func);
        builder.ins().call(puts_ref, &[msg]);

        let exit_id = self.asegurar_funcion_c("ExitProcess", &[types::I32], None);
        let exit_ref = self.module.declare_func_in_func(exit_id, builder.func);
        let uno = builder.ins().iconst(types::I32, 1);
        builder.ins().call(exit_ref, &[uno]);
        builder.ins().trap(cranelift_codegen::ir::TrapCode::UnreachableCodeReached);

        // Bloque OK: continuar
        builder.switch_to_block(bloque_ok);
        builder.seal_block(bloque_ok);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// dormir(ms) — MVP: llama a Sleep(ms) de kernel32 (bloquea el thread)
    /// TODO Fase 18B: integrar con reactor IOCP para suspensión real de tarea
    fn builtin_dormir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if argumentos.is_empty() {
            return Ok(builder.ins().iconst(types::I32, 0));
        }

        let ms_val = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Sleep(DWORD ms) — kernel32.dll, Windows x64 fastcall
        // DWORD es u32. Si el valor ya es I32, usar directo; si es I64, truncar.
        let ms_i32 = if builder.func.dfg.value_type(ms_val) == types::I64 {
            builder.ins().ireduce(types::I32, ms_val)
        } else {
            ms_val
        };

        let sleep_id = self.asegurar_funcion_c("Sleep", &[types::I32], None);
        let sleep_ref = self.module.declare_func_in_func(sleep_id, builder.func);
        builder.ins().call(sleep_ref, &[ms_i32]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    // === TCP Builtins (Fase 18B) — Winsock2 directo ===

    /// tcp_vincular(puerto) -> Entero64 (socket handle)
    /// Crea socket TCP, bind a 0.0.0.0:puerto, listen(128)
    fn builtin_tcp_vincular(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let puerto_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let puerto_i32 = if builder.func.dfg.value_type(puerto_val) == types::I64 {
            builder.ins().ireduce(types::I32, puerto_val)
        } else {
            puerto_val
        };

        // WSAStartup(0x0202, &wsadata) — wsadata: 408 bytes en stack
        let wsadata_slot = builder.create_sized_stack_slot(
            cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 408, 0));
        let wsadata_ptr = builder.ins().stack_addr(types::I64, wsadata_slot, 0);
        let wsa_version = builder.ins().iconst(types::I32, 0x0202);
        let wsa_startup_id = self.asegurar_funcion_c("WSAStartup", &[types::I32, types::I64], Some(types::I32));
        let wsa_startup_ref = self.module.declare_func_in_func(wsa_startup_id, builder.func);
        builder.ins().call(wsa_startup_ref, &[wsa_version, wsadata_ptr]);

        // socket(AF_INET=2, SOCK_STREAM=1, IPPROTO_TCP=6) -> SOCKET (u64)
        let socket_id = self.asegurar_funcion_c("socket", &[types::I32, types::I32, types::I32], Some(types::I64));
        let socket_ref = self.module.declare_func_in_func(socket_id, builder.func);
        let af_inet = builder.ins().iconst(types::I32, 2);
        let sock_stream = builder.ins().iconst(types::I32, 1);
        let ipproto_tcp = builder.ins().iconst(types::I32, 6);
        let call_socket = builder.ins().call(socket_ref, &[af_inet, sock_stream, ipproto_tcp]);
        let sock = builder.inst_results(call_socket)[0];

        // sockaddr_in (16 bytes): family(u16) + port(u16) + addr(u32) + zero(u64)
        let addr_slot = builder.create_sized_stack_slot(
            cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 16, 0));
        let addr_ptr = builder.ins().stack_addr(types::I64, addr_slot, 0);

        // sin_family = AF_INET = 2 (u16 at offset 0)
        let family_val = builder.ins().iconst(types::I16, 2);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), family_val, addr_ptr, 0);

        // sin_port = htons(puerto) — byte swap manual en little-endian
        // htons(x) = ((x & 0xFF) << 8) | ((x >> 8) & 0xFF)
        let mask_ff = builder.ins().iconst(types::I32, 0xFF);
        let low_byte = builder.ins().band(puerto_i32, mask_ff);
        let eight = builder.ins().iconst(types::I32, 8);
        let low_shifted = builder.ins().ishl(low_byte, eight);
        let eight2 = builder.ins().iconst(types::I32, 8);
        let high_byte = builder.ins().ushr(puerto_i32, eight2);
        let high_masked = builder.ins().band(high_byte, mask_ff);
        let port_net = builder.ins().bor(low_shifted, high_masked);
        let port_i16 = builder.ins().ireduce(types::I16, port_net);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), port_i16, addr_ptr, 2);

        // sin_addr = INADDR_ANY = 0 (u32 at offset 4)
        let zero_i32 = builder.ins().iconst(types::I32, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_i32, addr_ptr, 4);

        // bind(sock, &addr, 16)
        let bind_id = self.asegurar_funcion_c("bind", &[types::I64, types::I64, types::I32], Some(types::I32));
        let bind_ref = self.module.declare_func_in_func(bind_id, builder.func);
        let addr_len = builder.ins().iconst(types::I32, 16);
        builder.ins().call(bind_ref, &[sock, addr_ptr, addr_len]);

        // listen(sock, 128)
        let listen_id = self.asegurar_funcion_c("listen", &[types::I64, types::I32], Some(types::I32));
        let listen_ref = self.module.declare_func_in_func(listen_id, builder.func);
        let backlog = builder.ins().iconst(types::I32, 128);
        builder.ins().call(listen_ref, &[sock, backlog]);

        Ok(sock)
    }

    /// tcp_aceptar(listener) -> Entero64 (client socket)
    fn builtin_tcp_aceptar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let listener_val = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // accept(sock, NULL, NULL) -> SOCKET
        let accept_id = self.asegurar_funcion_c("accept", &[types::I64, types::I64, types::I64], Some(types::I64));
        let accept_ref = self.module.declare_func_in_func(accept_id, builder.func);
        let null_val = builder.ins().iconst(types::I64, 0);
        let call_accept = builder.ins().call(accept_ref, &[listener_val, null_val, null_val]);
        let client_sock = builder.inst_results(call_accept)[0];

        Ok(client_sock)
    }

    /// tcp_leer(socket, buffer_ptr, tam) -> Entero32 (bytes leídos)
    fn builtin_tcp_leer(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let sock_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let buf_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let tam_val = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let tam_i32 = if builder.func.dfg.value_type(tam_val) == types::I64 {
            builder.ins().ireduce(types::I32, tam_val)
        } else {
            tam_val
        };

        // recv(sock, buf, len, 0) -> int
        let recv_id = self.asegurar_funcion_c("recv", &[types::I64, types::I64, types::I32, types::I32], Some(types::I32));
        let recv_ref = self.module.declare_func_in_func(recv_id, builder.func);
        let flags_zero = builder.ins().iconst(types::I32, 0);
        let call_recv = builder.ins().call(recv_ref, &[sock_val, buf_val, tam_i32, flags_zero]);
        let bytes_read = builder.inst_results(call_recv)[0];

        Ok(bytes_read)
    }

    /// tcp_escribir(socket, buffer_ptr, tam) -> Entero32 (bytes escritos)
    fn builtin_tcp_escribir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let sock_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let buf_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let tam_val = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let tam_i32 = if builder.func.dfg.value_type(tam_val) == types::I64 {
            builder.ins().ireduce(types::I32, tam_val)
        } else {
            tam_val
        };

        // send(sock, buf, len, 0) -> int
        let send_id = self.asegurar_funcion_c("send", &[types::I64, types::I64, types::I32, types::I32], Some(types::I32));
        let send_ref = self.module.declare_func_in_func(send_id, builder.func);
        let flags_zero = builder.ins().iconst(types::I32, 0);
        let call_send = builder.ins().call(send_ref, &[sock_val, buf_val, tam_i32, flags_zero]);
        let bytes_sent = builder.inst_results(call_send)[0];

        Ok(bytes_sent)
    }

    /// tcp_cerrar(socket) -> void
    fn builtin_tcp_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let sock_val = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // closesocket(sock) -> int
        let close_id = self.asegurar_funcion_c("closesocket", &[types::I64], Some(types::I32));
        let close_ref = self.module.declare_func_in_func(close_id, builder.func);
        builder.ins().call(close_ref, &[sock_val]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    // === Canal Builtins (Fase 18C) — Mutex + Semaphore + Ring Buffer ===
    // Layout del canal en heap (malloc):
    //   offset 0:  HANDLE mutex (8 bytes)
    //   offset 8:  HANDLE semaphore (8 bytes)
    //   offset 16: i32 head
    //   offset 20: i32 tail
    //   offset 24: i32 count
    //   offset 28: i32 capacity
    //   offset 32: buffer[capacity] (i32 * capacity)

    /// canal_nuevo(capacidad) -> Entero64 (puntero al canal)
    fn builtin_canal_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let cap_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let cap_i32 = if builder.func.dfg.value_type(cap_val) == types::I64 {
            builder.ins().ireduce(types::I32, cap_val)
        } else {
            cap_val
        };

        // total_size = 32 + capacity * 4
        let cap_i64 = builder.ins().sextend(types::I64, cap_i32);
        let four = builder.ins().iconst(types::I64, 4);
        let buf_size = builder.ins().imul(cap_i64, four);
        let header_size = builder.ins().iconst(types::I64, 32);
        let total_size = builder.ins().iadd(buf_size, header_size);

        // malloc(total_size)
        let malloc_id = self.asegurar_funcion_c("malloc", &[types::I64], Some(types::I64));
        let malloc_ref = self.module.declare_func_in_func(malloc_id, builder.func);
        let call_malloc = builder.ins().call(malloc_ref, &[total_size]);
        let canal_ptr = builder.inst_results(call_malloc)[0];

        // CreateMutexW(NULL, FALSE, NULL) -> HANDLE
        let create_mutex_id = self.asegurar_funcion_c("CreateMutexW", &[types::I64, types::I32, types::I64], Some(types::I64));
        let create_mutex_ref = self.module.declare_func_in_func(create_mutex_id, builder.func);
        let null_val = builder.ins().iconst(types::I64, 0);
        let false_val = builder.ins().iconst(types::I32, 0);
        let call_mutex = builder.ins().call(create_mutex_ref, &[null_val, false_val, null_val]);
        let mutex_handle = builder.inst_results(call_mutex)[0];

        // CreateSemaphoreW(NULL, 0, capacity, NULL) -> HANDLE
        let create_sem_id = self.asegurar_funcion_c("CreateSemaphoreW", &[types::I64, types::I32, types::I32, types::I64], Some(types::I64));
        let create_sem_ref = self.module.declare_func_in_func(create_sem_id, builder.func);
        let zero_i32 = builder.ins().iconst(types::I32, 0);
        let call_sem = builder.ins().call(create_sem_ref, &[null_val, zero_i32, cap_i32, null_val]);
        let sem_handle = builder.inst_results(call_sem)[0];

        // Guardar en el struct del canal
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), mutex_handle, canal_ptr, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), sem_handle, canal_ptr, 8);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_i32, canal_ptr, 16); // head = 0
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_i32, canal_ptr, 20); // tail = 0
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), zero_i32, canal_ptr, 24); // count = 0
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cap_i32, canal_ptr, 28);  // capacity

        Ok(canal_ptr)
    }

    /// canal_enviar(canal, valor) — WaitForSingleObject(mutex), write ring buffer, ReleaseMutex, ReleaseSemaphore
    fn builtin_canal_enviar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let valor = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let valor_i32 = if builder.func.dfg.value_type(valor) == types::I64 {
            builder.ins().ireduce(types::I32, valor)
        } else {
            valor
        };

        // Cargar mutex handle (offset 0)
        let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 0);

        // WaitForSingleObject(mutex, INFINITE=0xFFFFFFFF)
        let wfs_id = self.asegurar_funcion_c("WaitForSingleObject", &[types::I64, types::I32], Some(types::I32));
        let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);
        let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
        builder.ins().call(wfs_ref, &[mutex_handle, infinite]);

        // Cargar tail (offset 20) y capacity (offset 28)
        let tail = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 20);
        let capacity = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 28);

        // buffer[32 + tail*4] = valor
        let tail_i64 = builder.ins().sextend(types::I64, tail);
        let four = builder.ins().iconst(types::I64, 4);
        let offset_buf = builder.ins().imul(tail_i64, four);
        let base_offset = builder.ins().iconst(types::I64, 32);
        let write_offset = builder.ins().iadd(base_offset, offset_buf);
        // store valor at canal_ptr + write_offset
        let write_addr = builder.ins().iadd(canal_ptr, write_offset);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), valor_i32, write_addr, 0);

        // tail = (tail + 1) % capacity
        let one = builder.ins().iconst(types::I32, 1);
        let tail_plus1 = builder.ins().iadd(tail, one);
        // Para modulo: si tail+1 >= capacity, tail = 0; else tail = tail+1
        let new_tail = builder.ins().urem(tail_plus1, capacity);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), new_tail, canal_ptr, 20);

        // count++ (offset 24)
        let count = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 24);
        let count_plus1 = builder.ins().iadd(count, one);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), count_plus1, canal_ptr, 24);

        // ReleaseMutex(mutex)
        let rel_mutex_id = self.asegurar_funcion_c("ReleaseMutex", &[types::I64], Some(types::I32));
        let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
        builder.ins().call(rel_mutex_ref, &[mutex_handle]);

        // ReleaseSemaphore(semaphore, 1, NULL)
        let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 8);
        let rel_sem_id = self.asegurar_funcion_c("ReleaseSemaphore", &[types::I64, types::I32, types::I64], Some(types::I32));
        let rel_sem_ref = self.module.declare_func_in_func(rel_sem_id, builder.func);
        let null_val = builder.ins().iconst(types::I64, 0);
        let one_i32 = builder.ins().iconst(types::I32, 1);
        builder.ins().call(rel_sem_ref, &[sem_handle, one_i32, null_val]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// canal_recibir(canal) -> Entero32 — WaitForSingleObject(semaphore), lock mutex, read ring buffer, unlock
    fn builtin_canal_recibir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Cargar semaphore handle (offset 8)
        let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 8);

        // WaitForSingleObject(semaphore, INFINITE) — bloquea si canal vacío
        let wfs_id = self.asegurar_funcion_c("WaitForSingleObject", &[types::I64, types::I32], Some(types::I32));
        let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);
        let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
        builder.ins().call(wfs_ref, &[sem_handle, infinite]);

        // Cargar mutex handle (offset 0) y lock
        let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 0);
        let wfs_ref2 = self.module.declare_func_in_func(wfs_id, builder.func);
        let infinite2 = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
        builder.ins().call(wfs_ref2, &[mutex_handle, infinite2]);

        // Cargar head (offset 16) y capacity (offset 28)
        let head = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 16);
        let capacity = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 28);

        // valor = buffer[32 + head*4]
        let head_i64 = builder.ins().sextend(types::I64, head);
        let four = builder.ins().iconst(types::I64, 4);
        let offset_buf = builder.ins().imul(head_i64, four);
        let base_offset = builder.ins().iconst(types::I64, 32);
        let read_offset = builder.ins().iadd(base_offset, offset_buf);
        let read_addr = builder.ins().iadd(canal_ptr, read_offset);
        let valor = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), read_addr, 0);

        // head = (head + 1) % capacity
        let one = builder.ins().iconst(types::I32, 1);
        let head_plus1 = builder.ins().iadd(head, one);
        let new_head = builder.ins().urem(head_plus1, capacity);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), new_head, canal_ptr, 16);

        // count-- (offset 24)
        let count = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 24);
        let neg_one = builder.ins().iconst(types::I32, -1i64);
        let count_minus1 = builder.ins().iadd(count, neg_one);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), count_minus1, canal_ptr, 24);

        // ReleaseMutex(mutex)
        let rel_mutex_id = self.asegurar_funcion_c("ReleaseMutex", &[types::I64], Some(types::I32));
        let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
        builder.ins().call(rel_mutex_ref, &[mutex_handle]);

        Ok(valor)
    }

    /// canal_cerrar(canal) — CloseHandle(mutex), CloseHandle(semaphore), free(canal)
    fn builtin_canal_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // CloseHandle(mutex)
        let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 0);
        let close_handle_id = self.asegurar_funcion_c("CloseHandle", &[types::I64], Some(types::I32));
        let close_handle_ref = self.module.declare_func_in_func(close_handle_id, builder.func);
        builder.ins().call(close_handle_ref, &[mutex_handle]);

        // CloseHandle(semaphore)
        let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 8);
        let close_handle_ref2 = self.module.declare_func_in_func(close_handle_id, builder.func);
        builder.ins().call(close_handle_ref2, &[sem_handle]);

        // free(canal)
        let free_id = self.asegurar_funcion_c("free", &[types::I64], None);
        let free_ref = self.module.declare_func_in_func(free_id, builder.func);
        builder.ins().call(free_ref, &[canal_ptr]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// cancelar() — cancela el executor activo (structured cancellation)
    /// Setea cancelled=1 en el pool y despierta todos los workers
    fn builtin_cancelar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Buscar el pool del executor activo
        if let Some(ref pool_var) = self.executor_pool_var {
            if let Some(&(pool_slot, _, _)) = variables.get(pool_var) {
                let pool_ptr = builder.ins().stack_load(types::I64, pool_slot, 0);

                // Lock mutex
                let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 0);
                let wfs_id = self.asegurar_funcion_c("WaitForSingleObject", &[types::I64, types::I32], Some(types::I32));
                let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);
                let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
                builder.ins().call(wfs_ref, &[mutex_handle, infinite]);

                // cancelled = 1 (offset 60)
                let one = builder.ins().iconst(types::I32, 1);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), one, pool_ptr, 60);

                // num_workers para despertar (offset 56)
                let num_workers = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 56);

                // Unlock mutex
                let rel_mutex_id = self.asegurar_funcion_c("ReleaseMutex", &[types::I64], Some(types::I32));
                let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
                builder.ins().call(rel_mutex_ref, &[mutex_handle]);

                // ReleaseSemaphore(sem, num_workers, NULL) — despertar todos
                let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 8);
                let rel_sem_id = self.asegurar_funcion_c("ReleaseSemaphore", &[types::I64, types::I32, types::I64], Some(types::I32));
                let rs_ref = self.module.declare_func_in_func(rel_sem_id, builder.func);
                let null_ptr = builder.ins().iconst(types::I64, 0);
                builder.ins().call(rs_ref, &[sem_handle, num_workers, null_ptr]);

                return Ok(builder.ins().iconst(types::I32, 0));
            }
        }
        // Sin executor activo: no-op
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// canal_intentar(canal) -> Entero32 — non-blocking try_recv
    /// WaitForSingleObject(semaphore, 0): si hay dato lo retorna, si no retorna i32::MIN
    fn builtin_canal_intentar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Cargar semaphore handle (offset 8)
        let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 8);

        // WaitForSingleObject(semaphore, 0) — timeout 0 = non-blocking
        let wfs_id = self.asegurar_funcion_c("WaitForSingleObject", &[types::I64, types::I32], Some(types::I32));
        let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);
        let zero_timeout = builder.ins().iconst(types::I32, 0);
        let call_wfs = builder.ins().call(wfs_ref, &[sem_handle, zero_timeout]);
        let wait_result = builder.inst_results(call_wfs)[0];

        // WAIT_OBJECT_0 = 0 → hay dato; WAIT_TIMEOUT = 258 → vacío
        let bloque_hay_dato = builder.create_block();
        let bloque_vacio = builder.create_block();
        let bloque_fin = builder.create_block();
        builder.append_block_param(bloque_fin, types::I32);

        let wait_object_0 = builder.ins().iconst(types::I32, 0);
        let es_dato = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, wait_result, wait_object_0);
        builder.ins().brif(es_dato, bloque_hay_dato, &[], bloque_vacio, &[]);

        // Bloque vacío: retornar i32::MIN (-2147483648)
        builder.switch_to_block(bloque_vacio);
        builder.seal_block(bloque_vacio);
        let sentinel = builder.ins().iconst(types::I32, -2147483648i64);
        builder.ins().jump(bloque_fin, &[sentinel]);

        // Bloque hay dato: lock mutex, leer del ring buffer, unlock
        builder.switch_to_block(bloque_hay_dato);
        builder.seal_block(bloque_hay_dato);

        let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 0);
        let wfs_ref2 = self.module.declare_func_in_func(wfs_id, builder.func);
        let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
        builder.ins().call(wfs_ref2, &[mutex_handle, infinite]);

        // Leer head (offset 16) y capacity (offset 28)
        let head = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 16);
        let capacity = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 28);

        // valor = buffer[32 + head*4]
        let head_i64 = builder.ins().sextend(types::I64, head);
        let four = builder.ins().iconst(types::I64, 4);
        let offset_buf = builder.ins().imul(head_i64, four);
        let base_offset = builder.ins().iconst(types::I64, 32);
        let read_offset = builder.ins().iadd(base_offset, offset_buf);
        let read_addr = builder.ins().iadd(canal_ptr, read_offset);
        let valor = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), read_addr, 0);

        // head = (head + 1) % capacity
        let one = builder.ins().iconst(types::I32, 1);
        let head_plus1 = builder.ins().iadd(head, one);
        let new_head = builder.ins().urem(head_plus1, capacity);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), new_head, canal_ptr, 16);

        // count-- (offset 24)
        let count = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), canal_ptr, 24);
        let neg_one = builder.ins().iconst(types::I32, -1i64);
        let count_minus1 = builder.ins().iadd(count, neg_one);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), count_minus1, canal_ptr, 24);

        // ReleaseMutex
        let rel_mutex_id = self.asegurar_funcion_c("ReleaseMutex", &[types::I64], Some(types::I32));
        let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
        builder.ins().call(rel_mutex_ref, &[mutex_handle]);

        builder.ins().jump(bloque_fin, &[valor]);

        // Bloque fin: recibir el resultado
        builder.switch_to_block(bloque_fin);
        builder.seal_block(bloque_fin);
        let resultado = builder.block_params(bloque_fin)[0];
        Ok(resultado)
    }

    /// lanzar f(args...) — MVP: crea un thread real del OS con CreateThread
    /// Genera un wrapper __hilo_N que CreateThread puede llamar (firma: fn(i64) -> i32)
    fn compilar_lanzar_hilo(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Generar nombre único para el wrapper
        let nombre_wrapper = format!("__hilo_{}", self.contador_closures);
        self.contador_closures += 1;

        // Evaluar argumentos y guardarlos en un buffer heap (malloc)
        // Layout del buffer: cada arg ocupa 8 bytes (alineado a i64)
        let num_args = llamada.argumentos.len();
        let buffer_size = (num_args * 8) as i64;

        // malloc(buffer_size)
        let malloc_id = self.asegurar_funcion_c("malloc", &[types::I64], Some(types::I64));
        let malloc_ref = self.module.declare_func_in_func(malloc_id, builder.func);
        let size_val = builder.ins().iconst(types::I64, buffer_size.max(8));
        let call_malloc = builder.ins().call(malloc_ref, &[size_val]);
        let buffer_ptr = builder.inst_results(call_malloc)[0];

        // Guardar cada argumento en el buffer (offset = i * 8)
        for (i, arg) in llamada.argumentos.iter().enumerate() {
            let arg_val = self.compilar_expresion(arg, builder, variables)?;
            let offset = (i * 8) as i32;
            // Si el valor es I32, extender a I64 para almacenamiento uniforme
            let arg_i64 = if builder.func.dfg.value_type(arg_val) == types::I32 {
                builder.ins().sextend(types::I64, arg_val)
            } else if builder.func.dfg.value_type(arg_val) == types::I8 {
                builder.ins().sextend(types::I64, arg_val)
            } else {
                arg_val
            };
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), arg_i64, buffer_ptr, offset);
        }

        // Declarar el wrapper como función externa (se compilará después)
        let mut sig_wrapper = Signature::new(self.call_conv_default());
        sig_wrapper.params.push(AbiParam::new(types::I64)); // LPVOID lpParameter
        sig_wrapper.returns.push(AbiParam::new(types::I32)); // DWORD retorno

        let wrapper_id = self.module.declare_function(&nombre_wrapper, Linkage::Local, &sig_wrapper)
            .map_err(|_| ())?;
        let wrapper_ref = self.module.declare_func_in_func(wrapper_id, builder.func);

        // Registrar el hilo pendiente para compilación diferida (con FuncId ya declarada)
        // Guardar tipos Cranelift de cada argumento para desempacar correctamente
        let arg_types: Vec<cranelift_codegen::ir::Type> = llamada.argumentos.iter()
            .map(|arg| {
                let tipo = self.inferir_tipo(arg, variables);
                self.tipo_a_cranelift(&tipo)
            })
            .collect();
        self.hilos_pendientes.push(HiloPendiente {
            nombre: nombre_wrapper.clone(),
            llamada: llamada.clone(),
            func_id_module: wrapper_id,
            arg_types,
        });

        // Obtener puntero a la función wrapper (func_addr)
        let wrapper_addr = builder.ins().func_addr(types::I64, wrapper_ref);

        // Si hay executor activo, encolar al pool en vez de CreateThread
        if let Some(ref pool_var) = self.executor_pool_var {
            if let Some(&(pool_slot, _, _)) = variables.get(pool_var) {
                let pool_ptr = builder.ins().stack_load(types::I64, pool_slot, 0);

                // Lock mutex: WaitForSingleObject(mutex, INFINITE)
                let mutex_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 0);
                let wfs_id = self.asegurar_funcion_c("WaitForSingleObject", &[types::I64, types::I32], Some(types::I32));
                let wfs_ref = self.module.declare_func_in_func(wfs_id, builder.func);
                let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF_u32 as i64);
                builder.ins().call(wfs_ref, &[mutex_handle, infinite]);

                // Escribir task en queue[tail]: {fn_ptr, args_ptr}
                let tail = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 36);
                let capacity = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 44);
                let tail_i64 = builder.ins().sextend(types::I64, tail);
                let sixteen = builder.ins().iconst(types::I64, 16);
                let task_offset = builder.ins().imul(tail_i64, sixteen);
                let base_64 = builder.ins().iconst(types::I64, 64);
                let task_addr_offset = builder.ins().iadd(base_64, task_offset);
                let task_addr = builder.ins().iadd(pool_ptr, task_addr_offset);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), wrapper_addr, task_addr, 0);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), buffer_ptr, task_addr, 8);

                // tail = (tail + 1) % capacity
                let one_i32 = builder.ins().iconst(types::I32, 1);
                let tail_plus1 = builder.ins().iadd(tail, one_i32);
                let new_tail = builder.ins().urem(tail_plus1, capacity);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), new_tail, pool_ptr, 36);

                // count++
                let count = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 40);
                let count_plus1 = builder.ins().iadd(count, one_i32);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), count_plus1, pool_ptr, 40);

                // Unlock mutex: ReleaseMutex(mutex)
                let rel_mutex_id = self.asegurar_funcion_c("ReleaseMutex", &[types::I64], Some(types::I32));
                let rel_mutex_ref = self.module.declare_func_in_func(rel_mutex_id, builder.func);
                builder.ins().call(rel_mutex_ref, &[mutex_handle]);

                // ReleaseSemaphore(sem, 1, NULL) — despertar un worker
                let sem_handle = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), pool_ptr, 8);
                let rel_sem_id = self.asegurar_funcion_c("ReleaseSemaphore", &[types::I64, types::I32, types::I64], Some(types::I32));
                let rs_ref = self.module.declare_func_in_func(rel_sem_id, builder.func);
                let null_ptr = builder.ins().iconst(types::I64, 0);
                builder.ins().call(rs_ref, &[sem_handle, one_i32, null_ptr]);

                return Ok(builder.ins().iconst(types::I64, 0));
            }
        }

        // Fallback: CreateThread directo (sin executor)
        // CreateThread(NULL, 0, wrapper_addr, buffer_ptr, 0, NULL)
        let create_thread_id = self.asegurar_funcion_c(
            "CreateThread",
            &[types::I64, types::I64, types::I64, types::I64, types::I32, types::I64],
            Some(types::I64),
        );
        let create_thread_ref = self.module.declare_func_in_func(create_thread_id, builder.func);

        let null_val = builder.ins().iconst(types::I64, 0);
        let zero_i32 = builder.ins().iconst(types::I32, 0);
        let call_ct = builder.ins().call(create_thread_ref, &[
            null_val,       // lpThreadAttributes = NULL
            null_val,       // dwStackSize = 0 (default)
            wrapper_addr,   // lpStartAddress = wrapper
            buffer_ptr,     // lpParameter = buffer con args
            zero_i32,       // dwCreationFlags = 0 (run immediately)
            null_val,       // lpThreadId = NULL
        ]);
        let _thread_handle = builder.inst_results(call_ct)[0];

        // Retornar 0 (handle no se usa en MVP)
        Ok(builder.ins().iconst(types::I64, 0))
    }

    /// Compila los wrappers de hilos (__hilo_N) como funciones independientes.
    /// Cada wrapper: fn(i64 buffer_ptr) -> i32 { desempaca args, llama target, retorna 0 }
    fn compilar_hilos_pendientes(&mut self) {
        let hilos = std::mem::take(&mut self.hilos_pendientes);

        for hilo in hilos {
            // Usar el FuncId ya declarado en compilar_lanzar_hilo
            let func_id = hilo.func_id_module;

            let mut sig = Signature::new(self.call_conv_default());
            sig.params.push(AbiParam::new(types::I64)); // buffer_ptr
            sig.returns.push(AbiParam::new(types::I32)); // DWORD

            let mut ctx = self.module.make_context();
            ctx.func.signature = sig;
            let mut func_ctx = FunctionBuilderContext::new();

            {
                let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
                let entry_block = builder.create_block();
                builder.append_block_params_for_function_params(entry_block);
                builder.switch_to_block(entry_block);
                builder.seal_block(entry_block);

                let buffer_ptr = builder.block_params(entry_block)[0];

                let nombre_target = hilo.llamada.funcion.clone();
                let num_args = hilo.llamada.argumentos.len();

                // Verificar si el target es un futuro (existe __poll_NOMBRE)
                let nombre_poll = format!("__poll_{}", nombre_target);
                let nombre_init = format!("__init_{}", nombre_target);
                let es_futuro = self.funciones.contains_key(&nombre_poll) && self.funciones.contains_key(&nombre_init);

                if es_futuro {
                    // Futuro: __init(args) + poll loop + free
                    let init_id = *self.funciones.get(&nombre_init).unwrap();
                    let poll_id = *self.funciones.get(&nombre_poll).unwrap();
                    let init_ref = self.module.declare_func_in_func(init_id, builder.func);
                    let poll_ref = self.module.declare_func_in_func(poll_id, builder.func);

                    // Desempacar args del buffer y llamar __init(args...)
                    let mut args = Vec::new();
                    for i in 0..num_args {
                        let offset = (i * 8) as i32;
                        let arg_i64 = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), buffer_ptr, offset);
                        let target_type = hilo.arg_types.get(i).copied().unwrap_or(types::I32);
                        let arg_val = if target_type == types::I64 {
                            arg_i64
                        } else if target_type == types::I32 {
                            builder.ins().ireduce(types::I32, arg_i64)
                        } else if target_type == types::I8 {
                            builder.ins().ireduce(types::I8, arg_i64)
                        } else {
                            arg_i64
                        };
                        args.push(arg_val);
                    }

                    let init_call = builder.ins().call(init_ref, &args);
                    let fut_ptr = builder.inst_results(init_call)[0];

                    // Poll loop: while __poll(fut_ptr) == 0 { Sleep(1); }
                    let sleep_id = self.asegurar_funcion_c("Sleep", &[types::I32], None);
                    let sleep_ref = self.module.declare_func_in_func(sleep_id, builder.func);

                    let bloque_check = builder.create_block();
                    let bloque_sleep = builder.create_block();
                    let bloque_done = builder.create_block();

                    builder.ins().jump(bloque_check, &[]);

                    // Check: poll(fut_ptr) == 0?
                    builder.switch_to_block(bloque_check);
                    let poll_call = builder.ins().call(poll_ref, &[fut_ptr]);
                    let poll_result = builder.inst_results(poll_call)[0];
                    let cero64 = builder.ins().iconst(types::I64, 0);
                    let es_pending = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, poll_result, cero64);
                    builder.ins().brif(es_pending, bloque_sleep, &[], bloque_done, &[]);

                    // Sleep(1) + jump back
                    builder.switch_to_block(bloque_sleep);
                    let uno32 = builder.ins().iconst(types::I32, 1);
                    builder.ins().call(sleep_ref, &[uno32]);
                    builder.ins().jump(bloque_check, &[]);
                    builder.seal_block(bloque_sleep);

                    // Done: sellar check (2 predecesores: entry + sleep)
                    builder.seal_block(bloque_check);
                    builder.switch_to_block(bloque_done);
                    builder.seal_block(bloque_done);

                    // free(fut_ptr)
                    let free_id = self.asegurar_funcion_c("free", &[types::I64], None);
                    let free_ref = self.module.declare_func_in_func(free_id, builder.func);
                    builder.ins().call(free_ref, &[fut_ptr]);
                } else if let Some(target_id) = self.funciones.get(&nombre_target).copied() {
                    // Función normal: llamada directa
                    let target_ref = self.module.declare_func_in_func(target_id, builder.func);

                    let mut args = Vec::new();
                    for i in 0..num_args {
                        let offset = (i * 8) as i32;
                        let arg_i64 = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), buffer_ptr, offset);
                        let target_type = hilo.arg_types.get(i).copied().unwrap_or(types::I32);
                        let arg_val = if target_type == types::I64 {
                            arg_i64
                        } else if target_type == types::I32 {
                            builder.ins().ireduce(types::I32, arg_i64)
                        } else if target_type == types::I8 {
                            builder.ins().ireduce(types::I8, arg_i64)
                        } else {
                            arg_i64
                        };
                        args.push(arg_val);
                    }

                    builder.ins().call(target_ref, &args);
                }

                // free(buffer_ptr) — liberar el buffer de argumentos
                let free_id = self.asegurar_funcion_c("free", &[types::I64], None);
                let free_ref = self.module.declare_func_in_func(free_id, builder.func);
                builder.ins().call(free_ref, &[buffer_ptr]);

                // return 0
                let cero = builder.ins().iconst(types::I32, 0);
                builder.ins().return_(&[cero]);
            }

            if let Err(_) = self.module.define_function(func_id, &mut ctx) {
                // Error silencioso en MVP
            }
        }
    }

    /// imprimir_linea("El valor de {x} es {y}") → printf por segmento (sin variadic)
    fn builtin_imprimir_interpolado(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        texto: &str,
        con_newline: bool,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Parsear interpolación: dividir en segmentos literales y variables
        let mut segmentos: Vec<(bool, String)> = Vec::new(); // (es_variable, contenido)
        let mut literal_actual = String::new();
        let mut chars = texto.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if !literal_actual.is_empty() {
                    segmentos.push((false, literal_actual.clone()));
                    literal_actual.clear();
                }
                let mut nombre = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '}' { chars.next(); break; }
                    nombre.push(ch);
                    chars.next();
                }
                segmentos.push((true, nombre));
            } else {
                literal_actual.push(c);
            }
        }
        if !literal_actual.is_empty() {
            segmentos.push((false, literal_actual));
        }

        // Imprimir cada segmento
        for (es_var, contenido) in &segmentos {
            if *es_var {
                // Variable: imprimir según su tipo
                if let Some((slot, tipo, _)) = variables.get(contenido) {
                    let slot = *slot;
                    let tipo = tipo.clone();
                    let (fmt_str, val) = match tipo {
                        Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 => {
                            let v = builder.ins().stack_load(types::I32, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero64 => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%lld\0", v)
                        }
                        Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 => {
                            let v = builder.ins().stack_load(types::I32, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural64 => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%llu\0", v)
                        }
                        Tipo::Flotante32 | Tipo::Flotante64 => {
                            let v = builder.ins().stack_load(types::F64, slot, 0);
                            ("%f\0", v)
                        }
                        Tipo::Booleano => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        _ => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%s\0", v)
                        }
                    };
                    let fmt_ptr = self.crear_string_literal(builder, fmt_str);
                    let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[fmt_ptr, val]);
                }
            } else {
                // Literal: imprimir con printf("%s", literal)
                let mut bytes = contenido.as_bytes().to_vec();
                bytes.push(0);
                let ptr = self.crear_string_literal_bytes(builder, &bytes);
                let fmt_ptr = self.crear_string_literal(builder, "%s\0");
                let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                builder.ins().call(func_ref, &[fmt_ptr, ptr]);
            }
        }

        // Newline final si es imprimir_linea
        if con_newline {
            let nl_ptr = self.crear_string_literal(builder, "\n\0");
            let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
            let func_ref = self.module.declare_func_in_func(func_id, builder.func);
            let fmt_ptr = self.crear_string_literal(builder, "%s\0");
            builder.ins().call(func_ref, &[fmt_ptr, nl_ptr]);
        }

        Ok(builder.ins().iconst(types::I64, 0))
    }

    /// Crea un string global desde un &str (agrega \0 si no lo tiene)
    fn crear_string_literal(&mut self, builder: &mut FunctionBuilder, s: &str) -> cranelift_codegen::ir::Value {
        self.contador_strings += 1;
        let data_id = self.module.declare_data(
            &format!("str_lit_{}", self.contador_strings),
            Linkage::Local,
            false,
            false,
        ).unwrap();
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0); // null terminator para compatibilidad C
        let mut desc = cranelift_module::DataDescription::new();
        desc.define(bytes.into_boxed_slice());
        self.module.define_data(data_id, &desc).unwrap();
        let global = self.module.declare_data_in_func(data_id, builder.func);
        builder.ins().global_value(types::I64, global)
    }

    /// Crea un string global desde bytes raw (ya incluye \0 si necesario)
    fn crear_string_literal_bytes(&mut self, builder: &mut FunctionBuilder, bytes: &[u8]) -> cranelift_codegen::ir::Value {
        self.contador_strings += 1;
        let data_id = self.module.declare_data(
            &format!("str_bytes_{}", self.contador_strings),
            Linkage::Local,
            false,
            false,
        ).unwrap();
        let mut desc = cranelift_module::DataDescription::new();
        desc.define(bytes.to_vec().into_boxed_slice());
        self.module.define_data(data_id, &desc).unwrap();
        let global = self.module.declare_data_in_func(data_id, builder.func);
        builder.ins().global_value(types::I64, global)
    }

    /// tamaño_de::<T>() → constante comptime con el tamaño del tipo en bytes
    fn builtin_tamano_de(
        &mut self,
        builder: &mut FunctionBuilder,
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tamano: i64 = if let Some(tipo) = tipo_args.first() {
            self.tamano_tipo(tipo) as i64
        } else {
            0
        };
        Ok(builder.ins().iconst(types::I64, tamano))
    }

    fn builtin_texto_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        Ok(self.descriptor_nuevo(builder))
    }

    fn builtin_texto_desde(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let src = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.llamar_strlen(builder, src);
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(len, uno);

        let data = self.llamar_malloc(builder, cap);
        self.llamar_memcpy(builder, data, src, cap);

        let desc = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
        Ok(desc)
    }

    fn builtin_texto_agregar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let src = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len_t = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_CAP);
        let len_s = self.llamar_strlen(builder, src);
        let uno = builder.ins().iconst(types::I64, 1);
        let temp_len = builder.ins().iadd(len_t, len_s);
        let new_len = builder.ins().iadd(temp_len, uno);

        // Si no cabe, realloc
        let necesita_realloc = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
            cap,
            new_len,
        );

        let mut then_block = builder.create_block();
        let mut merge_block = builder.create_block();
        let data_var = self.nueva_variable();
        let cap_var = self.nueva_variable();
        builder.declare_var(data_var, types::I64);
        builder.declare_var(cap_var, types::I64);
        builder.def_var(data_var, data);
        builder.def_var(cap_var, cap);

        builder.ins().brif(necesita_realloc, then_block, &[], merge_block, &[]);

        // then: realloc
        builder.switch_to_block(then_block);
        let dos = builder.ins().iconst(types::I64, 2);
        let new_cap = builder.ins().imul(dos, new_len);
        let data_var_val = builder.use_var(data_var);
        let data_then = self.llamar_realloc(builder, data_var_val, new_cap);
        builder.def_var(data_var, data_then);
        builder.def_var(cap_var, new_cap);
        builder.ins().jump(merge_block, &[]);
        builder.seal_block(then_block);

        // merge
        builder.switch_to_block(merge_block);
        let data_final = builder.use_var(data_var);
        let cap_final = builder.use_var(cap_var);
        builder.seal_block(merge_block);

        let offset = builder.ins().iadd(data_final, len_t);
        let copy_len = builder.ins().iadd(len_s, uno);
        self.llamar_memcpy(builder, offset, src, copy_len);

        let nueva_longitud = builder.ins().iadd(len_t, len_s);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data_final);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, nueva_longitud);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap_final);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    fn builtin_texto_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    fn builtin_texto_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        self.llamar_free(builder, data);
        self.llamar_free(builder, desc);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// Fase 15C: texto_concatenar(a: Texto, b: Texto) -> Texto
    /// Crea un nuevo Texto con a + b (no modifica los originales).
    fn builtin_texto_concatenar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_b = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_PTR);
        let len_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_LEN);
        let ptr_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_PTR);
        let len_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_LEN);

        // new_len = len_a + len_b
        let new_len = builder.ins().iadd(len_a, len_b);
        // cap = new_len + 1 (null terminator)
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(new_len, uno);

        // malloc(cap)
        let data = self.llamar_malloc(builder, cap);

        // memcpy(data, ptr_a, len_a)
        self.llamar_memcpy(builder, data, ptr_a, len_a);

        // memcpy(data + len_a, ptr_b, len_b + 1) — incluye null terminator de b
        let dest_b = builder.ins().iadd(data, len_a);
        let copy_b_len = builder.ins().iadd(len_b, uno);
        self.llamar_memcpy(builder, dest_b, ptr_b, copy_b_len);

        // Crear descriptor
        let desc = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, new_len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
        Ok(desc)
    }

    /// Fase 15C: texto_subtexto(t: Texto, inicio: Entero32, fin: Entero32) -> Texto
    /// Extrae bytes [inicio, fin) como nuevo Texto.
    fn builtin_texto_subtexto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let inicio = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fin = self.compilar_expresion(&argumentos[2], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);

        // sub_len = fin - inicio (como i64)
        let inicio_64 = builder.ins().sextend(types::I64, inicio);
        let fin_64 = builder.ins().sextend(types::I64, fin);
        let sub_len = builder.ins().isub(fin_64, inicio_64);

        // cap = sub_len + 1
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(sub_len, uno);

        // malloc(cap)
        let data = self.llamar_malloc(builder, cap);

        // memcpy(data, ptr + inicio, sub_len)
        let src = builder.ins().iadd(ptr, inicio_64);
        self.llamar_memcpy(builder, data, src, sub_len);

        // data[sub_len] = 0 (null terminator)
        let null_pos = builder.ins().iadd(data, sub_len);
        let cero = builder.ins().iconst(types::I8, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, null_pos, 0);

        // Crear descriptor
        let desc_nuevo = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc_nuevo, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc_nuevo, Self::OFFSET_LEN, sub_len);
        self.guardar_campo_descriptor(builder, desc_nuevo, Self::OFFSET_CAP, cap);
        Ok(desc_nuevo)
    }

    /// Fase 15C: texto_comparar(a: Texto, b: Texto) -> Entero32
    /// Compara byte a byte. Retorna 0 si iguales, <0 si a<b, >0 si a>b.
    fn builtin_texto_comparar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_b = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_PTR);
        let len_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_LEN);
        let ptr_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_PTR);
        let len_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_LEN);

        // min_len = min(len_a, len_b)
        let a_menor = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            len_a, len_b,
        );
        let min_len = builder.ins().select(a_menor, len_a, len_b);

        // Loop: for i in 0..min_len { if a[i] != b[i] return a[i] - b[i] }
        let header = builder.create_block();
        let body = builder.create_block();
        let next_block = builder.create_block();
        let done = builder.create_block();

        let var_i = self.nueva_variable();
        builder.declare_var(var_i, types::I64);
        let cero = builder.ins().iconst(types::I64, 0);
        builder.def_var(var_i, cero);
        builder.ins().jump(header, &[]);

        // header: if i < min_len goto body else goto done
        builder.switch_to_block(header);
        let i = builder.use_var(var_i);
        let cond = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            i, min_len,
        );
        builder.ins().brif(cond, body, &[], done, &[]);
        // NO sellar header aquí — falta el back-edge desde next_block

        // body: comparar bytes
        builder.switch_to_block(body);
        let i_body = builder.use_var(var_i);
        let addr_a = builder.ins().iadd(ptr_a, i_body);
        let addr_b = builder.ins().iadd(ptr_b, i_body);
        let byte_a = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_a, 0);
        let byte_b = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_b, 0);
        let byte_a_32 = builder.ins().uextend(types::I32, byte_a);
        let byte_b_32 = builder.ins().uextend(types::I32, byte_b);
        let iguales = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            byte_a_32, byte_b_32,
        );
        // si iguales → next_block (i++), si no → done (bytes difieren)
        builder.ins().brif(iguales, next_block, &[], done, &[]);
        builder.seal_block(body);

        // next_block: i++ y volver al header
        builder.switch_to_block(next_block);
        let i_next = builder.use_var(var_i);
        let uno = builder.ins().iconst(types::I64, 1);
        let i_mas = builder.ins().iadd(i_next, uno);
        builder.def_var(var_i, i_mas);
        builder.ins().jump(header, &[]);
        builder.seal_block(next_block);

        // AHORA sí sellar header (back-edge completo)
        builder.seal_block(header);

        // done: determinar resultado
        builder.switch_to_block(done);
        let i_final = builder.use_var(var_i);
        let salio_early = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            i_final, min_len,
        );
        // Si early exit: return byte_a[i] - byte_b[i]
        let addr_a_f = builder.ins().iadd(ptr_a, i_final);
        let addr_b_f = builder.ins().iadd(ptr_b, i_final);
        let ba = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_a_f, 0);
        let bb = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_b_f, 0);
        let ba_32 = builder.ins().uextend(types::I32, ba);
        let bb_32 = builder.ins().uextend(types::I32, bb);
        let diff = builder.ins().isub(ba_32, bb_32);
        // Si no early: return len_a - len_b (como i32)
        let len_a_32 = builder.ins().ireduce(types::I32, len_a);
        let len_b_32 = builder.ins().ireduce(types::I32, len_b);
        let len_diff = builder.ins().isub(len_a_32, len_b_32);
        let resultado = builder.ins().select(salio_early, diff, len_diff);
        builder.seal_block(done);

        Ok(resultado)
    }

    /// Fase 15C: texto_obtener_byte(t: Texto, indice: Entero32) -> Entero8
    /// Retorna el byte en la posición dada.
    fn builtin_texto_obtener_byte(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let indice = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let idx_64 = builder.ins().sextend(types::I64, indice);
        let addr = builder.ins().iadd(ptr, idx_64);
        let byte = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr, 0);
        Ok(byte)
    }

    /// Fase 15D: archivo_leer(ruta: Palabra) -> Texto
    /// Lee un archivo completo. Retorna Texto vacío si no existe.
    /// Usa C runtime: fopen, fseek, ftell, fread, fclose.
    fn builtin_archivo_leer(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // fopen(ruta, "rb")
        let modo = self.crear_string_literal(builder, "rb");
        let fopen_id = self.asegurar_funcion_c("fopen", &[types::I64, types::I64], Some(types::I64));
        let fopen_ref = self.module.declare_func_in_func(fopen_id, builder.func);
        let call_fopen = builder.ins().call(fopen_ref, &[ruta, modo]);
        let file = builder.inst_results(call_fopen)[0];

        // if file == NULL → descriptor vacío, else leer contenido
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let es_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, file, cero_64);

        let bloque_nulo = builder.create_block();
        let bloque_ok = builder.create_block();
        let merge = builder.create_block();

        // Variable para el descriptor resultado
        let var_desc = self.nueva_variable();
        builder.declare_var(var_desc, types::I64);

        builder.ins().brif(es_nulo, bloque_nulo, &[], bloque_ok, &[]);

        // bloque_nulo: descriptor vacío
        builder.switch_to_block(bloque_nulo);
        let desc_vacio = self.descriptor_nuevo(builder);
        builder.def_var(var_desc, desc_vacio);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_nulo);

        // bloque_ok: leer archivo
        builder.switch_to_block(bloque_ok);

        // fseek(file, 0, SEEK_END)
        let seek_end = builder.ins().iconst(types::I32, 2);
        let cero_32 = builder.ins().iconst(types::I32, 0);
        let fseek_id = self.asegurar_funcion_c("fseek", &[types::I64, types::I32, types::I32], Some(types::I32));
        let fseek_ref = self.module.declare_func_in_func(fseek_id, builder.func);
        builder.ins().call(fseek_ref, &[file, cero_32, seek_end]);

        // ftell(file) → tamaño
        let ftell_id = self.asegurar_funcion_c("ftell", &[types::I64], Some(types::I64));
        let ftell_ref = self.module.declare_func_in_func(ftell_id, builder.func);
        let call_ftell = builder.ins().call(ftell_ref, &[file]);
        let tamano = builder.inst_results(call_ftell)[0];

        // fseek(file, 0, SEEK_SET)
        let seek_set = builder.ins().iconst(types::I32, 0);
        let cero_32b = builder.ins().iconst(types::I32, 0);
        builder.ins().call(fseek_ref, &[file, cero_32b, seek_set]);

        // malloc(tamano + 1)
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(tamano, uno);
        let data = self.llamar_malloc(builder, cap);

        // fread(data, 1, tamano, file)
        let fread_id = self.asegurar_funcion_c("fread", &[types::I64, types::I64, types::I64, types::I64], Some(types::I64));
        let fread_ref = self.module.declare_func_in_func(fread_id, builder.func);
        builder.ins().call(fread_ref, &[data, uno, tamano, file]);

        // data[tamano] = 0
        let null_pos = builder.ins().iadd(data, tamano);
        let cero_8 = builder.ins().iconst(types::I8, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero_8, null_pos, 0);

        // fclose(file)
        let fclose_id = self.asegurar_funcion_c("fclose", &[types::I64], Some(types::I32));
        let fclose_ref = self.module.declare_func_in_func(fclose_id, builder.func);
        builder.ins().call(fclose_ref, &[file]);

        // Crear descriptor Texto
        let desc_ok = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc_ok, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc_ok, Self::OFFSET_LEN, tamano);
        self.guardar_campo_descriptor(builder, desc_ok, Self::OFFSET_CAP, cap);
        builder.def_var(var_desc, desc_ok);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_ok);

        // merge
        builder.switch_to_block(merge);
        let resultado = builder.use_var(var_desc);
        builder.seal_block(merge);

        Ok(resultado)
    }

    /// Fase 15D: archivo_escribir(ruta: Palabra, contenido: Texto) -> Entero32
    /// Escribe contenido a archivo. Retorna 0 si OK, -1 si error.
    fn builtin_archivo_escribir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);

        // fopen(ruta, "wb")
        let modo = self.crear_string_literal(builder, "wb");
        let fopen_id = self.asegurar_funcion_c("fopen", &[types::I64, types::I64], Some(types::I64));
        let fopen_ref = self.module.declare_func_in_func(fopen_id, builder.func);
        let call_fopen = builder.ins().call(fopen_ref, &[ruta, modo]);
        let file = builder.inst_results(call_fopen)[0];

        // if file == NULL → retornar -1
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let es_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, file, cero_64);
        let menos_uno = builder.ins().iconst(types::I32, -1);
        let cero_32 = builder.ins().iconst(types::I32, 0);

        let bloque_error = builder.create_block();
        let bloque_ok = builder.create_block();
        let merge = builder.create_block();
        builder.ins().brif(es_nulo, bloque_error, &[], bloque_ok, &[]);

        // bloque_error: retornar -1
        builder.switch_to_block(bloque_error);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_error);

        // bloque_ok: fwrite(ptr, 1, len, file) + fclose
        builder.switch_to_block(bloque_ok);
        let uno = builder.ins().iconst(types::I64, 1);
        let fwrite_id = self.asegurar_funcion_c("fwrite", &[types::I64, types::I64, types::I64, types::I64], Some(types::I64));
        let fwrite_ref = self.module.declare_func_in_func(fwrite_id, builder.func);
        builder.ins().call(fwrite_ref, &[ptr, uno, len, file]);

        let fclose_id = self.asegurar_funcion_c("fclose", &[types::I64], Some(types::I32));
        let fclose_ref = self.module.declare_func_in_func(fclose_id, builder.func);
        builder.ins().call(fclose_ref, &[file]);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_ok);

        // merge: select(es_nulo, -1, 0)
        builder.switch_to_block(merge);
        let resultado = builder.ins().select(es_nulo, menos_uno, cero_32);
        builder.seal_block(merge);

        Ok(resultado)
    }

    /// Fase 15D: archivo_existe(ruta: Palabra) -> Booleano
    /// Verifica si un archivo existe. Retorna I8 (0 o 1).
    fn builtin_archivo_existe(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // fopen(ruta, "rb")
        let modo = self.crear_string_literal(builder, "rb");
        let fopen_id = self.asegurar_funcion_c("fopen", &[types::I64, types::I64], Some(types::I64));
        let fopen_ref = self.module.declare_func_in_func(fopen_id, builder.func);
        let call_fopen = builder.ins().call(fopen_ref, &[ruta, modo]);
        let file = builder.inst_results(call_fopen)[0];

        // if file != NULL → fclose + retornar 1, else retornar 0
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let no_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, file, cero_64);

        let bloque_existe = builder.create_block();
        let bloque_no = builder.create_block();
        let merge = builder.create_block();
        builder.ins().brif(no_nulo, bloque_existe, &[], bloque_no, &[]);

        // bloque_existe: fclose(file), resultado = 1
        builder.switch_to_block(bloque_existe);
        let fclose_id = self.asegurar_funcion_c("fclose", &[types::I64], Some(types::I32));
        let fclose_ref = self.module.declare_func_in_func(fclose_id, builder.func);
        builder.ins().call(fclose_ref, &[file]);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_existe);

        // bloque_no: resultado = 0
        builder.switch_to_block(bloque_no);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_no);

        // merge: select(no_nulo, 1, 0) como I8
        builder.switch_to_block(merge);
        let uno_8 = builder.ins().iconst(types::I8, 1);
        let cero_8 = builder.ins().iconst(types::I8, 0);
        let resultado = builder.ins().select(no_nulo, uno_8, cero_8);
        builder.seal_block(merge);

        Ok(resultado)
    }

    // ============================================================
    // Fase 15E: Matemáticas
    // ============================================================

    /// abs(x: Entero32) -> Entero32
    fn builtin_abs(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let cero = builder.ins().iconst(types::I32, 0);
        let es_neg = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedLessThan, x, cero);
        let neg = builder.ins().ineg(x);
        let resultado = builder.ins().select(es_neg, neg, x);
        Ok(resultado)
    }

    /// max(a: Entero32, b: Entero32) -> Entero32
    fn builtin_max(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let b = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let a_mayor = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan, a, b);
        let resultado = builder.ins().select(a_mayor, a, b);
        Ok(resultado)
    }

    /// min(a: Entero32, b: Entero32) -> Entero32
    fn builtin_min(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let b = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let a_menor = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedLessThan, a, b);
        let resultado = builder.ins().select(a_menor, a, b);
        Ok(resultado)
    }

    /// raiz(x: Flotante64) -> Flotante64 — C sqrt()
    fn builtin_raiz(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let sqrt_id = self.asegurar_funcion_c("sqrt", &[types::F64], Some(types::F64));
        let sqrt_ref = self.module.declare_func_in_func(sqrt_id, builder.func);
        let call = builder.ins().call(sqrt_ref, &[x]);
        let resultado = builder.inst_results(call)[0];
        Ok(resultado)
    }

    /// potencia(base: Flotante64, exp: Flotante64) -> Flotante64 — C pow()
    fn builtin_potencia(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let base = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let exp = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let pow_id = self.asegurar_funcion_c("pow", &[types::F64, types::F64], Some(types::F64));
        let pow_ref = self.module.declare_func_in_func(pow_id, builder.func);
        let call = builder.ins().call(pow_ref, &[base, exp]);
        let resultado = builder.inst_results(call)[0];
        Ok(resultado)
    }

    fn builtin_vector_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_nuevo requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let _tipo_t = &tipo_args[0];
        Ok(self.descriptor_nuevo(builder))
    }

    fn builtin_vector_agregar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_agregar requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let val = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_CAP);

        let necesita_realloc = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
            len,
            cap,
        );

        let mut then_block = builder.create_block();
        let mut merge_block = builder.create_block();
        let data_var = self.nueva_variable();
        let cap_var = self.nueva_variable();
        builder.declare_var(data_var, types::I64);
        builder.declare_var(cap_var, types::I64);
        builder.def_var(data_var, data);
        builder.def_var(cap_var, cap);

        builder.ins().brif(necesita_realloc, then_block, &[], merge_block, &[]);

        // then
        builder.switch_to_block(then_block);
        let cero = builder.ins().iconst(types::I64, 0);
        let cap_actual = builder.use_var(cap_var);
        let es_cero = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            cap_actual,
            cero,
        );
        let mut if_cero = builder.create_block();
        let mut if_no_cero = builder.create_block();
        let mut merge_cap = builder.create_block();
        builder.ins().brif(es_cero, if_cero, &[], if_no_cero, &[]);

        // cap == 0: alloc 4 elementos
        builder.switch_to_block(if_cero);
        let cuatro = builder.ins().iconst(types::I64, 4);
        let tam_inicial = builder.ins().imul_imm(cuatro, tamano_t);
        let data_cero = self.llamar_malloc(builder, tam_inicial);
        builder.def_var(data_var, data_cero);
        builder.def_var(cap_var, cuatro);
        builder.ins().jump(merge_cap, &[]);
        builder.seal_block(if_cero);

        // cap > 0: realloc 2*cap
        builder.switch_to_block(if_no_cero);
        let dos = builder.ins().iconst(types::I64, 2);
        let cap_previa = builder.use_var(cap_var);
        let new_cap = builder.ins().imul(dos, cap_previa);
        let new_size = builder.ins().imul_imm(new_cap, tamano_t);
        let data_previo = builder.use_var(data_var);
        let data_realloc = self.llamar_realloc(builder, data_previo, new_size);
        builder.def_var(data_var, data_realloc);
        builder.def_var(cap_var, new_cap);
        builder.ins().jump(merge_cap, &[]);
        builder.seal_block(if_no_cero);

        builder.switch_to_block(merge_cap);
        builder.seal_block(merge_cap);
        builder.ins().jump(merge_block, &[]);
        builder.seal_block(then_block);

        // merge
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);
        let data_final = builder.use_var(data_var);
        let cap_final = builder.use_var(cap_var);

        // Guardar valor en data + len * tamano_t
        let offset = builder.ins().imul_imm(len, tamano_t);
        let addr = builder.ins().iadd(data_final, offset);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, addr, 0);

        // len++
        let new_len = builder.ins().iadd_imm(len, 1);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data_final);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, new_len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap_final);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    fn builtin_vector_obtener(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_obtener requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let idx_i64 = if cranelift_t == types::I32 {
            builder.ins().sextend(types::I64, idx)
        } else {
            idx
        };

        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let offset = builder.ins().imul_imm(idx_i64, tamano_t);
        let addr = builder.ins().iadd(data, offset);
        Ok(builder.ins().load(cranelift_t, cranelift_codegen::ir::MemFlags::new(), addr, 0))
    }

    fn builtin_vector_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    fn builtin_vector_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        self.llamar_free(builder, data);
        self.llamar_free(builder, desc);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// Compila una llamada a función genérica con monomorfización
    fn compilar_llamada_generica(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let func_generica = match self.funciones_genericas.get(&llamada.funcion) {
            Some(f) => f.clone(),
            None => return Err(()),
        };

        // Inferir valores concretos de const generics y tipos de type generics
        let mut sust_consts: HashMap<String, String> = HashMap::new();
        let mut sust_tipos: HashMap<String, Tipo> = HashMap::new();
        let mut valores_clave: Vec<String> = Vec::new();
        for gen in &func_generica.parametros_genericos {
            if let Some(ref _tipo_const) = gen.tipo {
                // Const generic: buscar en parámetros de la función
                let valor = self.inferir_const_generico(
                    &func_generica.parametros,
                    &llamada.argumentos,
                    variables,
                    &gen.nombre,
                );
                match valor {
                    Some(v) => {
                        sust_consts.insert(gen.nombre.clone(), v.to_string());
                        valores_clave.push(v.to_string());
                    }
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Tipo,
                            60,
                            llamada.span.clone(),
                            format!("No se pudo inferir el valor del parámetro const '{}' en llamada a '{}'",
                                gen.nombre, llamada.funcion),
                        ));
                        return Err(());
                    }
                }
            } else {
                // Type generic: inferir del tipo de los argumentos
                let tipo_concreto = self.inferir_tipo_generico(
                    &func_generica.parametros,
                    &llamada.argumentos,
                    variables,
                    &gen.nombre,
                );
                match tipo_concreto {
                    Some(t) => {
                        sust_tipos.insert(gen.nombre.clone(), t.clone());
                        valores_clave.push(self.nombre_tipo_instancia(&t));
                    }
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Tipo,
                            62,
                            llamada.span.clone(),
                            format!("No se pudo inferir el tipo del parámetro genérico '{}' en llamada a '{}'",
                                gen.nombre, llamada.funcion),
                        ));
                        return Err(());
                    }
                }
            }
        }

        // Verificar si ya existe una instanciación
        let clave = (llamada.funcion.clone(), valores_clave.clone());
        let func_id = match self.instanciaciones.get(&clave).copied() {
            Some(id) => id,
            None => {
                // Crear función especializada
                let func_especializada = self.especializar_funcion(
                    &func_generica,
                    &sust_consts,
                    &sust_tipos,
                );
                
                // Declarar y compilar la función especializada
                self.declarar_funcion(&func_especializada);
                let id = match self.funciones.get(&func_especializada.nombre) {
                    Some(&id) => id,
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            61,
                            llamada.span.clone(),
                            format!("Error interno al declarar '{}'", func_especializada.nombre),
                        ));
                        return Err(());
                    }
                };
                self.instanciaciones.insert(clave.clone(), id);
                
                if let Err(_) = self.compilar_funcion(&func_especializada) {
                    // Error ya agregado
                }
                
                id
            }
        };

        let func_ref = self.module.declare_func_in_func(func_id, builder.func);

        let mut args = Vec::new();
        for arg in &llamada.argumentos {
            let val = self.compilar_expresion(arg, builder, variables)?;
            args.push(val);
        }

        let call = builder.ins().call(func_ref, &args);
        let result = builder.inst_results(call);
        
        if result.is_empty() {
            Ok(builder.ins().iconst(types::I32, 0))
        } else {
            Ok(result[0])
        }
    }

    /// Infiere el valor de un const genérico a partir de los tipos de los argumentos
    fn inferir_const_generico(
        &self,
        parametros: &Vec<Parametro>,
        argumentos: &Vec<Expresion>,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        nombre_generico: &str,
    ) -> Option<usize> {
        // Buscar en qué parámetro se usa el genérico
        for (param, arg) in parametros.iter().zip(argumentos.iter()) {
            if self.tipo_contiene_generico(&param.tipo, nombre_generico) {
                // Inferir del tipo del argumento
                let tipo_arg = self.inferir_tipo(arg, variables);
                
                if let Some(valor) = self.extraer_valor_generico(&tipo_arg, nombre_generico) {
                    return Some(valor);
                }
            }
        }
        None
    }

    /// Verifica si un tipo contiene una referencia a un genérico (type o const)
    fn tipo_contiene_generico(&self, tipo: &Tipo, nombre_generico: &str) -> bool {
        match tipo {
            Tipo::Generico(n) if n == nombre_generico => true,
            Tipo::ArrayGenerico(_, n) if n == nombre_generico => true,
            Tipo::ArrayGenerico(t, _) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Array(t, _) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Vector(t) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Puntero(t) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Referencia(t) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::NombreGenerico(_, args) => args.iter().any(|a| self.tipo_contiene_generico(a, nombre_generico)),
            _ => false,
        }
    }

    /// Extrae el valor concreto de un genérico de un tipo
    fn extraer_valor_generico(&self, tipo: &Tipo, nombre_generico: &str) -> Option<usize> {
        match tipo {
            Tipo::Array(_, n) => Some(*n),
            _ => None,
        }
    }

    /// Crea una función especializada reemplazando genéricos por valores concretos
    fn especializar_funcion(
        &mut self,
        func: &FuncionDecl,
        sust_consts: &HashMap<String, String>,
        sust_tipos: &HashMap<String, Tipo>,
    ) -> FuncionDecl {
        let mut func_clon = func.clone();
        
        // Generar nombre especializado: longitud_5 o es_igual_Entero32
        let partes_nombre: Vec<String> = func.parametros_genericos.iter().map(|gen| {
            if gen.tipo.is_some() {
                sust_consts.get(&gen.nombre).cloned().unwrap_or_else(|| gen.nombre.clone())
            } else {
                sust_tipos.get(&gen.nombre).map(|t| self.nombre_tipo_instancia(t)).unwrap_or_else(|| gen.nombre.clone())
            }
        }).collect();
        let nombre_especializado = format!("{}_{}", func.nombre, partes_nombre.join("_"));
        func_clon.nombre = nombre_especializado;
        func_clon.parametros_genericos.clear();
        
        // Aplicar sustituciones a parámetros
        for param in &mut func_clon.parametros {
            self.sustituir_tipo(&mut param.tipo, sust_consts, sust_tipos);
        }
        
        // Aplicar sustituciones a retorno
        if let Some(ref mut ret) = func_clon.retorno {
            self.sustituir_tipo(ret, sust_consts, sust_tipos);
        }
        
        // Aplicar sustituciones al cuerpo (sentencias y expresiones)
        for sentencia in &mut func_clon.cuerpo.sentencias {
            self.sustituir_en_sentencia(sentencia, sust_consts, sust_tipos);
        }
        
        func_clon
    }

    /// Sustituye const genéricos por literales en una sentencia
    fn sustituir_en_sentencia(
        &self,
        sentencia: &mut Sentencia,
        sust_consts: &HashMap<String, String>,
        sust_tipos: &HashMap<String, Tipo>,
    ) {
        match sentencia {
            Sentencia::Expresion(expr) => self.sustituir_en_expresion(expr, sust_consts),
            Sentencia::DeclaracionVariable(decl) => {
                if let Some(ref mut tipo) = decl.tipo {
                    self.sustituir_tipo(tipo, sust_consts, sust_tipos);
                }
                self.sustituir_en_expresion(&mut decl.valor, sust_consts);
            }
            Sentencia::Asignacion(asig) => {
                self.sustituir_en_expresion(&mut asig.valor, sust_consts);
            }
            Sentencia::Retornar(expr, _) => {
                if let Some(expr) = expr {
                    self.sustituir_en_expresion(expr, sust_consts);
                }
            }
            Sentencia::Condicional(cond) => {
                self.sustituir_en_expresion(&mut cond.condicion, sust_consts);
                for sent in &mut cond.bloque_entonces.sentencias {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
                if let Some(bloque_sino) = &mut cond.bloque_sino {
                    for sent in &mut bloque_sino.sentencias {
                        self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                    }
                }
            }
            Sentencia::BucleMientras(bucle) => {
                self.sustituir_en_expresion(&mut bucle.condicion, sust_consts);
                for sent in &mut bucle.bloque.sentencias {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
            Sentencia::BuclePara(bucle) => {
                self.sustituir_en_expresion(&mut bucle.iterable, sust_consts);
                for sent in &mut bucle.bloque.sentencias {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                for sent in cuerpo {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
            Sentencia::Seleccionar(seleccionar) => {
                for rama in &mut seleccionar.ramas {
                    for sent in &mut rama.cuerpo.sentencias {
                        self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                    }
                }
            }
            Sentencia::ConExecutor { hilos: _, cuerpo, span: _ } => {
                for sent in cuerpo {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
        }
    }

    /// Sustituye const genéricos por literales en una expresión
    fn sustituir_en_expresion(
        &self,
        expr: &mut Expresion,
        sustituciones: &HashMap<String, String>,
    ) {
        match expr {
            Expresion::Identificador(nombre, span) => {
                if let Some(valor) = sustituciones.get(nombre) {
                    if let Ok(n) = valor.parse::<i64>() {
                        *expr = Expresion::Literal(Literal::Entero(n, span.clone()));
                    }
                }
            }
            Expresion::Binaria(izq, _, der, _) => {
                self.sustituir_en_expresion(izq, sustituciones);
                self.sustituir_en_expresion(der, sustituciones);
            }
            Expresion::Unaria(_, expr, _) => {
                self.sustituir_en_expresion(expr, sustituciones);
            }
            Expresion::Llamada(llamada) => {
                for arg in &mut llamada.argumentos {
                    self.sustituir_en_expresion(arg, sustituciones);
                }
            }
            Expresion::AccesoArray(array, indice, _) => {
                self.sustituir_en_expresion(array, sustituciones);
                self.sustituir_en_expresion(indice, sustituciones);
            }
            Expresion::LiteralArray(elementos, _) => {
                for elem in elementos {
                    self.sustituir_en_expresion(elem, sustituciones);
                }
            }
            Expresion::ArrayRelleno(elem, _, _) => {
                self.sustituir_en_expresion(elem, sustituciones);
            }
            Expresion::InicializacionStruct(_, campos, _) => {
                for (_, val) in campos {
                    self.sustituir_en_expresion(val, sustituciones);
                }
            }
            Expresion::AccesoCampo(base, _, _) => {
                self.sustituir_en_expresion(base, sustituciones);
            }
            Expresion::ConstructorEnum(_, _, args, _) => {
                for arg in args {
                    self.sustituir_en_expresion(arg, sustituciones);
                }
            }
            Expresion::EsVariante(base, _, _, _, _) => {
                self.sustituir_en_expresion(base, sustituciones);
            }
            _ => {}
        }
    }

    /// Sustituye genéricos por valores concretos en un tipo
    fn sustituir_tipo(
        &self,
        tipo: &mut Tipo,
        sust_consts: &HashMap<String, String>,
        sust_tipos: &HashMap<String, Tipo>,
    ) {
        match tipo {
            Tipo::Generico(nombre) => {
                if let Some(concreto) = sust_tipos.get(nombre) {
                    *tipo = concreto.clone();
                }
            }
            Tipo::ArrayGenerico(t, nombre) => {
                if let Some(valor) = sust_consts.get(nombre) {
                    if let Ok(n) = valor.parse::<usize>() {
                        *tipo = Tipo::Array(Box::new((**t).clone()), n);
                        return;
                    }
                }
                self.sustituir_tipo(t, sust_consts, sust_tipos);
            }
            Tipo::Array(t, _) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::Vector(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::Puntero(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::Referencia(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaMut(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaConLifetime(_, t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaMutConLifetime(_, t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaSelf(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaMutSelf(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::NombreGenerico(_, args) => {
                for arg in args {
                    self.sustituir_tipo(arg, sust_consts, sust_tipos);
                }
            }
            _ => {}
        }
    }

        /// Infiere el tipo concreto de un parámetro type generic a partir de los argumentos
        fn inferir_tipo_generico(
            &self,
            parametros: &Vec<Parametro>,
            argumentos: &Vec<Expresion>,
            variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
            nombre_generico: &str,
        ) -> Option<Tipo> {
            let mut resultado: Option<Tipo> = None;
            for (param, arg) in parametros.iter().zip(argumentos.iter()) {
                if self.tipo_contiene_generico(&param.tipo, nombre_generico) {
                    let tipo_arg = self.inferir_tipo(arg, variables);
                    if let Some(tipo_inferido) = self.extraer_tipo_generico(&tipo_arg, nombre_generico, &param.tipo) {
                        if let Some(prev) = &resultado {
                            if prev != &tipo_inferido {
                                return None; // inconsistencia
                            }
                        } else {
                            resultado = Some(tipo_inferido);
                        }
                    }
                }
            }
            resultado
        }

        /// Extrae el tipo concreto correspondiente a un genérico dentro de un tipo argumento
        fn extraer_tipo_generico(&self, tipo_arg: &Tipo, nombre_generico: &str, param_tipo: &Tipo) -> Option<Tipo> {
            match param_tipo {
                Tipo::Generico(n) if n == nombre_generico => Some(tipo_arg.clone()),
                Tipo::ArrayGenerico(elem_param, _) | Tipo::Array(elem_param, _) => {
                    if let Tipo::Array(elem_arg, n) = tipo_arg {
                        self.extraer_tipo_generico(elem_arg, nombre_generico, elem_param)
                            .map(|t| Tipo::Array(Box::new(t), *n))
                    } else {
                        None
                    }
                }
                Tipo::Vector(elem_param) => {
                    if let Tipo::Vector(elem_arg) = tipo_arg {
                        self.extraer_tipo_generico(elem_arg, nombre_generico, elem_param)
                            .map(|t| Tipo::Vector(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::Puntero(p) => {
                    if let Tipo::Puntero(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::Puntero(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::Referencia(p) => {
                    if let Tipo::Referencia(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::Referencia(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::ReferenciaSelf(p) => {
                    if let Tipo::ReferenciaSelf(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::ReferenciaSelf(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::ReferenciaMutSelf(p) => {
                    if let Tipo::ReferenciaMutSelf(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::ReferenciaMutSelf(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::NombreGenerico(_, args_param) => {
                    if let Tipo::NombreGenerico(_, args_arg) = tipo_arg {
                        for (ap, aa) in args_param.iter().zip(args_arg.iter()) {
                            if let Some(t) = self.extraer_tipo_generico(aa, nombre_generico, ap) {
                                return Some(t);
                            }
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        /// Genera un nombre válido para una instancia de tipo concreto
        fn nombre_tipo_instancia(&self, tipo: &Tipo) -> String {
            match tipo {
                Tipo::Entero8 => "Entero8".to_string(),
                Tipo::Entero16 => "Entero16".to_string(),
                Tipo::Entero32 => "Entero32".to_string(),
                Tipo::Entero64 => "Entero64".to_string(),
                Tipo::Natural8 => "Natural8".to_string(),
                Tipo::Natural16 => "Natural16".to_string(),
                Tipo::Natural32 => "Natural32".to_string(),
                Tipo::Natural64 => "Natural64".to_string(),
                Tipo::Flotante32 => "Flotante32".to_string(),
                Tipo::Flotante64 => "Flotante64".to_string(),
                Tipo::Booleano => "Booleano".to_string(),
                Tipo::Caracter => "Caracter".to_string(),
                Tipo::Palabra => "Palabra".to_string(),
                Tipo::Texto => "Texto".to_string(),
                Tipo::Vacio => "Vacio".to_string(),
                Tipo::Nombre(n) => n.clone(),
                Tipo::Generico(n) => n.clone(),
                Tipo::Array(t, n) => format!("Array_{}_{}", self.nombre_tipo_instancia(t), n),
                Tipo::ArrayGenerico(t, n) => format!("Array_{}_{}", self.nombre_tipo_instancia(t), n),
                Tipo::Vector(t) => format!("Vector_{}", self.nombre_tipo_instancia(t)),
                Tipo::Resultado(t, e) => format!("Resultado_{}_{}", self.nombre_tipo_instancia(t), self.nombre_tipo_instancia(e)),
                Tipo::Puntero(t) => format!("Ptr_{}", self.nombre_tipo_instancia(t)),
                Tipo::Referencia(t) => format!("Ref_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaMut(t) => format!("RefMut_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaConLifetime(_, t) => format!("Ref_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaMutConLifetime(_, t) => format!("RefMut_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaSelf(t) => format!("RefSelf_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaMutSelf(t) => format!("RefMutSelf_{}", self.nombre_tipo_instancia(t)),
                Tipo::NombreGenerico(n, args) => {
                    let args_str = args.iter().map(|a| self.nombre_tipo_instancia(a)).collect::<Vec<_>>().join("_");
                    format!("{}_{}", n, args_str)
                }
            }
        }

    fn compilar_operacion_binaria(
        &mut self,
        op: OperadorBinario,
        izq: cranelift_codegen::ir::Value,
        der: cranelift_codegen::ir::Value,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        use cranelift_codegen::ir::condcodes::IntCC;
        
        let val = match op {
            OperadorBinario::Suma => builder.ins().iadd(izq, der),
            OperadorBinario::Resta => builder.ins().isub(izq, der),
            OperadorBinario::Multiplicacion => builder.ins().imul(izq, der),
            OperadorBinario::Division => builder.ins().sdiv(izq, der),
            OperadorBinario::Modulo => builder.ins().srem(izq, der),
            OperadorBinario::Igual => builder.ins().icmp(IntCC::Equal, izq, der),
            OperadorBinario::Distinto => builder.ins().icmp(IntCC::NotEqual, izq, der),
            OperadorBinario::Menor => builder.ins().icmp(IntCC::SignedLessThan, izq, der),
            OperadorBinario::Mayor => builder.ins().icmp(IntCC::SignedGreaterThan, izq, der),
            OperadorBinario::MenorIgual => builder.ins().icmp(IntCC::SignedLessThanOrEqual, izq, der),
            OperadorBinario::MayorIgual => builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, izq, der),
            OperadorBinario::Y => builder.ins().band(izq, der),
            OperadorBinario::O => builder.ins().bor(izq, der),
            // Bitwise
            OperadorBinario::BitAnd => builder.ins().band(izq, der),
            OperadorBinario::BitOr => builder.ins().bor(izq, der),
            OperadorBinario::BitXor => builder.ins().bxor(izq, der),
            OperadorBinario::ShiftLeft => builder.ins().ishl(izq, der),
            OperadorBinario::ShiftRight => builder.ins().sshr(izq, der),
            OperadorBinario::ShiftRightLogico => builder.ins().ushr(izq, der),
        };
        
        Ok(val)
    }
    
    fn compilar_operacion_unaria(
        &mut self,
        op: OperadorUnario,
        val: cranelift_codegen::ir::Value,
        builder: &mut FunctionBuilder,
        span: &Span,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let resultado = match op {
            OperadorUnario::Negacion => {
                // Negación aritmética: 0 - val
                let cero = builder.ins().iconst(types::I32, 0);
                builder.ins().isub(cero, val)
            }
            OperadorUnario::NegacionLogica => {
                // Negación booleana: val XOR 1
                let uno = builder.ins().iconst(types::I8, 1);
                builder.ins().bxor(val, uno)
            }
            OperadorUnario::BitNot => {
                // Bitwise NOT: NOT val
                builder.ins().bnot(val)
            }
            _ => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    8,
                    span.clone(),
                    "Operador unario no soportado".to_string(),
                ));
                return Err(());
            }
        };
        
        Ok(resultado)
    }

    fn tipo_a_cranelift(
        &self,
        tipo: &Tipo,
    ) -> cranelift_codegen::ir::Type {
        match tipo {
            Tipo::Entero8 |
            Tipo::Natural8 => types::I8,
            Tipo::Entero16 |
            Tipo::Natural16 => types::I16,
            Tipo::Entero32 |
            Tipo::Natural32 => types::I32,
            Tipo::Entero64 |
            Tipo::Natural64 => types::I64,
            Tipo::Flotante32 => types::F32,
            Tipo::Flotante64 => types::F64,
            Tipo::Booleano => types::I8,
            Tipo::Caracter => types::I8,
            Tipo::Palabra |
            Tipo::Texto |
            Tipo::Vector(_) |
            Tipo::Resultado(_, _) |
            Tipo::Puntero(_) |
            Tipo::Referencia(_) |
            Tipo::ReferenciaMut(_) |
            Tipo::ReferenciaConLifetime(_, _) |
            Tipo::ReferenciaMutConLifetime(_, _) |
            Tipo::ReferenciaSelf(_) |
            Tipo::ReferenciaMutSelf(_) |
            Tipo::Array(_, _) |
            Tipo::ArrayGenerico(_, _) => types::I64, // Puntero de 64 bits
            Tipo::Vacio => types::I32,
            Tipo::Nombre(_) => types::I32,
            Tipo::Generico(_) => types::I32, // Se resuelve en monomorfización
            Tipo::NombreGenerico(_, _) => types::I32, // Se resuelve en monomorfización
        }
    }

    fn tamano_tipo(
        &self,
        tipo: &Tipo,
    ) -> u32 {
        match tipo {
            Tipo::Entero8 |
            Tipo::Natural8 |
            Tipo::Booleano |
            Tipo::Caracter => 1,
            Tipo::Entero16 |
            Tipo::Natural16 => 2,
            Tipo::Entero32 |
            Tipo::Natural32 |
            Tipo::Flotante32 => 4,
            Tipo::Entero64 |
            Tipo::Natural64 |
            Tipo::Flotante64 |
            Tipo::Palabra |
            Tipo::Texto |
            Tipo::Vector(_) |
            Tipo::Resultado(_, _) |
            Tipo::Puntero(_) |
            Tipo::Referencia(_) |
            Tipo::ReferenciaMut(_) |
            Tipo::ReferenciaConLifetime(_, _) |
            Tipo::ReferenciaMutConLifetime(_, _) |
            Tipo::ReferenciaSelf(_) |
            Tipo::ReferenciaMutSelf(_) => 8,
            Tipo::Array(tipo_elem, longitud) => self.tamano_tipo(tipo_elem) * (*longitud as u32),
            Tipo::ArrayGenerico(tipo_elem, _) => {
                // En monomorfización, esto se reemplaza por Array con tamaño conocido
                // Por ahora, retornar tamaño del elemento como fallback
                self.tamano_tipo(tipo_elem)
            }
            Tipo::Vacio => 4,
            Tipo::Nombre(nombre) => {
                // Buscar en structs o enums
                if let Some(layout) = self.structs.get(nombre) {
                    layout.tamano
                } else if let Some(layout) = self.enums.get(nombre) {
                    layout.tamano
                } else {
                    4
                }
            }
            Tipo::Generico(_) => 4, // Se resuelve en monomorfización
            Tipo::NombreGenerico(_, _) => 4, // Se resuelve en monomorfización
        }
    }

    fn inferir_tipo(
        &self,
        expr: &Expresion,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Tipo {
        match expr {
            Expresion::Literal(lit) => {
                match lit {
                    Literal::Entero(_, _) => Tipo::Entero32,
                    Literal::Flotante(_, _) => Tipo::Flotante64,
                    Literal::Palabra(_, _) => Tipo::Palabra,
                    Literal::Caracter(_, _) => Tipo::Caracter,
                    Literal::Booleano(_, _) => Tipo::Booleano,
                }
            }
            Expresion::Identificador(nombre, _) => {
                variables.get(nombre)
                    .map(|(_, tipo, _)| tipo.clone())
                    .unwrap_or(Tipo::Entero32)
            }
            Expresion::AccesoArray(array, _, _) => {
                let tipo_array = self.inferir_tipo(array, variables);
                match tipo_array {
                    Tipo::Array(t, _) => *t,
                    _ => Tipo::Entero32,
                }
            }
            Expresion::Binaria(_, _, _, _) => Tipo::Entero32, // Simplificado
            Expresion::ConstructorEnum(enum_nombre, _, _, _) => {
                // Para enums genéricos como Resultado, necesitamos inferir los tipos
                if enum_nombre == "Resultado" {
                    // Por defecto, asumir Entero32 para ambos parámetros
                    Tipo::Resultado(Box::new(Tipo::Entero32), Box::new(Tipo::Entero32))
                } else {
                    Tipo::Nombre(enum_nombre.clone())
                }
            }
            _ => Tipo::Entero32,
        }
    }

    fn inferir_tipo_rango(&self, inicio: &Expresion, variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>) -> Tipo {
        self.inferir_tipo(inicio, variables)
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

    fn crear_modulo_dummy() -> ObjectModule {
        // Crear un módulo dummy temporal
        let mut flag_builder = cranelift_codegen::settings::builder();
        let _ = flag_builder.set("use_colocated_libcalls", "false");
        let _ = flag_builder.set("is_pic", "true");
        
        let isa_builder = cranelift_native::builder().unwrap();
        let isa = isa_builder.finish(
            cranelift_codegen::settings::Flags::new(flag_builder)
        ).unwrap();

        let mut builder = ObjectBuilder::new(
            isa,
            b"dummy".to_vec(),
            cranelift_module::default_libcall_names(),
        ).unwrap();

        ObjectModule::new(builder)
    }

    /// Aplana declaraciones recursivamente (desciende en módulos).
    /// Devuelve (prefijo_de_nombre, declaracion).
    /// Ej: un módulo "matematicas" con función "suma" → ("matematicas::", Funcion("suma"))
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
}

/// Implementación del trait BackendFalcato para el backend Cranelift.
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
