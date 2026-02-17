//! JIT compilation engine for REPL inputs.
//!
//! Uses the full Mesh compiler pipeline (parse -> typecheck -> MIR -> LLVM IR)
//! to compile expressions, then executes them via LLVM's JIT execution engine.
//! This ensures REPL behavior is identical to compiled code.
//!
//! The actor runtime (mesh-rt) is linked and its symbols are registered with
//! LLVM so that JIT-compiled code can call runtime functions like
//! `mesh_gc_alloc`, `mesh_actor_spawn`, etc.

use crate::session::ReplSession;
use std::sync::Once;

static RUNTIME_INIT: Once = Once::new();

/// Initialize the Mesh runtime (GC + actor scheduler) and register all
/// runtime symbols with LLVM so that JIT-compiled code can resolve them.
///
/// This is called once at REPL startup. Subsequent calls are no-ops.
pub fn init_runtime() {
    RUNTIME_INIT.call_once(|| {
        // Initialize the GC arena
        mesh_rt::mesh_rt_init();

        // Initialize the actor scheduler with default number of workers
        mesh_rt::mesh_rt_init_actor(0);

        // Register mesh-rt symbols with LLVM's global symbol table so the
        // JIT execution engine can resolve them. LLVM's MCJIT uses dlsym
        // on some platforms, but explicit registration is more reliable.
        register_runtime_symbols();
    });
}

