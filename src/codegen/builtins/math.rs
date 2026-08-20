use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_abs(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let cero = builder.ins().iconst(types::I32, 0);
        let es_neg = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedLessThan, x, cero);
        let neg = builder.ins().ineg(x);
        let resultado = builder.ins().select(es_neg, neg, x);
        Ok(resultado)
    }

    /// max(a: Entero32, b: Entero32) -> Entero32
    pub(crate) fn builtin_max(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let b = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let a_mayor = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan, a, b);
        let resultado = builder.ins().select(a_mayor, a, b);
        Ok(resultado)
    }

    /// min(a: Entero32, b: Entero32) -> Entero32
    pub(crate) fn builtin_min(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let b = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let a_menor = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedLessThan, a, b);
        let resultado = builder.ins().select(a_menor, a, b);
        Ok(resultado)
    }

    /// raiz(x: Flotante64) -> Flotante64 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â C sqrt()
    pub(crate) fn builtin_raiz(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let sqrt_id = self.asegurar_funcion_c("sqrt", &[types::F64], Some(types::F64));
        let sqrt_ref = self.module.declare_func_in_func(sqrt_id, builder.func);
        let call = builder.ins().call(sqrt_ref, &[x]);
        let resultado = builder.inst_results(call)[0];
        Ok(resultado)
    }

    /// potencia(base: Flotante64, exp: Flotante64) -> Flotante64 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â C pow()
    pub(crate) fn builtin_potencia(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let base = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let exp = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let pow_id = self.asegurar_funcion_c("pow", &[types::F64, types::F64], Some(types::F64));
        let pow_ref = self.module.declare_func_in_func(pow_id, builder.func);
        let call = builder.ins().call(pow_ref, &[base, exp]);
        let resultado = builder.inst_results(call)[0];
        Ok(resultado)
    }

    // ============================================================
    // TRIGONOMETRÍA — F1: libm (preciso)
    // ============================================================

    /// seno(x: Real) -> Real — C sin()
    pub(crate) fn builtin_seno(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let sin_id = self.asegurar_funcion_c("sin", &[types::F64], Some(types::F64));
        let sin_ref = self.module.declare_func_in_func(sin_id, builder.func);
        let call = builder.ins().call(sin_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// coseno(x: Real) -> Real — C cos()
    pub(crate) fn builtin_coseno(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let cos_id = self.asegurar_funcion_c("cos", &[types::F64], Some(types::F64));
        let cos_ref = self.module.declare_func_in_func(cos_id, builder.func);
        let call = builder.ins().call(cos_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// tangente(x: Real) -> Real — C tan()
    pub(crate) fn builtin_tangente(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let tan_id = self.asegurar_funcion_c("tan", &[types::F64], Some(types::F64));
        let tan_ref = self.module.declare_func_in_func(tan_id, builder.func);
        let call = builder.ins().call(tan_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// arcseno(x: Real) -> Real — C asin()
    pub(crate) fn builtin_arcseno(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let asin_id = self.asegurar_funcion_c("asin", &[types::F64], Some(types::F64));
        let asin_ref = self.module.declare_func_in_func(asin_id, builder.func);
        let call = builder.ins().call(asin_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// arccoseno(x: Real) -> Real — C acos()
    pub(crate) fn builtin_arccoseno(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let acos_id = self.asegurar_funcion_c("acos", &[types::F64], Some(types::F64));
        let acos_ref = self.module.declare_func_in_func(acos_id, builder.func);
        let call = builder.ins().call(acos_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// arctangente(x: Real) -> Real — C atan()
    pub(crate) fn builtin_arctangente(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let atan_id = self.asegurar_funcion_c("atan", &[types::F64], Some(types::F64));
        let atan_ref = self.module.declare_func_in_func(atan_id, builder.func);
        let call = builder.ins().call(atan_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// arctangente2(y: Real, x: Real) -> Real — C atan2()
    pub(crate) fn builtin_arctangente2(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let y = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let x = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let atan2_id = self.asegurar_funcion_c("atan2", &[types::F64, types::F64], Some(types::F64));
        let atan2_ref = self.module.declare_func_in_func(atan2_id, builder.func);
        let call = builder.ins().call(atan2_ref, &[y, x]);
        Ok(builder.inst_results(call)[0])
    }

    // Hiperbólicas
    /// senoh(x: Real) -> Real — C sinh()
    pub(crate) fn builtin_seno_hiperbolico(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let sinh_id = self.asegurar_funcion_c("sinh", &[types::F64], Some(types::F64));
        let sinh_ref = self.module.declare_func_in_func(sinh_id, builder.func);
        let call = builder.ins().call(sinh_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// cosenoh(x: Real) -> Real — C cosh()
    pub(crate) fn builtin_coseno_hiperbolico(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let cosh_id = self.asegurar_funcion_c("cosh", &[types::F64], Some(types::F64));
        let cosh_ref = self.module.declare_func_in_func(cosh_id, builder.func);
        let call = builder.ins().call(cosh_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// tangenteh(x: Real) -> Real — C tanh()
    pub(crate) fn builtin_tangente_hiperbolica(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let tanh_id = self.asegurar_funcion_c("tanh", &[types::F64], Some(types::F64));
        let tanh_ref = self.module.declare_func_in_func(tanh_id, builder.func);
        let call = builder.ins().call(tanh_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    // Exponencial y logaritmo
    /// exp(x: Real) -> Real — C exp()
    pub(crate) fn builtin_exponencial(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let exp_id = self.asegurar_funcion_c("exp", &[types::F64], Some(types::F64));
        let exp_ref = self.module.declare_func_in_func(exp_id, builder.func);
        let call = builder.ins().call(exp_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// log(x: Real) -> Real — C log() (logaritmo natural)
    pub(crate) fn builtin_logaritmo(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let log_id = self.asegurar_funcion_c("log", &[types::F64], Some(types::F64));
        let log_ref = self.module.declare_func_in_func(log_id, builder.func);
        let call = builder.ins().call(log_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// log10(x: Real) -> Real — C log10()
    pub(crate) fn builtin_logaritmo10(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let log10_id = self.asegurar_funcion_c("log10", &[types::F64], Some(types::F64));
        let log10_ref = self.module.declare_func_in_func(log10_id, builder.func);
        let call = builder.ins().call(log10_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    // Otras útiles
    /// piso(x: Real) -> Real — C floor()
    pub(crate) fn builtin_piso(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let floor_id = self.asegurar_funcion_c("floor", &[types::F64], Some(types::F64));
        let floor_ref = self.module.declare_func_in_func(floor_id, builder.func);
        let call = builder.ins().call(floor_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// techo(x: Real) -> Real — C ceil()
    pub(crate) fn builtin_techo(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let ceil_id = self.asegurar_funcion_c("ceil", &[types::F64], Some(types::F64));
        let ceil_ref = self.module.declare_func_in_func(ceil_id, builder.func);
        let call = builder.ins().call(ceil_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// abs(x: Real) -> Real — C fabs()
    pub(crate) fn builtin_valor_absoluto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fabs_id = self.asegurar_funcion_c("fabs", &[types::F64], Some(types::F64));
        let fabs_ref = self.module.declare_func_in_func(fabs_id, builder.func);
        let call = builder.ins().call(fabs_ref, &[x]);
        Ok(builder.inst_results(call)[0])
    }

    /// fmod(x: Real, y: Real) -> Real — C fmod()
    pub(crate) fn builtin_modulo_flotante(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let y = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fmod_id = self.asegurar_funcion_c("fmod", &[types::F64, types::F64], Some(types::F64));
        let fmod_ref = self.module.declare_func_in_func(fmod_id, builder.func);
        let call = builder.ins().call(fmod_ref, &[x, y]);
        Ok(builder.inst_results(call)[0])
    }

    // ============================================================
    // TRIGONOMETRÍA RÁPIDA — F2/F3: math.rs (polinomios minimax)
    // ============================================================

    /// seno_rapido(x: Real) -> Real — F2: usa libm (funcional, futuro: minimax)
    pub(crate) fn builtin_seno_rapido(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Por ahora usa libm (igual que seno_preciso)
        // Futuro: polinomio minimax para ~10x más rápido
        self.builtin_seno(builder, variables, argumentos)
    }

    /// coseno_rapido(x: Real) -> Real — F2: usa libm (funcional, futuro: minimax)
    pub(crate) fn builtin_coseno_rapido(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_coseno(builder, variables, argumentos)
    }

    /// seno_2pi(fase: Real) -> Real — optimizado para fase ∈ [0, 1)
    pub(crate) fn builtin_seno_2pi(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Por ahora: sin(2π * fase)
        let fase = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let dos_pi = builder.ins().f64const(std::f64::consts::TAU);
        let angulo = builder.ins().fmul(fase, dos_pi);
        let sin_id = self.asegurar_funcion_c("sin", &[types::F64], Some(types::F64));
        let sin_ref = self.module.declare_func_in_func(sin_id, builder.func);
        let call = builder.ins().call(sin_ref, &[angulo]);
        Ok(builder.inst_results(call)[0])
    }

    /// coseno_2pi(fase: Real) -> Real — optimizado para fase ∈ [0, 1)
    pub(crate) fn builtin_coseno_2pi(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let fase = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let dos_pi = builder.ins().f64const(std::f64::consts::TAU);
        let angulo = builder.ins().fmul(fase, dos_pi);
        let cos_id = self.asegurar_funcion_c("cos", &[types::F64], Some(types::F64));
        let cos_ref = self.module.declare_func_in_func(cos_id, builder.func);
        let call = builder.ins().call(cos_ref, &[angulo]);
        Ok(builder.inst_results(call)[0])
    }

    /// exp_rapido(x: Real) -> Real — F2: usa libm
    pub(crate) fn builtin_exponencial_rapido(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_exponencial(builder, variables, argumentos)
    }

    /// log_rapido(x: Real) -> Real — F2: usa libm
    pub(crate) fn builtin_logaritmo_rapido(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_logaritmo(builder, variables, argumentos)
    }

    /// seno_aprox(x: Real) -> Real — F3: tabla + interpolación (placeholder)
    pub(crate) fn builtin_seno_aprox(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Placeholder: usa libm
        self.builtin_seno(builder, variables, argumentos)
    }

}
