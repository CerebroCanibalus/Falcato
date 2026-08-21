use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_tamano_de(
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

    /// entorno_obtener(nombre: Texto) -> Texto — variable de entorno
    pub(crate) fn builtin_entorno_obtener(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let nombre = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_entorno_obtener", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[nombre, desc_out]);
        Ok(desc_out)
    }

    /// directorio_actual() -> Texto — cwd
    pub(crate) fn builtin_directorio_actual(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_directorio_actual", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_out]);
        Ok(desc_out)
    }

    /// aleatorio() -> Entero64 — número aleatorio
    pub(crate) fn builtin_aleatorio(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let fn_id = self.asegurar_funcion_c("falcato_aleatorio", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        Ok(builder.inst_results(call)[0])
    }

    // ============================================================
    // Terminal (R7.2): modo raw + lectura de teclas
    // ============================================================

    /// terminal_modo_raw(activo: Entero32) -> Entero32 (1 = OK, 0 = error)
    pub(crate) fn builtin_terminal_modo_raw(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let activo = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let activo_i32 = if builder.func.dfg.value_type(activo) == types::I64 {
            builder.ins().ireduce(types::I32, activo)
        } else {
            activo
        };
        // falcato_terminal_modo_raw(activo: i32) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_terminal_modo_raw", &[types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[activo_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// terminal_leer_tecla() -> Entero32 (código de tecla, ver terminal.rs)
    pub(crate) fn builtin_terminal_leer_tecla(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // falcato_terminal_leer_tecla() -> i32
        let fn_id = self.asegurar_funcion_c("falcato_terminal_leer_tecla", &[], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        Ok(builder.inst_results(call)[0])
    }

    /// terminal_dimensiones() -> Entero64 — empaquetado: ancho en low 32, alto en high 32
    pub(crate) fn builtin_terminal_dimensiones(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // falcato_terminal_dimensiones() -> i64
        let fn_id = self.asegurar_funcion_c("falcato_terminal_dimensiones", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        Ok(builder.inst_results(call)[0])
    }

    // ============================================================
    // Entrada estándar (R7.3)
    // ============================================================

    /// entrada_leer() -> Texto (TODO stdin hasta EOF)
    pub(crate) fn builtin_entrada_leer(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // falcato_entrada_leer() -> *mut c_char (i64)
        let fn_id = self.asegurar_funcion_c("falcato_entrada_leer", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        let ptr = builder.inst_results(call)[0];

        // Construir descriptor Texto desde el puntero C (strlen + malloc + memcpy)
        let len = self.llamar_strlen(builder, ptr);
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(len, uno);

        let data = self.llamar_malloc(builder, cap);
        self.llamar_memcpy(builder, data, ptr, cap);

        // Liberar el buffer temporal devuelto por el runtime
        self.llamar_free(builder, ptr);

        let desc = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
        Ok(desc)
    }

    // ============================================================
    // Argumentos de línea de comandos (R7.5)
    // ============================================================

    /// argumentos() -> Vector<Texto> — argv del binario (crudo, estilo C).
    /// El runtime construye el descriptor completo; acá solo se recibe.
    pub(crate) fn builtin_argumentos(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // falcato_argumentos() -> *mut descriptor Vector<Texto> (i64)
        let fn_id = self.asegurar_funcion_c("falcato_argumentos", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        let desc = builder.inst_results(call)[0];

        // Si el runtime devolvió NULL, devolver un Vector vacío válido.
        // Patrón: merge block con parámetro de bloque (como canal_intentar).
        let bloque_nulo = builder.create_block();
        let bloque_ok = builder.create_block();
        let bloque_fin = builder.create_block();
        builder.append_block_param(bloque_fin, types::I64);

        let cero = builder.ins().iconst(types::I64, 0);
        let es_nulo = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            desc,
            cero,
        );
        builder.ins().brif(es_nulo, bloque_nulo, &[], bloque_ok, &[]);

        // Nulo: crear Vector vacío (descriptor con ptr=0, len=0, cap=0)
        builder.switch_to_block(bloque_nulo);
        builder.seal_block(bloque_nulo);
        let vacio = self.descriptor_nuevo(builder);
        builder.ins().jump(bloque_fin, &[vacio]);

        // OK: usar el descriptor del runtime
        builder.switch_to_block(bloque_ok);
        builder.seal_block(bloque_ok);
        builder.ins().jump(bloque_fin, &[desc]);

        // Merge
        builder.switch_to_block(bloque_fin);
        builder.seal_block(bloque_fin);
        Ok(builder.block_params(bloque_fin)[0])
    }

    // ============================================================
    // Reloj de pared (R7.4)
    // ============================================================

    /// fecha_unix() -> Entero64 (segundos desde epoch)
    pub(crate) fn builtin_fecha_unix(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // falcato_fecha_unix() -> i64
        let fn_id = self.asegurar_funcion_c("falcato_fecha_unix", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        Ok(builder.inst_results(call)[0])
    }

    /// fecha_ms() -> Entero64 (ms desde epoch)
    pub(crate) fn builtin_fecha_ms(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // falcato_fecha_ms() -> i64
        let fn_id = self.asegurar_funcion_c("falcato_fecha_ms", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        Ok(builder.inst_results(call)[0])
    }

    // ============================================================
    // DHT distribuido (R8.2)
    // ============================================================

    /// dht_nuevo(puerto: Entero32) -> Entero64 (handle, 0 = error)
    pub(crate) fn builtin_dht_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let puerto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let puerto_i32 = if builder.func.dfg.value_type(puerto) == types::I64 {
            builder.ins().ireduce(types::I32, puerto)
        } else {
            puerto
        };
        // falcato_dht_nuevo(puerto: u16) -> *mut c_void (i64)
        let fn_id = self.asegurar_funcion_c("falcato_dht_nuevo", &[types::I32], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[puerto_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// dht_publicar(handle, clave: Palabra, valor: Palabra) -> Entero32
    pub(crate) fn builtin_dht_publicar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let clave = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let valor = self.compilar_expresion(&argumentos[2], builder, variables)?;

        let clave_len = self.llamar_strlen(builder, clave);
        let valor_len = self.llamar_strlen(builder, valor);

        // falcato_dht_publicar(handle, clave, clave_len, valor, valor_len) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_dht_publicar",
            &[types::I64, types::I64, types::I64, types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, clave, clave_len, valor, valor_len]);
        Ok(builder.inst_results(call)[0])
    }

    /// dht_consultar(handle, clave: Palabra) -> Entero64 (puntero, NULL = no encontrado)
    pub(crate) fn builtin_dht_consultar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let clave = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let clave_len = self.llamar_strlen(builder, clave);

        // falcato_dht_consultar(handle, clave, clave_len) -> *mut u8 (i64)
        let fn_id = self.asegurar_funcion_c("falcato_dht_consultar",
            &[types::I64, types::I64, types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, clave, clave_len]);
        Ok(builder.inst_results(call)[0])
    }

    /// dht_bootstrap(handle, direccion: Palabra, puerto: Entero32) -> Entero32
    pub(crate) fn builtin_dht_bootstrap(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let direccion = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let puerto = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let puerto_i32 = if builder.func.dfg.value_type(puerto) == types::I64 {
            builder.ins().ireduce(types::I32, puerto)
        } else {
            puerto
        };

        // falcato_dht_bootstrap(handle, direccion: *const i8, puerto: u16) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_dht_bootstrap",
            &[types::I64, types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, direccion, puerto_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// dht_cerrar(handle) — libera el nodo
    pub(crate) fn builtin_dht_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_dht_cerrar(handle: *mut c_void) -> void
        let fn_id = self.asegurar_funcion_c("falcato_dht_cerrar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[handle]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // ============================================================
    // Memoria debug — lente graduable (niveles 0-3)
    // ============================================================

    /// memoria_usada() -> Entero64 — bytes vivos en heap (nivel 1+)
    pub(crate) fn builtin_memoria_usada(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let fn_id = self.asegurar_funcion_c("falcato_memoria_usada", &[], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[]);
        Ok(builder.inst_results(call)[0])
    }

    /// memoria_volcar(ptr: Entero64, n: Entero32) — hexdump con ASCII
    pub(crate) fn builtin_memoria_volcar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n_i32 = if builder.func.dfg.value_type(n) == types::I64 {
            builder.ins().ireduce(types::I32, n)
        } else { n };
        let fn_id = self.asegurar_funcion_c("falcato_memoria_volcar", &[types::I64, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[ptr, n_i32]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// memoria_rastrear(ptr: Entero64) — ficha completa (alloc site, canario, timeline)
    pub(crate) fn builtin_memoria_rastrear(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_memoria_rastrear", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[ptr]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// memoria_canario_verificar(ptr: Entero64) -> Booleano (1=OK, 0=corrupto, 0 si nivel<2)
    pub(crate) fn builtin_memoria_canario_verificar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_memoria_canario_verificar", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[ptr]);
        let res_i32 = builder.inst_results(call)[0];
        // Convertir I32 -> I8 Booleano (0/1)
        let res_i8 = builder.ins().ireduce(types::I8, res_i32);
        Ok(res_i8)
    }

}
