//! Análisis de sentencias, bloques y control de flujo

use super::*;

impl AnalizadorSemantico {
    /// Analiza un bloque de sentencias
    pub(crate) fn analizar_bloque(&mut self, bloque: &Bloque) {
        for sentencia in &bloque.sentencias {
            self.analizar_sentencia(sentencia);
        }
    }

    /// Analiza una sentencia individual
    pub(crate) fn analizar_sentencia(&mut self, sentencia: &Sentencia) {
        match sentencia {
            Sentencia::Expresion(expr) => {
                let _ = self.inferir_tipo(expr);
            }
            Sentencia::DeclaracionVariable(decl) => {
                // Manejo especial para ArrayRelleno: si hay tipo explícito Array, 
                // verificamos solo compatibilidad de tipo de elemento
                let tipo_valor = match (&decl.tipo, &decl.valor) {
                    (Some(Tipo::Array(tipo_esperado, n)), Expresion::ArrayRelleno(elem, _, _)) => {
                        let tipo_elem = self.inferir_tipo(elem);
                        if tipo_elem != **tipo_esperado {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                DISCONCORDANCIA_TIPO,
                                &decl.span,
                                format!("Disconcordancia de tipo en 'todos': elemento es '{:?}' pero arreglo espera '{:?}'",
                                    tipo_elem, tipo_esperado),
                                Some(format!("Cambia el tipo a '{:?}' o el valor de relleno", tipo_elem))
                            );
                        }
                        Tipo::Array(tipo_esperado.clone(), *n)
                    }
                    (Some(Tipo::ArrayGenerico(tipo_esperado, _)), Expresion::ArrayRelleno(elem, _, _)) => {
                        let tipo_elem = self.inferir_tipo(elem);
                        if tipo_elem != **tipo_esperado {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                DISCONCORDANCIA_TIPO,
                                &decl.span,
                                format!("Disconcordancia de tipo en 'todos': elemento es '{:?}' pero arreglo espera '{:?}'",
                                    tipo_elem, tipo_esperado),
                                Some(format!("Cambia el tipo a '{:?}' o el valor de relleno", tipo_elem))
                            );
                        }
                        Tipo::ArrayGenerico(tipo_esperado.clone(), String::new())
                    }
                    _ => self.inferir_tipo(&decl.valor)
                };
                
                // R7.7 F2 — artículo incierto: `un x = a + b` → Option<T> (checked con None)
                let mut tipo_valor = tipo_valor;
                if decl.articulo == Articulo::Un {
                    if let Expresion::Binaria(_, op, _, _) = &decl.valor {
                        let es_aritmetica = matches!(
                            op,
                            OperadorBinario::Suma
                                | OperadorBinario::Resta
                                | OperadorBinario::Multiplicacion
                        );
                        if es_aritmetica {
                            let es_entero_32 = matches!(
                                self.resolver_alias(&tipo_valor),
                                Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                                    | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32
                            );
                            if es_entero_32 {
                                tipo_valor = Tipo::Option(Box::new(tipo_valor));
                            } else {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    109,
                                    &decl.span,
                                    format!(
                                        "'un' (opcional checked) requiere enteros de 32 bits o menos, pero la operación es '{:?}'",
                                        tipo_valor
                                    ),
                                    Some("F2: soporta Entero8/16/32 y Natural8/16/32; flotantes no desbordan (inf)".to_string())
                                );
                            }
                        }
                    }
                }
                
