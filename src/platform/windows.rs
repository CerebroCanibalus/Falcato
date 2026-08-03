//! # Windows Runtime
//!
//! Primitivas de sincronización + builtins para Win32.
//!
//! ## Sync primitives layout (inline en struct canal)
//!
//! | Offset | Tamaño | Descripción |
//! |--------|--------|-------------|
//! | 0 | 8 | HANDLE mutex |
//! | 8 | 8 | HANDLE sem_signal |
//! | 16 | 8 | HANDLE sem_space |
//! | 24 | 4 | head (i32) |
//! | 28 | 4 | tail (i32) |
//! | 32 | 4 | count (i32) |
//! | 36 | 4 | capacity (i32) |
//! | 40 | ... | buffer[capacity] |

use cranelift_codegen::ir::{self, types, InstBuilder, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilder;

use crate::platform::traits::{CodegenCtx, PlatformRuntime};

pub struct WindowsRuntime;

impl WindowsRuntime {
    /// Índices de las sync primitives en el struct del canal
    const MUTEX_OFF: i32 = 0;
    const SEM_SIG_OFF: i32 = 8;
    const SEM_SPC_OFF: i32 = 16;
    const HEADER_SIZE: i32 = 40;
}

impl PlatformRuntime for WindowsRuntime {
    // ============================================================
    // Timers & Process
    // ============================================================

    fn sleep(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ms_val: Value) {
        // Sleep(DWORD ms) — DWORD = u32
        ctx.call_void("sleep", builder, &[ms_val]);
    }

    fn timestamp(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder) -> Value {
        // GetTickCount64() -> ULONGLONG (milisegundos desde boot)
        ctx.call_ret("timestamp", builder, &[])
    }

    fn exit_process(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, code: Value) {
        ctx.call_void("exit_process", builder, &[code]);
        builder.ins().trap(ir::TrapCode::UnreachableCodeReached);
    }

    // ============================================================
    // Mutex — CreateMutexW / ReleaseMutex / WaitForSingleObject / CloseHandle
    // ============================================================

    fn mutex_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        // CreateMutexW(NULL, FALSE, NULL) -> HANDLE
        let null_ptr = builder.ins().iconst(types::I64, 0);
        let false_val = builder.ins().iconst(types::I32, 0);
        let handle = ctx.call_ret("create_mutex", builder, &[null_ptr, false_val, null_ptr]);
        builder.ins().store(ir::MemFlags::new(), handle, ptr, Self::MUTEX_OFF);
    }

    fn mutex_lock(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let handle = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::MUTEX_OFF);
        let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF);
        ctx.call_void("wait_single_object", builder, &[handle, infinite]);
    }

    fn mutex_unlock(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let handle = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::MUTEX_OFF);
        ctx.call_void("release_mutex", builder, &[handle]);
    }

    fn mutex_destroy(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        let handle = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::MUTEX_OFF);
        ctx.call_void("close_handle", builder, &[handle]);
    }

    // ============================================================
    // Semaphore — CreateSemaphoreW / ReleaseSemaphore / WaitForSingleObject / CloseHandle
    // ============================================================

    fn sem_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, _max: Value) {
        // Dos semáforos: signal (init=0) y space (init=max)
        let null_ptr = builder.ins().iconst(types::I64, 0);
        let _false_val = builder.ins().iconst(types::I32, 0);

        // sem_signal: CreateSemaphoreW(NULL, 0, max, NULL)
        let cero = builder.ins().iconst(types::I32, 0);
        let handle_sig = ctx.call_ret("create_semaphore", builder, &[null_ptr, cero, _max, null_ptr]);
        builder.ins().store(ir::MemFlags::new(), handle_sig, ptr, Self::SEM_SIG_OFF);

        // sem_space: CreateSemaphoreW(NULL, max, max, NULL) — arranca lleno
        let handle_spc = ctx.call_ret("create_semaphore", builder, &[null_ptr, _max, _max, null_ptr]);
        builder.ins().store(ir::MemFlags::new(), handle_spc, ptr, Self::SEM_SPC_OFF);
    }

    fn sem_post(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        // ReleaseSemaphore(signal, 1, NULL)
        let one = builder.ins().iconst(types::I32, 1);
        let null_ptr = builder.ins().iconst(types::I64, 0);
        let handle = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_OFF);
        ctx.call_void("release_semaphore", builder, &[handle, one, null_ptr]);
    }

    fn sem_wait(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, _timeout_ms: Value) {
        // WaitForSingleObject(semaphore, timeout_ms)
        let handle = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_OFF);
        let infinite = builder.ins().iconst(types::I32, 0xFFFFFFFF);
        ctx.call_void("wait_single_object", builder, &[handle, infinite]);
    }

    fn sem_trywait(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) -> Value {
        // WaitForSingleObject(semaphore, 0) -> WAIT_OBJECT_0=0 if acquired
        let handle = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_OFF);
        let cero = builder.ins().iconst(types::I32, 0);
        let result = ctx.call_ret("wait_single_object", builder, &[handle, cero]);
        let cero_i32 = builder.ins().iconst(types::I32, 0);
        builder.ins().icmp(ir::condcodes::IntCC::Equal, result, cero_i32)
    }

    fn sem_destroy(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) {
        // CloseHandle(signal) + CloseHandle(space)
        let sig = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SIG_OFF);
        let spc = builder.ins().load(types::I64, ir::MemFlags::new(), ptr, Self::SEM_SPC_OFF);
        ctx.call_void("close_handle", builder, &[sig]);
        ctx.call_void("close_handle", builder, &[spc]);
    }

    // ============================================================
    // Networking
    // ============================================================

    fn net_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder) {
        // WSAStartup(0x0202, &wsadata)
        let slot = builder.create_sized_stack_slot(
            ir::StackSlotData::new(ir::StackSlotKind::ExplicitSlot, 408, 0));
        let wsa_ptr = builder.ins().stack_addr(types::I64, slot, 0);
        let version = builder.ins().iconst(types::I32, 0x0202);
        ctx.call_void("wsa_startup", builder, &[version, wsa_ptr]);
    }

    fn net_close(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, sock: Value) {
        ctx.call_void("close_socket", builder, &[sock]);
    }

    // ============================================================
    // Threads
    // ============================================================

    fn thread_wrapper_signature(&self) -> ir::Signature {
        // Windows: CreateThread callback es fn(LPVOID) -> DWORD
        // LPVOID = I64 (ptr), DWORD = I32
        let mut sig = ir::Signature::new(CallConv::WindowsFastcall);
        sig.params.push(ir::AbiParam::new(types::I64)); // lpParameter
        sig.returns.push(ir::AbiParam::new(types::I32)); // return value
        sig
    }
}
