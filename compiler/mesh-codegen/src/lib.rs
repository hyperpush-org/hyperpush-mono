//! LLVM code generation for the Mesh compiler.
//!
//! This crate transforms a typed Mesh program (represented by the parser's
//! `Parse` and the type checker's `TypeckResult`) into native machine code
//! via LLVM, using the Inkwell safe bindings.
//!
//! ## Architecture
//!
//! - [`mir`]: Mid-level IR definitions and lowering from typed AST
//! - [`pattern`]: Pattern match compilation to decision trees
//! - [`codegen`]: LLVM IR generation from MIR
//!
//! ## Pipeline
//!
//! ```text
//! Parse + TypeckResult -> MIR -> DecisionTree -> LLVM IR -> Object file -> Native binary
//! ```

pub mod codegen;
pub mod declared;
pub mod link;
pub mod mir;
pub mod pattern;

pub use declared::{
    declared_route_wrapper_name, prepare_clustered_route_handler_plan,
    prepare_declared_runtime_handlers, prepare_startup_work_registrations, DeclaredHandlerKind,
    DeclaredHandlerPlanEntry, DeclaredRuntimeRegistration, StartupWorkRegistration,
};

use std::collections::{HashMap, HashSet};
use std::path::Path;

use inkwell::context::Context;

use codegen::CodeGen;
use mir::lower::lower_to_mir;
use mir::mono::monomorphize;

pub(crate) mod build_trace {
    use std::path::{Path, PathBuf};

    use serde_json::{json, Map, Value};

    const TRACE_ENV: &str = "MESH_BUILD_TRACE_PATH";

    fn trace_path() -> Option<PathBuf> {
        std::env::var_os(TRACE_ENV).map(PathBuf::from)
    }

    fn read_trace(path: &Path) -> Map<String, Value> {
        let Some(raw) = std::fs::read_to_string(path).ok() else {
            return Map::new();
        };
        let Some(value) = serde_json::from_str::<Value>(&raw).ok() else {
            return Map::new();
        };
        value.as_object().cloned().unwrap_or_default()
    }

