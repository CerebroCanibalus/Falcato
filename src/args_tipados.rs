//! # Argumentos tipados (R7.5 Fase 2) — la innovación
//!
//! `función principal(args: MiStruct) -> Entero32` — el compiler genera
//! automáticamente el parseo de `--etiqueta valor` + validación de tipos +
//! `--ayuda` automático, TODO en español.
//!
//! ## El esquema se codifica con artículos en los campos del struct:
//! - `el campo: Tipo`  → requerido (error si falta `--campo`)
//! - `un campo: Tipo`  → opcional (default si falta)
//! - `la campo: Tipo`  → inmutable/validado (se valida tipo al asignar)
//! - `los campo: Tipo` → varargs posicionales (argumentos sin `--`)
//!
//! ## Cómo funciona:
//! 1. El parser ya lee los artículos en campos de struct (AST + parser).
//! 2. `preprocesar` transforma `principal(args: MiStruct)`:
//!    - Elimina el parámetro ABI (el SO llama a `principal()` sin args).
//!    - Sintetiza un prólogo Falcato que:
//!      - Llama `argumentos()` y recorre argv.
//!      - Detecta `--campo` y asigna el valor convertido al tipo del campo.
//!      - Valida requeridos (campo `el`) y tipos (con `texto_a_*`).
//!      - Genera `--ayuda` automático en español.
//!      - Construye el struct `args` y lo deja como variable local.
//!    - El cuerpo original del usuario queda después del prólogo, viendo
//!      `args.campo` con tipos correctos.
//!
//! No es sintaxis nueva: struct + artículos que ya existen. Es
//! preprocesamiento de AST, igual que el modo `prueba`.

use crate::ast::*;
use crate::span::Span;

/// Tipos de campo soportados para el parseo tipado.
fn soportado(tipo: &Tipo) -> bool {
    matches!(tipo,
        Tipo::Texto | Tipo::Entero32 | Tipo::Entero64
        | Tipo::Natural32 | Tipo::Natural64
        | Tipo::Flotante64 | Tipo::Booleano)
}

/// Construye la expresión de conversión del valor `val` (un Texto) al tipo del campo.
fn expr_conversion(tipo: &Tipo, val: Expresion) -> Expresion {
    match tipo {
        Tipo::Entero32 => llamada("como_entero32", vec![llamada("texto_a_entero", vec![val])]),
        Tipo::Entero64 => llamada("texto_a_entero", vec![val]),
        Tipo::Natural32 => llamada("como_entero32", vec![llamada("texto_a_natural", vec![val])]),
        Tipo::Natural64 => llamada("texto_a_natural", vec![val]),
        Tipo::Flotante64 => llamada("texto_a_flotante", vec![val]),
        Tipo::Booleano => llamada("texto_a_booleano", vec![val]),
        _ => val, // Texto
    }
}

fn default_tipo(tipo: &Tipo) -> Expresion {
    let sp = Span::vacio();
    match tipo {
        Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64
        | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 => {
            Expresion::Literal(Literal::Entero(0, sp))
        }
        Tipo::Flotante32 | Tipo::Flotante64 => Expresion::Literal(Literal::Flotante(0.0, sp)),
        Tipo::Booleano => Expresion::Literal(Literal::Booleano(false, sp)),
        _ => Expresion::Llamada(Llamada {
            funcion: "texto_nuevo".to_string(),
            tipo_args: vec![],
            argumentos: vec![],
            span: sp,
        }),
    }
}

/// Crea un literal de texto Palabra (para --etiqueta, mensajes).
fn palabra(s: &str) -> Expresion {
    Expresion::Literal(Literal::Palabra(s.to_string(), Span::vacio()))
}

/// Crea una llamada simple: f(args...)
fn llamada(f: &str, args: Vec<Expresion>) -> Expresion {
    Expresion::Llamada(Llamada {
        funcion: f.to_string(),
        tipo_args: vec![],
        argumentos: args,
        span: Span::vacio(),
    })
}

/// Crea una llamada con tipo genérico: f<T>(args...)
fn llamada_tipada(f: &str, tipo: Tipo, args: Vec<Expresion>) -> Expresion {
    Expresion::Llamada(Llamada {
        funcion: f.to_string(),
        tipo_args: vec![tipo],
        argumentos: args,
        span: Span::vacio(),
    })
}

/// Expresión: a == b (Entero32/64)
fn igual(a: Expresion, b: Expresion) -> Expresion {
    Expresion::Binaria(Box::new(a), OperadorBinario::Igual, Box::new(b), Span::vacio())
}

