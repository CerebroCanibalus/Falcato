//! # Codegen Helpers — Capa de abstracción sobre Cranelift
//!
//! Este módulo provee helpers seguros y reutilizables para el codegen.
//! Elimina los patrones manuales propensos a errores:
//! - Double-seal de bloques
//! - Gestión manual de variables SSA
//! - Declaración repetida de funciones C
//! - Store/Load manual de memoria
//!
//! ## Uso
//!
//! ```rust,ignore
//! use crate::codegen_helpers::{BlockBuilder, VariableManager, CFunctionCache, MemoryHelper};
//! ```

use cranelift_codegen::ir::{self, types, InstBuilder, StackSlotData, StackSlotKind, Type, Value};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use std::collections::{HashMap, HashSet};

use crate::ast::{Articulo, Tipo};

// ============================================================
// BlockBuilder — Builder de bloques seguro
// ============================================================

/// Wrapper seguro sobre FunctionBuilder que previene double-seal
/// y simplifica la creación de cadenas if/else.
pub struct BlockBuilder<'a, 'b> {
    pub builder: &'a mut FunctionBuilder<'b>,
    blocks_selled: HashSet<ir::Block>,
}

impl<'a, 'b> BlockBuilder<'a, 'b> {
    /// Crea un nuevo BlockBuilder.
    pub fn new(builder: &'a mut FunctionBuilder<'b>) -> Self {
        Self {
            builder,
            blocks_selled: HashSet::new(),
        }
    }

    /// Crea un bloque, le añade parámetros, y hace switch_to_block.
    /// **NO** sella el bloque — el caller decide cuándo.
    pub fn crear_bloque(&mut self) -> ir::Block {
        let block = self.builder.create_block();
        self.builder.switch_to_block(block);
        block
    }

    /// Crea un bloque con parámetros tipados.
    pub fn crear_bloque_con_parametros(&mut self, params: &[Type]) -> ir::Block {
        let block = self.builder.create_block();
        for tipo in params {
            self.builder.append_block_param(block, *tipo);
        }
        self.builder.switch_to_block(block);
        block
    }

    /// Sella un bloque solo si no fue sellado antes.
    /// Previene el panic de double-seal.
    pub fn sellar_si_necesario(&mut self, block: ir::Block) {
        if self.blocks_selled.contains(&block) {
            return; // Ya sellado, no hacer nada
        }
        self.builder.seal_block(block);
        self.blocks_selled.insert(block);
    }

    /// Sella un bloque forzadamente (para casos donde sabemos que es seguro).
    pub fn sellar_forzado(&mut self, block: ir::Block) {
        if !self.blocks_selled.contains(&block) {
            self.builder.seal_block(block);
            self.blocks_selled.insert(block);
        }
    }

    /// Crea un bloque de entrada para una función con parámetros.
    pub fn crear_bloque_entrada(&mut self, num_params: usize) -> ir::Block {
        let entry = self.crear_bloque();
        for _ in 0..num_params {
            self.builder.append_block_param(entry, types::I64); // Default I64
        }
        entry
    }

    /// Crea un bloque de entrada con parámetros tipados.
    pub fn crear_bloque_entrada_tipado(&mut self, params: &[Type]) -> ir::Block {
        let entry = self.crear_bloque();
        for tipo in params {
            self.builder.append_block_param(entry, *tipo);
        }
        entry
    }

    /// Compila una cadena if/else con dispatch automático.
    /// Retorna el bloque de merge donde continúa la ejecución.
    ///
    /// # Ejemplo
    /// ```text
    /// condiciones = [
    ///     (cond1, then_block1),
    ///     (cond2, then_block2),
    /// ]
    /// default_block = else_block
    /// ```
    pub fn cadena_si_sino(
        &mut self,
        condiciones: &[(Value, ir::Block)],
        default_block: ir::Block,
    ) -> ir::Block {
        if condiciones.is_empty() {
            return default_block;
        }

        let mut prev_block = None;

        for (i, (cond, then_block)) in condiciones.iter().enumerate() {
            let next_check = if i < condiciones.len() - 1 {
                Some(self.crear_bloque())
            } else {
                None
            };

            let fallthrough = next_check.unwrap_or(default_block);

            // Si hay bloque anterior, hacer switch_to_block
            if let Some(prev) = prev_block {
                self.builder.switch_to_block(prev);
                self.sellar_si_necesario(prev);
            }

            self.builder.ins().brif(*cond, *then_block, &[], fallthrough, &[]);

            if let Some(next) = next_check {
                prev_block = Some(next);
            }
        }

        // Retornar el último check block o el default
        prev_block.unwrap_or(default_block)
    }

