use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use std::process::Command;

mod ast;
mod backend;
mod codegen;
mod codegen_helpers;
mod error;
mod futuros;
mod lexer;
mod lsp;
mod parser;
mod platform;
mod resolver;
mod semantic;
mod span;

use crate::ast::Programa;
use crate::codegen::Codegen;
use crate::lexer::LexerFalcato;
use crate::parser::ParserFalcato;
use crate::resolver::Resolver;
use crate::semantic::AnalizadorSemantico;
// Cranelift - puro Rust, sin dependencias del sistema

/// CLI de Falcato
#[derive(Parser)]
#[command(name = "falcato")]
#[command(about = "Compilador del lenguaje Falcato")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    comando: Comandos,
}

#[derive(Subcommand)]
enum Comandos {
    /// Compila archivos .fc a binario
    Build {
        /// Archivo(s) fuente .fc (principal + dependencias)
        #[arg(required = true)]
        archivos: Vec<String>,
        /// Ruta de salida del binario
        #[arg(short, long)]
        output: Option<String>,
        /// Target triple (default: nativo)
        #[arg(long)]
        target: Option<String>,
        /// Modo release (optimizaciones)
        #[arg(long)]
        release: bool,
    },
    /// Compila y ejecuta archivos .fc
    Run {
        /// Archivo(s) fuente .fc (principal + dependencias)
        #[arg(required = true)]
        archivos: Vec<String>,
        /// Argumentos para el programa ejecutado
        #[arg(allow_hyphen_values = true, last = true)]
        args: Vec<String>,
    },
    /// Solo análisis (sin generar binario)
    Check {
        /// Archivo(s) fuente .fc
        #[arg(required = true)]
        archivos: Vec<String>,
    },
    /// Muestra la versión
    Version,
    /// Ejecuta las pruebas definidas con `prueba "nombre" { ... }`
    Test {
        /// Archivo(s) fuente .fc
        #[arg(required = true)]
        archivos: Vec<String>,
    },
    /// Inicia el servidor LSP (Language Server Protocol)
    Lsp,
}

