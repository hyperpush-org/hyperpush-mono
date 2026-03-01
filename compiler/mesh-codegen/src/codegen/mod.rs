//! LLVM IR generation from MIR.
//!
//! This module implements the core code generation pass that transforms
//! desugared, monomorphized MIR into LLVM IR using Inkwell 0.8.0.
//!
//! ## Architecture
//!
//! - [`CodeGen`]: Main codegen struct holding LLVM context, module, builder
//! - [`types`]: MirType to LLVM type mapping
//! - [`intrinsics`]: Runtime function declarations
//! - [`expr`]: Expression codegen (implemented in Task 2)
//! - [`pattern`]: Decision tree codegen (implemented in Task 2)

pub mod expr;
pub mod intrinsics;
pub mod pattern;
pub mod types;

use std::path::Path;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::StructType;
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::OptimizationLevel;
use rustc_hash::FxHashMap;

use crate::mir::{MirFunction, MirModule, MirStructDef, MirSumTypeDef, MirType};

use self::types::{create_sum_type_layout, llvm_closure_fn_type, llvm_fn_type, llvm_type};

// ── CodeGen ──────────────────────────────────────────────────────────

/// The main LLVM code generation context.
///
/// Holds the LLVM context, module, builder, and all cached type/function
/// mappings needed during IR generation.
pub struct CodeGen<'ctx> {
    /// The LLVM context (lifetime anchor for all LLVM values).
    pub(crate) context: &'ctx Context,
    /// The LLVM module being built.
    pub(crate) module: Module<'ctx>,
    /// The LLVM IR builder.
    pub(crate) builder: Builder<'ctx>,
    /// The target machine (for optimization and object file emission).
    pub(crate) target_machine: TargetMachine,

    // ── Type caches ──────────────────────────────────────────────────

    /// Cache of named struct types (MIR struct name -> LLVM struct type).
    pub(crate) struct_types: FxHashMap<String, StructType<'ctx>>,
    /// Cache of sum type tagged union layouts (MIR sum type name -> LLVM struct type).
    pub(crate) sum_type_layouts: FxHashMap<String, StructType<'ctx>>,
    /// Cache of sum type definitions for variant lookup.
    pub(crate) sum_type_defs: FxHashMap<String, MirSumTypeDef>,

    // ── Function tracking ────────────────────────────────────────────

    /// Map from MIR function name to LLVM function value.
    pub(crate) functions: FxHashMap<String, FunctionValue<'ctx>>,
    /// The current function being compiled.
    pub(crate) current_fn: Option<FunctionValue<'ctx>>,

    // ── Local variable tracking ──────────────────────────────────────

    /// Local variable allocas in the current function (name -> alloca pointer).
    pub(crate) locals: FxHashMap<String, PointerValue<'ctx>>,
    /// Local variable types (name -> MirType) for loading with correct types.
    pub(crate) local_types: FxHashMap<String, MirType>,

    // ── MIR reference ────────────────────────────────────────────────

    /// The MIR module being compiled (borrowed for arm body lookup).
    pub(crate) mir_functions: Vec<MirFunction>,

    /// MIR struct definitions (for field name -> index lookup).
    pub(crate) mir_struct_defs: FxHashMap<String, Vec<(String, MirType)>>,

    /// Loop context stack for break/continue targets.
    /// Each entry is (cond_bb, merge_bb) for the innermost enclosing while loop.
    pub(crate) loop_stack: Vec<(inkwell::basic_block::BasicBlock<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)>,

    /// Service dispatch tables.
    /// Maps service loop function name -> (call_handlers, cast_handlers).
    /// Each entry: (type_tag, handler_fn_name, num_args).
    pub(crate) service_dispatch: std::collections::HashMap<
        String,
        (Vec<(u64, String, usize)>, Vec<(u64, String, usize)>),
    >,

    /// TCE loop header block for the current tail-recursive function.
    /// Set during compile_function when has_tail_calls is true. NOT on loop_stack
    /// (separate from user while/for loops so break/continue don't interfere).
    pub(crate) tce_loop_header: Option<inkwell::basic_block::BasicBlock<'ctx>>,

    /// Parameter names for the current tail-recursive function, in order.
    /// Used by TailCall codegen to know which allocas to store into.
    pub(crate) tce_param_names: Vec<String>,
}

impl<'ctx> CodeGen<'ctx> {
    /// Create a new CodeGen instance.
    ///
    /// # Arguments
    ///
    /// * `context` - The LLVM context (must outlive the CodeGen)
    /// * `module_name` - Name for the LLVM module
    /// * `opt_level` - Optimization level (0 = none, 2 = default)
    /// * `target_triple` - Optional target triple; None = host default
    ///
    /// # Errors
    ///
    /// Returns an error if target initialization fails or the triple is invalid.
    pub fn new(
        context: &'ctx Context,
        module_name: &str,
        opt_level: u8,
        target_triple: Option<&str>,
    ) -> Result<Self, String> {
        // Initialize target
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to initialize native target: {}", e))?;

        let triple = match target_triple {
            Some(triple_str) => TargetTriple::create(triple_str),
            None => TargetMachine::get_default_triple(),
        };

        let target = Target::from_triple(&triple)
            .map_err(|e| format!("Invalid target triple '{}': {}", triple, e))?;

        let opt = match opt_level {
            0 => OptimizationLevel::None,
            1 => OptimizationLevel::Less,
            _ => OptimizationLevel::Default,
        };

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                opt,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or_else(|| format!("Failed to create target machine for '{}'", triple))?;

        let module = context.create_module(module_name);
        module.set_triple(&triple);
        // Set the data layout so LLVM uses correct alignment for the target.
        // Without this, LLVM defaults to a generic layout where i64 has 4-byte
        // alignment, causing SIGBUS on ARM64 (which requires 8-byte alignment
        // for 8-byte loads/stores).
        module.set_data_layout(&target_machine.get_target_data().get_data_layout());

        let builder = context.create_builder();

        Ok(CodeGen {
            context,
            module,
            builder,
            target_machine,
            struct_types: FxHashMap::default(),
            sum_type_layouts: FxHashMap::default(),
            sum_type_defs: FxHashMap::default(),
            functions: FxHashMap::default(),
            current_fn: None,
            locals: FxHashMap::default(),
            local_types: FxHashMap::default(),
            mir_functions: Vec::new(),
            mir_struct_defs: FxHashMap::default(),
            loop_stack: Vec::new(),
            service_dispatch: std::collections::HashMap::new(),
            tce_loop_header: None,
            tce_param_names: Vec::new(),
        })
    }