    fn write_trace(path: &Path, doc: &Map<String, Value>) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(rendered) = serde_json::to_string_pretty(doc) else {
            return;
        };
        let _ = std::fs::write(path, rendered);
    }

    pub(crate) fn update<F>(mutate: F)
    where
        F: FnOnce(&mut Map<String, Value>),
    {
        let Some(path) = trace_path() else {
            return;
        };
        let mut doc = read_trace(&path);
        mutate(&mut doc);
        write_trace(&path, &doc);
    }

    pub(crate) fn set_compile_context(output: &Path, object: &Path, target_triple: Option<&str>) {
        update(|doc| {
            doc.insert(
                "buildOutputPath".to_string(),
                json!(output.display().to_string()),
            );
            doc.insert(
                "objectPath".to_string(),
                json!(object.display().to_string()),
            );
            doc.insert(
                "requestedTargetTriple".to_string(),
                target_triple
                    .map(|value| json!(value))
                    .unwrap_or(Value::Null),
            );
            doc.insert(
                "llvmSys211Prefix".to_string(),
                std::env::var("LLVM_SYS_211_PREFIX")
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            doc.insert(
                "cargoTargetDir".to_string(),
                std::env::var("CARGO_TARGET_DIR")
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            doc.insert(
                "meshRtLibPath".to_string(),
                std::env::var("MESH_RT_LIB_PATH")
                    .map(Value::from)
                    .unwrap_or(Value::Null),
            );
            doc.insert("lastStage".to_string(), json!("compile-start"));
            doc.insert("success".to_string(), json!(false));
        });
    }

    pub(crate) fn set_stage(stage: &str) {
        update(|doc| {
            doc.insert("lastStage".to_string(), json!(stage));
        });
    }

    pub(crate) fn mark_object_emission_started() {
        update(|doc| {
            doc.insert("lastStage".to_string(), json!("emit-object"));
            doc.insert("objectEmissionStarted".to_string(), json!(true));
            doc.insert("objectEmissionCompleted".to_string(), json!(false));
        });
    }

    pub(crate) fn mark_object_emitted(object: &Path) {
        update(|doc| {
            doc.insert("lastStage".to_string(), json!("object-emitted"));
            doc.insert("objectEmissionCompleted".to_string(), json!(true));
            doc.insert("objectExistsAfterEmit".to_string(), json!(object.exists()));
        });
    }

    pub(crate) fn set_link_context(
        display_target: &str,
        runtime_path: Option<&Path>,
        runtime_exists: Option<bool>,
        linker_program: Option<&Path>,
    ) {
        update(|doc| {
            doc.insert("displayTarget".to_string(), json!(display_target));
            doc.insert(
                "runtimeLibraryPath".to_string(),
                runtime_path
                    .map(|path| json!(path.display().to_string()))
                    .unwrap_or(Value::Null),
            );
            doc.insert(
                "runtimeLibraryExists".to_string(),
                runtime_exists.map(Value::from).unwrap_or(Value::Null),
            );
            doc.insert(
                "linkerProgram".to_string(),
                linker_program
                    .map(|path| json!(path.display().to_string()))
                    .unwrap_or(Value::Null),
            );
        });
    }

    pub(crate) fn mark_link_started() {
        update(|doc| {
            doc.insert("lastStage".to_string(), json!("invoke-linker"));
            doc.insert("linkStarted".to_string(), json!(true));
            doc.insert("linkCompleted".to_string(), json!(false));
        });
    }

    pub(crate) fn mark_link_completed() {
        update(|doc| {
            doc.insert("lastStage".to_string(), json!("link-completed"));
            doc.insert("linkCompleted".to_string(), json!(true));
        });
    }

    pub(crate) fn mark_success() {
        update(|doc| {
            doc.insert("lastStage".to_string(), json!("compile-succeeded"));
            doc.insert("success".to_string(), json!(true));
            doc.remove("error");
        });
    }

    pub(crate) fn record_error(error: &str) {
        update(|doc| {
            doc.insert("error".to_string(), json!(error));
        });
    }
}

/// Lower a parsed and type-checked Mesh program to MIR.
///
/// This runs the full MIR lowering pipeline: AST-to-MIR conversion (with
/// pipe desugaring, string interpolation compilation, and closure conversion),
/// followed by the monomorphization pass.
///
/// # Errors
///
/// Returns an error string if MIR lowering fails.
pub fn lower_to_mir_module(
    parse: &mesh_parser::Parse,
    typeck: &mesh_typeck::TypeckResult,
) -> Result<mir::MirModule, String> {
    let empty_pub_fns = HashSet::new();
    let empty_inferred_fn_usage_types: HashMap<String, Vec<mesh_typeck::ty::Ty>> = HashMap::new();
    let mut module = lower_to_mir(
        parse,
        typeck,
        "",
        &empty_pub_fns,
        &empty_inferred_fn_usage_types,
    )?;
    monomorphize(&mut module);
    Ok(module)
}

/// Lower a parsed and type-checked Mesh program to MIR without monomorphization.
///
/// Use this when lowering multiple modules that will be merged before
/// monomorphization (which requires reachability analysis from the entry point).
///
/// # Errors
///
/// Returns an error string if MIR lowering fails.
pub fn lower_to_mir_raw(
    parse: &mesh_parser::Parse,
    typeck: &mesh_typeck::TypeckResult,
    module_name: &str,
    pub_fns: &HashSet<String>,
    inferred_fn_usage_types: &HashMap<String, Vec<mesh_typeck::ty::Ty>>,
) -> Result<mir::MirModule, String> {
    let module = lower_to_mir(parse, typeck, module_name, pub_fns, inferred_fn_usage_types)?;
    Ok(module)
}

/// Compile a parsed and type-checked Mesh program to an object file.
///
/// This is the main entry point for code generation. It:
/// 1. Lowers the AST to MIR
/// 2. Monomorphizes generic code
/// 3. Generates LLVM IR
/// 4. Optionally optimizes
/// 5. Emits an object file
///
/// # Arguments
///
/// * `parse` - The parsed Mesh source
/// * `typeck` - The type-checked results
/// * `output` - Path to write the object file
/// * `opt_level` - Optimization level (0 = none, 2 = default)
/// * `target_triple` - Optional target triple; None = host default
///
/// # Errors
///
/// Returns an error string if compilation fails at any stage.
pub fn compile_to_object(
    parse: &mesh_parser::Parse,
    typeck: &mesh_typeck::TypeckResult,
    output: &Path,
    opt_level: u8,
    target_triple: Option<&str>,
) -> Result<(), String> {
    let mir = lower_to_mir_module(parse, typeck)?;

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "mesh_module", opt_level, target_triple)?;
    codegen.compile(&mir)?;

    if opt_level > 0 {
        codegen.run_optimization_passes(opt_level)?;
    }

    codegen.emit_object(output)?;
    Ok(())
}