                // Verificar concordancia de tipo explícito
                if let Some(ref tipo_declarado) = decl.tipo {
                    let tipo_declarado_resuelto = self.resolver_alias(tipo_declarado);
                    let es_literal_entero = |e: &Expresion| match e {
                        Expresion::Literal(Literal::Entero(_, _)) => true,
                        Expresion::Unaria(_, inner, _) => matches!(inner.as_ref(), Expresion::Literal(Literal::Entero(_, _))),
                        _ => false,
                    };
                    let es_literal_flotante = |e: &Expresion| matches!(e, Expresion::Literal(Literal::Flotante(_, _)));
                    // F-013 — literal polimórfico con check de rango (genérico siempre cabe)
                    if !matches!(tipo_declarado_resuelto, Tipo::Generico(_)) && (es_literal_entero(&decl.valor) || es_literal_flotante(&decl.valor)) && !self.literal_cabe_en_tipo(&decl.valor, &tipo_declarado_resuelto) {
                        let val_str = self.valor_literal_entero(&decl.valor).map(|v| v.to_string()).unwrap_or("?".to_string());
                        self.reportar_error(
                            CategoriaError::Tipo,
                            DISCONCORDANCIA_TIPO,
                            &decl.span,
                            format!("Literal '{}' no cabe en '{:?}' para '{}'", val_str, tipo_declarado_resuelto, decl.nombre),
                            Some(format!("Rango de {:?}: {:?} — usa `como_` o tipo más ancho", tipo_declarado_resuelto, self.rango_tipo_entero(&tipo_declarado_resuelto))),
                        );
                    }
                    let tipo_valor_adaptado = match &decl.valor {
                        e if es_literal_entero(e) && self.es_entero(&tipo_declarado_resuelto) => {
                            tipo_declarado_resuelto.clone()
                        }
                        e if es_literal_entero(e) && self.es_flotante(&tipo_declarado_resuelto) => {
                            tipo_declarado_resuelto.clone()
                        }
                        e if es_literal_flotante(e) && self.es_flotante(&tipo_declarado_resuelto) => {
                            tipo_declarado_resuelto.clone()
                        }
                        _ => tipo_valor.clone(),
                    };
                    let tipo_valor = &tipo_valor_adaptado;
                    if !self.tipos_compatibles(tipo_declarado, tipo_valor) {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            DISCONCORDANCIA_TIPO,
                            &decl.span,
                            format!("Disconcordancia de tipo: '{}' es '{:?}' pero se declaró como '{:?}'",
                                decl.nombre, tipo_valor, tipo_declarado),
                            Some(format!("Cambia el tipo a '{:?}' o el valor", tipo_valor))
                        );
                    }
                }

                // Resolver alias de tipo al declarar
                let tipo_final = match &decl.tipo {
                    Some(t) => self.resolver_alias(t),
                    None => tipo_valor.clone(),
                };

                self.entorno.declarar(InfoVariable {
                    nombre: decl.nombre.clone(),
                    tipo: tipo_final,
                    articulo: decl.articulo,
                    span: decl.span.clone(),
                });
            }
            Sentencia::Asignacion(asig) => {
                // Verificación de efecto 'puro': no muta nada fuera de su scope
                if self.efecto_actual == crate::ast::Efecto::Puro {
                    if let Lugar::Identificador(nombre) = &asig.lugar {
                        if let Some(func) = &self.funcion_actual {
                            if func.parametros.iter().any(|p| p.nombre == *nombre) {
                                self.reportar_error(
                                    CategoriaError::Ownership,
                                    50,
                                    &asig.span,
                                    format!("Función 'puro' no puede mutar parámetro '{}'", nombre),
                                    Some("Una función pura no muta estado externo. Usa una variable local.".to_string())
                                );
                            }
                        }
                    }
                }
                
                match &asig.lugar {
                    Lugar::Identificador(nombre) => {
                        let tipo_valor = self.inferir_tipo(&asig.valor);
                        let info_opt = self.entorno.buscar(nombre).cloned();
                        
                        match info_opt {
                            Some(info) => {
                                let compatible = if self.es_literal_entero(&asig.valor) || matches!(&asig.valor, Expresion::Literal(Literal::Flotante(_, _))) {
                                    if matches!(self.resolver_alias(&info.tipo), Tipo::Generico(_)) {
                                        true
                                    } else if self.literal_cabe_en_tipo(&asig.valor, &info.tipo) { true } else {
                                        let val_str = self.valor_literal_entero(&asig.valor).map(|v| v.to_string()).unwrap_or("?".to_string());
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            ASIGNACION_INCOMPATIBLE,
                                            &asig.span,
                                            format!("Literal '{}' no cabe en '{:?}' para '{}'", val_str, info.tipo, nombre),
                                            Some(format!("Valor fuera de rango {:?} — usa `como_` o tipo más ancho", self.rango_tipo_entero(&info.tipo))),
                                        );
                                        true
                                    }
                                } else {
                                    self.tipos_compatibles(&info.tipo, &tipo_valor)
                                };
                                if !compatible {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        ASIGNACION_INCOMPATIBLE,
                                        &asig.span,
                                        format!("Disconcordancia en asignación: '{}' es '{:?}' pero se asigna '{:?}'",
                                            nombre, info.tipo, tipo_valor),
                                        None
                                    );
                                }
                                if !self.es_mutable(info.articulo) {
                                    self.reportar_error(
                                        CategoriaError::Ownership,
                                        1,
                                        &asig.span,
                                        format!("Disconcordancia de estado: '{}' se declaró con '{}' (inmutable/prestada). \
No puedes modificar algo que no es 'tuyo'.", 
                                            nombre, self.articulo_a_str(info.articulo)),
                                        Some(format!("Usa 'el {}' para hacerlo mutable (owned)", nombre))
                                    );
                                }
                            }
                            None => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    VARIABLE_NO_DECLARADA,
                                    &asig.span,
                                    format!("'{}' no tiene concordancia en este contexto. ¿Olvidaste declararlo con artículo?",
                                        nombre),
                                    Some("Los identificadores deben declararse con artículo: el, la, un, los, las".to_string())
                                );
                            }
                        }
                    }
                    Lugar::Array(array_expr, indice_expr) => {
                        let tipo_array = self.inferir_tipo(array_expr);
                        let tipo_indice = self.inferir_tipo(indice_expr);
                        let tipo_valor = self.inferir_tipo(&asig.valor);
                        
                        if tipo_indice != Tipo::Entero32 && tipo_indice != Tipo::Entero64 {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                15,
                                &asig.span,
                                "Índice de arreglo debe ser Entero".to_string(),
                                None
                            );
                        }
                        
                        match &tipo_array {
                            Tipo::Array(tipo_elem, _) | Tipo::ArrayGenerico(tipo_elem, _) => {
                                if !self.tipos_compatibles(tipo_elem, &tipo_valor) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        ASIGNACION_INCOMPATIBLE,
                                        &asig.span,
                                        format!("Disconcordancia: arreglo almacena '{:?}' pero se asigna '{:?}'",
                                            tipo_elem, tipo_valor),
                                        None
                                    );
                                }
                            }
                            _ => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    16,
                                    &asig.span,
                                    format!("Asignación a arreglo en tipo '{:?}' que no es arreglo", tipo_array),
                                    None
                                );
                            }
                        }
                    }
                    Lugar::Campo(base_expr, _nombre_campo) => {
                        let _tipo_valor = self.inferir_tipo(&asig.valor);
                        let _tipo_base = self.inferir_tipo(base_expr);
                    }
                }
            }
            Sentencia::Retornar(expr, span) => {
                let func = self.funcion_actual.clone();
                if let Some(func) = func {
                    if let Some(ref tipo_retorno) = func.retorno {
                        if let Some(expr) = expr {
                            let tipo_expr = self.inferir_tipo(expr);
                            let compatible = if self.es_literal_entero(expr) || matches!(expr, Expresion::Literal(Literal::Flotante(_, _))) {
                                if matches!(self.resolver_alias(tipo_retorno), Tipo::Generico(_)) {
                                    true
                                } else if self.literal_cabe_en_tipo(expr, tipo_retorno) { true } else {
                                    let val_str = self.valor_literal_entero(expr).map(|v| v.to_string()).unwrap_or("?".to_string());
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        DISCONCORDANCIA_RETORNO,
                                        span,
                                        format!("Literal '{}' no cabe en retorno '{:?}' de '{}'", val_str, tipo_retorno, func.nombre),
                                        Some(format!("Valor fuera de rango {:?} — usa `como_` o tipo más ancho", self.rango_tipo_entero(tipo_retorno))),
                                    );
                                    true
                                }
                            } else {
                                self.tipos_compatibles(tipo_retorno, &tipo_expr)
                            };
                            if !compatible {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    DISCONCORDANCIA_RETORNO,
                                    span,
                                    format!("Disconcordancia en retorno: función '{}' devuelve '{:?}' pero se retorna '{:?}'",
                                        func.nombre, tipo_retorno, tipo_expr),
                                    None
                                );
                            }
                        } else {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                RETORNO_FALTANTE,
                                span,
                                format!("Función '{}' debe retornar '{:?}'", func.nombre, tipo_retorno),
                                None
                            );
                        }
                    }
                }
            }
            Sentencia::Condicional(cond) => {
                let tipo_cond = self.inferir_tipo(&cond.condicion);
                let es_estativo_bare = matches!(&cond.condicion, Expresion::Identificador(_, _))
                    && cond.modo == ModoVerbal::Estativo;
                
                if es_estativo_bare {
                    let es_valido_para_estado = matches!(&tipo_cond,
                        Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                        Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
                        Tipo::Booleano | Tipo::Caracter |
                        Tipo::Palabra | Tipo::Puntero(_) | Tipo::Generico(_)
                    );
                    if !es_valido_para_estado {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            24,
                            &cond.span,
                            format!("'está' (bare) requiere tipo entero, Booleano o puntero, encontrado '{:?}'", tipo_cond),
                            Some("Usa una comparación explícita (==, !=) o cambia el tipo de la variable".to_string())
                        );
                    }
                } else if tipo_cond != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        CONDICIONAL_NO_BOOLEANO,
                        &cond.span,
                        format!("La condición 'si' requiere un valor Booleano, encontrado '{:?}'", tipo_cond),
                        Some("Usa una comparación (==, !=, <, >) o una variable Booleano".to_string())
                    );
                }
                
                let borrows_antes = self.borrows.clone();
                
                self.analizar_bloque(&cond.bloque_entonces);
                
                self.borrows = borrows_antes.clone();
                
                if let Some(ref bloque_sino) = cond.bloque_sino {
                    self.analizar_bloque(bloque_sino);
                }
                
                self.borrows = borrows_antes;
            }
            Sentencia::BucleMientras(bucle) => {
                let tipo_cond = self.inferir_tipo(&bucle.condicion);
                if tipo_cond != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        BUCLE_NO_BOOLEANO,
                        &bucle.span,
                        format!("La condición 'mientras' requiere un valor Booleano, encontrado '{:?}'", tipo_cond),
                        Some("Usa una comparación (==, !=, <, >) o una variable Booleano".to_string())
                    );
                }
                
                let borrows_antes = self.borrows.clone();
                self.profundidad_bucle += 1;
                self.analizar_bloque(&bucle.bloque);
                self.profundidad_bucle -= 1;
                self.borrows = borrows_antes;
            }
            Sentencia::BuclePara(bucle) => {
                let tipo_elem = match &bucle.iterable {
                    Expresion::Rango(inicio, fin, _, _) => {
                        let tipo_inicio = self.inferir_tipo(inicio);
                        let tipo_fin = self.inferir_tipo(fin);
                        if !self.es_entero(&tipo_inicio) || !self.es_entero(&tipo_fin) {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                23,
                                &bucle.span,
                                format!("Rango requiere extremos enteros, encontrado '{:?}' y '{:?}'", tipo_inicio, tipo_fin),
                                Some("Usa enteros: para i en 0..10 {{ ... }}".to_string())
                            );
                        }
                        tipo_inicio
                    }
                    _ => {
                        let tipo_iterable = self.inferir_tipo(&bucle.iterable);
                        match &tipo_iterable {
                            Tipo::Array(t, _) | Tipo::ArrayGenerico(t, _) => *t.clone(),
                            _ => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    23,
                                    &bucle.span,
                                    format!("'para' requiere un arreglo o rango, encontrado '{:?}'", tipo_iterable),
                                    Some("Usa un arreglo [T; N] o un rango: 0..10".to_string())
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                };
                
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                
                self.entorno.declarar(InfoVariable {
                    nombre: bucle.variable.clone(),
                    tipo: tipo_elem,
                    articulo: Articulo::El,
                    span: bucle.span.clone(),
                });
                
                let borrows_antes = self.borrows.clone();
                self.profundidad_bucle += 1;
                self.analizar_bloque(&bucle.bloque);
                self.profundidad_bucle -= 1;
                self.borrows = borrows_antes;
                
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
            Sentencia::Romper(span) => {
                if self.profundidad_bucle == 0 {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        107,
                        span,
                        "'romper' solo puede usarse dentro de un bucle (para/mientras)".to_string(),
                        Some("Coloca 'romper' dentro del cuerpo de un bucle para salir temprano".to_string())
                    );
                }
            }
            Sentencia::Continuar(span) => {
                if self.profundidad_bucle == 0 {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        108,
                        span,
                        "'continuar' solo puede usarse dentro de un bucle (para/mientras)".to_string(),
                        Some("Coloca 'continuar' dentro del cuerpo de un bucle para saltar a la siguiente iteración".to_string())
                    );
                }
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                
                for sentencia in cuerpo {
                    self.analizar_sentencia(sentencia);
                }
                
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
            Sentencia::Seleccionar(seleccionar) => {
                for rama in &seleccionar.ramas {
                    if rama.variable.is_some() {
                        let tipo_canal = self.inferir_tipo(&rama.canal);
                        if tipo_canal != Tipo::Entero64 && tipo_canal != Tipo::Vacio {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                90,
                                &rama.span,
                                format!(
                                    "El canal en 'seleccionar' debe ser Entero64, pero se encontró '{:?}'",
                                    tipo_canal
                                ),
                                Some("Usa canal_nuevo() que retorna Entero64".to_string()),
                            );
                        }
                    }
                    let entorno_anterior = std::mem::take(&mut self.entorno);
                    self.entorno = Entorno::con_padre(entorno_anterior);
                    
                    if let Some(ref var) = rama.variable {
                        self.entorno.declarar(InfoVariable {
                            nombre: var.clone(),
                            tipo: Tipo::Entero32,
                            articulo: crate::ast::Articulo::La,
                            span: rama.span.clone(),
                        });
                    }
                    
                    for sentencia in &rama.cuerpo.sentencias {
                        self.analizar_sentencia(sentencia);
                    }
                    
                    self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
                }
            }
            Sentencia::ConExecutor { hilos, cuerpo, span } => {
                let tipo_hilos = self.inferir_tipo(hilos);
                match tipo_hilos {
                    Tipo::Entero32 | Tipo::Entero64 | Tipo::Natural32 | Tipo::Natural64 => {}
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            91,
                            span,
                            format!(
                                "con_executor requiere un número entero de hilos, pero se encontró '{:?}'",
                                tipo_hilos
                            ),
                            Some("Usa un literal entero: con_executor(4) { ... }".to_string()),
                        );
                    }
                }
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                for sentencia in cuerpo {
                    self.analizar_sentencia(sentencia);
                }
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
        }
    }

    /// Extrae el path de acceso de una expresión (ej: "punto.x" desde AccesoCampo)
    pub(crate) fn extraer_path(&self, expr: &Expresion) -> Option<String> {
        match expr {
            Expresion::Identificador(nombre, _) => Some(nombre.clone()),
            Expresion::AccesoCampo(base, campo, _) => {
                if let Some(base_path) = self.extraer_path(base) {
                    Some(format!("{}.{}", base_path, campo))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Helper para reportar errores de forma consistente
    pub(crate) fn reportar_error(
        &mut self,
        categoria: CategoriaError,
        codigo: u32,
        span: &Span,
        mensaje: String,
        sugerencia: Option<String>,
    ) {
        let mut error = ErrorCompilador::nuevo(categoria, codigo, span.clone(), mensaje);
        if let Some(sug) = sugerencia {
            error = error.con_sugerencia(sug);
        }
        self.errores.agregar(error);
    }

    /// Helper para reportar warnings (no bloquean la compilación).
    /// Usado por F-006 para detectar shadowing (declaraciones duplicadas)
    /// sin romper archivos que ya compilan con duplicados (ej. `json_reparador.fc`).
    pub(crate) fn reportar_warning(
        &mut self,
        codigo: u32,
        span: &Span,
        mensaje: String,
        sugerencia: Option<String>,
    ) {
        let mut warning = ErrorCompilador::nuevo(CategoriaError::Tipo, codigo, span.clone(), mensaje);
        if let Some(sug) = sugerencia {
            warning = warning.con_sugerencia(sug);
        }
        self.errores.agregar_warning(warning);
    }
}
