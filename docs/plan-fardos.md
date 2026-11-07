# Plan Canónico — Fardos (Ecosistema P2P de Falcato)

> **Estado:** CANÓNICO v1.0 — 2026-08-22  
> **Autor:** General Beria + Cangrejo (Rust Systems Engineer)  
> **Nombre distintivo:** `fardo` (plural `fardos`). El proyecto es el `nido`.  
> **Visión:** Falcato + Cranelift + WASM + Bandada P2P = toolchain para código generado por IA. Velocidad de compilación > velocidad de ejecución. Libertad absoluta sin servidor central.

---

## 1. Filosofía — Por qué no `paquetes` ni `crates`

`paquete` es de Correos. `crate` es cajón gringo. Falcato es cetrería: el halcón guarda sus presas en el **fardo** (bulto de tela del halconero) y el proyecto es el **nido** tejido con plumas (fardos). Es hispano, corto (5 letras), sin tilde, intuitivo:

```bash
falcato fardo nuevo mi-web
falcato fardo agregar general_beria/json@0.1
falcato fardo buscar json
falcato fardo info general_beria/json
falcato fardo publicar
falcato fardo espejo --sembrar
```

`falcato.toml` usa sección `[fardos]` (alias `[dependencias]` tolerado con aviso `[W]`):

```toml
[paquete]
nombre = "mi-web"
version = "0.1.0"
edicion = "2024"

[fardos]
json = { origen = "pubkey:a1b2...d4e5", version = "0.1" }
red  = "pubkey:c3f8...9a1b/red@0.3"
```

---

## 2. Arquitectura — P2P Nativo, no Híbrido Tímido

```
  Tu clave ed25519 (identidad soberana, ~/.falcato/clave.priv)
        │
        ▼
┌──────────────┐  BEP44 PUT firmado (seq+hash+magnet)  ┌─────────────────┐
│ fardo        │ ─────────────────────────────────────► │  DHT Mainline   │
│ publicar     │  {nombre, versión, sha256, magnet}    │  (200k nodos)   │
└──────────────┘                                       └────────┬────────┘
                                                               │ GET 320ms
                                                               ▼
┌──────────────┐  QUIC 0-RTT (<256KB) / BitTorrent v2  ┌─────────────────┐
│ fardo        │ ◄──────────────────────────────────── │  Swarm Peers    │
│ instalar     │  .tar.zst + .o cache por infohash     │  (vecinos)      │
└──────────────┘                                       └─────────────────┘
        ▲
        │ falcato.lock = lista de infohash + firmas → verificación local
```

* **Sin nombres globales.** `general_beria/json` y `cangrejo/json` coexisten. Tu `falcato.toml` mapea alias local `json` → `pubkey`. Se acabó el squatting. Sistema `petnames` como Torrent/SSB.
* **Sin servidor central.** Publicar es `DHT PUT`, no `POST https://crates.io`. Bootstrap solo para encontrar primer peer (`bootstrap.falcato.org` lista de 3 nodos, luego puro P2P).
* **Content-addressed.** Artefacto = `sha256`. `falcato://a1b2.../json@0.1.0` verificable offline, pasable por QR.
* **Cache `.o` P2P.** `hash(fuente + versión + --destino + flags)` → si tu vecino lo compiló, lo bajas en 50ms. `sccache` descentralizado.

**Stack sin reinventar la rueda:** `mainline` (DHT), `ed25519-dalek`, `quinn` (QUIC), `sha2`, `pubgrub` (resolver), `comfy-table`+`owo-colors`+`indicatif` (TUI).

---

## 3. Roadmap en 3 Fases — Coherencia antes que prisa

| Fase | Ventana | Objetivo | Criterio de Éxito Honesto |
|------|---------|----------|---------------------------|
| **F1 — Nido Local** | 10 días | `fardo nuevo/agregar/lista/arbol` con `path`, firma local, `falcato.lock` con hash, cache `.o` local | Proyecto 500 líneas multi-fardo compila offline, `verifica` resuelve 2 fardos locales |
| **F2 — Bandada DHT** | 21 días | `clave generar`, `publicar` (BEP44), `buscar/info` (GET), `agregar` vía QUIC | Dos PCs con NAT distinto se pasan un fardo sin GitHub, `buscar` <500ms |
| **F3 — Vuelo Compartido** | 21 días | `espejo --sembrar`, cache `.o` P2P, `arbol` con deduplicación | 2ª compilación 20 fardos <1s (baja `.o` del swarm) |