    /// Compila un switch statement sobre un valor entero.
    /// Retorna el bloque de merge.
    pub fn switch_entero(
        &mut self,
        valor: Value,
        casos: &[(i64, ir::Block)],
        default_block: ir::Block,
    ) -> ir::Block {
        // Para pocos casos, usar cadena if/else
        if casos.len() <= 4 {
            let conds: Vec<(Value, ir::Block)> = casos.iter()
                .map(|(caso, block)| {
                    let const_val = self.builder.ins().iconst(types::I64, *caso);
                    let eq = self.builder.ins().icmp(IntCC::Equal, valor, const_val);
                    (eq, *block)
                })
                .collect();
            return self.cadena_si_sino(&conds, default_block);
        }

        // Para muchos casos, usar br_table (futuro)
        // Por ahora, fallback a cadena if/else
        let conds: Vec<(Value, ir::Block)> = casos.iter()
            .map(|(caso, block)| {
                let const_val = self.builder.ins().iconst(types::I64, *caso);
                let eq = self.builder.ins().icmp(IntCC::Equal, valor, const_val);
                (eq, *block)
            })
            .collect();
        self.cadena_si_sino(&conds, default_block)
    }

    /// Retorna el builder interno para operaciones avanzadas.
    pub fn inner(&mut self) -> &mut FunctionBuilder<'b> {
        self.builder
    }
}

// ============================================================
// VariableManager — Gestión de variables SSA
// ============================================================

/// Información de una variable compilada.
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub slot: ir::StackSlot,
    pub tipo: Tipo,
    pub articulo: Articulo,
    pub tipo_cranelift: Type,
    pub offset: Option<u32>, // Para variables en struct (futuros)
}

/// Gestor de variables que reemplaza el HashMap<String, (StackSlot, Tipo, Articulo)>.
pub struct VariableManager {
    variables: HashMap<String, VarInfo>,
    contador: u32,
}

impl VariableManager {
    pub fn nuevo() -> Self {
        Self {
            variables: HashMap::new(),
            contador: 0,
        }
    }

    /// Declara una variable con tipo automático.
    pub fn declarar(
        &mut self,
        nombre: &str,
        tipo: Tipo,
        articulo: Articulo,
        tipo_cranelift: Type,
        builder: &mut FunctionBuilder,
    ) -> Variable {
        let var = Variable::from_u32(self.contador);
        self.contador += 1;

        let tamano = Self::tamano_tipo_static(&tipo);
        let slot = builder.create_sized_stack_slot(
            StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                tamano,
                0,
            )
        );

        self.variables.insert(nombre.to_string(), VarInfo {
            slot,
            tipo,
            articulo,
            tipo_cranelift,
            offset: None,
        });

        var
    }

    /// Declara una variable con stack slot existente.
    pub fn declarar_con_slot(
        &mut self,
        nombre: &str,
        tipo: Tipo,
        articulo: Articulo,
        tipo_cranelift: Type,
        slot: ir::StackSlot,
    ) -> Variable {
        let var = Variable::from_u32(self.contador);
        self.contador += 1;

        self.variables.insert(nombre.to_string(), VarInfo {
            slot,
            tipo,
            articulo,
            tipo_cranelift,
            offset: None,
        });

        var
    }

    /// Obtiene el valor de una variable desde su stack slot.
    pub fn cargar(
        &self,
        nombre: &str,
        builder: &mut FunctionBuilder,
    ) -> Value {
        let info = self.variables.get(nombre)
            .expect(&format!("Variable '{}' no declarada", nombre));
        builder.ins().stack_load(info.tipo_cranelift, info.slot, 0)
    }

    /// Guarda un valor en la variable (stack slot).
    pub fn guardar(
        &self,
        nombre: &str,
        valor: Value,
        builder: &mut FunctionBuilder,
    ) {
        let info = self.variables.get(nombre)
            .expect(&format!("Variable '{}' no declarada", nombre));
        builder.ins().stack_store(valor, info.slot, 0);
    }

    /// Obtiene la información de una variable.
    pub fn info(&self, nombre: &str) -> Option<&VarInfo> {
        self.variables.get(nombre)
    }

    /// Verifica si una variable existe.
    pub fn existe(&self, nombre: &str) -> bool {
        self.variables.contains_key(nombre)
    }

    /// Obtiene el slot de una variable.
    pub fn slot(&self, nombre: &str) -> Option<ir::StackSlot> {
        self.variables.get(nombre).map(|v| v.slot)
    }

    /// Obtiene el tipo de una variable.
    pub fn tipo(&self, nombre: &str) -> Option<&Tipo> {
        self.variables.get(nombre).map(|v| &v.tipo)
    }

    /// Obtiene el artículo de una variable.
    pub fn articulo(&self, nombre: &str) -> Option<Articulo> {
        self.variables.get(nombre).map(|v| v.articulo)
    }

    /// Tamaño de tipo en bytes (versión estática para uso sin self).
    fn tamano_tipo_static(tipo: &Tipo) -> u32 {
        match tipo {
            Tipo::Entero8 | Tipo::Natural8 | Tipo::Booleano | Tipo::Caracter => 1,
            Tipo::Entero16 | Tipo::Natural16 => 2,
            Tipo::Entero32 | Tipo::Natural32 | Tipo::Flotante32 => 4,
            Tipo::Entero64 | Tipo::Natural64 | Tipo::Flotante64 |
            Tipo::Palabra | Tipo::Texto | Tipo::Vector(_) |
            Tipo::Diccionario(_, _) | Tipo::Conjunto(_) |
            Tipo::Resultado(_, _) | Tipo::Puntero(_) |
            Tipo::Referencia(_) | Tipo::ReferenciaMut(_) => 8,
            Tipo::Array(tipo_elem, longitud) => Self::tamano_tipo_static(tipo_elem) * (*longitud as u32),
            _ => 8, // Default para punteros y tipos complejos
        }
    }
}

