//! # BuiltinRegistry — Mapeo declarativo nombre→función C por plataforma
//!
//! Para builtins simples donde solo cambia el nombre de la función C
//! y/o los tipos de parámetros entre plataformas.
//!
//! ## Uso
//!
//! ```rust,ignore
//! let reg = BuiltinRegistry::for_target("x86_64-pc-windows-msvc");
//! let sleep = reg.lookup("Sleep").unwrap();
//! // sleep.name = "Sleep", sleep.params = [I32]
//! ```

use cranelift_codegen::ir::types;
use cranelift_codegen::ir::Type;
use std::collections::HashMap;

/// Firma de una función C: tipos de parámetros y retorno.
#[derive(Debug, Clone)]
pub struct FuncSignature {
    pub params: Vec<Type>,
    pub ret: Option<Type>,
}

/// Entrada del registry: nombre de función C + firma.
#[derive(Debug, Clone)]
pub struct BuiltinEntry {
    /// Nombre de la función C real (ej: "Sleep", "usleep")
    pub name: String,
    /// Firma Cranelift de la función
    pub sig: FuncSignature,
    /// Función variádica (printf): la firma exacta la decide el caller
    /// en cada llamada, no el registry. El registry solo remapea el nombre.
    pub variadic: bool,
}

/// Registry de builtins por plataforma.
///
/// Cada builtin tiene un nombre abstracto ("sleep", "exit", "timestamp")
/// que se mapea a una función C real según la plataforma.
///
/// ## Escalabilidad
///
/// Para agregar un nuevo builtin simple:
/// 1. Agregar a `BuiltinRegistry::windows()`
/// 2. Agregar a `BuiltinRegistry::linux()`
/// 3. Agregar a `BuiltinRegistry::macos()`
///
/// Para agregar una nueva plataforma:
/// 1. Agregar método `fn nueva_plataforma() -> Self`
/// 2. Agregar entrada en `for_target()`
pub struct BuiltinRegistry {
    entries: HashMap<String, BuiltinEntry>,
}

impl BuiltinRegistry {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Busca un builtin por su nombre abstracto.
    pub fn lookup(&self, abstract_name: &str) -> Option<&BuiltinEntry> {
        self.entries.get(abstract_name)
    }