/// Register all mesh-rt extern "C" symbols with LLVM's dynamic lookup.
///
/// This calls LLVMAddSymbol for each runtime function, making them
/// available to JIT-compiled code. Without this, the JIT engine cannot
/// resolve calls to runtime functions like mesh_gc_alloc, mesh_print, etc.
fn register_runtime_symbols() {
    // LLVMAddSymbol is the LLVM C API for registering symbols with the
    // JIT's symbol resolver. inkwell 0.8 doesn't expose it directly,
    // so we call into llvm-sys through the re-exported C bindings.
    extern "C" {
        fn LLVMAddSymbol(name: *const std::ffi::c_char, value: *mut std::ffi::c_void);
    }

    /// Register a single symbol with LLVM's JIT resolver.
    fn add_sym(name: &str, ptr: *const ()) {
        let c_name = std::ffi::CString::new(name).unwrap();
        unsafe {
            LLVMAddSymbol(c_name.as_ptr(), ptr as *mut std::ffi::c_void);
        }
    }

    // GC / runtime init
    add_sym("mesh_rt_init", mesh_rt::mesh_rt_init as *const ());
    add_sym("mesh_gc_alloc", mesh_rt::mesh_gc_alloc as *const ());
    add_sym("mesh_gc_alloc_actor", mesh_rt::mesh_gc_alloc_actor as *const ());

    // IO
    add_sym("mesh_print", mesh_rt::mesh_print as *const ());
    add_sym("mesh_println", mesh_rt::mesh_println as *const ());
    add_sym("mesh_io_read_line", mesh_rt::mesh_io_read_line as *const ());
    add_sym("mesh_io_eprintln", mesh_rt::mesh_io_eprintln as *const ());

    // String operations
    add_sym("mesh_string_new", mesh_rt::mesh_string_new as *const ());
    add_sym("mesh_string_concat", mesh_rt::mesh_string_concat as *const ());
    add_sym("mesh_string_length", mesh_rt::mesh_string_length as *const ());
    add_sym("mesh_string_eq", mesh_rt::mesh_string_eq as *const ());
    add_sym("mesh_string_contains", mesh_rt::mesh_string_contains as *const ());
    add_sym("mesh_string_slice", mesh_rt::mesh_string_slice as *const ());
    add_sym("mesh_string_replace", mesh_rt::mesh_string_replace as *const ());
    add_sym("mesh_string_starts_with", mesh_rt::mesh_string_starts_with as *const ());
    add_sym("mesh_string_ends_with", mesh_rt::mesh_string_ends_with as *const ());
    add_sym("mesh_string_trim", mesh_rt::mesh_string_trim as *const ());
    add_sym("mesh_string_to_upper", mesh_rt::mesh_string_to_upper as *const ());
    add_sym("mesh_string_to_lower", mesh_rt::mesh_string_to_lower as *const ());
    add_sym("mesh_int_to_string", mesh_rt::mesh_int_to_string as *const ());
    add_sym("mesh_bool_to_string", mesh_rt::mesh_bool_to_string as *const ());
    add_sym("mesh_float_to_string", mesh_rt::mesh_float_to_string as *const ());
    add_sym("mesh_string_split", mesh_rt::mesh_string_split as *const ());
    add_sym("mesh_string_join", mesh_rt::mesh_string_join as *const ());
    add_sym("mesh_string_to_int", mesh_rt::mesh_string_to_int as *const ());
    add_sym("mesh_string_to_float", mesh_rt::mesh_string_to_float as *const ());

    // Panic
    add_sym("mesh_panic", mesh_rt::mesh_panic as *const ());

    // Actor runtime
    add_sym("mesh_rt_init_actor", mesh_rt::mesh_rt_init_actor as *const ());
    add_sym("mesh_rt_run_scheduler", mesh_rt::mesh_rt_run_scheduler as *const ());
    add_sym("mesh_actor_spawn", mesh_rt::mesh_actor_spawn as *const ());
    add_sym("mesh_actor_send", mesh_rt::mesh_actor_send as *const ());
    add_sym("mesh_actor_receive", mesh_rt::mesh_actor_receive as *const ());
    add_sym("mesh_actor_self", mesh_rt::mesh_actor_self as *const ());
    add_sym("mesh_actor_link", mesh_rt::mesh_actor_link as *const ());
    add_sym("mesh_actor_set_terminate", mesh_rt::mesh_actor_set_terminate as *const ());
    add_sym("mesh_actor_register", mesh_rt::mesh_actor_register as *const ());
    add_sym("mesh_actor_whereis", mesh_rt::mesh_actor_whereis as *const ());
    add_sym("mesh_reduction_check", mesh_rt::mesh_reduction_check as *const ());

    // Services
    add_sym("mesh_service_call", mesh_rt::mesh_service_call as *const ());
    add_sym("mesh_service_reply", mesh_rt::mesh_service_reply as *const ());

    // Collections -- List
    add_sym("mesh_list_new", mesh_rt::mesh_list_new as *const ());
    add_sym("mesh_list_from_array", mesh_rt::mesh_list_from_array as *const ());
    add_sym("mesh_list_get", mesh_rt::mesh_list_get as *const ());
    add_sym("mesh_list_head", mesh_rt::mesh_list_head as *const ());
    add_sym("mesh_list_tail", mesh_rt::mesh_list_tail as *const ());
    add_sym("mesh_list_length", mesh_rt::mesh_list_length as *const ());
    add_sym("mesh_list_append", mesh_rt::mesh_list_append as *const ());
    add_sym("mesh_list_concat", mesh_rt::mesh_list_concat as *const ());
    add_sym("mesh_list_map", mesh_rt::mesh_list_map as *const ());
    add_sym("mesh_list_filter", mesh_rt::mesh_list_filter as *const ());
    add_sym("mesh_list_reduce", mesh_rt::mesh_list_reduce as *const ());
    add_sym("mesh_list_reverse", mesh_rt::mesh_list_reverse as *const ());
    add_sym("mesh_list_sort", mesh_rt::mesh_list_sort as *const ());
    add_sym("mesh_list_find", mesh_rt::mesh_list_find as *const ());
    add_sym("mesh_list_any", mesh_rt::mesh_list_any as *const ());
    add_sym("mesh_list_all", mesh_rt::mesh_list_all as *const ());
    add_sym("mesh_list_contains", mesh_rt::mesh_list_contains as *const ());
    add_sym("mesh_list_zip", mesh_rt::mesh_list_zip as *const ());
    add_sym("mesh_list_flat_map", mesh_rt::mesh_list_flat_map as *const ());
    add_sym("mesh_list_flatten", mesh_rt::mesh_list_flatten as *const ());
    add_sym("mesh_list_enumerate", mesh_rt::mesh_list_enumerate as *const ());
    add_sym("mesh_list_take", mesh_rt::mesh_list_take as *const ());
    add_sym("mesh_list_drop", mesh_rt::mesh_list_drop as *const ());
    add_sym("mesh_list_nth", mesh_rt::mesh_list_nth as *const ());
    add_sym("mesh_list_last", mesh_rt::mesh_list_last as *const ());

    // Collections -- Map
    add_sym("mesh_map_new", mesh_rt::mesh_map_new as *const ());
    add_sym("mesh_map_put", mesh_rt::mesh_map_put as *const ());
    add_sym("mesh_map_get", mesh_rt::mesh_map_get as *const ());
    add_sym("mesh_map_delete", mesh_rt::mesh_map_delete as *const ());
    add_sym("mesh_map_has_key", mesh_rt::mesh_map_has_key as *const ());
    add_sym("mesh_map_size", mesh_rt::mesh_map_size as *const ());
    add_sym("mesh_map_keys", mesh_rt::mesh_map_keys as *const ());
    add_sym("mesh_map_values", mesh_rt::mesh_map_values as *const ());
    add_sym("mesh_map_merge", mesh_rt::mesh_map_merge as *const ());
    add_sym("mesh_map_to_list", mesh_rt::mesh_map_to_list as *const ());
    add_sym("mesh_map_from_list", mesh_rt::mesh_map_from_list as *const ());

    // Collections -- Set
    add_sym("mesh_set_new", mesh_rt::mesh_set_new as *const ());
    add_sym("mesh_set_add", mesh_rt::mesh_set_add as *const ());
    add_sym("mesh_set_remove", mesh_rt::mesh_set_remove as *const ());
    add_sym("mesh_set_contains", mesh_rt::mesh_set_contains as *const ());
    add_sym("mesh_set_size", mesh_rt::mesh_set_size as *const ());
    add_sym("mesh_set_union", mesh_rt::mesh_set_union as *const ());
    add_sym("mesh_set_intersection", mesh_rt::mesh_set_intersection as *const ());
    add_sym("mesh_set_difference", mesh_rt::mesh_set_difference as *const ());
    add_sym("mesh_set_to_list", mesh_rt::mesh_set_to_list as *const ());
    add_sym("mesh_set_from_list", mesh_rt::mesh_set_from_list as *const ());

    // Collections -- Queue
    add_sym("mesh_queue_new", mesh_rt::mesh_queue_new as *const ());
    add_sym("mesh_queue_push", mesh_rt::mesh_queue_push as *const ());
    add_sym("mesh_queue_pop", mesh_rt::mesh_queue_pop as *const ());
    add_sym("mesh_queue_peek", mesh_rt::mesh_queue_peek as *const ());
    add_sym("mesh_queue_size", mesh_rt::mesh_queue_size as *const ());
    add_sym("mesh_queue_is_empty", mesh_rt::mesh_queue_is_empty as *const ());

    // Collections -- Range
    add_sym("mesh_range_new", mesh_rt::mesh_range_new as *const ());
    add_sym("mesh_range_to_list", mesh_rt::mesh_range_to_list as *const ());
    add_sym("mesh_range_length", mesh_rt::mesh_range_length as *const ());
    add_sym("mesh_range_map", mesh_rt::mesh_range_map as *const ());
    add_sym("mesh_range_filter", mesh_rt::mesh_range_filter as *const ());

    // Iterator protocol -- generic dispatch
    add_sym("mesh_iter_generic_next", mesh_rt::mesh_iter_generic_next as *const ());

    // Iterator adapters -- constructors
    add_sym("mesh_iter_map", mesh_rt::mesh_iter_map as *const ());
    add_sym("mesh_iter_filter", mesh_rt::mesh_iter_filter as *const ());
    add_sym("mesh_iter_take", mesh_rt::mesh_iter_take as *const ());
    add_sym("mesh_iter_skip", mesh_rt::mesh_iter_skip as *const ());
    add_sym("mesh_iter_enumerate", mesh_rt::mesh_iter_enumerate as *const ());
    add_sym("mesh_iter_zip", mesh_rt::mesh_iter_zip as *const ());

    // Iterator adapters -- next functions
    add_sym("mesh_iter_map_next", mesh_rt::mesh_iter_map_next as *const ());
    add_sym("mesh_iter_filter_next", mesh_rt::mesh_iter_filter_next as *const ());
    add_sym("mesh_iter_take_next", mesh_rt::mesh_iter_take_next as *const ());
    add_sym("mesh_iter_skip_next", mesh_rt::mesh_iter_skip_next as *const ());
    add_sym("mesh_iter_enumerate_next", mesh_rt::mesh_iter_enumerate_next as *const ());
    add_sym("mesh_iter_zip_next", mesh_rt::mesh_iter_zip_next as *const ());

    // Iterator terminal operations
    add_sym("mesh_iter_count", mesh_rt::mesh_iter_count as *const ());
    add_sym("mesh_iter_sum", mesh_rt::mesh_iter_sum as *const ());
    add_sym("mesh_iter_any", mesh_rt::mesh_iter_any as *const ());
    add_sym("mesh_iter_all", mesh_rt::mesh_iter_all as *const ());
    add_sym("mesh_iter_find", mesh_rt::mesh_iter_find as *const ());
    add_sym("mesh_iter_reduce", mesh_rt::mesh_iter_reduce as *const ());

    // Collect operations
    add_sym("mesh_list_collect", mesh_rt::mesh_list_collect as *const ());
    add_sym("mesh_map_collect", mesh_rt::mesh_map_collect as *const ());
    add_sym("mesh_map_collect_string_keys", mesh_rt::mesh_map_collect_string_keys as *const ());
    add_sym("mesh_set_collect", mesh_rt::mesh_set_collect as *const ());
    add_sym("mesh_string_collect", mesh_rt::mesh_string_collect as *const ());

    // Collection iterator constructors + next functions
    add_sym("mesh_list_iter_new", mesh_rt::collections::list::mesh_list_iter_new as *const ());
    add_sym("mesh_list_iter_next", mesh_rt::collections::list::mesh_list_iter_next as *const ());
    add_sym("mesh_map_iter_new", mesh_rt::collections::map::mesh_map_iter_new as *const ());
    add_sym("mesh_map_iter_next", mesh_rt::collections::map::mesh_map_iter_next as *const ());
    add_sym("mesh_set_iter_new", mesh_rt::collections::set::mesh_set_iter_new as *const ());
    add_sym("mesh_set_iter_next", mesh_rt::collections::set::mesh_set_iter_next as *const ());
    add_sym("mesh_range_iter_new", mesh_rt::collections::range::mesh_range_iter_new as *const ());
    add_sym("mesh_range_iter_next", mesh_rt::collections::range::mesh_range_iter_next as *const ());
    add_sym("mesh_iter_from", mesh_rt::collections::list::mesh_iter_from as *const ());

    // Hash operations
    add_sym("mesh_hash_int", mesh_rt::mesh_hash_int as *const ());
    add_sym("mesh_hash_float", mesh_rt::mesh_hash_float as *const ());
    add_sym("mesh_hash_string", mesh_rt::mesh_hash_string as *const ());
    add_sym("mesh_hash_bool", mesh_rt::mesh_hash_bool as *const ());
    add_sym("mesh_hash_combine", mesh_rt::mesh_hash_combine as *const ());

    // Timer + monitor operations
    add_sym("mesh_timer_sleep", mesh_rt::mesh_timer_sleep as *const ());
    add_sym("mesh_timer_send_after", mesh_rt::mesh_timer_send_after as *const ());
    add_sym("mesh_process_monitor", mesh_rt::mesh_process_monitor as *const ());
    add_sym("mesh_process_demonitor", mesh_rt::mesh_process_demonitor as *const ());

    // Collections -- Tuple
    add_sym("mesh_tuple_first", mesh_rt::mesh_tuple_first as *const ());
    add_sym("mesh_tuple_second", mesh_rt::mesh_tuple_second as *const ());
    add_sym("mesh_tuple_nth", mesh_rt::mesh_tuple_nth as *const ());
    add_sym("mesh_tuple_size", mesh_rt::mesh_tuple_size as *const ());

    // File operations
    add_sym("mesh_file_read", mesh_rt::mesh_file_read as *const ());
    add_sym("mesh_file_write", mesh_rt::mesh_file_write as *const ());
    add_sym("mesh_file_append", mesh_rt::mesh_file_append as *const ());
    add_sym("mesh_file_exists", mesh_rt::mesh_file_exists as *const ());
    add_sym("mesh_file_delete", mesh_rt::mesh_file_delete as *const ());

    // Environment
    add_sym("mesh_env_get", mesh_rt::mesh_env_get as *const ());
    add_sym("mesh_env_args", mesh_rt::mesh_env_args as *const ());

    // JSON
    add_sym("mesh_json_get", mesh_rt::mesh_json_get as *const ());
    add_sym("mesh_json_get_nested", mesh_rt::mesh_json_get_nested as *const ());
    add_sym("mesh_json_parse", mesh_rt::mesh_json_parse as *const ());
    add_sym("mesh_json_encode", mesh_rt::mesh_json_encode as *const ());
    add_sym("mesh_json_from_string", mesh_rt::mesh_json_from_string as *const ());
    add_sym("mesh_json_from_int", mesh_rt::mesh_json_from_int as *const ());
    add_sym("mesh_json_from_float", mesh_rt::mesh_json_from_float as *const ());
    add_sym("mesh_json_from_bool", mesh_rt::mesh_json_from_bool as *const ());
    add_sym("mesh_json_encode_string", mesh_rt::mesh_json_encode_string as *const ());
    add_sym("mesh_json_encode_int", mesh_rt::mesh_json_encode_int as *const ());
    add_sym("mesh_json_encode_bool", mesh_rt::mesh_json_encode_bool as *const ());
    add_sym("mesh_json_encode_list", mesh_rt::mesh_json_encode_list as *const ());
    add_sym("mesh_json_encode_map", mesh_rt::mesh_json_encode_map as *const ());

    // ORM SQL Generation (Phase 97)
    add_sym("mesh_orm_build_select", mesh_rt::mesh_orm_build_select as *const ());
    add_sym("mesh_orm_build_insert", mesh_rt::mesh_orm_build_insert as *const ());
    add_sym("mesh_orm_build_update", mesh_rt::mesh_orm_build_update as *const ());
    add_sym("mesh_orm_build_delete", mesh_rt::mesh_orm_build_delete as *const ());

    // Query Builder (Phase 98)
    add_sym("mesh_query_from", mesh_rt::mesh_query_from as *const ());
    add_sym("mesh_query_where", mesh_rt::mesh_query_where as *const ());
    add_sym("mesh_query_where_op", mesh_rt::mesh_query_where_op as *const ());
    add_sym("mesh_query_where_in", mesh_rt::mesh_query_where_in as *const ());
    add_sym("mesh_query_where_null", mesh_rt::mesh_query_where_null as *const ());
    add_sym("mesh_query_where_not_null", mesh_rt::mesh_query_where_not_null as *const ());
    add_sym("mesh_query_where_not_in", mesh_rt::mesh_query_where_not_in as *const ());
    add_sym("mesh_query_where_between", mesh_rt::mesh_query_where_between as *const ());
    add_sym("mesh_query_where_or", mesh_rt::mesh_query_where_or as *const ());
    add_sym("mesh_query_select", mesh_rt::mesh_query_select as *const ());
    add_sym("mesh_query_order_by", mesh_rt::mesh_query_order_by as *const ());
    add_sym("mesh_query_limit", mesh_rt::mesh_query_limit as *const ());
    add_sym("mesh_query_offset", mesh_rt::mesh_query_offset as *const ());
    add_sym("mesh_query_join", mesh_rt::mesh_query_join as *const ());
    add_sym("mesh_query_join_as", mesh_rt::mesh_query_join_as as *const ());
    add_sym("mesh_query_group_by", mesh_rt::mesh_query_group_by as *const ());
    add_sym("mesh_query_having", mesh_rt::mesh_query_having as *const ());
    add_sym("mesh_query_fragment", mesh_rt::mesh_query_fragment as *const ());
    // Query Builder Raw Extensions (Phase 103)
    add_sym("mesh_query_select_raw", mesh_rt::mesh_query_select_raw as *const ());
    add_sym("mesh_query_where_raw", mesh_rt::mesh_query_where_raw as *const ());
    // Raw ORDER BY / GROUP BY (Phase 106)
    add_sym("mesh_query_order_by_raw", mesh_rt::mesh_query_order_by_raw as *const ());
    add_sym("mesh_query_group_by_raw", mesh_rt::mesh_query_group_by_raw as *const ());

    // Repo Read Operations (Phase 98)
    add_sym("mesh_repo_all", mesh_rt::mesh_repo_all as *const ());
    add_sym("mesh_repo_one", mesh_rt::mesh_repo_one as *const ());
    add_sym("mesh_repo_get", mesh_rt::mesh_repo_get as *const ());
    add_sym("mesh_repo_get_by", mesh_rt::mesh_repo_get_by as *const ());
    add_sym("mesh_repo_count", mesh_rt::mesh_repo_count as *const ());
    add_sym("mesh_repo_exists", mesh_rt::mesh_repo_exists as *const ());
    // Repo Write Operations (Phase 98)
    add_sym("mesh_repo_insert", mesh_rt::mesh_repo_insert as *const ());
    add_sym("mesh_repo_update", mesh_rt::mesh_repo_update as *const ());
    add_sym("mesh_repo_delete", mesh_rt::mesh_repo_delete as *const ());
    add_sym("mesh_repo_transaction", mesh_rt::mesh_repo_transaction as *const ());

    // Extended Repo Write Operations (Phase 103)
    add_sym("mesh_repo_update_where", mesh_rt::mesh_repo_update_where as *const ());
    add_sym("mesh_repo_delete_where", mesh_rt::mesh_repo_delete_where as *const ());
    add_sym("mesh_repo_query_raw", mesh_rt::mesh_repo_query_raw as *const ());
    add_sym("mesh_repo_execute_raw", mesh_rt::mesh_repo_execute_raw as *const ());

    // Repo Preloading (Phase 100)
    add_sym("mesh_repo_preload", mesh_rt::mesh_repo_preload as *const ());

    // Repo Changeset Operations (Phase 99)
    add_sym("mesh_repo_insert_changeset", mesh_rt::mesh_repo_insert_changeset as *const ());
    add_sym("mesh_repo_update_changeset", mesh_rt::mesh_repo_update_changeset as *const ());

    // Migration DDL Operations (Phase 101)
    add_sym("mesh_migration_create_table", mesh_rt::mesh_migration_create_table as *const ());
    add_sym("mesh_migration_drop_table", mesh_rt::mesh_migration_drop_table as *const ());
    add_sym("mesh_migration_add_column", mesh_rt::mesh_migration_add_column as *const ());
    add_sym("mesh_migration_drop_column", mesh_rt::mesh_migration_drop_column as *const ());
    add_sym("mesh_migration_rename_column", mesh_rt::mesh_migration_rename_column as *const ());
    add_sym("mesh_migration_create_index", mesh_rt::mesh_migration_create_index as *const ());
    add_sym("mesh_migration_drop_index", mesh_rt::mesh_migration_drop_index as *const ());
    add_sym("mesh_migration_execute", mesh_rt::mesh_migration_execute as *const ());

    // Changeset Operations (Phase 99)
    add_sym("mesh_changeset_cast", mesh_rt::mesh_changeset_cast as *const ());
    add_sym("mesh_changeset_cast_with_types", mesh_rt::mesh_changeset_cast_with_types as *const ());
    add_sym("mesh_changeset_validate_required", mesh_rt::mesh_changeset_validate_required as *const ());
    add_sym("mesh_changeset_validate_length", mesh_rt::mesh_changeset_validate_length as *const ());
    add_sym("mesh_changeset_validate_format", mesh_rt::mesh_changeset_validate_format as *const ());
    add_sym("mesh_changeset_validate_inclusion", mesh_rt::mesh_changeset_validate_inclusion as *const ());
    add_sym("mesh_changeset_validate_number", mesh_rt::mesh_changeset_validate_number as *const ());
    add_sym("mesh_changeset_valid", mesh_rt::mesh_changeset_valid as *const ());
    add_sym("mesh_changeset_errors", mesh_rt::mesh_changeset_errors as *const ());
    add_sym("mesh_changeset_changes", mesh_rt::mesh_changeset_changes as *const ());
    add_sym("mesh_changeset_get_change", mesh_rt::mesh_changeset_get_change as *const ());
    add_sym("mesh_changeset_get_error", mesh_rt::mesh_changeset_get_error as *const ());

    // HTTP
    add_sym("mesh_http_get", mesh_rt::mesh_http_get as *const ());
    add_sym("mesh_http_post", mesh_rt::mesh_http_post as *const ());
    add_sym("mesh_http_router", mesh_rt::mesh_http_router as *const ());
    add_sym("mesh_http_route", mesh_rt::mesh_http_route as *const ());
    add_sym("mesh_http_serve", mesh_rt::mesh_http_serve as *const ());
    add_sym("mesh_http_request_method", mesh_rt::mesh_http_request_method as *const ());
    add_sym("mesh_http_request_path", mesh_rt::mesh_http_request_path as *const ());
    add_sym("mesh_http_request_body", mesh_rt::mesh_http_request_body as *const ());
    add_sym("mesh_http_request_header", mesh_rt::mesh_http_request_header as *const ());
    add_sym("mesh_http_request_query", mesh_rt::mesh_http_request_query as *const ());
    add_sym("mesh_http_response_new", mesh_rt::mesh_http_response_new as *const ());
}

