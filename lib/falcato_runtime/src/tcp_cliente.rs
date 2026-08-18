//! # TCP Cliente + DNS — conexión a servidores y resolución de nombres
//!
//! Abstracción portable sobre connect/getaddrinfo (Winsock2) y connect/getaddrinfo (POSIX).
//!
//! API expuesta (C ABI):
//! - `falcato_tcp_conectar(host, puerto) -> Handle` — conecta a host:puerto (resuelve DNS si necesario)
//! - `falcato_dns_resolver(host) -> Texto*` — resuelve nombre de host a IP (heap, caller libera)
//! - `falcato_tcp_establecer_timeout(sock, ms)` — establece timeout de lectura/escritura
//! - `falcato_tcp_datos_disponibles(sock) -> bool` — verifica si hay datos disponibles sin bloquear
//!
//! tcp_conectar acepta tanto IPs ("127.0.0.1") como nombres ("example.com").

use std::ffi::c_void;

// ============================================================
// Windows
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::ffi::c_char;
    use std::ptr;

    const AF_INET: i32 = 2;
    const SOCK_STREAM: i32 = 1;
    const IPPROTO_TCP: i32 = 6;
    const INVALID_SOCKET: usize = usize::MAX;

    #[repr(C)]
    struct AddrInfoA {
        ai_flags: i32,
        ai_family: i32,
        ai_socktype: i32,
        ai_protocol: i32,
        ai_addrlen: usize,
        ai_canonname: *mut c_char,
        ai_addr: *mut sockaddr,
        ai_next: *mut AddrInfoA,
    }

    #[repr(C)]
    struct sockaddr {
        sa_family: u16,
        sa_data: [u8; 14],
    }

    #[repr(C)]
    struct sockaddr_in {
        sin_family: u16,
        sin_port: u16,
        sin_addr: u32,
        sin_zero: [u8; 8],
    }

    extern "system" {
        fn getaddrinfo(
            node: *const c_char,
            service: *const c_char,
            hints: *const AddrInfoA,
            result: *mut *mut AddrInfoA,
        ) -> i32;
        fn freeaddrinfo(res: *mut AddrInfoA);
        fn connect(s: usize, name: *const sockaddr, namelen: i32) -> i32;
        fn closesocket(s: usize) -> i32;
        fn socket(af: i32, sock_type: i32, protocol: i32) -> usize;
        fn htons(hostshort: u16) -> u16;
        fn inet_addr(cp: *const c_char) -> u32;
        fn setsockopt(s: usize, level: i32, optname: i32, optval: *const u8, optlen: i32) -> i32;
        fn ioctlsocket(s: usize, cmd: i32, argp: *mut u32) -> i32;
    }

    const SOL_SOCKET: i32 = 0xffff;
    const SO_RCVTIMEO: i32 = 0x1006;
    const SO_SNDTIMEO: i32 = 0x1005;
    const FIONREAD: i32 = 0x4004667f;

    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    /// Convierte un host (IP o nombre) + puerto a un socket conectado.
    /// Devuelve el socket handle o 0 si falla.
    pub unsafe fn tcp_conectar(host: *const c_char, puerto: i32) -> usize {
        if host.is_null() {
            return 0;
        }

        // Inicializar Winsock si es necesario (ya debería estar inicializado por tcp_vincular)
        // Pero por seguridad, lo inicializamos aquí también
        let mut wsa_data = [0u8; 408];
        let version: u16 = 0x0202;
        extern "system" { fn WSAStartup(wVersionRequired: u16, lpWSAData: *mut u8) -> i32; }
        WSAStartup(version, wsa_data.as_mut_ptr());

        // Preparar hints para getaddrinfo
        let mut hints: AddrInfoA = std::mem::zeroed();
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;
        hints.ai_protocol = IPPROTO_TCP;

        // Convertir puerto a string para getaddrinfo
        let puerto_str = format!("{}\0", puerto);
        let puerto_ptr = puerto_str.as_ptr() as *const c_char;

        let mut result: *mut AddrInfoA = ptr::null_mut();
        let ret = getaddrinfo(host, puerto_ptr, &hints, &mut result);
        if ret != 0 || result.is_null() {
            return 0;
        }

        // Iterar resultados hasta encontrar uno que funcione
        let mut sock = INVALID_SOCKET;
        let mut curr = result;
        while !curr.is_null() {
            sock = socket((*curr).ai_family, (*curr).ai_socktype, (*curr).ai_protocol);
            if sock == INVALID_SOCKET {
                curr = (*curr).ai_next;
                continue;
            }

            if connect(sock, (*curr).ai_addr, (*curr).ai_addrlen as i32) == 0 {
                break; // Conectado
            }

            closesocket(sock);
            sock = INVALID_SOCKET;
            curr = (*curr).ai_next;
        }

        freeaddrinfo(result);

        if sock == INVALID_SOCKET {
            0
        } else {
            sock
        }
    }

    /// Resuelve un nombre de host a una dirección IP (string).
    /// Devuelve buffer malloc'ed con null terminator (caller libera con free) o NULL.
    pub unsafe fn dns_resolver(host: *const c_char) -> *mut c_char {
        if host.is_null() {
            return ptr::null_mut();
        }

        // Inicializar Winsock
        let mut wsa_data = [0u8; 408];
        let version: u16 = 0x0202;
        extern "system" { fn WSAStartup(wVersionRequired: u16, lpWSAData: *mut u8) -> i32; }
        WSAStartup(version, wsa_data.as_mut_ptr());

        let mut hints: AddrInfoA = std::mem::zeroed();
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;

        let mut result: *mut AddrInfoA = ptr::null_mut();
        let ret = getaddrinfo(host, ptr::null(), &hints, &mut result);
        if ret != 0 || result.is_null() {
            return ptr::null_mut();
        }

        // Tomar la primera dirección
        let addr = (*result).ai_addr;
        if addr.is_null() {
            freeaddrinfo(result);
            return ptr::null_mut();
        }

        // Extraer IP de sockaddr_in
        let sin = &*(addr as *const sockaddr_in);
        let ip_bytes = sin.sin_addr.to_ne_bytes();
        let ip_str = format!("{}.{}.{}.{}\0", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);

        freeaddrinfo(result);

        // Copiar a buffer malloc'ed
        let len = ip_str.len();
        let buf = malloc(len) as *mut u8;
        if buf.is_null() {
            return ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(ip_str.as_ptr(), buf, len);
        buf as *mut c_char
    }

    /// Establece timeout de lectura/escritura en milisegundos.
    pub unsafe fn tcp_establecer_timeout(sock: usize, ms: i32) {
        if sock == 0 || sock == usize::MAX {
            return;
        }
        let timeout = ms as u32;
        let timeout_ptr = &timeout as *const u32 as *const u8;
        let timeout_len = std::mem::size_of::<u32>() as i32;
        setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, timeout_ptr, timeout_len);
        setsockopt(sock, SOL_SOCKET, SO_SNDTIMEO, timeout_ptr, timeout_len);
    }

    /// Verifica si hay datos disponibles para leer sin bloquear.
    /// Devuelve 1 si hay datos, 0 si no.
    pub unsafe fn tcp_datos_disponibles(sock: usize) -> i32 {
        if sock == 0 || sock == usize::MAX {
            return 0;
        }
        let mut bytes_available: u32 = 0;
        let ret = ioctlsocket(sock, FIONREAD, &mut bytes_available);
        if ret == 0 && bytes_available > 0 {
            1
        } else {
            0
        }
    }
}