    /// Itera todas las entradas (para depuración).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &BuiltinEntry)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    // ============================================================
    // Windows
    // ============================================================
    pub fn windows() -> Self {
        let mut r = Self::new();

        // C runtime (compartido con POSIX)
        r.insert_cruntime();

        // Threading
        r.insert("create_thread", "CreateThread",
            &[types::I64, types::I64, types::I64, types::I64, types::I32, types::I64],
            Some(types::I64));
        r.insert("exit_thread", "ExitThread", &[types::I32], None);

        // Sync primitives
        r.insert("create_mutex", "CreateMutexW",
            &[types::I64, types::I32, types::I64], Some(types::I64));
        r.insert("release_mutex", "ReleaseMutex",
            &[types::I64], Some(types::I32));
        r.insert("create_semaphore", "CreateSemaphoreW",
            &[types::I64, types::I32, types::I32, types::I64], Some(types::I64));
        r.insert("release_semaphore", "ReleaseSemaphore",
            &[types::I64, types::I32, types::I64], Some(types::I32));
        r.insert("wait_single_object", "WaitForSingleObject",
            &[types::I64, types::I32], Some(types::I32));
        r.insert("close_handle", "CloseHandle",
            &[types::I64], Some(types::I32));
        r.insert("create_event", "CreateEventW",
            &[types::I64, types::I32, types::I32, types::I64], Some(types::I64));

        // Timers & process
        r.insert("sleep", "Sleep", &[types::I32], None);
        r.insert("timestamp", "GetTickCount64", &[], Some(types::I64));
        r.insert("exit_process", "ExitProcess", &[types::I32], None);
        r.insert("proceso_crear", "falcato_proceso_crear", &[types::I64], Some(types::I64));
        r.insert("proceso_esperar", "falcato_proceso_esperar", &[types::I64], Some(types::I32));
        r.insert("proceso_leer_salida_completa", "falcato_proceso_leer_salida", &[types::I64], Some(types::I64));
        r.insert("proceso_cerrar", "falcato_proceso_cerrar", &[types::I64], None);

        // Proceso bidireccional (pipes para MCP servers)
        r.insert("proceso_crear_con_pipes", "falcato_proceso_crear_con_pipes", &[types::I64], Some(types::I64));
        r.insert("proceso_escribir", "falcato_proceso_escribir", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("proceso_leer_salida_chunk", "falcato_proceso_leer_salida_chunk", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("proceso_leer_error_chunk", "falcato_proceso_leer_error_chunk", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("proceso_cerrar_entrada", "falcato_proceso_cerrar_entrada", &[types::I64], None);
        r.insert("proceso_listo_para_leer", "falcato_proceso_listo_para_leer", &[types::I64, types::I32], Some(types::I32));
        r.insert("proceso_cerrar_bidireccional", "falcato_proceso_cerrar_bidireccional", &[types::I64], None);

        // Terminal (R7.2): modo raw + lectura de teclas
        r.insert("terminal_modo_raw", "falcato_terminal_modo_raw", &[types::I32], Some(types::I32));
        r.insert("terminal_leer_tecla", "falcato_terminal_leer_tecla", &[], Some(types::I32));
        r.insert("terminal_dimensiones", "falcato_terminal_dimensiones", &[], Some(types::I64));

        // Entrada estándar (R7.3)
        r.insert("entrada_leer", "falcato_entrada_leer", &[], Some(types::I64));

        // Reloj de pared (R7.4)
        r.insert("fecha_unix", "falcato_fecha_unix", &[], Some(types::I64));
        r.insert("fecha_ms", "falcato_fecha_ms", &[], Some(types::I64));

        // Argumentos de línea de comandos (R7.5)
        r.insert("argumentos", "falcato_argumentos", &[], Some(types::I64));

        // Conversión texto→número (R7.5 Fase 2): reciben (ptr, len)
        r.insert("texto_a_entero", "falcato_texto_a_entero", &[types::I64, types::I64], Some(types::I64));
        r.insert("texto_a_natural", "falcato_texto_a_natural", &[types::I64, types::I64], Some(types::I64));
        r.insert("texto_a_flotante", "falcato_texto_a_flotante", &[types::I64, types::I64], Some(types::I64));
        r.insert("texto_a_booleano", "falcato_texto_a_booleano", &[types::I64, types::I64], Some(types::I64));

        // DHT (R8.2): índice P2P
        r.insert("dht_nuevo", "falcato_dht_nuevo", &[types::I32], Some(types::I64));
        r.insert("dht_publicar", "falcato_dht_publicar", &[types::I64, types::I64, types::I64, types::I64, types::I64], Some(types::I32));
        r.insert("dht_consultar", "falcato_dht_consultar", &[types::I64, types::I64, types::I64], Some(types::I64));
        r.insert("dht_bootstrap", "falcato_dht_bootstrap", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("dht_cerrar", "falcato_dht_cerrar", &[types::I64], None);

        // Networking
        r.insert("wsa_startup", "WSAStartup",
            &[types::I32, types::I64], Some(types::I32));
        r.insert("socket", "socket",
            &[types::I32, types::I32, types::I32], Some(types::I64));
        r.insert("bind", "bind",
            &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("listen", "listen",
            &[types::I64, types::I32], Some(types::I32));
        r.insert("accept", "accept",
            &[types::I64, types::I64, types::I64], Some(types::I64));
        r.insert("recv", "recv",
            &[types::I64, types::I64, types::I32, types::I32], Some(types::I32));
        r.insert("send", "send",
            &[types::I64, types::I64, types::I32, types::I32], Some(types::I32));
        r.insert("close_socket", "closesocket",
            &[types::I64], Some(types::I32));

        // TCP Cliente + DNS
        r.insert("tcp_conectar", "falcato_tcp_conectar", &[types::I64, types::I32], Some(types::I64));
        r.insert("dns_resolver", "falcato_dns_resolver", &[types::I64], Some(types::I64));
        r.insert("tcp_establecer_timeout", "falcato_tcp_establecer_timeout", &[types::I64, types::I32], None);
        r.insert("tcp_datos_disponibles", "falcato_tcp_datos_disponibles", &[types::I64], Some(types::I32));

        // Texto dinámico
        r.insert("texto_agregar_texto", "falcato_texto_agregar_texto", &[types::I64, types::I64], None);
        r.insert("texto_poner_byte", "falcato_texto_poner_byte", &[types::I64, types::I32, types::I32], None);
        r.insert("texto_puntero", "falcato_texto_puntero", &[types::I64], Some(types::I64));
        r.insert("texto_desde_bytes", "falcato_texto_desde_bytes", &[types::I64, types::I32, types::I64], None);

        // Conversión numérica
        r.insert("entero_a_texto", "falcato_entero_a_texto", &[types::I64, types::I64], None);
        r.insert("flotante_a_texto", "falcato_flotante_a_texto", &[types::I64, types::I64], None);
        r.insert("booleano_a_texto", "falcato_booleano_a_texto", &[types::I32, types::I64], None);

        // Archivos avanzados + entorno (R7.8 FASE 4)
        r.insert("archivo_agregar", "falcato_archivo_agregar", &[types::I64, types::I64], None);
        r.insert("archivo_borrar", "falcato_archivo_borrar", &[types::I64], None);
        r.insert("archivo_renombrar", "falcato_archivo_renombrar", &[types::I64, types::I64], None);
        r.insert("archivo_escribir_bytes", "falcato_archivo_escribir_bytes", &[types::I64, types::I64, types::I32], None);
        r.insert("entorno_obtener", "falcato_entorno_obtener", &[types::I64, types::I64], None);
        r.insert("directorio_actual", "falcato_directorio_actual", &[types::I64], None);
        r.insert("aleatorio", "falcato_aleatorio", &[], Some(types::I64));
        r.insert("archivo_listar", "falcato_archivo_listar", &[types::I64, types::I64], None);

        // TLS/HTTPS (R7.8 FASE 5)
        r.insert("tls_conectar", "falcato_tls_conectar", &[types::I64, types::I32], Some(types::I64));
        r.insert("tls_escribir", "falcato_tls_escribir", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("tls_leer", "falcato_tls_leer", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("tls_datos_disponibles", "falcato_tls_datos_disponibles", &[types::I64], Some(types::I32));
        r.insert("tls_cerrar", "falcato_tls_cerrar", &[types::I64], None);

        r
    }

    // ============================================================
    // Linux (POSIX)
    // ============================================================
    pub fn linux() -> Self {
        let mut r = Self::new();

        // C runtime (compartido con Windows)
        r.insert_cruntime();

        // Threading — pthread
        r.insert("create_thread", "pthread_create",
            &[types::I64, types::I64, types::I64, types::I64], Some(types::I32));
        r.insert("exit_thread", "pthread_exit", &[types::I64], None);

        // Sync primitives — pthread
        r.insert("create_mutex", "pthread_mutex_init",
            &[types::I64, types::I64], Some(types::I32));
        r.insert("release_mutex", "pthread_mutex_unlock",
            &[types::I64], Some(types::I32));
        r.insert("lock_mutex", "pthread_mutex_lock",
            &[types::I64], Some(types::I32));
        r.insert("create_semaphore", "sem_init",
            &[types::I64, types::I32, types::I32], Some(types::I32));
        r.insert("release_semaphore", "sem_post",
            &[types::I64], Some(types::I32));
        r.insert("wait_semaphore", "sem_wait",
            &[types::I64], Some(types::I32));
        r.insert("try_wait_semaphore", "sem_trywait",
            &[types::I64], Some(types::I32));
        r.insert("destroy_mutex", "pthread_mutex_destroy",
            &[types::I64], Some(types::I32));
        r.insert("destroy_semaphore", "sem_destroy",
            &[types::I64], Some(types::I32));
        r.insert("create_cond", "pthread_cond_init",
            &[types::I64, types::I64], Some(types::I32));
        r.insert("signal_cond", "pthread_cond_signal",
            &[types::I64], Some(types::I32));
        r.insert("broadcast_cond", "pthread_cond_broadcast",
            &[types::I64], Some(types::I32));
        r.insert("destroy_cond", "pthread_cond_destroy",
            &[types::I64], Some(types::I32));

        // Timers & process
        r.insert("sleep", "usleep", &[types::I32], Some(types::I32));
        r.insert("exit_process", "_exit", &[types::I32], None);
        r.insert("proceso_crear", "falcato_proceso_crear", &[types::I64], Some(types::I64));
        r.insert("proceso_esperar", "falcato_proceso_esperar", &[types::I64], Some(types::I32));
        r.insert("proceso_leer_salida_completa", "falcato_proceso_leer_salida", &[types::I64], Some(types::I64));
        r.insert("proceso_cerrar", "falcato_proceso_cerrar", &[types::I64], None);

        // Proceso bidireccional (pipes para MCP servers)
        r.insert("proceso_crear_con_pipes", "falcato_proceso_crear_con_pipes", &[types::I64], Some(types::I64));
        r.insert("proceso_escribir", "falcato_proceso_escribir", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("proceso_leer_salida_chunk", "falcato_proceso_leer_salida_chunk", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("proceso_leer_error_chunk", "falcato_proceso_leer_error_chunk", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("proceso_cerrar_entrada", "falcato_proceso_cerrar_entrada", &[types::I64], None);
        r.insert("proceso_listo_para_leer", "falcato_proceso_listo_para_leer", &[types::I64, types::I32], Some(types::I32));
        r.insert("proceso_cerrar_bidireccional", "falcato_proceso_cerrar_bidireccional", &[types::I64], None);

        // Terminal (R7.2): modo raw + lectura de teclas
        r.insert("terminal_modo_raw", "falcato_terminal_modo_raw", &[types::I32], Some(types::I32));
        r.insert("terminal_leer_tecla", "falcato_terminal_leer_tecla", &[], Some(types::I32));
        r.insert("terminal_dimensiones", "falcato_terminal_dimensiones", &[], Some(types::I64));

        // Entrada estándar (R7.3)
        r.insert("entrada_leer", "falcato_entrada_leer", &[], Some(types::I64));

        // Reloj de pared (R7.4)
        r.insert("fecha_unix", "falcato_fecha_unix", &[], Some(types::I64));
        r.insert("fecha_ms", "falcato_fecha_ms", &[], Some(types::I64));

        // Argumentos de línea de comandos (R7.5)
        r.insert("argumentos", "falcato_argumentos", &[], Some(types::I64));

        // Conversión texto→número (R7.5 Fase 2): reciben (ptr, len)
        r.insert("texto_a_entero", "falcato_texto_a_entero", &[types::I64, types::I64], Some(types::I64));
        r.insert("texto_a_natural", "falcato_texto_a_natural", &[types::I64, types::I64], Some(types::I64));
        r.insert("texto_a_flotante", "falcato_texto_a_flotante", &[types::I64, types::I64], Some(types::I64));
        r.insert("texto_a_booleano", "falcato_texto_a_booleano", &[types::I64, types::I64], Some(types::I64));

        // DHT (R8.2): índice P2P
        r.insert("dht_nuevo", "falcato_dht_nuevo", &[types::I32], Some(types::I64));
        r.insert("dht_publicar", "falcato_dht_publicar", &[types::I64, types::I64, types::I64, types::I64, types::I64], Some(types::I32));
        r.insert("dht_consultar", "falcato_dht_consultar", &[types::I64, types::I64, types::I64], Some(types::I64));
        r.insert("dht_bootstrap", "falcato_dht_bootstrap", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("dht_cerrar", "falcato_dht_cerrar", &[types::I64], None);

        // Networking — POSIX (misma API que Winsock, sin WSAStartup)
        r.insert("socket", "socket",
            &[types::I32, types::I32, types::I32], Some(types::I64));
        r.insert("bind", "bind",
            &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("listen", "listen",
            &[types::I64, types::I32], Some(types::I32));
        r.insert("accept", "accept",
            &[types::I64, types::I64, types::I64], Some(types::I64));
        r.insert("recv", "recv",
            &[types::I64, types::I64, types::I32, types::I32], Some(types::I32));
        r.insert("send", "send",
            &[types::I64, types::I64, types::I32, types::I32], Some(types::I32));
        r.insert("close_socket", "close",
            &[types::I64], Some(types::I32));

        // TCP Cliente + DNS
        r.insert("tcp_conectar", "falcato_tcp_conectar", &[types::I64, types::I32], Some(types::I64));
        r.insert("dns_resolver", "falcato_dns_resolver", &[types::I64], Some(types::I64));
        r.insert("tcp_establecer_timeout", "falcato_tcp_establecer_timeout", &[types::I64, types::I32], None);
        r.insert("tcp_datos_disponibles", "falcato_tcp_datos_disponibles", &[types::I64], Some(types::I32));

        // Texto dinámico
        r.insert("texto_agregar_texto", "falcato_texto_agregar_texto", &[types::I64, types::I64], None);
        r.insert("texto_poner_byte", "falcato_texto_poner_byte", &[types::I64, types::I32, types::I32], None);
        r.insert("texto_puntero", "falcato_texto_puntero", &[types::I64], Some(types::I64));
        r.insert("texto_desde_bytes", "falcato_texto_desde_bytes", &[types::I64, types::I32, types::I64], None);

        // Conversión numérica
        r.insert("entero_a_texto", "falcato_entero_a_texto", &[types::I64, types::I64], None);
        r.insert("flotante_a_texto", "falcato_flotante_a_texto", &[types::I64, types::I64], None);
        r.insert("booleano_a_texto", "falcato_booleano_a_texto", &[types::I32, types::I64], None);

        // Archivos avanzados + entorno (R7.8 FASE 4)
        r.insert("archivo_agregar", "falcato_archivo_agregar", &[types::I64, types::I64], None);
        r.insert("archivo_borrar", "falcato_archivo_borrar", &[types::I64], None);
        r.insert("archivo_renombrar", "falcato_archivo_renombrar", &[types::I64, types::I64], None);
        r.insert("archivo_escribir_bytes", "falcato_archivo_escribir_bytes", &[types::I64, types::I64, types::I32], None);
        r.insert("entorno_obtener", "falcato_entorno_obtener", &[types::I64, types::I64], None);
        r.insert("directorio_actual", "falcato_directorio_actual", &[types::I64], None);
        r.insert("aleatorio", "falcato_aleatorio", &[], Some(types::I64));
        r.insert("archivo_listar", "falcato_archivo_listar", &[types::I64, types::I64], None);

        // TLS/HTTPS (R7.8 FASE 5)
        r.insert("tls_conectar", "falcato_tls_conectar", &[types::I64, types::I32], Some(types::I64));
        r.insert("tls_escribir", "falcato_tls_escribir", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("tls_leer", "falcato_tls_leer", &[types::I64, types::I64, types::I32], Some(types::I32));
        r.insert("tls_datos_disponibles", "falcato_tls_datos_disponibles", &[types::I64], Some(types::I32));
        r.insert("tls_cerrar", "falcato_tls_cerrar", &[types::I64], None);

        r
    }

    // ============================================================
    // macOS (POSIX-ish, misma API que Linux en casi todo)
    // ============================================================
    pub fn macos() -> Self {
        let mut r = Self::linux(); // macOS es casi igual a Linux

        // En macOS, usleep es obsoleto pero funciona. nanosleep es preferible.
        r.insert("sleep", "nanosleep",
            &[types::I64, types::I64], Some(types::I32));

        // En macOS, pthread y semáforos POSIX están disponibles
        // (aunque sem_init está deprecado, usar dispatch_semaphore sería más nativo)
        // Por ahora, usar mismo que Linux — funciona.

        r
    }

    // ============================================================
    // Helpers
    // ============================================================

    /// C runtime functions — iguales en todas las plataformas
    fn insert_cruntime(&mut self) {
        self.insert("malloc", "malloc",
            &[types::I64], Some(types::I64));
        self.insert("free", "free",
            &[types::I64], None);
        self.insert("realloc", "realloc",
            &[types::I64, types::I64], Some(types::I64));
        self.insert("memcpy", "memcpy",
            &[types::I64, types::I64, types::I64], Some(types::I64));
        self.insert("strlen", "strlen",
            &[types::I64], Some(types::I64));
        self.insert("puts", "puts",
            &[types::I64], Some(types::I32));
        // printf es variádica: la firma exacta (número de args) la decide
        // el caller en cada llamada. El registry solo remapea el nombre.
        self.insert_variadic("printf", "printf",
            &[types::I64], Some(types::I32));

        // File I/O
        self.insert("fopen", "fopen",
            &[types::I64, types::I64], Some(types::I64));
        self.insert("fread", "fread",
            &[types::I64, types::I64, types::I64, types::I64], Some(types::I64));
        self.insert("fwrite", "fwrite",
            &[types::I64, types::I64, types::I64, types::I64], Some(types::I64));
        self.insert("fclose", "fclose",
            &[types::I64], Some(types::I32));
        self.insert("fseek", "fseek",
            &[types::I64, types::I32, types::I32], Some(types::I32));
        self.insert("ftell", "ftell",
            &[types::I64], Some(types::I64));

        // Math
        self.insert("sqrt", "sqrt",
            &[types::F64], Some(types::F64));
        self.insert("pow", "pow",
            &[types::F64, types::F64], Some(types::F64));
        // Trigonometría (libm - C math library)
        self.insert("sin", "sin",
            &[types::F64], Some(types::F64));
        self.insert("cos", "cos",
            &[types::F64], Some(types::F64));
        self.insert("tan", "tan",
            &[types::F64], Some(types::F64));
        self.insert("asin", "asin",
            &[types::F64], Some(types::F64));
        self.insert("acos", "acos",
            &[types::F64], Some(types::F64));
        self.insert("atan", "atan",
            &[types::F64], Some(types::F64));
        self.insert("atan2", "atan2",
            &[types::F64, types::F64], Some(types::F64));
        // Hiperbólicas
        self.insert("sinh", "sinh",
            &[types::F64], Some(types::F64));
        self.insert("cosh", "cosh",
            &[types::F64], Some(types::F64));
        self.insert("tanh", "tanh",
            &[types::F64], Some(types::F64));
        // Exponencial y logaritmo
        self.insert("exp", "exp",
            &[types::F64], Some(types::F64));
        self.insert("log", "log",
            &[types::F64], Some(types::F64));
        self.insert("log10", "log10",
            &[types::F64], Some(types::F64));
        // Otras útiles
        self.insert("floor", "floor",
            &[types::F64], Some(types::F64));
        self.insert("ceil", "ceil",
            &[types::F64], Some(types::F64));
        self.insert("fabs", "fabs",
            &[types::F64], Some(types::F64));
        self.insert("fmod", "fmod",
            &[types::F64, types::F64], Some(types::F64));
    }

    fn insert(&mut self, abstract_name: &str, name: &str, params: &[Type], ret: Option<Type>) {
        self.entries.insert(abstract_name.to_string(), BuiltinEntry {
            name: name.to_string(),
            sig: FuncSignature {
                params: params.to_vec(),
                ret,
            },
            variadic: false,
        });
    }

    /// Inserta una función variádica (printf): la firma exacta la decide
    /// el caller en cada llamada, no el registry.
    fn insert_variadic(&mut self, abstract_name: &str, name: &str, params: &[Type], ret: Option<Type>) {
        self.entries.insert(abstract_name.to_string(), BuiltinEntry {
            name: name.to_string(),
            sig: FuncSignature {
                params: params.to_vec(),
                ret,
            },
            variadic: true,
        });
    }

    /// Retorna el registry para un target triple dado.
    pub fn for_target(target: &str) -> Self {
        if target.contains("windows") {
            Self::windows()
        } else if target.contains("linux") {
            Self::linux()
        } else if target.contains("darwin") || target.contains("apple") {
            Self::macos()
        } else {
            Self::linux() // fallback POSIX
        }
    }

    /// Versión para compilación nativa (detecta OS actual).
    pub fn for_current_os() -> Self {
        #[cfg(target_os = "windows")]
        { Self::windows() }
        #[cfg(target_os = "linux")]
        { Self::linux() }
        #[cfg(target_os = "macos")]
        { Self::macos() }
    }

    /// Remapea un nombre de función Windows → nombre correcto según plataforma.
    /// Solo para funciones con MISMA FIRMA en ambas plataformas (nombre distinto).
    ///
    /// Funciones con FIRMAS DIFERENTES (CreateMutexW, WaitForSingleObject, etc.)
    /// deben usar PlatformRuntime trait — ver `platform/traits.rs`.
    pub fn remap(
        &self,
        win_name: &str,
        default_params: &[Type],
        default_ret: Option<Type>,
    ) -> (String, Vec<Type>, Option<Type>) {
        // Misma firma en Windows/Linux, solo cambia el nombre:
        let abstract_name = match win_name {
            "Sleep" => Some("sleep"),
            "ExitProcess" => Some("exit_process"),
            "ReleaseMutex" => Some("release_mutex"),   // misma firma: (I64) -> I32
            "puts" => Some("puts"),
            "printf" => Some("printf"),
            "malloc" => Some("malloc"),
            "free" => Some("free"),
            "memcpy" => Some("memcpy"),
            _ => None,
        };

        if let Some(abstract_name) = abstract_name {
            if let Some(entry) = self.lookup(abstract_name) {
                // Variádicas (printf): la firma exacta la decide el caller
                // en cada llamada. Solo remapeamos el nombre.
                if entry.variadic {
                    return (entry.name.clone(), default_params.to_vec(), default_ret);
                }
                return (entry.name.clone(), entry.sig.params.clone(), entry.sig.ret);
            }
        }

        (win_name.to_string(), default_params.to_vec(), default_ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_registry() {
        let reg = BuiltinRegistry::windows();
        assert!(reg.lookup("sleep").is_some());
        assert!(reg.lookup("create_thread").is_some());
        assert!(reg.lookup("wsa_startup").is_some());
        assert!(reg.lookup("timestamp").is_some());
        assert_eq!(reg.lookup("sleep").unwrap().name, "Sleep");
    }

    #[test]
    fn test_linux_registry() {
        let reg = BuiltinRegistry::linux();
        assert!(reg.lookup("sleep").is_some());
        assert!(reg.lookup("create_thread").is_some());
        assert!(!reg.lookup("wsa_startup").is_some()); // No WSA en Linux
        assert!(!reg.lookup("timestamp").is_some());   // clock_gettime via trait
        assert_eq!(reg.lookup("sleep").unwrap().name, "usleep");
    }

    #[test]
    fn test_cruntime_shared() {
        let win = BuiltinRegistry::windows();
        let lin = BuiltinRegistry::linux();
        assert_eq!(win.lookup("malloc").unwrap().name, "malloc");
        assert_eq!(lin.lookup("malloc").unwrap().name, "malloc");
        assert_eq!(win.lookup("puts").unwrap().sig.params.len(), 1);
    }

    #[test]
    fn test_for_target() {
        let win = BuiltinRegistry::for_target("x86_64-pc-windows-msvc");
        assert_eq!(win.lookup("sleep").unwrap().name, "Sleep");

        let lin = BuiltinRegistry::for_target("x86_64-unknown-linux-gnu");
        assert_eq!(lin.lookup("sleep").unwrap().name, "usleep");
    }
}
