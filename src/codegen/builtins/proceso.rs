use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_proceso_crear(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let comando = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_proceso_crear(comando: *const c_char) -> *mut c_void (i64)
        let fn_id = self.asegurar_funcion_c("falcato_proceso_crear", &[types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[comando]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_esperar(handle: Entero64) -> Entero32 (exit code)
    pub(crate) fn builtin_proceso_esperar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_proceso_esperar(handle: *mut c_void) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_proceso_esperar", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_leer_salida(handle: Entero64) -> Texto
    pub(crate) fn builtin_proceso_leer_salida(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // falcato_proceso_leer_salida(handle: *mut c_void) -> *mut c_char (i64)
        let fn_id = self.asegurar_funcion_c("falcato_proceso_leer_salida", &[types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle]);
        let ptr = builder.inst_results(call)[0];

        // Construir descriptor Texto desde el puntero C (strlen + malloc + memcpy)
        let len = self.llamar_strlen(builder, ptr);
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(len, uno);

        let data = self.llamar_malloc(builder, cap);
        self.llamar_memcpy(builder, data, ptr, cap);

        // Liberar el buffer temporal devuelto por el runtime (malloc'ed)
        self.llamar_free(builder, ptr);

        let desc = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
        Ok(desc)
    }

    /// proceso_cerrar(handle: Entero64) — libera el handle
    pub(crate) fn builtin_proceso_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_proceso_cerrar(handle: *mut c_void) -> void
        let fn_id = self.asegurar_funcion_c("falcato_proceso_cerrar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[handle]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    // ============================================================
    // Proceso bidireccional (pipes para MCP servers)
    // ============================================================

    /// proceso_crear_con_pipes(comando: Palabra) -> Entero64 (handle, 0 = error)
    pub(crate) fn builtin_proceso_crear_con_pipes(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let comando = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_proceso_crear_con_pipes(comando: *const c_char) -> *mut c_void (i64)
        let fn_id = self.asegurar_funcion_c("falcato_proceso_crear_con_pipes", &[types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[comando]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_escribir(handle: Entero64, datos: Palabra, n: Entero32) -> Entero32 (bytes escritos, -1 = error)
    pub(crate) fn builtin_proceso_escribir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let datos = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[2], builder, variables)?;
        // falcato_proceso_escribir(handle: *mut c_void, datos: *const u8, n: u32) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_proceso_escribir", &[types::I64, types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, datos, n]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_leer_salida_chunk(handle: Entero64, buf: Entero64, n: Entero32) -> Entero32 (bytes leídos, 0 = EOF)
    pub(crate) fn builtin_proceso_leer_salida_chunk(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let buf = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[2], builder, variables)?;
        // falcato_proceso_leer_salida_chunk(handle: *mut c_void, buf: *mut u8, n: u32) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_proceso_leer_salida_chunk", &[types::I64, types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, buf, n]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_leer_error_chunk(handle: Entero64, buf: Entero64, n: Entero32) -> Entero32 (bytes leídos, 0 = EOF)
    pub(crate) fn builtin_proceso_leer_error_chunk(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let buf = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[2], builder, variables)?;
        // falcato_proceso_leer_error_chunk(handle: *mut c_void, buf: *mut u8, n: u32) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_proceso_leer_error_chunk", &[types::I64, types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, buf, n]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_cerrar_entrada(handle: Entero64) — cierra stdin del proceso (envía EOF)
    pub(crate) fn builtin_proceso_cerrar_entrada(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_proceso_cerrar_entrada(handle: *mut c_void) -> void
        let fn_id = self.asegurar_funcion_c("falcato_proceso_cerrar_entrada", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[handle]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// proceso_listo_para_leer(handle: Entero64, ms: Entero32) -> Booleano (1 = hay datos, 0 = no)
    pub(crate) fn builtin_proceso_listo_para_leer(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let ms = self.compilar_expresion(&argumentos[1], builder, variables)?;
        // falcato_proceso_listo_para_leer(handle: *mut c_void, ms: u32) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_proceso_listo_para_leer", &[types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[handle, ms]);
        Ok(builder.inst_results(call)[0])
    }

    /// proceso_cerrar_bidireccional(handle: Entero64) — libera el handle bidireccional
    pub(crate) fn builtin_proceso_cerrar_bidireccional(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let handle = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_proceso_cerrar_bidireccional(handle: *mut c_void) -> void
        let fn_id = self.asegurar_funcion_c("falcato_proceso_cerrar_bidireccional", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[handle]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

}