/// The result of evaluating an expression in the REPL.
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// String representation of the evaluated value.
    pub value: String,
    /// Type name of the result.
    pub ty: String,
}

/// Keywords that indicate a definition (not an expression to evaluate).
const DEFINITION_KEYWORDS: &[&str] = &[
    "fn", "def", "let", "type", "struct", "module", "actor", "service", "interface", "trait",
    "impl", "supervisor",
];

/// Check whether the input appears to be a definition rather than an expression.
///
/// Definitions start with specific keywords (fn, let, type, struct, etc.)
/// and are added to the session context rather than evaluated for a result.
pub fn is_definition(input: &str) -> bool {
    let trimmed = input.trim();
    DEFINITION_KEYWORDS
        .iter()
        .any(|kw| {
            trimmed.starts_with(kw)
                && trimmed[kw.len()..]
                    .starts_with(|c: char| c.is_whitespace() || c == '(')
        })
}

/// Evaluate a Mesh expression or definition using the full compiler pipeline.
///
/// For expressions: wraps in a function, compiles via LLVM JIT, executes, and
/// returns the result value with its type.
///
/// For definitions: adds to the session context and returns a "Defined" result.
///
/// The LLVM Context is created per-evaluation. In a future optimization, the
/// context could be persisted across evaluations for better performance.
pub fn jit_eval(source: &str, session: &mut ReplSession) -> Result<EvalResult, String> {
    let trimmed = source.trim();

    if trimmed.is_empty() {
        return Ok(EvalResult {
            value: String::new(),
            ty: "Unit".to_string(),
        });
    }

    // Detect whether this is a definition or an expression
    if is_definition(trimmed) {
        return eval_definition(trimmed, session);
    }

    eval_expression(trimmed, session)
}

