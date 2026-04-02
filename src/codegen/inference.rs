use crate::common::NyType;
use crate::parser::ast::*;

use super::builtins;
use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    // ------------------------------------------------------------------
    // Infer the NyType of an expression (best-effort, used for codegen)
    // ------------------------------------------------------------------

    pub(super) fn infer_expr_type(&self, expr: &Expr) -> NyType {
        match expr {
            Expr::Literal { value, .. } => match value {
                LitValue::Int(_) => NyType::I32,
                LitValue::Float(_) => NyType::F64,
                LitValue::Bool(_) => NyType::Bool,
                LitValue::Str(_) => NyType::Str,
            },
            Expr::Ident { name, .. } => {
                if let Some((_, ty)) = self.variables.get(name) {
                    ty.clone()
                } else {
                    NyType::I32
                }
            }
            Expr::BinOp { op, lhs, rhs, .. } => match op {
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Gt
                | BinOp::Le
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => NyType::Bool,
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => {
                    let lhs_ty = self.infer_expr_type(lhs);
                    let rhs_ty = self.infer_expr_type(rhs);
                    // Return the wider numeric type
                    if lhs_ty.is_numeric() && rhs_ty.is_numeric() && lhs_ty != rhs_ty {
                        let l_bits = self.int_bit_width(&lhs_ty);
                        let r_bits = self.int_bit_width(&rhs_ty);
                        if lhs_ty.is_float() || rhs_ty.is_float() {
                            NyType::F64 // promote to f64 for mixed
                        } else if r_bits > l_bits {
                            rhs_ty
                        } else {
                            lhs_ty
                        }
                    } else {
                        lhs_ty
                    }
                }
            },
            Expr::UnaryOp { op, operand, .. } => match op {
                UnaryOp::Not => NyType::Bool,
                UnaryOp::Neg | UnaryOp::BitNot => self.infer_expr_type(operand),
            },
            Expr::Cast { target_type, .. } => self.resolve_type_annotation(target_type),
            Expr::Call { callee, .. } => {
                if let Some((_, _, ret_ty)) = self.functions.get(callee) {
                    ret_ty.clone()
                } else {
                    // Check if callee is a variable holding a function pointer
                    if let Some((_, var_ty)) = self.variables.get(callee) {
                        if let NyType::Function { ret, .. } = var_ty {
                            return *ret.clone();
                        }
                    }
                    // Also check closure captures for return type
                    if let Some((lambda_name, _)) = self.closure_captures.get(callee) {
                        if let Some((_, _, ret_ty)) = self.functions.get(lambda_name) {
                            return ret_ty.clone();
                        }
                    }
                    // Use builtin registry for return types
                    builtins::builtin_return_type(callee, &[]).unwrap_or(NyType::Unit)
                }
            }
            Expr::If { then_branch, .. } => self.infer_expr_type(then_branch),
            Expr::Block { tail_expr, .. } => {
                if let Some(expr) = tail_expr {
                    self.infer_expr_type(expr)
                } else {
                    NyType::Unit
                }
            }
            Expr::ArrayLit { elements, .. } => {
                let size = elements.len();
                let elem_ty = if let Some(first) = elements.first() {
                    self.infer_expr_type(first)
                } else {
                    NyType::I32
                };
                NyType::Array {
                    elem: Box::new(elem_ty),
                    size,
                }
            }
            Expr::Index { object, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match obj_ty {
                    NyType::Array { elem, .. } => *elem,
                    _ => NyType::I32,
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.infer_expr_type(object);
                let struct_ty = match &obj_ty {
                    NyType::Pointer(inner) => inner.as_ref(),
                    other => other,
                };
                if let Some(ft) = struct_ty.field_type(field) {
                    ft.clone()
                } else {
                    NyType::I32
                }
            }
            Expr::StructInit { name, .. } => {
                if let Some(fields) = self.struct_types.get(name) {
                    NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    }
                } else {
                    NyType::I32
                }
            }
            Expr::AddrOf { operand, .. } => {
                let inner_ty = self.infer_expr_type(operand);
                NyType::Pointer(Box::new(inner_ty))
            }
            Expr::Deref { operand, .. } => {
                let ptr_ty = self.infer_expr_type(operand);
                match ptr_ty {
                    NyType::Pointer(inner) => *inner,
                    _ => NyType::I32,
                }
            }
            Expr::MethodCall { object, method, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match &obj_ty {
                    NyType::Vec(elem) => match method.as_str() {
                        "len" => NyType::I64,
                        "get" | "pop" => *elem.clone(),
                        "contains" => NyType::Bool,
                        "index_of" => NyType::I32,
                        _ => NyType::Unit,
                    },
                    NyType::Slice(_) => match method.as_str() {
                        "len" => NyType::I64,
                        _ => NyType::Unit,
                    },
                    NyType::Str => match method.as_str() {
                        "len" => NyType::I64,
                        "substr" | "trim" | "to_upper" | "to_lower" => NyType::Str,
                        "char_at" | "index_of" => NyType::I32,
                        "contains" | "starts_with" | "ends_with" => NyType::Bool,
                        _ => NyType::Unit,
                    },
                    _ => {
                        // Look up by method name first
                        if let Some((_, _, ret_ty)) = self.functions.get(method) {
                            return ret_ty.clone();
                        }
                        // Try TypeName_method convention
                        let type_name = match &obj_ty {
                            NyType::Struct { name, .. } => name.clone(),
                            NyType::Pointer(inner) => match inner.as_ref() {
                                NyType::Struct { name, .. } => name.clone(),
                                _ => String::new(),
                            },
                            _ => String::new(),
                        };
                        if !type_name.is_empty() {
                            let qualified = format!("{}_{}", type_name, method);
                            if let Some((_, _, ret_ty)) = self.functions.get(&qualified) {
                                return ret_ty.clone();
                            }
                        }
                        NyType::Unit
                    }
                }
            }
            Expr::Match { arms, .. } => {
                if let Some(first_arm) = arms.first() {
                    self.infer_expr_type(&first_arm.body)
                } else {
                    NyType::Unit
                }
            }
            Expr::TupleLit { elements, .. } => {
                let elem_types: Vec<NyType> =
                    elements.iter().map(|e| self.infer_expr_type(e)).collect();
                NyType::Tuple(elem_types)
            }
            Expr::TupleIndex { object, index, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match obj_ty {
                    NyType::Tuple(elems) => elems.get(*index).cloned().unwrap_or(NyType::I32),
                    _ => NyType::I32,
                }
            }
            Expr::Try { operand, .. } => {
                let op_ty = self.infer_expr_type(operand);
                match &op_ty {
                    NyType::Enum { variants, .. } => {
                        if let Some((_, payload)) = variants.first() {
                            if payload.is_empty() {
                                NyType::Unit
                            } else {
                                payload[0].clone()
                            }
                        } else {
                            NyType::I32
                        }
                    }
                    _ => NyType::I32,
                }
            }
            Expr::Lambda {
                params,
                return_type,
                ..
            } => {
                let param_types: Vec<NyType> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(&p.ty))
                    .collect();
                let ret_ty = self.resolve_type_annotation(return_type);
                NyType::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                }
            }
            Expr::RangeIndex { object, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match obj_ty {
                    NyType::Array { elem, .. } => NyType::Slice(elem),
                    NyType::Slice(elem) => NyType::Slice(elem),
                    _ => NyType::Slice(Box::new(NyType::I32)),
                }
            }
            Expr::EnumVariant { enum_name, .. } => {
                if let Some(variant_defs) = self.enum_variants.get(enum_name) {
                    NyType::Enum {
                        name: enum_name.clone(),
                        variants: variant_defs.clone(),
                    }
                } else {
                    NyType::I32
                }
            }
        }
    }
}
