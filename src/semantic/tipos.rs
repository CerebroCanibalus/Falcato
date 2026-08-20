//! Verificación de tipos, inferencia y compatibilidad

use super::*;

impl AnalizadorSemantico {
    /// Inferir tipo de expresión con verificación de concordancia
    pub(crate) fn inferir_tipo(&mut self, expr: &Expresion) -> Tipo {
        match expr {
            Expresion::Literal(lit) => self.tipo_literal(lit),
            Expresion::Ruta(path, span) => {
                // Ruta cualificada: modulo::simbolo (siempre referencia cruzada)
                let nombre_cualificado = path.join("::");
                if let Some(firma) = self.buscar_funcion(&nombre_cualificado, true, span) {
                    firma.retorno.clone().unwrap_or(Tipo::Entero32)
                } else if let Some(_ts) = self.structs.get(&nombre_cualificado).or_else(|| self.structs_importados.get(&nombre_cualificado)) {
                    Tipo::Nombre(nombre_cualificado)
                } else if let Some(_te) = self.enums.get(&nombre_cualificado).or_else(|| self.enums_importados.get(&nombre_cualificado)) {
                    Tipo::Nombre(nombre_cualificado)
                } else {
                    let sugerencia = sugerir_nombre(&path[0], &self.entorno.todos_nombres());
                    let msg = match sugerencia {
                        Some(ref s) => format!("'{}' no tiene concordancia en este contexto. ¿Quizás quisiste decir '{}'?", path[0], s),
                        None => format!("'{}' no tiene concordancia en este contexto", path[0]),
                    };
                    self.reportar_error(
                        CategoriaError::Tipo,
                        VARIABLE_NO_DECLARADA,
                        span,
                        msg,
                        Some(format!("¿Olvidaste declarar '{}' como módulo?", path[0]))
                    );
                    Tipo::Entero32
                }
            }
            Expresion::Propagacion(expr, span) => {
                // Verificar que expr es Resultado<T, E> u Option<T> y retornar T
                let tipo_expr = self.inferir_tipo(expr);
                match tipo_expr {
                    Tipo::Resultado(tipo_exito, _) => *tipo_exito,
                    Tipo::Option(tipo_exito) => *tipo_exito,
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            29,
                            span,
                            format!("El operador '?' requiere Resultado<T, E> u Option<T>, pero se encontró '{:?}'", tipo_expr),
                            Some("Usa '?' solo en expresiones de tipo Resultado u Option".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::Identificador(nombre, span) => {
                // Verificar use-after-move (solo en Nivel 1+)
                if self.nivel_verificacion_actual != crate::ast::NivelVerificacion::Permisivo 
                    && self.variables_movidas.contains(nombre) {
                    self.reportar_error(
                        CategoriaError::Ownership,
                        1,
                        span,
                        format!("'{}' fue movido y ya no es válido", nombre),
                        Some(format!(
                            "Si necesitas usar '{}' después:\n       │   opción A: copiar {} antes de pasar\n       │   opción B: pasar por referencia (&{})\n       │   opción C: reordenar para usar {} antes del move",
                            nombre, nombre, nombre, nombre
                        ))
                    );
                }
                
                match self.entorno.buscar(nombre) {
                    Some(info) => info.tipo.clone(),
                    None => {
                        // Buscar como const genérico
                        if let Some((tipo, _)) = self.entorno.buscar_const(nombre) {
                            tipo.clone()
                        } else {
                            let sugerencia = sugerir_nombre(nombre, &self.entorno.todos_nombres());
                            let msg = match sugerencia {
                                Some(ref s) => format!("'{}' no tiene concordancia en este contexto. ¿Quizás quisiste decir '{}'?", nombre, s),
                                None => format!("'{}' no tiene concordancia en este contexto. ¿Olvidaste declararlo con artículo?", nombre),
                            };
                            self.reportar_error(
                                CategoriaError::Tipo,
                                VARIABLE_NO_DECLARADA,
                                span,
                                msg,
                                Some("Los identificadores deben declararse con artículo: el, la, un, los, las".to_string())
                            );
                            Tipo::Entero32 // Tipo por defecto para continuar análisis
                        }
                    }
                }
            }
            Expresion::Binaria(izq, op, der, span) => {
                let tipo_izq = self.inferir_tipo(izq);
                let tipo_der = self.inferir_tipo(der);

                // Verificar concordancia de tipos en operación binaria.
                // Los literales numéricos se adaptan al tipo del otro operando
                // (ej: id: Entero64 + 1 → el 1 se trata como Entero64).
                let (tipo_izq, tipo_der) = self.adaptar_literales_binaria(izq, &tipo_izq, der, &tipo_der);
                if tipo_izq != tipo_der {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        DISCONCORDANCIA_OPERANDOS,
                        span,
                        format!("Disconcordancia de tipo en operación '{:?}': izquierda '{:?}', derecha '{:?}'",
                            op, tipo_izq, tipo_der),
                        Some("Ambos operandos deben ser del mismo tipo".to_string())
                    );
                }

                // Verificar división por cero en constantes
                if matches!(op, OperadorBinario::Division | OperadorBinario::Modulo) {
                    if let Expresion::Literal(Literal::Entero(valor, _)) = der.as_ref() {
                        if *valor == 0 {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                99, // T099: división por cero
                                span,
                                format!("División por cero en operación '{:?}'", op),
                                Some("El divisor no puede ser cero. Usa un valor distinto de cero.".to_string())
                            );
                        }
                    }
                }

                self.tipo_operacion(*op, &tipo_izq, span)
            }
            // R7.7 F1 — Subjuntivo aritmético: `a + b fuese` → Resultado<T, Entero32>
            Expresion::Checked(inner, span) => {
                match inner.as_ref() {
                    Expresion::Binaria(izq, op, der, span_bin) => {
                        let es_aritmetica = matches!(
                            op,
                            OperadorBinario::Suma
                                | OperadorBinario::Resta
                                | OperadorBinario::Multiplicacion
                        );
                        if !es_aritmetica {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                104,
                                span,
                                format!(
                                    "'fuese' (checked) solo aplica a suma, resta o multiplicación, no a '{:?}'",
                                    op
                                ),
                                Some("Usa 'fuese' para detectar desbordamiento: el x = a + b fuese".to_string())
                            );
                            return Tipo::Entero32;
                        }

                        let tipo_izq = self.inferir_tipo(izq);
                        let tipo_der = self.inferir_tipo(der);
                        let (tipo_izq, tipo_der) = self.adaptar_literales_binaria(izq, &tipo_izq, der, &tipo_der);
                        if tipo_izq != tipo_der {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                DISCONCORDANCIA_OPERANDOS,
                                span_bin,
                                format!("Disconcordancia de tipo en operación '{:?}': izquierda '{:?}', derecha '{:?}'",
                                    op, tipo_izq, tipo_der),
                                Some("Ambos operandos deben ser del mismo tipo".to_string())
                            );
                        }

                        let tipo_operacion = self.tipo_operacion(*op, &tipo_izq, span_bin);
                        let es_entero_32 = matches!(
                            tipo_operacion,
                            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                                | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32
                        );
                        if !es_entero_32 {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                105,
                                span,
                                format!(
                                    "'fuese' (checked) requiere enteros de 32 bits o menos, pero la operación es '{:?}'",
                                    tipo_operacion
                                ),
                                Some("F1: soporta Entero8/16/32 y Natural8/16/32; Entero64/Natural64 pendiente (F1.1)".to_string())
                            );
                            return Tipo::Entero32;
                        }

                        Tipo::Resultado(Box::new(tipo_operacion), Box::new(Tipo::Entero32))
                    }
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            106,
                            span,
                            format!("'fuese' (checked) solo aplica a operaciones aritméticas, no a esta expresión"),
                            Some("Escribe la operación completa: el x = a + b fuese".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::Unaria(op, expr, span) => {
                let tipo = self.inferir_tipo(expr);
                match op {
                    OperadorUnario::Referencia => {
                        if self.nivel_verificacion_actual == crate::ast::NivelVerificacion::Estricto {
                            if let Some(path) = self.extraer_path(expr) {
                                let estado = self.borrows.get(&path).copied().unwrap_or(BorrowState::None);
                                match estado {
                                    BorrowState::Exclusive => {
                                        self.reportar_error(
                                            CategoriaError::Ownership,
                                            2,
                                            span,
                                            format!("No se puede crear referencia inmutable a '{}': ya tiene borrow mutable (&mut)", path),
                                            Some("Espera a que el borrow mutable termine antes de crear uno inmutable".to_string())
                                        );
                                    }
                                    BorrowState::Shared(n) => {
                                        self.borrows.insert(path.clone(), BorrowState::Shared(n + 1));
                                    }
                                    BorrowState::None => {
                                        self.borrows.insert(path.clone(), BorrowState::Shared(1));
                                    }
                                }
                            }
                        }
                        Tipo::Referencia(Box::new(tipo))
                    }
                    OperadorUnario::ReferenciaMut => {
                        if self.nivel_verificacion_actual == crate::ast::NivelVerificacion::Estricto {
                            if let Some(path) = self.extraer_path(expr) {
                                let estado = self.borrows.get(&path).copied().unwrap_or(BorrowState::None);
                                match estado {
                                    BorrowState::Exclusive => {
                                        self.reportar_error(
                                            CategoriaError::Ownership,
                                            3,
                                            span,
                                            format!("No se puede crear referencia mutable a '{}': ya tiene borrow mutable (&mut)", path),
                                            Some(format!(
                                                "Solo puede existir un borrow mutable a la vez.\n       │   opción A: usa el borrow mutable existente\n       │   opción B: reordena para que los borrows no se solapen\n       │   opción C: usa 'copiar {}' para trabajar con una copia", path
                                            ))
                                        );
                                    }
                                    BorrowState::Shared(_) => {
                                        self.reportar_error(
                                            CategoriaError::Ownership,
                                            4,
                                            span,
                                            format!("No se puede crear referencia mutable a '{}': ya tiene borrows inmutables (&)", path),
                                            Some(format!(
                                                "Espera a que los borrows inmutables terminen.\n       │   opción A: reordena para que el borrow mutable vaya primero\n       │   opción B: usa un scope ({{ ... }}) para limitar el borrow inmutable\n       │   opción C: usa 'copiar {}' para mutar una copia", path
                                            ))
                                        );
                                    }
                                    BorrowState::None => {
                                        self.borrows.insert(path.clone(), BorrowState::Exclusive);
                                    }
                                }
                            }
                        }
                        Tipo::ReferenciaMut(Box::new(tipo))
                    }
                    OperadorUnario::Desreferencia => {
                        match tipo {
                            Tipo::Referencia(t) | Tipo::ReferenciaMut(t) |
                            Tipo::ReferenciaConLifetime(_, t) | Tipo::ReferenciaMutConLifetime(_, t) |
                            Tipo::ReferenciaSelf(t) | Tipo::ReferenciaMutSelf(t) => *t,
                            _ => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    30,
                                    span,
                                    format!("No se puede desreferenciar tipo '{:?}' (no es una referencia)", tipo),
                                    Some("Usa '*' solo en referencias (&T o &mut T)".to_string())
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                    _ => self.tipo_operacion_unaria(*op, &tipo, span)
                }
            }
            Expresion::Llamada(llamada) => {
                let (nombre_resuelto, es_referencia_cruzada, viene_de_import) =
                    if llamada.funcion.contains("::") {
                        (llamada.funcion.clone(), true, false)
                    } else if self.funciones.contains_key(&llamada.funcion) {
                        (llamada.funcion.clone(), false, false)
                    } else {
                        let nombre_con_modulo = self.nombre_con_modulo(&llamada.funcion);
                        if self.funciones.contains_key(&nombre_con_modulo) {
                            (nombre_con_modulo, false, false)
                        } else if let Some(cualificado) = self.imports.get(&llamada.funcion) {
                            (cualificado.clone(), true, true)
                        } else if let Some(cualificado) = self.resolver_glob(&llamada.funcion) {
                            (cualificado, true, true)
                        } else {
                            (llamada.funcion.clone(), false, false)
                        }
                    };

                let firma_opt = self.buscar_funcion(&nombre_resuelto, es_referencia_cruzada, &llamada.span);
                match firma_opt {
                    Some(firma) => {
                        let firma_efectiva = if llamada.tipo_args.is_empty() {
                            firma.clone()
                        } else {
                            match self.aplicar_tipo_args_a_firma(&firma, &llamada.tipo_args, &llamada.span) {
                                Some(f) => f,
                                None => return Tipo::Entero32,
                            }
                        };

                        if llamada.argumentos.len() != firma_efectiva.parametros.len() {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                22,
                                &llamada.span,
                                format!("Función '{}' espera {} argumentos, pero se pasaron {}",
                                    llamada.funcion, firma_efectiva.parametros.len(), llamada.argumentos.len()),
                                None
                            );
                        } else {
                            let es_polimorfica = llamada.funcion == "imprimir" || llamada.funcion == "imprimir_linea" || llamada.funcion == "decir";
                            let es_conversion_numerica = matches!(llamada.funcion.as_str(),
                                "como_entero8" | "como_entero16" | "como_entero32" | "como_entero64" |
                                "como_flotante32" | "como_flotante64");
                            if !es_polimorfica {
                                for (i, (arg, (nombre_param, tipo_param))) in 
                                    llamada.argumentos.iter().zip(firma_efectiva.parametros.iter()).enumerate() {
                                    let tipo_arg = self.inferir_tipo(arg);
                                    let compatible = if es_conversion_numerica {
                                        matches!(self.resolver_alias(&tipo_arg),
                                            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                                            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
                                            Tipo::Flotante32 | Tipo::Flotante64)
                                    } else {
                                        self.tipos_compatibles(tipo_param, &tipo_arg)
                                    };
                                    if !compatible {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            DISCONCORDANCIA_TIPO,
                                            &llamada.span,
                                            format!("Argumento {} ('{}') de '{}': espera '{:?}', encontrado '{:?}'",
                                                i + 1, nombre_param, llamada.funcion, tipo_param, tipo_arg),
                                            Some(format!("Cambia el argumento a tipo '{:?}'", tipo_param))
                                        );
                                    }
                                }
                            } else {
                                for arg in &llamada.argumentos {
                                    self.inferir_tipo(arg);
                                }
                            }
                        }
                        firma_efectiva.retorno.clone().unwrap_or(Tipo::Entero32)
                    }
                    None => {
                        if viene_de_import {
                            self.reportar_error(
                                CategoriaError::Modulos,
                                SIMBOLO_NO_ENCONTRADO,
                                &llamada.span,
                                format!("Función importada '{}' no encontrada o no es pública", llamada.funcion),
                                Some("Verifica que el módulo exporte la función con 'el función'".to_string())
                            );
                        }
                        Tipo::Entero32
                    }
                }
            }
            Expresion::ArrayRelleno(elem, _, _span) => {
                let tipo_elem = self.inferir_tipo(elem);
                Tipo::Array(Box::new(tipo_elem), 0)
            }
            Expresion::AccesoArray(array, indice, span) => {
                let tipo_array = self.inferir_tipo(array);
                
                if tipo_array == Tipo::Texto {
                    let es_rango = matches!(indice.as_ref(), Expresion::Rango(_, _, _, _));
                    if es_rango {
                        return Tipo::Texto;
                    }
                    return Tipo::Entero8;
                }
                
                if let Tipo::Vector(tipo_elem) = &tipo_array {
                    return *tipo_elem.clone();
                }
                
                let tipo_indice = self.inferir_tipo(indice);
                if tipo_indice != Tipo::Entero32 && tipo_indice != Tipo::Entero64 {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        15,
                        span,
                            format!("Índice de arreglo debe ser Entero, encontrado '{:?}'", tipo_indice),
                        Some("Usa un valor Entero como índice".to_string())
                    );
                }
                
                match tipo_array {
                    Tipo::Array(tipo_elem, _) | Tipo::ArrayGenerico(tipo_elem, _) => *tipo_elem,
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            16,
                            span,
                            format!("Acceso a arreglo en tipo '{:?}' que no es arreglo", tipo_array),
                            None
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::LiteralArray(elementos, span) => {
                if elementos.is_empty() {
                    Tipo::Array(Box::new(Tipo::Entero32), 0)
                } else {
                    let tipo = self.inferir_tipo(&elementos[0]);
                    for (i, elem) in elementos.iter().enumerate().skip(1) {
                        let tipo_elem = self.inferir_tipo(elem);
                        if tipo_elem != tipo {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                17,
                                span,
                                format!("Elemento {} del arreglo es '{:?}' pero se espera '{:?}'", i, tipo_elem, tipo),
                                Some("Todos los elementos de un arreglo deben ser del mismo tipo".to_string())
                            );
                        }
                    }
                    Tipo::Array(Box::new(tipo), elementos.len())
                }
            }
            Expresion::InicializacionStruct(nombre, campos, span) => {
                let info_opt = self.buscar_struct(nombre);
                match info_opt {
                    Some(info) => {
                        if !info.campos_bits.is_empty() && info.campos.is_empty() {
                            for (nombre_campo, valor) in campos {
                                let _tipo_valor = self.inferir_tipo(valor);
                                if !info.campos_bits.iter().any(|c| c.nombre == *nombre_campo) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        18,
                                        span,
                                        format!("El struct '{}' no tiene campo '{}'", nombre, nombre_campo),
                                        None
                                    );
                                }
                            }
                            return Tipo::Nombre(nombre.clone());
                        }

                        let mut campos_vistos = std::collections::HashSet::new();
                        for (nombre_campo, valor) in campos {
                            let tipo_valor = self.inferir_tipo(valor);
                            match info.campos.iter().find(|c| c.nombre == *nombre_campo) {
                                Some(campo) => {
                                    if !self.tipos_compatibles(&campo.tipo, &tipo_valor) {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            DISCONCORDANCIA_TIPO,
                                            span,
                                            format!("Campo '{}' de struct '{}' es '{:?}' pero se asigna '{:?}'",
                                                nombre_campo, nombre, campo.tipo, tipo_valor),
                                            Some(format!("Cambia el tipo a '{:?}' o el valor", tipo_valor))
                                        );
                                    }
                                }
                                None => {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        18,
                                        span,
                                        format!("El struct '{}' no tiene campo '{}'", nombre, nombre_campo),
                                        None
                                    );
                                }
                            }
                            campos_vistos.insert(nombre_campo.clone());
                        }
                        for campo in &info.campos {
                            if !campos_vistos.contains(&campo.nombre) {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    19,
                                    span,
                                    format!("Falta campo '{}' en inicialización de struct '{}'", campo.nombre, nombre),
                                    None
                                );
                            }
                        }
                        Tipo::Nombre(nombre.clone())
                    }
                    None => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            20,
                            span,
                            format!("Struct '{}' no declarado", nombre),
                            Some("Declara el struct con 'estructural {} {{ ... }}'".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, span) => {
                let info_opt = self.buscar_enum(enum_nombre);
                match info_opt {
                    Some(info) => {
                        match info.variantes.iter().find(|v| v.nombre == *variante_nombre) {
                            Some(variante) => {
                                if let Some(ref campos) = variante.datos {
                                    if argumentos.len() != campos.len() {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            24,
                                            span,
                                            format!("Constructor '{}' de '{}' espera {} argumentos, pero se pasaron {}",
                                                variante_nombre, enum_nombre, campos.len(), argumentos.len()),
                                            None
                                        );
                                    } else {
                                        for (i, (arg, (nombre_campo, tipo_campo))) in
                                            argumentos.iter().zip(campos.iter()).enumerate() {
                                            let tipo_arg = self.inferir_tipo(arg);
                                            if matches!(tipo_campo, Tipo::Generico(_)) {
                                                // Aceptado
                                            } else if tipo_arg != *tipo_campo {
                                                self.reportar_error(
                                                    CategoriaError::Tipo,
                                                    DISCONCORDANCIA_TIPO,
                                                    span,
                                                    format!("Argumento {} ('{}') de '{}.{}': espera '{:?}', encontrado '{:?}'",
                                                        i + 1, nombre_campo, enum_nombre, variante_nombre, tipo_campo, tipo_arg),
                                                    Some(format!("Cambia el argumento a tipo '{:?}'", tipo_campo))
                                                );
                                            }
                                        }
                                    }
                                } else if !argumentos.is_empty() {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        25,
                                        span,
                                        format!("Variante '{}.{}' no tiene datos, pero se pasaron {} argumentos",
                                            enum_nombre, variante_nombre, argumentos.len()),
                                        None
                                    );
                                }
                                
                                if !info.parametros_genericos.is_empty() {
                                    if enum_nombre == "Resultado" && info.parametros_genericos.len() == 2 {
                                        if variante_nombre == "Exito" && !argumentos.is_empty() {
                                            let tipo_t = self.inferir_tipo(&argumentos[0]);
                                            Tipo::Resultado(Box::new(tipo_t), Box::new(Tipo::Entero32))
                                        } else if variante_nombre == "Error" && !argumentos.is_empty() {
                                            let tipo_e = self.inferir_tipo(&argumentos[0]);
                                            Tipo::Resultado(Box::new(Tipo::Entero32), Box::new(tipo_e))
                                        } else {
                                            Tipo::Resultado(Box::new(Tipo::Entero32), Box::new(Tipo::Entero32))
                                        }
                                    } else {
                                        let tipos_inferidos: Vec<Tipo> = info.parametros_genericos.iter()
                                            .map(|_| Tipo::Entero32)
                                            .collect();
                                        Tipo::NombreGenerico(enum_nombre.clone(), tipos_inferidos)
                                    }
                                } else {
                                    Tipo::Nombre(enum_nombre.clone())
                                }
                            }
                            None => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    26,
                                    span,
                                    format!("La enumeración '{}' no tiene variante '{}'", enum_nombre, variante_nombre),
                                    None
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                    None => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            27,
                            span,
                            format!("Enumeración '{}' no declarada", enum_nombre),
                            Some("Declara la enumeración con 'enumeración {} { ... }'".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::EsVariante(expr, enum_nombre, variante_nombre, _binding, span) => {
                let tipo_expr = self.inferir_tipo(expr);
                let tipo_es_enum = match &tipo_expr {
                    Tipo::Nombre(n) if n == enum_nombre => true,
                    Tipo::Resultado(_, _) if enum_nombre == "Resultado" => true,
                    Tipo::Option(_) if enum_nombre == "Option" => true,
                    _ => false,
                };
                
                if !tipo_es_enum {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        28,
                        span,
                        format!("Pattern matching en tipo '{:?}', pero se esperaba '{}'", tipo_expr, enum_nombre),
                        None
                    );
                }
                if let Some(info) = self.buscar_enum(enum_nombre) {
                    if !info.variantes.iter().any(|v| v.nombre == *variante_nombre) {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            26,
                            span,
                            format!("La enumeración '{}' no tiene variante '{}'", enum_nombre, variante_nombre),
                            None
                        );
                    }
                }
                Tipo::Booleano
            }
            Expresion::AccesoCampo(expr, nombre_campo, span) => {
                let tipo_expr = self.inferir_tipo(expr);
                match &tipo_expr {
                    Tipo::Nombre(nombre_struct) => {
                        let info_opt = self.buscar_struct(nombre_struct);
                        match info_opt {
                            Some(info) => {
                                if let Some(campo_bit) = info.campos_bits.iter().find(|c| c.nombre == *nombre_campo) {
                                    let _ = campo_bit;
                                    return Tipo::Entero32;
                                }
                                match info.campos.iter().find(|c| c.nombre == *nombre_campo) {
                                    Some(campo) => campo.tipo.clone(),
                                    None => {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            18,
                                            span,
                                            format!("El struct '{}' no tiene campo '{}'", nombre_struct, nombre_campo),
                                            None
                                        );
                                        Tipo::Entero32
                                    }
                                }
                            }
                            None => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    20,
                                    span,
                                    format!("Struct '{}' no declarado", nombre_struct),
                                    None
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            21,
                            span,
                            format!("Acceso a campo '{}' en tipo '{:?}' que no es struct", nombre_campo, tipo_expr),
                            None
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::Mover(nombre, _destino, span) => {
                let tipo = match self.entorno.buscar(nombre) {
                    Some(info) => info.tipo.clone(),
                    None => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            VARIABLE_NO_DECLARADA,
                            span,
                            format!("'{}' no tiene concordancia en este contexto (mover)", nombre),
                            Some("Declara la variable con artículo antes de moverla".to_string())
                        );
                        Tipo::Entero32
                    }
                };
                
                self.variables_movidas.insert(nombre.clone());
                
                tipo
            }
            Expresion::Copiar(expr, _span) => {
                self.inferir_tipo(expr)
            }
            Expresion::Rango(inicio, _fin, _inclusivo, _span) => {
                self.inferir_tipo(inicio)
            }
            Expresion::Closure(params, cuerpo, _span) => {
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);

                for (nombre, tipo_opt) in params {
                    let tipo = tipo_opt.clone().unwrap_or(Tipo::Entero32);
                    self.entorno.declarar(InfoVariable {
                        nombre: nombre.clone(),
                        tipo,
                        articulo: Articulo::La,
                        span: _span.clone(),
                    });
                }

                let _tipo_cuerpo = self.inferir_tipo(cuerpo);

                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));