---

## 4. Sistema Visual — Terminal Cetrera

Principios: español, densidad alta escaneable, color con semántica, <100ms, sin Nerd Fonts obligatorio.

**`fardo buscar json`:**
```
  ⠀⠀⠀⣏⡱ ⣏⡉ ⣏⡱ ⡇ ⣎⣱   FALCATO — Bandada
  Buscando "json" en 214 nodos... 12 fardos en 0.31s
  ─────────────────────────────────────────────────────────────────
   NOMBRE                VERS  AUTOR     ★ TRU   DESCRIPCIÓN         SEMB
  ─────────────────────────────────────────────────────────────────
 ► general_beria/json   0.1.0 a1b2…d4e5 ★★★★★ 12/12 JSON mínimo FC    47 ◆
   cangrejo/json        0.2.1 c3f8…9a1b ★★☆☆☆  2/12 Fork veloz         3 ◇
  ─────────────────────────────────────────────────────────────────
```

**`fardo info`:** caja con hash, magnet `falcato://`, sembradores, dependencias, `✓ firma válida`.  
**`fardo arbol`:** árbol deduplicado como `cargo tree`.  
**`--interactivo`:** TUI `ratatui` tipo `fzf` con filtro fuzzy y preview.

---

## 5. CIBERSEGURIDAD — Plan Blindado P2P (Crítico)

> **Amenaza actual:** 2025-08-19 ataque a `arrayref/internment/append-only-vec` (245M descargas) vía `proc-macro1` typosquat + `build.rs` que descarga binario remoto. npm Shai-Hulud afectó 492 paquetes. crates.io es objetivo de alto valor. Nuestro P2P no puede repetir esos errores.

### 5.1 Modelo de Amenazas (STRIDE)

| Vector | Ejemplo real | Impacto en Falcato si no blindamos |
|--------|--------------|-------------------------------------|
| **Suplantación (Spoofing)** | Typosquatting `reqwests`, robo cuenta mantenedor | Instalas `jsom` creyendo `json`, roban claves |
| **Manipulación (Tampering)** | DHT poisoning, BEP44 sin firma sobreescrito | Fardo `json@0.1` apunta a malware |
| **Repudio** | `yank` borra historial, no auditable | Autor niega haber publicado malware |
| **Divulgación** | Fuga IP al pedir fardo, tracker espía | Privacidad del nido expuesta |
| **DoS** | Sybil 10k nodos falsos, zip bomb 10GB, `dht_consultar(len=1e9)` | Nodo colgado, memoria agotada |
| **Elevación** | `build.rs` ejecuta binario, `proceso_crear` inyección, zip slip `../../.bashrc` | RCE en tu PC |

### 5.2 10 Capas de Defensa (no 8) — Defensa en Profundidad

#### Capa 1 — Identidad Soberana + Firma Obligatoria (ed25519)
* **Regla:** Sin firma no publica. `fardo publicar` firma `sha256(tar.zst)` con `~/.falcato/clave.priv`. `fardo instalar` verifica firma con `pubkey` del `falcato.lock`. Si falla → `[C010] firma inválida, fardo rechazado`.
* **Por qué:** Sigstore/cosign demuestra que firmar es barato y salva vidas. `ed25519-dalek` ya en Rust.
* **Mitiga:** Suplantación, tampering.

#### Capa 2 — Content-Addressing (sha256) + Solo Fuente
* **Regla:** Artefacto = `sha256`. `falcato.lock` guarda hash. Al descargar, se recalcula y compara. **Principio "solo fuente":** un fardo es solo `.fc` + `falcato.toml`. **Prohibido `build.rs`, binarios precompilados o scripts.** Si necesitas C, es `extern C` explícito y auditado.
* **Por qué:** El ataque `proc-macro1` de agosto 2026 funcionó porque `build.rs` descarga binario en `cargo build`. Nosotros nunca ejecutamos nada al instalar.
* **Mitiga:** Tampering, elevación (RCE).

