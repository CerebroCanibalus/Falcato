//! Builtins — todas las funciones built-in del lenguaje
//! (I/O, TCP, canales, texto, vector, diccionario, conjunto, archivo, math, string)

use super::*;

impl Codegen {
    pub(crate) fn builtin_imprimir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        con_newline: bool,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Verificar si hay interpolaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n: literal con {variable}
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
                // Enteros: printf %d ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â en Windows x64, args variÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡dicos se pasan como I64
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                // R7.6: %d solo lee 32 bits — Entero64/Natural64 necesitan %lld
                // (antes: el valor se truncaba al imprimir, p.ej. 3000000000 → -1294967296)
                let fmt = match tipo_arg {
                    Tipo::Entero64 | Tipo::Natural64 => {
                        if con_newline { "%lld\n" } else { "%lld" }
                    }
                    _ => {
                        if con_newline { "%d\n" } else { "%d" }
                    }
                };
                let fmt_ptr = self.crear_string_literal(builder, fmt);
                // Extender a I64 para passing variÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡dico correcto en Windows x64
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
                // Floats: printf %.17g (round-trip exacto — antes %f truncaba a 6 decimales:
                // 0.1+0.2 imprimía 0.300000 aunque el valor real es 0.30000000000000004)
                // Windows x64 variadic ABI: doubles se pasan como bit pattern en reg entero
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let fmt = if con_newline { "%.17g\n" } else { "%.17g" };
                let fmt_ptr = self.crear_string_literal(builder, fmt);
                // Bitcast F64 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ I64 para passing variÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡dico correcto
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

    /// afirmar(condiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â aborta si la condiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n es falsa
    pub(crate) fn builtin_afirmar(
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

        // Si condiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n es falsa ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ imprimir error y terminar
        let bloque_fallo = builder.create_block();
        let bloque_ok = builder.create_block();

        // Extender condiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n a I32 para comparaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n segura
        let cond_i32 = builder.ins().uextend(types::I32, cond);
        let cero = builder.ins().iconst(types::I32, 0);
        let es_falso = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, cond_i32, cero);
        builder.ins().brif(es_falso, bloque_fallo, &[], bloque_ok, &[]);

        // Bloque fallo: puts("  FALLO: afirmaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n fallida") + ExitProcess(1)
        builder.switch_to_block(bloque_fallo);
        builder.seal_block(bloque_fallo);

        let msg = self.crear_string_literal(builder, "  FALLO: afirmaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n fallida");
        let puts_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
        let puts_ref = self.module.declare_func_in_func(puts_id, builder.func);
        builder.ins().call(puts_ref, &[msg]);

        let uno = builder.ins().iconst(types::I32, 1);
        self.platform_call_void("exit_process", builder, &[uno]);
        builder.ins().trap(cranelift_codegen::ir::TrapCode::UnreachableCodeReached);

        // Bloque OK: continuar
        builder.switch_to_block(bloque_ok);
        builder.seal_block(bloque_ok);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// dormir(ms) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â MVP: llama a Sleep(ms) de kernel32 (bloquea el thread)
    /// TODO Fase 18B: integrar con reactor IOCP para suspensiÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n real de tarea
    pub(crate) fn builtin_dormir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if argumentos.is_empty() {
            return Ok(builder.ins().iconst(types::I32, 0));
        }

        let ms_val = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Sleep(DWORD ms) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â kernel32.dll, Windows x64 fastcall
        // DWORD es u32. Si el valor ya es I32, usar directo; si es I64, truncar.
        let ms_i32 = if builder.func.dfg.value_type(ms_val) == types::I64 {
            builder.ins().ireduce(types::I32, ms_val)
        } else {
            ms_val
        };