/// Process a definition: validate it parses and type-checks, then store it.
fn eval_definition(input: &str, session: &mut ReplSession) -> Result<EvalResult, String> {
    // Build full source with existing definitions + new one
    let mut full_source = session.definitions_source();
    if !full_source.is_empty() {
        full_source.push('\n');
    }
    full_source.push_str(input);

    // Parse to check for syntax errors
    let parse = mesh_parser::parse(&full_source);
    if !parse.ok() {
        let errors: Vec<String> = parse.errors().iter().map(|e| format!("{}", e)).collect();
        return Err(format!("Parse error: {}", errors.join(", ")));
    }

    // Type check to validate the definition
    let typeck = mesh_typeck::check(&parse);
    if !typeck.errors.is_empty() {
        let rendered = typeck.render_errors(&full_source, "<repl>", &mesh_typeck::diagnostics::DiagnosticOptions::colorless());
        return Err(rendered.join("\n"));
    }

    // Extract the definition name for display
    let def_name = extract_definition_name(input);

    // Extract the type of the definition for display
    let type_info = if let Some(ref result_ty) = typeck.result_type {
        format!("{}", result_ty)
    } else {
        String::new()
    };

    // Store the definition for future inputs
    session.add_definition(input);

    let display = if !type_info.is_empty() {
        format!("Defined: {} :: {}", def_name, type_info)
    } else {
        format!("Defined: {}", def_name)
    };

    Ok(EvalResult {
        value: display,
        ty: "Definition".to_string(),
    })
}