#### Capa 3 — Petnames + WoT (Web of Trust) contra Typosquatting
* **Regla:** No hay namespace global. `json` es alias local a `pubkey:a1b2...`. `fardo buscar` ordena por `★` = confianzas. `falcato fardo confiar a1b2...` firma confianza. Un fardo con `★★★★★ (12/12)` es más fiable que `★☆☆☆☆ (0)`.
* **Por qué:** crates.io sufre typosquatting porque `rust_decimal` vs `rustdecimal` compiten por string. Con petnames, tú decides qué `json` es tu `json`.
* **Mitiga:** Typosquatting, suplantación.

#### Capa 4 — Transparency Log Distribuido (Merkle DAG, estilo Sigstore Rekor)
* **Regla:** Cada `BEP44 PUT` incrementa `seq`. Historial es append-only. `fardo info --historial` muestra `seq=1..n`. `yank` no borra, publica `seq=n+1 {yank:true}` firmado. Cualquiera puede auditar `git log` de la DHT (cache de 1000 entradas por pubkey).
* **Mitiga:** Repudio, tampering silencioso.

#### Capa 5 — DHT Hardening contra Eclipse/Sybil
* **Investigación:** IPFS 2020 y libp2p 2025 documentan eclipse donde atacante crea miles de IDs baratos cerca de clave objetivo y eclipsa la vista. Firma no basta.
* **Mitigaciones:**
  1. **Peer Scoring + IP diversity:** tabla de ruteo exige `/16` distintos, puntúa por uptime y latencia (libp2p DoS mitigation). Sybil en misma IP = score bajo.
  2. **Disjoint lookups:** búsqueda usa 3 caminos disjuntos, ningún peer cruza de camino (paper Yashksaini 2026-07). Si un camino está eclipsado, los otros responden.
  3. **Proof-of-work ligero para ID:** generar `pubkey` requiere `hash(pubkey) < dificultad` (hashcash 20 bits, ~1s CPU). Crear 10k Sybils = 2.7h, no gratis. `mainline` ya deriva ID de pubkey, añadimos PoW.
  4. **BEP44 límites:** `PUT` max 1KB, 1 por minuto por pubkey, `seq` debe ser `prev+1` y firma válida. `GET` paginado.
* **Mitiga:** Eclipse, Sybil, DHT poisoning.

#### Capa 6 — Límites Anti-DoS / Anti-ZipBomb (R8S.2, R8S.3)
* **Reglas duras:**
  * `dht_consultar(len)` → clamp `1..=100`, paginación cursor.
  * `tar.zst` → max 10MB comprimido, 50MB descomprimido, max 10k archivos, max 1k por dir, profundidad max 10.
  * Descompresión streaming con contador: si `descomprimido > 10*comprimido` → aborta `[C012] posible zip bomb`.
  * `buffer slicing` → checks `offset+len <= len` con `checked_add`, todo `usize` validado.
  * `tokio` tasks con `timeout 30s`, `max_concurrent 32`, `memory cap 256MB` por `instalar`.
* **Mitiga:** DoS memoria/CPU, zip bomb, OOM.

#### Capa 7 — Path Traversal + Zip Slip (OWASP)
* **Regla:** Al extraer `tar.zst`, sanitizar: `path.is_absolute()==false`, `!path.contains("..")`, `canonicalize` debe estar bajo `cache/extraidos/<hash>/`. Si no → `[C013] ruta fuera del nido, fardo malicioso rechazado`. Test con `../../.ssh/authorized_keys`.
* **Mitiga:** Elevación vía escritura arbitraria.

#### Capa 8 — Sandboxing de Instalación (sin ejecución)
* **Regla:** `fardo instalar` nunca ejecuta `.fc` ni binarios. Solo copia fuentes a `~/.falcato/cache`. Compilación posterior es `cranelift` puro, sin `Command::new`. `proceso_crear` (builtin futuro) tendrá allowlist + sin shell (`sh -c` prohibido) y args como lista, no string interpolado (mitiga inyección R8S.5).
* **Mitiga:** RCE en instalación (el 90% de ataques npm/cargo).

#### Capa 9 — Privacidad + Auth de Peers
* **Regla:** `fardo instalar` puede usar `QUIC` con `noise` (libp2p) y no revela tu `falcato.toml` completo. Peers se autentican con `pubkey` (challenge-response). No hay `auth peers` débil (R8S.7): peer debe probar posesión de `privkey` correspondiente a `pubkey` que anuncia.
* **Bootstrap:** lista firmada `~/.falcato/bootstrap.toml` con 3 nodos iniciales, verificada con clave hardcodeada del equipo Falcato (rotatable via `fardo confiar`).

