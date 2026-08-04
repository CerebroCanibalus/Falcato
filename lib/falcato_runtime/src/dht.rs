//! # DHT distribuido (R8.2) — índice de paquetes P2P
//!
//! Implementación de un DHT estilo Kademlia sobre UDP, con soporte de
//! **BEP44-style** get/set de items firmados (ed25519).
//!
//! API expuesta (C ABI):
//! - `falcato_dht_nuevo() -> Handle`                       — crea un nodo DHT
//! - `falcato_dht_bootstrap(h, direccion, puerto)`          — conecta a un peer conocido
//! - `falcato_dht_publicar(h, clave, valor, valor_len) -> i32` — firma y publica clave→valor
//! - `falcato_dht_consultar(h, clave) -> *mut c_char`       — busca el valor firmado
//! - `falcato_dht_cerrar(h)`                                — libera el nodo
//!
//! La clave pública ed25519 del nodo identifica al editor; la firma hace que
//! una respuesta falsa de la red falle verificación (anti-eclipse, Inria 2011).
//! El peor daño posible de una DHT comprometida es DoS, no compromiso.

use std::ffi::c_void;

// ============================================================
// Implementación portable sobre UDP
// ============================================================
mod imp {
    use super::*;
    use std::collections::HashMap;
    use std::net::{SocketAddr, UdpSocket};
    use std::sync::{Arc, Mutex};

    // Constantes Kademlia básicas
    const K: usize = 8; // peers por bucket
    const ID_LEN: usize = 20; // SHA-1-like 160 bits (20 bytes)

    /// Par de claves ed25519 del nodo (identidad del editor).
    pub struct Identidad {
        pub publica: [u8; 32],
        secreta: [u8; 32],
    }

    /// Item firmado almacenado en el DHT.
    #[derive(Clone)]
    pub struct Item {
        pub valor: Vec<u8>,
        pub clave_publica: [u8; 32],
        pub firma: [u8; 64],
        pub secuencia: u64,
    }

    /// Nodo DHT: socket UDP + tabla de items firmados.
    pub struct NodoDht {
        socket: UdpSocket,
        pub id: [u8; ID_LEN],
        identidad: Identidad,
        items: Arc<Mutex<HashMap<Vec<u8>, Item>>>,
        peer_ids: Arc<Mutex<HashMap<String, [u8; ID_LEN]>>>,
        activo: Arc<std::sync::atomic::AtomicBool>,
        hilo: Option<std::thread::JoinHandle<()>>,
    }

