//! `dormir(ms)` en POSIX — conversión correcta de milisegundos.
//!
//! Windows mapea el builtin a `Sleep(ms)`, que ya espera milisegundos.
//! En macOS/Linux el registro apuntaba a `usleep`, que espera MICROsegundos:
//! `dormir(1000)` dormía ~1 ms en vez de 1 s. Además `usleep` rechaza esperas
//! de 1 segundo o más (`EINVAL`), así que multiplicar por mil tampoco alcanza.
//!
//! Esta función recibe los mismos milisegundos que `Sleep` y usa `nanosleep`,
//! que acepta cualquier duración y devuelve el tiempo restante si una señal
//! interrumpe la espera.

use std::io::Error;

/// `struct timespec` de POSIX (64-bit: `time_t` y `long` son i64).
#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

extern "C" {
    fn nanosleep(peticion: *const Timespec, sobrante: *mut Timespec) -> i32;
}

/// Interrupción por señal: hay que reintentar con el tiempo restante.
const EINTR: i32 = 4;

/// Dormir `ms` milisegundos (misma semántica que `Sleep` de Windows).
///
/// ABI fija: el codegen la invoca como `void falcato_dormir_ms(i32)`.
#[no_mangle]
pub unsafe extern "C" fn falcato_dormir_ms(ms: i32) {
    if ms <= 0 {
        return;
    }

    let mut peticion = Timespec {
        tv_sec: (ms / 1000) as i64,
        tv_nsec: ((ms % 1000) as i64) * 1_000_000,
    };

    loop {
        let mut sobrante = Timespec { tv_sec: 0, tv_nsec: 0 };
        let resultado = nanosleep(&peticion, &mut sobrante);
        if resultado == 0 {
            return;
        }

        // Sólo se reintenta si una señal interrumpió la espera; cualquier otro
        // error (duración inválida) termina para no quedarse colgado.
        let errno = Error::last_os_error().raw_os_error().unwrap_or(0);
        if errno != EINTR || (sobrante.tv_sec == 0 && sobrante.tv_nsec == 0) {
            return;
        }
        peticion = sobrante;
    }
}
