use crate::codegen::*;

impl Codegen {
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

    // ============================================================
    // TCP Cliente + DNS
    // ============================================================

    /// tcp_conectar(host: Palabra, puerto: Entero32) -> Entero64 (socket handle, 0 = error)
    pub(crate) fn builtin_tcp_conectar(
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
        // falcato_tcp_conectar(host: *const c_char, puerto: i32) -> i64
        let fn_id = self.asegurar_funcion_c("falcato_tcp_conectar", &[types::I64, types::I32], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[host, puerto_i32]);
        Ok(builder.inst_results(call)[0])
    }

    /// dns_resolver(host: Palabra) -> Texto (IP resuelta)
    pub(crate) fn builtin_dns_resolver(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let host = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_dns_resolver(host: *const c_char) -> *mut c_char (i64)
        let fn_id = self.asegurar_funcion_c("falcato_dns_resolver", &[types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[host]);
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

    /// tcp_establecer_timeout(sock: Entero64, ms: Entero32) — establece timeout de lectura/escritura
    pub(crate) fn builtin_tcp_establecer_timeout(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let sock = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let ms = self.compilar_expresion(&argumentos[1], builder, variables)?;
        // falcato_tcp_establecer_timeout(sock: i64, ms: i32) -> void
        let fn_id = self.asegurar_funcion_c("falcato_tcp_establecer_timeout", &[types::I64, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[sock, ms]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// tcp_datos_disponibles(sock: Entero64) -> Booleano (1 = hay datos, 0 = no)
    pub(crate) fn builtin_tcp_datos_disponibles(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let sock = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_tcp_datos_disponibles(sock: i64) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_tcp_datos_disponibles", &[types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[sock]);
        Ok(builder.inst_results(call)[0])
    }

}
