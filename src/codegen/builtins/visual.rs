//! # Builtins Visual — wrappers Cranelift para ventanas, lienzo, imagen, sonido

use crate::codegen::*;
use cranelift_codegen::ir::types;
use std::collections::HashMap;

impl Codegen {
    // === Ventana ===

    pub(crate) fn builtin_ventana_nueva(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_titulo = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let ancho = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let alto = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let ancho_i32 = builder.ins().ireduce(types::I32, ancho);
        let alto_i32 = builder.ins().ireduce(types::I32, alto);
        let fn_id = self.asegurar_funcion_c("falcato_ventana_nueva", &[types::I64, types::I32, types::I32], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc_titulo, ancho_i32, alto_i32]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_ventana_mostrar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_ventana_mostrar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[hwnd]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_ventana_cerrar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_ventana_cerrar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[hwnd]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_ventana_bucle_mensajes(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_ventana_bucle_mensajes", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[hwnd]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_ventana_titulo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_ventana_titulo", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[hwnd, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_ventana_establecer_titulo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_titulo = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_ventana_establecer_titulo", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[hwnd, desc_titulo]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_ventana_posicion(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_ventana_posicion", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[hwnd, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_ventana_tamano(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let hwnd = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_ventana_tamano", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[hwnd, desc_out]);
        Ok(desc_out)
    }

    // === Lienzo ===

    pub(crate) fn builtin_lienzo_nuevo(
        &mut self, builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ancho = self.compilar_expresion(&argumentos[0], builder, _variables)?;
        let alto = self.compilar_expresion(&argumentos[1], builder, _variables)?;
        let ancho_i32 = builder.ins().ireduce(types::I32, ancho);
        let alto_i32 = builder.ins().ireduce(types::I32, alto);
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_nuevo", &[types::I32, types::I32], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[ancho_i32, alto_i32]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_lienzo_limpiar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_lienzo = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let color = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let color_i32 = builder.ins().ireduce(types::I32, color);
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_limpiar", &[types::I64, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_lienzo, color_i32]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_lienzo_linea(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let v3 = self.compilar_expresion(&argumentos[3], builder, variables)?;
        let v4 = self.compilar_expresion(&argumentos[4], builder, variables)?;
        let x1 = builder.ins().ireduce(types::I32, v1);
        let y1 = builder.ins().ireduce(types::I32, v2);
        let x2 = builder.ins().ireduce(types::I32, v3);
        let y2 = builder.ins().ireduce(types::I32, v4);
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_linea", &[types::I64, types::I32, types::I32, types::I32, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, x1, y1, x2, y2]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_lienzo_rectangulo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let v3 = self.compilar_expresion(&argumentos[3], builder, variables)?;
        let v4 = self.compilar_expresion(&argumentos[4], builder, variables)?;
        let x = builder.ins().ireduce(types::I32, v1);
        let y = builder.ins().ireduce(types::I32, v2);
        let w = builder.ins().ireduce(types::I32, v3);
        let h = builder.ins().ireduce(types::I32, v4);
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_rectangulo", &[types::I64, types::I32, types::I32, types::I32, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, x, y, w, h]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_lienzo_circulo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let v3 = self.compilar_expresion(&argumentos[3], builder, variables)?;
        let cx = builder.ins().ireduce(types::I32, v1);
        let cy = builder.ins().ireduce(types::I32, v2);
        let r = builder.ins().ireduce(types::I32, v3);
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_circulo", &[types::I64, types::I32, types::I32, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, cx, cy, r]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_lienzo_texto(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let desc_texto = self.compilar_expresion(&argumentos[3], builder, variables)?;
        let x = builder.ins().ireduce(types::I32, v1);
        let y = builder.ins().ireduce(types::I32, v2);
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_texto", &[types::I64, types::I32, types::I32, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, x, y, desc_texto]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_lienzo_guardar_png(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_ruta = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_guardar_png", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc, desc_ruta]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_lienzo_liberar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_lienzo_liberar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // === Imagen ===

    pub(crate) fn builtin_imagen_desde_archivo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_imagen_desde_archivo", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_ruta, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_imagen_ancho(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_imagen_ancho", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_imagen_alto(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_imagen_alto", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_imagen_redimensionar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let w = builder.ins().ireduce(types::I32, v1);
        let h = builder.ins().ireduce(types::I32, v2);
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_imagen_redimensionar", &[types::I64, types::I32, types::I32, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, w, h, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_imagen_guardar_png(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_ruta = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_imagen_guardar_png", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc, desc_ruta]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_imagen_liberar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_imagen_liberar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // === Sonido ===

    pub(crate) fn builtin_audio_nuevo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let v1 = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let canales = builder.ins().ireduce(types::I32, v1);
        let freq = builder.ins().ireduce(types::I32, v2);
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_audio_nuevo", &[types::I32, types::I32, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[canales, freq, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_audio_desde_archivo(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_audio_desde_archivo", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_ruta, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_audio_tono(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let freq = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let v2 = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let v3 = self.compilar_expresion(&argumentos[3], builder, variables)?;
        let dur = builder.ins().ireduce(types::I32, v1);
        let canales = builder.ins().ireduce(types::I32, v2);
        let freq_m = builder.ins().ireduce(types::I32, v3);
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_audio_tono", &[types::F64, types::I32, types::I32, types::I32, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[freq, dur, canales, freq_m, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_audio_mezclar(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let b = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_audio_mezclar", &[types::I64, types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[a, b, desc_out]);
        Ok(desc_out)
    }

    pub(crate) fn builtin_audio_fade_in(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let dur = builder.ins().ireduce(types::I32, v1);
        let fn_id = self.asegurar_funcion_c("falcato_audio_fade_in", &[types::I64, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, dur]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_audio_fade_out(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let v1 = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let dur = builder.ins().ireduce(types::I32, v1);
        let fn_id = self.asegurar_funcion_c("falcato_audio_fade_out", &[types::I64, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc, dur]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_audio_guardar_wav(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_ruta = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_audio_guardar_wav", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc, desc_ruta]);
        Ok(builder.inst_results(call)[0])
    }

    pub(crate) fn builtin_audio_reproducir(
        &mut self, builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_audio_reproducir", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc]);
        Ok(builder.inst_results(call)[0])
    }
}