/// Expresión: a != b
fn distinto(a: Expresion, b: Expresion) -> Expresion {
    Expresion::Binaria(Box::new(a), OperadorBinario::Distinto, Box::new(b), Span::vacio())
}

/// Expresión: a < b
fn menor(a: Expresion, b: Expresion) -> Expresion {
    Expresion::Binaria(Box::new(a), OperadorBinario::Menor, Box::new(b), Span::vacio())
}

/// Expresión: a + b
fn suma(a: Expresion, b: Expresion) -> Expresion {
    Expresion::Binaria(Box::new(a), OperadorBinario::Suma, Box::new(b), Span::vacio())
}

/// Identificador simple
fn id(n: &str) -> Expresion {
    Expresion::Identificador(n.to_string(), Span::vacio())
}

/// Declaración de variable: art nombre: Tipo = valor
fn declarar(art: Articulo, nombre: &str, tipo: Tipo, valor: Expresion) -> Sentencia {
    Sentencia::DeclaracionVariable(DeclaracionVariable {
        articulo: art,
        nombre: nombre.to_string(),
        tipo: Some(tipo),
        valor,
        span: Span::vacio(),
    })
}

/// Asignación: nombre = valor
fn asignar(nombre: &str, valor: Expresion) -> Sentencia {
    Sentencia::Asignacion(Asignacion {
        lugar: Lugar::Identificador(nombre.to_string()),
        valor,
        span: Span::vacio(),
    })
}

/// Sentencia de expresión (llamada suelta)
fn expr_sent(expr: Expresion) -> Sentencia {
    Sentencia::Expresion(expr)
}

/// retornar expr
fn retornar(expr: Expresion) -> Sentencia {
    Sentencia::Retornar(Some(expr), Span::vacio())
}

/// Bloque
fn bloque(sentencias: Vec<Sentencia>) -> Bloque {
    Bloque {
        sentencias,
        span: Span::vacio(),
    }
}

/// si cond { cuerpo }
fn si(cond: Expresion, cuerpo: Vec<Sentencia>) -> Sentencia {
    Sentencia::Condicional(Condicional {
        condicion: cond,
        bloque_entonces: bloque(cuerpo),
        bloque_sino: None,
        modo: ModoVerbal::Indicativo,
        span: Span::vacio(),
    })
}

/// mientras cond { cuerpo }
fn mientras(cond: Expresion, cuerpo: Vec<Sentencia>) -> Sentencia {
    Sentencia::BucleMientras(BucleMientras {
        condicion: cond,
        bloque: bloque(cuerpo),
        span: Span::vacio(),
    })
}