/// Compile a parsed and type-checked Mesh program to LLVM IR text.
///
/// Similar to `compile_to_object` but emits human-readable LLVM IR (.ll file)
/// instead of a binary object file. Useful for debugging and inspection.
///
/// # Arguments
///
/// * `parse` - The parsed Mesh source
/// * `typeck` - The type-checked results
/// * `output` - Path to write the .ll file
/// * `target_triple` - Optional target triple; None = host default
///
/// # Errors
///
/// Returns an error string if compilation fails at any stage.
pub fn compile_to_llvm_ir(
    parse: &mesh_parser::Parse,
    typeck: &mesh_typeck::TypeckResult,
    output: &Path,
    target_triple: Option<&str>,
) -> Result<(), String> {
    let mir = lower_to_mir_module(parse, typeck)?;

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "mesh_module", 0, target_triple)?;
    codegen.compile(&mir)?;

    codegen.emit_llvm_ir(output)?;
    Ok(())
}

/// Compile a parsed and type-checked Mesh program to a native binary.
///
/// This is the full compilation pipeline: lower to MIR, generate LLVM IR,
/// optimize, emit object file, and link with mesh-rt to produce a native
/// executable.
///
/// # Arguments
///
/// * `parse` - The parsed Mesh source
/// * `typeck` - The type-checked results
/// * `output` - Path to write the final executable
/// * `opt_level` - Optimization level (0 = none, 2 = default)
/// * `target_triple` - Optional target triple; None = host default
/// * `rt_lib_path` - Optional path to the Mesh runtime static library; None = auto-detect
///
/// # Errors
///
/// Returns an error string if compilation or linking fails.
pub fn compile_to_binary(
    parse: &mesh_parser::Parse,
    typeck: &mesh_typeck::TypeckResult,
    output: &Path,
    opt_level: u8,
    target_triple: Option<&str>,
    rt_lib_path: Option<&Path>,
) -> Result<(), String> {
    let obj_path = output.with_extension("o");
    build_trace::set_compile_context(output, &obj_path, target_triple);
    let link_plan = link::prepare_link(target_triple, rt_lib_path)?;

    let result: Result<(), String> = (|| -> Result<(), String> {
        build_trace::set_stage("lower-to-mir");
        let mir = lower_to_mir_module(parse, typeck)?;

        build_trace::set_stage("pre-llvm-init");
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "mesh_module", opt_level, target_triple)?;

        build_trace::set_stage("compile-llvm-module");
        codegen.compile(&mir)?;

        if opt_level > 0 {
            build_trace::set_stage("run-optimization-passes");
            codegen.run_optimization_passes(opt_level)?;
        }

        build_trace::mark_object_emission_started();
        codegen.emit_object(&obj_path)?;
        build_trace::mark_object_emitted(&obj_path);

        link::link_with_plan(&obj_path, output, &link_plan)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            build_trace::mark_success();
            Ok(())
        }
        Err(error) => {
            build_trace::record_error(&error);
            Err(error)
        }
    }
}

/// Compile a parsed and type-checked Mesh program (verify-only pipeline).
///
/// Compiles through the full LLVM IR generation to verify correctness, but
/// does not emit any files. Useful for testing.
///
/// # Errors
///
/// Returns an error string if compilation fails at any stage.
pub fn compile(
    parse: &mesh_parser::Parse,
    typeck: &mesh_typeck::TypeckResult,
) -> Result<(), String> {
    let mir = lower_to_mir_module(parse, typeck)?;

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "mesh_module", 0, None)?;
    codegen.compile(&mir)?;

    Ok(())
}

