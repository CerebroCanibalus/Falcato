# Falcato 0.8.0-beta.1 — Release Notes

**Fecha:** 2026-09-03
**Tipo:** Pre-release

## 🎯 Resumen

0.8.0 es la actualización que rediseña y mejora brutalmente el lenguaje. Agrupa los problemas de diseño abiertos más importantes y las lecciones de Go. Es el "gofmt + stdlib + rediseño" que lleva a Falcato de "Rust con artículos" a "lenguaje de plataforma para LLMs".

## ✅ Nuevas Features

### LSP (Language Server Protocol)
- **153 builtins** en autocompletado (antes: 15)
- **22 snippets** de template (`si`, `para`, `fn`, `estructural`, etc.)
- **Métodos por tipo** después de `.` (Texto: 17, Vector: 9, Diccionario: 5)
- **Variantes de enum** después de `Enum.`
- **Inlay hints** — muestra tipos inferidos
- **Rename symbol** — renombrado seguro
- **Call hierarchy** — prevenir + incoming + outgoing
- **Code lens** — "▶ Ejecutar" / "🧪 Test" en cada función
- **Format document** — formateo automático (4 espacios)
- **Cross-file** — resolución de imports `usar`

### LibEst (Librería Estándar)
- **20 módulos**, **169 funciones**
- **12 dominios**: núcleo, colecciones, archivo, red, tiempo, proceso, sistema, matemáticas, visual, compat
- **Builtins Rust**: texto (22), HTTP (2), JSON (4), TCP (10), TLS (5), archivo (8), tiempo (5), vector (15), diccionario (10), conjunto (7), opción/resultado (4), visual (31)

### Skills
- **SKILL.md** — gramática completa (339 líneas)
- **builtins.md** — 153 builtins con firmas
- **libEst.md** — 169 funciones por módulo
- **patterns.md** — patrones y arquitecturas (505 líneas)
- **errores.md** — códigos de error y fixes

## 🔧 Mejoras

- **Namespace `::`** funcional en parser + semántico + codegen
- **Coerción polimórfica** `42`→`Entero64`/`Natural`
- **Profiling** `reloj_mono_ns`+`perfil_*`
- **Cross-file** `verifica a.fc b.fc`

## 📊 Métricas

| Métrica | Valor |
|---------|-------|
| Builtins | 153 |
| Funciones stdlib | 169 |
| Archivos LSP | 1 |
| Líneas LSP | ~2500 |
| Skills | 5 archivos |
| Tests | 54/54 + 19 unitest |
| Ejemplos | 76/83 compilan |

## 🚀 Próximos pasos (0.8.0 estable)

- [ ] P-005: Reducir builtins (mover a Falcato puro)
- [ ] P-009: Documentar artículos `el`/`la`/`un`
- [ ] P-014: Warning de `Resultado` no manejado
- [ ] P-017: Regla unificadora de API
- [ ] `falcato formatea` CLI
- [ ] Stdlib HTTP+JSON en Falcato puro
- [ ] Modernizers (`falcato arregla`)

## 📝 Notas

- **Windows only** por ahora (target x86_64-pc-windows-msvc)
- **109 warnings** en release (no bloqueantes)
- **Dependencies**: Cranelift 0.112, tower-lsp 0.20, tokio 1.x, serde_json 1.0

---

**Descarga:** `cargo install falcato --beta`
**Repo:** https://github.com/CerebroCanibalus/falcato
