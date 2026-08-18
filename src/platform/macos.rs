//! # macOS Runtime
//!
//! macOS es POSIX como Linux, con diferencias menores:
//! - `clock_gettime` no está disponible en macOS < 10.12 (usar `mach_absolute_time`)
//! - `sem_init` está deprecado (usar `dispatch_semaphore` o `mach semaphores`)
//! - Calling convention: AppleAarch64 en ARM, SystemV en x64
//!
//! ## Implementación actual
//!
//! Por ahora, macOS usa las mismas primitivas que Linux.
//! Las diferencias se ajustarán cuando tengamos acceso a un Mac para probar.

use cranelift_codegen::ir::{self, types, InstBuilder, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;

use crate::platform::traits::{CodegenCtx, PlatformRuntime};

pub struct MacOsRuntime;

impl MacOsRuntime {
    const MUTEX_PTR_OFF: i32 = 0;
    const SEM_SIG_PTR_OFF: i32 = 8;
    const SEM_SPC_PTR_OFF: i32 = 16;
    const HEADER_SIZE: i32 = 40;

    fn malloc(ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, tam: i64) -> Value {
        let tam_val = builder.ins().iconst(types::I64, tam);
        ctx.call_ret("malloc", builder, &[tam_val])
    }
}

impl PlatformRuntime for MacOsRuntime {
    // Por ahora, idéntico a LinuxRuntime.
    // TODO: ajustar cuando tengamos un Mac para testing.

    fn call_conv_default(&self) -> CallConv {
        // AppleAarch64 en ARM64 (Apple Silicon), SystemV en x86_64
        #[cfg(target_arch = "aarch64")]
        { CallConv::AppleAarch64 }
        #[cfg(not(target_arch = "aarch64"))]
        { CallConv::SystemV }
    }

    fn sleep(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ms_val: Value) {
        // En macOS, nanosleep es preferible a usleep
        // struct timespec { time_t tv_sec; long tv_nsec; }
        // Por ahora, usamos usleep como Linux
        let mil = builder.ins().iconst(types::I32, 1000);
        let us = builder.ins().imul(ms_val, mil);
        ctx.call_void("sleep", builder, &[us]);
    }

    fn timestamp(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder) -> Value {
        // clock_gettime también funciona en macOS 10.12+
        // Fallback: mach_absolute_time() si clock_gettime no está
        let ts_slot = builder.create_sized_stack_slot(
            ir::StackSlotData::new(ir::StackSlotKind::ExplicitSlot, 16, 0));
        let ts_ptr = builder.ins().stack_addr(types::I64, ts_slot, 0);

        let clock_id = ctx.cache.asegurar(
            ctx.module,
            "clock_gettime",
            &[types::I32, types::I64],
            Some(types::I32),
        );
        let func_ref = ctx.module.declare_func_in_func(clock_id, builder.func);
        let clock_mono = builder.ins().iconst(types::I32, 6); // CLOCK_MONOTONIC=6 en macOS (1 en Linux)
        builder.ins().call(func_ref, &[clock_mono, ts_ptr]);

        let tv_sec = builder.ins().load(types::I64, ir::MemFlags::new(), ts_ptr, 0);
        let tv_nsec = builder.ins().load(types::I64, ir::MemFlags::new(), ts_ptr, 8);
        let mil = builder.ins().iconst(types::I64, 1000);
        let sec_ms = builder.ins().imul(tv_sec, mil);
        let millon = builder.ins().iconst(types::I64, 1_000_000);
        let nsec_ms = builder.ins().udiv(tv_nsec, millon);
        builder.ins().iadd(sec_ms, nsec_ms)
    }

    fn exit_process(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, code: Value) {
        ctx.call_void("exit_process", builder, &[code]);
        builder.ins().trap(ir::TrapCode::UnreachableCodeReached);
    }

    fn mutex_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let mutex_ptr = Self::malloc(ctx, builder, 64); // pthread_mutex_t en macOS
        let null_ptr = builder.ins().iconst(types::I64, 0);
        ctx.call_void("create_mutex", builder, &[mutex_ptr, null_ptr]);
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

    fn sem_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, max: Value) {
        let sem_sig_ptr = Self::malloc(ctx, builder, 32);
        let sem_spc_ptr = Self::malloc(ctx, builder, 32);
        let cero_i32 = builder.ins().iconst(types::I32, 0);
        let pshared = builder.ins().iconst(types::I32, 0);

        // En macOS sem_init está deprecado. Usamos dispatch_semaphore en el futuro.
        ctx.call_void("create_semaphore", builder, &[sem_sig_ptr, pshared, cero_i32]);
        ctx.call_void("create_semaphore", builder, &[sem_spc_ptr, pshared, max]);
        builder.ins().store(ir::MemFlags::new(), sem_sig_ptr, ptr, Self::SEM_SIG_PTR_OFF);
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

    fn net_init(&self, _ctx: &mut CodegenCtx, _builder: &mut FunctionBuilder) {}
    fn net_close(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, sock: Value) {
        ctx.call_void("close_socket", builder, &[sock]);
    }

    fn thread_wrapper_signature(&self) -> ir::Signature {
        // AppleAarch64 en ARM64 (Apple Silicon), SystemV en x86_64
        #[cfg(target_arch = "aarch64")]
        let conv = CallConv::AppleAarch64;
        #[cfg(not(target_arch = "aarch64"))]
        let conv = CallConv::SystemV;
        let mut sig = ir::Signature::new(conv);
        sig.params.push(ir::AbiParam::new(types::I64));
        sig.returns.push(ir::AbiParam::new(types::I64));
        sig
    }
}
