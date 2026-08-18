//! Codegen Math — Polinomios minimax para seno/coseno
//! 
//! F7: polinomios Remez sobre Cranelift
//! Cada instrucción llama `builder.ins()` directamente (no almacenar `ins`)

use super::*;
use cranelift_codegen::ir::{types, Value, condcodes::FloatCC};
use cranelift_frontend::FunctionBuilder;

// ─── Coeficientes minimax (Remez) ────────────────────────────────
// seno(2πx) grado 5: x ∈ [0, 0.25]
const SENO_COEF: [f64; 3] = [
    6.2831853071795865,    // c0 = 2π
   -41.3417022403088760,   // c1 = -(2π)³/6
    81.4260845325658680,   // c2 = (2π)⁵/120
];

// coseno(2πx) grado 6: x ∈ [0, 0.25]
const COSENO_COEF: [f64; 4] = [
    1.000000000000000000,  // c0
   -19.7392088021787172,   // c1 = -(2π)²/2
    64.1365764145603870,   // c2 = (2π)⁴/24
   -76.1032447746217250,   // c3 = -(2π)⁶/720
];

/// Horner grado 5 para seno: x × (c0 + x² × (c1 + x² × c2))
fn eval_seno(b: &mut FunctionBuilder, x: Value) -> Value {
    let x2 = b.ins().fmul(x, x);
    let c2 = b.ins().f64const(SENO_COEF[2]);
    let mut acc = b.ins().fmul(c2, x2);
    let c1 = b.ins().f64const(SENO_COEF[1]);
    acc = b.ins().fadd(acc, c1);
    acc = b.ins().fmul(acc, x2);
    let c0 = b.ins().f64const(SENO_COEF[0]);
    acc = b.ins().fadd(acc, c0);
    b.ins().fmul(x, acc)
}

/// Horner grado 6 para coseno: c0 + x² × (c1 + x² × (c2 + x² × c3))
fn eval_coseno(b: &mut FunctionBuilder, x: Value) -> Value {
    let x2 = b.ins().fmul(x, x);
    let c3 = b.ins().f64const(COSENO_COEF[3]);
    let mut acc = b.ins().fmul(c3, x2);
    let c2 = b.ins().f64const(COSENO_COEF[2]);
    acc = b.ins().fadd(acc, c2);
    acc = b.ins().fmul(acc, x2);
    let c1 = b.ins().f64const(COSENO_COEF[1]);
    acc = b.ins().fadd(acc, c1);
    acc = b.ins().fmul(acc, x2);
    let c0 = b.ins().f64const(COSENO_COEF[0]);
    b.ins().fadd(acc, c0)
}

/// seno(2π × x) — x ∈ [0, 1), polinomio grado 5 con reducción por simetrías
pub fn emitir_seno_2pi(b: &mut FunctionBuilder, x: Value) -> Value {
    let uno = b.ins().f64const(1.0);
    let medio = b.ins().f64const(0.5);
    let cuarto = b.ins().f64const(0.25);
    let tres_cuartos = b.ins().f64const(0.75);

    let blk_025 = b.create_block();
    b.append_block_param(blk_025, types::F64);
    let blk_05 = b.create_block();
    b.append_block_param(blk_05, types::F64);
    let blk_075 = b.create_block();
    b.append_block_param(blk_075, types::F64);
    let blk_ge075 = b.create_block();
    b.append_block_param(blk_ge075, types::F64);
    let blk_merge = b.create_block();
    b.append_block_param(blk_merge, types::F64);

    let c1 = b.ins().fcmp(FloatCC::LessThan, x, cuarto);
    b.ins().brif(c1, blk_025, &[x], blk_05, &[x]);

    // x < 0.25: polinomio seno directo
    b.switch_to_block(blk_025);
    b.seal_block(blk_025);
    let ax = b.block_params(blk_025)[0];
    let r1 = eval_seno(b, ax);
    b.ins().jump(blk_merge, &[r1]);

    // x < 0.5: coseno_2pi(x - 0.25)
    b.switch_to_block(blk_05);
    let bx = b.block_params(blk_05)[0];
    let c2 = b.ins().fcmp(FloatCC::LessThan, bx, medio);
    let bx_sub = b.ins().fsub(bx, cuarto);
    let r2 = eval_coseno(b, bx_sub);
    b.ins().brif(c2, blk_05, &[], blk_075, &[bx]);
    b.seal_block(blk_05);
    b.ins().jump(blk_merge, &[r2]);

    // x < 0.75: -seno_2pi(x - 0.5)
    b.switch_to_block(blk_075);
    let cx = b.block_params(blk_075)[0];
    let c3 = b.ins().fcmp(FloatCC::LessThan, cx, tres_cuartos);
    let cx_sub = b.ins().fsub(cx, medio);
    let r3_inner = eval_seno(b, cx_sub);
    let r3 = b.ins().fneg(r3_inner);
    b.ins().brif(c3, blk_075, &[], blk_ge075, &[cx]);
    b.seal_block(blk_075);
    b.ins().jump(blk_merge, &[r3]);

    // x >= 0.75: -seno_2pi(1.0 - x)
    b.switch_to_block(blk_ge075);
    b.seal_block(blk_ge075);
    let dx = b.block_params(blk_ge075)[0];
    let dx_sub = b.ins().fsub(uno, dx);
    let r4_inner = eval_seno(b, dx_sub);
    let r4 = b.ins().fneg(r4_inner);
    b.ins().jump(blk_merge, &[r4]);

    b.switch_to_block(blk_merge);
    b.seal_block(blk_merge);
    b.block_params(blk_merge)[0]
}

