//! # Tiempo — reloj del sistema (Unix epoch) para fechas legibles
//!
//! API expuesta (C ABI):
//! - `falcato_fecha_unix() -> i64` — segundos desde Unix epoch (1970-01-01 UTC)
//! - `falcato_fecha_ms() -> i64`   — milisegundos desde Unix epoch
//!
//! Diferencia con el builtin `timestamp` (GetTickCount64): ese mide ms desde
//! el arranque del sistema (monótono, útil para medir intervalos). Estos
//! devuelven el reloj de pared real, necesario para sesiones y logs legibles.

use std::ffi::c_void;

// ============================================================
// Windows
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;

    const EPOCH_DIFF: i64 = 116444736000000000; // 1601-01-01 → 1970-01-01 en 100ns

    extern "system" {
        fn GetSystemTimeAsFileTime(lp_file_time: *mut i64);
    }

    pub unsafe fn fecha_unix() -> i64 {
        // time(NULL) del CRT — más simple y fiable
        extern "C" {
            fn time(timer: *mut i64) -> i64;
        }
        time(std::ptr::null_mut())
    }

    pub unsafe fn fecha_ms() -> i64 {
        let mut ft: i64 = 0;
        GetSystemTimeAsFileTime(&mut ft);
        // 100ns units → ms, y compensar epoch
        (ft - EPOCH_DIFF) / 10_000
    }
}

// ============================================================
// POSIX (Linux/macOS)
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    pub unsafe fn fecha_unix() -> i64 {
        extern "C" {
            fn time(timer: *mut i64) -> i64;
        }
        time(std::ptr::null_mut())
    }

    pub unsafe fn fecha_ms() -> i64 {
        extern "C" {
            fn clock_gettime(clk_id: i32, tp: *mut Timespec) -> i32;
        }
        #[repr(C)]
        struct Timespec {
            tv_sec: i64,
            tv_nsec: i64,
        }
        let mut ts: Timespec = Timespec { tv_sec: 0, tv_nsec: 0 };
        // CLOCK_REALTIME = 0
        clock_gettime(0, &mut ts);
        ts.tv_sec * 1000 + ts.tv_nsec / 1_000_000
    }
}

pub use imp::*;