// ============================================================
// CFunctionCache — Cache de funciones C externas
// ============================================================

/// Cache de funciones C para no declararlas múltiples veces.
pub struct CFunctionCache {
    funciones: HashMap<String, FuncId>,
}

impl CFunctionCache {
    pub fn nuevo() -> Self {
        Self {
            funciones: HashMap::new(),
        }
    }

    /// Asegura que una función C esté declarada en el módulo.
    /// Si ya existe, retorna el FuncId cached.
    pub fn asegurar(
        &mut self,
        module: &mut ObjectModule,
        nombre: &str,
        params: &[Type],
        retorno: Option<Type>,
    ) -> FuncId {
        if let Some(&id) = self.funciones.get(nombre) {
            return id;
        }

        let mut sig = ir::Signature::new(CallConv::SystemV);
        for p in params {
            sig.params.push(ir::AbiParam::new(*p));
        }
        if let Some(r) = retorno {
            sig.returns.push(ir::AbiParam::new(r));
        }

        let id = module.declare_function(nombre, Linkage::Import, &sig)
            .unwrap_or_else(|e| panic!("Error declarando función C '{}': {}", nombre, e));

        self.funciones.insert(nombre.to_string(), id);
        id
    }

    /// Declara una función C con convención de llamada específica.
    pub fn asegurar_con_conv(
        &mut self,
        module: &mut ObjectModule,
        nombre: &str,
        params: &[Type],
        retorno: Option<Type>,
        call_conv: CallConv,
    ) -> FuncId {
        if let Some(&id) = self.funciones.get(nombre) {
            return id;
        }

        let mut sig = ir::Signature::new(call_conv);
        for p in params {
            sig.params.push(ir::AbiParam::new(*p));
        }
        if let Some(r) = retorno {
            sig.returns.push(ir::AbiParam::new(r));
        }

        let id = module.declare_function(nombre, Linkage::Import, &sig)
            .unwrap_or_else(|e| panic!("Error declarando función C '{}': {}", nombre, e));

        self.funciones.insert(nombre.to_string(), id);
        id
    }

    /// Verifica si una función ya está cacheada.
    pub fn existe(&self, nombre: &str) -> bool {
        self.funciones.contains_key(nombre)
    }

    /// Obtiene el FuncId de una función cacheada.
    pub fn obtener(&self, nombre: &str) -> Option<FuncId> {
        self.funciones.get(nombre).copied()
    }
}

// ============================================================
// MemoryHelper — Helpers de memoria
// ============================================================

/// Helpers para operaciones de memoria comunes.
/// Estos son métodos que se pueden mezclar con Codegen via trait o composición.
pub struct MemoryHelper;

impl MemoryHelper {
    /// Store genérico a dirección + offset.
    pub fn almacenar(
        builder: &mut FunctionBuilder,
        valor: Value,
        ptr: Value,
        offset: i32,
    ) {
        builder.ins().store(ir::MemFlags::new(), valor, ptr, offset);
    }