// ── Multi-Module Compilation (Phase 39) ────────────────────────────────

/// Compile a pre-built MIR module to a native binary.
///
/// This accepts a MIR module directly (already lowered and optionally merged
/// from multiple source modules) and produces a native executable.
pub fn compile_mir_to_binary(
    mir: &mir::MirModule,
    declared_handlers: &[DeclaredRuntimeRegistration],
    startup_work_registrations: &[StartupWorkRegistration],
    output: &Path,
    opt_level: u8,
    target_triple: Option<&str>,
    rt_lib_path: Option<&Path>,
) -> Result<(), String> {
    let obj_path = output.with_extension("o");
    build_trace::set_compile_context(output, &obj_path, target_triple);
    let link_plan = link::prepare_link(target_triple, rt_lib_path)?;

    let result: Result<(), String> = (|| -> Result<(), String> {
        build_trace::set_stage("pre-llvm-init");
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "mesh_module", opt_level, target_triple)?;
        codegen.set_declared_handlers(declared_handlers);
        codegen.set_startup_work_registrations(startup_work_registrations);

        build_trace::set_stage("compile-llvm-module");
        codegen.compile(mir)?;

        if opt_level > 0 {
            build_trace::set_stage("run-optimization-passes");
            codegen.run_optimization_passes(opt_level)?;
        }

        build_trace::mark_object_emission_started();
        codegen.emit_object(&obj_path)?;
        build_trace::mark_object_emitted(&obj_path);

        link::link_with_plan(&obj_path, output, &link_plan)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            build_trace::mark_success();
            Ok(())
        }
        Err(error) => {
            build_trace::record_error(&error);
            Err(error)
        }
    }
}

/// Compile a pre-built MIR module to LLVM IR text.
pub fn compile_mir_to_llvm_ir(
    mir: &mir::MirModule,
    declared_handlers: &[DeclaredRuntimeRegistration],
    startup_work_registrations: &[StartupWorkRegistration],
    output: &Path,
    target_triple: Option<&str>,
) -> Result<(), String> {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "mesh_module", 0, target_triple)?;
    codegen.set_declared_handlers(declared_handlers);
    codegen.set_startup_work_registrations(startup_work_registrations);
    codegen.compile(mir)?;

    codegen.emit_llvm_ir(output)?;
    Ok(())
}

