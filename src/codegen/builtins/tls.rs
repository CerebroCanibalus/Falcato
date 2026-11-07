use crate::codegen::*;

impl Codegen {
    // ============================================================
    // TLS/HTTPS (R7.8 FASE 5)
    // ============================================================

    /// tls_conectar(host: Texto, puerto: Entero32) -> Entero64 — conecta a servidor TLS
    pub(crate) fn builtin_tls_conectar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let host = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let puerto = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let puerto_i32 = if builder.func.dfg.value_type(puerto) == types::I64 {
            builder.ins().ireduce(types::I32, puerto)
        } else {
            puerto
        };
        let fn_id = self.asegurar_funcion_c("falcato_tls_conectar", &[types::I64, types::I32], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[host, puerto_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// tls_escribir(conn: Entero64, datos: Entero64, n: Entero32) -> Entero32
    pub(crate) fn builtin_tls_escribir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let conn = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let datos = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let n_i32 = if builder.func.dfg.value_type(n) == types::I64 {
            builder.ins().ireduce(types::I32, n)
        } else {
            n
        };
        let fn_id = self.asegurar_funcion_c("falcato_tls_escribir", &[types::I64, types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[conn, datos, n_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// tls_leer(conn: Entero64, buf: Entero64, n: Entero32) -> Entero32
    pub(crate) fn builtin_tls_leer(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let conn = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let buf = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let n_i32 = if builder.func.dfg.value_type(n) == types::I64 {
            builder.ins().ireduce(types::I32, n)
        } else {
            n
        };
        let fn_id = self.asegurar_funcion_c("falcato_tls_leer", &[types::I64, types::I64, types::I32], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[conn, buf, n_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// tls_datos_disponibles(conn: Entero64) -> Entero32
    pub(crate) fn builtin_tls_datos_disponibles(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let conn = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_tls_datos_disponibles", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[conn]);
        Ok(builder.inst_results(call)[0])
    }

    /// tls_cerrar(conn: Entero64)
    pub(crate) fn builtin_tls_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let conn = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_tls_cerrar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[conn]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

}
