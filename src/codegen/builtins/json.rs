//! # Builtins JSON — wrappers Cranelift para falcato_json_*

use crate::codegen::*;
use cranelift_codegen::ir::types;
use std::collections::HashMap;

impl Codegen {
    /// json_parsear(json: Texto) -> Texto
    /// Parsea JSON y retorna el valor parseado como texto.
    pub(crate) fn builtin_json_parsear(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_json = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        // falcato_json_parsear(desc_json: i64, desc_out: i64) -> i32
        let fn_id = self.asegurar_funcion_c(
            "falcato_json_parsear",
            &[types::I64, types::I64],
            Some(types::I32),
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_json, desc_out]);

        Ok(desc_out)
    }

    /// json_serializar(valor: Texto) -> Texto
    /// Serializa un valor JSON a texto.
    pub(crate) fn builtin_json_serializar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_valor = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        // falcato_json_serializar(desc_valor: i64, desc_out: i64) -> void
        let fn_id = self.asegurar_funcion_c(
            "falcato_json_serializar",
            &[types::I64, types::I64],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_valor, desc_out]);

        Ok(desc_out)
    }

    /// json_escapar(texto: Texto) -> Texto
    /// Escapa un string para JSON.
    pub(crate) fn builtin_json_escapar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let fn_id = self.asegurar_funcion_c(
            "falcato_json_escapar",
            &[types::I64, types::I64],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// json_obtener(json: Texto, clave: Texto) -> Texto
    /// Extrae un campo de un objeto JSON por nombre.
    pub(crate) fn builtin_json_obtener(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_json = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_clave = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let fn_id = self.asegurar_funcion_c(
            "falcato_json_obtener",
            &[types::I64, types::I64, types::I64],
            Some(types::I32),
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_json, desc_clave, desc_out]);

        Ok(desc_out)
    }
}