#### Capa 10 — Auditoría Continua + SBOM
* **Regla:** `falcato fardo auditar` genera `SBOM` (CycloneDX) del nido + verifica todos los hashes/firmas contra cache. `falcato fardo actualizar --auditar` falla si algún fardo tiene `yank` o firma revocada (blocklist DHT `falcato/blocklist` firmada por 2 mantenedores de confianza).
* **CI:** `cargo deny` like para Falcato + `clippy` pedante. Cada PR que toque `src/fardo/` requiere 2 revisores + `cargo audit`.

#### Capa 11 — Reputación Soberana e Inalterable (Valoraciones de la Bandada) ⭐ — NUEVA
* **Idea única Falcato:** crates.io tiene estrellas opacas y el autor puede esconder issues. En Falcato la reputación **no la controla quien publica**, la controla la bandada. Cada valoración es un documento firmado por quien la emite, anclado al hash del fardo, y el autor no puede borrarla, editarla ni censurarla.

* **Primitiva criptográfica — Valoración:** JSON canónico de ~500 bytes, firmado con tu `clave.priv`:
  ```json
  {
    "fardo_hash": "sha256:a1b2…d4e5",
    "fardo_id": "a1b2…d4e5/json@0.1.0",
    "revisor": "pubkey:c3f8…9a1b",
    "estrellas": 5,
    "comentario": "JSON mínimo, compila en 40ms, sin sorpresas",
    "etiquetas": ["veloz","puro-fc"],
    "timestamp": 1724290000,
    "seq": 1,
    "firma": "ed25519:..."
  }
  ```
  Reglas: `1 ≤ estrellas ≤ 5`, `comentario ≤ 500 chars`, `etiquetas` de vocabulario cerrado, `seq` incremental por revisor+fardo. Firma cubre todo. Sin firma → descartada.

* **Almacenamiento — Fuera del alcance del autor (clave de la inalterabilidad):**
  1. **No vive bajo la clave del autor.** Vive bajo la clave del revisor: `BEP44 PUT <revisor_pubkey>:valoracion:<fardo_hash>` (mutable del revisor) + **réplica inmutable** en los 8 nodos DHT más cercanos a `hash("valoracion:"+fardo_hash)`. Esos 8 nodos son el índice colectivo.
  2. Autor publica en `a1b2…:json`, valoraciones viven en `hash(valoracion:a1b2…)`. Aunque el autor eclipse su propia clave o haga `yank`, las valoraciones siguen en otro shard que él no controla. No hay `DELETE`, solo `seq+1` del revisor para editar su propia valoración.
  3. **Inmutable por contenido:** cada valoración también se guarda como item inmutable `sha256(valoracion)` → cualquier peer puede re-sembrar aunque los 8 nodos caigan.

* **Descubrimiento:** `fardo info` hace 2 GET en paralelo: `GET fardo` + `GET valoraciones(hash(fardo))` a los 8 nodos, recoge hasta 100 valoraciones, verifica firmas local. <400ms. `fardo valoraciones general_beria/json --pagina 2` pagina.

* **Anti-Sybil / Anti-Spam (investigación 2025-2026: Sybil es el cáncer de reputación P2P):**
  1. **Peso por WoT, no conteo bruto:** `★` global no es media aritmética. Es **media ponderada por confianza**. Tu `falcato.toml [confianzas]` y el grafo WoT deciden peso. Un Sybil nuevo con 0 confianzas pesa ~0 aunque deje 1000 reviews de 5★. Atacante necesita que gente real confíe en sus Sybils (coste social, no solo CPU). Inspirado en `Chainscore Labs` + `BARM (Springer 2025)` contra colusión.
  2. **Prueba de uso + PoW ligero:** para valorar debes probar que descargaste el fardo: la valoración incluye `prueba_descarga = firma(fardo_hash + revisor_pubkey)` y solo se acepta si tu nodo tiene el `fardo_hash` en `~/.falcato/cache` (verificable por pares). + PoW 18 bits por valoración (~0.25s) para frenar spam masivo.
  3. **Límites duros:** 1 valoración activa por `revisor+fardo`, 1 valoración/hora por revisor global, `comentario` con filtro de longitud y rate-limit. Editar es `seq+1`, no borrar.
  4. **Anti-colusión/bad-mouthing:** Detección de colusión `BARM`: si 20 cuentas nuevas valoran 1★ el mismo día, el agregador las devalúa por correlación temporal + grafo. Moderación comunitaria: `fardo reportar <hash_valoracion> --motivo spam` también firmado; reportes con peso WoT ocultan valoraciones tóxicas sin borrarlas.

