//! # Builtins HTTP — wrappers Cranelift para falcato_http_get/post

use crate::codegen::*;
use cranelift_codegen::ir::types;
use std::collections::HashMap;

impl Codegen {
    /// http_get(host: Texto, puerto: Entero32, path: Texto) -> Texto
    /// GET HTTP básico. Retorna el body de la respuesta.
    pub(crate) fn builtin_http_get(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_host = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let puerto = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let desc_path = self.compilar_expresion(&argumentos[2], builder, variables)?;

        // Puerto: convertir I64 a I32 si es necesario
        let puerto_i32 = if builder.func.dfg.value_type(puerto) == types::I64 {
            builder.ins().ireduce(types::I32, puerto)
        } else {
            puerto
        };

        // Crear descriptor de salida para el body
        let desc_body_out = self.descriptor_nuevo(builder);

        // falcato_http_get(host: i64, puerto: i32, path: i64, body_out: i64) -> i32
        let fn_id = self.asegurar_funcion_c(
            "falcato_http_get",
            &[types::I64, types::I32, types::I64, types::I64],
            Some(types::I32),
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc_host, puerto_i32, desc_path, desc_body_out]);

        // Retornar el body (desc_body_out), no el status code
        // El status code queda en inst_results(call)[0] si se necesita
        Ok(desc_body_out)
    }

    /// http_post(host: Texto, puerto: Entero32, path: Texto, body: Texto) -> Texto
    /// POST HTTP básico. Retorna el body de la respuesta.
    pub(crate) fn builtin_http_post(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_host = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let puerto = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let desc_path = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let desc_body_req = self.compilar_expresion(&argumentos[3], builder, variables)?;

        let puerto_i32 = if builder.func.dfg.value_type(puerto) == types::I64 {
            builder.ins().ireduce(types::I32, puerto)
        } else {
            puerto
        };

        let desc_body_out = self.descriptor_nuevo(builder);

        // falcato_http_post(host, puerto, path, body_req, body_out) -> i32
        let fn_id = self.asegurar_funcion_c(
            "falcato_http_post",
            &[types::I64, types::I32, types::I64, types::I64, types::I64],
            Some(types::I32),
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_host, puerto_i32, desc_path, desc_body_req, desc_body_out]);

        Ok(desc_body_out)
    }
}
