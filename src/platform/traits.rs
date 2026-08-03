//! # PlatformRuntime — Primitivas de sincronización por plataforma
//!
//! El ring buffer de canales es el mismo en todas las plataformas.
//! Solo cambian las primitivas de sincronización:
//!
//! | Operación | Windows | Linux |
//! |-----------|---------|-------|
//! | init_mutex | CreateMutexW | pthread_mutex_init |
//! | lock | WaitForSingleObject | pthread_mutex_lock |
//! | unlock | ReleaseMutex | pthread_mutex_unlock |
//! | destroy_mutex | CloseHandle | pthread_mutex_destroy |
//! | init_sem(cnt) | CreateSemaphoreW(0, cnt) | sem_init(0, cnt) |
//! | post(sem) | ReleaseSemaphore | sem_post |
//! | wait(sem) | WaitForSingleObject | sem_wait |
//! | trywait(sem) | WaitForSingleObject(0) | sem_trywait |
//! | destroy_sem | CloseHandle | sem_destroy |
//!
//! ## Uso
//!
//! codegen.rs llama a `ctx.platform.lock_mutex(...)` sin saber qué OS es.

use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use crate::codegen_helpers::CFunctionCache;
use crate::platform::registry::BuiltinRegistry;

/// Contexto que la codegen pasa a las funciones de plataforma.
pub struct CodegenCtx<'a> {
    pub cache: &'a mut CFunctionCache,
    pub module: &'a mut ObjectModule,
    pub registry: &'a BuiltinRegistry,
}

impl<'a> CodegenCtx<'a> {
    pub fn new(
        cache: &'a mut CFunctionCache,
        module: &'a mut ObjectModule,
        registry: &'a BuiltinRegistry,
    ) -> Self {
        Self { cache, module, registry }
    }

    /// Llama una función C del registry y retorna el resultado (si tiene retorno).
    pub fn call(&mut self, name: &str, builder: &mut FunctionBuilder, args: &[Value]) -> Option<Value> {
        let entry = self.registry.lookup(name)?;
        let func_id = self.cache.asegurar(self.module, &entry.name, &entry.sig.params, entry.sig.ret);
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let call = builder.ins().call(func_ref, args);
        entry.sig.ret.map(|_| builder.inst_results(call)[0])
    }

    pub fn call_ret(&mut self, name: &str, builder: &mut FunctionBuilder, args: &[Value]) -> Value {
        self.call(name, builder, args).expect(&format!("builtin '{}' no encontrado", name))
    }

    pub fn call_void(&mut self, name: &str, builder: &mut FunctionBuilder, args: &[Value]) {
        self.call(name, builder, args);
    }
}

/// Trait que cada plataforma implementa con sus primitivas de sync + builtins complejos.
pub trait PlatformRuntime {
    // ============================================================
    // Timers & Process
    // ============================================================
    fn sleep(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ms_val: Value);
    fn timestamp(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder) -> Value;
    fn exit_process(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, code: Value);

    // ============================================================
    // Mutex primitives
    // ============================================================
    /// Inicializa un mutex en la dirección dada.
    fn mutex_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value);
    /// Lock mutex (bloqueante).
    fn mutex_lock(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value);
    /// Unlock mutex.
    fn mutex_unlock(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value);
    /// Destruye un mutex.
    fn mutex_destroy(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value);

    // ============================================================
    // Semaphore primitives
    // ============================================================
    /// Inicializa un semáforo con valor inicial 0 y máximo `max`.
    fn sem_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, max: Value);
    /// Post (incrementar) semáforo.
    fn sem_post(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value);
    /// Wait (decrementar) semáforo (bloqueante).
    fn sem_wait(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value, timeout_ms: Value);
    /// Try wait (no bloqueante). Retorna true si adquirió.
    fn sem_trywait(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value) -> Value;
    /// Destruye un semáforo.
    fn sem_destroy(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, ptr: Value);

    // ============================================================
    // Networking
    // ============================================================
    /// Inicializa networking. En Windows es WSAStartup, en POSIX no-op.
    fn net_init(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder);
    /// Cierra un socket. En Windows es closesocket, en POSIX es close.
    fn net_close(&self, ctx: &mut CodegenCtx, builder: &mut FunctionBuilder, sock: Value);

    // ============================================================
    // Threads
    // ============================================================
    /// Firma para el wrapper de hilos: fn(i64) -> i32 (ptr a args → ret code)
    fn thread_wrapper_signature(&self) -> cranelift_codegen::ir::Signature;
}