* **Visualización honesta (dos medias):**
  ```
  fardo info general_beria/json
  Reputación: ★★★★☆ 4.7  (23 valoraciones)
              Tu bandada: ★★★★★ 5.0 (4 de tus confianzas)
              Global:     ★★★★☆ 4.6 (23)
  Distribución: 5★ ████████████ 18  4★ ██ 3  3★ █ 1  2★ 0  1★ █ 1
  Últimas:
    c3f8…9a1b ★★★★★ "Compila en 40ms"  hace 2d  ✓ usa el fardo
    9f2a…1c8e ★★★☆☆ "Falta doc"        hace 5d
  ```
  `buscar` muestra `★` ponderada por TU WoT, no global inflada. Honestidad radical.

* **Comandos:**
  ```bash
  falcato fardo valorar general_beria/json --estrellas 5 --comentario "Excelente"
  falcato fardo valoraciones general_beria/json --pagina 1 --orden recientes
  falcato fardo reportar <hash_valoracion> --motivo spam
  falcato fardo confiar c3f8…9a1b  # tu voto de confianza alimenta el peso
  ```

* **Garantías:** Autor no puede borrar/editar valoraciones ajenas (no tiene su clave). Solo el revisor con su `privkey` puede editar la suya (seq+1). DHT replica en 8 nodos → aunque autor controle 3, quedan 5. Firmas hacen que nodos maliciosos no puedan forjar valoraciones.

### 5.3 Matriz de Cobertura — Pendientes R8S.1-7 Cerrados

| Pendiente AGENTS | Capa que lo cierra | Estado en Plan |
|------------------|--------------------|----------------|
| R8S.1 SET sin firma | Capa 1 + 5 (BEP44 seq+firma) | ✅ PUT rechaza sin firma |
| R8S.2 DoS memoria/CPU | Capa 6 (límites + timeout) | ✅ clamp + cap 256MB |
| R8S.3 buffer slicing | Capa 6 (checked_add) | ✅ |
| R8S.4 `dht_consultar` len | Capa 6 (1..=100 paginado) | ✅ |
| R8S.5 `proceso_crear` inyección | Capa 8 (allowlist, sin shell) | ✅ |
| R8S.6 auth peers | Capa 9 (challenge-response) | ✅ |
| R8S.7 permisos por tipos | Capa 2 + 8 (solo fuente + permisos declarados) | ✅ `[permisos=["red"]]` verificado en `verifica` |

### 5.4 Qué NO Haremos con Reputación (para no repetir errores)

* No likes anónimos sin firma (Sybil gratis).
* No media global sin peso WoT (ballot stuffing).
* No borrar valoraciones (solo ocultar por reportes con peso).

### 5.4 Qué NO Haremos (para no repetir errores de IPFS/libp2p)

* No PoW pesado tipo blockchain (ahuyenta devs). 20 bits basta.
* No token/incentivo económico (complejidad legal). Reputación WoT es suficiente como en Torrent privado.
* No ejecutar WASM del fardo en instalación (aunque sea tentador sandboxear).

---

## 6. Criterios de Éxito Honestos

1. `fardo buscar` <500ms con 200 peers, sin servidor central.
2. Ataque Sybil 1k nodos no eclipsa búsqueda (test de caos).
3. `arrayref`-style typosquat es imposible por petnames + firma.
4. Zip bomb/zip slip rechazados con mensaje en español `[C012]/[C013]`.
5. Segunda compilación 20 fardos <1s (cache `.o` P2P).

---

## 7. Próximos Pasos

1. **F1:** Tipos Rust `FardoId(pubkey,nombre)`, `Manifiesto`, `Bloqueo`, trait `Bandada` (DHT).
2. **Docs:** Actualizar `AGENTS.md` R8 y skill `falcato-language`.
3. **Seguridad:** Añadir `ed25519-dalek`, `sha2`, `quinn`, `mainline` a `Cargo.toml` en F2.

*Plan firmado v1.1 (con reputación): General Beria — Cangrejo — 2026-08-22*
