//! MIR expression to LLVM IR translation.
//!
//! Implements `codegen_expr` which translates each MIR expression variant
//! into corresponding LLVM IR instructions using the alloca+mem2reg pattern
//! for control flow merges.

use inkwell::intrinsics::Intrinsic;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum};
use inkwell::IntPredicate;

use super::intrinsics::get_intrinsic;
use super::types::{closure_type, variant_struct_type};
use super::CodeGen;
use crate::mir::{BinOp, MirChildSpec, MirExpr, MirMatchArm, MirPattern, MirType, UnaryOp};
use crate::pattern::compile::compile_match;

impl<'ctx> CodeGen<'ctx> {
    /// Generate LLVM IR for a MIR expression.
    ///
    /// Returns the LLVM value representing the expression result.
    pub(crate) fn codegen_expr(
        &mut self,
        expr: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            MirExpr::IntLit(val, _) => {
                Ok(self.context.i64_type().const_int(*val as u64, true).into())
            }

            MirExpr::FloatLit(val, _) => {
                Ok(self.context.f64_type().const_float(*val).into())
            }

            MirExpr::BoolLit(val, _) => {
                Ok(self
                    .context
                    .bool_type()
                    .const_int(if *val { 1 } else { 0 }, false)
                    .into())
            }

            MirExpr::StringLit(s, _) => self.codegen_string_lit(s),

            MirExpr::Var(name, ty) => self.codegen_var(name, ty),

            MirExpr::BinOp { op, lhs, rhs, ty } => self.codegen_binop(op, lhs, rhs, ty),

            MirExpr::UnaryOp { op, operand, ty } => self.codegen_unaryop(op, operand, ty),

            MirExpr::Call { func, args, ty } => self.codegen_call(func, args, ty),

            MirExpr::ClosureCall { closure, args, ty } => {
                self.codegen_closure_call(closure, args, ty)
            }

            MirExpr::If {
                cond,
                then_body,
                else_body,
                ty,
            } => self.codegen_if(cond, then_body, else_body, ty),

            MirExpr::Let {
                name,
                ty,
                value,
                body,
            } => self.codegen_let(name, ty, value, body),

            MirExpr::Block(exprs, _ty) => self.codegen_block(exprs),

            MirExpr::Match {
                scrutinee,
                arms,
                ty,
            } => self.codegen_match(scrutinee, arms, ty),

            MirExpr::StructLit {
                name,
                fields,
                ty: _,
            } => self.codegen_struct_lit(name, fields),

            MirExpr::StructUpdate {
                base,
                overrides,
                ty,
            } => self.codegen_struct_update(base, overrides, ty),

            MirExpr::FieldAccess { object, field, ty } => {
                self.codegen_field_access(object, field, ty)
            }

            MirExpr::ConstructVariant {
                type_name,
                variant,
                fields,
                ty: _,
            } => self.codegen_construct_variant(type_name, variant, fields),

            MirExpr::MakeClosure {
                fn_name,
                captures,
                ty: _,
            } => self.codegen_make_closure(fn_name, captures),

            MirExpr::Return(inner) => self.codegen_return(inner),

            MirExpr::Panic {
                message,
                file,
                line,
            } => self.codegen_panic(message, file, *line),

            MirExpr::Unit => Ok(self.context.struct_type(&[], false).const_zero().into()),

            // Actor primitives
            MirExpr::ActorSpawn {
                func,
                args,
                priority,
                terminate_callback,
                ty: _,
            } => self.codegen_actor_spawn(func, args, *priority, terminate_callback.as_deref()),

            MirExpr::ActorSend {
                target,
                message,
                ty: _,
            } => self.codegen_actor_send(target, message),

            MirExpr::ActorReceive {
                arms,
                timeout_ms,
                timeout_body,
                ty,
            } => self.codegen_actor_receive(arms, timeout_ms.as_deref(), timeout_body.as_deref(), ty),

            MirExpr::ActorSelf { ty: _ } => self.codegen_actor_self(),

            MirExpr::ActorLink { target, ty: _ } => self.codegen_actor_link(target),

            MirExpr::ListLit { elements, .. } => self.codegen_list_lit(elements),

            MirExpr::While { cond, body, ty } => self.codegen_while(cond, body, ty),

            MirExpr::Break => self.codegen_break(),

            MirExpr::Continue => self.codegen_continue(),

            MirExpr::ForInRange { var, start, end, filter, body, ty } => {
                self.codegen_for_in_range(var, start, end, filter.as_deref(), body, ty)
            }

            MirExpr::ForInList { var, collection, filter, body, elem_ty, body_ty, ty } => {
                self.codegen_for_in_list(var, collection, filter.as_deref(), body, elem_ty, body_ty, ty)
            }

            MirExpr::ForInMap { key_var, val_var, collection, filter, body, key_ty, val_ty, body_ty, ty } => {
                self.codegen_for_in_map(key_var, val_var, collection, filter.as_deref(), body, key_ty, val_ty, body_ty, ty)
            }

            MirExpr::ForInSet { var, collection, filter, body, elem_ty, body_ty, ty } => {
                self.codegen_for_in_set(var, collection, filter.as_deref(), body, elem_ty, body_ty, ty)
            }

            MirExpr::ForInIterator { var, iterator, filter, body, elem_ty, body_ty, next_fn, iter_fn, ty } => {
                self.codegen_for_in_iterator(var, iterator, filter.as_deref(), body, elem_ty, body_ty, next_fn, iter_fn, ty)
            }

            MirExpr::SupervisorStart {
                name,
                strategy,
                max_restarts,
                max_seconds,
                children,
                ty: _,
            } => self.codegen_supervisor_start(name, *strategy, *max_restarts, *max_seconds, children),

            MirExpr::TailCall { args, .. } => {
                let tce_loop_bb = self.tce_loop_header
                    .ok_or("TailCall encountered but no TCE loop header set")?;

                // Step 1: Evaluate ALL arguments to temporary values FIRST.
                // This is critical: if args reference current params (e.g., swap(b, a)),
                // we must read all param values before overwriting any of them.
                let mut new_vals = Vec::with_capacity(args.len());
                for arg in args.iter() {
                    new_vals.push(self.codegen_expr(arg)?);
                }

                // Step 2: Store all evaluated values into parameter allocas.
                for (i, param_name) in self.tce_param_names.clone().iter().enumerate() {
                    if let Some(&alloca) = self.locals.get(param_name) {
                        self.builder
                            .build_store(alloca, new_vals[i])
                            .map_err(|e| e.to_string())?;
                    }
                }

                // Step 3: Emit reduction check for preemptive scheduling.
                // Without this, tight tail-recursive loops would starve other actors.
                self.emit_reduction_check();

                // Step 4: Branch to loop header.
                self.builder
                    .build_unconditional_branch(tce_loop_bb)
                    .map_err(|e| e.to_string())?;

                // Return a dummy value -- this block is terminated, the value is never used.
                // Use i64 zero as the dummy (same pattern as Break/Continue).
                Ok(self.context.i64_type().const_zero().into())
            }
        }
    }

    // ── String literals ──────────────────────────────────────────────

    pub(crate) fn codegen_string_lit(&mut self, s: &str) -> Result<BasicValueEnum<'ctx>, String> {
        // Create a global constant for the string data
        let str_val = self.context.const_string(s.as_bytes(), false);
        let global = self.module.add_global(str_val.get_type(), None, ".str");
        global.set_initializer(&str_val);
        global.set_constant(true);
        global.set_unnamed_addr(true);

        // Call mesh_string_new(data_ptr, len)
        let data_ptr = global.as_pointer_value();
        let len = self
            .context
            .i64_type()
            .const_int(s.len() as u64, false);

        let string_new = get_intrinsic(&self.module, "mesh_string_new");
        let result = self
            .builder
            .build_call(
                string_new,
                &[data_ptr.into(), len.into()],
                "str",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_string_new returned void".to_string())
    }

    // ── Variable reference ───────────────────────────────────────────

    fn codegen_var(
        &mut self,
        name: &str,
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Math.pi constant (Phase 43) -- accessed without parentheses as a variable
        if name == "mesh_math_pi" {
            return Ok(self.context.f64_type().const_float(std::f64::consts::PI).into());
        }

        // Check if it's a known function reference (for passing as fn ptr)
        if let Some(fn_val) = self.functions.get(name) {
            return Ok(fn_val.as_global_value().as_pointer_value().into());
        }

        // Check if it's a runtime intrinsic function (e.g., mesh_int_to_string
        // used as a callback function pointer for collection Display).
        if let Some(fn_val) = self.module.get_function(name) {
            return Ok(fn_val.as_global_value().as_pointer_value().into());
        }

        // Load from local variable alloca
        if let Some(alloca) = self.locals.get(name) {
            let alloca = *alloca;
            let llvm_ty = self.llvm_type(ty);
            let val = self
                .builder
                .build_load(llvm_ty, alloca, name)
                .map_err(|e| e.to_string())?;
            Ok(val)
        } else {
            Err(format!("Undefined variable '{}'", name))
        }
    }

    // ── Binary operations ────────────────────────────────────────────

    fn codegen_binop(
        &mut self,
        op: &BinOp,
        lhs: &MirExpr,
        rhs: &MirExpr,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Short-circuit for boolean And/Or
        match op {
            BinOp::And => return self.codegen_short_circuit_and(lhs, rhs),
            BinOp::Or => return self.codegen_short_circuit_or(lhs, rhs),
            _ => {}
        }

        let lhs_val = self.codegen_expr(lhs)?;
        let rhs_val = self.codegen_expr(rhs)?;

        let lhs_ty = lhs.ty();

        // Concat operator: list ++ or string ++
        if matches!(op, BinOp::Concat) {
            if matches!(lhs_ty, MirType::Ptr) {
                return self.codegen_list_concat(lhs_val, rhs_val);
            }
            return self.codegen_string_concat(lhs_val, rhs_val);
        }

        // String equality
        if matches!(lhs_ty, MirType::String) && matches!(op, BinOp::Eq | BinOp::NotEq) {
            return self.codegen_string_compare(op, lhs_val, rhs_val);
        }

        match lhs_ty {
            MirType::Int => self.codegen_int_binop(op, lhs_val, rhs_val),
            MirType::Float => self.codegen_float_binop(op, lhs_val, rhs_val),
            MirType::Bool => self.codegen_bool_binop(op, lhs_val, rhs_val),
            _ => Err(format!("Unsupported binop type: {:?}", lhs_ty)),
        }
    }

    fn codegen_int_binop(
        &mut self,
        op: &BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l = lhs.into_int_value();
        let r = rhs.into_int_value();

        let result: BasicValueEnum<'ctx> = match op {
            BinOp::Add => self.builder.build_int_add(l, r, "add").map_err(|e| e.to_string())?.into(),
            BinOp::Sub => self.builder.build_int_sub(l, r, "sub").map_err(|e| e.to_string())?.into(),
            BinOp::Mul => self.builder.build_int_mul(l, r, "mul").map_err(|e| e.to_string())?.into(),
            BinOp::Div => self.builder.build_int_signed_div(l, r, "div").map_err(|e| e.to_string())?.into(),
            BinOp::Mod => self.builder.build_int_signed_rem(l, r, "mod").map_err(|e| e.to_string())?.into(),
            BinOp::Eq => self.builder.build_int_compare(IntPredicate::EQ, l, r, "eq").map_err(|e| e.to_string())?.into(),
            BinOp::NotEq => self.builder.build_int_compare(IntPredicate::NE, l, r, "ne").map_err(|e| e.to_string())?.into(),
            BinOp::Lt => self.builder.build_int_compare(IntPredicate::SLT, l, r, "lt").map_err(|e| e.to_string())?.into(),
            BinOp::Gt => self.builder.build_int_compare(IntPredicate::SGT, l, r, "gt").map_err(|e| e.to_string())?.into(),
            BinOp::LtEq => self.builder.build_int_compare(IntPredicate::SLE, l, r, "le").map_err(|e| e.to_string())?.into(),
            BinOp::GtEq => self.builder.build_int_compare(IntPredicate::SGE, l, r, "ge").map_err(|e| e.to_string())?.into(),
            _ => return Err(format!("Unsupported int binop: {:?}", op)),
        };
        Ok(result)
    }

    fn codegen_float_binop(
        &mut self,
        op: &BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l = lhs.into_float_value();
        let r = rhs.into_float_value();

        let result: BasicValueEnum<'ctx> = match op {
            BinOp::Add => self.builder.build_float_add(l, r, "fadd").map_err(|e| e.to_string())?.into(),
            BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").map_err(|e| e.to_string())?.into(),
            BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").map_err(|e| e.to_string())?.into(),
            BinOp::Div => self.builder.build_float_div(l, r, "fdiv").map_err(|e| e.to_string())?.into(),
            BinOp::Mod => self.builder.build_float_rem(l, r, "fmod").map_err(|e| e.to_string())?.into(),
            BinOp::Eq => self.builder.build_float_compare(inkwell::FloatPredicate::OEQ, l, r, "feq").map_err(|e| e.to_string())?.into(),
            BinOp::NotEq => self.builder.build_float_compare(inkwell::FloatPredicate::ONE, l, r, "fne").map_err(|e| e.to_string())?.into(),
            BinOp::Lt => self.builder.build_float_compare(inkwell::FloatPredicate::OLT, l, r, "flt").map_err(|e| e.to_string())?.into(),
            BinOp::Gt => self.builder.build_float_compare(inkwell::FloatPredicate::OGT, l, r, "fgt").map_err(|e| e.to_string())?.into(),
            BinOp::LtEq => self.builder.build_float_compare(inkwell::FloatPredicate::OLE, l, r, "fle").map_err(|e| e.to_string())?.into(),
            BinOp::GtEq => self.builder.build_float_compare(inkwell::FloatPredicate::OGE, l, r, "fge").map_err(|e| e.to_string())?.into(),
            _ => return Err(format!("Unsupported float binop: {:?}", op)),
        };
        Ok(result)
    }

    fn codegen_bool_binop(
        &mut self,
        op: &BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l = lhs.into_int_value();
        let r = rhs.into_int_value();

        let result: BasicValueEnum<'ctx> = match op {
            BinOp::Eq => self.builder.build_int_compare(IntPredicate::EQ, l, r, "beq").map_err(|e| e.to_string())?.into(),
            BinOp::NotEq => self.builder.build_int_compare(IntPredicate::NE, l, r, "bne").map_err(|e| e.to_string())?.into(),
            _ => return Err(format!("Unsupported bool binop: {:?}", op)),
        };
        Ok(result)
    }

    fn codegen_short_circuit_and(
        &mut self,
        lhs: &MirExpr,
        rhs: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let lhs_val = self.codegen_expr(lhs)?.into_int_value();

        let rhs_bb = self.context.append_basic_block(fn_val, "and_rhs");
        let merge_bb = self.context.append_basic_block(fn_val, "and_merge");

        self.builder
            .build_conditional_branch(lhs_val, rhs_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // RHS block
        self.builder.position_at_end(rhs_bb);
        let rhs_val = self.codegen_expr(rhs)?.into_int_value();
        let rhs_end_bb = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // Merge block with phi
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.context.bool_type(), "and_result")
            .map_err(|e| e.to_string())?;

        let false_val = self.context.bool_type().const_int(0, false);
        let lhs_bb = fn_val
            .get_basic_blocks()
            .into_iter()
            .find(|bb| {
                // Find the block that branches to merge_bb but is not rhs_end_bb
                bb != &rhs_bb && bb != &merge_bb && bb != &rhs_end_bb
            });

        // Use the block where lhs was evaluated (could be entry or wherever)
        if let Some(lhs_end_bb) = lhs_bb {
            phi.add_incoming(&[(&false_val, lhs_end_bb), (&rhs_val, rhs_end_bb)]);
        } else {
            // Fallback: just use false
            phi.add_incoming(&[(&rhs_val, rhs_end_bb)]);
        }

        Ok(phi.as_basic_value())
    }

    fn codegen_short_circuit_or(
        &mut self,
        lhs: &MirExpr,
        rhs: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let lhs_val = self.codegen_expr(lhs)?.into_int_value();

        let rhs_bb = self.context.append_basic_block(fn_val, "or_rhs");
        let merge_bb = self.context.append_basic_block(fn_val, "or_merge");

        self.builder
            .build_conditional_branch(lhs_val, merge_bb, rhs_bb)
            .map_err(|e| e.to_string())?;

        let lhs_end_bb = self.builder.get_insert_block().unwrap();

        // RHS block
        self.builder.position_at_end(rhs_bb);
        let rhs_val = self.codegen_expr(rhs)?.into_int_value();
        let rhs_end_bb = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.context.bool_type(), "or_result")
            .map_err(|e| e.to_string())?;

        let true_val = self.context.bool_type().const_int(1, false);
        phi.add_incoming(&[(&true_val, lhs_end_bb), (&rhs_val, rhs_end_bb)]);

        Ok(phi.as_basic_value())
    }

    fn codegen_string_concat(
        &mut self,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let concat_fn = get_intrinsic(&self.module, "mesh_string_concat");
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        // Coerce args to ptr if needed (e.g., Unit {} -> null ptr, i64 -> inttoptr)
        let lhs_coerced: BasicMetadataValueEnum<'ctx> = match lhs {
            BasicValueEnum::StructValue(sv) if sv.get_type().count_fields() == 0 => {
                ptr_ty.const_null().into()
            }
            BasicValueEnum::IntValue(iv) => {
                self.builder.build_int_to_ptr(iv, ptr_ty, "concat_arg_ptr")
                    .map_err(|e| e.to_string())?.into()
            }
            other => other.into(),
        };
        let rhs_coerced: BasicMetadataValueEnum<'ctx> = match rhs {
            BasicValueEnum::StructValue(sv) if sv.get_type().count_fields() == 0 => {
                ptr_ty.const_null().into()
            }
            BasicValueEnum::IntValue(iv) => {
                self.builder.build_int_to_ptr(iv, ptr_ty, "concat_arg_ptr")
                    .map_err(|e| e.to_string())?.into()
            }
            other => other.into(),
        };
        let result = self
            .builder
            .build_call(concat_fn, &[lhs_coerced, rhs_coerced], "concat")
            .map_err(|e| e.to_string())?;
        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_string_concat returned void".to_string())
    }

    fn codegen_list_concat(
        &mut self,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let concat_fn = get_intrinsic(&self.module, "mesh_list_concat");
        let result = self
            .builder
            .build_call(concat_fn, &[lhs.into(), rhs.into()], "list_concat")
            .map_err(|e| e.to_string())?;
        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_concat returned void".to_string())
    }

    fn codegen_string_compare(
        &mut self,
        op: &BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let eq_fn = get_intrinsic(&self.module, "mesh_string_eq");
        let result = self
            .builder
            .build_call(eq_fn, &[lhs.into(), rhs.into()], "str_eq")
            .map_err(|e| e.to_string())?;
        let i8_result = result
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_string_eq returned void")?
            .into_int_value();

        let zero = self.context.i8_type().const_int(0, false);
        let eq_result = self
            .builder
            .build_int_compare(IntPredicate::NE, i8_result, zero, "str_eq_bool")
            .map_err(|e| e.to_string())?;

        let final_result = match op {
            BinOp::Eq => eq_result,
            BinOp::NotEq => self
                .builder
                .build_not(eq_result, "str_neq")
                .map_err(|e| e.to_string())?,
            _ => return Err(format!("Unsupported string comparison: {:?}", op)),
        };

        Ok(final_result.into())
    }

    // ── Unary operations ─────────────────────────────────────────────

    fn codegen_unaryop(
        &mut self,
        op: &UnaryOp,
        operand: &MirExpr,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.codegen_expr(operand)?;
        match op {
            UnaryOp::Neg => {
                match operand.ty() {
                    MirType::Int => {
                        let int_val = val.into_int_value();
                        Ok(self
                            .builder
                            .build_int_neg(int_val, "neg")
                            .map_err(|e| e.to_string())?
                            .into())
                    }
                    MirType::Float => {
                        let float_val = val.into_float_value();
                        Ok(self
                            .builder
                            .build_float_neg(float_val, "fneg")
                            .map_err(|e| e.to_string())?
                            .into())
                    }
                    _ => Err(format!("Cannot negate type {:?}", operand.ty())),
                }
            }
            UnaryOp::Not => {
                let bool_val = val.into_int_value();
                Ok(self
                    .builder
                    .build_not(bool_val, "not")
                    .map_err(|e| e.to_string())?
                    .into())
            }
        }
    }

    // ── Function calls ───────────────────────────────────────────────

    fn codegen_call(
        &mut self,
        func: &MirExpr,
        args: &[MirExpr],
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Check if this is a user-defined function (declared via MirFunction).
        // User functions accept closure params as {ptr, ptr} structs directly.
        // Runtime intrinsics expect closures split into separate (fn_ptr, env_ptr) args.
        let is_user_fn = if let MirExpr::Var(name, _) = func {
            self.functions.contains_key(name)
        } else {
            false
        };

        // Compile arguments, splitting closure structs into (fn_ptr, env_ptr) pairs
        // only for runtime intrinsics that expect separate pointer arguments.
        //
        // To avoid adding spurious env_ptr args to functions that don't expect them
        // (e.g., mesh_http_route_post takes a plain fn_ptr, not a closure pair),
        // we check whether expanding closures/FnPtrs would exceed the target function's
        // declared parameter count.
        let target_param_count: Option<usize> = if let MirExpr::Var(name, _) = func {
            self.module.get_function(name).map(|f| f.count_params() as usize)
        } else {
            None
        };
        // Pre-compute how many LLVM args we'd produce with expansion.
        let expanded_arg_count: usize = args.iter().map(|arg| {
            if !is_user_fn && matches!(arg.ty(), MirType::Closure(_, _) | MirType::FnPtr(_, _)) {
                2 // fn_ptr + env_ptr
            } else {
                1
            }
        }).sum();
        // Only expand closure/FnPtr args if the expanded count matches the target's param count.
        // If expansion would cause a mismatch, pass values as-is.
        let should_expand_closures = match target_param_count {
            Some(expected) => expanded_arg_count == expected && !is_user_fn,
            None => !is_user_fn, // unknown target, expand by default
        };

        let mut arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        let mut _has_closure_args = false;
        for arg in args {
            let val = self.codegen_expr(arg)?;
            if matches!(arg.ty(), MirType::Closure(_, _)) && should_expand_closures {
                // Extract fn_ptr and env_ptr from the closure struct { ptr, ptr }.
                let cls_ty = closure_type(self.context);
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                let closure_alloca = self
                    .builder
                    .build_alloca(cls_ty, "cls_split")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(closure_alloca, val)
                    .map_err(|e| e.to_string())?;
                let fn_ptr_gep = self
                    .builder
                    .build_struct_gep(cls_ty, closure_alloca, 0, "cls_fn_ptr")
                    .map_err(|e| e.to_string())?;
                let fn_ptr_val = self
                    .builder
                    .build_load(ptr_ty, fn_ptr_gep, "fn_ptr")
                    .map_err(|e| e.to_string())?;
                let env_ptr_gep = self
                    .builder
                    .build_struct_gep(cls_ty, closure_alloca, 1, "cls_env_ptr")
                    .map_err(|e| e.to_string())?;
                let env_ptr_val = self
                    .builder
                    .build_load(ptr_ty, env_ptr_gep, "env_ptr")
                    .map_err(|e| e.to_string())?;
                arg_vals.push(fn_ptr_val.into());
                arg_vals.push(env_ptr_val.into());
                _has_closure_args = true;
            } else if matches!(arg.ty(), MirType::FnPtr(_, _)) && should_expand_closures {
                // Bare function reference passed to runtime intrinsic.
                // Runtime expects (fn_ptr, env_ptr) pairs; env_ptr is null for non-closures.
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                arg_vals.push(val.into());
                arg_vals.push(ptr_ty.const_null().into());
                _has_closure_args = true;
            } else {
                arg_vals.push(val.into());
            }
        }

        // Check if it's a service call helper (mesh_service_call with inline args).
        // Pattern: Call to mesh_service_call with [pid, tag, ...extra_args]
        // We need to pack extra_args into a payload buffer.
        if let MirExpr::Var(name, _) = func {
            if name == "mesh_service_call" && args.len() >= 2 {
                return self.codegen_service_call_helper(args, ty);
            }
            // Check if it's a service cast helper (mesh_actor_send with [pid, tag, ...args]).
            // Pattern: Call to mesh_actor_send from a __service_*_cast_* function.
            if name == "mesh_actor_send" && args.len() >= 2 {
                // Check if second arg is a literal tag (service cast pattern).
                if let MirExpr::IntLit(_, _) = &args[1] {
                    return self.codegen_service_cast_helper(args);
                }
            }
            // Synthetic tuple allocation intrinsic.
            // __mesh_make_tuple(elem0, elem1, ...) -> ptr
            // Allocates { u64 len, u64[N] elements } on the GC heap.
            if name == "__mesh_make_tuple" {
                return self.codegen_make_tuple(&arg_vals);
            }
            // Timer.send_after(pid, ms, msg) -> mesh_timer_send_after(pid, ms, msg_ptr, msg_size)
            // The 3rd arg (msg) needs message serialization like codegen_actor_send.
            if name == "mesh_timer_send_after" && args.len() == 3 {
                return self.codegen_timer_send_after(args);
            }
            // ── Phase 67: Node distribution special codegen ─────────────────
            // Node.start(name, cookie) -> mesh_node_start(name_ptr, name_len, cookie_ptr, cookie_len)
            if name == "mesh_node_start" && args.len() == 2 {
                return self.codegen_node_start(args);
            }
            // Node.connect(name) -> mesh_node_connect(name_ptr, name_len)
            if name == "mesh_node_connect" {
                return self.codegen_node_string_call(args, "mesh_node_connect");
            }
            // Node.monitor(node_name) -> mesh_node_monitor(name_ptr, name_len)
            if name == "mesh_node_monitor" {
                return self.codegen_node_string_call(args, "mesh_node_monitor");
            }
            // Node.spawn(node, func, args...) -> mesh_node_spawn(node_ptr, node_len, fn_name_ptr, fn_name_len, args_ptr, args_size, link_flag)
            if name == "mesh_node_spawn" {
                return self.codegen_node_spawn(args, &arg_vals, 0);
            }
            // Node.spawn_link -> same as spawn but with link_flag=1
            if name == "mesh_node_spawn_link" {
                return self.codegen_node_spawn(args, &arg_vals, 1);
            }
            // ── Phase 68: Global Registry codegen ───────────────────────────
            // Global.register(name, pid) -> mesh_global_register(name_ptr, name_len, pid)
            if name == "mesh_global_register" && args.len() == 2 {
                return self.codegen_global_register(args);
            }
            // Global.whereis(name) -> mesh_global_whereis(name_ptr, name_len)
            if name == "mesh_global_whereis" {
                return self.codegen_node_string_call(args, "mesh_global_whereis");
            }
            // Global.unregister(name) -> mesh_global_unregister(name_ptr, name_len)
            if name == "mesh_global_unregister" {
                return self.codegen_node_string_call(args, "mesh_global_unregister");
            }
            // List.contains on String elements: redirect to mesh_list_contains_str which
            // uses mesh_string_eq (byte comparison) instead of raw pointer equality.
            if name == "mesh_list_contains" && args.len() == 2
                && matches!(args[1].ty(), MirType::String)
            {
                let list_val = self.codegen_expr(&args[0])?;
                let elem_val = self.codegen_expr(&args[1])?;
                let f = get_intrinsic(&self.module, "mesh_list_contains_str");
                let result = self.builder
                    .build_call(f, &[list_val.into(), elem_val.into()], "list_contains_str")
                    .map_err(|e| e.to_string())?;
                return result.try_as_basic_value().basic()
                    .ok_or_else(|| "mesh_list_contains_str returned void".to_string());
            }
        }

        // Math/Int/Float stdlib intrinsics (Phase 43)
        if let MirExpr::Var(name, _) = func {
            match name.as_str() {
                "mesh_math_abs" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    return match args[0].ty() {
                        MirType::Int => {
                            // llvm.abs.i64(val, is_int_min_poison=false)
                            let intrinsic = Intrinsic::find("llvm.abs").ok_or("llvm.abs not found")?;
                            let i64_ty = self.context.i64_type();
                            let decl = intrinsic.get_declaration(&self.module, &[i64_ty.into()])
                                .ok_or("Failed to get llvm.abs declaration")?;
                            let is_poison = self.context.bool_type().const_int(0, false);
                            let result = self.builder.build_call(decl, &[arg_val.into(), is_poison.into()], "abs")
                                .map_err(|e| e.to_string())?;
                            result.try_as_basic_value().basic().ok_or("abs returned void".into())
                        }
                        MirType::Float => {
                            // llvm.fabs.f64(val)
                            let intrinsic = Intrinsic::find("llvm.fabs").ok_or("llvm.fabs not found")?;
                            let f64_ty = self.context.f64_type();
                            let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                                .ok_or("Failed to get llvm.fabs declaration")?;
                            let result = self.builder.build_call(decl, &[arg_val.into()], "fabs")
                                .map_err(|e| e.to_string())?;
                            result.try_as_basic_value().basic().ok_or("fabs returned void".into())
                        }
                        other => Err(format!("Math.abs: unsupported type {:?}", other)),
                    };
                }
                "mesh_math_min" => {
                    let lhs = self.codegen_expr(&args[0])?;
                    let rhs = self.codegen_expr(&args[1])?;
                    return match args[0].ty() {
                        MirType::Int => {
                            let intrinsic = Intrinsic::find("llvm.smin").ok_or("llvm.smin not found")?;
                            let i64_ty = self.context.i64_type();
                            let decl = intrinsic.get_declaration(&self.module, &[i64_ty.into()])
                                .ok_or("Failed to get llvm.smin declaration")?;
                            let result = self.builder.build_call(decl, &[lhs.into(), rhs.into()], "smin")
                                .map_err(|e| e.to_string())?;
                            result.try_as_basic_value().basic().ok_or("smin returned void".into())
                        }
                        MirType::Float => {
                            let intrinsic = Intrinsic::find("llvm.minnum").ok_or("llvm.minnum not found")?;
                            let f64_ty = self.context.f64_type();
                            let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                                .ok_or("Failed to get llvm.minnum declaration")?;
                            let result = self.builder.build_call(decl, &[lhs.into(), rhs.into()], "minnum")
                                .map_err(|e| e.to_string())?;
                            result.try_as_basic_value().basic().ok_or("minnum returned void".into())
                        }
                        other => Err(format!("Math.min: unsupported type {:?}", other)),
                    };
                }
                "mesh_math_max" => {
                    let lhs = self.codegen_expr(&args[0])?;
                    let rhs = self.codegen_expr(&args[1])?;
                    return match args[0].ty() {
                        MirType::Int => {
                            let intrinsic = Intrinsic::find("llvm.smax").ok_or("llvm.smax not found")?;
                            let i64_ty = self.context.i64_type();
                            let decl = intrinsic.get_declaration(&self.module, &[i64_ty.into()])
                                .ok_or("Failed to get llvm.smax declaration")?;
                            let result = self.builder.build_call(decl, &[lhs.into(), rhs.into()], "smax")
                                .map_err(|e| e.to_string())?;
                            result.try_as_basic_value().basic().ok_or("smax returned void".into())
                        }
                        MirType::Float => {
                            let intrinsic = Intrinsic::find("llvm.maxnum").ok_or("llvm.maxnum not found")?;
                            let f64_ty = self.context.f64_type();
                            let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                                .ok_or("Failed to get llvm.maxnum declaration")?;
                            let result = self.builder.build_call(decl, &[lhs.into(), rhs.into()], "maxnum")
                                .map_err(|e| e.to_string())?;
                            result.try_as_basic_value().basic().ok_or("maxnum returned void".into())
                        }
                        other => Err(format!("Math.max: unsupported type {:?}", other)),
                    };
                }
                "mesh_int_to_float" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    let int_val = arg_val.into_int_value();
                    let float_val = self.builder
                        .build_signed_int_to_float(int_val, self.context.f64_type(), "int_to_float")
                        .map_err(|e| e.to_string())?;
                    return Ok(float_val.into());
                }
                "mesh_float_to_int" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    let float_val = arg_val.into_float_value();
                    let int_val = self.builder
                        .build_float_to_signed_int(float_val, self.context.i64_type(), "float_to_int")
                        .map_err(|e| e.to_string())?;
                    return Ok(int_val.into());
                }
                // ── pow/sqrt/floor/ceil/round (Phase 43 Plan 02) ──────────
                "mesh_math_pow" => {
                    let base_val = self.codegen_expr(&args[0])?;
                    let exp_val = self.codegen_expr(&args[1])?;
                    let intrinsic = Intrinsic::find("llvm.pow").ok_or("llvm.pow not found")?;
                    let f64_ty = self.context.f64_type();
                    let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                        .ok_or("Failed to get llvm.pow declaration")?;
                    let result = self.builder.build_call(decl, &[base_val.into(), exp_val.into()], "pow")
                        .map_err(|e| e.to_string())?;
                    return result.try_as_basic_value().basic().ok_or("pow returned void".into());
                }
                "mesh_math_sqrt" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    let intrinsic = Intrinsic::find("llvm.sqrt").ok_or("llvm.sqrt not found")?;
                    let f64_ty = self.context.f64_type();
                    let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                        .ok_or("Failed to get llvm.sqrt declaration")?;
                    let result = self.builder.build_call(decl, &[arg_val.into()], "sqrt")
                        .map_err(|e| e.to_string())?;
                    return result.try_as_basic_value().basic().ok_or("sqrt returned void".into());
                }
                "mesh_math_floor" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    let intrinsic = Intrinsic::find("llvm.floor").ok_or("llvm.floor not found")?;
                    let f64_ty = self.context.f64_type();
                    let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                        .ok_or("Failed to get llvm.floor declaration")?;
                    let float_result = self.builder.build_call(decl, &[arg_val.into()], "floor")
                        .map_err(|e| e.to_string())?
                        .try_as_basic_value().basic().ok_or("floor returned void")?;
                    let int_result = self.builder
                        .build_float_to_signed_int(float_result.into_float_value(), self.context.i64_type(), "floor_to_int")
                        .map_err(|e| e.to_string())?;
                    return Ok(int_result.into());
                }
                "mesh_math_ceil" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    let intrinsic = Intrinsic::find("llvm.ceil").ok_or("llvm.ceil not found")?;
                    let f64_ty = self.context.f64_type();
                    let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                        .ok_or("Failed to get llvm.ceil declaration")?;
                    let float_result = self.builder.build_call(decl, &[arg_val.into()], "ceil")
                        .map_err(|e| e.to_string())?
                        .try_as_basic_value().basic().ok_or("ceil returned void")?;
                    let int_result = self.builder
                        .build_float_to_signed_int(float_result.into_float_value(), self.context.i64_type(), "ceil_to_int")
                        .map_err(|e| e.to_string())?;
                    return Ok(int_result.into());
                }
                "mesh_math_round" => {
                    let arg_val = self.codegen_expr(&args[0])?;
                    let intrinsic = Intrinsic::find("llvm.round").ok_or("llvm.round not found")?;
                    let f64_ty = self.context.f64_type();
                    let decl = intrinsic.get_declaration(&self.module, &[f64_ty.into()])
                        .ok_or("Failed to get llvm.round declaration")?;
                    let float_result = self.builder.build_call(decl, &[arg_val.into()], "round")
                        .map_err(|e| e.to_string())?
                        .try_as_basic_value().basic().ok_or("round returned void")?;
                    let int_result = self.builder
                        .build_float_to_signed_int(float_result.into_float_value(), self.context.i64_type(), "round_to_int")
                        .map_err(|e| e.to_string())?;
                    return Ok(int_result.into());
                }
                _ => {} // Fall through to normal call handling
            }
        }

        // Check if it's a direct call to a known function
        if let MirExpr::Var(name, _) = func {
            if let Some(fn_val) = self.functions.get(name).copied() {
                // Coerce argument types to match function parameter types.
                // Handles cases where MIR type resolution is imprecise (e.g.,
                // Unit {} values that should be Int/Ptr, or struct vs ptr mismatches).
                let param_types = fn_val.get_type().get_param_types();
                let mut coerced_args = arg_vals.clone();
                for (i, param_ty) in param_types.iter().enumerate() {
                    if i < coerced_args.len() {
                        match coerced_args[i] {
                            BasicMetadataValueEnum::StructValue(sv) => {
                                // Unit {} (empty struct) -> i64/ptr: pass zero/null
                                if sv.get_type().count_fields() == 0 {
                                    if let inkwell::types::BasicMetadataTypeEnum::IntType(it) = param_ty {
                                        coerced_args[i] = it.const_zero().into();
                                    } else if let inkwell::types::BasicMetadataTypeEnum::PointerType(pt) = param_ty {
                                        coerced_args[i] = pt.const_null().into();
                                    }
                                } else if let inkwell::types::BasicMetadataTypeEnum::PointerType(_) = param_ty {
                                    // Non-empty struct -> ptr: heap-alloc + store
                                    let sv_ty = sv.get_type();
                                    let i64_type = self.context.i64_type();
                                    let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                                    let align = i64_type.const_int(8, false);
                                    let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                                        .ok_or("mesh_gc_alloc_actor not found")?;
                                    let heap_ptr = self.builder
                                        .build_call(gc_alloc, &[size.into(), align.into()], "arg_struct_heap")
                                        .map_err(|e| e.to_string())?
                                        .try_as_basic_value()
                                        .basic()
                                        .ok_or("gc_alloc returned void")?
                                        .into_pointer_value();
                                    self.builder
                                        .build_store(heap_ptr, sv)
                                        .map_err(|e| e.to_string())?;
                                    coerced_args[i] = heap_ptr.into();
                                } else if let inkwell::types::BasicMetadataTypeEnum::IntType(it) = param_ty {
                                    if it.get_bit_width() == 64 {
                                        // Non-empty struct -> i64: heap-alloc + ptrtoint
                                        let sv_ty = sv.get_type();
                                        let i64_type = self.context.i64_type();
                                        let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                                        let align = i64_type.const_int(8, false);
                                        let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                                            .ok_or("mesh_gc_alloc_actor not found")?;
                                        let heap_ptr = self.builder
                                            .build_call(gc_alloc, &[size.into(), align.into()], "arg_struct_heap")
                                            .map_err(|e| e.to_string())?
                                            .try_as_basic_value()
                                            .basic()
                                            .ok_or("gc_alloc returned void")?
                                            .into_pointer_value();
                                        self.builder
                                            .build_store(heap_ptr, sv)
                                            .map_err(|e| e.to_string())?;
                                        let cast = self.builder
                                            .build_ptr_to_int(heap_ptr, *it, "struct_ptr_to_i64")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                }
                            }
                            BasicMetadataValueEnum::IntValue(iv) => {
                                if let inkwell::types::BasicMetadataTypeEnum::PointerType(_) = param_ty {
                                    if iv.get_type().get_bit_width() == 64 {
                                        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                                        let cast = self.builder
                                            .build_int_to_ptr(iv, ptr_ty, "i64_to_ptr")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                } else if let inkwell::types::BasicMetadataTypeEnum::IntType(param_it) = param_ty {
                                    if iv.get_type().get_bit_width() < param_it.get_bit_width() {
                                        let extended = self.builder
                                            .build_int_z_extend(iv, *param_it, "zext_arg")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = extended.into();
                                    }
                                }
                            }
                            BasicMetadataValueEnum::PointerValue(pv) => {
                                if let inkwell::types::BasicMetadataTypeEnum::IntType(param_it) = param_ty {
                                    if param_it.get_bit_width() == 64 {
                                        let cast = self.builder
                                            .build_ptr_to_int(pv, *param_it, "ptr_to_i64")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                let call = self
                    .builder
                    .build_call(fn_val, &coerced_args, "call")
                    .map_err(|e| e.to_string())?;

                // Insert reduction check after function call
                self.emit_reduction_check();

                if matches!(ty, MirType::Unit) {
                    return Ok(self.context.struct_type(&[], false).const_zero().into());
                }

                let result = call
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "Function call returned void".to_string())?;

                // User function returning struct when MIR expects Ptr (e.g.,
                // From conversion returning a struct that goes into Result's
                // { i8, ptr } Err variant). Heap-allocate the struct and return
                // a pointer so it survives the current stack frame.
                if matches!(ty, MirType::Ptr) {
                    if let BasicValueEnum::StructValue(sv) = result {
                        let sv_ty = sv.get_type();
                        let i64_type = self.context.i64_type();
                        let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                        let align = i64_type.const_int(8, false);
                        let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                            .ok_or("mesh_gc_alloc_actor not found")?;
                        let heap_ptr = self.builder
                            .build_call(gc_alloc, &[size.into(), align.into()], "struct_to_ptr")
                            .map_err(|e| e.to_string())?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("gc_alloc returned void")?
                            .into_pointer_value();
                        self.builder
                            .build_store(heap_ptr, sv)
                            .map_err(|e| e.to_string())?;
                        return Ok(heap_ptr.into());
                    }
                }

                return Ok(result);
            }

            // Check if it's a runtime intrinsic (don't add reduction check for runtime calls)
            if let Some(fn_val) = self.module.get_function(name) {
                // Coerce argument types to match runtime function signatures:
                // - Bool i1 -> i8/i64 (zero-extend)
                // - Ptr -> i64 (ptrtoint, for uniform-value functions like map_put)
                // - Float f64 -> i64 (bitcast, for uniform-value functions like list_append)
                let mut coerced_args = arg_vals.clone();
                let param_types = fn_val.get_type().get_param_types();
                for (i, param_ty) in param_types.iter().enumerate() {
                    if i < coerced_args.len() {
                        match coerced_args[i] {
                            BasicMetadataValueEnum::IntValue(arg_iv) => {
                                if let inkwell::types::BasicMetadataTypeEnum::IntType(param_it) = param_ty {
                                    if arg_iv.get_type().get_bit_width() < param_it.get_bit_width() {
                                        let extended = self
                                            .builder
                                            .build_int_z_extend(arg_iv, *param_it, "zext_arg")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = extended.into();
                                    }
                                } else if let inkwell::types::BasicMetadataTypeEnum::PointerType(_) = param_ty {
                                    // Runtime function expects ptr but we have i64
                                    // (e.g., connection handle passed to mesh_ws_send).
                                    if arg_iv.get_type().get_bit_width() == 64 {
                                        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                                        let cast = self
                                            .builder
                                            .build_int_to_ptr(arg_iv, ptr_ty, "i64_to_ptr")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                }
                            }
                            BasicMetadataValueEnum::PointerValue(arg_pv) => {
                                // If the runtime function expects i64 but we have a pointer
                                // (e.g., string values passed to mesh_map_put), cast ptr->i64.
                                if let inkwell::types::BasicMetadataTypeEnum::IntType(param_it) = param_ty {
                                    if param_it.get_bit_width() == 64 {
                                        let cast = self
                                            .builder
                                            .build_ptr_to_int(arg_pv, *param_it, "ptr_to_i64")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                }
                            }
                            BasicMetadataValueEnum::FloatValue(arg_fv) => {
                                // If the runtime function expects i64 but we have a float
                                // (e.g., Float values passed to mesh_list_append), bitcast f64->i64.
                                if let inkwell::types::BasicMetadataTypeEnum::IntType(param_it) = param_ty {
                                    if param_it.get_bit_width() == 64 {
                                        let cast = self
                                            .builder
                                            .build_bit_cast(arg_fv, *param_it, "f64_to_i64")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                }
                            }
                            BasicMetadataValueEnum::StructValue(arg_sv) => {
                                // Special case: empty struct {} (Unit type) - pass null/zero.
                                // This occurs when type resolution couldn't determine the variable's
                                // type and defaulted to Unit.
                                if arg_sv.get_type().count_fields() == 0 {
                                    if let inkwell::types::BasicMetadataTypeEnum::PointerType(pt) = param_ty {
                                        coerced_args[i] = pt.const_null().into();
                                    } else if let inkwell::types::BasicMetadataTypeEnum::IntType(it) = param_ty {
                                        coerced_args[i] = it.const_zero().into();
                                    }
                                }
                                // If the runtime function expects i64 but we have a struct value
                                // (e.g., ConnectionState passed to mesh_map_put), heap-alloc + ptrtoint.
                                else if let inkwell::types::BasicMetadataTypeEnum::IntType(param_it) = param_ty {
                                    if param_it.get_bit_width() == 64 {
                                        let sv_ty = arg_sv.get_type();
                                        let i64_type = self.context.i64_type();
                                        let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                                        let align = i64_type.const_int(8, false);
                                        let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                                            .ok_or("mesh_gc_alloc_actor not found")?;
                                        let heap_ptr = self.builder
                                            .build_call(gc_alloc, &[size.into(), align.into()], "struct_heap")
                                            .map_err(|e| e.to_string())?
                                            .try_as_basic_value()
                                            .basic()
                                            .ok_or("gc_alloc returned void")?
                                            .into_pointer_value();
                                        self.builder
                                            .build_store(heap_ptr, arg_sv)
                                            .map_err(|e| e.to_string())?;
                                        let cast = self
                                            .builder
                                            .build_ptr_to_int(heap_ptr, *param_it, "struct_ptr_to_i64")
                                            .map_err(|e| e.to_string())?;
                                        coerced_args[i] = cast.into();
                                    }
                                }
                                // If the runtime function expects a pointer but we have a struct value
                                // (e.g., struct passed to mesh_alloc_result), heap-allocate + store + pass ptr.
                                // Must use GC heap (not stack alloca) because the pointer may be stored
                                // in a MeshResult that outlives the current stack frame.
                                else if let inkwell::types::BasicMetadataTypeEnum::PointerType(_) = param_ty {
                                    let sv_ty = arg_sv.get_type();
                                    let i64_type = self.context.i64_type();
                                    let _ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                                    let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                                    let align = i64_type.const_int(8, false);
                                    let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                                        .ok_or("mesh_gc_alloc_actor not found")?;
                                    let heap_ptr = self.builder
                                        .build_call(gc_alloc, &[size.into(), align.into()], "struct_heap")
                                        .map_err(|e| e.to_string())?
                                        .try_as_basic_value()
                                        .basic()
                                        .ok_or("gc_alloc returned void")?
                                        .into_pointer_value();
                                    self.builder
                                        .build_store(heap_ptr, arg_sv)
                                        .map_err(|e| e.to_string())?;
                                    coerced_args[i] = heap_ptr.into();
                                }
                            }
                            _ => {}
                        }
                    }
                }

                let call = self
                    .builder
                    .build_call(fn_val, &coerced_args, "call")
                    .map_err(|e| e.to_string())?;

                if matches!(ty, MirType::Unit) {
                    return Ok(self.context.struct_type(&[], false).const_zero().into());
                }

                let result = call
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "Function call returned void".to_string())?;

                // Runtime functions returning i8 or i64 for Bool values need
                // truncation to i1 to match Mesh's Bool representation.
                // i8: functions like mesh_set_contains that return bool as i8.
                // i64: functions like mesh_list_get that return u64 (uniform storage).
                if matches!(ty, MirType::Bool) {
                    if let BasicValueEnum::IntValue(iv) = result {
                        let bw = iv.get_type().get_bit_width();
                        if bw > 1 {
                            let i1_val = self
                                .builder
                                .build_int_truncate(iv, self.context.bool_type(), "to_bool")
                                .map_err(|e| e.to_string())?;
                            return Ok(i1_val.into());
                        }
                    }
                }

                // Runtime functions returning i64 for Float values (e.g., list_get
                // returning a Float stored as bitcast u64) need bitcast conversion.
                if matches!(ty, MirType::Float) {
                    if let BasicValueEnum::IntValue(iv) = result {
                        if iv.get_type().get_bit_width() == 64 {
                            let f64_val = self
                                .builder
                                .build_bit_cast(iv, self.context.f64_type(), "i64_to_f64")
                                .map_err(|e| e.to_string())?;
                            return Ok(f64_val.into());
                        }
                    }
                }

                // Runtime functions returning i64 for pointer values (e.g., map_get
                // returning a string pointer as u64) need inttoptr conversion.
                if matches!(ty, MirType::String | MirType::Ptr
                    | MirType::Struct(_) | MirType::SumType(_)
                    | MirType::Pid(_)) {
                    if let BasicValueEnum::IntValue(iv) = result {
                        if iv.get_type().get_bit_width() == 64 {
                            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                            let ptr_val = self
                                .builder
                                .build_int_to_ptr(iv, ptr_ty, "i64_to_ptr")
                                .map_err(|e| e.to_string())?;
                            return Ok(ptr_val.into());
                        }
                    }
                }

                // Functions returning struct values when MIR expects Ptr (e.g.,
                // From conversion returning a struct that goes into Result's
                // { i8, ptr } Err variant). Heap-allocate the struct and return
                // a pointer so it survives the current stack frame.
                if matches!(ty, MirType::Ptr) {
                    if let BasicValueEnum::StructValue(sv) = result {
                        let sv_ty = sv.get_type();
                        let i64_type = self.context.i64_type();
                        let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                        let align = i64_type.const_int(8, false);
                        let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                            .ok_or("mesh_gc_alloc_actor not found")?;
                        let heap_ptr = self.builder
                            .build_call(gc_alloc, &[size.into(), align.into()], "struct_to_ptr")
                            .map_err(|e| e.to_string())?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("gc_alloc returned void")?
                            .into_pointer_value();
                        self.builder
                            .build_store(heap_ptr, sv)
                            .map_err(|e| e.to_string())?;
                        return Ok(heap_ptr.into());
                    }
                }

                return Ok(result);
            }
        }

        // Indirect call through a function pointer or closure
        let fn_ptr = self.codegen_expr(func)?;
        let ret_ty = self.llvm_type(ty);
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = args
            .iter()
            .map(|a| self.llvm_type(a.ty()).into())
            .collect();
        let fn_type = ret_ty.fn_type(&param_types, false);

        let call = self
            .builder
            .build_indirect_call(fn_type, fn_ptr.into_pointer_value(), &arg_vals, "icall")
            .map_err(|e| e.to_string())?;

        // Insert reduction check after indirect call
        self.emit_reduction_check();

        if matches!(ty, MirType::Unit) {
            return Ok(self.context.struct_type(&[], false).const_zero().into());
        }

        call.try_as_basic_value()
            .basic()
            .ok_or_else(|| "Indirect call returned void".to_string())
    }

    // ── Closure calls ────────────────────────────────────────────────

    fn codegen_closure_call(
        &mut self,
        closure: &MirExpr,
        args: &[MirExpr],
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let closure_val = self.codegen_expr(closure)?;
        let closure_ty = closure_type(self.context);

        // Alloca for the closure struct so we can GEP into it
        let closure_alloca = self
            .builder
            .build_alloca(closure_ty, "closure_tmp")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(closure_alloca, closure_val)
            .map_err(|e| e.to_string())?;

        // Load fn_ptr (field 0)
        let fn_ptr_ptr = self
            .builder
            .build_struct_gep(closure_ty, closure_alloca, 0, "fn_ptr_ptr")
            .map_err(|e| e.to_string())?;
        let fn_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(inkwell::AddressSpace::default()),
                fn_ptr_ptr,
                "fn_ptr",
            )
            .map_err(|e| e.to_string())?;

        // Load env_ptr (field 1)
        let env_ptr_ptr = self
            .builder
            .build_struct_gep(closure_ty, closure_alloca, 1, "env_ptr_ptr")
            .map_err(|e| e.to_string())?;
        let env_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(inkwell::AddressSpace::default()),
                env_ptr_ptr,
                "env_ptr",
            )
            .map_err(|e| e.to_string())?;

        // Build call args: env_ptr first, then user args
        let mut call_args: Vec<BasicMetadataValueEnum<'ctx>> = vec![env_ptr.into()];
        for arg in args {
            let val = self.codegen_expr(arg)?;
            call_args.push(val.into());
        }

        // Build the function type for the indirect call
        let ret_ty = self.llvm_type(ty);
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = vec![
            self.context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ];
        for arg in args {
            param_types.push(self.llvm_type(arg.ty()).into());
        }
        let fn_type = ret_ty.fn_type(&param_types, false);

        let call = self
            .builder
            .build_indirect_call(fn_type, fn_ptr.into_pointer_value(), &call_args, "clscall")
            .map_err(|e| e.to_string())?;

        // Insert reduction check after closure call
        self.emit_reduction_check();

        if matches!(ty, MirType::Unit) {
            return Ok(self.context.struct_type(&[], false).const_zero().into());
        }

        call.try_as_basic_value()
            .basic()
            .ok_or_else(|| "Closure call returned void".to_string())
    }

    // ── If/else expression ───────────────────────────────────────────

    fn codegen_if(
        &mut self,
        cond: &MirExpr,
        then_body: &MirExpr,
        else_body: &MirExpr,
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let cond_val = self.codegen_expr(cond)?.into_int_value();

        // LLVM br requires an i1 condition. If the condition is a wider integer
        // (e.g., i64 from mesh_result_is_ok), truncate to i1 (nonzero = true).
        let cond_i1 = if cond_val.get_type().get_bit_width() != 1 {
            self.builder
                .build_int_truncate(cond_val, self.context.bool_type(), "cond_i1")
                .map_err(|e| e.to_string())?
        } else {
            cond_val
        };

        let result_ty = self.llvm_type(ty);
        // Use entry-block alloca to prevent stack growth in TCE loops.
        let result_alloca = if self.tce_loop_header.is_some() {
            self.build_entry_alloca(result_ty, "if_result")?
        } else {
            self.builder
                .build_alloca(result_ty, "if_result")
                .map_err(|e| e.to_string())?
        };

        let then_bb = self.context.append_basic_block(fn_val, "then");
        let else_bb = self.context.append_basic_block(fn_val, "else");
        let merge_bb = self.context.append_basic_block(fn_val, "if_merge");

        self.builder
            .build_conditional_branch(cond_i1, then_bb, else_bb)
            .map_err(|e| e.to_string())?;

        // Then branch
        self.builder.position_at_end(then_bb);
        let then_val = self.codegen_expr(then_body)?;
        // Only store result and branch if block is not already terminated
        // (break/continue/return may have terminated the block)
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder
                .build_store(result_alloca, then_val)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| e.to_string())?;
        }

        // Else branch
        self.builder.position_at_end(else_bb);
        let else_val = self.codegen_expr(else_body)?;
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder
                .build_store(result_alloca, else_val)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| e.to_string())?;
        }

        // Merge block
        self.builder.position_at_end(merge_bb);
        let result = self
            .builder
            .build_load(result_ty, result_alloca, "if_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    // ── Let binding ──────────────────────────────────────────────────

    fn codegen_let(
        &mut self,
        name: &str,
        ty: &MirType,
        value: &MirExpr,
        body: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let llvm_ty = self.llvm_type(ty);
        // Use entry-block alloca to prevent stack growth in TCE loops.
        let alloca = if self.tce_loop_header.is_some() {
            self.build_entry_alloca(llvm_ty, name)?
        } else {
            self.builder
                .build_alloca(llvm_ty, name)
                .map_err(|e| e.to_string())?
        };

        let val = self.codegen_expr(value)?;

        // When binding a runtime-returned pointer to a sum type or struct variable,
        // dereference the pointer to load the actual value.
        // Runtime functions like mesh_file_read return *mut MeshResult (ptr)
        // but the variable type is SumType (a by-value struct).
        // Similarly, from_json for nested structs returns a heap pointer via
        // mesh_alloc_result/mesh_result_unwrap, but the field type is Struct.
        let val = if matches!(ty, MirType::SumType(_) | MirType::Struct(_))
            && val.is_pointer_value()
            && !llvm_ty.is_pointer_type()
        {
            self.builder
                .build_load(llvm_ty, val.into_pointer_value(), "deref_sum")
                .map_err(|e| e.to_string())?
        } else {
            val
        };

        self.builder
            .build_store(alloca, val)
            .map_err(|e| e.to_string())?;

        // Register the variable
        let old_alloca = self.locals.insert(name.to_string(), alloca);
        let old_type = self.local_types.insert(name.to_string(), ty.clone());

        // Compile the body
        let result = self.codegen_expr(body)?;

        // Restore previous binding (if any)
        if let Some(prev) = old_alloca {
            self.locals.insert(name.to_string(), prev);
        } else {
            self.locals.remove(name);
        }
        if let Some(prev_ty) = old_type {
            self.local_types.insert(name.to_string(), prev_ty);
        } else {
            self.local_types.remove(name);
        }

        Ok(result)
    }

    // ── Block expression ─────────────────────────────────────────────

    fn codegen_block(
        &mut self,
        exprs: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if exprs.is_empty() {
            return Ok(self.context.struct_type(&[], false).const_zero().into());
        }

        let mut result = self.context.struct_type(&[], false).const_zero().into();
        for expr in exprs {
            // If the current block is already terminated (e.g., by break/continue/return),
            // skip remaining expressions -- they are unreachable.
            if let Some(bb) = self.builder.get_insert_block() {
                if bb.get_terminator().is_some() {
                    break;
                }
            }
            result = self.codegen_expr(expr)?;
        }
        Ok(result)
    }

    // ── Pattern match ────────────────────────────────────────────────

    fn codegen_match(
        &mut self,
        scrutinee: &MirExpr,
        arms: &[MirMatchArm],
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Evaluate the scrutinee
        let scrutinee_val = self.codegen_expr(scrutinee)?;
        let scrutinee_ty = scrutinee.ty();

        // Alloca for the scrutinee so pattern codegen can GEP into it.
        // For runtime-returned sum types (heap pointers), the value was already
        // dereferenced at the let binding site, so here it's a proper struct value.
        let scrutinee_llvm_ty = self.llvm_type(scrutinee_ty);
        let scrutinee_alloca = if matches!(scrutinee_ty, MirType::SumType(_))
            && scrutinee_val.is_pointer_value()
            && !scrutinee_llvm_ty.is_pointer_type()
        {
            // Rare case: scrutinee is a direct pointer to a sum type
            // (e.g., inline case on a function call result). Use it directly.
            scrutinee_val.into_pointer_value()
        } else {
            let alloca = if self.tce_loop_header.is_some() {
                self.build_entry_alloca(scrutinee_llvm_ty, "scrutinee")?
            } else {
                self.builder
                    .build_alloca(scrutinee_llvm_ty, "scrutinee")
                    .map_err(|e| e.to_string())?
            };
            self.builder
                .build_store(alloca, scrutinee_val)
                .map_err(|e| e.to_string())?;
            alloca
        };

        // Compile pattern to decision tree
        let tree = compile_match(scrutinee_ty, arms, "<unknown>", 0, &self.sum_type_defs);

        // Alloca for the match result
        let result_ty = self.llvm_type(ty);
        let result_alloca = if self.tce_loop_header.is_some() {
            self.build_entry_alloca(result_ty, "match_result")?
        } else {
            self.builder
                .build_alloca(result_ty, "match_result")
                .map_err(|e| e.to_string())?
        };

        let fn_val = self.current_function();
        let merge_bb = self.context.append_basic_block(fn_val, "match_merge");

        // Generate code for the decision tree
        self.codegen_decision_tree(
            &tree,
            scrutinee_alloca,
            scrutinee_ty,
            arms,
            result_alloca,
            merge_bb,
        )?;

        // Merge block
        self.builder.position_at_end(merge_bb);
        let result = self
            .builder
            .build_load(result_ty, result_alloca, "match_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    // ── Struct literal ───────────────────────────────────────────────

    fn codegen_struct_lit(
        &mut self,
        name: &str,
        fields: &[(String, MirExpr)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let struct_ty = self
            .struct_types
            .get(name)
            .ok_or_else(|| format!("Unknown struct type '{}'", name))?;
        let struct_ty = *struct_ty;

        let alloca = self
            .builder
            .build_alloca(struct_ty, "struct_lit")
            .map_err(|e| e.to_string())?;

        for (i, (_, field_expr)) in fields.iter().enumerate() {
            let val = self.codegen_expr(field_expr)?;
            let field_ptr = self
                .builder
                .build_struct_gep(struct_ty, alloca, i as u32, "field_ptr")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(field_ptr, val)
                .map_err(|e| e.to_string())?;
        }

        let result = self
            .builder
            .build_load(struct_ty.as_basic_type_enum(), alloca, "struct_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    // ── Struct update ─────────────────────────────────────────────────

    fn codegen_struct_update(
        &mut self,
        base: &MirExpr,
        overrides: &[(String, MirExpr)],
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Determine the struct name from the type.
        let struct_name = match ty {
            MirType::Struct(name) => name.clone(),
            _ => return Err(format!("Struct update on non-struct type: {:?}", ty)),
        };

        let struct_ty = self
            .struct_types
            .get(&struct_name)
            .ok_or_else(|| format!("Unknown struct type '{}'", struct_name))?;
        let struct_ty = *struct_ty;

        // Get the struct field definitions so we know the order.
        let field_defs = self
            .mir_struct_defs
            .get(&struct_name)
            .ok_or_else(|| format!("Unknown struct type '{}'", struct_name))?
            .clone();

        // Codegen the base expression.
        let base_val = self.codegen_expr(base)?;

        // Allocate a temporary for the base struct so we can GEP into it.
        let base_alloca = self
            .builder
            .build_alloca(struct_ty.as_basic_type_enum(), "update_base")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(base_alloca, base_val)
            .map_err(|e| e.to_string())?;

        // Allocate the new struct.
        let new_alloca = self
            .builder
            .build_alloca(struct_ty, "struct_update")
            .map_err(|e| e.to_string())?;

        // Build a lookup map for the override field names.
        let override_map: std::collections::HashMap<&str, &MirExpr> = overrides
            .iter()
            .map(|(name, expr)| (name.as_str(), expr))
            .collect();

        // For each field in the struct definition:
        for (i, (field_name, _field_ty)) in field_defs.iter().enumerate() {
            let val = if let Some(override_expr) = override_map.get(field_name.as_str()) {
                // Override: codegen the override expression.
                self.codegen_expr(override_expr)?
            } else {
                // Copy from base: load the field from the base struct.
                let field_ptr = self
                    .builder
                    .build_struct_gep(struct_ty, base_alloca, i as u32, "base_field_ptr")
                    .map_err(|e| e.to_string())?;
                let field_llvm_ty = struct_ty.get_field_type_at_index(i as u32)
                    .ok_or_else(|| format!("No field at index {} in struct '{}'", i, struct_name))?;
                self.builder
                    .build_load(field_llvm_ty, field_ptr, "base_field_val")
                    .map_err(|e| e.to_string())?
            };

            // Store the value in the new struct.
            let new_field_ptr = self
                .builder
                .build_struct_gep(struct_ty, new_alloca, i as u32, "new_field_ptr")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(new_field_ptr, val)
                .map_err(|e| e.to_string())?;
        }

        // Load and return the new struct value.
        let result = self
            .builder
            .build_load(struct_ty.as_basic_type_enum(), new_alloca, "struct_update_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    // ── Field access ─────────────────────────────────────────────────

    fn codegen_field_access(
        &mut self,
        object: &MirExpr,
        field: &str,
        ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.codegen_expr(object)?;

        // Determine the struct name
        let struct_name = match object.ty() {
            MirType::Struct(name) => name.clone(),
            _ => return Err(format!("Field access on non-struct type: {:?}", object.ty())),
        };

        let struct_ty = self
            .struct_types
            .get(&struct_name)
            .ok_or_else(|| format!("Unknown struct type '{}'", struct_name))?;
        let struct_ty = *struct_ty;

        let field_idx = self.find_struct_field_index(&struct_name, field)?;

        let alloca = self
            .builder
            .build_alloca(struct_ty.as_basic_type_enum(), "obj_tmp")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(alloca, obj_val)
            .map_err(|e| e.to_string())?;

        let field_ptr = self
            .builder
            .build_struct_gep(struct_ty, alloca, field_idx as u32, "field_ptr")
            .map_err(|e| e.to_string())?;

        let result_ty = self.llvm_type(ty);
        let result = self
            .builder
            .build_load(result_ty, field_ptr, "field_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    /// Find the field index in a struct definition.
    pub(crate) fn find_struct_field_index(&self, struct_name: &str, field: &str) -> Result<usize, String> {
        let fields = self
            .mir_struct_defs
            .get(struct_name)
            .ok_or_else(|| format!("Unknown struct type '{}'", struct_name))?;

        fields
            .iter()
            .position(|(name, _)| name == field)
            .ok_or_else(|| format!("Field '{}' not found in struct '{}'", field, struct_name))
    }

    // ── Sum type variant construction ────────────────────────────────

    fn codegen_construct_variant(
        &mut self,
        type_name: &str,
        variant: &str,
        fields: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let sum_layout = self
            .lookup_sum_type_layout(type_name)
            .ok_or_else(|| format!("Unknown sum type '{}'", type_name))?;
        let sum_layout = *sum_layout;

        let sum_def = self
            .lookup_sum_type_def(type_name)
            .ok_or_else(|| format!("Unknown sum type def '{}'", type_name))?
            .clone();

        let variant_def = sum_def
            .variants
            .iter()
            .find(|v| v.name == variant)
            .ok_or_else(|| format!("Unknown variant '{}.{}'", type_name, variant))?;

        let tag = variant_def.tag;

        // Alloca the sum type
        let alloca = self
            .builder
            .build_alloca(sum_layout.as_basic_type_enum(), "variant")
            .map_err(|e| e.to_string())?;

        // Store the tag at field 0
        let tag_ptr = self
            .builder
            .build_struct_gep(sum_layout, alloca, 0, "tag_ptr")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(tag as u64, false))
            .map_err(|e| e.to_string())?;

        // Store fields: create variant overlay struct type and GEP into it
        if !fields.is_empty() {
            let field_types: Vec<MirType> = variant_def.fields.clone();
            let variant_ty =
                variant_struct_type(self.context, &field_types, &self.struct_types, &self.sum_type_layouts);

            // Store each field via the variant overlay.
            // For generic sum types like Result/Option where the layout is {i8, ptr},
            // a struct payload (e.g., Ok(MyStruct{x,y})) may be larger than the 8-byte
            // ptr slot. In that case, heap-allocate the struct and store the pointer
            // so the store fits within the {i8, ptr} layout without overflow.
            for (i, field_expr) in fields.iter().enumerate() {
                let val = self.codegen_expr(field_expr)?;

                // Struct values in variant fields must always be heap-allocated (pointer-boxed).
                // The variant layout {i8 tag, ptr data} stores an opaque pointer in the data
                // slot. Pattern matching reads the slot as a pointer and dereferences it, so
                // storing a struct directly (even an 8-byte one) causes a dereference of a
                // raw integer value (segfault). Always box: heap-allocate and store the ptr.
                let val = if val.is_struct_value() {
                    let sv = val.into_struct_value();
                    let sv_ty = sv.get_type();
                    let i64_ty = self.context.i64_type();
                    let target_data = self.target_machine.get_target_data();
                    let struct_size = target_data.get_store_size(&sv_ty);
                    let size = sv_ty.size_of().unwrap_or(i64_ty.const_int(struct_size, false));
                    let align = i64_ty.const_int(8, false);
                    let gc_alloc = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
                    let heap_ptr = self
                        .builder
                        .build_call(gc_alloc, &[size.into(), align.into()], "variant_box")
                        .map_err(|e| e.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("mesh_gc_alloc_actor returned void")?
                        .into_pointer_value();
                    self.builder
                        .build_store(heap_ptr, sv)
                        .map_err(|e| e.to_string())?;
                    heap_ptr.into()
                } else {
                    val
                };

                // GEP into the variant overlay: field 0 is tag, field 1+ are data fields
                let field_ptr = self
                    .builder
                    .build_struct_gep(variant_ty, alloca, (i + 1) as u32, "vfield_ptr")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(field_ptr, val)
                    .map_err(|e| e.to_string())?;
            }
        }

        // Load the complete sum type value
        let result = self
            .builder
            .build_load(sum_layout.as_basic_type_enum(), alloca, "variant_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    // ── Closure creation ─────────────────────────────────────────────

    fn codegen_make_closure(
        &mut self,
        fn_name: &str,
        captures: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cls_ty = closure_type(self.context);

        // Get the function pointer
        let fn_val = self
            .functions
            .get(fn_name)
            .ok_or_else(|| format!("Closure function '{}' not found", fn_name))?;
        let fn_ptr = fn_val.as_global_value().as_pointer_value();

        // Allocate environment on GC heap.
        // Mesh closures always have __env as first param, so env_ptr must be
        // non-null even for zero-capture closures. This ensures runtime HOFs
        // (map, filter, reduce) use the closure calling convention fn(env, ...).
        let env_ptr = if captures.is_empty() {
            // No captures -> allocate a minimal 8-byte env (non-null sentinel).
            let gc_alloc = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
            let size_val = self.context.i64_type().const_int(8, false);
            let align_val = self.context.i64_type().const_int(8, false);
            let env_raw = self
                .builder
                .build_call(gc_alloc, &[size_val.into(), align_val.into()], "env_dummy")
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("mesh_gc_alloc_actor returned void")?;
            env_raw.into_pointer_value()
        } else {
            // Build an env struct type from capture types
            let cap_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = captures
                .iter()
                .map(|c| self.llvm_type(c.ty()))
                .collect();
            let env_struct_ty = self.context.struct_type(&cap_types, false);

            // Calculate size via target data
            let target_data = self.target_machine.get_target_data();
            let env_size = target_data.get_store_size(&env_struct_ty);

            // Allocate via mesh_gc_alloc_actor(size, align=8)
            let gc_alloc = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
            let size_val = self.context.i64_type().const_int(env_size, false);
            let align_val = self.context.i64_type().const_int(8, false);
            let env_raw = self
                .builder
                .build_call(gc_alloc, &[size_val.into(), align_val.into()], "env_raw")
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("mesh_gc_alloc_actor returned void")?;

            let env_ptr_val = env_raw.into_pointer_value();

            // Store each captured value into the env struct
            for (i, cap_expr) in captures.iter().enumerate() {
                let val = self.codegen_expr(cap_expr)?;
                let field_ptr = self
                    .builder
                    .build_struct_gep(env_struct_ty, env_ptr_val, i as u32, "cap_ptr")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(field_ptr, val)
                    .map_err(|e| e.to_string())?;
            }

            env_ptr_val
        };

        // Pack into closure struct { fn_ptr, env_ptr }
        let closure_alloca = self
            .builder
            .build_alloca(cls_ty, "closure")
            .map_err(|e| e.to_string())?;

        let fn_slot = self
            .builder
            .build_struct_gep(cls_ty, closure_alloca, 0, "fn_slot")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(fn_slot, fn_ptr)
            .map_err(|e| e.to_string())?;

        let env_slot = self
            .builder
            .build_struct_gep(cls_ty, closure_alloca, 1, "env_slot")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(env_slot, env_ptr)
            .map_err(|e| e.to_string())?;

        let result = self
            .builder
            .build_load(cls_ty.as_basic_type_enum(), closure_alloca, "closure_val")
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    // ── Return ───────────────────────────────────────────────────────

    fn codegen_return(
        &mut self,
        inner: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.codegen_expr(inner)?;

        // When the inner expression produces a pointer but the function returns
        // a struct type (e.g., Result's { i8, ptr }), load the struct value
        // from the pointer before the return instruction. This handles cases
        // where variant construction or other codegen paths return a pointer
        // instead of the value itself.
        let fn_val = self.current_function();
        let ret_ty = fn_val.get_type().get_return_type();
        let return_val = if val.is_pointer_value() {
            if let Some(ret_ty) = ret_ty {
                if ret_ty.is_struct_type() {
                    self.builder
                        .build_load(ret_ty, val.into_pointer_value(), "ret_load")
                        .map_err(|e| e.to_string())?
                } else {
                    val
                }
            } else {
                val
            }
        } else {
            val
        };

        self.builder
            .build_return(Some(&return_val))
            .map_err(|e| e.to_string())?;
        // Return a dummy value since we've already emitted a return
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    // ── Actor primitives ──────────────────────────────────────────────

    fn codegen_actor_spawn(
        &mut self,
        func: &MirExpr,
        args: &[MirExpr],
        priority: u8,
        terminate_callback: Option<&MirExpr>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_ty = self.context.i64_type();

        // Get function pointer for the actor entry function.
        let fn_ptr_val = self.codegen_expr(func)?;
        let fn_ptr = if fn_ptr_val.is_pointer_value() {
            fn_ptr_val.into_pointer_value()
        } else {
            // Cast to pointer if needed
            self.builder
                .build_int_to_ptr(fn_ptr_val.into_int_value(), ptr_ty, "fn_ptr")
                .map_err(|e| e.to_string())?
        };

        // Serialize arguments to a byte buffer.
        // For scalar args (int, ptr, float): each is stored as an i64 (8 bytes).
        // For struct args: the struct is stored directly with its full byte size.
        let (args_ptr, args_size) = if args.is_empty() {
            (ptr_ty.const_null(), i64_ty.const_int(0, false))
        } else {
            let arg_vals: Vec<BasicValueEnum<'ctx>> = args
                .iter()
                .map(|a| self.codegen_expr(a))
                .collect::<Result<Vec<_>, _>>()?;

            // Compute the total byte size needed for all args.
            // Scalar values (int, ptr, float) take 8 bytes each.
            // Struct values take their full LLVM store size.
            let target_data = self.target_machine.get_target_data();
            let mut total_size: u64 = 0;
            let mut arg_offsets: Vec<u64> = Vec::new();
            for val in &arg_vals {
                arg_offsets.push(total_size);
                if val.is_struct_value() {
                    let st = val.into_struct_value().get_type();
                    total_size += target_data.get_store_size(&st);
                } else {
                    total_size += 8; // i64, ptr, or float
                }
            }

            // Allocate spawn args on the GC heap (not the stack) because the
            // actor runs asynchronously after the caller returns. Stack allocas
            // would be freed before the actor reads the args.
            let gc_alloc_fn = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
            let size_val = i64_ty.const_int(total_size, false);
            let align_val = i64_ty.const_int(8, false);
            let buf_alloca = self.builder
                .build_call(gc_alloc_fn, &[size_val.into(), align_val.into()], "spawn_args")
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("mesh_gc_alloc_actor returned void")?
                .into_pointer_value();

            // Store each arg into the buffer at its computed offset.
            let i8_ty = self.context.i8_type();
            for (i, val) in arg_vals.iter().enumerate() {
                let offset = arg_offsets[i];
                let element_ptr = if offset == 0 {
                    buf_alloca
                } else {
                    unsafe {
                        self.builder
                            .build_gep(i8_ty, buf_alloca, &[i64_ty.const_int(offset, false)], &format!("arg_ptr_{}", i))
                            .map_err(|e| e.to_string())?
                    }
                };

                if val.is_struct_value() {
                    // Store the full struct value directly.
                    self.builder
                        .build_store(element_ptr, *val)
                        .map_err(|e| e.to_string())?;
                } else {
                    // Convert scalar to i64 and store.
                    let int_val = if val.is_int_value() {
                        val.into_int_value()
                    } else if val.is_pointer_value() {
                        self.builder
                            .build_ptr_to_int(val.into_pointer_value(), i64_ty, "arg_int")
                            .map_err(|e| e.to_string())?
                    } else if val.is_float_value() {
                        self.builder
                            .build_bit_cast(val.into_float_value(), i64_ty, "arg_int")
                            .map_err(|e: inkwell::builder::BuilderError| e.to_string())?
                            .into_int_value()
                    } else {
                        // Fallback: store as zero
                        i64_ty.const_int(0, false)
                    };
                    self.builder
                        .build_store(element_ptr, int_val)
                        .map_err(|e| e.to_string())?;
                }
            }

            (buf_alloca, i64_ty.const_int(total_size, false))
        };

        let priority_val = self.context.i8_type().const_int(priority as u64, false);

        // Call mesh_actor_spawn(fn_ptr, args, args_size, priority) -> i64
        let spawn_fn = get_intrinsic(&self.module, "mesh_actor_spawn");
        let pid_val = self
            .builder
            .build_call(
                spawn_fn,
                &[fn_ptr.into(), args_ptr.into(), args_size.into(), priority_val.into()],
                "pid",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_actor_spawn returned void")?;

        // If terminate callback exists, call mesh_actor_set_terminate(pid, callback_fn_ptr)
        if let Some(cb_expr) = terminate_callback {
            let cb_val = self.codegen_expr(cb_expr)?;
            let cb_ptr = if cb_val.is_pointer_value() {
                cb_val.into_pointer_value()
            } else {
                self.builder
                    .build_int_to_ptr(cb_val.into_int_value(), ptr_ty, "cb_ptr")
                    .map_err(|e| e.to_string())?
            };
            let set_terminate_fn = get_intrinsic(&self.module, "mesh_actor_set_terminate");
            self.builder
                .build_call(
                    set_terminate_fn,
                    &[pid_val.into(), cb_ptr.into()],
                    "",
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(pid_val)
    }

    fn codegen_actor_send(
        &mut self,
        target: &MirExpr,
        message: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Evaluate the target PID (i64).
        let target_val = self.codegen_expr(target)?.into_int_value();

        // Serialize the message to bytes.
        let msg_val = self.codegen_expr(message)?;
        let (msg_ptr, msg_size) = if matches!(message.ty(), MirType::Unit) {
            (ptr_ty.const_null(), i64_ty.const_int(0, false))
        } else {
            // Store the message value on the stack and pass a pointer + size.
            let msg_ty = self.llvm_type(message.ty());
            let msg_alloca = self
                .builder
                .build_alloca(msg_ty, "msg_buf")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(msg_alloca, msg_val)
                .map_err(|e| e.to_string())?;

            // Compute size via target data.
            let target_data = self.target_machine.get_target_data();
            let size = target_data.get_store_size(&msg_ty);

            (msg_alloca, i64_ty.const_int(size, false))
        };

        // Call mesh_actor_send(target_pid, msg_ptr, msg_size)
        let send_fn = get_intrinsic(&self.module, "mesh_actor_send");
        self.builder
            .build_call(
                send_fn,
                &[target_val.into(), msg_ptr.into(), msg_size.into()],
                "",
            )
            .map_err(|e| e.to_string())?;

        // Send returns Unit.
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    /// Codegen for Timer.send_after(pid, ms, msg).
    ///
    /// Serializes the message (3rd arg) to (ptr, size) like codegen_actor_send,
    /// then calls mesh_timer_send_after(pid, ms, msg_ptr, msg_size).
    fn codegen_timer_send_after(
        &mut self,
        args: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();

        // Evaluate pid (i64) and ms (i64).
        let pid_val = self.codegen_expr(&args[0])?.into_int_value();
        let ms_val = self.codegen_expr(&args[1])?.into_int_value();

        // Serialize the message (3rd arg) to (ptr, size) -- same pattern as codegen_actor_send.
        let msg_val = self.codegen_expr(&args[2])?;
        let (msg_ptr, msg_size) = {
            let msg_ty = self.llvm_type(args[2].ty());
            let msg_alloca = self
                .builder
                .build_alloca(msg_ty, "timer_msg_buf")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(msg_alloca, msg_val)
                .map_err(|e| e.to_string())?;

            let target_data = self.target_machine.get_target_data();
            let size = target_data.get_store_size(&msg_ty);

            (msg_alloca, i64_ty.const_int(size, false))
        };

        // Call mesh_timer_send_after(pid, ms, msg_ptr, msg_size)
        let send_after_fn = get_intrinsic(&self.module, "mesh_timer_send_after");
        self.builder
            .build_call(
                send_after_fn,
                &[pid_val.into(), ms_val.into(), msg_ptr.into(), msg_size.into()],
                "",
            )
            .map_err(|e| e.to_string())?;

        // Returns Unit.
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    // ── Phase 67: Node distribution codegen helpers ──────────────────────

    /// Extract raw (data_ptr, len) from a MeshString pointer.
    ///
    /// MeshString layout: `{ len: u64, data_bytes... }`.
    /// - `len` is at offset 0 (first 8 bytes)
    /// - `data_ptr` is at `string_ptr + 8` (bytes following the header)
    fn codegen_unpack_string(
        &mut self,
        string_val: BasicValueEnum<'ctx>,
    ) -> Result<(inkwell::values::PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Get the MeshString pointer (may be a pointer or i64-encoded pointer).
        let string_ptr = if string_val.is_pointer_value() {
            string_val.into_pointer_value()
        } else {
            // inttoptr: i64 -> ptr
            self.builder
                .build_int_to_ptr(string_val.into_int_value(), ptr_ty, "str_ptr")
                .map_err(|e| e.to_string())?
        };

        // Load len from offset 0 (the MeshString header).
        let len_val = self
            .builder
            .build_load(i64_ty, string_ptr, "str_len")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Data pointer is string_ptr + 8 bytes (after the u64 len field).
        let eight = i64_ty.const_int(8, false);
        let data_ptr = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), string_ptr, &[eight], "str_data")
                .map_err(|e| e.to_string())?
        };

        Ok((data_ptr, len_val))
    }

    /// Codegen for Node.start(name, cookie).
    ///
    /// Unpacks two MeshString args into (ptr, len) pairs and calls
    /// mesh_node_start(name_ptr, name_len, cookie_ptr, cookie_len).
    fn codegen_node_start(
        &mut self,
        args: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let name_val = self.codegen_expr(&args[0])?;
        let cookie_val = self.codegen_expr(&args[1])?;

        let (name_ptr, name_len) = self.codegen_unpack_string(name_val)?;
        let (cookie_ptr, cookie_len) = self.codegen_unpack_string(cookie_val)?;

        let start_fn = get_intrinsic(&self.module, "mesh_node_start");
        let result = self
            .builder
            .build_call(
                start_fn,
                &[name_ptr.into(), name_len.into(), cookie_ptr.into(), cookie_len.into()],
                "node_start",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_node_start returned void".to_string())
    }

    /// Codegen for Node functions taking a single string arg (connect, monitor).
    ///
    /// Unpacks the MeshString arg into (ptr, len) and calls the given intrinsic.
    fn codegen_node_string_call(
        &mut self,
        args: &[MirExpr],
        intrinsic_name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let str_val = self.codegen_expr(&args[0])?;
        let (data_ptr, data_len) = self.codegen_unpack_string(str_val)?;

        let func = get_intrinsic(&self.module, intrinsic_name);
        let result = self
            .builder
            .build_call(
                func,
                &[data_ptr.into(), data_len.into()],
                "node_call",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("{} returned void", intrinsic_name))
    }

    /// Codegen for Global.register(name, pid).
    ///
    /// Unpacks the first string argument to (ptr, len), passes the second argument
    /// (pid as i64) through directly. Calls mesh_global_register(name_ptr, name_len, pid).
    fn codegen_global_register(
        &mut self,
        args: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Unpack string argument (name)
        let str_val = self.codegen_expr(&args[0])?;
        let (name_ptr, name_len) = self.codegen_unpack_string(str_val)?;

        // Second argument is pid (i64)
        let pid_val = self.codegen_expr(&args[1])?;

        let func = get_intrinsic(&self.module, "mesh_global_register");
        let result = self
            .builder
            .build_call(
                func,
                &[name_ptr.into(), name_len.into(), pid_val.into()],
                "global_register",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_global_register returned void".to_string())
    }

    /// Codegen for Node.spawn / Node.spawn_link.
    ///
    /// Node.spawn(node_name, func_ref, args...) compiles to:
    ///   mesh_node_spawn(node_ptr, node_len, fn_name_ptr, fn_name_len, args_ptr, args_size, link_flag)
    ///
    /// The function reference (args[1]) is a MirExpr::Var whose name is the function name.
    /// Instead of evaluating it as a function pointer, we emit the function name as a
    /// string constant. The remaining args (args[2..]) are packed into an i64 array
    /// (same as local actor spawn).
    fn codegen_node_spawn(
        &mut self,
        args: &[MirExpr],
        _pre_evaluated: &[BasicMetadataValueEnum<'ctx>],
        link_flag: u8,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let i8_ty = self.context.i8_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // args[0] = node name (String expression).
        // When the arg is a variable with MirType::Unit (from unresolved Ty::Var),
        // codegen_expr loads from the alloca with type {} and returns an empty struct.
        // The actual stored value is an i64 (string pointer). Reload as i64 directly
        // from the local alloca to recover the correct runtime value.
        let node_val = match &args[0] {
            MirExpr::Var(name, mir_ty) if matches!(mir_ty, MirType::Unit) => {
                // Variable has unresolved type -- load as i64 from the alloca directly
                if let Some(alloca) = self.locals.get(name).copied() {
                    self.builder
                        .build_load(i64_ty, alloca, &format!("{}_as_i64", name))
                        .map_err(|e| e.to_string())?
                } else {
                    self.codegen_expr(&args[0])?
                }
            }
            _ => self.codegen_expr(&args[0])?,
        };
        let (node_ptr, node_len) = self.codegen_unpack_string(node_val)?;

        // args[1] = function reference -- extract the name as a string constant.
        // The MIR has this as MirExpr::Var("function_name", FnPtr(...)).
        let fn_name = match &args[1] {
            MirExpr::Var(name, _) => name.clone(),
            _ => {
                // Fallback: try to evaluate and use a placeholder.
                // This should not happen in practice -- Node.spawn's second arg
                // should always be a named function reference.
                "unknown".to_string()
            }
        };

        // Create a global string constant for the function name.
        let fn_name_global = self
            .builder
            .build_global_string_ptr(&fn_name, "spawn_fn_name")
            .map_err(|e| e.to_string())?;
        let fn_name_len = i64_ty.const_int(fn_name.len() as u64, false);

        // args[2..] = actor arguments. Pack into i64 array on GC heap (same as local spawn).
        let spawn_args = &args[2..];
        let (args_ptr, args_size) = if spawn_args.is_empty() {
            (ptr_ty.const_null(), i64_ty.const_int(0, false))
        } else {
            let arg_vals: Vec<BasicValueEnum<'ctx>> = spawn_args
                .iter()
                .map(|a| self.codegen_expr(a))
                .collect::<Result<Vec<_>, _>>()?;

            let total_size = (arg_vals.len() * 8) as u64;
            let gc_alloc_fn = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
            let size_val = i64_ty.const_int(total_size, false);
            let align_val = i64_ty.const_int(8, false);
            let buf_ptr = self
                .builder
                .build_call(gc_alloc_fn, &[size_val.into(), align_val.into()], "spawn_args")
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("mesh_gc_alloc_actor returned void")?
                .into_pointer_value();
            let arr_ty = i64_ty.array_type(arg_vals.len() as u32);

            for (i, val) in arg_vals.iter().enumerate() {
                let int_val = if val.is_int_value() {
                    val.into_int_value()
                } else if val.is_pointer_value() {
                    self.builder
                        .build_ptr_to_int(val.into_pointer_value(), i64_ty, "arg_int")
                        .map_err(|e| e.to_string())?
                } else if val.is_float_value() {
                    self.builder
                        .build_bit_cast(val.into_float_value(), i64_ty, "arg_int")
                        .map_err(|e: inkwell::builder::BuilderError| e.to_string())?
                        .into_int_value()
                } else {
                    i64_ty.const_int(0, false)
                };
                let idx = self.context.i32_type().const_int(i as u64, false);
                let zero = self.context.i32_type().const_int(0, false);
                let element_ptr = unsafe {
                    self.builder
                        .build_gep(arr_ty, buf_ptr, &[zero, idx], "arg_ptr")
                        .map_err(|e| e.to_string())?
                };
                self.builder
                    .build_store(element_ptr, int_val)
                    .map_err(|e| e.to_string())?;
            }

            (buf_ptr, i64_ty.const_int(total_size, false))
        };

        let link_val = i8_ty.const_int(link_flag as u64, false);

        // Call mesh_node_spawn(node_ptr, node_len, fn_name_ptr, fn_name_len, args_ptr, args_size, link_flag)
        let spawn_fn = get_intrinsic(&self.module, "mesh_node_spawn");
        let result = self
            .builder
            .build_call(
                spawn_fn,
                &[
                    node_ptr.into(),
                    node_len.into(),
                    fn_name_global.as_pointer_value().into(),
                    fn_name_len.into(),
                    args_ptr.into(),
                    args_size.into(),
                    link_val.into(),
                ],
                "remote_pid",
            )
            .map_err(|e| e.to_string())?;

        result
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_node_spawn returned void".to_string())
    }

    fn codegen_actor_receive(
        &mut self,
        arms: &[MirMatchArm],
        timeout_ms: Option<&MirExpr>,
        timeout_body: Option<&MirExpr>,
        result_ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();

        // Evaluate timeout: -1 for infinite wait, or the specified value.
        let timeout_val = if let Some(timeout_expr) = timeout_ms {
            self.codegen_expr(timeout_expr)?.into_int_value()
        } else {
            // Infinite wait: -1
            i64_ty.const_int(u64::MAX, true) // -1 as i64
        };

        // Call mesh_actor_receive(timeout_ms) -> ptr (null when timeout fires)
        let receive_fn = get_intrinsic(&self.module, "mesh_actor_receive");
        let msg_ptr = self
            .builder
            .build_call(receive_fn, &[timeout_val.into()], "msg_ptr")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_actor_receive returned void")?
            .into_pointer_value();

        // When timeout_body is present, we need null-check branching:
        //   [mesh_actor_receive] -> [is_null?] -> timeout_bb (null) / msg_bb (non-null) -> recv_merge_bb
        // When timeout_body is None, the runtime waits indefinitely (no null possible).
        if let Some(timeout_expr) = timeout_body {
            let fn_val = self.current_function();
            let result_llvm_ty = self.llvm_type(result_ty);
            let result_alloca = if self.tce_loop_header.is_some() {
                self.build_entry_alloca(result_llvm_ty, "recv_result")?
            } else {
                self.builder
                    .build_alloca(result_llvm_ty, "recv_result")
                    .map_err(|e| e.to_string())?
            };

            let timeout_bb = self.context.append_basic_block(fn_val, "timeout_bb");
            let msg_bb = self.context.append_basic_block(fn_val, "msg_bb");
            let recv_merge_bb = self.context.append_basic_block(fn_val, "recv_merge_bb");

            // Null check: timeout fires when mesh_actor_receive returns null.
            let is_null = self
                .builder
                .build_is_null(msg_ptr, "msg_is_null")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_conditional_branch(is_null, timeout_bb, msg_bb)
                .map_err(|e| e.to_string())?;

            // timeout_bb: execute the timeout body expression.
            self.builder.position_at_end(timeout_bb);
            let timeout_val = self.codegen_expr(timeout_expr)?;
            if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                self.builder
                    .build_store(result_alloca, timeout_val)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_unconditional_branch(recv_merge_bb)
                    .map_err(|e| e.to_string())?;
            }

            // msg_bb: process the received message (existing logic).
            self.builder.position_at_end(msg_bb);
            let msg_val = self.codegen_recv_load_message(msg_ptr, result_ty)?;
            let msg_result = self.codegen_recv_process_arms(arms, msg_val)?;
            if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                self.builder
                    .build_store(result_alloca, msg_result)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_unconditional_branch(recv_merge_bb)
                    .map_err(|e| e.to_string())?;
            }

            // recv_merge_bb: load and return the result.
            self.builder.position_at_end(recv_merge_bb);
            let result = self
                .builder
                .build_load(result_llvm_ty, result_alloca, "recv_val")
                .map_err(|e| e.to_string())?;
            Ok(result)
        } else {
            // No timeout body: infinite wait path (existing behavior, no null possible).
            let msg_val = self.codegen_recv_load_message(msg_ptr, result_ty)?;
            self.codegen_recv_process_arms(arms, msg_val)
        }
    }

    /// Load the message data from the received message pointer.
    /// Message layout: [u64 type_tag (8 bytes), u64 data_len (8 bytes), u8... data]
    fn codegen_recv_load_message(
        &mut self,
        msg_ptr: inkwell::values::PointerValue<'ctx>,
        result_ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Skip the 16-byte header to get to the data.
        let data_ptr = unsafe {
            self.builder
                .build_gep(
                    self.context.i8_type(),
                    msg_ptr,
                    &[i64_ty.const_int(16, false)],
                    "data_ptr",
                )
                .map_err(|e| e.to_string())?
        };

        // Load the message data as the expected type.
        let msg_val: BasicValueEnum<'ctx> = match result_ty {
            MirType::Int => {
                self.builder
                    .build_load(i64_ty, data_ptr, "msg_int")
                    .map_err(|e| e.to_string())?
            }
            MirType::Float => {
                self.builder
                    .build_load(self.context.f64_type(), data_ptr, "msg_float")
                    .map_err(|e| e.to_string())?
            }
            MirType::Bool => {
                self.builder
                    .build_load(self.context.i8_type(), data_ptr, "msg_bool")
                    .map_err(|e| e.to_string())?
            }
            MirType::String => {
                self.builder
                    .build_load(ptr_ty, data_ptr, "msg_string")
                    .map_err(|e| e.to_string())?
            }
            _ => {
                self.builder
                    .build_load(i64_ty, data_ptr, "msg_data")
                    .map_err(|e| e.to_string())?
            }
        };

        Ok(msg_val)
    }

    /// Process receive arms: bind pattern variable and execute arm body.
    fn codegen_recv_process_arms(
        &mut self,
        arms: &[MirMatchArm],
        msg_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Some(arm) = arms.first() {
            // Bind the pattern variable if it's a simple variable pattern.
            match &arm.pattern {
                MirPattern::Var(name, _) => {
                    let alloca = if self.tce_loop_header.is_some() {
                        self.build_entry_alloca(msg_val.get_type(), name)?
                    } else {
                        self.builder
                            .build_alloca(msg_val.get_type(), name)
                            .map_err(|e| e.to_string())?
                    };
                    self.builder
                        .build_store(alloca, msg_val)
                        .map_err(|e| e.to_string())?;
                    self.locals.insert(name.clone(), alloca);
                }
                MirPattern::Wildcard => {
                    // No binding needed.
                }
                MirPattern::Literal(_) => {
                    // Literal patterns in receive: just fall through to body.
                }
                _ => {
                    // For other pattern types (constructor, tuple, etc.), skip binding.
                }
            }

            // Execute the arm body.
            let body_val = self.codegen_expr(&arm.body)?;
            Ok(body_val)
        } else {
            // No arms: return the raw message value.
            Ok(msg_val)
        }
    }

    fn codegen_actor_self(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        // Call mesh_actor_self() -> i64
        let self_fn = get_intrinsic(&self.module, "mesh_actor_self");
        let result = self
            .builder
            .build_call(self_fn, &[], "self_pid")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_actor_self returned void")?;

        Ok(result)
    }

    fn codegen_actor_link(
        &mut self,
        target: &MirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Evaluate the target PID.
        let target_val = self.codegen_expr(target)?.into_int_value();

        // Call mesh_actor_link(target_pid)
        let link_fn = get_intrinsic(&self.module, "mesh_actor_link");
        self.builder
            .build_call(link_fn, &[target_val.into()], "")
            .map_err(|e| e.to_string())?;

        // Link returns Unit.
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    // ── While loop ────────────────────────────────────────────────────

    fn codegen_while(
        &mut self,
        cond: &MirExpr,
        body: &MirExpr,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();

        // Create basic blocks: cond_check, body, merge
        let cond_bb = self.context.append_basic_block(fn_val, "while_cond");
        let body_bb = self.context.append_basic_block(fn_val, "while_body");
        let merge_bb = self.context.append_basic_block(fn_val, "while_merge");

        // Push loop context for break/continue
        self.loop_stack.push((cond_bb, merge_bb));

        // Branch from current block to cond_check
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| e.to_string())?;

        // -- Condition check block --
        self.builder.position_at_end(cond_bb);
        let cond_val = self.codegen_expr(cond)?.into_int_value();
        self.builder
            .build_conditional_branch(cond_val, body_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // -- Body block --
        self.builder.position_at_end(body_bb);
        let _body_val = self.codegen_expr(body)?;

        // After body codegen, if block is NOT terminated (break/continue may have terminated it),
        // emit reduction check and branch back to cond_check (the back-edge).
        if let Some(bb) = self.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                self.emit_reduction_check();
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        // Pop loop context
        self.loop_stack.pop();

        // Position at merge block
        self.builder.position_at_end(merge_bb);

        // While returns Unit
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    // ── For-in range loop ──────────────────────────────────────────────

    fn codegen_for_in_range(
        &mut self,
        var: &str,
        start_expr: &MirExpr,
        end_expr: &MirExpr,
        filter: Option<&MirExpr>,
        body_expr: &MirExpr,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Codegen start and end values.
        let start_val = self.codegen_expr(start_expr)?.into_int_value();
        let end_val = self.codegen_expr(end_expr)?.into_int_value();

        // Compute range length: max(0, end - start).
        let diff = self.builder.build_int_sub(end_val, start_val, "range_diff")
            .map_err(|e| e.to_string())?;
        let zero = i64_ty.const_int(0, false);
        let is_positive = self.builder.build_int_compare(
            IntPredicate::SGT, diff, zero, "is_positive",
        ).map_err(|e| e.to_string())?;
        let range_len = self.builder.build_select(is_positive, diff, zero, "range_len")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Pre-allocate result list builder.
        let list_builder_new = get_intrinsic(&self.module, "mesh_list_builder_new");
        let result_list = self.builder.build_call(list_builder_new, &[range_len.into()], "result_list")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_builder_new returned void".to_string())?
            .into_pointer_value();

        // Alloca to hold the result list pointer (for break to return partial list).
        let result_alloca = self.builder.build_alloca(ptr_ty, "result_alloca")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(result_alloca, result_list)
            .map_err(|e| e.to_string())?;

        // Create alloca for the loop counter.
        let counter = self.builder.build_alloca(i64_ty, var)
            .map_err(|e| e.to_string())?;
        self.builder.build_store(counter, start_val)
            .map_err(|e| e.to_string())?;

        // Create four basic blocks: header, body, latch, merge.
        let header_bb = self.context.append_basic_block(fn_val, "forin_header");
        let body_bb = self.context.append_basic_block(fn_val, "forin_body");
        let latch_bb = self.context.append_basic_block(fn_val, "forin_latch");
        let merge_bb = self.context.append_basic_block(fn_val, "forin_merge");

        // Push loop context: continue -> latch, break -> merge.
        self.loop_stack.push((latch_bb, merge_bb));

        // Branch from current block to header.
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Header block: load counter, compare < end, branch --
        self.builder.position_at_end(header_bb);
        let counter_val = self.builder.build_load(i64_ty, counter, "i")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let cmp = self.builder.build_int_compare(
            IntPredicate::SLT, counter_val, end_val, "forin_cmp",
        ).map_err(|e| e.to_string())?;
        self.builder.build_conditional_branch(cmp, body_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // -- Body block: bind loop variable, codegen body --
        self.builder.position_at_end(body_bb);

        // Save previous local binding for the variable name (if any).
        let old_alloca = self.locals.insert(var.to_string(), counter);
        let old_type = self.local_types.insert(var.to_string(), MirType::Int);

        // If filter present, add conditional branch to skip body+push.
        if let Some(filter_expr) = filter {
            let filter_val = self.codegen_expr(filter_expr)?
                .into_int_value();
            let do_body_bb = self.context.append_basic_block(fn_val, "forin_do_body");
            self.builder.build_conditional_branch(filter_val, do_body_bb, latch_bb)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(do_body_bb);
        }

        // Codegen the body expression.
        let body_val = self.codegen_expr(body_expr)?;

        // After body, if block is not terminated, push body result to result list.
        if let Some(bb) = self.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                let body_ty = body_expr.ty();
                let body_as_i64 = self.convert_to_list_element(body_val, body_ty)?;
                let list_builder_push = get_intrinsic(&self.module, "mesh_list_builder_push");
                let result_loaded = self.builder.build_load(ptr_ty, result_alloca, "res_list")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                self.builder.build_call(list_builder_push, &[result_loaded.into(), body_as_i64.into()], "")
                    .map_err(|e| e.to_string())?;
                self.builder.build_unconditional_branch(latch_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        // -- Latch block: increment counter, reduction check, branch to header --
        self.builder.position_at_end(latch_bb);
        let latch_counter = self.builder.build_load(i64_ty, counter, "i_latch")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let incremented = self.builder.build_int_add(
            latch_counter, i64_ty.const_int(1, false), "i_next",
        ).map_err(|e| e.to_string())?;
        self.builder.build_store(counter, incremented)
            .map_err(|e| e.to_string())?;
        self.emit_reduction_check();
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Cleanup --
        self.loop_stack.pop();

        // Restore previous local binding.
        if let Some(prev) = old_alloca {
            self.locals.insert(var.to_string(), prev);
        } else {
            self.locals.remove(var);
        }
        if let Some(prev) = old_type {
            self.local_types.insert(var.to_string(), prev);
        } else {
            self.local_types.remove(var);
        }

        // Position at merge block.
        self.builder.position_at_end(merge_bb);

        // Return the result list (comprehension semantics).
        let final_result = self.builder.build_load(ptr_ty, result_alloca, "forin_result")
            .map_err(|e| e.to_string())?;
        Ok(final_result)
    }

    fn codegen_break(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let (_, merge_bb) = self
            .loop_stack
            .last()
            .copied()
            .ok_or_else(|| "break outside loop".to_string())?;

        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // Return a dummy Unit value (unreachable code after break)
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    fn codegen_continue(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let (cond_bb, _) = self
            .loop_stack
            .last()
            .copied()
            .ok_or_else(|| "continue outside loop".to_string())?;

        // Continue is also a back-edge -- emit reduction check
        self.emit_reduction_check();

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| e.to_string())?;

        // Return a dummy Unit value (unreachable code after continue)
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    // ── Reduction check ─────────────────────────────────────────────────

    /// Emit a call to mesh_reduction_check() for preemptive scheduling.
    ///
    /// Inserted after function call sites and closure calls to enable
    /// cooperative preemption of actor processes.
    /// Coerce any BasicValueEnum to an i64 IntValue.
    /// - IntValue: zero-extend if narrower than 64 bits, identity if already i64
    /// - PointerValue: ptrtoint
    /// - FloatValue: bitcast f64 to i64
    /// - StructValue: store to alloca, load as i64 (first 8 bytes)
    fn coerce_to_i64(
        &self,
        val: BasicValueEnum<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        match val {
            BasicValueEnum::IntValue(iv) => {
                if iv.get_type().get_bit_width() < 64 {
                    self.builder
                        .build_int_z_extend(iv, i64_ty, "zext_to_i64")
                        .map_err(|e| e.to_string())
                } else {
                    Ok(iv)
                }
            }
            BasicValueEnum::PointerValue(pv) => {
                self.builder
                    .build_ptr_to_int(pv, i64_ty, "ptr_to_i64")
                    .map_err(|e| e.to_string())
            }
            BasicValueEnum::FloatValue(fv) => {
                let alloca = self.builder
                    .build_alloca(self.context.f64_type(), "f64_tmp")
                    .map_err(|e| e.to_string())?;
                self.builder.build_store(alloca, fv).map_err(|e| e.to_string())?;
                Ok(self.builder
                    .build_load(i64_ty, alloca, "f64_to_i64")
                    .map_err(|e| e.to_string())?
                    .into_int_value())
            }
            BasicValueEnum::StructValue(sv) => {
                let alloca = self.builder
                    .build_alloca(sv.get_type(), "struct_tmp")
                    .map_err(|e| e.to_string())?;
                self.builder.build_store(alloca, sv).map_err(|e| e.to_string())?;
                Ok(self.builder
                    .build_load(i64_ty, alloca, "struct_to_i64")
                    .map_err(|e| e.to_string())?
                    .into_int_value())
            }
            _ => Err(format!("Cannot coerce {:?} to i64", val)),
        }
    }

    fn emit_reduction_check(&self) {
        if let Some(check_fn) = self.module.get_function("mesh_reduction_check") {
            // Only emit if the current block is not yet terminated.
            if let Some(bb) = self.builder.get_insert_block() {
                if bb.get_terminator().is_none() {
                    let _ = self.builder.build_call(check_fn, &[], "");
                }
            }
        }
    }

    // ── Supervisor start ──────────────────────────────────────────────

    fn codegen_supervisor_start(
        &mut self,
        name: &str,
        strategy: u8,
        max_restarts: u32,
        max_seconds: u64,
        children: &[MirChildSpec],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Build the binary config buffer for mesh_supervisor_start.
        // Format: strategy(u8) + max_restarts(u32 LE) + max_seconds(u64 LE) +
        //         child_count(u32 LE) + for each child:
        //           id_len(u32 LE) + id_bytes + fn_ptr_placeholder(u64) +
        //           restart_type(u8) + shutdown_ms(u64 LE) + child_type(u8)
        let mut config_bytes: Vec<u8> = Vec::new();

        // Strategy (1 byte)
        config_bytes.push(strategy);

        // Max restarts (4 bytes LE)
        config_bytes.extend_from_slice(&max_restarts.to_le_bytes());

        // Max seconds (8 bytes LE)
        config_bytes.extend_from_slice(&max_seconds.to_le_bytes());

        // Child count (4 bytes LE)
        config_bytes.extend_from_slice(&(children.len() as u32).to_le_bytes());

        // For each child, we need to embed an offset/placeholder for the function pointer.
        // The fn_ptr will be patched at runtime or we can store a function index.
        // For now, store the child spec metadata; the start function is referenced by name.
        let mut fn_ptr_offsets: Vec<(usize, String)> = Vec::new();

        for child in children {
            // id_len (4 bytes LE)
            let id_bytes = child.id.as_bytes();
            config_bytes.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
            // id_bytes
            config_bytes.extend_from_slice(id_bytes);

            // fn_ptr placeholder (8 bytes) -- we'll patch this with a relocation.
            let fn_ptr_offset = config_bytes.len();
            fn_ptr_offsets.push((fn_ptr_offset, child.start_fn.clone()));
            config_bytes.extend_from_slice(&0u64.to_le_bytes()); // placeholder

            // restart_type (1 byte)
            config_bytes.push(child.restart_type);

            // shutdown_ms (8 bytes LE)
            config_bytes.extend_from_slice(&child.shutdown_ms.to_le_bytes());

            // child_type (1 byte)
            config_bytes.push(child.child_type);
        }

        // Create a global constant for the config buffer.
        let config_data = self.context.const_string(&config_bytes, false);
        let config_name = format!(".sup_config_{}", name);
        let config_global = self.module.add_global(config_data.get_type(), None, &config_name);
        config_global.set_initializer(&config_data);
        config_global.set_constant(true);
        config_global.set_unnamed_addr(true);

        // For each child spec, we need to store the function pointer into the config buffer.
        // We do this at runtime by writing the fn_ptr into the config buffer copy on the stack.
        // Actually, since the config is a global constant, we can't patch it.
        // Instead, let's allocate a stack copy and patch fn_ptrs there.
        let config_size = config_bytes.len() as u64;
        let config_size_val = i64_ty.const_int(config_size, false);

        // Allocate stack copy of the config.
        let config_arr_ty = self.context.i8_type().array_type(config_size as u32);
        let config_alloca = self
            .builder
            .build_alloca(config_arr_ty, "sup_config")
            .map_err(|e| e.to_string())?;

        // Memcpy from global to stack.
        let config_global_ptr = config_global.as_pointer_value();
        self.builder
            .build_memcpy(
                config_alloca,
                1,
                config_global_ptr,
                1,
                i64_ty.const_int(config_size, false),
            )
            .map_err(|e| e.to_string())?;

        // Patch function pointers into the stack copy.
        for (offset, fn_name) in &fn_ptr_offsets {
            if fn_name.is_empty() {
                continue;
            }
            // Get the function pointer value.
            let fn_ptr_val = if let Some(fn_val) = self.functions.get(fn_name).copied() {
                fn_val.as_global_value().as_pointer_value()
            } else {
                // Function not found; use null.
                ptr_ty.const_null()
            };

            // Convert fn_ptr to i64.
            let fn_ptr_int = self
                .builder
                .build_ptr_to_int(fn_ptr_val, i64_ty, "fn_ptr_int")
                .map_err(|e| e.to_string())?;

            // GEP to the offset in the config buffer.
            let offset_val = self.context.i32_type().const_int(*offset as u64, false);
            let zero = self.context.i32_type().const_int(0, false);
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(config_arr_ty, config_alloca, &[zero, offset_val], "fn_ptr_slot")
                    .map_err(|e| e.to_string())?
            };

            // Store the fn_ptr as i64 into the config buffer.
            self.builder
                .build_store(elem_ptr, fn_ptr_int)
                .map_err(|e| e.to_string())?;
        }

        // Call mesh_supervisor_start(config_ptr, config_size) -> i64 (PID)
        let sup_start_fn = get_intrinsic(&self.module, "mesh_supervisor_start");
        let pid_val = self
            .builder
            .build_call(
                sup_start_fn,
                &[config_alloca.into(), config_size_val.into()],
                "sup_pid",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_supervisor_start returned void")?;

        Ok(pid_val)
    }

    // ── Panic ────────────────────────────────────────────────────────

    pub(crate) fn codegen_panic(
        &mut self,
        message: &str,
        file: &str,
        line: u32,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let panic_fn = get_intrinsic(&self.module, "mesh_panic");

        // Create global constants for message and file strings
        let msg_val = self.context.const_string(message.as_bytes(), false);
        let msg_global = self.module.add_global(msg_val.get_type(), None, ".panic_msg");
        msg_global.set_initializer(&msg_val);
        msg_global.set_constant(true);

        let file_val = self.context.const_string(file.as_bytes(), false);
        let file_global = self.module.add_global(file_val.get_type(), None, ".panic_file");
        file_global.set_initializer(&file_val);
        file_global.set_constant(true);

        let msg_ptr = msg_global.as_pointer_value();
        let msg_len = self
            .context
            .i64_type()
            .const_int(message.len() as u64, false);
        let file_ptr = file_global.as_pointer_value();
        let file_len = self
            .context
            .i64_type()
            .const_int(file.len() as u64, false);
        let line_val = self.context.i32_type().const_int(line as u64, false);

        self.builder
            .build_call(
                panic_fn,
                &[
                    msg_ptr.into(),
                    msg_len.into(),
                    file_ptr.into(),
                    file_len.into(),
                    line_val.into(),
                ],
                "",
            )
            .map_err(|e| e.to_string())?;

        self.builder
            .build_unreachable()
            .map_err(|e| e.to_string())?;

        // Return a dummy value (unreachable)
        Ok(self.context.i8_type().const_int(0, false).into())
    }

    // ── Service codegen ─────────────────────────────────────────────────

    /// Generate the service loop function body.
    ///
    /// The loop:
    /// 1. Calls mesh_actor_receive(-1) to get a raw message pointer
    /// 2. Extracts type_tag from data offset 0
    /// 3. Extracts caller_pid from data offset 8
    /// 4. Extracts handler args from data offset 16+
    /// 5. Dispatches to the appropriate handler based on type_tag
    /// 6. For call handlers: replies to caller, then tail-calls loop with new state
    /// 7. For cast handlers: tail-calls loop with new state
    pub(crate) fn codegen_service_loop(
        &mut self,
        _loop_fn_name: &str,
        call_handlers: &[(u64, String, usize)],
        cast_handlers: &[(u64, String, usize)],
    ) -> Result<(), String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i8_ty = self.context.i8_type();

        let fn_val = self.current_function();

        // Determine the state LLVM type from the first handler function's first param.
        // All handlers share the same state type (the init function's return type).
        // The state may be a struct type (e.g., RateLimitState, StreamState), a pointer,
        // or i64. We must use the actual LLVM type, not just ptr-vs-i64.
        let first_handler_name = call_handlers.first()
            .map(|(_, name, _)| name.as_str())
            .or_else(|| cast_handlers.first().map(|(_, name, _)| name.as_str()));
        let state_llvm_ty: inkwell::types::BasicTypeEnum<'ctx> = if let Some(name) = first_handler_name {
            if let Some(handler_fn) = self.functions.get(name) {
                let param_types = handler_fn.get_type().get_param_types();
                if !param_types.is_empty() {
                    inkwell::types::BasicTypeEnum::try_from(param_types[0])
                        .unwrap_or(i64_ty.into())
                } else {
                    i64_ty.into()
                }
            } else {
                i64_ty.into()
            }
        } else {
            i64_ty.into()
        };

        // The service loop function receives a ptr to the args buffer.
        // Load the initial state from the args buffer (first 8-byte slot).
        let args_ptr_alloca = *self.locals.get("__args_ptr")
            .ok_or("Missing __args_ptr parameter in service loop")?;
        let args_ptr_val = self.builder
            .build_load(ptr_ty, args_ptr_alloca, "args_ptr_val")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        let init_state = self.builder
            .build_load(state_llvm_ty, args_ptr_val, "init_state")
            .map_err(|e| e.to_string())?;

        // Create a state alloca to hold the mutable state across iterations.
        let state_alloca = self.builder
            .build_alloca(state_llvm_ty, "__state")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(state_alloca, init_state)
            .map_err(|e| e.to_string())?;

        // Create the loop block.
        let loop_bb = self.context.append_basic_block(fn_val, "loop");
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|e| e.to_string())?;
        self.builder.position_at_end(loop_bb);

        // Load the current state from the state alloca.
        let state_val = self.builder
            .build_load(state_llvm_ty, state_alloca, "state")
            .map_err(|e| e.to_string())?;

        // Call mesh_actor_receive(-1) -> ptr (blocks until message arrives).
        let receive_fn = get_intrinsic(&self.module, "mesh_actor_receive");
        let timeout = i64_ty.const_int(u64::MAX, true); // -1
        let msg_ptr = self.builder
            .build_call(receive_fn, &[timeout.into()], "msg_ptr")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_actor_receive returned void")?
            .into_pointer_value();

        // Check for null (shutdown signal). If null, exit the loop.
        let exit_bb = self.context.append_basic_block(fn_val, "exit_loop");
        let continue_bb = self.context.append_basic_block(fn_val, "continue_loop");
        let is_null = self.builder
            .build_is_null(msg_ptr, "msg_is_null")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_conditional_branch(is_null, exit_bb, continue_bb)
            .map_err(|e| e.to_string())?;

        // Exit block: return from the service loop function.
        self.builder.position_at_end(exit_bb);
        self.builder
            .build_return(Some(&self.context.struct_type(&[], false).const_zero()))
            .map_err(|e| e.to_string())?;

        // Continue block: process the message normally.
        self.builder.position_at_end(continue_bb);

        // Message layout after 16-byte header: [u64 type_tag][u64 caller_pid][i64... args]
        // Skip the 16-byte MessageBuffer header.
        let data_ptr = unsafe {
            self.builder
                .build_gep(i8_ty, msg_ptr, &[i64_ty.const_int(16, false)], "data_ptr")
                .map_err(|e| e.to_string())?
        };

        // Extract type_tag (offset 0 from data_ptr).
        let type_tag = self.builder
            .build_load(i64_ty, data_ptr, "type_tag")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Extract caller_pid (offset 8 from data_ptr).
        let caller_ptr = unsafe {
            self.builder
                .build_gep(i8_ty, data_ptr, &[i64_ty.const_int(8, false)], "caller_ptr")
                .map_err(|e| e.to_string())?
        };
        let caller_pid = self.builder
            .build_load(i64_ty, caller_ptr, "caller_pid")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Build dispatch: if/else chain on type_tag.
        let all_handlers: Vec<(u64, &str, usize, bool)> = call_handlers
            .iter()
            .map(|(tag, name, nargs)| (*tag, name.as_str(), *nargs, true))
            .chain(
                cast_handlers
                    .iter()
                    .map(|(tag, name, nargs)| (*tag, name.as_str(), *nargs, false)),
            )
            .collect();

        // Create blocks for each handler + default.
        let default_bb = self.context.append_basic_block(fn_val, "default");

        // Build the switch instruction.
        let _switch = self.builder
            .build_switch(
                type_tag,
                default_bb,
                &all_handlers
                    .iter()
                    .map(|(tag, _, _, _)| {
                        let bb = self.context.append_basic_block(fn_val, &format!("handler_{}", tag));
                        (i64_ty.const_int(*tag, false), bb)
                    })
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| e.to_string())?;

        // Re-collect blocks from the switch (they're in the same order).
        let handler_blocks: Vec<_> = all_handlers
            .iter()
            .enumerate()
            .map(|(i, _)| {
                // The switch cases are added in order; find the corresponding block.
                let block_name = format!("handler_{}", all_handlers[i].0);
                fn_val.get_basic_blocks().into_iter()
                    .find(|bb| bb.get_name().to_str().unwrap_or("") == block_name)
                    .unwrap()
            })
            .collect();

        // Generate code for each handler.
        for (i, (_tag, handler_fn_name, num_args, is_call)) in all_handlers.iter().enumerate() {
            let bb = handler_blocks[i];
            self.builder.position_at_end(bb);

            // Look up handler function to get parameter types for correct arg loading.
            let handler_fn = self.functions.get(*handler_fn_name).copied()
                .ok_or_else(|| format!("Handler function '{}' not found", handler_fn_name))?;
            let handler_param_types = handler_fn.get_type().get_param_types();

            // Extract handler arguments from the message (offset 16 from data_ptr).
            // First arg is state (already loaded), then handler params.
            let mut handler_args: Vec<BasicMetadataValueEnum<'ctx>> = vec![state_val.into()];
            for arg_idx in 0..*num_args {
                let arg_offset = 16 + (arg_idx * 8);
                let arg_ptr = unsafe {
                    self.builder
                        .build_gep(
                            i8_ty,
                            data_ptr,
                            &[i64_ty.const_int(arg_offset as u64, false)],
                            &format!("arg_{}_ptr", arg_idx),
                        )
                        .map_err(|e| e.to_string())?
                };
                // Load arg with the correct type based on handler function param type.
                // Param index is arg_idx + 1 (first param is state).
                let param_idx = arg_idx + 1;
                let load_ty: inkwell::types::BasicTypeEnum<'ctx> = if param_idx < handler_param_types.len() && handler_param_types[param_idx].is_pointer_type() {
                    ptr_ty.into()
                } else {
                    i64_ty.into()
                };
                let arg_val = self.builder
                    .build_load(load_ty, arg_ptr, &format!("arg_{}", arg_idx))
                    .map_err(|e| e.to_string())?;

                // Convert loaded i64 to the expected handler parameter type.
                // The sender (coerce_to_i64) encodes: Bool→zext i1→i64, Float→bitcast f64→i64,
                // Struct→store+load as i64. We reverse those operations here.
                let arg_val = if param_idx < handler_param_types.len() {
                    let expected_meta_ty = handler_param_types[param_idx];
                    if expected_meta_ty.is_int_type() {
                        let int_ty = expected_meta_ty.into_int_type();
                        if int_ty.get_bit_width() < 64 {
                            // Bool (i1) or other narrow int: truncate i64 -> iN
                            self.builder
                                .build_int_truncate(
                                    arg_val.into_int_value(),
                                    int_ty,
                                    &format!("trunc_arg_{}", arg_idx),
                                )
                                .map_err(|e| e.to_string())?
                                .into()
                        } else {
                            arg_val
                        }
                    } else if expected_meta_ty.is_float_type() {
                        // Float: bitcast i64 -> f64 via alloca
                        let alloca = self.builder
                            .build_alloca(self.context.i64_type(), &format!("arg_{}_f64_tmp", arg_idx))
                            .map_err(|e| e.to_string())?;
                        self.builder.build_store(alloca, arg_val).map_err(|e| e.to_string())?;
                        self.builder
                            .build_load(self.context.f64_type(), alloca, &format!("arg_{}_as_f64", arg_idx))
                            .map_err(|e| e.to_string())?
                    } else if expected_meta_ty.is_struct_type() {
                        // Small struct: bitcast i64 -> struct via alloca
                        let expected_ty = inkwell::types::BasicTypeEnum::try_from(expected_meta_ty)
                            .map_err(|_| format!("Cannot convert struct param type for arg {}", arg_idx))?;
                        let alloca = self.builder
                            .build_alloca(self.context.i64_type(), &format!("arg_{}_struct_tmp", arg_idx))
                            .map_err(|e| e.to_string())?;
                        self.builder.build_store(alloca, arg_val).map_err(|e| e.to_string())?;
                        self.builder
                            .build_load(expected_ty, alloca, &format!("arg_{}_as_struct", arg_idx))
                            .map_err(|e| e.to_string())?
                    } else {
                        arg_val
                    }
                } else {
                    arg_val
                };

                handler_args.push(arg_val.into());
            }

            // Call the handler function.
            let handler_result = self.builder
                .build_call(
                    handler_fn,
                    &handler_args,
                    "handler_result",
                )
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("Handler returned void")?;

            let new_state = if *is_call {
                // For call handlers, the result is the body return value.
                // The call handler body returns a tuple (new_state, reply).
                // At the MIR level, we lowered the body as-is. The handler returns
                // the full body result. We need to extract new_state and reply.
                //
                // Convention: call handler returns the body value directly.
                // The body should return a tuple (new_state, reply_value).
                // Since tuples are lowered to runtime allocations, and our handler
                // functions return Int (which is how the body evaluates), we need to
                // handle this carefully.
                //
                // SIMPLIFICATION: For scalar state and reply types (Int), the call handler
                // body returns a Tuple which at the LLVM level is a struct {i64, i64}.
                // But our handler function has return type Int (i64).
                //
                // REALITY CHECK: The handler body returns whatever the Mesh code returns.
                // For a counter service: `(count, count)` returns a tuple.
                // But MIR lowered the handler with return_type: MirType::Int.
                //
                // This means the handler will actually return the tuple evaluation
                // which in our LLVM codegen produces a struct value, but the function
                // signature says i64. This mismatch needs to be resolved.
                //
                // PRAGMATIC FIX: Since all Mesh values can be represented as i64 at
                // the LLVM level (ints, pointers, bools), and tuples are allocated
                // as runtime objects that return a pointer, we can treat the handler
                // return as i64 and interpret it accordingly.
                //
                // For now: treat handler_result as i64.
                // The reply value is the same as handler_result (for simple cases).
                // The new_state is the first element of the tuple.
                //
                // SIMPLEST APPROACH: The handler function returns whatever its body
                // evaluates to. For call handlers that return (new_state, reply),
                // we'll treat the result as the reply value, and the new_state
                // is passed back via a convention (the handler modifies state by
                // returning a new value).
                //
                // ACTUAL DESIGN: In the type checker, call handlers return (state, reply).
                // The handler body expression evaluates to this tuple. At the LLVM level,
                // tuples become runtime allocated {i64, i64} structs. The handler function
                // returns this as a pointer.
                //
                // For now let's use a simple convention: the handler result IS the reply,
                // and the new state is the same as old state (for get_count which is
                // read-only), or we extract from the result.
                //
                // Actually the handler body (as lowered from Mesh source) already computes
                // both new_state and reply. E.g.:
                //   call get_count() |count| :: Int do
                //     (count, count)
                //   end
                // This returns a tuple. The handler function body IS this expression.
                // At LLVM level, (count, count) becomes a heap-allocated tuple ptr.
                //
                // We need to:
                //   1. Extract reply from result[1] (second element)
                //   2. Extract new_state from result[0] (first element)
                //   3. Call mesh_service_reply(caller_pid, &reply, 8)
                //   4. Recurse with new_state
                //
                // Since tuples are represented as runtime pointers, we need to load
                // from the tuple. Mesh tuples use mesh_tuple_first/mesh_tuple_second.
                //
                // HOWEVER: The Mesh tuple (count, count) in the handler body will be
                // lowered by lower_tuple_expr which creates a runtime tuple allocation.
                // The result is a pointer (MirType::Ptr).
                //
                // Our handler function has return_type MirType::Int, but the body
                // returns a tuple pointer. This is already a type mismatch that
                // codegen_expr handles by truncating/coercing.
                //
                // Let me look at how the tuple works at LLVM level...
                // Actually, this complexity means I should use a simpler encoding.
                //
                // NEW APPROACH: Instead of having the handler return a tuple,
                // generate TWO separate calls from the loop:
                //   1. Call a "handler_body" function that takes (state, args) and
                //      returns the raw body result (a tuple ptr)
                //   2. Extract reply via mesh_tuple_second(result)
                //   3. Extract new_state via mesh_tuple_first(result)
                //   4. Reply with the reply value
                //
                // But this requires the handler function to return Ptr (tuple pointer).
                // Let me adjust the handler function return type.
                //
                // The handler returns a heap-allocated tuple pointer.
                // If it's an IntValue (legacy path), cast to ptr. If it's already a ptr, use directly.
                let result_ptr = if handler_result.is_pointer_value() {
                    handler_result.into_pointer_value()
                } else {
                    self.builder
                        .build_int_to_ptr(handler_result.into_int_value(), ptr_ty, "result_ptr")
                        .map_err(|e| e.to_string())?
                };

                // Extract new_state = tuple_first(result_ptr) -> i64
                let tuple_first_fn = get_intrinsic(&self.module, "mesh_tuple_first");
                let new_state_val = self.builder
                    .build_call(tuple_first_fn, &[result_ptr.into()], "new_state")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("mesh_tuple_first returned void")?
                    .into_int_value();

                // Extract reply = tuple_second(result_ptr) -> i64
                let tuple_second_fn = get_intrinsic(&self.module, "mesh_tuple_second");
                let reply_val = self.builder
                    .build_call(tuple_second_fn, &[result_ptr.into()], "reply")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("mesh_tuple_second returned void")?
                    .into_int_value();

                // Send reply to caller: mesh_service_reply(caller_pid, &reply, 8)
                let reply_alloca = self.builder
                    .build_alloca(i64_ty, "reply_buf")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(reply_alloca, reply_val)
                    .map_err(|e| e.to_string())?;
                let reply_size = i64_ty.const_int(8, false);

                let service_reply_fn = get_intrinsic(&self.module, "mesh_service_reply");
                self.builder
                    .build_call(
                        service_reply_fn,
                        &[caller_pid.into(), reply_alloca.into(), reply_size.into()],
                        "",
                    )
                    .map_err(|e| e.to_string())?;

                // Convert new_state_val (i64 from tuple) to the proper state type.
                // Tuples store all values as i64. For pointer/struct state types,
                // the i64 is actually a pointer value (inttoptr). For struct types,
                // the tuple encoding depends on size:
                //   - Small structs (<= 8 bytes): bitcast from i64 (struct bits stored directly)
                //   - Large structs (> 8 bytes): heap-allocated, i64 is pointer-as-int
                if state_llvm_ty.is_pointer_type() {
                    let state_ptr: inkwell::values::PointerValue<'ctx> = self.builder
                        .build_int_to_ptr(new_state_val, ptr_ty, "new_state_ptr")
                        .map_err(|e| e.to_string())?;
                    state_ptr.into()
                } else if state_llvm_ty.is_struct_type() {
                    let target_data = self.target_machine.get_target_data();
                    let struct_size = target_data.get_store_size(&state_llvm_ty.into_struct_type());
                    if struct_size <= 8 {
                        // Small struct (<= 8 bytes): the i64 IS the struct bits (bitcast).
                        // Reinterpret i64 bits as the struct type via alloca + load.
                        let tmp = self.builder
                            .build_alloca(i64_ty, "state_i64_tmp")
                            .map_err(|e| e.to_string())?;
                        self.builder
                            .build_store(tmp, new_state_val)
                            .map_err(|e| e.to_string())?;
                        self.builder
                            .build_load(state_llvm_ty, tmp, "new_state_small_struct")
                            .map_err(|e| e.to_string())?
                    } else {
                        // Large struct (> 8 bytes): the i64 from the tuple is a pointer
                        // to the heap-allocated struct. Convert to pointer, then load.
                        let state_ptr = self.builder
                            .build_int_to_ptr(new_state_val, ptr_ty, "new_state_struct_ptr")
                            .map_err(|e| e.to_string())?;
                        self.builder
                            .build_load(state_llvm_ty, state_ptr, "new_state_struct")
                            .map_err(|e| e.to_string())?
                    }
                } else {
                    let v: inkwell::values::BasicValueEnum<'ctx> = new_state_val.into();
                    v
                }
            } else {
                // For cast handlers, the result IS the new state.
                // The handler already returns the correct type.
                handler_result
            };

            // Update the state alloca and branch back to loop.
            self.builder
                .build_store(state_alloca, new_state)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_unconditional_branch(loop_bb)
                .map_err(|e| e.to_string())?;
        }

        // Default block: just loop again with unchanged state (unknown tag).
        self.builder.position_at_end(default_bb);
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|e| e.to_string())?;

        // The function never returns normally (it loops forever).
        // We already have terminators on all blocks.
        Ok(())
    }

    /// Generate an actor wrapper function body that deserializes arguments from
    /// a raw pointer buffer and calls the actual actor body function.
    ///
    /// The runtime calls actor entry functions with `extern "C" fn(*const u8)`.
    /// The wrapper loads each argument from the args buffer at 8-byte offsets
    /// (matching the serialization in `codegen_actor_spawn`) and calls the
    /// typed body function.
    pub(crate) fn codegen_actor_wrapper(
        &mut self,
        _wrapper_name: &str,
        body_fn_name: &str,
    ) -> Result<(), String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Get the body function (already forward-declared).
        let body_fn = *self
            .functions
            .get(body_fn_name)
            .ok_or_else(|| format!("Actor body function '{}' not found", body_fn_name))?;

        // Load the __args_ptr parameter value.
        let args_ptr_alloca = *self
            .locals
            .get("__args_ptr")
            .ok_or("Missing __args_ptr parameter in actor wrapper")?;
        let args_ptr_val = self
            .builder
            .build_load(ptr_ty, args_ptr_alloca, "args_ptr_val")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        // Get the body function's parameter types from LLVM.
        let body_param_types = body_fn.get_type().get_param_types();
        let num_params = body_param_types.len();

        // Create an array type for GEP indexing (same as codegen_actor_spawn).
        let arr_ty = i64_ty.array_type(num_params as u32);

        // Load each argument from the buffer at 8-byte offsets.
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            Vec::with_capacity(num_params);

        for i in 0..num_params {
            let idx = self.context.i32_type().const_int(i as u64, false);
            let zero = self.context.i32_type().const_int(0, false);
            let element_ptr = unsafe {
                self.builder
                    .build_gep(arr_ty, args_ptr_val, &[zero, idx], &format!("arg_ptr_{}", i))
                    .map_err(|e| e.to_string())?
            };

            // Convert BasicMetadataTypeEnum to BasicTypeEnum for type checking.
            let param_ty: inkwell::types::BasicTypeEnum<'ctx> =
                inkwell::types::BasicTypeEnum::try_from(body_param_types[i])
                    .unwrap_or(i64_ty.into());

            let loaded_val = if param_ty.is_pointer_type() {
                // Load as i64 then inttoptr
                let raw = self
                    .builder
                    .build_load(i64_ty, element_ptr, &format!("arg_raw_{}", i))
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                self.builder
                    .build_int_to_ptr(raw, ptr_ty, &format!("arg_ptr_{}", i))
                    .map_err(|e| e.to_string())?
                    .into()
            } else if param_ty.is_float_type() {
                // Load as i64 then bitcast to float
                let raw = self
                    .builder
                    .build_load(i64_ty, element_ptr, &format!("arg_raw_{}", i))
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_bit_cast(raw, param_ty, &format!("arg_float_{}", i))
                    .map_err(|e| e.to_string())?
            } else {
                // Integer type: load as i64 directly
                self.builder
                    .build_load(i64_ty, element_ptr, &format!("arg_{}", i))
                    .map_err(|e| e.to_string())?
            };

            call_args.push(loaded_val.into());
        }

        // Call the body function with deserialized arguments.
        self.builder
            .build_call(body_fn, &call_args, "body_call")
            .map_err(|e| e.to_string())?;

        // Return Unit (empty struct).
        let unit_val = self.context.struct_type(&[], false).const_zero();
        self.builder
            .build_return(Some(&unit_val))
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Generate a service call helper function body.
    ///
    /// Allocate a runtime tuple on the GC heap.
    /// Layout: { u64 len, u64[N] elements }
    /// Args are the pre-compiled element values.
    fn codegen_make_tuple(
        &mut self,
        elements: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let n = elements.len();
        let total_size = 8 + n * 8; // u64 len + n * u64 elements

        // Allocate via mesh_gc_alloc_actor(size, align)
        let gc_alloc = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
        let size_val = i64_type.const_int(total_size as u64, false);
        let align_val = i64_type.const_int(8, false);
        let tuple_ptr = self.builder
            .build_call(gc_alloc, &[size_val.into(), align_val.into()], "tuple_ptr")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_gc_alloc_actor returned void")?
            .into_pointer_value();

        // Store length at offset 0
        let len_val = i64_type.const_int(n as u64, false);
        self.builder
            .build_store(tuple_ptr, len_val)
            .map_err(|e| e.to_string())?;

        // Store each element at offset 8 + i*8
        for (i, elem) in elements.iter().enumerate() {
            let offset = (8 + i * 8) as u64;
            let base_int = self.builder
                .build_ptr_to_int(tuple_ptr, i64_type, "tuple_base")
                .map_err(|e| format!("{}", e))?;
            let addr_int = self.builder
                .build_int_add(base_int, i64_type.const_int(offset, false), "elem_addr")
                .map_err(|e| format!("{}", e))?;
            let elem_ptr = self.builder
                .build_int_to_ptr(addr_int, ptr_type, "elem_ptr")
                .map_err(|e| format!("{}", e))?;

            // Elements may be i64 or ptr. Convert to i64 for storage.
            let elem_i64 = match *elem {
                BasicMetadataValueEnum::IntValue(iv) => {
                    if iv.get_type().get_bit_width() < 64 {
                        self.builder
                            .build_int_z_extend(iv, i64_type, "zext_elem")
                            .map_err(|e| e.to_string())?
                    } else {
                        iv
                    }
                }
                BasicMetadataValueEnum::PointerValue(pv) => {
                    self.builder
                        .build_ptr_to_int(pv, i64_type, "ptr_to_i64")
                        .map_err(|e| e.to_string())?
                }
                BasicMetadataValueEnum::FloatValue(fv) => {
                    // Bit-cast float to i64 for tuple storage.
                    let fv_alloca = self.builder
                        .build_alloca(self.context.f64_type(), "float_tmp")
                        .map_err(|e| format!("{}", e))?;
                    self.builder
                        .build_store(fv_alloca, fv)
                        .map_err(|e| format!("{}", e))?;
                    self.builder
                        .build_load(i64_type, fv_alloca, "float_to_i64")
                        .map_err(|e| format!("{}", e))?
                        .into_int_value()
                }
                BasicMetadataValueEnum::StructValue(sv) => {
                    let sv_ty = sv.get_type();
                    let target_data = self.target_machine.get_target_data();
                    let struct_size = target_data.get_store_size(&sv_ty);
                    if struct_size <= 8 {
                        // Small struct (e.g., tagged union {i8, ptr}): store as opaque i64 bits.
                        let sv_alloca = self.builder
                            .build_alloca(sv_ty, "struct_tmp")
                            .map_err(|e| format!("{}", e))?;
                        self.builder
                            .build_store(sv_alloca, sv)
                            .map_err(|e| format!("{}", e))?;
                        self.builder
                            .build_load(i64_type, sv_alloca, "struct_to_i64")
                            .map_err(|e| format!("{}", e))?
                            .into_int_value()
                    } else {
                        // Large struct (e.g., service state): heap-allocate and store pointer.
                        // The tuple consumer (service loop) will inttoptr -> load to recover the struct.
                        let size = sv_ty.size_of().unwrap_or(i64_type.const_int(struct_size, false));
                        let align = i64_type.const_int(8, false);
                        let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                            .ok_or("mesh_gc_alloc_actor not found")?;
                        let heap_ptr = self.builder
                            .build_call(gc_alloc, &[size.into(), align.into()], "struct_heap")
                            .map_err(|e| format!("{}", e))?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("mesh_gc_alloc_actor returned void")?
                            .into_pointer_value();
                        self.builder
                            .build_store(heap_ptr, sv)
                            .map_err(|e| format!("{}", e))?;
                        self.builder
                            .build_ptr_to_int(heap_ptr, i64_type, "struct_ptr_to_i64")
                            .map_err(|e| e.to_string())?
                    }
                }
                _ => return Err("Unsupported tuple element type".to_string()),
            };

            self.builder
                .build_store(elem_ptr, elem_i64)
                .map_err(|e| e.to_string())?;
        }

        // Return the pointer (as ptr type, will be cast to i64 by caller if needed)
        Ok(tuple_ptr.into())
    }

    /// Takes MIR args: [pid, tag, ...handler_args]
    /// Packs into a message buffer: [u64 handler_args[0], handler_args[1], ...]
    /// Calls mesh_service_call(pid, tag, payload_ptr, payload_size) -> ptr
    /// Loads the reply from the returned pointer and converts based on expected type.
    fn codegen_service_call_helper(
        &mut self,
        args: &[MirExpr],
        reply_ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i8_ty = self.context.i8_type();

        // First arg is pid, second is tag, rest are handler args.
        let pid_val = self.codegen_expr(&args[0])?.into_int_value();
        let tag_val = self.codegen_expr(&args[1])?.into_int_value();

        // Compile handler args and coerce all values to i64 for the payload buffer.
        // Pointers become ptrtoint, bools become zext, etc.
        let handler_args: Vec<_> = args[2..]
            .iter()
            .map(|a| {
                let v = self.codegen_expr(a)?;
                self.coerce_to_i64(v)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Build payload buffer: [i64 arg0, i64 arg1, ...]
        let payload_size = handler_args.len() * 8;
        let (payload_ptr, payload_size_val) = if handler_args.is_empty() {
            (ptr_ty.const_null(), i64_ty.const_int(0, false))
        } else {
            let arr_ty = i64_ty.array_type(handler_args.len() as u32);
            let buf = self.builder
                .build_alloca(arr_ty, "call_payload")
                .map_err(|e| e.to_string())?;

            for (i, arg) in handler_args.iter().enumerate() {
                let idx = self.context.i32_type().const_int(i as u64, false);
                let zero = self.context.i32_type().const_int(0, false);
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(arr_ty, buf, &[zero, idx], "payload_elem")
                        .map_err(|e| e.to_string())?
                };
                self.builder
                    .build_store(elem_ptr, *arg)
                    .map_err(|e| e.to_string())?;
            }

            (buf, i64_ty.const_int(payload_size as u64, false))
        };

        // Call mesh_service_call(pid, tag, payload_ptr, payload_size) -> ptr
        let service_call_fn = get_intrinsic(&self.module, "mesh_service_call");
        let result_ptr = self.builder
            .build_call(
                service_call_fn,
                &[pid_val.into(), tag_val.into(), payload_ptr.into(), payload_size_val.into()],
                "call_result",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("mesh_service_call returned void")?
            .into_pointer_value();

        // The reply is a raw message pointer. The data after the 16-byte header
        // is the reply value (i64).
        let reply_data_ptr = unsafe {
            self.builder
                .build_gep(i8_ty, result_ptr, &[i64_ty.const_int(16, false)], "reply_data")
                .map_err(|e| e.to_string())?
        };
        let reply_i64 = self.builder
            .build_load(i64_ty, reply_data_ptr, "reply_i64")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // The reply is a tuple-encoded i64. How to interpret it depends on the
        // expected return type. In the service handler's return tuple, each
        // element is stored as i64:
        //   - Small structs (≤ 8 bytes): bitcast to i64 (struct bits stored directly)
        //   - Large structs (> 8 bytes): heap-allocated, i64 is pointer-as-int
        //   - Pointers (String, Ptr): ptrtoint'd to i64
        //   - Scalars (Int, Bool, Float, Pid): i64 IS the value
        //
        // We must reverse this encoding based on the reply type.
        match reply_ty {
            MirType::SumType(name) => {
                let layout = self.lookup_sum_type_layout(name)
                    .ok_or_else(|| format!("Unknown sum type layout '{}' in service call reply", name))?;
                let layout = *layout;
                let target_data = self.target_machine.get_target_data();
                let struct_size = target_data.get_store_size(&layout);
                if struct_size <= 8 {
                    // Small sum type: i64 contains the struct bits (bitcast).
                    let tmp = self.builder
                        .build_alloca(i64_ty, "reply_i64_tmp")
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_store(tmp, reply_i64)
                        .map_err(|e| e.to_string())?;
                    let reply_val = self.builder
                        .build_load(layout, tmp, "reply_small_sum")
                        .map_err(|e| e.to_string())?;
                    Ok(reply_val)
                } else {
                    // Large sum type: i64 is a heap pointer. Load the struct.
                    let reply_ptr = self.builder
                        .build_int_to_ptr(reply_i64, ptr_ty, "reply_ptr")
                        .map_err(|e| e.to_string())?;
                    let reply_val = self.builder
                        .build_load(layout, reply_ptr, "reply_sum")
                        .map_err(|e| e.to_string())?;
                    Ok(reply_val)
                }
            }
            MirType::Struct(name) => {
                let struct_ty = self.struct_types.get(name)
                    .ok_or_else(|| format!("Unknown struct type '{}' in service call reply", name))?;
                let struct_ty = *struct_ty;
                let target_data = self.target_machine.get_target_data();
                let struct_size = target_data.get_store_size(&struct_ty);
                if struct_size <= 8 {
                    let tmp = self.builder
                        .build_alloca(i64_ty, "reply_i64_tmp")
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_store(tmp, reply_i64)
                        .map_err(|e| e.to_string())?;
                    let reply_val = self.builder
                        .build_load(struct_ty, tmp, "reply_small_struct")
                        .map_err(|e| e.to_string())?;
                    Ok(reply_val)
                } else {
                    let reply_ptr = self.builder
                        .build_int_to_ptr(reply_i64, ptr_ty, "reply_ptr")
                        .map_err(|e| e.to_string())?;
                    let reply_val = self.builder
                        .build_load(struct_ty, reply_ptr, "reply_struct")
                        .map_err(|e| e.to_string())?;
                    Ok(reply_val)
                }
            }
            MirType::String | MirType::Ptr => {
                let reply_ptr = self.builder
                    .build_int_to_ptr(reply_i64, ptr_ty, "reply_ptr")
                    .map_err(|e| e.to_string())?;
                Ok(reply_ptr.into())
            }
            MirType::Bool => {
                // Bool is i1 in LLVM. Truncate the i64 reply to i1.
                let bool_val = self.builder
                    .build_int_truncate(reply_i64, self.context.bool_type(), "reply_bool")
                    .map_err(|e| e.to_string())?;
                Ok(bool_val.into())
            }
            MirType::Float => {
                // Float is f64 in LLVM. The i64 reply contains the float bits.
                // Reinterpret via alloca store+load.
                let tmp = self.builder
                    .build_alloca(i64_ty, "reply_float_tmp")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(tmp, reply_i64)
                    .map_err(|e| e.to_string())?;
                let float_val = self.builder
                    .build_load(self.context.f64_type(), tmp, "reply_float")
                    .map_err(|e| e.to_string())?;
                Ok(float_val)
            }
            _ => {
                // Int, Pid, Unit: i64 is the value itself.
                Ok(reply_i64.into())
            }
        }
    }

    /// Generate a service cast helper function body.
    ///
    /// Takes MIR args: [pid, tag, ...handler_args]
    /// Packs into a message buffer: [u64 tag][u64 0 (no caller)][i64 handler_args...]
    /// Calls mesh_actor_send(pid, msg_ptr, msg_size).
    fn codegen_service_cast_helper(
        &mut self,
        args: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();

        // First arg is pid, second is tag, rest are handler args.
        let pid_val = self.codegen_expr(&args[0])?.into_int_value();
        let tag_val = self.codegen_expr(&args[1])?.into_int_value();

        // Compile handler args and coerce all values to i64 for the message buffer.
        let handler_args: Vec<_> = args[2..]
            .iter()
            .map(|a| {
                let v = self.codegen_expr(a)?;
                self.coerce_to_i64(v)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Build message buffer: [u64 type_tag][u64 0 (no caller)][i64 handler_args...]
        let num_elements = 2 + handler_args.len(); // tag + caller_pid + args
        let arr_ty = i64_ty.array_type(num_elements as u32);
        let buf = self.builder
            .build_alloca(arr_ty, "cast_msg")
            .map_err(|e| e.to_string())?;

        // Store type_tag.
        let zero = self.context.i32_type().const_int(0, false);
        let tag_ptr = unsafe {
            self.builder
                .build_gep(arr_ty, buf, &[zero, zero], "tag_slot")
                .map_err(|e| e.to_string())?
        };
        self.builder
            .build_store(tag_ptr, tag_val)
            .map_err(|e| e.to_string())?;

        // Store caller_pid = 0 (fire-and-forget, no reply expected).
        let one = self.context.i32_type().const_int(1, false);
        let caller_ptr = unsafe {
            self.builder
                .build_gep(arr_ty, buf, &[zero, one], "caller_slot")
                .map_err(|e| e.to_string())?
        };
        self.builder
            .build_store(caller_ptr, i64_ty.const_int(0, false))
            .map_err(|e| e.to_string())?;

        // Store handler args.
        for (i, arg) in handler_args.iter().enumerate() {
            let idx = self.context.i32_type().const_int((i + 2) as u64, false);
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(arr_ty, buf, &[zero, idx], "arg_slot")
                    .map_err(|e| e.to_string())?
            };
            self.builder
                .build_store(elem_ptr, *arg)
                .map_err(|e| e.to_string())?;
        }

        let msg_size = i64_ty.const_int((num_elements * 8) as u64, false);

        // Call mesh_actor_send(pid, msg_ptr, msg_size).
        let send_fn = get_intrinsic(&self.module, "mesh_actor_send");
        self.builder
            .build_call(
                send_fn,
                &[pid_val.into(), buf.into(), msg_size.into()],
                "",
            )
            .map_err(|e| e.to_string())?;

        // Cast returns Unit.
        Ok(self.context.struct_type(&[], false).const_zero().into())
    }

    // ── List literal codegen ─────────────────────────────────────────

    fn codegen_list_lit(
        &mut self,
        elements: &[MirExpr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Placeholder: will be fully implemented in Task 2.
        // For now, stack-allocate an i64 array and call mesh_list_from_array.
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let count = elements.len();
        let array_type = i64_type.array_type(count as u32);
        let array_alloca = self.builder.build_alloca(array_type, "list_arr")
            .map_err(|e| e.to_string())?;

        for (i, elem) in elements.iter().enumerate() {
            let val = self.codegen_expr(elem)?;
            // Convert value to i64 for uniform list storage.
            let val_as_i64 = self.convert_to_list_element(val, elem.ty())?;
            let idx = self.context.i32_type().const_int(i as u64, false);
            let zero = self.context.i32_type().const_int(0, false);
            let gep = unsafe {
                self.builder.build_gep(array_type, array_alloca, &[zero, idx], "elem_ptr")
                    .map_err(|e| e.to_string())?
            };
            self.builder.build_store(gep, val_as_i64)
                .map_err(|e| e.to_string())?;
        }

        let array_ptr = self.builder.build_pointer_cast(
            array_alloca, ptr_type, "arr_ptr"
        ).map_err(|e| e.to_string())?;
        let count_val = i64_type.const_int(count as u64, false);

        let from_array_fn = get_intrinsic(&self.module, "mesh_list_from_array");
        let result = self.builder
            .build_call(from_array_fn, &[array_ptr.into(), count_val.into()], "list")
            .map_err(|e| e.to_string())?;

        result.try_as_basic_value().basic()
            .ok_or_else(|| "mesh_list_from_array returned void".to_string())
    }

    /// Convert a value to i64 for uniform list element storage.
    /// Bool (i1) -> zero-extend to i64
    /// Float (f64) -> bitcast to i64
    /// Ptr -> ptrtoint to i64
    /// Int (i64) -> use directly
    fn convert_to_list_element(
        &mut self,
        val: BasicValueEnum<'ctx>,
        mir_ty: &MirType,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        match mir_ty {
            MirType::Bool => {
                let bool_val = val.into_int_value();
                self.builder.build_int_z_extend(bool_val, i64_type, "bool_to_i64")
                    .map_err(|e| e.to_string())
            }
            MirType::Float => {
                let float_val = val.into_float_value();
                let cast_result = self.builder.build_bit_cast(float_val, i64_type, "float_to_i64")
                    .map_err(|e| e.to_string())?;
                Ok(cast_result.into_int_value())
            }
            MirType::String | MirType::Ptr | MirType::Pid(_)
            | MirType::Closure(_, _) | MirType::FnPtr(_, _) => {
                let ptr_val = val.into_pointer_value();
                self.builder.build_ptr_to_int(ptr_val, i64_type, "ptr_to_i64")
                    .map_err(|e| e.to_string())
            }
            MirType::Struct(_) | MirType::SumType(_) => {
                // Struct and SumType values are inline LLVM StructValues.
                // Heap-allocate them via GC so we can store a pointer in the list.
                let struct_val = val.into_struct_value();
                let gc_alloc_fn = get_intrinsic(&self.module, "mesh_gc_alloc_actor");
                let val_ty = struct_val.get_type();
                let size = val_ty.size_of().unwrap_or(i64_type.const_int(8, false));
                let align = i64_type.const_int(8, false);
                let heap_ptr = self.builder
                    .build_call(gc_alloc_fn, &[size.into(), align.into()], "heap_alloc")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value().basic()
                    .ok_or("mesh_gc_alloc_actor returned void")?
                    .into_pointer_value();
                self.builder.build_store(heap_ptr, struct_val)
                    .map_err(|e| e.to_string())?;
                self.builder.build_ptr_to_int(heap_ptr, i64_type, "struct_ptr_to_i64")
                    .map_err(|e| e.to_string())
            }
            MirType::Int => {
                Ok(val.into_int_value())
            }
            MirType::Unit => {
                // Unit values are stored as 0 in lists.
                Ok(self.context.i64_type().const_int(0, false))
            }
            _ => {
                // For any other type, try as int value (best effort).
                Ok(val.into_int_value())
            }
        }
    }

    /// Convert an i64 value from the runtime back to a typed BasicValueEnum.
    /// This is the inverse of `convert_to_list_element`.
    fn convert_from_list_element(
        &mut self,
        val: inkwell::values::IntValue<'ctx>,
        target_ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        match target_ty {
            MirType::Int => Ok(val.into()),
            MirType::Bool => {
                let truncated = self.builder.build_int_truncate(val, self.context.bool_type(), "i64_to_bool")
                    .map_err(|e| e.to_string())?;
                Ok(truncated.into())
            }
            MirType::Float => {
                let f64_type = self.context.f64_type();
                let cast_result = self.builder.build_bit_cast(val, f64_type, "i64_to_float")
                    .map_err(|e| e.to_string())?;
                Ok(cast_result)
            }
            MirType::String | MirType::Ptr | MirType::Struct(_) | MirType::SumType(_)
            | MirType::Pid(_) | MirType::Closure(_, _) | MirType::FnPtr(_, _) => {
                let ptr_val = self.builder.build_int_to_ptr(val, ptr_type, "i64_to_ptr")
                    .map_err(|e| e.to_string())?;
                Ok(ptr_val.into())
            }
            MirType::Unit => {
                Ok(self.context.struct_type(&[], false).const_zero().into())
            }
            _ => {
                // Best effort: return as i64.
                Ok(val.into())
            }
        }
    }

    // ── For-in over List ─────────────────────────────────────────────────

    fn codegen_for_in_list(
        &mut self,
        var: &str,
        collection_expr: &MirExpr,
        filter: Option<&MirExpr>,
        body_expr: &MirExpr,
        elem_ty: &MirType,
        body_ty: &MirType,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Codegen collection expression.
        let collection = self.codegen_expr(collection_expr)?.into_pointer_value();

        // Get length of the list.
        let list_length = get_intrinsic(&self.module, "mesh_list_length");
        let len = self.builder.build_call(list_length, &[collection.into()], "len")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_length returned void".to_string())?
            .into_int_value();

        // Pre-allocate result list builder.
        let list_builder_new = get_intrinsic(&self.module, "mesh_list_builder_new");
        let result_list = self.builder.build_call(list_builder_new, &[len.into()], "result_list")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_builder_new returned void".to_string())?
            .into_pointer_value();

        // Alloca for result list pointer (break returns partial list).
        let result_alloca = self.builder.build_alloca(ptr_ty, "result_alloca")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(result_alloca, result_list)
            .map_err(|e| e.to_string())?;

        // Create counter alloca, store 0.
        let counter = self.builder.build_alloca(i64_ty, "forin_counter")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(counter, i64_ty.const_int(0, false))
            .map_err(|e| e.to_string())?;

        // Four basic blocks.
        let header_bb = self.context.append_basic_block(fn_val, "forin_header");
        let body_bb = self.context.append_basic_block(fn_val, "forin_body");
        let latch_bb = self.context.append_basic_block(fn_val, "forin_latch");
        let merge_bb = self.context.append_basic_block(fn_val, "forin_merge");

        // Push loop context: continue -> latch, break -> merge.
        self.loop_stack.push((latch_bb, merge_bb));

        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Header: load counter, compare < len, branch --
        self.builder.position_at_end(header_bb);
        let counter_val = self.builder.build_load(i64_ty, counter, "idx")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let cmp = self.builder.build_int_compare(
            IntPredicate::SLT, counter_val, len, "forin_cmp",
        ).map_err(|e| e.to_string())?;
        self.builder.build_conditional_branch(cmp, body_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // -- Body: get element, bind loop variable, codegen body, push result --
        self.builder.position_at_end(body_bb);
        let counter_in_body = self.builder.build_load(i64_ty, counter, "idx_body")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Call mesh_list_get(collection, counter) -> u64.
        let list_get = get_intrinsic(&self.module, "mesh_list_get");
        let raw_elem = self.builder.build_call(list_get, &[collection.into(), counter_in_body.into()], "raw_elem")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_get returned void".to_string())?
            .into_int_value();

        // Convert from i64 to typed value.
        let typed_elem = self.convert_from_list_element(raw_elem, elem_ty)?;

        // Create alloca for loop variable.
        let elem_llvm_ty = self.llvm_type(elem_ty);
        let var_alloca = self.builder.build_alloca(elem_llvm_ty, var)
            .map_err(|e| e.to_string())?;
        self.builder.build_store(var_alloca, typed_elem)
            .map_err(|e| e.to_string())?;

        // Save old locals for restoration.
        let old_alloca = self.locals.insert(var.to_string(), var_alloca);
        let old_type = self.local_types.insert(var.to_string(), elem_ty.clone());

        // If filter present, add conditional branch to skip body+push.
        if let Some(filter_expr) = filter {
            let filter_val = self.codegen_expr(filter_expr)?
                .into_int_value();
            let do_body_bb = self.context.append_basic_block(fn_val, "forin_do_body");
            self.builder.build_conditional_branch(filter_val, do_body_bb, latch_bb)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(do_body_bb);
        }

        // Codegen body.
        let body_val = self.codegen_expr(body_expr)?;

        // If not terminated, push body result to result list and branch to latch.
        if let Some(bb) = self.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                let body_as_i64 = self.convert_to_list_element(body_val, body_ty)?;
                let list_builder_push = get_intrinsic(&self.module, "mesh_list_builder_push");
                let result_loaded = self.builder.build_load(ptr_ty, result_alloca, "res_list")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                self.builder.build_call(list_builder_push, &[result_loaded.into(), body_as_i64.into()], "")
                    .map_err(|e| e.to_string())?;
                self.builder.build_unconditional_branch(latch_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        // -- Latch: increment counter, reduction check, branch to header --
        self.builder.position_at_end(latch_bb);
        let latch_counter = self.builder.build_load(i64_ty, counter, "idx_latch")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let incremented = self.builder.build_int_add(
            latch_counter, i64_ty.const_int(1, false), "idx_next",
        ).map_err(|e| e.to_string())?;
        self.builder.build_store(counter, incremented)
            .map_err(|e| e.to_string())?;
        self.emit_reduction_check();
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Cleanup --
        self.loop_stack.pop();

        // Restore old locals.
        if let Some(prev) = old_alloca {
            self.locals.insert(var.to_string(), prev);
        } else {
            self.locals.remove(var);
        }
        if let Some(prev) = old_type {
            self.local_types.insert(var.to_string(), prev);
        } else {
            self.local_types.remove(var);
        }

        // Position at merge, return result list.
        self.builder.position_at_end(merge_bb);
        let final_result = self.builder.build_load(ptr_ty, result_alloca, "forin_result")
            .map_err(|e| e.to_string())?;
        Ok(final_result)
    }

    // ── For-in over Iterator (Iterable/Iterator protocol) ─────────────

    /// Resolve a mangled trait method name to its runtime function name.
    /// For built-in iterator types (ListIterator, MapIterator, etc.), the
    /// mangled name (e.g., "Iterator__next__ListIterator") maps to a C runtime
    /// function (e.g., "mesh_list_iter_next"). For user-defined types, the
    /// mangled name IS the function name (compiled from their impl block).
    fn resolve_iterator_fn(&self, mangled: &str) -> Option<inkwell::values::FunctionValue<'ctx>> {
        // First: try to find as a user-compiled function in the module.
        if let Some(f) = self.module.get_function(mangled) {
            return Some(f);
        }
        // Second: map known built-in iterator mangled names to runtime names.
        let runtime_name = match mangled {
            "Iterator__next__ListIterator" => "mesh_list_iter_next",
            "Iterator__next__MapIterator" => "mesh_map_iter_next",
            "Iterator__next__SetIterator" => "mesh_set_iter_next",
            "Iterator__next__RangeIterator" => "mesh_range_iter_next",
            "Iterable__iter__List" => "mesh_list_iter_new",
            "Iterable__iter__Map" => "mesh_map_iter_new",
            "Iterable__iter__Set" => "mesh_set_iter_new",
            // Phase 78: Adapter iterator next dispatch
            "Iterator__next__MapAdapterIterator" => "mesh_iter_map_next",
            "Iterator__next__FilterAdapterIterator" => "mesh_iter_filter_next",
            "Iterator__next__TakeAdapterIterator" => "mesh_iter_take_next",
            "Iterator__next__SkipAdapterIterator" => "mesh_iter_skip_next",
            "Iterator__next__EnumerateAdapterIterator" => "mesh_iter_enumerate_next",
            "Iterator__next__ZipAdapterIterator" => "mesh_iter_zip_next",
            _ => mangled, // Fall through to intrinsic lookup.
        };
        self.module.get_function(runtime_name)
    }

    fn codegen_for_in_iterator(
        &mut self,
        var: &str,
        iterator_expr: &MirExpr,
        filter: Option<&MirExpr>,
        body_expr: &MirExpr,
        elem_ty: &MirType,
        body_ty: &MirType,
        next_fn: &str,
        iter_fn: &str,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let i64_ty = self.context.i64_type();
        let i8_ty = self.context.i8_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Step 1: Codegen the collection/iterator expression.
        let collection_val = self.codegen_expr(iterator_expr)?;

        // Step 2: If iter_fn is non-empty, call iter() to get the iterator.
        let iter_val = if !iter_fn.is_empty() {
            let iter_func = self.resolve_iterator_fn(iter_fn)
                .unwrap_or_else(|| get_intrinsic(&self.module, iter_fn));
            let result = self.builder.build_call(iter_func, &[collection_val.into()], "iter")
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| format!("{} returned void", iter_fn))?;
            result.into_pointer_value()
        } else {
            // Direct Iterator: the expression IS the iterator.
            collection_val.into_pointer_value()
        };

        // Step 3: Store iterator in alloca.
        let iter_alloca = self.builder.build_alloca(ptr_ty, "iter_alloca")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(iter_alloca, iter_val)
            .map_err(|e| e.to_string())?;

        // Step 4: Pre-allocate result list builder (comprehension semantics).
        let list_builder_new = get_intrinsic(&self.module, "mesh_list_builder_new");
        let result_list = self.builder.build_call(list_builder_new, &[i64_ty.const_int(0, false).into()], "result_list")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_builder_new returned void".to_string())?
            .into_pointer_value();

        let result_alloca = self.builder.build_alloca(ptr_ty, "result_alloca")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(result_alloca, result_list)
            .map_err(|e| e.to_string())?;

        // Step 5: Create basic blocks.
        let header_bb = self.context.append_basic_block(fn_val, "iter_header");
        let body_bb = self.context.append_basic_block(fn_val, "iter_body");
        let latch_bb = self.context.append_basic_block(fn_val, "iter_latch");
        let merge_bb = self.context.append_basic_block(fn_val, "iter_merge");

        // Push loop context for break/continue.
        self.loop_stack.push((latch_bb, merge_bb));

        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // Step 6: Header -- call next(), check Option tag.
        self.builder.position_at_end(header_bb);
        let iter_loaded = self.builder.build_load(ptr_ty, iter_alloca, "iter_loaded")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        // Call Iterator__next__TypeName(iter) or mesh_*_iter_next(iter).
        let next_func = self.resolve_iterator_fn(next_fn)
            .unwrap_or_else(|| get_intrinsic(&self.module, next_fn));
        let next_result = self.builder.build_call(next_func, &[iter_loaded.into()], "next_result")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("{} returned void", next_fn))?
            .into_pointer_value();

        // Option is MeshOption { tag: u8, value: *mut u8 }.
        // tag 0 = Some, tag 1 = None.
        // GEP to tag field (index 0).
        let mesh_option_ty = self.context.struct_type(
            &[i8_ty.into(), ptr_ty.into()],
            false,
        );
        let tag_ptr = self.builder.build_struct_gep(mesh_option_ty, next_result, 0, "tag_ptr")
            .map_err(|e| e.to_string())?;
        let tag_val = self.builder.build_load(i8_ty, tag_ptr, "tag")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Compare tag == 0 (Some).
        let is_some = self.builder.build_int_compare(
            IntPredicate::EQ, tag_val, i8_ty.const_int(0, false), "is_some",
        ).map_err(|e| e.to_string())?;

        self.builder.build_conditional_branch(is_some, body_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // Step 7: Body -- extract element, bind variable, run body, push result.
        self.builder.position_at_end(body_bb);

        // GEP to value field (index 1).
        let value_ptr = self.builder.build_struct_gep(mesh_option_ty, next_result, 1, "value_ptr")
            .map_err(|e| e.to_string())?;
        let raw_value = self.builder.build_load(ptr_ty, value_ptr, "raw_value")
            .map_err(|e| e.to_string())?;

        // Convert from raw pointer to typed element.
        // For Int: ptr -> i64 via ptrtoint. For String/Ptr types: ptr -> ptr (no conversion).
        let typed_elem: BasicValueEnum<'ctx> = match elem_ty {
            MirType::Int => {
                let as_int = self.builder.build_ptr_to_int(raw_value.into_pointer_value(), i64_ty, "as_int")
                    .map_err(|e| e.to_string())?;
                as_int.into()
            }
            MirType::Float => {
                let as_int = self.builder.build_ptr_to_int(raw_value.into_pointer_value(), i64_ty, "as_int_f")
                    .map_err(|e| e.to_string())?;
                self.builder.build_bit_cast(as_int, self.context.f64_type(), "as_float")
                    .map_err(|e| e.to_string())?
            }
            MirType::Bool => {
                let as_int = self.builder.build_ptr_to_int(raw_value.into_pointer_value(), i64_ty, "as_int_b")
                    .map_err(|e| e.to_string())?;
                self.builder.build_int_truncate(as_int, self.context.bool_type(), "as_bool")
                    .map_err(|e| e.to_string())?
                    .into()
            }
            _ => {
                // Ptr types (String, structs, collections): already a pointer.
                raw_value
            }
        };

        // Create alloca for loop variable.
        let elem_llvm_ty = self.llvm_type(elem_ty);
        let var_alloca = self.builder.build_alloca(elem_llvm_ty, var)
            .map_err(|e| e.to_string())?;
        self.builder.build_store(var_alloca, typed_elem)
            .map_err(|e| e.to_string())?;

        // Save old locals.
        let old_alloca = self.locals.insert(var.to_string(), var_alloca);
        let old_type = self.local_types.insert(var.to_string(), elem_ty.clone());

        // Optional filter.
        if let Some(filter_expr) = filter {
            let filter_val = self.codegen_expr(filter_expr)?
                .into_int_value();
            let do_body_bb = self.context.append_basic_block(fn_val, "iter_do_body");
            self.builder.build_conditional_branch(filter_val, do_body_bb, latch_bb)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(do_body_bb);
        }

        // Codegen body.
        let body_val = self.codegen_expr(body_expr)?;

        // Push body result to result list.
        if let Some(bb) = self.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                let body_as_i64 = self.convert_to_list_element(body_val, body_ty)?;
                let list_builder_push = get_intrinsic(&self.module, "mesh_list_builder_push");
                let result_loaded = self.builder.build_load(ptr_ty, result_alloca, "res_list")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                self.builder.build_call(list_builder_push, &[result_loaded.into(), body_as_i64.into()], "")
                    .map_err(|e| e.to_string())?;
                self.builder.build_unconditional_branch(latch_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        // Step 8: Latch -- reduction check, branch back to header.
        self.builder.position_at_end(latch_bb);
        self.emit_reduction_check();
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // Step 9: Cleanup.
        self.loop_stack.pop();

        if let Some(prev) = old_alloca {
            self.locals.insert(var.to_string(), prev);
        } else {
            self.locals.remove(var);
        }
        if let Some(prev) = old_type {
            self.local_types.insert(var.to_string(), prev);
        } else {
            self.local_types.remove(var);
        }

        // Return result list.
        self.builder.position_at_end(merge_bb);
        let final_result = self.builder.build_load(ptr_ty, result_alloca, "iter_result")
            .map_err(|e| e.to_string())?;
        Ok(final_result)
    }

    // ── For-in over Map ──────────────────────────────────────────────────

    fn codegen_for_in_map(
        &mut self,
        key_var: &str,
        val_var: &str,
        collection_expr: &MirExpr,
        filter: Option<&MirExpr>,
        body_expr: &MirExpr,
        key_ty: &MirType,
        val_ty: &MirType,
        body_ty: &MirType,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Codegen collection expression.
        let collection = self.codegen_expr(collection_expr)?.into_pointer_value();

        // Get size of the map.
        let map_size = get_intrinsic(&self.module, "mesh_map_size");
        let len = self.builder.build_call(map_size, &[collection.into()], "map_len")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_map_size returned void".to_string())?
            .into_int_value();

        // Pre-allocate result list builder.
        let list_builder_new = get_intrinsic(&self.module, "mesh_list_builder_new");
        let result_list = self.builder.build_call(list_builder_new, &[len.into()], "result_list")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_builder_new returned void".to_string())?
            .into_pointer_value();

        // Alloca for result list pointer.
        let result_alloca = self.builder.build_alloca(ptr_ty, "result_alloca")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(result_alloca, result_list)
            .map_err(|e| e.to_string())?;

        // Create counter alloca, store 0.
        let counter = self.builder.build_alloca(i64_ty, "forin_counter")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(counter, i64_ty.const_int(0, false))
            .map_err(|e| e.to_string())?;

        // Four basic blocks.
        let header_bb = self.context.append_basic_block(fn_val, "forin_header");
        let body_bb = self.context.append_basic_block(fn_val, "forin_body");
        let latch_bb = self.context.append_basic_block(fn_val, "forin_latch");
        let merge_bb = self.context.append_basic_block(fn_val, "forin_merge");

        self.loop_stack.push((latch_bb, merge_bb));

        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Header --
        self.builder.position_at_end(header_bb);
        let counter_val = self.builder.build_load(i64_ty, counter, "idx")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let cmp = self.builder.build_int_compare(
            IntPredicate::SLT, counter_val, len, "forin_cmp",
        ).map_err(|e| e.to_string())?;
        self.builder.build_conditional_branch(cmp, body_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // -- Body --
        self.builder.position_at_end(body_bb);
        let counter_in_body = self.builder.build_load(i64_ty, counter, "idx_body")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Get key and value for this entry.
        let map_entry_key = get_intrinsic(&self.module, "mesh_map_entry_key");
        let raw_key = self.builder.build_call(map_entry_key, &[collection.into(), counter_in_body.into()], "raw_key")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_map_entry_key returned void".to_string())?
            .into_int_value();

        let map_entry_value = get_intrinsic(&self.module, "mesh_map_entry_value");
        let raw_val = self.builder.build_call(map_entry_value, &[collection.into(), counter_in_body.into()], "raw_val")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_map_entry_value returned void".to_string())?
            .into_int_value();

        // Convert from i64 to typed values.
        let typed_key = self.convert_from_list_element(raw_key, key_ty)?;
        let typed_val = self.convert_from_list_element(raw_val, val_ty)?;

        // Create allocas for key and value variables.
        let key_llvm_ty = self.llvm_type(key_ty);
        let key_alloca = self.builder.build_alloca(key_llvm_ty, key_var)
            .map_err(|e| e.to_string())?;
        self.builder.build_store(key_alloca, typed_key)
            .map_err(|e| e.to_string())?;

        let val_llvm_ty = self.llvm_type(val_ty);
        let val_alloca = self.builder.build_alloca(val_llvm_ty, val_var)
            .map_err(|e| e.to_string())?;
        self.builder.build_store(val_alloca, typed_val)
            .map_err(|e| e.to_string())?;

        // Save old locals.
        let old_key_alloca = self.locals.insert(key_var.to_string(), key_alloca);
        let old_key_type = self.local_types.insert(key_var.to_string(), key_ty.clone());
        let old_val_alloca = self.locals.insert(val_var.to_string(), val_alloca);
        let old_val_type = self.local_types.insert(val_var.to_string(), val_ty.clone());

        // If filter present, add conditional branch to skip body+push.
        if let Some(filter_expr) = filter {
            let filter_val = self.codegen_expr(filter_expr)?
                .into_int_value();
            let do_body_bb = self.context.append_basic_block(fn_val, "forin_do_body");
            self.builder.build_conditional_branch(filter_val, do_body_bb, latch_bb)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(do_body_bb);
        }

        // Codegen body.
        let body_val = self.codegen_expr(body_expr)?;

        // Push body result to result list.
        if let Some(bb) = self.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                let body_as_i64 = self.convert_to_list_element(body_val, body_ty)?;
                let list_builder_push = get_intrinsic(&self.module, "mesh_list_builder_push");
                let result_loaded = self.builder.build_load(ptr_ty, result_alloca, "res_list")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                self.builder.build_call(list_builder_push, &[result_loaded.into(), body_as_i64.into()], "")
                    .map_err(|e| e.to_string())?;
                self.builder.build_unconditional_branch(latch_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        // -- Latch --
        self.builder.position_at_end(latch_bb);
        let latch_counter = self.builder.build_load(i64_ty, counter, "idx_latch")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let incremented = self.builder.build_int_add(
            latch_counter, i64_ty.const_int(1, false), "idx_next",
        ).map_err(|e| e.to_string())?;
        self.builder.build_store(counter, incremented)
            .map_err(|e| e.to_string())?;
        self.emit_reduction_check();
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Cleanup --
        self.loop_stack.pop();

        // Restore old locals for both key and value.
        if let Some(prev) = old_key_alloca {
            self.locals.insert(key_var.to_string(), prev);
        } else {
            self.locals.remove(key_var);
        }
        if let Some(prev) = old_key_type {
            self.local_types.insert(key_var.to_string(), prev);
        } else {
            self.local_types.remove(key_var);
        }
        if let Some(prev) = old_val_alloca {
            self.locals.insert(val_var.to_string(), prev);
        } else {
            self.locals.remove(val_var);
        }
        if let Some(prev) = old_val_type {
            self.local_types.insert(val_var.to_string(), prev);
        } else {
            self.local_types.remove(val_var);
        }

        // Position at merge, return result list.
        self.builder.position_at_end(merge_bb);
        let final_result = self.builder.build_load(ptr_ty, result_alloca, "forin_result")
            .map_err(|e| e.to_string())?;
        Ok(final_result)
    }

    // ── For-in over Set ──────────────────────────────────────────────────

    fn codegen_for_in_set(
        &mut self,
        var: &str,
        collection_expr: &MirExpr,
        filter: Option<&MirExpr>,
        body_expr: &MirExpr,
        elem_ty: &MirType,
        body_ty: &MirType,
        _ty: &MirType,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.current_function();
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        // Codegen collection expression.
        let collection = self.codegen_expr(collection_expr)?.into_pointer_value();

        // Get size of the set.
        let set_size = get_intrinsic(&self.module, "mesh_set_size");
        let len = self.builder.build_call(set_size, &[collection.into()], "set_len")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_set_size returned void".to_string())?
            .into_int_value();

        // Pre-allocate result list builder.
        let list_builder_new = get_intrinsic(&self.module, "mesh_list_builder_new");
        let result_list = self.builder.build_call(list_builder_new, &[len.into()], "result_list")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_list_builder_new returned void".to_string())?
            .into_pointer_value();

        // Alloca for result list pointer.
        let result_alloca = self.builder.build_alloca(ptr_ty, "result_alloca")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(result_alloca, result_list)
            .map_err(|e| e.to_string())?;

        // Create counter alloca, store 0.
        let counter = self.builder.build_alloca(i64_ty, "forin_counter")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(counter, i64_ty.const_int(0, false))
            .map_err(|e| e.to_string())?;

        // Four basic blocks.
        let header_bb = self.context.append_basic_block(fn_val, "forin_header");
        let body_bb = self.context.append_basic_block(fn_val, "forin_body");
        let latch_bb = self.context.append_basic_block(fn_val, "forin_latch");
        let merge_bb = self.context.append_basic_block(fn_val, "forin_merge");

        self.loop_stack.push((latch_bb, merge_bb));

        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Header --
        self.builder.position_at_end(header_bb);
        let counter_val = self.builder.build_load(i64_ty, counter, "idx")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let cmp = self.builder.build_int_compare(
            IntPredicate::SLT, counter_val, len, "forin_cmp",
        ).map_err(|e| e.to_string())?;
        self.builder.build_conditional_branch(cmp, body_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // -- Body --
        self.builder.position_at_end(body_bb);
        let counter_in_body = self.builder.build_load(i64_ty, counter, "idx_body")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // Call mesh_set_element_at(collection, counter) -> u64.
        let set_element_at = get_intrinsic(&self.module, "mesh_set_element_at");
        let raw_elem = self.builder.build_call(set_element_at, &[collection.into(), counter_in_body.into()], "raw_elem")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "mesh_set_element_at returned void".to_string())?
            .into_int_value();

        // Convert from i64 to typed value.
        let typed_elem = self.convert_from_list_element(raw_elem, elem_ty)?;

        // Create alloca for loop variable.
        let elem_llvm_ty = self.llvm_type(elem_ty);
        let var_alloca = self.builder.build_alloca(elem_llvm_ty, var)
            .map_err(|e| e.to_string())?;
        self.builder.build_store(var_alloca, typed_elem)
            .map_err(|e| e.to_string())?;

        // Save old locals.
        let old_alloca = self.locals.insert(var.to_string(), var_alloca);
        let old_type = self.local_types.insert(var.to_string(), elem_ty.clone());

        // If filter present, add conditional branch to skip body+push.
        if let Some(filter_expr) = filter {
            let filter_val = self.codegen_expr(filter_expr)?
                .into_int_value();
            let do_body_bb = self.context.append_basic_block(fn_val, "forin_do_body");
            self.builder.build_conditional_branch(filter_val, do_body_bb, latch_bb)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(do_body_bb);
        }

        // Codegen body.
        let body_val = self.codegen_expr(body_expr)?;

        // Push body result to result list.
        if let Some(bb) = self.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                let body_as_i64 = self.convert_to_list_element(body_val, body_ty)?;
                let list_builder_push = get_intrinsic(&self.module, "mesh_list_builder_push");
                let result_loaded = self.builder.build_load(ptr_ty, result_alloca, "res_list")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                self.builder.build_call(list_builder_push, &[result_loaded.into(), body_as_i64.into()], "")
                    .map_err(|e| e.to_string())?;
                self.builder.build_unconditional_branch(latch_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        // -- Latch --
        self.builder.position_at_end(latch_bb);
        let latch_counter = self.builder.build_load(i64_ty, counter, "idx_latch")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let incremented = self.builder.build_int_add(
            latch_counter, i64_ty.const_int(1, false), "idx_next",
        ).map_err(|e| e.to_string())?;
        self.builder.build_store(counter, incremented)
            .map_err(|e| e.to_string())?;
        self.emit_reduction_check();
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|e| e.to_string())?;

        // -- Cleanup --
        self.loop_stack.pop();

        // Restore old locals.
        if let Some(prev) = old_alloca {
            self.locals.insert(var.to_string(), prev);
        } else {
            self.locals.remove(var);
        }
        if let Some(prev) = old_type {
            self.local_types.insert(var.to_string(), prev);
        } else {
            self.local_types.remove(var);
        }

        // Position at merge, return result list.
        self.builder.position_at_end(merge_bb);
        let final_result = self.builder.build_load(ptr_ty, result_alloca, "forin_result")
            .map_err(|e| e.to_string())?;
        Ok(final_result)
    }
}