/// Preprocesa el programa: si `principal` tiene un parámetro de tipo struct,
/// lo transforma para parsear argumentos tipados. Devuelve Ok(()) siempre que
/// no haya un error de configuración (campo con tipo no soportado, etc.).
pub fn preprocesar(programa: &mut Programa) -> Result<(), String> {
    // Buscar `principal`
    let idx = programa.declaraciones.iter().position(|d| match d {
        Declaracion::Funcion(f) => f.nombre == "principal",
        _ => false,
    });

    let Some(idx) = idx else { return Ok(()) };
    let func = match &programa.declaraciones[idx] {
        Declaracion::Funcion(f) => f.clone(),
        _ => return Ok(()),
    };

    // Si no tiene un parámetro de tipo struct → no hay nada que hacer
    if func.parametros.len() != 1 {
        return Ok(());
    }
    let param = &func.parametros[0];
    let nombre_struct = match &param.tipo {
        Tipo::Nombre(n) => n.clone(),
        _ => return Ok(()),
    };

    // Buscar la definición del struct
    let struct_decl = programa.declaraciones.iter().find_map(|d| match d {
        Declaracion::Estructural(s) if s.nombre == nombre_struct => Some(s.clone()),
        _ => None,
    });
    let Some(struct_decl) = struct_decl else {
        return Ok(());
    };

    // Validar que todos los campos sean parseables
    for campo in &struct_decl.campos {
        if campo.articulo == Articulo::Los || campo.articulo == Articulo::Las {
            return Err(format!(
                "Campo '{}' del struct de argumentos '{}' usa artículo '{}': los argumentos posicionales (varargs) aún no están soportados en la Fase 2 — usa 'el' (requerido), 'un' (opcional) o 'la' (inmutable)",
                campo.nombre, nombre_struct,
                if campo.articulo == Articulo::Los { "los" } else { "las" },
            ));
        }
        if !soportado(&campo.tipo) {
            return Err(format!(
                "Campo '{}' del struct de argumentos '{}' tiene tipo no soportado para parseo tipado (soportados: Texto, Entero32, Entero64, Natural32, Natural64, Flotante64, Booleano)",
                campo.nombre, nombre_struct,
            ));
        }
    }

    // ─── Sintetizar el prólogo ─────────────────────────────────────────────
    let mut preludio: Vec<Sentencia> = Vec::new();

    // los __argv: Vector<Texto> = argumentos();
    preludio.push(declarar(
        Articulo::Los,
        "__argv",
        Tipo::Vector(Box::new(Tipo::Texto)),
        llamada("argumentos", vec![]),
    ));
    // el __n: Entero32 = vector_longitud<Texto>(__argv);
    preludio.push(declarar(
        Articulo::El,
        "__n",
        Tipo::Entero32,
        llamada_tipada("vector_longitud", Tipo::Texto, vec![id("__argv")]),
    ));

    // Variables temporales para cada campo con su default
    let nombre_var = |i: usize| format!("__campo_{}", i);
    for (i, campo) in struct_decl.campos.iter().enumerate() {
        let default = default_tipo(&campo.tipo);
        preludio.push(declarar(Articulo::El, &nombre_var(i), campo.tipo.clone(), default));
    }
    // Bandera de "se vio el campo" para requeridos (el/la)
    let mut bandera: Vec<(usize, String)> = Vec::new();
    for (i, campo) in struct_decl.campos.iter().enumerate() {
        if campo.articulo == Articulo::El || campo.articulo == Articulo::La {
            let f = format!("__visto_{}", i);
            bandera.push((i, f.clone()));
            preludio.push(declarar(Articulo::El, &f, Tipo::Booleano, Expresion::Literal(Literal::Booleano(false, Span::vacio()))));
        }
    }

    // Etiquetas Texto (texto_comparar exige Texto, no Palabra): una por campo + --ayuda + -h
    let nombre_etiqueta = |i: usize| format!("__etiqueta_{}", i);
    for (i, campo) in struct_decl.campos.iter().enumerate() {
        preludio.push(declarar(
            Articulo::El,
            &nombre_etiqueta(i),
            Tipo::Texto,
            llamada("texto_desde", vec![palabra(&format!("--{}", campo.nombre))]),
        ));
    }
    preludio.push(declarar(
        Articulo::El,
        "__etiqueta_ayuda",
        Tipo::Texto,
        llamada("texto_desde", vec![palabra("--ayuda")]),
    ));
    preludio.push(declarar(
        Articulo::El,
        "__etiqueta_h",
        Tipo::Texto,
        llamada("texto_desde", vec![palabra("-h")]),
    ));

    // el __i: Entero32 = 1;   (saltar argv[0] = nombre del programa)
    preludio.push(declarar(Articulo::El, "__i", Tipo::Entero32, Expresion::Literal(Literal::Entero(1, Span::vacio()))));

    // mientras __i < __n {
    //   el __arg: Texto = vector_obtener<Texto>(__argv, __i);
    //   ... para cada campo:
    //   si texto_comparar(__arg, __etiqueta_i) == 0 {
    //       el __val: Texto = vector_obtener<Texto>(__argv, __i + 1);
    //       __campo_i = <conversión>(__val);   // o directo para Texto
    //       __visto_i = verdadero;
    //       __i = __i + 1;
    //   }
    //   __i = __i + 1;
    // }
    let mut cuerpo_loop: Vec<Sentencia> = Vec::new();
    cuerpo_loop.push(declarar(
        Articulo::El,
        "__arg",
        Tipo::Texto,
        llamada_tipada("vector_obtener", Tipo::Texto, vec![id("__argv"), id("__i")]),
    ));

    // --ayuda automático
    let mut uso = format!("Uso: {}", func.nombre);
    for campo in &struct_decl.campos {
        uso.push_str(&format!(" [--{} <valor>]", campo.nombre));
    }
    let ayuda_body = vec![
        expr_sent(llamada("imprimir_linea", vec![palabra(&uso)])),
        expr_sent(llamada("imprimir_linea", vec![palabra(&format!("Ayuda: {}.", uso))])),
        retornar(Expresion::Literal(Literal::Entero(0, Span::vacio()))),
    ];
    cuerpo_loop.push(si(
        igual(llamada("texto_comparar", vec![id("__arg"), id("__etiqueta_ayuda")]), Expresion::Literal(Literal::Entero(0, Span::vacio()))),
        ayuda_body.clone(),
    ));
    cuerpo_loop.push(si(
        igual(llamada("texto_comparar", vec![id("__arg"), id("__etiqueta_h")]), Expresion::Literal(Literal::Entero(0, Span::vacio()))),
        ayuda_body.clone(),
    ));

    // Para cada campo: si __arg == __etiqueta_i { asignar y marcar visto }
    for (i, campo) in struct_decl.campos.iter().enumerate() {
        let mut body: Vec<Sentencia> = Vec::new();
        body.push(declarar(
            Articulo::El,
            "__val",
            Tipo::Texto,
            llamada_tipada("vector_obtener", Tipo::Texto, vec![id("__argv"), suma(id("__i"), Expresion::Literal(Literal::Entero(1, Span::vacio())))]),
        ));
        // Convertir según tipo
        let valor_conv = expr_conversion(&campo.tipo, id("__val"));
        body.push(asignar(&nombre_var(i), valor_conv));
        // Marcar visto si es requerido
        if let Some((_, f)) = bandera.iter().find(|(bi, _)| *bi == i) {
            body.push(asignar(f, Expresion::Literal(Literal::Booleano(true, Span::vacio()))));
        }
        body.push(asignar("__i", suma(id("__i"), Expresion::Literal(Literal::Entero(1, Span::vacio())))));
        cuerpo_loop.push(si(
            igual(llamada("texto_comparar", vec![id("__arg"), id(&nombre_etiqueta(i))]), Expresion::Literal(Literal::Entero(0, Span::vacio()))),
            body,
        ));
    }

    cuerpo_loop.push(asignar("__i", suma(id("__i"), Expresion::Literal(Literal::Entero(1, Span::vacio())))));
    preludio.push(mientras(menor(id("__i"), id("__n")), cuerpo_loop));

    // Validar requeridos: si !__visto_i { error; retornar 1; }
    for (i, campo) in struct_decl.campos.iter().enumerate() {
        let art = campo.articulo;
        if art != Articulo::El && art != Articulo::La {
            continue;
        }
        let f = format!("__visto_{}", i);
        // si !__visto_i → la condicion es: __visto_i == falso
        let cond = igual(id(&f), Expresion::Literal(Literal::Booleano(false, Span::vacio())));
        let msg = format!("Falta el argumento requerido: --{}", campo.nombre);
        preludio.push(si(cond, vec![
            expr_sent(llamada("imprimir_linea", vec![palabra(&msg)])),
            retornar(Expresion::Literal(Literal::Entero(1, Span::vacio()))),
        ]));
    }

    // Liberar etiquetas (los __val y __argv se liberan como Textos en vector_liberar)
    for (i, _) in struct_decl.campos.iter().enumerate() {
        preludio.push(expr_sent(llamada("texto_liberar", vec![id(&nombre_etiqueta(i))])));
    }
    preludio.push(expr_sent(llamada("texto_liberar", vec![id("__etiqueta_ayuda")])));
    preludio.push(expr_sent(llamada("texto_liberar", vec![id("__etiqueta_h")])));

    // Construir el struct: el args: MiStruct = MiStruct { campo0: __campo_0, ... }
    let mut inicializacion: Vec<(String, Expresion)> = Vec::new();
    for (i, campo) in struct_decl.campos.iter().enumerate() {
        inicializacion.push((campo.nombre.clone(), id(&nombre_var(i))));
    }
    let init_struct = Expresion::InicializacionStruct(nombre_struct.clone(), inicializacion, Span::vacio());
    preludio.push(declarar(param.articulo, &param.nombre, Tipo::Nombre(nombre_struct.clone()), init_struct));

    // ─── Reemplazar principal: sin parámetro ABI + prólogo + cuerpo original ──
    let mut nuevo_cuerpo = preludio;
    nuevo_cuerpo.extend(func.cuerpo.sentencias.clone());

    let nueva_func = FuncionDecl {
        nombre: func.nombre.clone(),
        parametros_genericos: func.parametros_genericos.clone(),
        parametros: vec![], // el SO llama a principal() sin args
        retorno: func.retorno.clone(),
        cuerpo: bloque(nuevo_cuerpo),
        es_insegura: func.es_insegura,
        es_vectorizable: func.es_vectorizable,
        nivel_verificacion: func.nivel_verificacion,
        efecto: func.efecto.clone(),
        visibilidad: func.visibilidad,
        es_futuro: func.es_futuro,
        span: func.span.clone(),
    };

    programa.declaraciones[idx] = Declaracion::Funcion(nueva_func);
    Ok(())
}