    /// Compile a MIR module to LLVM IR.
    ///
    /// This is the main compilation entry point. It:
    /// 1. Declares runtime intrinsics
    /// 2. Creates type layouts for structs and sum types
    /// 3. Forward-declares all functions
    /// 4. Compiles function bodies
    /// 5. Generates a main wrapper (if entry function exists)
    /// 6. Verifies the LLVM module
    pub fn compile(&mut self, mir: &MirModule) -> Result<(), String> {
        // Store MIR functions for arm body lookup during pattern codegen.
        self.mir_functions = mir.functions.clone();

        // Store service dispatch tables for codegen.
        self.service_dispatch = mir.service_dispatch.clone();

        // Step 1: Declare runtime intrinsics.
        intrinsics::declare_intrinsics(&self.module);

        // Step 2: Create type layouts and store MIR struct defs for field lookup.
        //
        // Order matters here due to cross-dependencies:
        //   - Sum type layouts for Option/Result need to know if variant fields are
        //     pointer-sized (to choose { i8, ptr } vs { i8, [N x i8] } layout).
        //     Struct fields are always heap-allocated pointers, so they don't need
        //     the struct size — just the opaque struct type as a placeholder.
        //   - Struct LLVM types need sum type layouts for fields like
        //     `exception :: Option<ExceptionInfo>` to use { i8, ptr } correctly.
        //
        // Solution: three-pass type creation:
        //   Pass A: register opaque struct placeholders (forward declarations)
        //   Pass B: create sum type layouts (can reference opaque struct types)
        //   Pass C: set struct type bodies (can reference sum type layouts)
        for s in &mir.structs {
            self.context.opaque_struct_type(&s.name);
        }
        self.create_sum_type_layouts(&mir.sum_types);
        self.create_struct_types(&mir.structs);
        for s in &mir.structs {
            self.mir_struct_defs
                .insert(s.name.clone(), s.fields.clone());
        }

        // Step 3: Forward-declare all functions.
        self.declare_functions(&mir.functions);

        // Step 4: Compile function bodies.
        for func in &mir.functions {
            self.compile_function(func)?;
        }

        // Step 5: Generate main wrapper if entry function exists.
        if let Some(entry_name) = &mir.entry_function {
            self.generate_main_wrapper(entry_name)?;
        }

        // Step 6: Verify the module.
        self.module
            .verify()
            .map_err(|e| format!("LLVM module verification failed: {}", e))?;

        Ok(())
    }

    /// Run LLVM optimization passes on the module.
    pub fn run_optimization_passes(&self, opt_level: u8) -> Result<(), String> {
        let passes = match opt_level {
            0 => "default<O0>",
            1 => "default<O1>",
            _ => "default<O2>",
        };
        self.module
            .run_passes(passes, &self.target_machine, PassBuilderOptions::create())
            .map_err(|e| format!("Optimization passes failed: {}", e))
    }