/// Merge multiple MIR modules into a single module.
///
/// Functions, struct definitions, and sum type definitions from all modules
/// are combined. The entry function is taken from the designated entry module.
/// Duplicate struct/sum type definitions (e.g., builtins registered in every
/// module) are deduplicated by name.
///
/// After merging, runs the monomorphization pass to eliminate unreachable
/// functions (which requires the entry point from the merged module).
pub fn merge_mir_modules(
    modules: Vec<mir::MirModule>,
    entry_module_idx: usize,
    extra_reachable_fns: &[String],
) -> mir::MirModule {
    use std::collections::HashSet;

    let mut merged = mir::MirModule {
        functions: Vec::new(),
        structs: Vec::new(),
        sum_types: Vec::new(),
        entry_function: None,
        service_dispatch: std::collections::HashMap::new(),
    };

    let mut seen_functions: HashSet<String> = HashSet::new();
    let mut seen_structs: HashSet<String> = HashSet::new();
    let mut seen_sum_types: HashSet<String> = HashSet::new();

    // Process entry module first so its lowered `mesh_main` wins if multiple
    // modules define a source-level `fn main()`.
    if let Some(entry) = modules.get(entry_module_idx) {
        merged.entry_function = entry.entry_function.clone();
    }

    let mut merge_module = |module: &mir::MirModule| {
        for func in &module.functions {
            if seen_functions.insert(func.name.clone()) {
                merged.functions.push(func.clone());
            }
        }
        for s in &module.structs {
            if seen_structs.insert(s.name.clone()) {
                merged.structs.push(s.clone());
            }
        }
        for st in &module.sum_types {
            if seen_sum_types.insert(st.name.clone()) {
                merged.sum_types.push(st.clone());
            }
        }
        for (key, value) in &module.service_dispatch {
            merged
                .service_dispatch
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    };

    if let Some(entry) = modules.get(entry_module_idx) {
        merge_module(entry);
    }

    for (idx, module) in modules.iter().enumerate() {
        if idx == entry_module_idx {
            continue;
        }
        merge_module(module);
    }

    // Run monomorphization on the merged module to eliminate unreachable
    // functions (builtins like Ord__compare__String that are generated in
    // every module but only used if referenced from main).
    // Keep manifest-declared executable symbols alive even if the local Mesh
    // entrypoint never references them directly; declared-handler preparation
    // runs after merge and still needs those lowered functions available.
    crate::mir::mono::monomorphize_with_roots(&mut merged, extra_reachable_fns);

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{MirExpr, MirFunction, MirModule, MirType};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn merge_mir_modules_prefers_entry_module_mesh_main_when_multiple_modules_define_main() {
        let merged = merge_mir_modules(
            vec![
                MirModule {
                    functions: vec![MirFunction {
                        name: "mesh_main".to_string(),
                        params: Vec::new(),
                        return_type: MirType::Int,
                        body: MirExpr::IntLit(1, MirType::Int),
                        is_closure_fn: false,
                        captures: Vec::new(),
                        has_tail_calls: false,
                    }],
                    structs: Vec::new(),
                    sum_types: Vec::new(),
                    entry_function: Some("mesh_main".to_string()),
                    service_dispatch: std::collections::HashMap::new(),
                },
                MirModule {
                    functions: vec![MirFunction {
                        name: "mesh_main".to_string(),
                        params: Vec::new(),
                        return_type: MirType::Int,
                        body: MirExpr::IntLit(2, MirType::Int),
                        is_closure_fn: false,
                        captures: Vec::new(),
                        has_tail_calls: false,
                    }],
                    structs: Vec::new(),
                    sum_types: Vec::new(),
                    entry_function: Some("mesh_main".to_string()),
                    service_dispatch: std::collections::HashMap::new(),
                },
            ],
            1,
            &[],
        );

        let entry = merged
            .functions
            .iter()
            .find(|function| function.name == "mesh_main")
            .expect("merged MIR should retain mesh_main");

        assert_eq!(merged.entry_function.as_deref(), Some("mesh_main"));
        match &entry.body {
            MirExpr::IntLit(value, MirType::Int) => assert_eq!(*value, 2),
            other => panic!("expected entry mesh_main body from entry module, got {other:?}"),
        }
    }

    #[test]
    fn llvm_codegen_prefers_local_string_binding_over_same_named_function() {
        let source = r#"
fn node_name() -> String do
  "fn"
end

fn request_registry_name_for_node(node_name :: String) -> String do
  if String.length(node_name) > 0 do
    "cluster_proof_work_requests@#{node_name}"
  else
    "cluster_proof_work_requests"
  end
end

fn main() do
  request_registry_name_for_node("peer")
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);
        assert!(
            typeck.errors.is_empty(),
            "expected test source to type-check cleanly, got {:?}",
            typeck.errors
        );

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let ll_path = std::env::temp_dir().join(format!("mesh-codegen-shadowing-{stamp}.ll"));

        compile_to_llvm_ir(&parse, &typeck, &ll_path, None)
            .expect("failed to emit llvm for shadowing regression");
        let llvm = fs::read_to_string(&ll_path).expect("failed to read emitted llvm");
        let _ = fs::remove_file(&ll_path);

        let request_fn = llvm
            .split("define ptr @request_registry_name_for_node")
            .nth(1)
            .and_then(|rest| rest.split("define ").next())
            .expect("request_registry_name_for_node body missing from llvm");

        assert!(
            request_fn.contains("mesh_string_length(ptr %"),
            "expected request_registry_name_for_node to pass a local pointer into mesh_string_length, got:\n{request_fn}"
        );
        assert!(
            !request_fn.contains("mesh_string_length(ptr @node_name)"),
            "request_registry_name_for_node must not pass the same-named function symbol to mesh_string_length:\n{request_fn}"
        );
        assert!(
            !request_fn.contains("mesh_string_concat(ptr %str, ptr @node_name)"),
            "request_registry_name_for_node must interpolate the local binding, not the function symbol:\n{request_fn}"
        );
    }
}