/// Process an expression: wrap it, compile via full pipeline, and execute via JIT.
fn eval_expression(input: &str, session: &mut ReplSession) -> Result<EvalResult, String> {
    let (full_source, wrapper_fn) = session.wrap_expression(input);

    // Step 1: Parse
    let parse = mesh_parser::parse(&full_source);
    if !parse.ok() {
        let errors: Vec<String> = parse.errors().iter().map(|e| format!("{}", e)).collect();
        return Err(format!("Parse error: {}", errors.join(", ")));
    }

    // Step 2: Type check
    let typeck = mesh_typeck::check(&parse);
    if !typeck.errors.is_empty() {
        let rendered = typeck.render_errors(&full_source, "<repl>", &mesh_typeck::diagnostics::DiagnosticOptions::colorless());
        return Err(rendered.join("\n"));
    }

    // Get the result type from the type checker
    let result_type_name = if let Some(ref ty) = typeck.result_type {
        format!("{}", ty)
    } else {
        "Unit".to_string()
    };

    // Step 3: Lower to MIR
    let mir = mesh_codegen::lower_to_mir_module(&parse, &typeck)?;

    // Step 4: Generate LLVM IR and execute via JIT
    let value = jit_execute(&mir, &wrapper_fn, &result_type_name)?;

    // Record the result in session history
    session.record_result(value.clone(), result_type_name.clone());

    Ok(EvalResult {
        value,
        ty: result_type_name,
    })
}