    /// Emit the LLVM module as an object file.
    pub fn emit_object(&self, path: &Path) -> Result<(), String> {
        self.target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("Failed to emit object file: {}", e))
    }

    /// Emit the LLVM module as human-readable LLVM IR (.ll file).
    pub fn emit_llvm_ir(&self, path: &Path) -> Result<(), String> {
        self.module.print_to_file(path).map_err(|e| format!("Failed to emit LLVM IR: {}", e))
    }

    /// Get the LLVM IR as a string (for testing).
    pub fn get_llvm_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Consume the CodeGen and return the underlying LLVM module.
    ///
    /// This is used by the REPL JIT engine to create an execution engine
    /// from the compiled module.
    pub fn into_module(self) -> Module<'ctx> {
        self.module
    }

    // ── Type layout creation ─────────────────────────────────────────

    fn create_struct_types(&mut self, structs: &[MirStructDef]) {
        for s in structs {
            let field_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = s
                .fields
                .iter()
                .map(|(_, ty)| llvm_type(self.context, ty, &self.struct_types, &self.sum_type_layouts))
                .collect();
            let struct_ty = self.context.opaque_struct_type(&s.name);
            struct_ty.set_body(&field_types, false);
            self.struct_types.insert(s.name.clone(), struct_ty);
        }
    }

    /// Look up a sum type layout by name, falling back to the base name
    /// for monomorphized types (e.g., `Result_String_String` -> `Result`).
    pub(crate) fn lookup_sum_type_layout(&self, name: &str) -> Option<&StructType<'ctx>> {
        if let Some(layout) = self.sum_type_layouts.get(name) {
            return Some(layout);
        }
        if let Some(base) = name.split('_').next() {
            self.sum_type_layouts.get(base)
        } else {
            None
        }
    }

    /// Look up a sum type definition by name, falling back to the base name
    /// for monomorphized types.
    pub(crate) fn lookup_sum_type_def(&self, name: &str) -> Option<&MirSumTypeDef> {
        if let Some(def) = self.sum_type_defs.get(name) {
            return Some(def);
        }
        if let Some(base) = name.split('_').next() {
            self.sum_type_defs.get(base)
        } else {
            None
        }
    }

    fn create_sum_type_layouts(&mut self, sum_types: &[MirSumTypeDef]) {
        let target_data = self.target_machine.get_target_data();
        for st in sum_types {
            let layout = create_sum_type_layout(
                self.context,
                st,
                &self.struct_types,
                &self.sum_type_layouts,
                &target_data,
            );
            self.sum_type_layouts.insert(st.name.clone(), layout);
            self.sum_type_defs.insert(st.name.clone(), st.clone());
        }
    }

    // ── Function declaration and compilation ─────────────────────────

    fn declare_functions(&mut self, functions: &[MirFunction]) {
        for func in functions {
            let fn_type = if func.is_closure_fn {
                // Closure functions: (env_ptr, params...) -> ret
                // The first param in MIR is __env, skip it for user params
                let user_params: Vec<MirType> = func
                    .params
                    .iter()
                    .skip(1) // skip __env
                    .map(|(_, ty)| ty.clone())
                    .collect();
                llvm_closure_fn_type(
                    self.context,
                    &user_params,
                    &func.return_type,
                    &self.struct_types,
                    &self.sum_type_layouts,
                )
            } else {
                let param_types: Vec<MirType> =
                    func.params.iter().map(|(_, ty)| ty.clone()).collect();
                llvm_fn_type(
                    self.context,
                    &param_types,
                    &func.return_type,
                    &self.struct_types,
                    &self.sum_type_layouts,
                )
            };

            let fn_val = self.module.add_function(&func.name, fn_type, None);
            self.functions.insert(func.name.clone(), fn_val);
        }
    }

    fn compile_function(&mut self, func: &MirFunction) -> Result<(), String> {
        let fn_val = *self
            .functions
            .get(&func.name)
            .ok_or_else(|| format!("Function '{}' not declared", func.name))?;

        self.current_fn = Some(fn_val);
        self.locals.clear();
        self.local_types.clear();

        // Create entry basic block.
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Alloca for each parameter and store incoming values.
        for (i, (name, ty)) in func.params.iter().enumerate() {
            let llvm_ty = llvm_type(
                self.context,
                ty,
                &self.struct_types,
                &self.sum_type_layouts,
            );
            let alloca = self.builder.build_alloca(llvm_ty, name).map_err(|e| e.to_string())?;
            let param_val = fn_val.get_nth_param(i as u32).ok_or_else(|| {
                format!("Missing parameter {} for function '{}'", i, func.name)
            })?;
            self.builder
                .build_store(alloca, param_val)
                .map_err(|e| e.to_string())?;
            self.locals.insert(name.clone(), alloca);
            self.local_types.insert(name.clone(), ty.clone());
        }

        // For closure functions, load captured variables from the __env struct.
        if func.is_closure_fn && !func.captures.is_empty() {
            let env_alloca = self
                .locals
                .get("__env")
                .copied()
                .ok_or("Missing __env parameter in closure function")?;
            let env_ptr = self
                .builder
                .build_load(
                    self.context.ptr_type(inkwell::AddressSpace::default()),
                    env_alloca,
                    "env_ptr",
                )
                .map_err(|e| e.to_string())?
                .into_pointer_value();

            // Build the env struct type from capture types.
            let cap_llvm_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = func
                .captures
                .iter()
                .map(|(_, ty)| {
                    llvm_type(
                        self.context,
                        ty,
                        &self.struct_types,
                        &self.sum_type_layouts,
                    )
                })
                .collect();
            let env_struct_ty = self.context.struct_type(&cap_llvm_types, false);

            // Load each captured variable from the env struct and create a local alloca.
            for (i, (name, ty)) in func.captures.iter().enumerate() {
                let cap_llvm_ty = llvm_type(
                    self.context,
                    ty,
                    &self.struct_types,
                    &self.sum_type_layouts,
                );
                let field_ptr = self
                    .builder
                    .build_struct_gep(env_struct_ty, env_ptr, i as u32, &format!("cap_{}", name))
                    .map_err(|e| e.to_string())?;
                let val = self
                    .builder
                    .build_load(cap_llvm_ty, field_ptr, name)
                    .map_err(|e| e.to_string())?;
                let alloca = self
                    .builder
                    .build_alloca(cap_llvm_ty, name)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(alloca, val)
                    .map_err(|e| e.to_string())?;
                self.locals.insert(name.clone(), alloca);
                self.local_types.insert(name.clone(), ty.clone());
            }
        }

        // TCE: If this function has tail calls, wrap body in a loop.
        // Create a loop header block that the TailCall codegen will branch back to.
        if func.has_tail_calls {
            let tce_loop_bb = self.context.append_basic_block(fn_val, "tce_loop");
            self.builder.build_unconditional_branch(tce_loop_bb).map_err(|e| e.to_string())?;
            self.builder.position_at_end(tce_loop_bb);
            self.tce_loop_header = Some(tce_loop_bb);
            self.tce_param_names = func.params.iter().map(|(name, _)| name.clone()).collect();
        }

        // Check if this is a service loop function that needs special codegen.
        if func.name.starts_with("__service_") && func.name.ends_with("_loop") {
            if let Some(dispatch_info) = self.service_dispatch.get(&func.name).cloned() {
                return self.codegen_service_loop(&func.name, &dispatch_info.0, &dispatch_info.1);
            }
        }

        // Check if this is an actor wrapper function that needs arg deserialization.
        // Actor wrappers have a single __args_ptr param and a matching __actor_{name}_body function.
        {
            let actor_body_name = format!("__actor_{}_body", func.name);
            if func.params.len() == 1
                && func.params[0].0 == "__args_ptr"
                && func.params[0].1 == MirType::Ptr
                && self.functions.contains_key(&actor_body_name)
            {
                return self.codegen_actor_wrapper(&func.name, &actor_body_name);
            }
        }

        // Compile the function body.
        let result = self.codegen_expr(&func.body)?;

        // Build return instruction (if not already terminated).
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            match func.return_type {
                MirType::Unit => {
                    let unit_val = self.context.struct_type(&[], false).const_zero();
                    self.builder
                        .build_return(Some(&unit_val))
                        .map_err(|e| e.to_string())?;
                }
                MirType::Never => {
                    self.builder
                        .build_unreachable()
                        .map_err(|e| e.to_string())?;
                }
                _ => {
                    // Coerce the return value to match the function's declared return type.
                    // This handles mismatches like ptr vs { i8, ptr } (Result type).
                    let fn_ret_ty = fn_val.get_type().get_return_type();
                    let coerced_result = if let Some(expected_ty) = fn_ret_ty {
                        self.coerce_return_value(result, expected_ty)?
                    } else {
                        result
                    };
                    self.builder
                        .build_return(Some(&coerced_result))
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        // Clear TCE state after function compilation.
        self.tce_loop_header = None;
        self.tce_param_names.clear();

        Ok(())
    }

    /// Coerce a return value to match the function's declared return type.
    ///
    /// Handles common mismatches:
    /// - ptr result, struct return type: load struct from pointer
    /// - struct result, ptr return type: heap-alloc + store + return pointer
    /// - struct result, different struct return type: bitcast via alloca
    fn coerce_return_value(
        &self,
        result: inkwell::values::BasicValueEnum<'ctx>,
        expected_ty: inkwell::types::BasicTypeEnum<'ctx>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>, String> {
        use inkwell::values::BasicValueEnum;

        let result_ty = result.get_type();

        // Types already match - no coercion needed
        if result_ty == expected_ty {
            return Ok(result);
        }

        match (result, expected_ty) {
            // ptr -> struct (e.g., ptr -> { i8, ptr } Result type):
            // Load the struct from the pointer
            (BasicValueEnum::PointerValue(pv), expected) if expected.is_struct_type() => {
                self.builder
                    .build_load(expected, pv, "ret_coerce_load")
                    .map_err(|e| e.to_string())
            }

            // struct -> ptr: heap-alloc + store + return pointer
            (BasicValueEnum::StructValue(sv), expected) if expected.is_pointer_type() => {
                let sv_ty = sv.get_type();
                let i64_type = self.context.i64_type();
                let size = sv_ty.size_of().unwrap_or(i64_type.const_int(64, false));
                let align = i64_type.const_int(8, false);
                let gc_alloc = self.module.get_function("mesh_gc_alloc_actor")
                    .ok_or("mesh_gc_alloc_actor not found")?;
                let heap_ptr = self.builder
                    .build_call(gc_alloc, &[size.into(), align.into()], "ret_coerce_heap")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("gc_alloc returned void")?
                    .into_pointer_value();
                self.builder
                    .build_store(heap_ptr, sv)
                    .map_err(|e| e.to_string())?;
                Ok(heap_ptr.into())
            }

            // int -> ptr: inttoptr
            (BasicValueEnum::IntValue(iv), expected) if expected.is_pointer_type() => {
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                let cast = self.builder
                    .build_int_to_ptr(iv, ptr_ty, "ret_coerce_inttoptr")
                    .map_err(|e| e.to_string())?;
                Ok(cast.into())
            }

            // ptr -> int: ptrtoint
            (BasicValueEnum::PointerValue(pv), expected) if expected.is_int_type() => {
                let cast = self.builder
                    .build_ptr_to_int(pv, expected.into_int_type(), "ret_coerce_ptrtoint")
                    .map_err(|e| e.to_string())?;
                Ok(cast.into())
            }

            // Struct -> different struct: bitcast via alloca
            // (e.g., { %ProcessorState, { i8, ptr } } vs expected struct layout)
            (BasicValueEnum::StructValue(sv), expected) if expected.is_struct_type() => {
                let alloca = self.builder
                    .build_alloca(sv.get_type(), "ret_coerce_tmp")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(alloca, sv)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_load(expected, alloca, "ret_coerce_reinterpret")
                    .map_err(|e| e.to_string())
            }

            // No coercion possible - return as-is (LLVM may still error)
            (val, _) => Ok(val),
        }
    }

    /// Generate the C-level `main` function wrapper.
    ///
    /// Creates: `main(argc: i32, argv: ptr) -> i32` that calls
    /// `mesh_rt_init()`, then calls the Mesh entry function, then returns 0.
    fn generate_main_wrapper(&mut self, entry_name: &str) -> Result<(), String> {
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let main_type = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);
        let main_fn = self.module.add_function("main", main_type, None);

        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);

        // Call mesh_rt_init()
        let rt_init = intrinsics::get_intrinsic(&self.module, "mesh_rt_init");
        self.builder
            .build_call(rt_init, &[], "")
            .map_err(|e| e.to_string())?;

        // Call mesh_rt_init_actor(0) -- initialize actor scheduler with default threads
        let rt_init_actor = intrinsics::get_intrinsic(&self.module, "mesh_rt_init_actor");
        let zero = self.context.i32_type().const_int(0, false);
        self.builder
            .build_call(rt_init_actor, &[zero.into()], "")
            .map_err(|e| e.to_string())?;

        // Register all top-level functions for remote spawn (Phase 67).
        // Must happen before the entry function runs because the entry function
        // may call Node.spawn which needs the registry populated.
        let register_fn = intrinsics::get_intrinsic(&self.module, "mesh_register_function");
        for mir_fn in &self.mir_functions {
            // Skip closure/lambda functions (they capture environment pointers,
            // cannot be spawned remotely) and compiler-internal functions.
            if mir_fn.is_closure_fn || mir_fn.name.starts_with("__") {
                continue;
            }

            // Create a global string constant for the function name.
            let name_global = self.builder.build_global_string_ptr(
                &mir_fn.name,
                &format!("fn_reg_{}", mir_fn.name),
            ).map_err(|e| e.to_string())?;

            let name_len = self.context.i64_type().const_int(
                mir_fn.name.len() as u64, false,
            );

            // Get the LLVM function value for this MIR function.
            if let Some(fn_val) = self.functions.get(&mir_fn.name) {
                self.builder.build_call(
                    register_fn,
                    &[
                        name_global.as_pointer_value().into(),
                        name_len.into(),
                        fn_val.as_global_value().as_pointer_value().into(),
                    ],
                    "",
                ).map_err(|e| e.to_string())?;
            }
        }

        // Call the Mesh entry function on the main thread.
        // mesh_main runs synchronously, spawning service/job actors along the way.
        // The runtime handles service calls from the main thread context by using
        // a dedicated main process entry in the process table.
        let mesh_main = self
            .functions
            .get(entry_name)
            .ok_or_else(|| format!("Entry function '{}' not found", entry_name))?;
        self.builder
            .build_call(*mesh_main, &[], "")
            .map_err(|e| e.to_string())?;

        // Run the actor scheduler to process all spawned actors.
        // This blocks until all actors have completed.
        let rt_run_scheduler = intrinsics::get_intrinsic(&self.module, "mesh_rt_run_scheduler");
        self.builder
            .build_call(rt_run_scheduler, &[], "")
            .map_err(|e| e.to_string())?;

        // Return 0
        self.builder
            .build_return(Some(&i32_type.const_int(0, false)))
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ── Helpers ──────────────────────────────────────────────────────

    /// Get the LLVM type for a MIR type, using the cached struct/sum type layouts.
    pub(crate) fn llvm_type(&self, ty: &MirType) -> inkwell::types::BasicTypeEnum<'ctx> {
        llvm_type(
            self.context,
            ty,
            &self.struct_types,
            &self.sum_type_layouts,
        )
    }

    /// Get the current function being compiled.
    pub(crate) fn current_function(&self) -> FunctionValue<'ctx> {
        self.current_fn.expect("No current function during codegen")
    }

    /// Build an alloca in the function's entry block.
    ///
    /// In TCE functions, allocas inside the loop body would grow the stack
    /// on each iteration (since LLVM treats alloca in a loop as dynamic
    /// stack allocation). This helper always places allocas in the entry
    /// block so they are allocated once regardless of how many loop
    /// iterations occur.
    pub(crate) fn build_entry_alloca(
        &self,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let fn_val = self.current_function();
        let entry_bb = fn_val.get_first_basic_block()
            .ok_or("Function has no entry block")?;

        // Save the current insertion point.
        let current_bb = self.builder.get_insert_block();

        // Position at the start of the entry block (before any existing instructions).
        // If the entry block has instructions, position before the first one.
        // Otherwise position at the end.
        if let Some(first_inst) = entry_bb.get_first_instruction() {
            self.builder.position_before(&first_inst);
        } else {
            self.builder.position_at_end(entry_bb);
        }

        let alloca = self.builder.build_alloca(ty, name).map_err(|e| e.to_string())?;

        // Restore the original insertion point.
        if let Some(bb) = current_bb {
            self.builder.position_at_end(bb);
        }

        Ok(alloca)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BinOp, MirExpr, MirLiteral, MirMatchArm, MirModule, MirPattern,
        MirStructDef, MirSumTypeDef, MirType, MirVariantDef, UnaryOp,
    };

    fn empty_mir_module() -> MirModule {
        MirModule {
            functions: vec![],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        }
    }

    fn hello_world_mir() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                name: "mesh_main".to_string(),
                params: vec![],
                return_type: MirType::Unit,
                body: MirExpr::Unit,
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![],
            sum_types: vec![],
            entry_function: Some("mesh_main".to_string()),
            service_dispatch: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_empty_module_verifies() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        let mir = empty_mir_module();
        codegen.compile(&mir).unwrap();
    }

    #[test]
    fn test_native_target_init() {
        let context = Context::create();
        let codegen = CodeGen::new(&context, "test", 0, None);
        assert!(codegen.is_ok(), "Native target should initialize");
    }

    #[test]
    fn test_invalid_triple_error() {
        let context = Context::create();
        let result = CodeGen::new(&context, "test", 0, Some("invalid-triple-xxx"));
        match result {
            Ok(_) => panic!("Invalid triple should return error"),
            Err(err) => {
                assert!(
                    err.contains("Invalid target triple") || err.contains("invalid-triple-xxx"),
                    "Error should mention the invalid triple, got: {}",
                    err
                );
            }
        }
    }

    #[test]
    fn test_hello_world_module_verifies() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        let mir = hello_world_mir();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(ir.contains("define i32 @main"), "Should have main wrapper");
        assert!(ir.contains("mesh_rt_init"), "Should call mesh_rt_init");
    }

    #[test]
    fn test_emit_llvm_ir() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        let mir = empty_mir_module();
        codegen.compile(&mir).unwrap();

        let tmp = std::env::temp_dir().join("mesh_test.ll");
        codegen.emit_llvm_ir(&tmp).unwrap();
        assert!(tmp.exists());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_emit_object_file() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        let mir = empty_mir_module();
        codegen.compile(&mir).unwrap();

        let tmp = std::env::temp_dir().join("mesh_test.o");
        codegen.emit_object(&tmp).unwrap();
        assert!(tmp.exists());
        std::fs::remove_file(&tmp).ok();
    }

    // ── Expression codegen tests ─────────────────────────────────────

    /// Helper: compile a single function body and return the LLVM IR string.
    fn compile_expr_to_ir(body: MirExpr, ret_ty: MirType) -> String {
        let mir = MirModule {
            functions: vec![MirFunction {
                name: "test_fn".to_string(),
                params: vec![],
                return_type: ret_ty,
                body,
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();
        codegen.get_llvm_ir()
    }

    /// Helper: compile a function with parameters (to avoid constant folding).
    fn compile_fn_to_ir(
        params: Vec<(String, MirType)>,
        body: MirExpr,
        ret_ty: MirType,
    ) -> String {
        let mir = MirModule {
            functions: vec![MirFunction {
                name: "test_fn".to_string(),
                params,
                return_type: ret_ty,
                body,
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();
        codegen.get_llvm_ir()
    }

    #[test]
    fn test_int_arithmetic() {
        // Use parameters to avoid constant folding
        let body = MirExpr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(MirExpr::Var("a".to_string(), MirType::Int)),
            rhs: Box::new(MirExpr::Var("b".to_string(), MirType::Int)),
            ty: MirType::Int,
        };
        let ir = compile_fn_to_ir(
            vec![
                ("a".to_string(), MirType::Int),
                ("b".to_string(), MirType::Int),
            ],
            body,
            MirType::Int,
        );
        assert!(ir.contains("add i64"), "Should contain i64 add: {}", ir);
    }

    #[test]
    fn test_int_constant_folding() {
        // Constants should be folded by LLVM
        let body = MirExpr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(MirExpr::IntLit(1, MirType::Int)),
            rhs: Box::new(MirExpr::IntLit(2, MirType::Int)),
            ty: MirType::Int,
        };
        let ir = compile_expr_to_ir(body, MirType::Int);
        assert!(
            ir.contains("ret i64 3"),
            "Constants should be folded to 3: {}",
            ir
        );
    }

    #[test]
    fn test_float_arithmetic() {
        let body = MirExpr::BinOp {
            op: BinOp::Mul,
            lhs: Box::new(MirExpr::Var("a".to_string(), MirType::Float)),
            rhs: Box::new(MirExpr::Var("b".to_string(), MirType::Float)),
            ty: MirType::Float,
        };
        let ir = compile_fn_to_ir(
            vec![
                ("a".to_string(), MirType::Float),
                ("b".to_string(), MirType::Float),
            ],
            body,
            MirType::Float,
        );
        assert!(ir.contains("fmul double"), "Should contain fmul: {}", ir);
    }

    #[test]
    fn test_boolean_not() {
        let body = MirExpr::UnaryOp {
            op: UnaryOp::Not,
            operand: Box::new(MirExpr::Var("b".to_string(), MirType::Bool)),
            ty: MirType::Bool,
        };
        let ir = compile_fn_to_ir(
            vec![("b".to_string(), MirType::Bool)],
            body,
            MirType::Bool,
        );
        assert!(ir.contains("xor i1"), "Should contain xor for not: {}", ir);
    }

    #[test]
    fn test_string_literal_calls_mesh_string_new() {
        let body = MirExpr::Block(
            vec![
                MirExpr::StringLit("hello world".to_string(), MirType::String),
            ],
            MirType::String,
        );
        let ir = compile_expr_to_ir(body, MirType::String);
        assert!(
            ir.contains("mesh_string_new"),
            "Should call mesh_string_new: {}",
            ir
        );
    }

    #[test]
    fn test_if_else_branching() {
        let body = MirExpr::If {
            cond: Box::new(MirExpr::BoolLit(true, MirType::Bool)),
            then_body: Box::new(MirExpr::IntLit(1, MirType::Int)),
            else_body: Box::new(MirExpr::IntLit(2, MirType::Int)),
            ty: MirType::Int,
        };
        let ir = compile_expr_to_ir(body, MirType::Int);
        assert!(ir.contains("br i1"), "Should have conditional branch: {}", ir);
        assert!(ir.contains("then"), "Should have then block: {}", ir);
        assert!(ir.contains("else"), "Should have else block: {}", ir);
        assert!(ir.contains("if_merge"), "Should have merge block: {}", ir);
    }

    #[test]
    fn test_let_binding() {
        let body = MirExpr::Let {
            name: "x".to_string(),
            ty: MirType::Int,
            value: Box::new(MirExpr::IntLit(42, MirType::Int)),
            body: Box::new(MirExpr::Var("x".to_string(), MirType::Int)),
        };
        let ir = compile_expr_to_ir(body, MirType::Int);
        assert!(ir.contains("alloca i64"), "Should alloca for x: {}", ir);
        assert!(ir.contains("store i64 42"), "Should store 42: {}", ir);
    }

    #[test]
    fn test_function_call() {
        // Create a module with an add function and a main that calls it
        let mir = MirModule {
            functions: vec![
                MirFunction {
                    name: "add".to_string(),
                    params: vec![
                        ("a".to_string(), MirType::Int),
                        ("b".to_string(), MirType::Int),
                    ],
                    return_type: MirType::Int,
                    body: MirExpr::BinOp {
                        op: BinOp::Add,
                        lhs: Box::new(MirExpr::Var("a".to_string(), MirType::Int)),
                        rhs: Box::new(MirExpr::Var("b".to_string(), MirType::Int)),
                        ty: MirType::Int,
                    },
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
                MirFunction {
                    name: "mesh_main".to_string(),
                    params: vec![],
                    return_type: MirType::Unit,
                    body: MirExpr::Block(
                        vec![MirExpr::Call {
                            func: Box::new(MirExpr::Var(
                                "add".to_string(),
                                MirType::FnPtr(
                                    vec![MirType::Int, MirType::Int],
                                    Box::new(MirType::Int),
                                ),
                            )),
                            args: vec![
                                MirExpr::IntLit(1, MirType::Int),
                                MirExpr::IntLit(2, MirType::Int),
                            ],
                            ty: MirType::Int,
                        }],
                        MirType::Unit,
                    ),
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
            ],
            structs: vec![],
            sum_types: vec![],
            entry_function: Some("mesh_main".to_string()),
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(ir.contains("define i64 @add"), "Should have add function");
        assert!(ir.contains("call i64 @add"), "Should call add");
    }

    #[test]
    fn test_println_hello_world() {
        // Simulate: println("Hello, world!")
        let body = MirExpr::Block(
            vec![MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_println".to_string(),
                    MirType::FnPtr(vec![MirType::String], Box::new(MirType::Unit)),
                )),
                args: vec![MirExpr::StringLit(
                    "Hello, world!".to_string(),
                    MirType::String,
                )],
                ty: MirType::Unit,
            }],
            MirType::Unit,
        );

        let mir = MirModule {
            functions: vec![MirFunction {
                name: "mesh_main".to_string(),
                params: vec![],
                return_type: MirType::Unit,
                body,
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![],
            sum_types: vec![],
            entry_function: Some("mesh_main".to_string()),
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(ir.contains("mesh_string_new"), "Should call mesh_string_new");
        assert!(ir.contains("mesh_println"), "Should call mesh_println");
        assert!(ir.contains("Hello, world!"), "Should contain string literal");
    }

    #[test]
    fn test_construct_sum_type_variant() {
        let mir = MirModule {
            functions: vec![MirFunction {
                name: "test_fn".to_string(),
                params: vec![],
                return_type: MirType::SumType("Option".to_string()),
                body: MirExpr::ConstructVariant {
                    type_name: "Option".to_string(),
                    variant: "Some".to_string(),
                    fields: vec![MirExpr::IntLit(42, MirType::Int)],
                    ty: MirType::SumType("Option".to_string()),
                },
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![],
            sum_types: vec![MirSumTypeDef {
                name: "Option".to_string(),
                variants: vec![
                    MirVariantDef {
                        name: "Some".to_string(),
                        fields: vec![MirType::Int],
                        tag: 0,
                    },
                    MirVariantDef {
                        name: "None".to_string(),
                        fields: vec![],
                        tag: 1,
                    },
                ],
            }],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(ir.contains("store i8 0"), "Should store tag 0 for Some");
    }

    #[test]
    fn test_match_on_sum_type() {
        let mir = MirModule {
            functions: vec![MirFunction {
                name: "test_fn".to_string(),
                params: vec![],
                return_type: MirType::Int,
                body: MirExpr::Match {
                    scrutinee: Box::new(MirExpr::ConstructVariant {
                        type_name: "Option".to_string(),
                        variant: "Some".to_string(),
                        fields: vec![MirExpr::IntLit(42, MirType::Int)],
                        ty: MirType::SumType("Option".to_string()),
                    }),
                    arms: vec![
                        MirMatchArm {
                            pattern: MirPattern::Constructor {
                                type_name: "Option".to_string(),
                                variant: "Some".to_string(),
                                fields: vec![MirPattern::Var(
                                    "x".to_string(),
                                    MirType::Int,
                                )],
                                bindings: vec![("x".to_string(), MirType::Int)],
                            },
                            guard: None,
                            body: MirExpr::Var("x".to_string(), MirType::Int),
                        },
                        MirMatchArm {
                            pattern: MirPattern::Constructor {
                                type_name: "Option".to_string(),
                                variant: "None".to_string(),
                                fields: vec![],
                                bindings: vec![],
                            },
                            guard: None,
                            body: MirExpr::IntLit(0, MirType::Int),
                        },
                    ],
                    ty: MirType::Int,
                },
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![],
            sum_types: vec![MirSumTypeDef {
                name: "Option".to_string(),
                variants: vec![
                    MirVariantDef {
                        name: "Some".to_string(),
                        fields: vec![MirType::Int],
                        tag: 0,
                    },
                    MirVariantDef {
                        name: "None".to_string(),
                        fields: vec![],
                        tag: 1,
                    },
                ],
            }],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(ir.contains("switch i8"), "Should have switch on tag: {}", ir);
    }

    #[test]
    fn test_closure_creation() {
        let mir = MirModule {
            functions: vec![
                MirFunction {
                    name: "__closure_0".to_string(),
                    params: vec![
                        ("__env".to_string(), MirType::Ptr),
                        ("x".to_string(), MirType::Int),
                    ],
                    return_type: MirType::Int,
                    body: MirExpr::Var("x".to_string(), MirType::Int),
                    is_closure_fn: true,
                    captures: vec![],
                    has_tail_calls: false,
                },
                MirFunction {
                    name: "test_fn".to_string(),
                    params: vec![],
                    return_type: MirType::Closure(
                        vec![MirType::Int],
                        Box::new(MirType::Int),
                    ),
                    body: MirExpr::MakeClosure {
                        fn_name: "__closure_0".to_string(),
                        captures: vec![],
                        ty: MirType::Closure(
                            vec![MirType::Int],
                            Box::new(MirType::Int),
                        ),
                    },
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
            ],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(
            ir.contains("@__closure_0"),
            "Should have closure function: {}",
            ir
        );
        // Closure function takes ptr as first arg
        assert!(
            ir.contains("define { ptr, ptr } @test_fn"),
            "test_fn should return closure struct: {}",
            ir
        );
    }

    #[test]
    fn test_comparison_operators() {
        let body = MirExpr::BinOp {
            op: BinOp::Lt,
            lhs: Box::new(MirExpr::Var("a".to_string(), MirType::Int)),
            rhs: Box::new(MirExpr::Var("b".to_string(), MirType::Int)),
            ty: MirType::Bool,
        };
        let ir = compile_fn_to_ir(
            vec![
                ("a".to_string(), MirType::Int),
                ("b".to_string(), MirType::Int),
            ],
            body,
            MirType::Bool,
        );
        assert!(ir.contains("icmp slt"), "Should contain signed less-than: {}", ir);
    }

    #[test]
    fn test_negation() {
        let body = MirExpr::UnaryOp {
            op: UnaryOp::Neg,
            operand: Box::new(MirExpr::Var("x".to_string(), MirType::Int)),
            ty: MirType::Int,
        };
        let ir = compile_fn_to_ir(
            vec![("x".to_string(), MirType::Int)],
            body,
            MirType::Int,
        );
        assert!(ir.contains("sub i64 0"), "Should contain int negation: {}", ir);
    }

    #[test]
    fn test_actor_spawn_codegen() {
        // Test that ActorSpawn generates a call to mesh_actor_spawn
        let body = MirExpr::ActorSpawn {
            func: Box::new(MirExpr::Var(
                "my_actor".to_string(),
                MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Pid(None))),
            )),
            args: vec![MirExpr::IntLit(42, MirType::Int)],
            priority: 1,
            terminate_callback: None,
            ty: MirType::Pid(None),
        };

        // Need to declare a "my_actor" function in the module
        let mir = MirModule {
            functions: vec![
                MirFunction {
                    name: "my_actor".to_string(),
                    params: vec![("n".to_string(), MirType::Int)],
                    return_type: MirType::Pid(None),
                    body: MirExpr::ActorSelf { ty: MirType::Pid(None) },
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
                MirFunction {
                    name: "test_fn".to_string(),
                    params: vec![],
                    return_type: MirType::Pid(None),
                    body,
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
            ],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(
            ir.contains("mesh_actor_spawn"),
            "Should call mesh_actor_spawn: {}",
            ir
        );
        assert!(
            ir.contains("mesh_actor_self"),
            "Should call mesh_actor_self: {}",
            ir
        );
    }

    #[test]
    fn test_actor_send_codegen() {
        let body = MirExpr::ActorSend {
            target: Box::new(MirExpr::Var("pid".to_string(), MirType::Pid(None))),
            message: Box::new(MirExpr::IntLit(99, MirType::Int)),
            ty: MirType::Unit,
        };
        let ir = compile_fn_to_ir(
            vec![("pid".to_string(), MirType::Pid(None))],
            body,
            MirType::Unit,
        );
        assert!(
            ir.contains("mesh_actor_send"),
            "Should call mesh_actor_send: {}",
            ir
        );
    }

    #[test]
    fn test_actor_receive_codegen() {
        let body = MirExpr::ActorReceive {
            arms: vec![],
            timeout_ms: None,
            timeout_body: None,
            ty: MirType::Int,
        };
        let ir = compile_expr_to_ir(body, MirType::Int);
        assert!(
            ir.contains("mesh_actor_receive"),
            "Should call mesh_actor_receive: {}",
            ir
        );
    }

    #[test]
    fn test_actor_link_codegen() {
        let body = MirExpr::ActorLink {
            target: Box::new(MirExpr::Var("pid".to_string(), MirType::Pid(None))),
            ty: MirType::Unit,
        };
        let ir = compile_fn_to_ir(
            vec![("pid".to_string(), MirType::Pid(None))],
            body,
            MirType::Unit,
        );
        assert!(
            ir.contains("mesh_actor_link"),
            "Should call mesh_actor_link: {}",
            ir
        );
    }

    #[test]
    fn test_main_wrapper_init_actor() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        let mir = hello_world_mir();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(
            ir.contains("mesh_rt_init_actor"),
            "Main should call mesh_rt_init_actor: {}",
            ir
        );
        assert!(
            ir.contains("mesh_rt_run_scheduler"),
            "Main should call mesh_rt_run_scheduler: {}",
            ir
        );
    }

    #[test]
    fn test_reduction_check_after_call() {
        // After a user function call, there should be a mesh_reduction_check
        let mir = MirModule {
            functions: vec![
                MirFunction {
                    name: "helper".to_string(),
                    params: vec![],
                    return_type: MirType::Int,
                    body: MirExpr::IntLit(1, MirType::Int),
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
                MirFunction {
                    name: "test_fn".to_string(),
                    params: vec![],
                    return_type: MirType::Int,
                    body: MirExpr::Call {
                        func: Box::new(MirExpr::Var(
                            "helper".to_string(),
                            MirType::FnPtr(vec![], Box::new(MirType::Int)),
                        )),
                        args: vec![],
                        ty: MirType::Int,
                    },
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
            ],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(
            ir.contains("mesh_reduction_check"),
            "Should insert mesh_reduction_check after call: {}",
            ir
        );
    }

    #[test]
    fn test_pid_is_i64_in_ir() {
        // Verify that Pid type maps to i64 in the generated IR
        let body = MirExpr::ActorSelf { ty: MirType::Pid(None) };
        let ir = compile_expr_to_ir(body, MirType::Pid(None));
        // mesh_actor_self returns i64
        assert!(
            ir.contains("i64 @mesh_actor_self"),
            "mesh_actor_self should return i64: {}",
            ir
        );
    }

    #[test]
    fn test_actor_spawn_with_terminate_callback() {
        // Test that mesh_actor_set_terminate is called after spawn when callback exists
        let body = MirExpr::ActorSpawn {
            func: Box::new(MirExpr::Var(
                "my_actor".to_string(),
                MirType::FnPtr(vec![], Box::new(MirType::Pid(None))),
            )),
            args: vec![],
            priority: 1,
            terminate_callback: Some(Box::new(MirExpr::Var(
                "__terminate_my_actor".to_string(),
                MirType::FnPtr(
                    vec![MirType::Ptr, MirType::Ptr],
                    Box::new(MirType::Unit),
                ),
            ))),
            ty: MirType::Pid(None),
        };

        let mir = MirModule {
            functions: vec![
                MirFunction {
                    name: "my_actor".to_string(),
                    params: vec![],
                    return_type: MirType::Pid(None),
                    body: MirExpr::ActorSelf { ty: MirType::Pid(None) },
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
                MirFunction {
                    name: "__terminate_my_actor".to_string(),
                    params: vec![
                        ("state_ptr".to_string(), MirType::Ptr),
                        ("reason_ptr".to_string(), MirType::Ptr),
                    ],
                    return_type: MirType::Unit,
                    body: MirExpr::Unit,
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
                MirFunction {
                    name: "test_fn".to_string(),
                    params: vec![],
                    return_type: MirType::Pid(None),
                    body,
                    is_closure_fn: false,
                    captures: vec![],
                    has_tail_calls: false,
                },
            ],
            structs: vec![],
            sum_types: vec![],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();
        assert!(
            ir.contains("mesh_actor_spawn"),
            "Should call mesh_actor_spawn: {}",
            ir
        );
        assert!(
            ir.contains("mesh_actor_set_terminate"),
            "Should call mesh_actor_set_terminate: {}",
            ir
        );
    }

    #[test]
    fn test_match_on_integer_literal() {
        let body = MirExpr::Match {
            scrutinee: Box::new(MirExpr::IntLit(1, MirType::Int)),
            arms: vec![
                MirMatchArm {
                    pattern: MirPattern::Literal(MirLiteral::Int(1)),
                    guard: None,
                    body: MirExpr::IntLit(10, MirType::Int),
                },
                MirMatchArm {
                    pattern: MirPattern::Wildcard,
                    guard: None,
                    body: MirExpr::IntLit(0, MirType::Int),
                },
            ],
            ty: MirType::Int,
        };
        let ir = compile_expr_to_ir(body, MirType::Int);
        assert!(
            ir.contains("icmp eq i64"),
            "Should have int comparison for literal match: {}",
            ir
        );
    }

    #[test]
    fn test_while_loop_basic_blocks() {
        // While loop should produce cond/body/merge basic blocks
        let body = MirExpr::While {
            cond: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
            body: Box::new(MirExpr::IntLit(42, MirType::Int)),
            ty: MirType::Unit,
        };
        let ir = compile_expr_to_ir(body, MirType::Unit);
        assert!(ir.contains("while_cond"), "Should have while_cond block: {}", ir);
        assert!(ir.contains("while_body"), "Should have while_body block: {}", ir);
        assert!(ir.contains("while_merge"), "Should have while_merge block: {}", ir);
    }

    #[test]
    fn test_while_loop_reduction_check() {
        // While loop body should emit mesh_reduction_check at back-edge
        let body = MirExpr::While {
            cond: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
            body: Box::new(MirExpr::IntLit(42, MirType::Int)),
            ty: MirType::Unit,
        };
        let ir = compile_expr_to_ir(body, MirType::Unit);
        assert!(
            ir.contains("mesh_reduction_check"),
            "Should emit mesh_reduction_check at loop back-edge: {}",
            ir
        );
    }

    #[test]
    fn test_while_with_break() {
        // While with break should compile and have merge block reachable
        let body = MirExpr::While {
            cond: Box::new(MirExpr::BoolLit(true, MirType::Bool)),
            body: Box::new(MirExpr::Break),
            ty: MirType::Unit,
        };
        let ir = compile_expr_to_ir(body, MirType::Unit);
        assert!(ir.contains("while_merge"), "Should have while_merge block: {}", ir);
        // Break should branch to while_merge
        assert!(ir.contains("br label %while_merge"), "Break should branch to while_merge: {}", ir);
    }

    #[test]
    fn test_while_with_continue() {
        // While with continue should emit reduction check and branch to cond
        let body = MirExpr::While {
            cond: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
            body: Box::new(MirExpr::Continue),
            ty: MirType::Unit,
        };
        let ir = compile_expr_to_ir(body, MirType::Unit);
        // Continue should emit reduction check before branching to cond
        assert!(
            ir.contains("mesh_reduction_check"),
            "Continue should emit mesh_reduction_check: {}",
            ir
        );
    }

    // ── For-in range codegen tests ──────────────────────────────────

    #[test]
    fn test_for_in_range_basic_blocks() {
        // For-in range should produce header/body/latch/merge basic blocks
        let body = MirExpr::ForInRange {
            var: "i".to_string(),
            start: Box::new(MirExpr::IntLit(0, MirType::Int)),
            end: Box::new(MirExpr::IntLit(10, MirType::Int)),
            filter: None,
            body: Box::new(MirExpr::IntLit(1, MirType::Int)),
            ty: MirType::Ptr,
        };
        let ir = compile_expr_to_ir(body, MirType::Ptr);
        assert!(ir.contains("forin_header"), "Should have forin_header block: {}", ir);
        assert!(ir.contains("forin_body"), "Should have forin_body block: {}", ir);
        assert!(ir.contains("forin_latch"), "Should have forin_latch block: {}", ir);
        assert!(ir.contains("forin_merge"), "Should have forin_merge block: {}", ir);
    }

    #[test]
    fn test_for_in_range_slt_comparison() {
        // For-in range should use icmp slt (NOT sle) for half-open range
        let body = MirExpr::ForInRange {
            var: "i".to_string(),
            start: Box::new(MirExpr::IntLit(0, MirType::Int)),
            end: Box::new(MirExpr::IntLit(5, MirType::Int)),
            filter: None,
            body: Box::new(MirExpr::IntLit(1, MirType::Int)),
            ty: MirType::Ptr,
        };
        let ir = compile_expr_to_ir(body, MirType::Ptr);
        assert!(
            ir.contains("icmp slt"),
            "Should use icmp slt for half-open range: {}",
            ir
        );
    }

    #[test]
    fn test_for_in_range_reduction_check_in_latch() {
        // Latch block should contain mesh_reduction_check
        let body = MirExpr::ForInRange {
            var: "i".to_string(),
            start: Box::new(MirExpr::IntLit(0, MirType::Int)),
            end: Box::new(MirExpr::IntLit(10, MirType::Int)),
            filter: None,
            body: Box::new(MirExpr::IntLit(1, MirType::Int)),
            ty: MirType::Ptr,
        };
        let ir = compile_expr_to_ir(body, MirType::Ptr);
        assert!(
            ir.contains("mesh_reduction_check"),
            "Should emit mesh_reduction_check in latch block: {}",
            ir
        );
    }

    #[test]
    fn test_for_in_range_returns_list() {
        // ForInRange now returns a list (Ptr), not Unit
        let body = MirExpr::ForInRange {
            var: "i".to_string(),
            start: Box::new(MirExpr::IntLit(0, MirType::Int)),
            end: Box::new(MirExpr::IntLit(3, MirType::Int)),
            filter: None,
            body: Box::new(MirExpr::Var("i".to_string(), MirType::Int)),
            ty: MirType::Ptr,
        };
        let ir = compile_expr_to_ir(body, MirType::Ptr);
        assert!(
            ir.contains("mesh_list_builder_new"),
            "ForInRange should use list builder: {}",
            ir
        );
        assert!(
            ir.contains("mesh_list_builder_push"),
            "ForInRange should push body results to list: {}",
            ir
        );
        assert!(
            ir.contains("ret ptr"),
            "ForInRange should return ptr (list): {}",
            ir
        );
    }

    #[test]
    fn test_for_in_list_basic_blocks() {
        // ForInList should produce four basic blocks and use list builder
        let body = MirExpr::ForInList {
            var: "x".to_string(),
            collection: Box::new(MirExpr::ListLit {
                elements: vec![],
                ty: MirType::Ptr,
            }),
            filter: None,
            body: Box::new(MirExpr::Var("x".to_string(), MirType::Int)),
            elem_ty: MirType::Int,
            body_ty: MirType::Int,
            ty: MirType::Ptr,
        };
        let ir = compile_expr_to_ir(body, MirType::Ptr);
        assert!(ir.contains("forin_header"), "Should have forin_header block: {}", ir);
        assert!(ir.contains("forin_body"), "Should have forin_body block: {}", ir);
        assert!(ir.contains("forin_latch"), "Should have forin_latch block: {}", ir);
        assert!(ir.contains("forin_merge"), "Should have forin_merge block: {}", ir);
        assert!(
            ir.contains("mesh_list_length"),
            "Should call mesh_list_length: {}",
            ir
        );
        assert!(
            ir.contains("mesh_list_builder_new"),
            "Should use list builder: {}",
            ir
        );
        assert!(
            ir.contains("mesh_list_get"),
            "Should call mesh_list_get: {}",
            ir
        );
    }

    /// Verify that constructing a variant with a multi-field struct payload
    /// generates pointer-boxing code (heap-allocate + store pointer) instead of
    /// directly storing the oversized struct into the {i8, ptr} sum type slot.
    ///
    /// This tests the fix for the ABI segfault where Result<MultiFieldStruct, String>
    /// caused a buffer overflow: the struct (>8 bytes) was written into the ptr slot
    /// (8 bytes) of the {i8, ptr} sum type layout.
    #[test]
    fn test_struct_in_result_pointer_boxing() {
        // Build a MIR module with:
        // - struct TestPair { x: Int, y: Int } (16 bytes, > 8 byte ptr slot)
        // - sum type Result with Ptr fields (like builtin registration)
        // - function that constructs Ok(TestPair { x: 42, y: 99 })
        let mir = MirModule {
            functions: vec![MirFunction {
                name: "make_ok".to_string(),
                params: vec![],
                return_type: MirType::SumType("Result".to_string()),
                body: MirExpr::ConstructVariant {
                    type_name: "Result".to_string(),
                    variant: "Ok".to_string(),
                    fields: vec![MirExpr::StructLit {
                        name: "TestPair".to_string(),
                        fields: vec![
                            ("x".to_string(), MirExpr::IntLit(42, MirType::Int)),
                            ("y".to_string(), MirExpr::IntLit(99, MirType::Int)),
                        ],
                        ty: MirType::Struct("TestPair".to_string()),
                    }],
                    ty: MirType::SumType("Result".to_string()),
                },
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![MirStructDef {
                name: "TestPair".to_string(),
                fields: vec![
                    ("x".to_string(), MirType::Int),
                    ("y".to_string(), MirType::Int),
                ],
            }],
            sum_types: vec![MirSumTypeDef {
                name: "Result".to_string(),
                variants: vec![
                    MirVariantDef {
                        name: "Ok".to_string(),
                        fields: vec![MirType::Ptr],
                        tag: 0,
                    },
                    MirVariantDef {
                        name: "Err".to_string(),
                        fields: vec![MirType::Ptr],
                        tag: 1,
                    },
                ],
            }],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();

        // The IR should contain a call to mesh_gc_alloc_actor for heap-allocating
        // the struct before storing it into the variant's ptr slot.
        assert!(
            ir.contains("variant_box") && ir.contains("mesh_gc_alloc_actor"),
            "Should heap-allocate struct via mesh_gc_alloc_actor (variant_box): {}",
            ir
        );

        // The IR should store the struct into the heap pointer, not into the variant directly.
        assert!(
            ir.contains("store %TestPair") && ir.contains("variant_box"),
            "Should store struct into heap allocation: {}",
            ir
        );

        // The variant's ptr slot should receive the heap pointer, not the struct.
        assert!(
            ir.contains("store ptr %variant_box"),
            "Should store heap pointer into variant ptr slot: {}",
            ir
        );
    }

    /// Verify that constructing a variant with a struct payload AND pattern matching
    /// to extract it generates correct IR with pointer-boxing (construction) and
    /// pointer dereferencing (destructuring).
    #[test]
    fn test_struct_in_result_construct_and_match_ir() {
        // Build a MIR module that constructs Ok(TestPair{42, 99}) and pattern
        // matches to extract pair.x + pair.y. This mirrors the real-world
        // case that caused segfaults before the pointer-boxing fix.
        let mir = MirModule {
            functions: vec![MirFunction {
                name: "roundtrip".to_string(),
                params: vec![],
                return_type: MirType::Int,
                body: MirExpr::Match {
                    scrutinee: Box::new(MirExpr::ConstructVariant {
                        type_name: "Result".to_string(),
                        variant: "Ok".to_string(),
                        fields: vec![MirExpr::StructLit {
                            name: "TestPair".to_string(),
                            fields: vec![
                                ("x".to_string(), MirExpr::IntLit(42, MirType::Int)),
                                ("y".to_string(), MirExpr::IntLit(99, MirType::Int)),
                            ],
                            ty: MirType::Struct("TestPair".to_string()),
                        }],
                        ty: MirType::SumType("Result".to_string()),
                    }),
                    arms: vec![
                        MirMatchArm {
                            pattern: MirPattern::Constructor {
                                type_name: "Result".to_string(),
                                variant: "Ok".to_string(),
                                fields: vec![MirPattern::Var(
                                    "pair".to_string(),
                                    MirType::Struct("TestPair".to_string()),
                                )],
                                bindings: vec![(
                                    "pair".to_string(),
                                    MirType::Struct("TestPair".to_string()),
                                )],
                            },
                            guard: None,
                            body: MirExpr::BinOp {
                                op: BinOp::Add,
                                lhs: Box::new(MirExpr::FieldAccess {
                                    object: Box::new(MirExpr::Var(
                                        "pair".to_string(),
                                        MirType::Struct("TestPair".to_string()),
                                    )),
                                    field: "x".to_string(),
                                    ty: MirType::Int,
                                }),
                                rhs: Box::new(MirExpr::FieldAccess {
                                    object: Box::new(MirExpr::Var(
                                        "pair".to_string(),
                                        MirType::Struct("TestPair".to_string()),
                                    )),
                                    field: "y".to_string(),
                                    ty: MirType::Int,
                                }),
                                ty: MirType::Int,
                            },
                        },
                        MirMatchArm {
                            pattern: MirPattern::Constructor {
                                type_name: "Result".to_string(),
                                variant: "Err".to_string(),
                                fields: vec![MirPattern::Wildcard],
                                bindings: vec![],
                            },
                            guard: None,
                            body: MirExpr::IntLit(0, MirType::Int),
                        },
                    ],
                    ty: MirType::Int,
                },
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            }],
            structs: vec![MirStructDef {
                name: "TestPair".to_string(),
                fields: vec![
                    ("x".to_string(), MirType::Int),
                    ("y".to_string(), MirType::Int),
                ],
            }],
            sum_types: vec![MirSumTypeDef {
                name: "Result".to_string(),
                variants: vec![
                    MirVariantDef {
                        name: "Ok".to_string(),
                        fields: vec![MirType::Ptr],
                        tag: 0,
                    },
                    MirVariantDef {
                        name: "Err".to_string(),
                        fields: vec![MirType::Ptr],
                        tag: 1,
                    },
                ],
            }],
            entry_function: None,
            service_dispatch: std::collections::HashMap::new(),
        };

        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test", 0, None).unwrap();
        codegen.compile(&mir).unwrap();

        let ir = codegen.get_llvm_ir();

        // Construction: should heap-allocate the struct payload.
        assert!(
            ir.contains("variant_box"),
            "Construction should heap-allocate struct (variant_box): {}",
            ir
        );

        // Destructuring: should load through the heap pointer.
        // The pattern match binding loads the ptr from the variant slot, then
        // dereferences it to get the actual struct (deref_struct).
        assert!(
            ir.contains("deref_struct"),
            "Destructuring should dereference heap pointer (deref_struct): {}",
            ir
        );

        // Should have switch on tag byte for pattern matching.
        assert!(
            ir.contains("switch i8"),
            "Should switch on variant tag: {}",
            ir
        );

        // Should have add instruction for pair.x + pair.y.
        assert!(
            ir.contains("add i64"),
            "Should add pair.x + pair.y: {}",
            ir
        );
    }
}
