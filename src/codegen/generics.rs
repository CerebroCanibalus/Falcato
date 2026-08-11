//! Genéricos — monomorfización, especialización, sustitución de tipos

use super::*;

impl Codegen {
    pub(crate) fn compilar_llamada_generica(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let func_generica = match self.funciones_genericas.get(&llamada.funcion) {
            Some(f) => f.clone(),
            None => return Err(()),
        };

        // Inferir valores concretos de const generics y tipos de type generics
        let mut sust_consts: HashMap<String, String> = HashMap::new();
        let mut sust_tipos: HashMap<String, Tipo> = HashMap::new();
        let mut valores_clave: Vec<String> = Vec::new();
        for gen in &func_generica.parametros_genericos {
            if let Some(ref _tipo_const) = gen.tipo {
                // Const generic: buscar en parÃƒÆ’Ã‚Â¡metros de la funciÃƒÆ’Ã‚Â³n
                let valor = self.inferir_const_generico(
                    &func_generica.parametros,
                    &llamada.argumentos,
                    variables,
                    &gen.nombre,
                );
                match valor {
                    Some(v) => {
                        sust_consts.insert(gen.nombre.clone(), v.to_string());
                        valores_clave.push(v.to_string());
                    }
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Tipo,
                            60,
                            llamada.span.clone(),
                            format!("No se pudo inferir el valor del parÃƒÆ’Ã‚Â¡metro const '{}' en llamada a '{}'",
                                gen.nombre, llamada.funcion),
                        ));
                        return Err(());
                    }
                }
            } else {
                // Type generic: inferir del tipo de los argumentos
                let tipo_concreto = self.inferir_tipo_generico(
                    &func_generica.parametros,
                    &llamada.argumentos,
                    variables,
                    &gen.nombre,
                );
                match tipo_concreto {
                    Some(t) => {
                        sust_tipos.insert(gen.nombre.clone(), t.clone());
                        valores_clave.push(self.nombre_tipo_instancia(&t));
                    }
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Tipo,
                            62,
                            llamada.span.clone(),
                            format!("No se pudo inferir el tipo del parÃƒÆ’Ã‚Â¡metro genÃƒÆ’Ã‚Â©rico '{}' en llamada a '{}'",
                                gen.nombre, llamada.funcion),
                        ));
                        return Err(());
                    }
                }
            }
        }

        // Verificar si ya existe una instanciaciÃƒÆ’Ã‚Â³n
        let clave = (llamada.funcion.clone(), valores_clave.clone());
        let func_id = match self.instanciaciones.get(&clave).copied() {
            Some(id) => id,
            None => {
                // Crear funciÃƒÆ’Ã‚Â³n especializada
                let func_especializada = self.especializar_funcion(
                    &func_generica,
                    &sust_consts,
                    &sust_tipos,
                );
                
                // Declarar y compilar la funciÃƒÆ’Ã‚Â³n especializada
                self.declarar_funcion(&func_especializada);
                let id = match self.funciones.get(&func_especializada.nombre) {
                    Some(&id) => id,
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            61,
                            llamada.span.clone(),
                            format!("Error interno al declarar '{}'", func_especializada.nombre),
                        ));
                        return Err(());
                    }
                };
                self.instanciaciones.insert(clave.clone(), id);
                
                if let Err(_) = self.compilar_funcion(&func_especializada) {
                    // Error ya agregado
                }
                
                id
            }
        };

        let func_ref = self.module.declare_func_in_func(func_id, builder.func);

        let mut args = Vec::new();
        for arg in &llamada.argumentos {
            let val = self.compilar_expresion(arg, builder, variables)?;
            args.push(val);
        }

        let call = builder.ins().call(func_ref, &args);
        let result = builder.inst_results(call);
        
        if result.is_empty() {
            Ok(builder.ins().iconst(types::I32, 0))
        } else {
            Ok(result[0])
        }
    }

    /// Infiere el valor de un const genÃƒÆ’Ã‚Â©rico a partir de los tipos de los argumentos
    pub(crate) fn inferir_const_generico(
        &self,
        parametros: &Vec<Parametro>,
        argumentos: &Vec<Expresion>,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        nombre_generico: &str,
    ) -> Option<usize> {
        // Buscar en quÃƒÆ’Ã‚Â© parÃƒÆ’Ã‚Â¡metro se usa el genÃƒÆ’Ã‚Â©rico
        for (param, arg) in parametros.iter().zip(argumentos.iter()) {
            if self.tipo_contiene_generico(&param.tipo, nombre_generico) {
                // Inferir del tipo del argumento
                let tipo_arg = self.inferir_tipo(arg, variables);
                
                if let Some(valor) = self.extraer_valor_generico(&tipo_arg, nombre_generico) {
                    return Some(valor);
                }
            }
        }
        None
    }

    /// Verifica si un tipo contiene una referencia a un genÃƒÆ’Ã‚Â©rico (type o const)
    pub(crate) fn tipo_contiene_generico(&self, tipo: &Tipo, nombre_generico: &str) -> bool {
        match tipo {
            Tipo::Generico(n) if n == nombre_generico => true,
            Tipo::ArrayGenerico(_, n) if n == nombre_generico => true,
            Tipo::ArrayGenerico(t, _) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Array(t, _) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Vector(t) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Puntero(t) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::Referencia(t) => self.tipo_contiene_generico(t, nombre_generico),
            Tipo::NombreGenerico(_, args) => args.iter().any(|a| self.tipo_contiene_generico(a, nombre_generico)),
            _ => false,
        }
    }

    /// Extrae el valor concreto de un genÃƒÆ’Ã‚Â©rico de un tipo
    pub(crate) fn extraer_valor_generico(&self, tipo: &Tipo, _nombre_generico: &str) -> Option<usize> {
        match tipo {
            Tipo::Array(_, n) => Some(*n),
            _ => None,
        }
    }

    /// Crea una funciÃƒÆ’Ã‚Â³n especializada reemplazando genÃƒÆ’Ã‚Â©ricos por valores concretos
    pub(crate) fn especializar_funcion(
        &mut self,
        func: &FuncionDecl,
        sust_consts: &HashMap<String, String>,
        sust_tipos: &HashMap<String, Tipo>,
    ) -> FuncionDecl {
        let mut func_clon = func.clone();
        
        // Generar nombre especializado: longitud_5 o es_igual_Entero32
        let partes_nombre: Vec<String> = func.parametros_genericos.iter().map(|gen| {
            if gen.tipo.is_some() {
                sust_consts.get(&gen.nombre).cloned().unwrap_or_else(|| gen.nombre.clone())
            } else {
                sust_tipos.get(&gen.nombre).map(|t| self.nombre_tipo_instancia(t)).unwrap_or_else(|| gen.nombre.clone())
            }
        }).collect();
        let nombre_especializado = format!("{}_{}", func.nombre, partes_nombre.join("_"));
        func_clon.nombre = nombre_especializado;
        func_clon.parametros_genericos.clear();
        
        // Aplicar sustituciones a parÃƒÆ’Ã‚Â¡metros
        for param in &mut func_clon.parametros {
            self.sustituir_tipo(&mut param.tipo, sust_consts, sust_tipos);
        }
        
        // Aplicar sustituciones a retorno
        if let Some(ref mut ret) = func_clon.retorno {
            self.sustituir_tipo(ret, sust_consts, sust_tipos);
        }
        
        // Aplicar sustituciones al cuerpo (sentencias y expresiones)
        for sentencia in &mut func_clon.cuerpo.sentencias {
            self.sustituir_en_sentencia(sentencia, sust_consts, sust_tipos);
        }
        
        func_clon
    }

    /// Sustituye const genÃƒÆ’Ã‚Â©ricos por literales en una sentencia
    pub(crate) fn sustituir_en_sentencia(
        &self,
        sentencia: &mut Sentencia,
        sust_consts: &HashMap<String, String>,
        sust_tipos: &HashMap<String, Tipo>,
    ) {
        match sentencia {
            Sentencia::Expresion(expr) => self.sustituir_en_expresion(expr, sust_consts),
            Sentencia::DeclaracionVariable(decl) => {
                if let Some(ref mut tipo) = decl.tipo {
                    self.sustituir_tipo(tipo, sust_consts, sust_tipos);
                }
                self.sustituir_en_expresion(&mut decl.valor, sust_consts);
            }
            Sentencia::Asignacion(asig) => {
                self.sustituir_en_expresion(&mut asig.valor, sust_consts);
            }
            Sentencia::Retornar(expr, _) => {
                if let Some(expr) = expr {
                    self.sustituir_en_expresion(expr, sust_consts);
                }
            }
            // R7.7 — romper/continuar no contienen expresiones que sustituir
            Sentencia::Romper(_) => {}
            Sentencia::Continuar(_) => {}
            Sentencia::Condicional(cond) => {
                self.sustituir_en_expresion(&mut cond.condicion, sust_consts);
                for sent in &mut cond.bloque_entonces.sentencias {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
                if let Some(bloque_sino) = &mut cond.bloque_sino {
                    for sent in &mut bloque_sino.sentencias {
                        self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                    }
                }
            }
            Sentencia::BucleMientras(bucle) => {
                self.sustituir_en_expresion(&mut bucle.condicion, sust_consts);
                for sent in &mut bucle.bloque.sentencias {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
            Sentencia::BuclePara(bucle) => {
                self.sustituir_en_expresion(&mut bucle.iterable, sust_consts);
                for sent in &mut bucle.bloque.sentencias {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                for sent in cuerpo {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
            Sentencia::Seleccionar(seleccionar) => {
                for rama in &mut seleccionar.ramas {
                    for sent in &mut rama.cuerpo.sentencias {
                        self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                    }
                }
            }
            Sentencia::ConExecutor { hilos: _, cuerpo, span: _ } => {
                for sent in cuerpo {
                    self.sustituir_en_sentencia(sent, sust_consts, sust_tipos);
                }
            }
        }
    }

    /// Sustituye const genÃƒÆ’Ã‚Â©ricos por literales en una expresiÃƒÆ’Ã‚Â³n
    pub(crate) fn sustituir_en_expresion(
        &self,
        expr: &mut Expresion,
        sustituciones: &HashMap<String, String>,
    ) {
        match expr {
            Expresion::Identificador(nombre, span) => {
                if let Some(valor) = sustituciones.get(nombre) {
                    if let Ok(n) = valor.parse::<i64>() {
                        *expr = Expresion::Literal(Literal::Entero(n, span.clone()));
                    }
                }
            }
            Expresion::Binaria(izq, _, der, _) => {
                self.sustituir_en_expresion(izq, sustituciones);
                self.sustituir_en_expresion(der, sustituciones);
            }
            Expresion::Unaria(_, expr, _) => {
                self.sustituir_en_expresion(expr, sustituciones);
            }
            Expresion::Llamada(llamada) => {
                for arg in &mut llamada.argumentos {
                    self.sustituir_en_expresion(arg, sustituciones);
                }
            }
            Expresion::AccesoArray(array, indice, _) => {
                self.sustituir_en_expresion(array, sustituciones);
                self.sustituir_en_expresion(indice, sustituciones);
            }
            Expresion::LiteralArray(elementos, _) => {
                for elem in elementos {
                    self.sustituir_en_expresion(elem, sustituciones);
                }
            }
            Expresion::ArrayRelleno(elem, _, _) => {
                self.sustituir_en_expresion(elem, sustituciones);
            }
            Expresion::InicializacionStruct(_, campos, _) => {
                for (_, val) in campos {
                    self.sustituir_en_expresion(val, sustituciones);
                }
            }
            Expresion::AccesoCampo(base, _, _) => {
                self.sustituir_en_expresion(base, sustituciones);
            }
            Expresion::ConstructorEnum(_, _, args, _) => {
                for arg in args {
                    self.sustituir_en_expresion(arg, sustituciones);
                }
            }
            Expresion::EsVariante(base, _, _, _, _) => {
                self.sustituir_en_expresion(base, sustituciones);
            }
            _ => {}
        }
    }

    /// Sustituye genÃƒÆ’Ã‚Â©ricos por valores concretos en un tipo
    pub(crate) fn sustituir_tipo(
        &self,
        tipo: &mut Tipo,
        sust_consts: &HashMap<String, String>,
        sust_tipos: &HashMap<String, Tipo>,
    ) {
        match tipo {
            Tipo::Generico(nombre) => {
                if let Some(concreto) = sust_tipos.get(nombre) {
                    *tipo = concreto.clone();
                }
            }
            Tipo::ArrayGenerico(t, nombre) => {
                if let Some(valor) = sust_consts.get(nombre) {
                    if let Ok(n) = valor.parse::<usize>() {
                        *tipo = Tipo::Array(Box::new((**t).clone()), n);
                        return;
                    }
                }
                self.sustituir_tipo(t, sust_consts, sust_tipos);
            }
            Tipo::Array(t, _) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::Vector(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::Puntero(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::Referencia(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaMut(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaConLifetime(_, t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaMutConLifetime(_, t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaSelf(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::ReferenciaMutSelf(t) => self.sustituir_tipo(t, sust_consts, sust_tipos),
            Tipo::NombreGenerico(_, args) => {
                for arg in args {
                    self.sustituir_tipo(arg, sust_consts, sust_tipos);
                }
            }
            _ => {}
        }
    }

        /// Infiere el tipo concreto de un parÃƒÆ’Ã‚Â¡metro type generic a partir de los argumentos
        fn inferir_tipo_generico(
            &self,
            parametros: &Vec<Parametro>,
            argumentos: &Vec<Expresion>,
            variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
            nombre_generico: &str,
        ) -> Option<Tipo> {
            let mut resultado: Option<Tipo> = None;
            for (param, arg) in parametros.iter().zip(argumentos.iter()) {
                if self.tipo_contiene_generico(&param.tipo, nombre_generico) {
                    let tipo_arg = self.inferir_tipo(arg, variables);
                    if let Some(tipo_inferido) = self.extraer_tipo_generico(&tipo_arg, nombre_generico, &param.tipo) {
                        if let Some(prev) = &resultado {
                            if prev != &tipo_inferido {
                                return None; // inconsistencia
                            }
                        } else {
                            resultado = Some(tipo_inferido);
                        }
                    }
                }
            }
            resultado
        }

        /// Extrae el tipo concreto correspondiente a un genÃƒÆ’Ã‚Â©rico dentro de un tipo argumento
        fn extraer_tipo_generico(&self, tipo_arg: &Tipo, nombre_generico: &str, param_tipo: &Tipo) -> Option<Tipo> {
            match param_tipo {
                Tipo::Generico(n) if n == nombre_generico => Some(tipo_arg.clone()),
                Tipo::ArrayGenerico(elem_param, _) | Tipo::Array(elem_param, _) => {
                    if let Tipo::Array(elem_arg, n) = tipo_arg {
                        self.extraer_tipo_generico(elem_arg, nombre_generico, elem_param)
                            .map(|t| Tipo::Array(Box::new(t), *n))
                    } else {
                        None
                    }
                }
                Tipo::Vector(elem_param) => {
                    if let Tipo::Vector(elem_arg) = tipo_arg {
                        self.extraer_tipo_generico(elem_arg, nombre_generico, elem_param)
                            .map(|t| Tipo::Vector(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::Puntero(p) => {
                    if let Tipo::Puntero(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::Puntero(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::Referencia(p) => {
                    if let Tipo::Referencia(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::Referencia(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::ReferenciaSelf(p) => {
                    if let Tipo::ReferenciaSelf(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::ReferenciaSelf(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::ReferenciaMutSelf(p) => {
                    if let Tipo::ReferenciaMutSelf(a) = tipo_arg {
                        self.extraer_tipo_generico(a, nombre_generico, p)
                            .map(|t| Tipo::ReferenciaMutSelf(Box::new(t)))
                    } else {
                        None
                    }
                }
                Tipo::NombreGenerico(_, args_param) => {
                    if let Tipo::NombreGenerico(_, args_arg) = tipo_arg {
                        for (ap, aa) in args_param.iter().zip(args_arg.iter()) {
                            if let Some(t) = self.extraer_tipo_generico(aa, nombre_generico, ap) {
                                return Some(t);
                            }
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        /// Genera un nombre vÃƒÆ’Ã‚Â¡lido para una instancia de tipo concreto
        fn nombre_tipo_instancia(&self, tipo: &Tipo) -> String {
            match tipo {
                Tipo::Entero8 => "Entero8".to_string(),
                Tipo::Entero16 => "Entero16".to_string(),
                Tipo::Entero32 => "Entero32".to_string(),
                Tipo::Entero64 => "Entero64".to_string(),
                Tipo::Natural8 => "Natural8".to_string(),
                Tipo::Natural16 => "Natural16".to_string(),
                Tipo::Natural32 => "Natural32".to_string(),
                Tipo::Natural64 => "Natural64".to_string(),
                Tipo::Flotante32 => "Flotante32".to_string(),
                Tipo::Flotante64 => "Flotante64".to_string(),
                Tipo::Booleano => "Booleano".to_string(),
                Tipo::Caracter => "Caracter".to_string(),
                Tipo::Palabra => "Palabra".to_string(),
                Tipo::Texto => "Texto".to_string(),
                Tipo::Vacio => "Vacio".to_string(),
                Tipo::Nombre(n) => n.clone(),
                Tipo::Generico(n) => n.clone(),
                Tipo::Array(t, n) => format!("Array_{}_{}", self.nombre_tipo_instancia(t), n),
                Tipo::ArrayGenerico(t, n) => format!("Array_{}_{}", self.nombre_tipo_instancia(t), n),
                Tipo::Vector(t) => format!("Vector_{}", self.nombre_tipo_instancia(t)),
                Tipo::Diccionario(k, v) => format!("Diccionario_{}_{}", self.nombre_tipo_instancia(k), self.nombre_tipo_instancia(v)),
                Tipo::Conjunto(t) => format!("Conjunto_{}", self.nombre_tipo_instancia(t)),
                Tipo::Resultado(t, e) => format!("Resultado_{}_{}", self.nombre_tipo_instancia(t), self.nombre_tipo_instancia(e)),
                Tipo::Puntero(t) => format!("Ptr_{}", self.nombre_tipo_instancia(t)),
                Tipo::Referencia(t) => format!("Ref_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaMut(t) => format!("RefMut_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaConLifetime(_, t) => format!("Ref_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaMutConLifetime(_, t) => format!("RefMut_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaSelf(t) => format!("RefSelf_{}", self.nombre_tipo_instancia(t)),
                Tipo::ReferenciaMutSelf(t) => format!("RefMutSelf_{}", self.nombre_tipo_instancia(t)),
                Tipo::NombreGenerico(n, args) => {
                    let args_str = args.iter().map(|a| self.nombre_tipo_instancia(a)).collect::<Vec<_>>().join("_");
                    format!("{}_{}", n, args_str)
                }
            }
        }

}
