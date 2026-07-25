![Falcato Title](assets/images/falcato_title.png)

> **Nivel:** Principiante  
> **Tiempo:** 5-10 minutos  
> **📖 Primero lee:** [GUIA.md](GUIA.md) — visión general del lenguaje

---

## 📚 Documentación relacionada

| Guía | Descripción |
|------|-------------|
| [📖 GUIA.md](GUIA.md) | Tutorial completo desde cero |
| [⚙️ INSTALL.md](INSTALL.md) | **← Estás aquí** |
| [📗 REFERENCIA.md](REFERENCIA.md) | Catálogo de funciones built-in |
| [🚨 ERRORES.md](ERRORES.md) | Códigos de error y soluciones |

---

## Requisitos

- **Windows 10/11** (64 bits)
- Opcional: [Visual Studio Code](https://code.visualstudio.com/) para edición con LSP

---

## Opción 1: Descargar el ZIP (recomendado)

### Paso 1: Descargar

1. Ve a [github.com/CerebroCanibalus/falcato/releases](https://github.com/CerebroCanibalus/falcato/releases)
2. Descarga el archivo `falcato-v0.2.0-x86_64-windows.zip`
3. Extrae en alguna carpeta, por ejemplo `C:\Falcato`

### Paso 2: DLLs necesarias

Ejecuta el script que copia las DLLs:

```powershell
cd C:\Falcato
.\bundle_dlls.ps1
```

Esto copia `VCRUNTIME140.dll` (necesaria para ejecutar el compilador).

### Paso 3: Probar

```powershell
.\falcato.exe version
# → Falcato v0.1.0
```

### Paso 4: (Opcional) Agregar al PATH

Para poder escribir `falcato` desde cualquier carpeta:

```powershell
# PowerShell (como administrador)
[Environment]::SetEnvironmentVariable("Path", "$env:Path;C:\Falcato", "User")
```

---

## Opción 2: Compilar desde código fuente

### Paso 1: Instalar Rust

Descarga e instala Rust desde [rustup.rs](https://rustup.rs/).

### Paso 2: Instalar Visual Studio Build Tools

Necesitas el linker de Microsoft. Descarga [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) y selecciona:

- "Desktop development with C++"

### Paso 3: Clonar y compilar

```powershell
git clone https://github.com/CerebroCanibalus/falcato.git
cd falcato
cargo build --release
```

### Paso 4: Probar

```powershell
.\target\release\falcato.exe version
```

El binario compilado está en `target\release\falcato.exe`.

---

## 🧠 LSP — La Pieza Clave para Agentes e IA

El servidor LSP de Falcato es lo que permite que **editores** (VS Code) y **agentes** (OpenCode) entiendan tu código en tiempo real. Sin el LSP, el compilador solo te dice los errores cuando ejecutas; con el LSP, los errores aparecen **mientras escribes**.

### Lo que hace el LSP

| Feature | Para qué sirve |
|---------|---------------|
| **Diagnósticos** | Errores y warnings en tiempo real mientras escribes |
| **Autocompletado** | Keywords, tipos, funciones built-in, variables en scope |
| **Signature Help** | Muestra parámetros al escribir `funcion(` |
| **Hover** | Tipo y ownership de variables, firmas de funciones |
| **Ir a definición** | Salta a la declaración de variables y funciones |
| **Document Symbols** | Outline del archivo (funciones, structs, enums) |
| **Code Actions** | Quick fixes para errores comunes |

### Cómo se conecta

- **VS Code**: via la extensión (`client.js`), que lanza `falcato lsp` como child process
- **OpenCode**: configurado en `opencode.jsonc` global, lanza `D:\Falcato\target\release\falcato.exe lsp`

Sin el LSP instalado, el compilador sigue funcionando, pero **pierdes toda la retroalimentación en tiempo real** que hace que Falcato sea usable como herramienta de enseñanza para IA.

---

## Opción 3: VS Code Extension

Para tener resaltado de sintaxis, LSP y tema "Falcato Dorado":

### Desde el ZIP

El archivo `.vsix` viene incluido en el ZIP de release. Instálalo:

```powershell
code --install-extension falcato-vscode\falcato-language-*.vsix
```

### O desde VS Code

1. Abre VS Code
2. `Ctrl+Shift+P` → "Install from VSIX..."
3. Selecciona el archivo `.vsix`
4. Abre un archivo `.fc` — verás el resaltado automáticamente

### Probar el tema

1. `Ctrl+K Ctrl+T`
2. Busca "Falcato Dorado"
3. Selecciónalo

### Para desarrolladores: construir el VSIX

```powershell
cd D:\Falcato\falcato-vscode
npm install                        # solo la primera vez
npx vsce package                   # genera .vsix
code --install-extension falcato-language-*.vsix --force
```

---

## Verificar que todo funciona

Crea un archivo `hola.fc`:

```falcato
función principal() -> Entero32 {
    decir("¡Hola, mundo!");
    retornar 0;
}
```

Compila y ejecuta:

```powershell
falcato run hola.fc
# → ¡Hola, mundo!
```

---

## Solución de problemas

| Problema | Causa | Solución |
|----------|-------|----------|
| `VCRUNTIME140.dll` no encontrada | Faltan DLLs | Ejecuta `bundle_dlls.ps1` |
| `link.exe` no encontrado | Faltan Build Tools | Instala Visual Studio Build Tools |
| `falcato no se reconoce` | No está en PATH | Usa ruta completa o agrega al PATH |
| El tema no aparece en VS Code | Extensión no instalada | Instala desde VSIX o reinicia VS Code |
