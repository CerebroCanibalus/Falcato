# Falcato — Documento del Agente

> Este archivo registra decisiones de diseño para el agente (no para el usuario final).
> Todo cambio grande debe anotarse aquí + en `skill falcato-language` en la misma tanda (Day-0).

## F1 — Nido Local (2026-08-22) — COMPLETADO

**Nombre:** `fardo` (nido = proyecto). Comandos: `falcato fardo nuevo/agregar/lista/arbol` (alias `paquete`).

**Arquitectura modular `src/fardo/`:**
- `modelo.rs` — newtypes `NombreFardo`/`Version`/`FardoId`/`Dependencia`/`Origen`, `Restriccion` caret, `Bloqueo`, `Valoracion` (Capa 11 modelada)
- `validacion.rs` — kebab-case, semver, hash hex64, `ruta_segura`/`unir_ruta_segura` (anti ZipSlip)
- `error.rs` — `thiserror` `[F001..F020]` en español
- `manifiesto.rs` — `falcato.toml` IO atómico (`.tmp`+rename), alias `[fardos]`/`[dependencias]`, `iniciar_proyecto`, `agregar_ruta`
- `bloqueo.rs` — `falcato.lock` IO atómico, orden determinista, validación hash
- `fuentes/mod.rs` + `ruta.rs` + `registro.rs` — trait `Fuente` (escalable a DHT F2), `FuenteRuta` permite `C:\` y `../` pero rechaza `/etc`
- `resolver.rs` — DFS + ciclo `[F010]` + orden topológico + límite 50 profundidad, `colectar_fuentes()` para `verifica`/`compila`
- `cache.rs` — `.falcato/cache_resolucion.bin` hash 016x
- `formato.rs` — tablas cetrera `lista`/`arbol` (sin deps externas, F2 migrará a comfy-table)

**CLI:**
- `fardo nuevo <dir> --nombre <n>` + `paquete` alias
- `fardo agregar <nombre> --ruta <path> [version] [dir]` — actualiza `falcato.lock` vía `resolver_nido`
- `fardo lista [dir]` / `fardo arbol [dir]` — tablas con `◆`/`✓`
- `verifica`/`compila` → `expandir_con_fardos()` (fardos primero, luego principal)

**Seguridad F1:**
- Escritura atómica, canonicalize, rechazo POSIX `/etc`, límite 100 fardos / 200KB TOML / 50 profundidad, `checked` semver.

**Tests:** 29/29 fardo + 54 previos = 83/83 `cargo test` ok. `cargo check` 0 errores. `cargo clippy` fardo 0 warnings (legado 127 warnings preexistentes documentados como deuda).

**Criterio F1:** nido 500 líneas multi-fardo verificado y compilado offline con `path` — **cumplido** (smoke test `util::hola()`).

**Próximo:** F2 Bandada DHT (BEP44 + QUIC, `fardo buscar/publicar`).

---

## LibEst — Librería Estándar de Falcato (2026-08-28)

### Arquitectura

La libEst sigue el patrón **híbrido**: builtins en Rust (FFI, rendimiento) + wrappers en Falcato (.fc) para la API pública.

```
┌─────────────────────────────────────┐
│  Falcato (.fc)                      │  ← API pública en español
│  texto_contiene(), http_get(), etc. │
├─────────────────────────────────────┤
│  Cranelift wrappers                 │  ← codegen/builtins/*.rs
│  builtin_texto_contiene(), etc.     │
├─────────────────────────────────────┤
│  Runtime Rust (staticlib)           │  ← lib/falcato_runtime/src/
│  falcato_texto_contiene(), etc.     │
├─────────────────────────────────────┤
│  C / OS APIs                        │  ← Winsock, POSIX, GDI+
└─────────────────────────────────────┘
```

### Módulos de la libEst

```
libEst/
├── nucleo/
│   ├── texto.fc        (26 funciones) — strings, búsqueda, transformación, base64
│   ├── numeros.fc      (14 funciones) — abs, max, min, raíz, trig, log
│   └── opcion.fc       (4 funciones)  — pattern matching Option/Resultado
├── colecciones/
│   ├── vector.fc       (16 funciones) — CRUD, búsqueda, clonar, invertir
│   ├── diccionario.fc  (10 funciones) — CRUD, claves, valores, limpiar
│   └── conjunto.fc     (7 funciones)  — CRUD, contiene, elementos
├── archivo/
│   └── archivo.fc      (8 funciones)  — leer, escribir, existe, borrar, listar, tamaño
├── red/
│   ├── tcp.fc          (10 funciones) — conectar, enviar, recibir, DNS, servidor
│   ├── http.fc         (2 funciones)  — GET, POST (TCP + HTTP/1.1)
│   └── json.fc         (9 funciones)  — parsear, serializar, escapar, obtener
├── tiempo/
│   └── tiempo.fc       (5 funciones)  — reloj, fecha Unix, componentes
├── proceso/
│   └── proceso.fc      (8 funciones)  — crear, esperar, hilos, canales
├── sistema/
│   └── sistema.fc      (7 funciones)  — entorno, argumentos, consola, aleatorio
├── matematicas/
│   └── matematicas.fc  (20 funciones) — trig, log, piso, techo, pi, e
├── visual/
│   ├── ventana.fc      (11 funciones) — Win32, CreateWindow, mensajes
│   ├── color.fc        (7 funciones)  — RGB, hex, mezclar
│   ├── lienzo.fc       (8 funciones)  — Canvas 2D, GDI
│   ├── imagen.fc       (6 funciones)  — carga, redimensiona, guarda
│   └── sonido.fc       (8 funciones)  — WAV, tono, mezcla, fade
└── compat/
    └── compat.fc       — aliases de builtins viejos (P-005)
```

### Builtins Rust (153 funciones)

| Categoría | Builtins | Runtime | Estado |
|-----------|----------|---------|--------|
| Texto | 22 | `texto_dinamico.rs` | ✅ |
| HTTP | 2 | `http.rs` | ✅ |
| JSON | 4 | `json.rs` | ✅ |
| TCP | 10 | `tcp_cliente.rs` | ✅ |
| TLS | 5 | `tls.rs` | ✅ |
| Archivo | 8 | `archivo_avanzado.rs` | ✅ |
| Tiempo | 5 | `archivo_avanzado.rs` | ✅ |
| Proceso | 8 | `proceso.rs` | ✅ |
| Sistema | 7 | `sistema.rs` | ✅ |
| Matemáticas | 14 | `math.rs` | ✅ |
| Vector | 15 | `vector.rs` | ✅ |
| Diccionario | 10 | `diccionario.rs` | ✅ |
| Conjunto | 7 | `diccionario.rs` | ✅ |
| Opción/Resultado | 4 | `opcion.rs` | ✅ |
| Visual | 31 | `visual.rs` | ✅ (stub imagen/sonido) |

### Falcato puro (16 funciones)

Constructores triviales, constantes, aritmética simple, composiciones de builtins.

### Namespace `::`

El parser soporta `mod::func` desde v0.7.5. Ejemplo:
```falcato
usar archivo::*;
el contenido: Texto = archivo_leer("datos.txt");
el largo: Entero32 = texto_longitud(contenido);
```

### Decisiones de diseño

- **P-001 RESUELTO:** "tipos fragmentados + verbos consistentes + namespaces explícitos"
- **P-002 RESUELTO:** `::` es el separador de namespace
- **P-005:** Migración gradual de builtins a stdlib (aliases en `compat`)
- **P-017:** Regla unificadora de API — misma familia = mismos tipos

---

## Decisiones Globales

- **fardo vs paquete:** `fardo` canónico, `paquete` alias compat.
- **Sección TOML:** `[fardos]` canónica, `[dependencias]` alias con aviso futuro.
- **Origen path:** permitir `C:\` absoluta (usuario explícito), rechazar `/` POSIX.
- **Reputación (Capa 11) modelada pero no implementada hasta F2.**