    /// Load genérico desde dirección + offset.
    pub fn cargar(
        builder: &mut FunctionBuilder,
        tipo: Type,
        ptr: Value,
        offset: i32,
    ) -> Value {
        builder.ins().load(tipo, ir::MemFlags::new(), ptr, offset)
    }

    /// Store a stack slot.
    pub fn almacenar_stack(
        builder: &mut FunctionBuilder,
        valor: Value,
        slot: ir::StackSlot,
        offset: i32,
    ) {
        builder.ins().stack_store(valor, slot, offset);
    }

    /// Load desde stack slot.
    pub fn cargar_stack(
        builder: &mut FunctionBuilder,
        tipo: Type,
        slot: ir::StackSlot,
        offset: i32,
    ) -> Value {
        builder.ins().stack_load(tipo, slot, offset)
    }

    /// Constante entera I64.
    pub fn const_i64(builder: &mut FunctionBuilder, val: i64) -> Value {
        builder.ins().iconst(types::I64, val)
    }

    /// Constante entera I32.
    pub fn const_i32(builder: &mut FunctionBuilder, val: i32) -> Value {
        builder.ins().iconst(types::I32, val as i64)
    }

    /// Constante entera I8.
    pub fn const_i8(builder: &mut FunctionBuilder, val: i8) -> Value {
        builder.ins().iconst(types::I8, val as i64)
    }

    /// Constante flotante F64.
    pub fn const_f64(builder: &mut FunctionBuilder, val: f64) -> Value {
        builder.ins().f64const(val)
    }

    /// Comparación entera.
    pub fn icmp(
        builder: &mut FunctionBuilder,
        cc: IntCC,
        a: Value,
        b: Value,
    ) -> Value {
        builder.ins().icmp(cc, a, b)
    }

    /// Suma entera.
    pub fn iadd(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().iadd(a, b)
    }

    /// Resta entera.
    pub fn isub(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().isub(a, b)
    }

    /// Multiplicación entera.
    pub fn imul(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().imul(a, b)
    }

    /// División entera.
    pub fn sdiv(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().sdiv(a, b)
    }

    /// AND bitwise.
    pub fn band(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().band(a, b)
    }

    /// OR bitwise.
    pub fn bor(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().bor(a, b)
    }

    /// XOR bitwise.
    pub fn bxor(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().bxor(a, b)
    }

    /// Shift left.
    pub fn ishl(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().ishl(a, b)
    }

    /// Shift right (arithmetic).
    pub fn sshr(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().sshr(a, b)
    }

    /// Shift right (logical).
    pub fn ushr(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().ushr(a, b)
    }

    /// NOT bitwise.
    pub fn bnot(builder: &mut FunctionBuilder, a: Value) -> Value {
        builder.ins().bnot(a)
    }

    /// Negación entera.
    pub fn ineg(builder: &mut FunctionBuilder, a: Value) -> Value {
        builder.ins().ineg(a)
    }

    /// Return con valores.
    pub fn return_(builder: &mut FunctionBuilder, vals: &[Value]) {
        builder.ins().return_(vals);
    }

    /// Branch incondicional.
    pub fn jump(builder: &mut FunctionBuilder, block: ir::Block, args: &[Value]) {
        builder.ins().jump(block, args);
    }

    /// Branch condicional.
    pub fn brif(
        builder: &mut FunctionBuilder,
        cond: Value,
        then_block: ir::Block,
        then_args: &[Value],
        else_block: ir::Block,
        else_args: &[Value],
    ) {
        builder.ins().brif(cond, then_block, then_args, else_block, else_args);
    }
}

// ============================================================
// Trait auxiliar para convertir Tipo → Type en contexts donde
// no tenemos acceso a Codegen.
// ============================================================