/// coseno(2π × x) — x ∈ [0, 1), polinomio grado 6 con reducción por simetrías
pub fn emitir_coseno_2pi(b: &mut FunctionBuilder, x: Value) -> Value {
    let uno = b.ins().f64const(1.0);
    let medio = b.ins().f64const(0.5);
    let cuarto = b.ins().f64const(0.25);
    let tres_cuartos = b.ins().f64const(0.75);

    let blk_025 = b.create_block();
    b.append_block_param(blk_025, types::F64);
    let blk_05 = b.create_block();
    b.append_block_param(blk_05, types::F64);
    let blk_075 = b.create_block();
    b.append_block_param(blk_075, types::F64);
    let blk_ge075 = b.create_block();
    b.append_block_param(blk_ge075, types::F64);
    let blk_merge = b.create_block();
    b.append_block_param(blk_merge, types::F64);

    let c1 = b.ins().fcmp(FloatCC::LessThan, x, cuarto);
    b.ins().brif(c1, blk_025, &[x], blk_05, &[x]);

    // x < 0.25: polinomio coseno directo
    b.switch_to_block(blk_025);
    b.seal_block(blk_025);
    let ax = b.block_params(blk_025)[0];
    let r1 = eval_coseno(b, ax);
    b.ins().jump(blk_merge, &[r1]);

    // x < 0.5: -seno_2pi(x - 0.25)
    b.switch_to_block(blk_05);
    let bx = b.block_params(blk_05)[0];
    let c2 = b.ins().fcmp(FloatCC::LessThan, bx, medio);
    let bx_sub = b.ins().fsub(bx, cuarto);
    let r2_inner = eval_seno(b, bx_sub);
    let r2 = b.ins().fneg(r2_inner);
    b.ins().brif(c2, blk_05, &[], blk_075, &[bx]);
    b.seal_block(blk_05);
    b.ins().jump(blk_merge, &[r2]);

    // x < 0.75: -coseno_2pi(x - 0.5)
    b.switch_to_block(blk_075);
    let cx = b.block_params(blk_075)[0];
    let c3 = b.ins().fcmp(FloatCC::LessThan, cx, tres_cuartos);
    let cx_sub = b.ins().fsub(cx, medio);
    let r3_inner = eval_coseno(b, cx_sub);
    let r3 = b.ins().fneg(r3_inner);
    b.ins().brif(c3, blk_075, &[], blk_ge075, &[cx]);
    b.seal_block(blk_075);
    b.ins().jump(blk_merge, &[r3]);

    // x >= 0.75: -coseno_2pi(1.0 - x)
    b.switch_to_block(blk_ge075);
    b.seal_block(blk_ge075);
    let dx = b.block_params(blk_ge075)[0];
    let dx_sub = b.ins().fsub(uno, dx);
    let r4_inner = eval_coseno(b, dx_sub);
    let r4 = b.ins().fneg(r4_inner);
    b.ins().jump(blk_merge, &[r4]);

    b.switch_to_block(blk_merge);
    b.seal_block(blk_merge);
    b.block_params(blk_merge)[0]
}