// ============================================================
// POSIX (Linux/macOS)
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;
    use std::ffi::c_char;
    use std::ptr;

    const AF_INET: i32 = 2;
    const SOCK_STREAM: i32 = 1;
    const IPPROTO_TCP: i32 = 6;

    #[repr(C)]
    struct AddrInfo {
        ai_flags: i32,
        ai_family: i32,
        ai_socktype: i32,
        ai_protocol: i32,
        ai_addrlen: u32,
        ai_addr: *mut sockaddr,
        ai_canonname: *mut c_char,
        ai_next: *mut AddrInfo,
    }

    #[repr(C)]
    struct sockaddr {
        sa_family: u16,
        sa_data: [u8; 14],
    }

    #[repr(C)]
    struct sockaddr_in {
        sin_family: u16,
        sin_port: u16,
        sin_addr: u32,
        sin_zero: [u8; 8],
    }

    extern "C" {
        fn getaddrinfo(
            node: *const c_char,
            service: *const c_char,
            hints: *const AddrInfo,
            result: *mut *mut AddrInfo,
        ) -> i32;
        fn freeaddrinfo(res: *mut AddrInfo);
        fn connect(sockfd: i32, addr: *const sockaddr, addrlen: u32) -> i32;
        fn close(fd: i32) -> i32;
        fn socket(domain: i32, sock_type: i32, protocol: i32) -> i32;
        fn htons(hostshort: u16) -> u16;
        fn inet_addr(cp: *const c_char) -> u32;
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
        fn setsockopt(sockfd: i32, level: i32, optname: i32, optval: *const u8, optlen: u32) -> i32;
        fn ioctl(fd: i32, request: u64, argp: *mut u32) -> i32;
    }

    const SOL_SOCKET: i32 = 1;
    const SO_RCVTIMEO: i32 = 20;
    const SO_SNDTIMEO: i32 = 21;
    const FIONREAD: u64 = 0x541b; // Linux

    pub unsafe fn tcp_conectar(host: *const c_char, puerto: i32) -> i64 {
        if host.is_null() {
            return 0;
        }

        let mut hints: AddrInfo = std::mem::zeroed();
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;
        hints.ai_protocol = IPPROTO_TCP;

        let puerto_str = format!("{}\0", puerto);
        let puerto_ptr = puerto_str.as_ptr() as *const c_char;

        let mut result: *mut AddrInfo = ptr::null_mut();
        let ret = getaddrinfo(host, puerto_ptr, &hints, &mut result);
        if ret != 0 || result.is_null() {
            return 0;
        }

        let mut sock = -1i32;
        let mut curr = result;
        while !curr.is_null() {
            sock = socket((*curr).ai_family, (*curr).ai_socktype, (*curr).ai_protocol);
            if sock < 0 {
                curr = (*curr).ai_next;
                continue;
            }

            if connect(sock, (*curr).ai_addr, (*curr).ai_addrlen) == 0 {
                break;
            }

            close(sock);
            sock = -1;
            curr = (*curr).ai_next;
        }

        freeaddrinfo(result);

        sock as i64
    }

    pub unsafe fn dns_resolver(host: *const c_char) -> *mut c_char {
        if host.is_null() {
            return ptr::null_mut();
        }

        let mut hints: AddrInfo = std::mem::zeroed();
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;

        let mut result: *mut AddrInfo = ptr::null_mut();
        let ret = getaddrinfo(host, ptr::null(), &hints, &mut result);
        if ret != 0 || result.is_null() {
            return ptr::null_mut();
        }

        let addr = (*result).ai_addr;
        if addr.is_null() {
            freeaddrinfo(result);
            return ptr::null_mut();
        }

        let sin = &*(addr as *const sockaddr_in);
        let ip_bytes = sin.sin_addr.to_ne_bytes();
        let ip_str = format!("{}.{}.{}.{}\0", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);

        freeaddrinfo(result);

        let len = ip_str.len();
        let buf = malloc(len) as *mut u8;
        if buf.is_null() {
            return ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(ip_str.as_ptr(), buf, len);
        buf as *mut c_char
    }

    /// Establece timeout de lectura/escritura en milisegundos.
    pub unsafe fn tcp_establecer_timeout(sock: i64, ms: i32) {
        if sock <= 0 {
            return;
        }
        let s = sock as i32;
        // struct timeval { tv_sec, tv_usec }
        let sec = ms / 1000;
        let usec = (ms % 1000) * 1000;
        let mut timeout: [i32; 2] = [sec, usec];
        let timeout_ptr = timeout.as_mut_ptr() as *const u8;
        let timeout_len = std::mem::size_of::<[i32; 2]>() as u32;
        setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, timeout_ptr, timeout_len);
        setsockopt(s, SOL_SOCKET, SO_SNDTIMEO, timeout_ptr, timeout_len);
    }

    /// Verifica si hay datos disponibles para leer sin bloquear.
    /// Devuelve 1 si hay datos, 0 si no.
    pub unsafe fn tcp_datos_disponibles(sock: i64) -> i32 {
        if sock <= 0 {
            return 0;
        }
        let s = sock as i32;
        let mut bytes_available: u32 = 0;
        let ret = ioctl(s, FIONREAD, &mut bytes_available);
        if ret == 0 && bytes_available > 0 {
            1
        } else {
            0
        }
    }
}

pub use imp::*;