        self.platform_call_void("sleep", builder, &[ms_i32]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    // === TCP Builtins (Fase 18B) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Winsock2 directo ===

    /// tcp_vincular(puerto) -> Entero64 (socket handle)
    /// Crea socket TCP, bind a 0.0.0.0:puerto, listen(128)
    pub(crate) fn builtin_tcp_vincular(
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

        // Inicializar networking via PlatformRuntime (WSAStartup en Win, no-op en Linux)
        {
            let mut ctx = self.ctx();
            let runtime = platform::current_runtime();
            runtime.net_init(&mut ctx, builder);
        }

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

        // sin_port = htons(puerto) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â byte swap manual en little-endian
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
    pub(crate) fn builtin_tcp_aceptar(
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

    /// tcp_leer(socket, buffer_ptr, tam) -> Entero32 (bytes leÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­dos)
    pub(crate) fn builtin_tcp_leer(
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
    pub(crate) fn builtin_tcp_escribir(
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
    pub(crate) fn builtin_tcp_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let sock_val = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // net_close(sock) via PlatformRuntime (closesocket en Win, close en POSIX)
        let mut ctx = self.ctx();
        let runtime = platform::current_runtime();
        runtime.net_close(&mut ctx, builder, sock_val);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    // === Canal Builtins (Fase 18C) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Mutex + Semaphore + Ring Buffer ===
    // Layout del canal en heap (malloc):
    //   offset 0:  HANDLE mutex (8 bytes)
    //   offset 8:  HANDLE semaphore (8 bytes)
    //   offset 16: i32 head
    //   offset 20: i32 tail
    //   offset 24: i32 count
    //   offset 28: i32 capacity
    //   offset 32: buffer[capacity] (i32 * capacity)

    /// canal_nuevo(capacidad) -> Entero64 (puntero al canal)
    pub(crate) fn builtin_canal_nuevo(
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
        let elem_size_i64 = builder.ins().iconst(types::I64, 4);
        let cap_i64 = builder.ins().uextend(types::I64, cap_i32);

        // falcato_channel_new(capacity: i32, elem_size: i32) -> *mut c_void (i64)
        let fn_id = self.asegurar_funcion_c("falcato_channel_new", &[types::I64, types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[cap_i64, elem_size_i64]);
        Ok(builder.inst_results(call)[0])
    }

    /// canal_enviar(canal, valor) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â WaitForSingleObject(mutex), write ring buffer, ReleaseMutex, ReleaseSemaphore
    pub(crate) fn builtin_canal_enviar(
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

        // Asignar buffer temporal con malloc para pasar el valor por puntero
        let four_i64 = builder.ins().iconst(types::I64, 4);
        let data_ptr = self.llamar_malloc(builder, four_i64);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), valor_i32, data_ptr, 0);

        // falcato_channel_send(ch: *mut c_void, data: *const c_void) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_channel_send", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call_inst = builder.ins().call(fn_ref, &[canal_ptr, data_ptr]);
        let _ret_code = builder.inst_results(call_inst)[0];

        self.llamar_free(builder, data_ptr);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// canal_recibir(canal) -> Entero32 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â WaitForSingleObject(semaphore), lock mutex, read ring buffer, unlock
    pub(crate) fn builtin_canal_recibir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Asignar buffer temporal con malloc para recibir datos por puntero
        let four_i64 = builder.ins().iconst(types::I64, 4);
        let data_ptr = self.llamar_malloc(builder, four_i64);

        // falcato_channel_recv(ch: *mut c_void, data: *mut c_void) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_channel_recv", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call_inst = builder.ins().call(fn_ref, &[canal_ptr, data_ptr]);
        let _ret_code = builder.inst_results(call_inst)[0];

        // Cargar el valor recibido del buffer
        let valor = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), data_ptr, 0);

        // Liberar buffer temporal
        self.llamar_free(builder, data_ptr);

        Ok(valor)
    }

    /// canal_cerrar(canal) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â CloseHandle(mutex), CloseHandle(semaphore), free(canal)
    pub(crate) fn builtin_canal_cerrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // falcato_channel_close(ch: *mut c_void) -> void
        let fn_id = self.asegurar_funcion_c("falcato_channel_close", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[canal_ptr]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// cancelar() ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â cancela el executor activo (structured cancellation)
    /// Setea cancelled=1 en el pool y despierta todos los workers
    pub(crate) fn builtin_cancelar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if let Some(ref pool_var) = self.executor_pool_var {
            if let Some(&(pool_slot, _, _)) = variables.get(pool_var) {
                let pool_ptr = builder.ins().stack_load(types::I64, pool_slot, 0);

                // falcato_executor_cancel(exec)
                let fn_id = self.asegurar_funcion_c("falcato_executor_cancel", &[types::I64], None);
                let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
                builder.ins().call(fn_ref, &[pool_ptr]);
            }
        }

        Ok(builder.ins().iconst(types::I64, 0))
    }

    /// canal_intentar(canal) -> Entero32 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â non-blocking try_recv
    /// WaitForSingleObject(semaphore, 0): si hay dato lo retorna, si no retorna i32::MIN
    pub(crate) fn builtin_canal_intentar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let canal_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Asignar buffer temporal con malloc
        let four_i64 = builder.ins().iconst(types::I64, 4);
        let buf = self.llamar_malloc(builder, four_i64);

        // falcato_channel_try_recv(canal, buf) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_channel_try_recv", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[canal_ptr, buf]);
        let ret_code = builder.inst_results(call)[0];

        // Si ret_code == 0: cargar, luego free. Si no: free, sentinel.
        let bloque_hay = builder.create_block();
        let bloque_vacio = builder.create_block();
        let bloque_fin = builder.create_block();
        builder.append_block_param(bloque_fin, types::I32);

        let cero = builder.ins().iconst(types::I32, 0);
        let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, ret_code, cero);
        builder.ins().brif(cmp, bloque_hay, &[], bloque_vacio, &[]);

        // Vacio
        builder.switch_to_block(bloque_vacio);
        builder.seal_block(bloque_vacio);
        self.llamar_free(builder, buf);
        let sentinel = builder.ins().iconst(types::I32, -2147483648i64);
        builder.ins().jump(bloque_fin, &[sentinel]);

        // Hay dato
        builder.switch_to_block(bloque_hay);
        builder.seal_block(bloque_hay);
        let valor = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), buf, 0);
        self.llamar_free(builder, buf);
        builder.ins().jump(bloque_fin, &[valor]);

        // Merge
        builder.switch_to_block(bloque_fin);
        builder.seal_block(bloque_fin);
        Ok(builder.block_params(bloque_fin)[0])
    }

    /// lanzar f(args...) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â MVP: crea un thread real del OS con CreateThread
    /// Genera un wrapper __hilo_N que CreateThread puede llamar (firma: fn(i64) -> i32)
    pub(crate) fn builtin_imprimir_interpolado(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        texto: &str,
        con_newline: bool,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Parsear interpolaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n: dividir en segmentos literales y variables
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
                // Variable (o acceso a campo args.nombre): imprimir según su tipo
                if contenido.contains('.') {
                    // Acceso a campo: compilar la expresión AccesoCampo real.
                    // Soporta un nivel: args.nombre (base.campo).
                    let partes: Vec<&str> = contenido.split('.').collect();
                    if partes.len() == 2 {
                        let base = partes[0].to_string();
                        let campo = partes[1].to_string();
                        let expr_campo = Expresion::AccesoCampo(
                            Box::new(Expresion::Identificador(base.clone(), Span::vacio())),
                            campo.clone(),
                            Span::vacio(),
                        );
                        // Compilar la expresión completa → devuelve el valor del campo cargado
                        let val = self.compilar_expresion(&expr_campo, builder, variables)?;
                        let tipo = self.inferir_tipo(&expr_campo, variables);
                        self.imprimir_valor_interpolado(builder, variables, val, &tipo, contenido)?;
                    }
                } else if let Some((slot, tipo, _)) = variables.get(contenido) {
                    let slot = *slot;
                    let tipo = self.resolver_alias(tipo);
                    let (fmt_str, val) = match tipo {
                        Tipo::Entero8 => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero16 => {
                            let v = builder.ins().stack_load(types::I16, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero32 => {
                            let v = builder.ins().stack_load(types::I32, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero64 => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%lld\0", v)
                        }
                        Tipo::Natural8 => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural16 => {
                            let v = builder.ins().stack_load(types::I16, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural32 => {
                            let v = builder.ins().stack_load(types::I32, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural64 => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%llu\0", v)
                        }
                        Tipo::Flotante32 => {
                            // Cargar F32 y promover a F64 (el slot es de 4 bytes)
                            let v32 = builder.ins().stack_load(types::F32, slot, 0);
                            let v = builder.ins().fpromote(types::F64, v32);
                            // %.17g (round-trip exacto) espera un double; la firma Cranelift usa I64,
                            // así que pasamos los bits del F64 como I64 (bitcast).
                            let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), v);
                            ("%.17g\0", bits)
                        }
                        Tipo::Flotante64 => {
                            let v = builder.ins().stack_load(types::F64, slot, 0);
                            // %.17g (round-trip exacto); la firma Cranelift usa I64,
                            // así que pasamos los bits del F64 como I64 (bitcast).
                            let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), v);
                            ("%.17g\0", bits)
                        }
                        Tipo::Booleano => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Caracter => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%c\0", ext)
                        }
                        Tipo::Texto => {
                            // Texto = puntero a descriptor {ptr, len, cap}.
                            // Cargar descriptor y luego el ptr de datos (offset 0).
                            let desc = builder.ins().stack_load(types::I64, slot, 0);
                            let v = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), desc, Self::OFFSET_PTR);
                            ("%s\0", v)
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

    /// R7.5 Fase 2: imprime un valor ya cargado (resultado de compilar una
    /// expresión como AccesoCampo) según su tipo, usando printf.
    pub(crate) fn imprimir_valor_interpolado(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        val: cranelift_codegen::ir::Value,
        tipo: &Tipo,
        _contenido: &str,
    ) -> Result<(), ()> {
        let tipo = self.resolver_alias(tipo);
        let (fmt_str, val_out) = match tipo {
            Tipo::Entero8 | Tipo::Caracter => {
                let ext = builder.ins().sextend(types::I64, val);
                ("%d\0", ext)
            }
            Tipo::Entero16 => {
                let ext = builder.ins().sextend(types::I64, val);
                ("%d\0", ext)
            }
            Tipo::Entero32 => {
                let ext = builder.ins().sextend(types::I64, val);
                ("%d\0", ext)
            }
            Tipo::Entero64 => ("%lld\0", val),
            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Booleano => {
                let ext = builder.ins().uextend(types::I64, val);
                ("%u\0", ext)
            }
            Tipo::Natural64 => ("%llu\0", val),
            Tipo::Flotante32 => {
                let f = builder.ins().fpromote(types::F64, val);
                let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), f);
                ("%f\0", bits)
            }
            Tipo::Flotante64 => {
                let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), val);
                ("%f\0", bits)
            }
            Tipo::Texto => {
                // val es puntero al descriptor {ptr, len, cap} → extraer ptr de datos
                let v = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), val, Self::OFFSET_PTR);
                ("%s\0", v)
            }
            _ => ("%s\0", val),
        };
        let fmt_ptr = self.crear_string_literal(builder, fmt_str);
        let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        builder.ins().call(func_ref, &[fmt_ptr, val_out]);
        Ok(())
    }

    /// Crea un string global desde un &str (agrega \0 si no lo tiene).
    /// INTERNADO (R7.6): mismo contenido → mismo DataId → mismo puntero.
    /// Requisito para Diccionario<Palabra, V>: comparación de claves por puntero.
    pub(crate) fn crear_string_literal(&mut self, builder: &mut FunctionBuilder, s: &str) -> cranelift_codegen::ir::Value {
        if let Some(data_id) = self.strings_internados.get(s) {
            let global = self.module.declare_data_in_func(*data_id, builder.func);
            return builder.ins().global_value(types::I64, global);
        }
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
        self.strings_internados.insert(s.to_string(), data_id);
        let global = self.module.declare_data_in_func(data_id, builder.func);
        builder.ins().global_value(types::I64, global)
    }

    /// Crea un string global desde bytes raw (ya incluye \0 si necesario)
    pub(crate) fn crear_string_literal_bytes(&mut self, builder: &mut FunctionBuilder, bytes: &[u8]) -> cranelift_codegen::ir::Value {
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

    /// tamaÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â±o_de::<T>() ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ constante comptime con el tamaÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â±o del tipo en bytes
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

    pub(crate) fn builtin_texto_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Texto vacío REAL: descriptor con ptr a un buffer de 1 byte ('\0'),
        // len=0, cap=1. printf("%s") con NULL imprimiría "(null)".
        let desc = self.descriptor_nuevo(builder);
        let uno = builder.ins().iconst(types::I64, 1);
        let data = self.llamar_malloc(builder, uno);
        let cero_byte = builder.ins().iconst(types::I8, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero_byte, data, 0);
        let flags = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags, data, desc, Self::OFFSET_PTR);
        builder.ins().store(flags, uno, desc, Self::OFFSET_CAP);
        Ok(desc)
    }

    pub(crate) fn builtin_texto_desde(
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

    pub(crate) fn builtin_texto_agregar(
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

        let then_block = builder.create_block();
        let merge_block = builder.create_block();
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

    pub(crate) fn builtin_texto_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    pub(crate) fn builtin_texto_liberar(
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
    pub(crate) fn builtin_texto_concatenar(
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

        // memcpy(data + len_a, ptr_b, len_b + 1) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â incluye null terminator de b
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
    pub(crate) fn builtin_texto_subtexto(
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
    pub(crate) fn builtin_texto_comparar(
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
        // NO sellar header aquÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­ ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â falta el back-edge desde next_block

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
        // si iguales ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ next_block (i++), si no ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ done (bytes difieren)
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

        // AHORA sÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­ sellar header (back-edge completo)
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
    /// Retorna el byte en la posiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n dada.
    pub(crate) fn builtin_texto_obtener_byte(
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

    /// Fase GUI-1: texto_a_puntero(texto: Palabra) -> Entero64
    /// Retorna la direcciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n de memoria de un literal de cadena.
    /// ÃƒÆ’Ã†â€™Ãƒâ€¦Ã‚Â¡til para pasar punteros a string en structs FFI (ej: WNDCLASSEXA).
    pub(crate) fn builtin_texto_a_puntero(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        Ok(ptr)
    }

    /// Fase GUI-1: como_entero64(valor: Entero32) -> Entero64
    /// Extiende Entero32 a Entero64 con signo. Para pasar NULL (0) como puntero en FFI.
    /// R9.0.3: también acepta Flotante64 (trunca con fcvt_to_sint).
    pub(crate) fn builtin_como_entero64(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_conversion_numerica(argumentos, Tipo::Entero64, builder, variables)
    }

    /// R7.5 Fase 2: como_entero32(valor: Entero64) -> Entero32
    /// Trunca Entero64 a Entero32 (para parseo tipado de args).
    pub(crate) fn builtin_como_entero32(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_conversion_numerica(argumentos, Tipo::Entero32, builder, variables)
    }

    /// R9.0.3 — Conversión numérica completa.
    /// `builtin_conversion_numerica(argumentos, destino, ...)` convierte el primer argumento
    /// al tipo destino, manejando todas las combinaciones:
    /// - entero → entero: sextend/ireduce según ancho
    /// - flotante → entero: fcvt_to_sint (trunca hacia cero, como cast en C)
    /// - entero → flotante: fcvt_from_sint
    /// - flotante → flotante: fpromote/fdemote
    /// VITAL para WAV 16/24-bit (muestras Flotante64 → Entero16/32) y DSP.
    pub(crate) fn builtin_conversion_numerica(
        &mut self,
        argumentos: &Vec<Expresion>,
        destino: Tipo,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {        let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let tipo_fuente = self.resolver_alias(&self.inferir_tipo(&argumentos[0], variables));
        let ir_destino = self.tipo_a_cranelift(&destino);

        let es_fuente_flotante = matches!(tipo_fuente, Tipo::Flotante32 | Tipo::Flotante64);
        let es_destino_flotante = matches!(destino, Tipo::Flotante32 | Tipo::Flotante64);

        match (es_fuente_flotante, es_destino_flotante) {
            // flotante → entero: truncar hacia cero (fcvt_to_sint)
            (true, false) => Ok(builder.ins().fcvt_to_sint(ir_destino, val)),
            // entero → flotante
            (false, true) => Ok(builder.ins().fcvt_from_sint(ir_destino, val)),
            // flotante → flotante: promover o degradar
            (true, true) => {
                if ir_destino == types::F64 && self.tipo_a_cranelift(&tipo_fuente) == types::F32 {
                    Ok(builder.ins().fpromote(types::F64, val))
                } else if ir_destino == types::F32 && self.tipo_a_cranelift(&tipo_fuente) == types::F64 {
                    Ok(builder.ins().fdemote(types::F32, val))
                } else {
                    Ok(val)
                }
            }
            // entero → entero: extender o truncar
            (false, false) => {
                let ir_fuente = self.tipo_a_cranelift(&tipo_fuente);
                let ancho_fuente = ir_fuente.bits();
                let ancho_destino = ir_destino.bits();
                if ancho_destino > ancho_fuente {
                    Ok(builder.ins().sextend(ir_destino, val))
                } else if ancho_destino < ancho_fuente {
                    Ok(builder.ins().ireduce(ir_destino, val))
                } else {
                    Ok(val)
                }
            }
        }
    }

    /// R7.5 Fase 2: conversión de Texto a número (texto_a_entero/natural/flotante/booleano).
    /// Extrae (ptr, len) del descriptor Texto y llama a la función C del runtime
    /// `falcato_texto_a_*` que parsea respetando `len` (sin asumir null terminator).
    fn builtin_texto_convertir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        nombre_c: &str,
        retorno: cranelift_codegen::ir::types::Type,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let func_id = self.asegurar_funcion_c(nombre_c, &[types::I64, types::I64], Some(retorno));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let inst = builder.ins().call(func_ref, &[ptr, len]);
        Ok(builder.func.dfg.first_result(inst))
    }

    /// R7.5 Fase 2: texto_a_entero(t: Texto) -> Entero64
    pub(crate) fn builtin_texto_a_entero(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_entero", types::I64)
    }

    /// R7.5 Fase 2: texto_a_natural(t: Texto) -> Entero64 (-1 si no es número)
    pub(crate) fn builtin_texto_a_natural(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_natural", types::I64)
    }

    /// R7.5 Fase 2: texto_a_flotante(t: Texto) -> Flotante64
    pub(crate) fn builtin_texto_a_flotante(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_flotante", types::F64)
    }

    /// R7.5 Fase 2: texto_a_booleano(t: Texto) -> Booleano (1/0)
    pub(crate) fn builtin_texto_a_booleano(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_booleano", types::I64)
    }

    /// Fase 15D: archivo_leer(ruta: Palabra) -> Texto
    /// Lee un archivo completo. Retorna Texto vacÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­o si no existe.
    /// Usa C runtime: fopen, fseek, ftell, fread, fclose.
    pub(crate) fn builtin_archivo_leer(
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

        // if file == NULL ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ descriptor vacÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­o, else leer contenido
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let es_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, file, cero_64);

        let bloque_nulo = builder.create_block();
        let bloque_ok = builder.create_block();
        let merge = builder.create_block();

        // Variable para el descriptor resultado
        let var_desc = self.nueva_variable();
        builder.declare_var(var_desc, types::I64);

        builder.ins().brif(es_nulo, bloque_nulo, &[], bloque_ok, &[]);

        // bloque_nulo: descriptor vacÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­o
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

        // ftell(file) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ tamaÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â±o
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
    pub(crate) fn builtin_archivo_escribir(
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

        // if file == NULL ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ retornar -1
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
    pub(crate) fn builtin_archivo_existe(
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

        // if file != NULL ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ fclose + retornar 1, else retornar 0
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
    // Fase 15E: MatemÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡ticas
    // ============================================================

    /// abs(x: Entero32) -> Entero32
    pub(crate) fn builtin_abs(
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
    pub(crate) fn builtin_max(
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
    pub(crate) fn builtin_min(
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

    /// raiz(x: Flotante64) -> Flotante64 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â C sqrt()
    pub(crate) fn builtin_raiz(
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

    /// potencia(base: Flotante64, exp: Flotante64) -> Flotante64 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â C pow()
    pub(crate) fn builtin_potencia(
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

    pub(crate) fn builtin_vector_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_nuevo requiere un tipo genÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©rico".to_string(),
            ));
            return Err(());
        }
        let _tipo_t = &tipo_args[0];
        Ok(self.descriptor_nuevo(builder))
    }

    pub(crate) fn builtin_vector_agregar(
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
                "vector_agregar requiere un tipo genÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©rico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let _cranelift_t = self.tipo_a_cranelift(tipo_t);

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

        let then_block = builder.create_block();
        let merge_block = builder.create_block();
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
        let if_cero = builder.create_block();
        let if_no_cero = builder.create_block();
        let merge_cap = builder.create_block();
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

    pub(crate) fn builtin_vector_obtener(
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
                "vector_obtener requiere un tipo genÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©rico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx = self.compilar_expresion(&argumentos[1], builder, variables)?;
        // El índice siempre es Entero32 → extender a I64 para el cálculo de offset
        // (el offset se suma a un puntero I64). Bug preexistente: la condición
        // anterior dependía del tipo del ELEMENTO, no del índice → con Texto
        // (I64) no convertía y `iadd(I64, I32)` rompía el verifier.
        let idx_i64 = builder.ins().sextend(types::I64, idx);

        // Bounds check (R7.6): si idx >= len → devolver 0 definido (no UB).
        // Spec: acceso fuera de rango devuelve 0 en lugar de leer memoria basura.
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let en_rango = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan, idx_i64, len);
        let ok_block = builder.create_block();
        let fuera_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, cranelift_t);
        builder.ins().brif(en_rango, ok_block, &[], fuera_block, &[]);
        builder.seal_block(ok_block);
        builder.seal_block(fuera_block);

        // Dentro de rango: leer el elemento
        builder.switch_to_block(ok_block);
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let offset = builder.ins().imul_imm(idx_i64, tamano_t);
        let addr = builder.ins().iadd(data, offset);
        let valor = builder.ins().load(cranelift_t, cranelift_codegen::ir::MemFlags::new(), addr, 0);
        builder.ins().jump(merge_block, &[valor]);

        // Fuera de rango: devolver 0 (definido, no UB)
        builder.switch_to_block(fuera_block);
        let cero = match cranelift_t {
            types::F32 => builder.ins().f32const(0.0),
            types::F64 => builder.ins().f64const(0.0),
            _ => builder.ins().iconst(cranelift_t, 0),
        };
        builder.ins().jump(merge_block, &[cero]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.block_params(merge_block)[0])
    }

    pub(crate) fn builtin_vector_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    pub(crate) fn builtin_vector_liberar(
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

    // ============================================================
    // Diccionario<K, V> ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â implementaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n como array de pares (MVP)
    // Cada bucket: hash(4) + occupied(1) + padding(3) + key(K) + value(V)
    // ============================================================

    pub(crate) fn diccionario_bucket_stride(&self, tipo_k: &Tipo, tipo_v: &Tipo) -> u32 {
        let key_size = self.tamano_tipo(tipo_k);
        let val_size = self.tamano_tipo(tipo_v);
        let raw = 8 + key_size + val_size;
        ((raw + 7) / 8) * 8
    }

    pub(crate) fn diccionario_guardar_valor(
        &self,
        builder: &mut FunctionBuilder,
        addr: cranelift_codegen::ir::Value,
        val: cranelift_codegen::ir::Value,
        tipo: &Tipo,
        flags: cranelift_codegen::ir::MemFlags,
    ) {
        // R9.0.2 — structs (incluso de 8/4/2/1 bytes) se guardan COPIANDO del ptr:
        // `val` es la dirección del slot del struct. Sin esto, un struct de 8 bytes
        // (2×Entero32) caía en el caso I64 y guardaba el PUNTERO en vez del struct.
        if self.tipo_es_struct(tipo).is_some() {
            let tam = self.tamano_tipo(tipo);
            for off in (0..tam).step_by(8) {
                let fv = builder.ins().load(types::I64, flags, val, off as i32);
                builder.ins().store(flags, fv, addr, off as i32);
            }
            return;
        }
        let tam = self.tamano_tipo(tipo);
        match tam {
            1 => { let v = builder.ins().ireduce(types::I8, val); builder.ins().store(flags, v, addr, 0); }
            4 => { let v = match builder.func.dfg.value_type(val) { types::I64 => builder.ins().ireduce(types::I32, val), _ => val }; builder.ins().store(flags, v, addr, 0); }
            8 => { let v = match builder.func.dfg.value_type(val) { types::I32 => builder.ins().uextend(types::I64, val), _ => val }; builder.ins().store(flags, v, addr, 0); }
            _ => {
                for off in (0..tam).step_by(8) {
                    let fv = builder.ins().load(types::I64, flags, val, off as i32);
                    builder.ins().store(flags, fv, addr, off as i32);
                }
            }
        }
    }

    pub(crate) fn diccionario_cargar_valor(
        &self,
        builder: &mut FunctionBuilder,
        addr: cranelift_codegen::ir::Value,
        tipo: &Tipo,
        flags: cranelift_codegen::ir::MemFlags,
    ) -> cranelift_codegen::ir::Value {
        // R9.0.2 — structs: devolver el PUNTERO al valor en el bucket (el caller
        // copia al slot o accede por campos). Incluye structs de 8 bytes (2×Entero32),
        // que antes caían en el caso I64 y devolvían el struct empaquetado como puntero.
        if self.tipo_es_struct(tipo).is_some() {
            return addr;
        }
        let tam = self.tamano_tipo(tipo);
        match tam {
            1 => {
                let loaded = builder.ins().load(types::I8, flags, addr, 0);
                builder.ins().uextend(types::I32, loaded)
            }
            4 => builder.ins().load(types::I32, flags, addr, 0),
            8 => builder.ins().load(types::I64, flags, addr, 0),
            _ => builder.ins().load(types::I64, flags, addr, 0),
        }
    }

    pub(crate) fn compilar_hash(
        &self,
        tipo: &Tipo,
        builder: &mut FunctionBuilder,
        val: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        match tipo {
            Tipo::Entero32 => {
                let prime = builder.ins().iconst(types::I32, 0x45D9F3B);
                builder.ins().imul(val, prime)
            }
            Tipo::Palabra | Tipo::Entero64 => {
                let lo = builder.ins().ireduce(types::I32, val);
                let shift_amt = builder.ins().iconst(types::I64, 32);
                let hi_shifted = builder.ins().ushr(val, shift_amt);
                let hi = builder.ins().ireduce(types::I32, hi_shifted);
                let mixed = builder.ins().bxor(lo, hi);
                let prime = builder.ins().iconst(types::I32, 0x45D9F3B);
                builder.ins().imul(mixed, prime)
            }
            _ => {
                if builder.func.dfg.value_type(val) == types::I64 {
                    builder.ins().ireduce(types::I32, val)
                } else { val }
            }
        }
    }

    pub(crate) fn compilar_comparar_claves(
        &self,
        _tipo: &Tipo,
        builder: &mut FunctionBuilder,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        let cc = cranelift_codegen::ir::condcodes::IntCC::Equal;
        builder.ins().icmp(cc, a, b)
    }

    /// Retorna I32: bucket index si existe, -1 si no
    pub(crate) fn compilar_buscar_en_diccionario(
        &self,
        builder: &mut FunctionBuilder,
        buckets_ptr: cranelift_codegen::ir::Value,
        cap: cranelift_codegen::ir::Value,
        tipo_k: &Tipo,
        key_val: cranelift_codegen::ir::Value,
        hash_val: cranelift_codegen::ir::Value,
        stride: u32,
    ) -> cranelift_codegen::ir::Value {
        let flags = cranelift_codegen::ir::MemFlags::new();
        let one_i64 = builder.ins().iconst(types::I64, 1);
        let neg_one = builder.ins().iconst(types::I32, -1);
        let stride_val = builder.ins().iconst(types::I64, stride as i64);
        let four_i64 = builder.ins().iconst(types::I64, 4);
        let eight_i64 = builder.ins().iconst(types::I64, 8);

        // Compute initial index = hash % cap
        let cap_i32 = builder.ins().ireduce(types::I32, cap);
        let start_idx = builder.ins().urem(hash_val, cap_i32);
        let start_idx_i64 = builder.ins().uextend(types::I64, start_idx);

        let header_block = builder.create_block();
        builder.append_block_param(header_block, types::I64);
        let body_block = builder.create_block();
        let found_block = builder.create_block();
        let exit_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, types::I32);

        builder.ins().jump(header_block, &[start_idx_i64]);

        // Loop header: compare i < cap
        builder.switch_to_block(header_block);
        let i = builder.block_params(header_block)[0];
        let done = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual, i, cap);
        builder.ins().brif(done, exit_block, &[], body_block, &[]);
        // body_block tiene 1 predecesor (el brif del header) → sellar inmediato es seguro
        builder.seal_block(body_block);

        // Body: check if bucket is occupied and key matches
        builder.switch_to_block(body_block);
        let offset = builder.ins().imul(i, stride_val);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset);
        let occupied_addr = builder.ins().iadd(bucket_addr, four_i64);
        let occupied_i8 = builder.ins().load(types::I8, flags, occupied_addr, 0);
        let occupied_i32 = builder.ins().uextend(types::I32, occupied_i8);
        let uno = builder.ins().iconst(types::I32, 1);
        let is_occupied = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, occupied_i32, uno);
        
        let check_block = builder.create_block();
        let advance_block = builder.create_block();
        builder.ins().brif(is_occupied, check_block, &[], advance_block, &[]);
        builder.seal_block(check_block);
        
        // Occupied: check key match
        builder.switch_to_block(check_block);
        let key_addr = builder.ins().iadd(bucket_addr, eight_i64);
        let stored_key = self.diccionario_cargar_valor(builder, key_addr, tipo_k, flags);
        let keys_match = self.compilar_comparar_claves(tipo_k, builder, stored_key, key_val);
        builder.ins().brif(keys_match, found_block, &[], advance_block, &[]);
        builder.seal_block(advance_block);

        // Advance: i++
        builder.switch_to_block(advance_block);
        let next_i = builder.ins().iadd(i, one_i64);
        let wrapped = builder.ins().urem(next_i, cap);
        // Check if wrapped back to start ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ full circle, exit
        let full_circle = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, wrapped, start_idx_i64);
        builder.ins().brif(full_circle, exit_block, &[], header_block, &[wrapped]);
        // NOTE: header_block sealed AFTER this brif (in back-edge)

        // Seal header after back-edge
        builder.seal_block(header_block);

        // Found
        builder.seal_block(found_block);
        builder.switch_to_block(found_block);
        let found_idx = builder.ins().ireduce(types::I32, i);
        builder.ins().jump(merge_block, &[found_idx]);

        // Exit (not found)
        builder.seal_block(exit_block);
        builder.switch_to_block(exit_block);
        builder.ins().jump(merge_block, &[neg_one]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        builder.block_params(merge_block)[0]
    }

    pub(crate) fn builtin_diccionario_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Diccionario necesita buckets REALES desde el inicio: descriptor_nuevo
        // deja cap=0 y compilar_buscar hace `hash % cap` → división por cero.
        // Bug R7.6: cap inicial 16 buckets (stride = 8 + K + V, alineado a 8).
        let tipo_k = &tipo_args[0];
        let tipo_v = &tipo_args[1];
        let stride = self.diccionario_bucket_stride(tipo_k, tipo_v);
        let cap_inicial: i64 = 16;
        let tam_buckets = builder.ins().iconst(types::I64, stride as i64 * cap_inicial);
        let buckets = self.llamar_malloc(builder, tam_buckets);
        let desc = self.descriptor_nuevo(builder);
        let flags = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags, buckets, desc, Self::OFFSET_PTR);
        let cap_val = builder.ins().iconst(types::I64, cap_inicial);
        builder.ins().store(flags, cap_val, desc, Self::OFFSET_CAP);
        Ok(desc)
    }

    pub(crate) fn builtin_diccionario_insertar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let tipo_v = &tipo_args[1];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let val_val = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let flags = cranelift_codegen::ir::MemFlags::new();
        let stride = self.diccionario_bucket_stride(tipo_k, tipo_v);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_insert = self.compilar_hash(tipo_k, builder, key_val);

        let existing_idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_insert, stride);
        
        // Constantes compartidas por AMBOS bloques (found/not_found) — definirlas
        // AQUÍ (bloque dominante) o el verifier falla: "uses value from non-dominating
        // inst" (SSA dominance). Bug R7.6: estaban dentro de found_block.
        let stride_i64 = builder.ins().iconst(types::I64, stride as i64);
        let val_offset_amt = (8 + self.tamano_tipo(tipo_k)) as i64;
        let val_offset_val = builder.ins().iconst(types::I64, val_offset_amt);

        let found_block = builder.create_block();
        let not_found_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, types::I64);
        let neg_one = builder.ins().iconst(types::I32, -1);
        let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, existing_idx, neg_one);
        builder.ins().brif(cmp, found_block, &[], not_found_block, &[]);
        builder.seal_block(found_block);
        builder.seal_block(not_found_block);

        // Found: overwrite value at existing_idx
        builder.switch_to_block(found_block);
        let idx_i64 = builder.ins().uextend(types::I64, existing_idx);
        let offset_bytes = builder.ins().imul(idx_i64, stride_i64);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset_bytes);
        let val_addr = builder.ins().iadd(bucket_addr, val_offset_val);
        self.diccionario_guardar_valor(builder, val_addr, val_val, tipo_v, flags);
        builder.ins().jump(merge_block, &[dict_ptr]);

        // Not found: insert into first empty slot (at len position)
        builder.switch_to_block(not_found_block);
        let len = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        // Resize (R7.6): si len >= cap → realloc a 2*cap. realloc preserva el
        // contenido; la búsqueda escanea todos los buckets (probing completo),
        // así que las claves existentes siguen siendo encontrables.
        let necesita_resize = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            len,
            cap,
        );
        let resize_block = builder.create_block();
        let no_resize_block = builder.create_block();
        let merge_resize = builder.create_block();
        builder.ins().brif(necesita_resize, resize_block, &[], no_resize_block, &[]);
        builder.seal_block(resize_block);
        builder.seal_block(no_resize_block);

        builder.switch_to_block(resize_block);
        let dos = builder.ins().iconst(types::I64, 2);
        let new_cap = builder.ins().imul(dos, cap);
        let new_size = builder.ins().imul_imm(new_cap, stride as i64);
        let buckets_nuevos = self.llamar_realloc(builder, buckets_ptr, new_size);
        let flags_resize = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags_resize, buckets_nuevos, dict_ptr, Self::OFFSET_PTR);
        builder.ins().store(flags_resize, new_cap, dict_ptr, Self::OFFSET_CAP);
        builder.ins().jump(merge_resize, &[]);

        builder.switch_to_block(no_resize_block);
        builder.ins().jump(merge_resize, &[]);
        builder.seal_block(merge_resize);
        builder.switch_to_block(merge_resize);

        // Re-cargar tras el posible resize (memoria = fuente de verdad)
        let buckets_final = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let len_offset = builder.ins().imul(len, stride_i64);
        let empty_addr = builder.ins().iadd(buckets_final, len_offset);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);
        builder.ins().store(flags, hash_val, empty_addr, 0);
        let uno_i8 = builder.ins().iconst(types::I8, 1);
        builder.ins().store(flags, uno_i8, empty_addr, 4);
        let key_offset = builder.ins().iconst(types::I64, 8);
        let key_addr = builder.ins().iadd(empty_addr, key_offset);
        self.diccionario_guardar_valor(builder, key_addr, key_val, tipo_k, flags);
        let val_addr2 = builder.ins().iadd(empty_addr, val_offset_val);
        self.diccionario_guardar_valor(builder, val_addr2, val_val, tipo_v, flags);
        let one_i64 = builder.ins().iconst(types::I64, 1);
        let real_new_len = builder.ins().iadd(len, one_i64);
        builder.ins().store(flags, real_new_len, dict_ptr, Self::OFFSET_LEN);
        builder.ins().jump(merge_block, &[dict_ptr]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.block_params(merge_block)[0])
    }

    pub(crate) fn builtin_diccionario_obtener(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let tipo_v = &tipo_args[1];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let flags = cranelift_codegen::ir::MemFlags::new();
        let stride = self.diccionario_bucket_stride(tipo_k, tipo_v);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);

        let idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_val, stride);
        let stride_i64 = builder.ins().iconst(types::I64, stride as i64);
        let idx_i64 = builder.ins().uextend(types::I64, idx);
        let offset_bytes = builder.ins().imul(idx_i64, stride_i64);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset_bytes);
        let val_offset_amt = (8 + self.tamano_tipo(tipo_k)) as i64;
        let val_offset_val = builder.ins().iconst(types::I64, val_offset_amt);
        let val_addr = builder.ins().iadd(bucket_addr, val_offset_val);
        Ok(self.diccionario_cargar_valor(builder, val_addr, tipo_v, flags))
    }

    pub(crate) fn builtin_diccionario_existe(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let stride = self.diccionario_bucket_stride(tipo_k, &Tipo::Booleano);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);

        let idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_val, stride);
        let found = builder.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual, idx, 0);
        let uno = builder.ins().iconst(types::I32, 1);
        let cero = builder.ins().iconst(types::I32, 0);
        Ok(builder.ins().select(found, uno, cero))
    }

    pub(crate) fn builtin_diccionario_eliminar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let flags = cranelift_codegen::ir::MemFlags::new();
        let stride = self.diccionario_bucket_stride(tipo_k, &Tipo::Booleano);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);

        let idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_val, stride);
        let found_block = builder.create_block();
        let not_found_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, types::I32);
        let neg_one = builder.ins().iconst(types::I32, -1);
        let found = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, idx, neg_one);
        builder.ins().brif(found, found_block, &[], not_found_block, &[]);
        builder.seal_block(found_block);
        builder.seal_block(not_found_block);

        builder.switch_to_block(found_block);
        let stride_i64 = builder.ins().iconst(types::I64, stride as i64);
        let idx_i64 = builder.ins().uextend(types::I64, idx);
        let offset_bytes = builder.ins().imul(idx_i64, stride_i64);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset_bytes);
        let zero_i8 = builder.ins().iconst(types::I8, 0);
        builder.ins().store(flags, zero_i8, bucket_addr, 4);
        let len = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_LEN);
        let uno_i64 = builder.ins().iconst(types::I64, 1);
        let new_len = builder.ins().isub(len, uno_i64);
        builder.ins().store(flags, new_len, dict_ptr, Self::OFFSET_LEN);
        let uno_ret = builder.ins().iconst(types::I32, 1);
        builder.ins().jump(merge_block, &[uno_ret]);

        builder.switch_to_block(not_found_block);
        let cero_ret = builder.ins().iconst(types::I32, 0);
        builder.ins().jump(merge_block, &[cero_ret]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.block_params(merge_block)[0])
    }

    pub(crate) fn builtin_diccionario_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    pub(crate) fn builtin_diccionario_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        _tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let data = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        self.llamar_free(builder, data);
        self.llamar_free(builder, dict_ptr);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // Conjunto<T> ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â wrapper de Diccionario<T, Booleano>
    pub(crate) fn builtin_conjunto_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        _tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        Ok(self.descriptor_nuevo(builder))
    }

    pub(crate) fn builtin_conjunto_insertar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let mut dict_args = vec![argumentos[0].clone(), argumentos[1].clone()];
        dict_args.push(Expresion::Literal(crate::ast::Literal::Entero(1, crate::span::Span::vacio())));
        let dict_tipos = vec![tipo_args[0].clone(), Tipo::Booleano];
        self.builtin_diccionario_insertar(builder, variables, &dict_args, &dict_tipos)
    }

    pub(crate) fn builtin_conjunto_contiene(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_tipos = vec![tipo_args[0].clone(), Tipo::Booleano];
        self.builtin_diccionario_existe(builder, variables, argumentos, &dict_tipos)
    }

    pub(crate) fn builtin_conjunto_eliminar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_tipos = vec![tipo_args[0].clone(), Tipo::Booleano];
        self.builtin_diccionario_eliminar(builder, variables, argumentos, &dict_tipos)
    }

    pub(crate) fn builtin_conjunto_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_diccionario_longitud(builder, variables, argumentos)
    }

    pub(crate) fn builtin_conjunto_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        _tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_diccionario_liberar(builder, variables, argumentos, _tipo_args)
    }

    // ============================================================
    // Procesos (R7.1): lanzar comandos y capturar salida
    // ============================================================

    /// proceso_crear(comando: Palabra) -> Entero64 (handle, 0 = error)
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

}