/// Compile MIR to LLVM IR and execute the wrapper function via JIT.
///
/// Uses LLVM's JIT execution engine to call the generated wrapper function
/// and capture its return value.
fn jit_execute(
    mir: &mesh_codegen::mir::MirModule,
    wrapper_fn_name: &str,
    result_type: &str,
) -> Result<String, String> {
    use inkwell::context::Context;
    use inkwell::targets::{InitializationConfig, Target};
    use mesh_codegen::codegen::CodeGen;

    // Initialize native target for JIT
    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("Failed to initialize native target: {}", e))?;

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "repl_jit", 0, None)?;
    codegen.compile(mir)?;

    // Extract the LLVM module and create a JIT execution engine
    let module = codegen.into_module();
    let ee = module
        .create_jit_execution_engine(inkwell::OptimizationLevel::None)
        .map_err(|e| format!("Failed to create JIT engine: {}", e))?;

    // Look up the wrapper function
    let maybe_fn = unsafe { ee.get_function::<unsafe extern "C" fn() -> i64>(wrapper_fn_name) };

    match maybe_fn {
        Ok(jit_fn) => {
            let result = unsafe { jit_fn.call() };
            let formatted = format_jit_result(result, result_type);
            Ok(formatted)
        }
        Err(_) => {
            // Function might return void (Unit type)
            let maybe_void_fn =
                unsafe { ee.get_function::<unsafe extern "C" fn()>(wrapper_fn_name) };
            match maybe_void_fn {
                Ok(jit_fn) => {
                    unsafe { jit_fn.call() };
                    Ok("()".to_string())
                }
                Err(e) => Err(format!(
                    "Failed to find JIT function '{}': {}",
                    wrapper_fn_name, e
                )),
            }
        }
    }
}

