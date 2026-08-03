//! # Linux Runtime (POSIX)
//!
//! Primitivas de sincronización + builtins para Linux/POSIX.
//!
//! ## Sync primitives layout (punteros a heap)
//!
//! En Linux, pthread_mutex_t y sem_t son estructuras de tamaño variable
//! y no portátil. Por eso almacenamos PUNTEROS a objetos heap-allocados.
//!
//! | Offset | Tamaño | Descripción |
//! |--------|--------|-------------|
//! | 0 | 8 | ptr → pthread_mutex_t |
//! | 8 | 8 | ptr → sem_t (signal) |
//! | 16 | 8 | ptr → sem_t (space) |
//! | 24 | 4 | head (i32) |
//! | 28 | 4 | tail (i32) |
//! | 32 | 4 | count (i32) |
//! | 36 | 4 | capacity (i32) |
//! | 40 | ... | buffer[capacity] |

use cranelift_codegen::ir::{self, types, InstBuilder, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;

use crate::platform::traits::{CodegenCtx, PlatformRuntime};

pub struct LinuxRuntime;

impl LinuxRuntime {
    const MUTEX_PTR_OFF: i32 = 0;
    const SEM_SIG_PTR_OFF: i32 = 8;
    const SEM_SPC_PTR_OFF: i32 = 16;
    const HEADER_SIZE: i32 = 40;

    /// malloc(tam) helper
    fn malloc(
        ctx: &mut CodegenCtx,
        builder: &mut FunctionBuilder,
        tam: i64,
    ) -> Value {
        let tam_val = builder.ins().iconst(types::I64, tam);
        ctx.call_ret("malloc", builder, &[tam_val])
    }

    /// Tamaño típico de pthread_mutex_t en glibc/x64
    /// NO confiar en este valor — usar sizeof real sería mejor.
    /// Por seguridad, usamos 40 bytes (lo que ocupa en glibc).
    const MUTEX_SIZE: i64 = 40;
    const SEM_SIZE: i64 = 32; // sem_t en glibc/x64
}

impl PlatformRuntime for LinuxRuntime {
    // ============================================================
    // Calling convention
    // ============================================================

    fn call_conv_default(&self) -> CallConv {
        CallConv::SystemV
    }

    // ============================================================
    // Timers & Process
    // ============================================================

    fn sleep(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ms_val: Value) {
        // usleep(useconds_t) — useconds_t es u32. ms*1000 = us.
        // Por ahora: usleep(ms * 1000)
        let mil = builder.ins().iconst(types::I32, 1000);
        let us = builder.ins().imul(ms_val, mil);
        ctx.call_void("sleep", builder, &[us]);
    }

    fn timestamp(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder) -> Value {
        // clock_gettime(CLOCK_MONOTONIC, &ts)
        // Necesitamos: struct timespec { time_t tv_sec; long tv_nsec; }
        // Luego: resultado = tv_sec * 1000 + tv_nsec / 1000000
        //
        // Creamos struct timespec en stack (16 bytes en x64)
        let ts_slot = builder.create_sized_stack_slot(
            ir::StackSlotData::new(ir::StackSlotKind::ExplicitSlot, 16, 0));
        let ts_ptr = builder.ins().stack_addr(types::I64, ts_slot, 0);

        // clock_gettime no está en el registry (es POSIX, no C99)
        // Lo declaramos directamente: clock_gettime(i32, i64) -> i32
        let clock_id = ctx.cache.asegurar(
            ctx.module,
            "clock_gettime",
            &[types::I32, types::I64],
            Some(types::I32),
        );
        let func_ref = ctx.module.declare_func_in_func(clock_id, builder.func);
        let clock_mono = builder.ins().iconst(types::I32, 1); // CLOCK_MONOTONIC = 1
        builder.ins().call(func_ref, &[clock_mono, ts_ptr]);

        // Cargar tv_sec (offset 0, i64) y tv_nsec (offset 8, i64)
        let tv_sec = builder.ins().load(types::I64, ir::MemFlags::new(), ts_ptr, 0);
        let tv_nsec = builder.ins().load(types::I64, ir::MemFlags::new(), ts_ptr, 8);

        // ms = tv_sec * 1000 + tv_nsec / 1000000
        let mil = builder.ins().iconst(types::I64, 1000);
        let sec_ms = builder.ins().imul(tv_sec, mil);
        let millon = builder.ins().iconst(types::I64, 1_000_000);
        let nsec_ms = builder.ins().udiv(tv_nsec, millon);
        builder.ins().iadd(sec_ms, nsec_ms)
    }

    fn exit_process(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, code: Value) {
        // _exit(int status) — POSIX, termina sin cleanup
        ctx.call_void("exit_process", builder, &[code]);
        builder.ins().trap(ir::TrapCode::UnreachableCodeReached);
    }

    // ============================================================
    // Mutex — pthread_mutex_init/lock/unlock/destroy
    // ============================================================

    fn mutex_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        // malloc(MUTEX_SIZE) → pthread_mutex_init(ptr, NULL)
        let mutex_ptr = Self::malloc(ctx, builder, Self::MUTEX_SIZE);
        let null_ptr = builder.ins().iconst(types::I64, 0);
        ctx.call_void("create_mutex", builder, &[mutex_ptr, null_ptr]);
        // Guardar puntero en el struct del canal
        builder.ins().store(ir::MemFlags::new(), mutex_ptr, ptr, Self::MUTEX_PTR_OFF);
    }

    fn mutex_lock(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let mutex_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::MUTEX_PTR_OFF);
        ctx.call_void("lock_mutex", builder, &[mutex_ptr]);
    }

    fn mutex_unlock(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let mutex_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::MUTEX_PTR_OFF);
        ctx.call_void("release_mutex", builder, &[mutex_ptr]);
    }

    fn mutex_destroy(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let mutex_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::MUTEX_PTR_OFF);
        ctx.call_void("destroy_mutex", builder, &[mutex_ptr]);
        ctx.call_void("free", builder, &[mutex_ptr]);
    }

    // ============================================================
    // Semaphore — sem_init(ptr, 0, initial_value) / sem_post / sem_wait / sem_trywait / sem_destroy
    // ============================================================

    fn sem_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, max: Value) {
        // signal sem: sem_init(ptr, 0, 0)
        let sem_sig_ptr = Self::malloc(ctx, builder, Self::SEM_SIZE);
        let cero_i32 = builder.ins().iconst(types::I32, 0);
        ctx.call_void("create_semaphore", builder, &[sem_sig_ptr, cero_i32, cero_i32]);
        builder.ins().store(ir::MemFlags::new(), sem_sig_ptr, ptr, Self::SEM_SIG_PTR_OFF);

        // space sem: sem_init(ptr, 0, max)
        let sem_spc_ptr = Self::malloc(ctx, builder, Self::SEM_SIZE);
        let pshered = builder.ins().iconst(types::I32, 0); // pshared=0 (thread-local)
        ctx.call_void("create_semaphore", builder, &[sem_spc_ptr, pshered, max]);
        builder.ins().store(ir::MemFlags::new(), sem_spc_ptr, ptr, Self::SEM_SPC_PTR_OFF);
    }

    fn sem_post(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let sem_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_PTR_OFF);
        ctx.call_void("release_semaphore", builder, &[sem_ptr]);
    }

    fn sem_wait(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, _timeout_ms: Value) {
        let sem_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_PTR_OFF);
        ctx.call_void("wait_semaphore", builder, &[sem_ptr]);
    }

    fn sem_trywait(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) -> Value {
        let sem_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_PTR_OFF);
        let result = ctx.call_ret("try_wait_semaphore", builder, &[sem_ptr]);
        // sem_trywait retorna 0 si adquirió
        let cero = builder.ins().iconst(types::I32, 0);
        builder.ins().icmp(ir::condcodes::IntCC::Equal, result, cero)
    }

    fn sem_destroy(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let sig_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_PTR_OFF);
        let spc_ptr = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SPC_PTR_OFF);
        ctx.call_void("destroy_semaphore", builder, &[sig_ptr]);
        ctx.call_void("destroy_semaphore", builder, &[spc_ptr]);
        ctx.call_void("free", builder, &[sig_ptr]);
        ctx.call_void("free", builder, &[spc_ptr]);
    }

    // ============================================================
    // Networking
    // ============================================================

    fn net_init(&self, _ctx: &mut CodegenCtx, _builder: &mut FunctionBuilder) {
        // No-op: POSIX sockets no necesitan inicialización
    }

    fn net_close(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, sock: Value) {
        ctx.call_void("close_socket", builder, &[sock]);
    }

    // ============================================================
    // Threads
    // ============================================================

    fn thread_wrapper_signature(&self) -> ir::Signature {
        // pthread_create callback: fn(void*) -> void*
        // void* = I64
        let mut sig = ir::Signature::new(CallConv::SystemV);
        sig.params.push(ir::AbiParam::new(types::I64)); // arg
        sig.returns.push(ir::AbiParam::new(types::I64)); // retval
        sig
    }
}
