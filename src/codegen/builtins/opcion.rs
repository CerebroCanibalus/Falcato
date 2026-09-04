//! # Builtins Opción/Resultado — helpers de pattern matching
//!
//! En Falcato, Option<T> y Resultado<T,E> son enums con variantes:
//! - Option: Nada (tag=0), Algo (tag=1, valor)
//! - Resultado: Error (tag=0, valor), Exito (tag=1, valor)
//!
//! Los helpers verifican el tag sin extraer el valor.

use crate::codegen::*;
use cranelift_codegen::ir::types;
use std::collections::HashMap;

impl Codegen {
    /// opcion_es_alguno(o: Option<T>) -> Booleano
    /// Retorna 1 si es Algo, 0 si es Nada.
    pub(crate) fn builtin_opcion_es_alguno(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Option se almacena como: tag (i32) + padding + valor
        // El tag está en el offset 0 del descriptor
        let tag = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::new(),
            desc,
            0,
        );

        // tag == 1 significa Algo
        let uno = builder.ins().iconst(types::I32, 1);
        let es_alguno = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            tag,
            uno,
        );

        // Convertir a I8 (Booleano en Falcato)
        let result = builder.ins().uextend(types::I8, es_alguno);
        Ok(result)
    }

    /// opcion_es_ninguno(o: Option<T>) -> Booleano
    /// Retorna 1 si es Nada, 0 si es Algo.
    pub(crate) fn builtin_opcion_es_ninguno(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;

        let tag = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::new(),
            desc,
            0,
        );

        // tag == 0 significa Nada
        let cero = builder.ins().iconst(types::I32, 0);
        let es_ninguno = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            tag,
            cero,
        );

        let result = builder.ins().uextend(types::I8, es_ninguno);
        Ok(result)
    }

    /// resultado_es_exito(r: Resultado<T, E>) -> Booleano
    /// Retorna 1 si es Exito, 0 si es Error.
    pub(crate) fn builtin_resultado_es_exito(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Resultado: tag (i32) en offset 0. Error=0, Exito=1.
        let tag = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::new(),
            desc,
            0,
        );

        let uno = builder.ins().iconst(types::I32, 1);
        let es_exito = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            tag,
            uno,
        );

        let result = builder.ins().uextend(types::I8, es_exito);
        Ok(result)
    }

    /// resultado_es_error(r: Resultado<T, E>) -> Booleano
    /// Retorna 1 si es Error, 0 si es Exito.
    pub(crate) fn builtin_resultado_es_error(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, crate::ast::Tipo, crate::ast::Articulo)>,
        argumentos: &[crate::ast::Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;

        let tag = builder.ins().load(
            types::I32,
            cranelift_codegen::ir::MemFlags::new(),
            desc,
            0,
        );

        let cero = builder.ins().iconst(types::I32, 0);
        let es_error = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            tag,
            cero,
        );

        let result = builder.ins().uextend(types::I8, es_error);
        Ok(result)
    }
}