/// Format a raw JIT result value based on its Mesh type.
fn format_jit_result(raw: i64, type_name: &str) -> String {
    match type_name {
        "Int" => format!("{}", raw),
        "Bool" => {
            if raw != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        "Float" => {
            let f = f64::from_bits(raw as u64);
            format!("{}", f)
        }
        "Unit" | "()" => "()".to_string(),
        _ => format!("<{} at 0x{:x}>", type_name, raw),
    }
}

/// Extract the name from a definition for display.
fn extract_definition_name(input: &str) -> String {
    let trimmed = input.trim();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() >= 2 {
        // Handle "fn name(...)" by stripping parens
        let name = tokens[1];
        if let Some(paren_pos) = name.find('(') {
            name[..paren_pos].to_string()
        } else {
            name.to_string()
        }
    } else {
        "<anonymous>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_definition_fn() {
        assert!(is_definition("fn foo() do 1 end"));
        assert!(is_definition("fn bar(x :: Int) :: Int do x end"));
        assert!(is_definition("  fn indented() do 1 end"));
    }

    #[test]
    fn test_is_definition_let() {
        assert!(is_definition("let x = 42"));
        assert!(is_definition("let (a, b) = (1, 2)"));
    }

    #[test]
    fn test_is_definition_type() {
        assert!(is_definition("type Color do Red | Green | Blue end"));
        assert!(is_definition("struct Point do x :: Int y :: Int end"));
    }

    #[test]
    fn test_is_definition_others() {
        assert!(is_definition("module Foo do end"));
        assert!(is_definition("actor Counter do end"));
        assert!(is_definition("service Cache do end"));
        assert!(is_definition("interface Printable do end"));
    }

    #[test]
    fn test_is_not_definition() {
        assert!(!is_definition("1 + 2"));
        assert!(!is_definition("foo()"));
        assert!(!is_definition("if true do 1 else 2 end"));
        assert!(!is_definition("x"));
    }

    #[test]
    fn test_extract_definition_name() {
        assert_eq!(extract_definition_name("fn add(a, b) do a + b end"), "add");
        assert_eq!(extract_definition_name("let x = 42"), "x");
        assert_eq!(extract_definition_name("type Color do end"), "Color");
        assert_eq!(extract_definition_name("struct Point do end"), "Point");
    }

    #[test]
    fn test_format_jit_result_int() {
        assert_eq!(format_jit_result(42, "Int"), "42");
        assert_eq!(format_jit_result(-1, "Int"), "-1");
        assert_eq!(format_jit_result(0, "Int"), "0");
    }

    #[test]
    fn test_format_jit_result_bool() {
        assert_eq!(format_jit_result(1, "Bool"), "true");
        assert_eq!(format_jit_result(0, "Bool"), "false");
    }

    #[test]
    fn test_format_jit_result_unit() {
        assert_eq!(format_jit_result(0, "Unit"), "()");
        assert_eq!(format_jit_result(0, "()"), "()");
    }

    #[test]
    fn test_eval_empty_input() {
        let mut session = ReplSession::new();
        let result = jit_eval("", &mut session).unwrap();
        assert_eq!(result.ty, "Unit");
    }

    #[test]
    fn test_eval_whitespace_input() {
        let mut session = ReplSession::new();
        let result = jit_eval("   ", &mut session).unwrap();
        assert_eq!(result.ty, "Unit");
    }

    #[test]
    fn test_init_runtime_is_idempotent() {
        // Should not panic when called multiple times
        init_runtime();
        init_runtime();
    }

    #[test]
    fn test_repl_config_default() {
        let config = crate::ReplConfig::default();
        assert_eq!(config.prompt, "mesh> ");
        assert_eq!(config.continuation, "  ... ");
    }
}