                Tipo::Entero64
            }
            Expresion::Coincidir(sujeto, brazos, span) => {
                let tipo_sujeto = self.inferir_tipo(sujeto);

                if brazos.is_empty() {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        70,
                        span,
                        "'coincidir' requiere al menos un brazo".to_string(),
                        Some("Agrega al menos un patrón: coincidir x { _ => ... }".to_string()),
                    );
                    return Tipo::Entero32;
                }

                let mut tiene_comodin = false;
                let mut tipo_resultado: Option<Tipo> = None;

                for brazo in brazos {
                    match &brazo.patron {
                        crate::ast::PatronMatch::Comodin(_) => {
                            tiene_comodin = true;
                        }
                        crate::ast::PatronMatch::Literal(lit) => {
                            let tipo_lit = self.tipo_literal(lit);
                            if tipo_lit != tipo_sujeto {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    71,
                                    &brazo.span,
                                    format!("Disconcordancia en patrón: el sujeto es '{}' pero el patrón es '{}'", self.nombre_tipo_string(&tipo_sujeto), self.nombre_tipo_string(&tipo_lit)),
                                    Some("El patrón debe ser del mismo tipo que el sujeto".to_string()),
                                );
                            }
                        }
                        crate::ast::PatronMatch::VarianteEnum(enum_nombre, variante, binding, span_pat) => {
                            if let Some(info_enum) = self.buscar_enum(enum_nombre) {
                                if !info_enum.variantes.iter().any(|v| &v.nombre == variante) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        72,
                                        span_pat,
                                        format!("La variante '{}' no existe en la enumeración '{}'", variante, enum_nombre),
                                        Some(format!("Variantes disponibles: {}", info_enum.variantes.iter().map(|v| v.nombre.as_str()).collect::<Vec<_>>().join(", "))),
                                    );
                                }
                            }
                            if let Some(nombre_binding) = binding {
                                self.entorno.declarar(InfoVariable {
                                    nombre: nombre_binding.clone(),
                                    tipo: Tipo::Entero32,
                                    articulo: Articulo::La,
                                    span: span_pat.clone(),
                                });
                            }
                        }
                    }

                    let tipo_cuerpo = self.inferir_tipo(&brazo.cuerpo);
                    if let Some(ref tipo_previo) = tipo_resultado {
                        if *tipo_previo != tipo_cuerpo {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                73,
                                &brazo.span,
                                format!("Todos los brazos de 'coincidir' deben retornar el mismo tipo: se esperaba '{}' pero este brazo retorna '{}'", self.nombre_tipo_string(tipo_previo), self.nombre_tipo_string(&tipo_cuerpo)),
                                Some("Unifica los tipos de retorno de todos los brazos".to_string()),
                            );
                        }
                    } else {
                        tipo_resultado = Some(tipo_cuerpo);
                    }
                }

                if !tiene_comodin && matches!(tipo_sujeto, Tipo::Entero32 | Tipo::Entero64 | Tipo::Natural32 | Tipo::Natural64) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        74,
                        span,
                        "'coincidir' no es exhaustivo: faltan casos por cubrir".to_string(),
                        Some("Agrega un brazo comodín: _ => ...".to_string()),
                    );
                }

                tipo_resultado.unwrap_or(Tipo::Entero32)
            }
            Expresion::Esperar(expr_interno, span) => {
                let dentro_de_fut = self.funcion_actual.as_ref()
                    .map(|f| f.es_futuro)
                    .unwrap_or(false);
                if !dentro_de_fut {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        80,
                        span,
                        "'esperar' solo puede usarse dentro de 'fut función'".to_string(),
                        Some("Marca la función como async: 'fut función nombre(...) { ... }'".to_string()),
                    );
                }
                let tipo = self.inferir_tipo(expr_interno);
                tipo
            }
            Expresion::Lanzar(expr_interno, _span) => {
                let _tipo = self.inferir_tipo(expr_interno);
                Tipo::Entero64
            }
            Expresion::Bloquear(expr_interno, span) => {
                let dentro_de_fut = self.funcion_actual.as_ref()
                    .map(|f| f.es_futuro)
                    .unwrap_or(false);
                if dentro_de_fut {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        84,
                        span,
                        "'bloquear()' dentro de 'fut función' causaría deadlock".to_string(),
                        Some("Usa 'esperar' en su lugar dentro de funciones async".to_string()),
                    );
                }
                let tipo = self.inferir_tipo(expr_interno);
                tipo
            }
            Expresion::DireccionDe(nombre_funcion, span) => {
                let funcion_existe = self.funciones.contains_key(nombre_funcion)
                    || self.simbolos_publicos_importados.contains_key(nombre_funcion);
                if !funcion_existe {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        85,
                        span,
                        format!("Función '{}' no encontrada para 'direccion_de'", nombre_funcion),
                        Some("Asegúrate de que la función exista y sea accesible en el ámbito actual".to_string()),
                    );
                }
                Tipo::Entero64
            }
            Expresion::Bloque(bloque) => {
                for sentencia in &bloque.sentencias {
                    self.analizar_sentencia(sentencia);
                }
                if let Some(ultima) = bloque.sentencias.last() {
                    match ultima {
                        Sentencia::Expresion(expr) => self.inferir_tipo(expr),
                        Sentencia::Retornar(Some(expr), _) => self.inferir_tipo(expr),
                        _ => Tipo::Vacio,
                    }
                } else {
                    Tipo::Vacio
                }
            }
            Expresion::Metodo(receptor, nombre, args, span) => {
                let tipo_receptor = self.inferir_tipo(receptor);
                
                if let Some(builtin) = metodo_a_builtin(&tipo_receptor, nombre) {
                    if let Some(firma) = self.funciones.get(builtin) {
                        let esperado_args = if builtin.ends_with("_nuevo") || builtin.ends_with("_desde") {
                            let total_params = firma.parametros.len();
                            if total_params > 0 { total_params - 1 } else { 0 }
                        } else if builtin.ends_with("_concatenar") || builtin.ends_with("_comparar") {
                            let total_params = firma.parametros.len();
                            if total_params > 0 { total_params - 1 } else { 0 }
                        } else {
                            let total_params = firma.parametros.len();
                            if total_params > 0 { total_params - 1 } else { 0 }
                        };
                        let tipo_retorno = firma.retorno.clone().unwrap_or(Tipo::Entero32);
                        
                        if args.len() != esperado_args {
                            self.reportar_error(
                                CategoriaError::Tipo, 1, span,
                                format!(".{} requiere {} argumento(s), se pasaron {}", nombre, esperado_args, args.len()),
                                None,
                            );
                        }
                        
                        tipo_retorno
                    } else {
                        Tipo::Entero32
                    }
                } else {
                    let es_entero = matches!(&tipo_receptor,
                        Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                        Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64
                    );
                    
                    if es_entero {
                        match nombre.as_str() {
                            "poner_bit" | "quitar_bit" | "alternar_bit" => {
                                if args.len() != 1 {
                                    self.reportar_error(CategoriaError::Tipo, 1, span,
                                        format!(".{} requiere exactamente 1 argumento (posición del bit)", nombre), None);
                                }
                            }
                            "extraer_bits" => {
                                if args.len() != 2 {
                                    self.reportar_error(CategoriaError::Tipo, 1, span,
                                        ".extraer_bits requiere 2 argumentos (offset, cantidad)".to_string(), None);
                                }
                            }
                            "ceros_izquierda" | "unos" => {
                                if !args.is_empty() {
                                    self.reportar_error(CategoriaError::Tipo, 1, span,
                                        format!(".{} no acepta argumentos", nombre), None);
                                }
                            }
                            _ => {
                                self.reportar_error(CategoriaError::Tipo, 1, span,
                                    format!("Tipo '{:?}' no tiene método '.{}'", tipo_receptor, nombre),
                                    Some("Revisa el nombre del método. Para enteros: poner_bit, quitar_bit, alternar_bit, extraer_bits, ceros_izquierda, unos. Para Texto: agregar, tam, liberar, obtener, concatenar, subtexto, comparar, desde. Para Vector: agregar, tam, obtener, liberar.".to_string()),
                                );
                            }
                        }
                        tipo_receptor
                    } else {
                        self.reportar_error(CategoriaError::Tipo, 1, span,
                            format!("Tipo '{:?}' no tiene método '.{}'", tipo_receptor, nombre),
                            Some("Los métodos disponibles dependen del tipo. Para enteros: poner_bit, quitar_bit, etc. Para Texto: agregar, tam, etc.".to_string()),
                        );
                        Tipo::Entero32
                    }
                }
            }
        }
    }

    pub(crate) fn tipo_literal(&self, lit: &Literal) -> Tipo {
        match lit {
            Literal::Entero(_, _) => Tipo::Entero32,
            Literal::Flotante(_, _) => Tipo::Flotante64,
            Literal::Palabra(_, _) => Tipo::Palabra,
            Literal::Caracter(_, _) => Tipo::Caracter,
            Literal::Booleano(_, _) => Tipo::Booleano,
        }
    }

    pub(crate) fn tipo_operacion(&mut self, op: OperadorBinario, tipo: &Tipo, span: &Span) -> Tipo {
        match op {
            OperadorBinario::Suma => {
                if *tipo == Tipo::Texto {
                    tipo.clone()
                } else if !self.es_numerico(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operación '+' no válida para tipo '{:?}'. Se requiere tipo numérico (Entero o Real) o Texto para concatenación", tipo),
                        None
                    );
                    Tipo::Entero32
                } else {
                    tipo.clone()
                }
            }
            OperadorBinario::Resta |
            OperadorBinario::Multiplicacion |
            OperadorBinario::Division |
            OperadorBinario::Modulo => {
                if !self.es_numerico(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operación aritmética no válida para tipo '{:?}'. Se requiere tipo numérico (Entero o Real)", tipo),
                        None
                    );
                }
                tipo.clone()
            }
            OperadorBinario::Igual |
            OperadorBinario::Distinto |
            OperadorBinario::Menor |
            OperadorBinario::Mayor |
            OperadorBinario::MenorIgual |
            OperadorBinario::MayorIgual => {
                if !self.es_comparable(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        COMPARACION_INVALIDA,
                        span,
                        format!("Comparación no válida para tipo '{:?}'", tipo),
                        None
                    );
                }
                Tipo::Booleano
            }
            OperadorBinario::Y |
            OperadorBinario::O => {
                if *tipo != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_LOGICA_INVALIDA,
                        span,
                        format!("Operación lógica requiere Booleano, encontrado '{:?}'", tipo),
                        None
                    );
                }
                Tipo::Booleano
            }
            OperadorBinario::BitAnd |
            OperadorBinario::BitOr |
            OperadorBinario::BitXor |
            OperadorBinario::ShiftLeft |
            OperadorBinario::ShiftRight |
            OperadorBinario::ShiftRightLogico => {
                if !self.es_entero(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operación bitwise requiere tipo entero, encontrado '{:?}'", tipo),
                        Some("Los operadores &, |, ^, <<, >> solo funcionan con Entero8/16/32/64 o Natural8/16/32/64".to_string())
                    );
                }
                tipo.clone()
            }
        }
    }

    pub(crate) fn tipo_operacion_unaria(&mut self, op: OperadorUnario, tipo: &Tipo, span: &Span) -> Tipo {
        match op {
            OperadorUnario::Negacion => {
                if !self.es_numerico(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        NEGACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Negación aritmética no válida para tipo '{:?}'", tipo),
                        None
                    );
                }
                tipo.clone()
            }
            OperadorUnario::NegacionLogica => {
                if *tipo != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        NEGACION_LOGICA_INVALIDA,
                        span,
                        format!("Negación lógica requiere Booleano, encontrado '{:?}'", tipo),
                        None
                    );
                }
                Tipo::Booleano
            }
            OperadorUnario::BitNot => {
                if !self.es_entero(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operador ~ (bitwise NOT) requiere tipo entero, encontrado '{:?}'", tipo),
                        Some("Usa ~ solo con Entero8/16/32/64 o Natural8/16/32/64".to_string())
                    );
                }
                tipo.clone()
            }
            _ => tipo.clone(),
        }
    }

    pub(crate) fn resolver_alias(&self, tipo: &Tipo) -> Tipo {
        match tipo {
            Tipo::Nombre(nombre) => {
                if let Some(alias) = self.aliases.get(nombre) {
                    self.resolver_alias(alias)
                } else {
                    tipo.clone()
                }
            }
            Tipo::Vector(inner) => Tipo::Vector(Box::new(self.resolver_alias(inner))),
            Tipo::Array(inner, n) => Tipo::Array(Box::new(self.resolver_alias(inner)), *n),
            Tipo::Puntero(inner) => Tipo::Puntero(Box::new(self.resolver_alias(inner))),
            Tipo::Referencia(inner) => Tipo::Referencia(Box::new(self.resolver_alias(inner))),
            Tipo::ReferenciaMut(inner) => Tipo::ReferenciaMut(Box::new(self.resolver_alias(inner))),
            _ => tipo.clone(),
        }
    }

    pub(crate) fn tipos_compatibles(&self,
        tipo_param: &Tipo,
        tipo_arg: &Tipo,
    ) -> bool {
        let tipo_param = self.resolver_alias(tipo_param);
        let tipo_arg = self.resolver_alias(tipo_arg);
        if tipo_param == tipo_arg {
            return true;
        }

        match (&tipo_param, &tipo_arg) {
            (Tipo::Generico(_), _) | (_, Tipo::Generico(_)) => true,
            (Tipo::Entero64, Tipo::Array(_, _)) => true,
            (Tipo::ArrayGenerico(elem_param, _), Tipo::Array(elem_arg, _)) => self.tipos_compatibles(elem_param, elem_arg),
            (Tipo::Array(elem_param, _), Tipo::Array(elem_arg, _)) => self.tipos_compatibles(elem_param, elem_arg),
            (Tipo::Puntero(p), Tipo::Puntero(a)) => self.tipos_compatibles(p, a),
            (Tipo::Referencia(p), Tipo::Referencia(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaConLifetime(_, p), Tipo::Referencia(a)) => self.tipos_compatibles(p, a),
            (Tipo::Referencia(p), Tipo::ReferenciaConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaConLifetime(_, p), Tipo::ReferenciaConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutConLifetime(_, p), Tipo::ReferenciaMut(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMut(p), Tipo::ReferenciaMutConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutConLifetime(_, p), Tipo::ReferenciaMutConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaSelf(p), Tipo::Referencia(a)) => self.tipos_compatibles(p, a),
            (Tipo::Referencia(p), Tipo::ReferenciaSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaSelf(p), Tipo::ReferenciaSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutSelf(p), Tipo::ReferenciaMut(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMut(p), Tipo::ReferenciaMutSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutSelf(p), Tipo::ReferenciaMutSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaSelf(p), Tipo::ReferenciaConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaConLifetime(_, p), Tipo::ReferenciaSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutSelf(p), Tipo::ReferenciaMutConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutConLifetime(_, p), Tipo::ReferenciaMutSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::Entero64, Tipo::Referencia(_)) |
            (Tipo::Entero64, Tipo::ReferenciaMut(_)) |
            (Tipo::Entero64, Tipo::ReferenciaConLifetime(_, _)) |
            (Tipo::Entero64, Tipo::ReferenciaMutConLifetime(_, _)) |
            (Tipo::Entero64, Tipo::ReferenciaSelf(_)) |
            (Tipo::Entero64, Tipo::ReferenciaMutSelf(_)) => true,
            _ => false,
        }
    }

    pub(crate) fn es_numerico(&self, tipo: &Tipo) -> bool {
        match tipo {
            Tipo::Generico(nombre) => self.tiene_bound(nombre, "Numérico"),
            _ => matches!(tipo,
                Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
                Tipo::Flotante32 | Tipo::Flotante64
            ),
        }
    }

    pub(crate) fn es_entero(&self, tipo: &Tipo) -> bool {
        matches!(tipo,
            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64
        )
    }

    pub(crate) fn es_flotante(&self, tipo: &Tipo) -> bool {
        matches!(tipo, Tipo::Flotante32 | Tipo::Flotante64)
    }

    pub(crate) fn adaptar_literales_binaria(&self, izq: &Expresion, tipo_izq: &Tipo, der: &Expresion, tipo_der: &Tipo) -> (Tipo, Tipo) {
        let es_literal_entero = |e: &Expresion| match e {
            Expresion::Literal(Literal::Entero(_, _)) => true,
            Expresion::Unaria(_, inner, _) => matches!(inner.as_ref(), Expresion::Literal(Literal::Entero(_, _))),
            _ => false,
        };
        let es_literal_flotante = |e: &Expresion| matches!(e, Expresion::Literal(Literal::Flotante(_, _)));

        match (es_literal_entero(izq), es_literal_entero(der), es_literal_flotante(izq), es_literal_flotante(der)) {
            (true, false, _, _) if self.es_numerico(tipo_der) && !es_literal_flotante(izq) => (tipo_der.clone(), tipo_der.clone()),
            (false, true, _, _) if self.es_numerico(tipo_izq) && !es_literal_flotante(der) => (tipo_izq.clone(), tipo_izq.clone()),
            (_, _, true, false) if self.es_flotante(tipo_der) => (tipo_der.clone(), tipo_der.clone()),
            (_, _, false, true) if self.es_flotante(tipo_izq) => (tipo_izq.clone(), tipo_izq.clone()),
            _ => (tipo_izq.clone(), tipo_der.clone()),
        }
    }

    pub(crate) fn es_comparable(&self, tipo: &Tipo) -> bool {
        match tipo {
            Tipo::Generico(nombre) => {
                self.tiene_bound(nombre, "Comparable") || self.tiene_bound(nombre, "Ordenable")
            }
            _ => self.es_numerico(tipo) || matches!(tipo, Tipo::Caracter | Tipo::Booleano),
        }
    }

    pub(crate) fn nombre_tipo_string(&self, tipo: &Tipo) -> String {
        match tipo {
            Tipo::Nombre(n) => n.clone(),
            Tipo::Generico(n) => n.clone(),
            _ => format!("{:?}", tipo),
        }
    }
}