/// Convierte un Tipo de Falcato a un Type de Cranelift.
pub fn tipo_a_cranelift(tipo: &Tipo) -> Type {
    match tipo {
        Tipo::Entero8 | Tipo::Natural8 => types::I8,
        Tipo::Entero16 | Tipo::Natural16 => types::I16,
        Tipo::Entero32 | Tipo::Natural32 => types::I32,
        Tipo::Entero64 | Tipo::Natural64 => types::I64,
        Tipo::Flotante32 => types::F32,
        Tipo::Flotante64 => types::F64,
        Tipo::Booleano => types::I8,
        Tipo::Caracter => types::I8,
        Tipo::Palabra => types::I64,
        Tipo::Texto => types::I64,
        Tipo::Vacio => types::I8,
        Tipo::Puntero(_) => types::I64,
        Tipo::Referencia(_) => types::I64,
        Tipo::ReferenciaMut(_) => types::I64,
        Tipo::ReferenciaConLifetime(_, _) => types::I64,
        Tipo::ReferenciaMutConLifetime(_, _) => types::I64,
        Tipo::ReferenciaSelf(_) => types::I64,
        Tipo::ReferenciaMutSelf(_) => types::I64,
        Tipo::Array(_, _) => types::I64,
        Tipo::ArrayGenerico(_, _) => types::I64,
        Tipo::Vector(_) => types::I64,
        Tipo::Diccionario(_, _) => types::I64,
        Tipo::Conjunto(_) => types::I64,
        Tipo::Resultado(_, _) => types::I64,
        Tipo::Generico(n) => panic!("No se puede compilar tipo genérico '{}' sin concretar", n),
        Tipo::Nombre(n) => panic!("No se puede compilar tipo Nombre '{}' sin resolver", n),
        Tipo::NombreGenerico(n, _) => panic!("Tipo NombreGenerico '{}' no se pudo resolver", n),
    }
}

/// Retorna el tamaño en bytes de un Tipo de Falcato.
pub fn tamano_tipo(tipo: &Tipo) -> u32 {
    match tipo {
        Tipo::Entero8 | Tipo::Natural8 | Tipo::Booleano | Tipo::Caracter => 1,
        Tipo::Entero16 | Tipo::Natural16 => 2,
        Tipo::Entero32 | Tipo::Natural32 | Tipo::Flotante32 => 4,
        Tipo::Entero64 | Tipo::Natural64 | Tipo::Flotante64 |
        Tipo::Palabra | Tipo::Texto | Tipo::Vector(_) |
        Tipo::Diccionario(_, _) | Tipo::Conjunto(_) |
        Tipo::Resultado(_, _) | Tipo::Puntero(_) |
        Tipo::Referencia(_) | Tipo::ReferenciaMut(_) |
        Tipo::ReferenciaConLifetime(_, _) |
        Tipo::ReferenciaMutConLifetime(_, _) |
        Tipo::ReferenciaSelf(_) |
        Tipo::ReferenciaMutSelf(_) => 8,
        Tipo::Array(tipo_elem, longitud) => tamano_tipo(tipo_elem) * (*longitud as u32),
        Tipo::ArrayGenerico(tipo_elem, _) => tamano_tipo(tipo_elem),
        Tipo::Vacio => 4,
        Tipo::Nombre(_) | Tipo::Generico(_) | Tipo::NombreGenerico(_, _) => 4,
    }
}

// ============================================================
// Builder pattern para crear Codegen más limpio
// ============================================================

/// Helper para crear un Codegen paso a paso.
pub struct CodegenBuilder {
    nombre_modulo: String,
    cache: CFunctionCache,
}

impl CodegenBuilder {
    pub fn new(nombre_modulo: &str) -> Self {
        Self {
            nombre_modulo: nombre_modulo.to_string(),
            cache: CFunctionCache::nuevo(),
        }
    }

    /// Construye el Codegen final.
    /// Nota: Esto es un placeholder — la integración real con Codegen
    /// requiere acceso al módulo interno.
    pub fn build(self) -> CFunctionCache {
        self.cache
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tamano_tipo() {
        assert_eq!(tamano_tipo(&Tipo::Entero8), 1);
        assert_eq!(tamano_tipo(&Tipo::Entero32), 4);
        assert_eq!(tamano_tipo(&Tipo::Entero64), 8);
        assert_eq!(tamano_tipo(&Tipo::Booleano), 1);
        assert_eq!(tamano_tipo(&Tipo::Palabra), 8);
        assert_eq!(tamano_tipo(&Tipo::Texto), 8);
    }

    #[test]
    fn test_tipo_a_cranelift() {
        assert_eq!(tipo_a_cranelift(&Tipo::Entero8), types::I8);
        assert_eq!(tipo_a_cranelift(&Tipo::Entero32), types::I32);
        assert_eq!(tipo_a_cranelift(&Tipo::Entero64), types::I64);
        assert_eq!(tipo_a_cranelift(&Tipo::Flotante64), types::F64);
        assert_eq!(tipo_a_cranelift(&Tipo::Texto), types::I64);
    }

    #[test]
    fn test_variable_manager_nuevo() {
        let vm = VariableManager::nuevo();
        assert!(!vm.existe("x"));
    }
}
