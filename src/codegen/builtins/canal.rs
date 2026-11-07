use crate::codegen::*;

impl Codegen {
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

}