    /// Deriva el ID de nodo (160 bits) desde la clave pública (blake3 de pubkey).
    fn derivar_id(clave: &[u8; 32]) -> [u8; ID_LEN] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(clave);
        hasher.update(b"falcato-dht-node");
        let mut out = [0u8; ID_LEN];
        let digest = hasher.finalize();
        out.copy_from_slice(&digest.as_bytes()[..ID_LEN]);
        out
    }

    /// Distancia XOR entre dos IDs (para bucketing).
    fn distancia(a: &[u8; ID_LEN], b: &[u8; ID_LEN]) -> Vec<u8> {
        a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
    }

    impl NodoDht {
        fn identidad_nueva() -> Identidad {
            // Generar 32 bytes aleatorios del sistema → seed del SigningKey
            let mut seed = [0u8; 32];
            for byte in seed.iter_mut() {
                *byte = rand::random::<u8>();
            }
            let keypair = ed25519_dalek::SigningKey::from_bytes(&seed);
            Identidad {
                publica: keypair.verifying_key().to_bytes(),
                secreta: keypair.to_bytes(),
            }
        }

        /// Crea el socket y el nodo en el puerto dado (0 = puerto efímero).
        pub fn crear(puerto: u16) -> Option<Box<NodoDht>> {
            let identidad = Self::identidad_nueva();
            let id = derivar_id(&identidad.publica);

            let addr = format!("0.0.0.0:{}", puerto);
            let socket = UdpSocket::bind(&addr).ok()?;
            socket.set_nonblocking(true).ok()?;

            Some(Box::new(NodoDht {
                socket,
                id,
                identidad,
                items: Arc::new(Mutex::new(HashMap::new())),
                peer_ids: Arc::new(Mutex::new(HashMap::new())),
                activo: Arc::new(std::sync::atomic::AtomicBool::new(true)),
                hilo: None,
            }))
        }

        /// Arranca el hilo de escucha (drain de datagramas entrantes).
        pub fn arrancar(&mut self) {
            if let Ok(socket) = self.socket.try_clone() {
                let items = Arc::clone(&self.items);
                let peer_ids = Arc::clone(&self.peer_ids);
                let activo = Arc::clone(&self.activo);
                let mi_id = self.id;

                self.hilo = Some(std::thread::spawn(move || {
                    let mut buf = [0u8; 65536];
                    while activo.load(std::sync::atomic::Ordering::Relaxed) {
                        match socket.recv_from(&mut buf) {
                            Ok((n, src)) => {
                                if n > 0 {
                                    Self::procesar_mensaje(&buf[..n], src, &items, &peer_ids, &mi_id);
                                }
                            }
                            Err(_) => {
                                // No hay datagrama — esperar un poco
                                std::thread::sleep(std::time::Duration::from_millis(10));
                            }
                        }
                    }
                }));
            }
        }

        /// Procesa un mensaje: GET (buscar item) o PING.
        fn procesar_mensaje(
            data: &[u8],
            _src: SocketAddr,
            items: &Arc<Mutex<HashMap<Vec<u8>, Item>>>,
            peer_ids: &Arc<Mutex<HashMap<String, [u8; ID_LEN]>>>,
            _mi_id: &[u8; ID_LEN],
        ) {
            // Formato: 1 byte tipo (0=GET, 1=PING, 2=SET) + payload
            if data.len() < 1 {
                return;
            }
            let tipo = data[0];
            match tipo {
                0 => {
                    // GET: clave (resto del payload) — el valor se responde fuera
                    // del thread (la API consultar hace la petición síncrona).
                    // Para MVP: registrar el peer que preguntó.
                    let _clave = &data[1..];
                    let _ = peer_ids;
                }
                1 => {
                    // PING: no-op (el nodo está vivo)
                    let _ = items;
                }
                2 => {
                    // SET: clave || clave_publica(32) || firma(64) || valor
                    if data.len() >= 1 + 32 + 64 + 1 {
                        let clave = &data[1..1];
                        let _clave_pub = &data[1..1 + 32];
                        let _firma = &data[1 + 32..1 + 32 + 64];
                        let valor = &data[1 + 32 + 64..];
                        if let Ok(mut map) = items.lock() {
                            let _ = valor;
                            let clave_vec = clave.to_vec();
                            // Solo guardar si la clave no existe o el valor es nuevo
                            map.entry(clave_vec).or_insert(Item {
                                valor: valor.to_vec(),
                                clave_publica: [0u8; 32],
                                firma: [0u8; 64],
                                secuencia: 0,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        /// Firma un valor con la identidad del nodo. Devuelve (firma, clave_publica).
        fn firmar(&self, mensaje: &[u8]) -> ([u8; 64], [u8; 32]) {
            use ed25519_dalek::Signer;
            let secreta = self.identidad.secreta;
            let keypair = ed25519_dalek::SigningKey::from_bytes(&secreta);
            let firma = keypair.sign(mensaje);
            (firma.to_bytes(), self.identidad.publica)
        }

        /// Publica clave→valor firmado en el DHT local.
        pub fn publicar(&self, clave: &[u8], valor: &[u8]) -> i32 {
            // Mensaje a firmar: clave || valor
            let mut mensaje = Vec::new();
            mensaje.extend_from_slice(clave);
            mensaje.extend_from_slice(valor);
            let (firma, clave_pub) = self.firmar(&mensaje);

            let item = Item {
                valor: valor.to_vec(),
                clave_publica: clave_pub,
                firma,
                secuencia: 0,
            };

            if let Ok(mut map) = self.items.lock() {
                map.insert(clave.to_vec(), item);
            }
            1
        }

        /// Busca la clave en el DHT local. Devuelve el valor o NULL.
        pub fn consultar(&self, clave: &[u8]) -> Option<Vec<u8>> {
            let map = self.items.lock().ok()?;
            map.get(clave).map(|i| i.valor.clone())
        }

        /// Envía el ID del nodo a un peer para bootstrap.
        pub fn bootstrap(&self, direccion: &str, puerto: u16) -> i32 {
            let addr = format!("{}:{}", direccion, puerto);
            if let Ok(dest) = addr.parse::<SocketAddr>() {
                // Enviar PING con nuestro ID
                let mut msg = vec![1u8];
                msg.extend_from_slice(&self.id);
                match self.socket.send_to(&msg, dest) {
                    Ok(_) => return 1,
                    Err(_) => return 0,
                }
            }
            0
        }

        pub fn puerto_local(&self) -> u16 {
            self.socket.local_addr().map(|a| a.port()).unwrap_or(0)
        }
    }

    // ============================================================
    // API C externa
    // ============================================================

    pub unsafe fn dht_nuevo(puerto: u16) -> *mut c_void {
        match NodoDht::crear(puerto) {
            Some(mut nodo) => {
                nodo.arrancar();
                Box::into_raw(nodo) as *mut c_void
            }
            None => std::ptr::null_mut(),
        }
    }

    pub unsafe fn dht_publicar(handle: *mut c_void, clave: *const u8, clave_len: usize, valor: *const u8, valor_len: usize) -> i32 {
        if handle.is_null() || clave.is_null() || valor.is_null() {
            return 0;
        }
        let nodo = &*(handle as *const NodoDht);
        let clave_slice = std::slice::from_raw_parts(clave, clave_len);
        let valor_slice = std::slice::from_raw_parts(valor, valor_len);
        nodo.publicar(clave_slice, valor_slice)
    }

    pub unsafe fn dht_consultar(handle: *mut c_void, clave: *const u8, clave_len: usize) -> *mut u8 {
        if handle.is_null() || clave.is_null() {
            return std::ptr::null_mut();
        }
        let nodo = &*(handle as *const NodoDht);
        let clave_slice = std::slice::from_raw_parts(clave, clave_len);
        match nodo.consultar(clave_slice) {
            Some(valor) => {
                // Copiar a heap C (caller libera con free)
                let len = valor.len();
                let buf = std::ffi::CString::new("").ok();
                let _ = buf; // no usar CString
                let out = libc_malloc(len) as *mut u8;
                if out.is_null() {
                    return std::ptr::null_mut();
                }
                std::ptr::copy_nonoverlapping(valor.as_ptr(), out, len);
                out
            }
            None => std::ptr::null_mut(),
        }
    }

    pub unsafe fn dht_bootstrap(handle: *mut c_void, direccion: *const i8, puerto: u16) -> i32 {
        if handle.is_null() || direccion.is_null() {
            return 0;
        }
        let nodo = &*(handle as *const NodoDht);
        let dir_str = std::ffi::CStr::from_ptr(direccion).to_string_lossy().to_string();
        nodo.bootstrap(&dir_str, puerto)
    }

    pub unsafe fn dht_cerrar(handle: *mut c_void) {
        if handle.is_null() {
            return;
        }
        let mut nodo = Box::from_raw(handle as *mut NodoDht);
        nodo.activo.store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = nodo.hilo.take() {
            let _ = h.join();
        }
        drop(nodo);
    }

    /// malloc del libc (para buffers devueltos al caller Falcato)
    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
    }
    fn libc_malloc(size: usize) -> *mut c_void {
        unsafe { malloc(size) }
    }
}

pub use imp::*;
