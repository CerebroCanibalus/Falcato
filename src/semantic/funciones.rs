//! Análisis de funciones, llamadas y genéricos

use super::*;

impl AnalizadorSemantico {
    /// Analiza una función: registra parámetros, analiza cuerpo, restaura entorno
    pub(crate) fn analizar_funcion(&mut self, func: &FuncionDecl) {
        self.funcion_actual = Some(func.clone());
        
        // Establecer nivel de verificación de ownership y limpiar estado anterior
        self.nivel_verificacion_actual = func.nivel_verificacion;
        self.variables_movidas.clear();
        self.borrows.clear();
        self.efecto_actual = func.efecto.clone();
        
        // Nuevo entorno para la función
        let entorno_anterior = std::mem::take(&mut self.entorno);
        self.entorno = Entorno::con_padre(entorno_anterior);

        // Registrar parámetros genéricos
        for gen in &func.parametros_genericos {
            if let Some(ref tipo) = gen.tipo {
                // Const param: N: Entero32
                self.entorno.declarar_const(gen.nombre.clone(), tipo.clone());
            } else {
                // Type param: T
                self.entorno.declarar_tipo(gen.nombre.clone(), Tipo::Generico(gen.nombre.clone()));
            }
        }

        // Registrar parámetros
        for param in &func.parametros {
            self.entorno.declarar(InfoVariable {
                nombre: param.nombre.clone(),
                tipo: param.tipo.clone(),
                articulo: param.articulo,
                span: param.span.clone(),
            });
        }

        // Analizar cuerpo
        self.analizar_bloque(&func.cuerpo);

        // Restaurar entorno
        self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
        self.funcion_actual = None;
    }

    /// Sustituye parámetros genéricos de tipo por tipos concretos en una firma.
    pub(crate) fn aplicar_tipo_args_a_firma(
        &mut self,
        firma: &FirmaFuncion,
        tipo_args: &Vec<Tipo>,
        span: &Span,
    ) -> Option<FirmaFuncion> {
        if firma.parametros_genericos.is_empty() {
            return Some(firma.clone());
        }

        if tipo_args.len() != firma.parametros_genericos.len() {
            self.reportar_error(
                CategoriaError::Tipo,
                70,
                span,
                format!("Función '{}' espera {} argumentos de tipo, pero se pasaron {}",
                    firma.nombre, firma.parametros_genericos.len(), tipo_args.len()),
                Some("Proporciona los tipos genéricos requeridos, e.g., vector_nuevo<Entero32>()".to_string()),
            );
            return None;
        }

        let mut sustituciones: HashMap<String, Tipo> = HashMap::new();
        for (gen, tipo) in firma.parametros_genericos.iter().zip(tipo_args.iter()) {
            sustituciones.insert(gen.nombre.clone(), tipo.clone());
        }

        let parametros = firma.parametros.iter()
            .map(|(n, t)| (n.clone(), self.sustituir_genericos(t, &sustituciones)))
            .collect();
        let retorno = firma.retorno.as_ref()
            .map(|t| self.sustituir_genericos(t, &sustituciones));

        Some(FirmaFuncion {
            nombre: firma.nombre.clone(),
            parametros_genericos: vec![], // ya instanciados
            parametros,
            retorno,
            span: firma.span.clone(),
            es_publica: firma.es_publica,
        })
    }

    /// Reemplaza Tipo::Generico(n) por el tipo concreto asociado.
    pub(crate) fn sustituir_genericos(
        &self,
        tipo: &Tipo,
        sustituciones: &HashMap<String, Tipo>,
    ) -> Tipo {
        match tipo {
            Tipo::Generico(nombre) => {
                sustituciones.get(nombre).cloned().unwrap_or(tipo.clone())
            }
            Tipo::Vector(t) => Tipo::Vector(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Diccionario(k, v) => Tipo::Diccionario(
                Box::new(self.sustituir_genericos(k, sustituciones)),
                Box::new(self.sustituir_genericos(v, sustituciones)),
            ),
            Tipo::Conjunto(t) => Tipo::Conjunto(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Resultado(t, e) => Tipo::Resultado(
                Box::new(self.sustituir_genericos(t, sustituciones)),
                Box::new(self.sustituir_genericos(e, sustituciones)),
            ),
            Tipo::Puntero(t) => Tipo::Puntero(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Referencia(t) => Tipo::Referencia(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaMut(t) => Tipo::ReferenciaMut(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaConLifetime(n, t) => Tipo::ReferenciaConLifetime(n.clone(), Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaMutConLifetime(n, t) => Tipo::ReferenciaMutConLifetime(n.clone(), Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaSelf(t) => Tipo::ReferenciaSelf(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaMutSelf(t) => Tipo::ReferenciaMutSelf(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Array(t, n) => Tipo::Array(Box::new(self.sustituir_genericos(t, sustituciones)), *n),
            Tipo::ArrayGenerico(t, n) => Tipo::ArrayGenerico(Box::new(self.sustituir_genericos(t, sustituciones)), n.clone()),
            Tipo::NombreGenerico(n, args) => {
                let nuevos = args.iter()
                    .map(|a| self.sustituir_genericos(a, sustituciones))
                    .collect();
                Tipo::NombreGenerico(n.clone(), nuevos)
            }
            _ => tipo.clone(),
        }
    }

    /// Verifica si un tipo genérico tiene un bound específico
    pub(crate) fn tiene_bound(&self, nombre: &str, bound: &str) -> bool {
        if let Some(func) = &self.funcion_actual {
            func.parametros_genericos.iter().any(|pg| {
                pg.nombre == nombre && pg.bounds.iter().any(|b| b == bound)
            })
        } else {
            false
        }
    }
}