fn main() {
    let cli = Cli::parse();

    match cli.comando {
        Comandos::Build {
            archivos,
            output,
            target,
            release,
        } => {
            if let Err(e) = compilar(&archivos,
                output.as_deref(),
                target.as_deref(),
                release,
            ) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
        Comandos::Run { archivos, args } => {
            if let Err(e) = compilar_y_ejecutar(&archivos, &args) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
        Comandos::Check { archivos } => {
            if let Err(e) = verificar(&archivos) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
        Comandos::Version => {
            println!("Falcato 0.1.0");
            println!("Lenguaje de programación de sistemas iberohablante");
        }
        Comandos::Test { archivos } => {
            if let Err(e) = ejecutar_pruebas(&archivos) {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        }
        Comandos::Lsp => {
            eprintln!("[Falcato LSP] Iniciando servidor...");
            eprintln!("[Falcato LSP] Usando stdio para comunicación");
            let runtime = tokio::runtime::Runtime::new()
                .expect("No se pudo crear runtime de Tokio");
            runtime.block_on(async {
                lsp::iniciar_lsp().await;
            });
        }
    }
}

/// Compila múltiples archivos usando el Resolver y el backend Cranelift.
fn compilar(
    archivos: &[String],
    output: Option<&str>,
    target: Option<&str>,
    release: bool,
) -> Result<(), String> {
    if archivos.is_empty() {
        return Err("No se especificaron archivos fuente.".to_string());
    }

    // Si es un solo archivo, usar ruta rápida monolítica (legacy).
    // El resolver multi-archivo se usa solo cuando se pasan múltiples archivos explícitamente.
    if archivos.len() == 1 {
        let archivo = &archivos[0];
        return compilar_individual(archivo, output, target, release);
    }

    // Ruta multi-archivo con Resolver
    println!("[Falcato] Compilando {} archivo(s)...", archivos.len());

    let base_dir = Path::new(&archivos[0]).parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut resolver = Resolver::nuevo(&base_dir);

    for archivo in archivos {
        resolver.agregar_archivo(Path::new(archivo))?;
    }

    resolver.calcular_orden()?;

    println!("[Falcato] Orden de compilación: {:?}", resolver.orden);

    let objetos = resolver.compilar_todo()?;

    // Linkear todos los .o juntos
    let primer_archivo = &archivos[0];
    let binario = output.map(String::from)
        .unwrap_or_else(|| {
            Path::new(primer_archivo)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| format!("{}.exe", s))
                .unwrap_or_else(|| "a.exe".to_string())
        });

    let rutas_obj: Vec<&str> = objetos.iter().map(|(_, ruta)| ruta.as_str()).collect();
    link_objetos(&rutas_obj, &binario, target, release)?;

    println!("[Falcato] Binario generado: {}", binario);
    Ok(())
}

/// Ruta rápida legacy para un solo archivo sin imports (comportamiento anterior).
fn compilar_individual(
    archivo: &str,
    output: Option<&str>,
    target: Option<&str>,
    _release: bool,
) -> Result<(), String> {
    println!("[Falcato] Compilando '{}'...", archivo);

    let fuente = fs::read_to_string(archivo)
        .map_err(|e| format!("No se pudo leer '{}': {}", archivo, e))?;

    let lexer = LexerFalcato::nuevo(&fuente, archivo);
    let tokens = lexer.tokenizar();
    println!("[Falcato] {} tokens generados", tokens.len());

    let programa = ParserFalcato::parse(tokens)
        .map_err(|errores| {
            let msgs: Vec<String> = errores.iter()
                .map(|e| e.error.to_string())
                .collect();
            format!("Errores de parseo:\n{}", msgs.join("\n"))
        })?;
    println!("[Falcato] AST generado: {} declaraciones", programa.declaraciones.len());

    let mut semantica = AnalizadorSemantico::nuevo();
    semantica.analizar(&programa)
        .map_err(|e| format!("Errores semánticos:\n{}", e))?;
    println!("[Falcato] Análisis semántico: sin errores de concordancia");

    let mut codegen = Codegen::nuevo("main")
        .map_err(|e| format!("Error inicializando codegen: {}", e))?;
    codegen.compilar_programa(&programa)
        .map_err(|e| format!("Errores de compilación:\n{:?}", e))?;

    let obj_ruta = format!("{}.o", archivo.strip_suffix(".fc").unwrap_or(archivo));
    codegen.escribir_objeto(&obj_ruta)?;
    println!("[Falcato] Objeto generado: {}", obj_ruta);

    let binario = output.map(String::from)
        .unwrap_or_else(|| {
            Path::new(archivo)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| format!("{}.exe", s))
                .unwrap_or_else(|| "a.exe".to_string())
        });

    link_objeto(&obj_ruta, &binario, target, false)?;
    println!("[Falcato] Binario generado: {}", binario);
    Ok(())
}

fn compilar_y_ejecutar(archivos: &[String], args: &[String]) -> Result<(), String> {
    if archivos.is_empty() {
        return Err("No se especificaron archivos fuente.".to_string());
    }

    let primer = &archivos[0];
    let binario = format!("{}.exe", primer.strip_suffix(".fc").unwrap_or(primer));
    
    compilar(archivos, Some(&binario), None, false)?;

    println!("[Falcato] Ejecutando '{}'...", binario);
    
    let mut cmd = Command::new(&binario);
    cmd.args(args);
    
    let status = cmd.status()
        .map_err(|e| format!("No se pudo ejecutar '{}': {}", binario, e))?;

    if !status.success() {
        return Err(format!("El programa terminó con código: {}",
            status.code().unwrap_or(-1)));
    }

    Ok(())
}

fn verificar(archivos: &[String]) -> Result<(), String> {
    if archivos.is_empty() {
        return Err("No se especificaron archivos fuente.".to_string());
    }

    for archivo in archivos {
        println!("[Falcato] Verificando '{}'...", archivo);

        let fuente = fs::read_to_string(archivo)
            .map_err(|e| format!("No se pudo leer '{}': {}", archivo, e))?;

        let lexer = LexerFalcato::nuevo(&fuente, archivo);
        let tokens = lexer.tokenizar();

        let programa = ParserFalcato::parse(tokens)
            .map_err(|errores| {
                let msgs: Vec<String> = errores.iter()
                    .map(|e| e.error.to_string())
                    .collect();
                format!("Errores de parseo en '{}':\n{}", archivo, msgs.join("\n"))
            })?;

        let mut semantica = AnalizadorSemantico::nuevo();
        semantica.analizar(&programa)
            .map_err(|e| format!("Errores semánticos en '{}':\n{}", archivo, e))?;

        println!("[Falcato] '{}' verificado: sin errores", archivo);
    }

    Ok(())
}

/// Compila y ejecuta las pruebas definidas con `prueba "nombre" { ... }`
fn ejecutar_pruebas(archivos: &[String]) -> Result<(), String> {
    if archivos.is_empty() {
        return Err("No se especificaron archivos fuente.".to_string());
    }

    let archivo = &archivos[0];
    println!("[Falcato] Ejecutando pruebas de '{}'...", archivo);

    let fuente = fs::read_to_string(archivo)
        .map_err(|e| format!("No se pudo leer '{}': {}", archivo, e))?;

    let lexer = LexerFalcato::nuevo(&fuente, archivo);
    let tokens = lexer.tokenizar();

    let mut programa = ParserFalcato::parse(tokens)
        .map_err(|errores| {
            let msgs: Vec<String> = errores.iter()
                .map(|e| e.error.to_string())
                .collect();
            format!("Errores de parseo:\n{}", msgs.join("\n"))
        })?;

    // Extraer pruebas y eliminarlas del AST
    let pruebas: Vec<ast::PruebaDecl> = programa.declaraciones.iter()
        .filter_map(|d| {
            if let ast::Declaracion::Prueba(p) = d { Some(p.clone()) } else { None }
        })
        .collect();

    if pruebas.is_empty() {
        println!("[Falcato] No se encontraron pruebas.");
        return Ok(());
    }

    // Eliminar pruebas y renombrar principal del usuario
    programa.declaraciones.retain(|d| !matches!(d, ast::Declaracion::Prueba(_)));
    for decl in &mut programa.declaraciones {
        if let ast::Declaracion::Funcion(ref mut func) = decl {
            if func.nombre == "principal" {
                func.nombre = "__principal_usuario".to_string();
            }
        }
    }

    // Generar funciones de prueba como AST normal
    let span_dummy = span::Span::vacio();
    for (i, prueba) in pruebas.iter().enumerate() {
        // función __prueba_N() -> Entero32 { ...cuerpo...; retornar 0; }
        let mut sentencias = prueba.bloque.sentencias.clone();
        sentencias.push(ast::Sentencia::Retornar(
            Some(ast::Expresion::Literal(ast::Literal::Entero(0, span_dummy.clone()))),
            span_dummy.clone(),
        ));

        let func_prueba = ast::FuncionDecl {
            nombre: format!("__prueba_{}", i),
            parametros: vec![],
            parametros_genericos: vec![],
            retorno: Some(ast::Tipo::Entero32),
            cuerpo: ast::Bloque { sentencias, span: prueba.span.clone() },
            es_insegura: false,
            nivel_verificacion: ast::NivelVerificacion::Permisivo,
            efecto: ast::Efecto::Conservador,
            visibilidad: None,
            es_futuro: false,
            span: prueba.span.clone(),
        };
        programa.declaraciones.push(ast::Declaracion::Funcion(func_prueba));
    }

    // Generar principal() que llama a cada prueba e imprime resultados
    let mut sentencias_main: Vec<ast::Sentencia> = Vec::new();
    for (i, prueba) in pruebas.iter().enumerate() {
        // imprimir_linea("  prueba: <nombre>...")
        let msg = format!("  prueba: {}...", prueba.nombre);
        sentencias_main.push(ast::Sentencia::Expresion(
            ast::Expresion::Llamada(ast::Llamada {
                funcion: "imprimir_linea".to_string(),
                tipo_args: vec![],
                argumentos: vec![ast::Expresion::Literal(ast::Literal::Palabra(msg, span_dummy.clone()))],
                span: span_dummy.clone(),
            }),
        ));
        // __prueba_N()
        sentencias_main.push(ast::Sentencia::Expresion(
            ast::Expresion::Llamada(ast::Llamada {
                funcion: format!("__prueba_{}", i),
                tipo_args: vec![],
                argumentos: vec![],
                span: span_dummy.clone(),
            }),
        ));
        // imprimir_linea("    OK")
        sentencias_main.push(ast::Sentencia::Expresion(
            ast::Expresion::Llamada(ast::Llamada {
                funcion: "imprimir_linea".to_string(),
                tipo_args: vec![],
                argumentos: vec![ast::Expresion::Literal(ast::Literal::Palabra("    OK".to_string(), span_dummy.clone()))],
                span: span_dummy.clone(),
            }),
        ));
    }
    // imprimir_linea("\nTodas las pruebas pasaron.")
    sentencias_main.push(ast::Sentencia::Expresion(
        ast::Expresion::Llamada(ast::Llamada {
            funcion: "imprimir_linea".to_string(),
            tipo_args: vec![],
            argumentos: vec![ast::Expresion::Literal(ast::Literal::Palabra("\nTodas las pruebas pasaron.".to_string(), span_dummy.clone()))],
            span: span_dummy.clone(),
        }),
    ));
    // retornar 0
    sentencias_main.push(ast::Sentencia::Retornar(
        Some(ast::Expresion::Literal(ast::Literal::Entero(0, span_dummy.clone()))),
        span_dummy.clone(),
    ));

    let func_main = ast::FuncionDecl {
        nombre: "principal".to_string(),
        parametros: vec![],
        parametros_genericos: vec![],
        retorno: Some(ast::Tipo::Entero32),
        cuerpo: ast::Bloque { sentencias: sentencias_main, span: span_dummy.clone() },
        es_insegura: false,
        nivel_verificacion: ast::NivelVerificacion::Permisivo,
        efecto: ast::Efecto::Conservador,
        visibilidad: None,
        es_futuro: false,
        span: span_dummy.clone(),
    };
    programa.declaraciones.push(ast::Declaracion::Funcion(func_main));

    // Compilar normalmente
    let mut semantica = AnalizadorSemantico::nuevo();
    semantica.analizar(&programa)
        .map_err(|e| format!("Errores semánticos:\n{}", e))?;

    let mut codegen = Codegen::nuevo("main")
        .map_err(|e| format!("Error inicializando codegen: {}", e))?;
    codegen.compilar_programa(&programa)
        .map_err(|e| format!("Errores de compilación:\n{:?}", e))?;

    let obj_ruta = format!("{}.o", archivo.strip_suffix(".fc").unwrap_or(archivo));
    codegen.escribir_objeto(&obj_ruta)?;

    let binario = format!("{}_test.exe", archivo.strip_suffix(".fc").unwrap_or(archivo));
    link_objeto(&obj_ruta, &binario, None, false)?;

    println!("[Falcato] Binario de pruebas generado: {}", binario);
    println!();

    let status = Command::new(&binario)
        .status()
        .map_err(|e| format!("No se pudo ejecutar '{}': {}", binario, e))?;

    if !status.success() {
        return Err(format!("Pruebas fallaron (código: {})", status.code().unwrap_or(-1)));
    }

    Ok(())
}

fn link_objeto(
    obj: &str,
    binario: &str,
    target: Option<&str>,
    _release: bool,
) -> Result<(), String> {
    link_objetos(&[obj], binario, target, _release)
}

fn link_objetos(
    objetos: &[&str],
    binario: &str,
    target: Option<&str>,
    _release: bool,
) -> Result<(), String> {
    let target = target.unwrap_or("x86_64-pc-windows-msvc");

    if target.contains("windows") {
        // Buscar link.exe en ubicaciones comunes de Visual Studio
        let link_paths = [
            r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.29.30133\bin\HostX64\x64\link.exe",
            r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.16.27023\bin\HostX64\x64\link.exe",
            r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.29.30133\bin\HostX64\x64\link.exe",
        ];
        
        let link_exe = link_paths.iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(|p| p.to_string())
            .or_else(|| {
                // Intentar encontrar en PATH
                Command::new("where").arg("link.exe").output().ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
            })
            .ok_or("No se encontró link.exe. Instala Visual Studio Build Tools o añádelo al PATH.")?;
        
        let mut cmd = Command::new(&link_exe);
        for obj in objetos {
            cmd.arg(obj);
        }
        // GUI trampolín C precompilado (lib/trampolin_win32.obj)
        let trampolin = std::path::Path::new("lib/trampolin_win32.obj");
        if trampolin.exists() {
            cmd.arg(trampolin);
        }
        // Runtime library (falcato_runtime staticlib)
        let runtime_lib = std::path::Path::new("lib/falcato_runtime/target/release/falcato_runtime.lib");
        if runtime_lib.exists() {
            cmd.arg(runtime_lib);
        }
        cmd.arg(format!("/OUT:{}", binario))
            .arg("/SUBSYSTEM:CONSOLE")
            .arg("/ENTRY:principal")
            // VC++ runtime libs
            .arg("/LIBPATH:C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Tools\\MSVC\\14.29.30133\\lib\\x64")
            .arg("/LIBPATH:C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Tools\\MSVC\\14.16.27023\\lib\\x64")
            // UCRT + UM (Windows SDK)
            .arg("/LIBPATH:C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.26100.0\\ucrt\\x64")
            .arg("/LIBPATH:C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.26100.0\\um\\x64")
            .arg("/LIBPATH:C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.22621.0\\ucrt\\x64")
            .arg("/LIBPATH:C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.22621.0\\um\\x64")
            .arg("libcmt.lib")
            .arg("ucrt.lib")
            .arg("legacy_stdio_definitions.lib")
            .arg("vcruntime.lib")
            .arg("kernel32.lib")
            .arg("user32.lib")
            .arg("gdi32.lib")
            .arg("ws2_32.lib")
            .arg("ntdll.lib")
            .arg("userenv.lib");

        let output = cmd.output()
            .map_err(|e| format!("Error al ejecutar linker: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Error de link:\nSTDERR:\n{}\nSTDOUT:\n{}", stderr, stdout));
        }
    } else {
        // Linux/macOS: usar gcc o clang
        let mut cmd = Command::new("gcc");
        for obj in objetos {
            cmd.arg(obj);
        }
        cmd.arg("-o")
            .arg(binario);

        let output = cmd.output()
            .map_err(|e| format!("Error al ejecutar linker: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Error de link:\n{}", stderr));
        }
    }

    Ok(())
}
