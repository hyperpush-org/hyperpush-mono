//! Runtime function declarations in the LLVM module.
//!
//! Declares all `mesh-rt` extern "C" functions so that codegen can emit
//! calls to them. These declarations must match the signatures in the
//! `mesh-rt` crate exactly.

use inkwell::module::Module;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::FunctionValue;
use inkwell::AddressSpace;

/// Declare all Mesh runtime functions in the LLVM module.
///
/// This should be called once during module initialization, before any
/// codegen that might reference runtime functions.
pub fn declare_intrinsics<'ctx>(module: &Module<'ctx>) {
    let context = module.get_context();
    let void_type = context.void_type();
    let i8_type = context.i8_type();
    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let f64_type = context.f64_type();
    let ptr_type = context.ptr_type(AddressSpace::default());

    // mesh_rt_init() -> void
    let init_ty = void_type.fn_type(&[], false);
    module.add_function("mesh_rt_init", init_ty, Some(inkwell::module::Linkage::External));

    // mesh_gc_alloc_actor(size: u64, align: u64) -> ptr
    let gc_alloc_ty = ptr_type.fn_type(
        &[i64_type.into(), i64_type.into()],
        false,
    );
    module.add_function("mesh_gc_alloc_actor", gc_alloc_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_new(data: ptr, len: u64) -> ptr
    let string_new_ty = ptr_type.fn_type(
        &[ptr_type.into(), i64_type.into()],
        false,
    );
    module.add_function("mesh_string_new", string_new_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_concat(a: ptr, b: ptr) -> ptr
    let string_concat_ty = ptr_type.fn_type(
        &[ptr_type.into(), ptr_type.into()],
        false,
    );
    module.add_function("mesh_string_concat", string_concat_ty, Some(inkwell::module::Linkage::External));

    // mesh_int_to_string(val: i64) -> ptr
    let int_to_string_ty = ptr_type.fn_type(&[i64_type.into()], false);
    module.add_function("mesh_int_to_string", int_to_string_ty, Some(inkwell::module::Linkage::External));

    // mesh_float_to_string(val: f64) -> ptr
    let float_to_string_ty = ptr_type.fn_type(&[f64_type.into()], false);
    module.add_function("mesh_float_to_string", float_to_string_ty, Some(inkwell::module::Linkage::External));

    // mesh_bool_to_string(val: i8) -> ptr
    let bool_to_string_ty = ptr_type.fn_type(&[i8_type.into()], false);
    module.add_function("mesh_bool_to_string", bool_to_string_ty, Some(inkwell::module::Linkage::External));

    // mesh_print(s: ptr) -> void
    let print_ty = void_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_print", print_ty, Some(inkwell::module::Linkage::External));

    // mesh_println(s: ptr) -> void
    let println_ty = void_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_println", println_ty, Some(inkwell::module::Linkage::External));

    // ── Actor runtime functions ──────────────────────────────────────────

    // mesh_rt_init_actor(num_schedulers: i32) -> void
    let init_actor_ty = void_type.fn_type(&[i32_type.into()], false);
    module.add_function("mesh_rt_init_actor", init_actor_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_spawn(fn_ptr: ptr, args: ptr, args_size: i64, priority: i8) -> i64
    let spawn_ty = i64_type.fn_type(
        &[ptr_type.into(), ptr_type.into(), i64_type.into(), i8_type.into()],
        false,
    );
    module.add_function("mesh_actor_spawn", spawn_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_send(target_pid: i64, msg_ptr: ptr, msg_size: i64) -> void
    let send_ty = void_type.fn_type(
        &[i64_type.into(), ptr_type.into(), i64_type.into()],
        false,
    );
    module.add_function("mesh_actor_send", send_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_receive(timeout_ms: i64) -> ptr
    let receive_ty = ptr_type.fn_type(&[i64_type.into()], false);
    module.add_function("mesh_actor_receive", receive_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_self() -> i64
    let self_ty = i64_type.fn_type(&[], false);
    module.add_function("mesh_actor_self", self_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_link(target_pid: i64) -> void
    let link_ty = void_type.fn_type(&[i64_type.into()], false);
    module.add_function("mesh_actor_link", link_ty, Some(inkwell::module::Linkage::External));

    // mesh_reduction_check() -> void
    let reduction_ty = void_type.fn_type(&[], false);
    module.add_function("mesh_reduction_check", reduction_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_set_terminate(pid: i64, callback_fn_ptr: ptr) -> void
    let set_terminate_ty = void_type.fn_type(&[i64_type.into(), ptr_type.into()], false);
    module.add_function("mesh_actor_set_terminate", set_terminate_ty, Some(inkwell::module::Linkage::External));

    // mesh_rt_run_scheduler() -> void
    let run_scheduler_ty = void_type.fn_type(&[], false);
    module.add_function("mesh_rt_run_scheduler", run_scheduler_ty, Some(inkwell::module::Linkage::External));

    // ── Supervisor runtime functions ─────────────────────────────────────

    // mesh_supervisor_start(config_ptr: ptr, config_size: i64) -> i64 (PID)
    let sup_start_ty = i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
    module.add_function("mesh_supervisor_start", sup_start_ty, Some(inkwell::module::Linkage::External));

    // mesh_supervisor_start_child(sup_pid: i64, args_ptr: ptr, args_size: i64) -> i64
    let sup_start_child_ty = i64_type.fn_type(&[i64_type.into(), ptr_type.into(), i64_type.into()], false);
    module.add_function("mesh_supervisor_start_child", sup_start_child_ty, Some(inkwell::module::Linkage::External));

    // mesh_supervisor_terminate_child(sup_pid: i64, child_pid: i64) -> i64
    let sup_term_child_ty = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    module.add_function("mesh_supervisor_terminate_child", sup_term_child_ty, Some(inkwell::module::Linkage::External));

    // mesh_supervisor_count_children(sup_pid: i64) -> i64
    let sup_count_ty = i64_type.fn_type(&[i64_type.into()], false);
    module.add_function("mesh_supervisor_count_children", sup_count_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_trap_exit() -> void
    let trap_exit_ty = void_type.fn_type(&[], false);
    module.add_function("mesh_actor_trap_exit", trap_exit_ty, Some(inkwell::module::Linkage::External));

    // mesh_actor_exit(target_pid: i64, reason_tag: i8) -> void
    let actor_exit_ty = void_type.fn_type(&[i64_type.into(), i8_type.into()], false);
    module.add_function("mesh_actor_exit", actor_exit_ty, Some(inkwell::module::Linkage::External));

    // ── Standard library: String operations (Phase 8) ──────────────────

    // mesh_string_length(s: ptr) -> i64
    let string_length_ty = i64_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_string_length", string_length_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_slice(s: ptr, start: i64, end: i64) -> ptr
    let string_slice_ty = ptr_type.fn_type(
        &[ptr_type.into(), i64_type.into(), i64_type.into()],
        false,
    );
    module.add_function("mesh_string_slice", string_slice_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_contains(haystack: ptr, needle: ptr) -> i8
    let string_contains_ty = i8_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_string_contains", string_contains_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_starts_with(s: ptr, prefix: ptr) -> i8
    let string_starts_with_ty = i8_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_string_starts_with", string_starts_with_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_ends_with(s: ptr, suffix: ptr) -> i8
    let string_ends_with_ty = i8_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_string_ends_with", string_ends_with_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_trim(s: ptr) -> ptr
    let string_trim_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_string_trim", string_trim_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_to_upper(s: ptr) -> ptr
    let string_to_upper_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_string_to_upper", string_to_upper_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_to_lower(s: ptr) -> ptr
    let string_to_lower_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_string_to_lower", string_to_lower_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_replace(s: ptr, from: ptr, to: ptr) -> ptr
    let string_replace_ty = ptr_type.fn_type(
        &[ptr_type.into(), ptr_type.into(), ptr_type.into()],
        false,
    );
    module.add_function("mesh_string_replace", string_replace_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_eq(a: ptr, b: ptr) -> i8
    let string_eq_ty = i8_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_string_eq", string_eq_ty, Some(inkwell::module::Linkage::External));

    // Phase 46: String split/join/to_int/to_float
    // mesh_string_split(s: ptr, delim: ptr) -> ptr
    let string_split_ty = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_string_split", string_split_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_join(list: ptr, sep: ptr) -> ptr
    let string_join_ty = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_string_join", string_join_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_to_int(s: ptr) -> ptr (MeshOption)
    let string_to_int_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_string_to_int", string_to_int_ty, Some(inkwell::module::Linkage::External));

    // mesh_string_to_float(s: ptr) -> ptr (MeshOption)
    let string_to_float_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_string_to_float", string_to_float_ty, Some(inkwell::module::Linkage::External));

    // ── Standard library: File I/O functions (Phase 8) ────────────────

    // mesh_file_read(path: ptr) -> ptr (MeshResult)
    let file_read_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_file_read", file_read_ty, Some(inkwell::module::Linkage::External));

    // mesh_file_write(path: ptr, content: ptr) -> ptr (MeshResult)
    let file_write_ty = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_file_write", file_write_ty, Some(inkwell::module::Linkage::External));

    // mesh_file_append(path: ptr, content: ptr) -> ptr (MeshResult)
    let file_append_ty = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_file_append", file_append_ty, Some(inkwell::module::Linkage::External));

    // mesh_file_exists(path: ptr) -> i8
    let file_exists_ty = i8_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_file_exists", file_exists_ty, Some(inkwell::module::Linkage::External));

    // mesh_file_delete(path: ptr) -> ptr (MeshResult)
    let file_delete_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_file_delete", file_delete_ty, Some(inkwell::module::Linkage::External));

    // ── Standard library: IO functions (Phase 8) ─────────────────────

    // mesh_io_read_line() -> ptr (MeshResult)
    let io_read_line_ty = ptr_type.fn_type(&[], false);
    module.add_function("mesh_io_read_line", io_read_line_ty, Some(inkwell::module::Linkage::External));

    // mesh_io_eprintln(s: ptr) -> void
    let io_eprintln_ty = void_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_io_eprintln", io_eprintln_ty, Some(inkwell::module::Linkage::External));

    // ── Standard library: Env functions (Phase 8) ────────────────────

    // mesh_env_get(key: ptr) -> ptr (MeshOption)
    let env_get_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    module.add_function("mesh_env_get", env_get_ty, Some(inkwell::module::Linkage::External));

    // mesh_env_args() -> ptr (packed array)
    let env_args_ty = ptr_type.fn_type(&[], false);
    module.add_function("mesh_env_args", env_args_ty, Some(inkwell::module::Linkage::External));

    // ── Standard library: Collection functions (Phase 8 Plan 02) ──────────

    // List functions
    module.add_function("mesh_list_new", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_length", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_append", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_head", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_tail", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_get", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_concat", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_reverse", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_map", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_filter", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_reduce", i64_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_from_array", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_builder_new", ptr_type.fn_type(&[i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_builder_push", void_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // Phase 46: List sort, find, any, all, contains
    // mesh_list_sort(list: ptr, fn_ptr: ptr, env_ptr: ptr) -> ptr
    module.add_function("mesh_list_sort", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_list_find(list: ptr, fn_ptr: ptr, env_ptr: ptr) -> ptr (MeshOption)
    module.add_function("mesh_list_find", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_list_any(list: ptr, fn_ptr: ptr, env_ptr: ptr) -> i8 (Bool)
    module.add_function("mesh_list_any", i8_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_list_all(list: ptr, fn_ptr: ptr, env_ptr: ptr) -> i8 (Bool)
    module.add_function("mesh_list_all", i8_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_list_contains(list: ptr, elem: i64) -> i8 (Bool)
    module.add_function("mesh_list_contains", i8_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // Phase 47: List zip, flat_map, flatten, enumerate, take, drop, last, nth
    module.add_function("mesh_list_zip", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_flat_map", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_flatten", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_enumerate", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_take", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_drop", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_last", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_list_nth", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // Map functions
    module.add_function("mesh_map_new", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_new_typed", ptr_type.fn_type(&[i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_tag_string", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_put", ptr_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_get", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_has_key", i8_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_delete", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_size", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_keys", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_values", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // Phase 47: Map merge/to_list/from_list
    module.add_function("mesh_map_merge", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_to_list", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_from_list", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_entry_key", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_map_entry_value", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // Set functions
    module.add_function("mesh_set_new", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_add", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_remove", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_contains", i8_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_size", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_union", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_intersection", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_element_at", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    // Phase 47: Set difference/to_list/from_list
    module.add_function("mesh_set_difference", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_to_list", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_set_from_list", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // Tuple functions
    module.add_function("mesh_tuple_nth", i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_tuple_first", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_tuple_second", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_tuple_size", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // Range functions
    module.add_function("mesh_range_new", ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_range_to_list", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_range_map", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_range_filter", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_range_length", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // Queue functions
    module.add_function("mesh_queue_new", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_queue_push", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_queue_pop", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_queue_peek", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_queue_size", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    module.add_function("mesh_queue_is_empty", i8_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Standard library: JSON functions (Phase 8 Plan 04) ──────────────

    // mesh_json_parse(input: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_parse", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_encode(json: ptr) -> ptr (MeshString)
    module.add_function("mesh_json_encode", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_encode_string(s: ptr) -> ptr (MeshString)
    module.add_function("mesh_json_encode_string", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_encode_int(val: i64) -> ptr (MeshString)
    module.add_function("mesh_json_encode_int", ptr_type.fn_type(&[i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_encode_bool(val: i8) -> ptr (MeshString)
    module.add_function("mesh_json_encode_bool", ptr_type.fn_type(&[i8_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_encode_map(map: ptr) -> ptr (MeshString)
    module.add_function("mesh_json_encode_map", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_encode_list(list: ptr) -> ptr (MeshString)
    module.add_function("mesh_json_encode_list", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_from_int(val: i64) -> ptr
    module.add_function("mesh_json_from_int", ptr_type.fn_type(&[i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_from_float(val: f64) -> ptr
    module.add_function("mesh_json_from_float", ptr_type.fn_type(&[f64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_from_bool(val: i8) -> ptr
    module.add_function("mesh_json_from_bool", ptr_type.fn_type(&[i8_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_json_from_string(s: ptr) -> ptr
    module.add_function("mesh_json_from_string", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 103: JSON field extraction (no DB roundtrip) ────────────
    // mesh_json_get(json: ptr, key: ptr) -> ptr (MeshString)
    module.add_function("mesh_json_get", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_get_nested(json: ptr, path1: ptr, path2: ptr) -> ptr (MeshString)
    module.add_function("mesh_json_get_nested", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Structured JSON object/array functions (Phase 49) ──────────────
    // mesh_json_object_new() -> ptr
    module.add_function("mesh_json_object_new", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    // mesh_json_object_put(obj: ptr, key: ptr, val: ptr) -> ptr
    module.add_function("mesh_json_object_put", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_object_get(obj: ptr, key: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_object_get", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_array_new() -> ptr
    module.add_function("mesh_json_array_new", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    // mesh_json_array_push(arr: ptr, val: ptr) -> ptr
    module.add_function("mesh_json_array_push", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_array_get(arr: ptr, index: i64) -> ptr (MeshResult)
    module.add_function("mesh_json_array_get", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_as_int(json: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_as_int", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_as_float(json: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_as_float", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_as_string(json: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_as_string", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_as_bool(json: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_as_bool", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_null() -> ptr
    module.add_function("mesh_json_null", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));
    // mesh_json_from_list(list: ptr, elem_fn: ptr) -> ptr
    module.add_function("mesh_json_from_list", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_from_map(map: ptr, val_fn: ptr) -> ptr
    module.add_function("mesh_json_from_map", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_to_list(json_arr: ptr, elem_fn: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_to_list", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_json_to_map(json_obj: ptr, val_fn: ptr) -> ptr (MeshResult)
    module.add_function("mesh_json_to_map", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Result helpers (Phase 49: from_json Result propagation) ─────────
    // mesh_alloc_result(tag: i64, value: ptr) -> ptr
    module.add_function("mesh_alloc_result", ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_result_is_ok(result: ptr) -> i64
    module.add_function("mesh_result_is_ok", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_result_unwrap(result: ptr) -> ptr
    module.add_function("mesh_result_unwrap", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Standard library: HTTP functions (Phase 8 Plan 05) ──────────────

    // mesh_http_router() -> ptr
    module.add_function("mesh_http_router", ptr_type.fn_type(&[], false), Some(inkwell::module::Linkage::External));

    // mesh_http_route(router: ptr, pattern: ptr, handler_fn: ptr) -> ptr
    module.add_function("mesh_http_route", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_serve(router: ptr, port: i64) -> void
    module.add_function("mesh_http_serve", void_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_serve_tls(router: ptr, port: i64, cert_path: ptr, key_path: ptr) -> void
    module.add_function("mesh_http_serve_tls", void_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── WebSocket functions (Phase 60) ──────────────────────────────────
    // mesh_ws_serve(on_connect_fn: ptr, on_connect_env: ptr, on_message_fn: ptr, on_message_env: ptr, on_close_fn: ptr, on_close_env: ptr, port: i64) -> void
    module.add_function("mesh_ws_serve", void_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_ws_send(conn: ptr, msg: ptr) -> i64
    module.add_function("mesh_ws_send", i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_ws_send_binary(conn: ptr, data: ptr, len: i64) -> i64
    module.add_function("mesh_ws_send_binary", i64_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_ws_serve_tls(on_connect_fn: ptr, on_connect_env: ptr, on_message_fn: ptr, on_message_env: ptr, on_close_fn: ptr, on_close_env: ptr, port: i64, cert_path: ptr, key_path: ptr) -> void
    module.add_function("mesh_ws_serve_tls", void_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), i64_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── WebSocket Room functions (Phase 62) ──────────────────────────────
    // mesh_ws_join(conn: ptr, room: ptr) -> i64
    module.add_function("mesh_ws_join", i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_ws_leave(conn: ptr, room: ptr) -> i64
    module.add_function("mesh_ws_leave", i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_ws_broadcast(room: ptr, msg: ptr) -> i64
    module.add_function("mesh_ws_broadcast", i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_ws_broadcast_except(room: ptr, msg: ptr, except_conn: ptr) -> i64
    module.add_function("mesh_ws_broadcast_except", i64_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_response_new(status: i64, body: ptr) -> ptr
    module.add_function("mesh_http_response_new", ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_response_with_headers(status: i64, body: ptr, headers: ptr) -> ptr
    module.add_function("mesh_http_response_with_headers", ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_get(url: ptr) -> ptr (MeshResult)
    module.add_function("mesh_http_get", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_post(url: ptr, body: ptr) -> ptr (MeshResult)
    module.add_function("mesh_http_post", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_request_method(req: ptr) -> ptr
    module.add_function("mesh_http_request_method", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_request_path(req: ptr) -> ptr
    module.add_function("mesh_http_request_path", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_request_body(req: ptr) -> ptr
    module.add_function("mesh_http_request_body", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_request_header(req: ptr, name: ptr) -> ptr (MeshOption)
    module.add_function("mesh_http_request_header", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_request_query(req: ptr, name: ptr) -> ptr (MeshOption)
    module.add_function("mesh_http_request_query", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 51: Method-specific routing and path parameter extraction ──

    // mesh_http_route_get(router: ptr, pattern: ptr, handler_fn: ptr) -> ptr
    module.add_function("mesh_http_route_get", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_route_post(router: ptr, pattern: ptr, handler_fn: ptr) -> ptr
    module.add_function("mesh_http_route_post", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_route_put(router: ptr, pattern: ptr, handler_fn: ptr) -> ptr
    module.add_function("mesh_http_route_put", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_route_delete(router: ptr, pattern: ptr, handler_fn: ptr) -> ptr
    module.add_function("mesh_http_route_delete", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_http_request_param(req: ptr, name: ptr) -> ptr (MeshOption)
    module.add_function("mesh_http_request_param", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 52: Middleware ──────────────────────────────────────────────

    // mesh_http_use_middleware(router: ptr, middleware_fn: ptr) -> ptr
    module.add_function("mesh_http_use_middleware", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 53: SQLite ──────────────────────────────────────────────

    // mesh_sqlite_open(path: ptr) -> ptr (MeshResult)
    module.add_function("mesh_sqlite_open",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_sqlite_close(conn: i64) -> void
    module.add_function("mesh_sqlite_close",
        void_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_sqlite_execute(conn: i64, sql: ptr, params: ptr) -> ptr (MeshResult)
    module.add_function("mesh_sqlite_execute",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_sqlite_query(conn: i64, sql: ptr, params: ptr) -> ptr (MeshResult)
    module.add_function("mesh_sqlite_query",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 54: PostgreSQL ──────────────────────────────────────────────

    // mesh_pg_connect(url: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pg_connect",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_close(conn: i64) -> void
    module.add_function("mesh_pg_close",
        void_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_execute(conn: i64, sql: ptr, params: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pg_execute",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_query(conn: i64, sql: ptr, params: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pg_query",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 57: PostgreSQL Transactions ──────────────────────────────

    // mesh_pg_begin(conn: i64) -> ptr (MeshResult)
    module.add_function("mesh_pg_begin",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_commit(conn: i64) -> ptr (MeshResult)
    module.add_function("mesh_pg_commit",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_rollback(conn: i64) -> ptr (MeshResult)
    module.add_function("mesh_pg_rollback",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_transaction(conn: i64, fn_ptr: ptr, env_ptr: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pg_transaction",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 57: SQLite Transactions ──────────────────────────────────

    // mesh_sqlite_begin(conn: i64) -> ptr (MeshResult)
    module.add_function("mesh_sqlite_begin",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_sqlite_commit(conn: i64) -> ptr (MeshResult)
    module.add_function("mesh_sqlite_commit",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_sqlite_rollback(conn: i64) -> ptr (MeshResult)
    module.add_function("mesh_sqlite_rollback",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 57: Connection Pool ──────────────────────────────────────

    // mesh_pool_open(url: ptr, min: i64, max: i64, timeout: i64) -> ptr (MeshResult)
    module.add_function("mesh_pool_open",
        ptr_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pool_close(pool: i64) -> void
    module.add_function("mesh_pool_close",
        void_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pool_checkout(pool: i64) -> ptr (MeshResult)
    module.add_function("mesh_pool_checkout",
        ptr_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pool_checkin(pool: i64, conn: i64) -> void
    module.add_function("mesh_pool_checkin",
        void_type.fn_type(&[i64_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pool_query(pool: i64, sql: ptr, params: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pool_query",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pool_execute(pool: i64, sql: ptr, params: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pool_execute",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 58: Row Parsing & Struct-to-Row Mapping ────────────────────

    // mesh_row_from_row_get(row: ptr, col_name: ptr) -> ptr (MeshResult)
    module.add_function("mesh_row_from_row_get",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_row_parse_int(s: ptr) -> ptr (MeshResult)
    module.add_function("mesh_row_parse_int",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_row_parse_float(s: ptr) -> ptr (MeshResult)
    module.add_function("mesh_row_parse_float",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_row_parse_bool(s: ptr) -> ptr (MeshResult)
    module.add_function("mesh_row_parse_bool",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pg_query_as(conn: i64, sql: ptr, params: ptr, from_row_fn: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pg_query_as",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_pool_query_as(pool: i64, sql: ptr, params: ptr, from_row_fn: ptr) -> ptr (MeshResult)
    module.add_function("mesh_pool_query_as",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Hash runtime functions (Phase 21 Plan 01) ──────────────────────

    // mesh_hash_int(value: i64) -> i64
    module.add_function("mesh_hash_int", i64_type.fn_type(&[i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_hash_float(value: f64) -> i64
    module.add_function("mesh_hash_float", i64_type.fn_type(&[f64_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_hash_bool(value: i8) -> i64
    module.add_function("mesh_hash_bool", i64_type.fn_type(&[i8_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_hash_string(s: ptr) -> i64
    module.add_function("mesh_hash_string", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_hash_combine(hash_a: i64, hash_b: i64) -> i64
    module.add_function("mesh_hash_combine", i64_type.fn_type(&[i64_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Collection Display runtime functions (Phase 21 Plan 04) ──────────

    // mesh_list_to_string(list: ptr, elem_to_str: ptr) -> ptr
    module.add_function("mesh_list_to_string", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_map_to_string(map: ptr, key_to_str: ptr, val_to_str: ptr) -> ptr
    module.add_function("mesh_map_to_string", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_set_to_string(set: ptr, elem_to_str: ptr) -> ptr
    module.add_function("mesh_set_to_string", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_string_to_string(val: u64) -> ptr (identity for string elements in collections)
    module.add_function("mesh_string_to_string", ptr_type.fn_type(&[i64_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── List Eq/Ord runtime functions (Phase 27 Plan 01) ──────────────

    // mesh_list_eq(list_a: ptr, list_b: ptr, elem_eq: ptr) -> i8
    module.add_function("mesh_list_eq", i8_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // mesh_list_compare(list_a: ptr, list_b: ptr, elem_cmp: ptr) -> i64
    module.add_function("mesh_list_compare", i64_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Service runtime functions (Phase 9 Plan 03) ──────────────────────

    // mesh_service_call(target_pid: i64, msg_tag: i64, payload_ptr: ptr, payload_size: i64) -> ptr
    let service_call_ty = ptr_type.fn_type(
        &[i64_type.into(), i64_type.into(), ptr_type.into(), i64_type.into()],
        false,
    );
    module.add_function("mesh_service_call", service_call_ty, Some(inkwell::module::Linkage::External));

    // mesh_service_reply(caller_pid: i64, reply_ptr: ptr, reply_size: i64) -> void
    let service_reply_ty = void_type.fn_type(
        &[i64_type.into(), ptr_type.into(), i64_type.into()],
        false,
    );
    module.add_function("mesh_service_reply", service_reply_ty, Some(inkwell::module::Linkage::External));

    // ── Job runtime functions (Phase 9 Plan 04) ──────────────────────────

    // mesh_job_async(fn_ptr: ptr, env_ptr: ptr) -> i64 (PID)
    let job_async_ty = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_job_async", job_async_ty, Some(inkwell::module::Linkage::External));

    // mesh_job_await(job_pid: i64) -> ptr (MeshResult)
    let job_await_ty = ptr_type.fn_type(&[i64_type.into()], false);
    module.add_function("mesh_job_await", job_await_ty, Some(inkwell::module::Linkage::External));

    // mesh_job_await_timeout(job_pid: i64, timeout_ms: i64) -> ptr (MeshResult)
    let job_await_timeout_ty = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    module.add_function("mesh_job_await_timeout", job_await_timeout_ty, Some(inkwell::module::Linkage::External));

    // mesh_job_map(list_ptr: ptr, fn_ptr: ptr, env_ptr: ptr) -> ptr (List of MeshResult)
    let job_map_ty = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false);
    module.add_function("mesh_job_map", job_map_ty, Some(inkwell::module::Linkage::External));

    // ── Timer functions (Phase 44 Plan 02) ──────────────────────────────

    // mesh_timer_sleep(ms: i64) -> void
    let timer_sleep_ty = void_type.fn_type(&[i64_type.into()], false);
    module.add_function("mesh_timer_sleep", timer_sleep_ty, Some(inkwell::module::Linkage::External));

    // mesh_timer_send_after(pid: i64, ms: i64, msg_ptr: ptr, msg_size: i64) -> void
    let timer_send_after_ty = void_type.fn_type(&[i64_type.into(), i64_type.into(), ptr_type.into(), i64_type.into()], false);
    module.add_function("mesh_timer_send_after", timer_send_after_ty, Some(inkwell::module::Linkage::External));

    // mesh_panic(msg: ptr, msg_len: u64, file: ptr, file_len: u64, line: u32) -> void
    // (noreturn -- marked via attribute)
    let panic_params: Vec<BasicMetadataTypeEnum<'ctx>> = vec![
        ptr_type.into(),     // msg
        i64_type.into(),     // msg_len
        ptr_type.into(),     // file
        i64_type.into(),     // file_len
        i32_type.into(),     // line
    ];
    let panic_ty = void_type.fn_type(&panic_params, false);
    let panic_fn = module.add_function("mesh_panic", panic_ty, Some(inkwell::module::Linkage::External));
    // Mark as noreturn
    panic_fn.add_attribute(
        inkwell::attributes::AttributeLoc::Function,
        context.create_enum_attribute(
            inkwell::attributes::Attribute::get_named_enum_kind_id("noreturn"),
            0,
        ),
    );

    // ── Phase 67: Node distribution & remote spawn ──────────────────────

    // mesh_node_start(name_ptr: ptr, name_len: i64, cookie_ptr: ptr, cookie_len: i64) -> i64
    module.add_function("mesh_node_start",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_node_connect(name_ptr: ptr, name_len: i64) -> i64
    module.add_function("mesh_node_connect",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_node_self() -> ptr
    module.add_function("mesh_node_self",
        ptr_type.fn_type(&[], false),
        Some(inkwell::module::Linkage::External));

    // mesh_node_list() -> ptr
    module.add_function("mesh_node_list",
        ptr_type.fn_type(&[], false),
        Some(inkwell::module::Linkage::External));

    // mesh_node_monitor(node_ptr: ptr, node_len: i64) -> i64
    module.add_function("mesh_node_monitor",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_node_spawn(node_ptr: ptr, node_len: i64, fn_name_ptr: ptr, fn_name_len: i64, args_ptr: ptr, args_size: i64, link_flag: i8) -> i64
    module.add_function("mesh_node_spawn",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into(), i64_type.into(), ptr_type.into(), i64_type.into(), i8_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_register_function(name_ptr: ptr, name_len: i64, fn_ptr: ptr) -> void
    module.add_function("mesh_register_function",
        void_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_process_monitor(target_pid: i64) -> i64
    module.add_function("mesh_process_monitor",
        i64_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_process_demonitor(monitor_ref: i64) -> i64
    module.add_function("mesh_process_demonitor",
        i64_type.fn_type(&[i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_process_register(name: ptr, pid: i64) -> i64
    module.add_function("mesh_process_register",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_process_whereis(name: ptr) -> i64
    module.add_function("mesh_process_whereis",
        i64_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_actor_send_named(name_ptr: ptr, name_len: i64, node_ptr: ptr, node_len: i64, msg_ptr: ptr, msg_size: i64) -> void
    module.add_function("mesh_actor_send_named",
        void_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into(), i64_type.into(), ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 76: Iterator runtime functions ──────────────────────────────
    // mesh_list_iter_new(list: ptr) -> ptr
    module.add_function("mesh_list_iter_new", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_list_iter_next(iter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_list_iter_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_map_iter_new(map: ptr) -> ptr
    module.add_function("mesh_map_iter_new", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_map_iter_next(iter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_map_iter_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_set_iter_new(set: ptr) -> ptr
    module.add_function("mesh_set_iter_new", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_set_iter_next(iter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_set_iter_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_range_iter_new(start: i64, end: i64) -> ptr
    module.add_function("mesh_range_iter_new", ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_range_iter_next(iter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_range_iter_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_from(collection: ptr) -> ptr (Iter.from entry point)
    module.add_function("mesh_iter_from", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 78: Lazy Combinators & Terminals ──────────────────────────
    // Combinators: adapter constructors
    // mesh_iter_map(source: ptr, fn_ptr: ptr, env_ptr: ptr) -> ptr
    module.add_function("mesh_iter_map", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_filter(source: ptr, fn_ptr: ptr, env_ptr: ptr) -> ptr
    module.add_function("mesh_iter_filter", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_take(source: ptr, n: i64) -> ptr
    module.add_function("mesh_iter_take", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_skip(source: ptr, n: i64) -> ptr
    module.add_function("mesh_iter_skip", ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_enumerate(source: ptr) -> ptr
    module.add_function("mesh_iter_enumerate", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_zip(source_a: ptr, source_b: ptr) -> ptr
    module.add_function("mesh_iter_zip", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // Terminals
    // mesh_iter_count(iter: ptr) -> i64
    module.add_function("mesh_iter_count", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_sum(iter: ptr) -> i64
    module.add_function("mesh_iter_sum", i64_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_any(iter: ptr, fn_ptr: ptr, env_ptr: ptr) -> i8 (Bool)
    module.add_function("mesh_iter_any", i8_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_all(iter: ptr, fn_ptr: ptr, env_ptr: ptr) -> i8 (Bool)
    module.add_function("mesh_iter_all", i8_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_find(iter: ptr, fn_ptr: ptr, env_ptr: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_find", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_reduce(iter: ptr, init: i64, fn_ptr: ptr, env_ptr: ptr) -> i64
    module.add_function("mesh_iter_reduce", i64_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into(), ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // Adapter _next functions (for resolve_iterator_fn dispatch)
    // mesh_iter_generic_next(iter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_generic_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_map_next(adapter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_map_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_filter_next(adapter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_filter_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_take_next(adapter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_take_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_skip_next(adapter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_skip_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_enumerate_next(adapter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_enumerate_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_iter_zip_next(adapter: ptr) -> ptr (MeshOption)
    module.add_function("mesh_iter_zip_next", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 79: Collect terminal operations ────────────────────────────
    // mesh_list_collect(iter: ptr) -> ptr
    module.add_function("mesh_list_collect", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_map_collect(iter: ptr) -> ptr
    module.add_function("mesh_map_collect", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_map_collect_string_keys(iter: ptr) -> ptr (Phase 96: string key variant)
    module.add_function("mesh_map_collect_string_keys", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_set_collect(iter: ptr) -> ptr
    module.add_function("mesh_set_collect", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));
    // mesh_string_collect(iter: ptr) -> ptr
    module.add_function("mesh_string_collect", ptr_type.fn_type(&[ptr_type.into()], false), Some(inkwell::module::Linkage::External));

    // ── Phase 97: ORM SQL Generation ──────────────────────────────────

    // mesh_orm_build_select(table: ptr, columns: ptr, where_clauses: ptr, order_by: ptr, limit: i64, offset: i64) -> ptr
    module.add_function("mesh_orm_build_select",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), i64_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_orm_build_insert(table: ptr, columns: ptr, returning: ptr) -> ptr
    module.add_function("mesh_orm_build_insert",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_orm_build_update(table: ptr, set_columns: ptr, where_clauses: ptr, returning: ptr) -> ptr
    module.add_function("mesh_orm_build_update",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_orm_build_delete(table: ptr, where_clauses: ptr, returning: ptr) -> ptr
    module.add_function("mesh_orm_build_delete",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 98: Query Builder ───────────────────────────────────────

    // mesh_query_from(table: ptr) -> ptr
    module.add_function("mesh_query_from",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where(q: ptr, field: ptr, value: ptr) -> ptr
    module.add_function("mesh_query_where",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_op(q: ptr, field: ptr, op: ptr, value: ptr) -> ptr
    module.add_function("mesh_query_where_op",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_in(q: ptr, field: ptr, values: ptr) -> ptr
    module.add_function("mesh_query_where_in",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_null(q: ptr, field: ptr) -> ptr
    module.add_function("mesh_query_where_null",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_not_null(q: ptr, field: ptr) -> ptr
    module.add_function("mesh_query_where_not_null",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_not_in(q: ptr, field: ptr, values: ptr) -> ptr
    module.add_function("mesh_query_where_not_in",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_between(q: ptr, field: ptr, low: ptr, high: ptr) -> ptr
    module.add_function("mesh_query_where_between",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_or(q: ptr, fields: ptr, values: ptr) -> ptr
    module.add_function("mesh_query_where_or",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_select(q: ptr, fields: ptr) -> ptr
    module.add_function("mesh_query_select",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_order_by(q: ptr, field: ptr, direction: ptr) -> ptr
    module.add_function("mesh_query_order_by",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_limit(q: ptr, n: i64) -> ptr
    module.add_function("mesh_query_limit",
        ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_offset(q: ptr, n: i64) -> ptr
    module.add_function("mesh_query_offset",
        ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_join(q: ptr, type: ptr, table: ptr, on_clause: ptr) -> ptr
    module.add_function("mesh_query_join",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_join_as(q: ptr, type: ptr, table: ptr, alias: ptr, on_clause: ptr) -> ptr
    module.add_function("mesh_query_join_as",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_group_by(q: ptr, field: ptr) -> ptr
    module.add_function("mesh_query_group_by",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_having(q: ptr, clause: ptr, value: ptr) -> ptr
    module.add_function("mesh_query_having",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_fragment(q: ptr, sql: ptr, params: ptr) -> ptr
    module.add_function("mesh_query_fragment",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 103: Query Builder Raw Extensions ──────────────────────

    // mesh_query_select_raw(q: ptr, expressions: ptr) -> ptr
    module.add_function("mesh_query_select_raw",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_where_raw(q: ptr, clause: ptr, params: ptr) -> ptr
    module.add_function("mesh_query_where_raw",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 106: Raw ORDER BY / GROUP BY ───────────────────────────

    // mesh_query_order_by_raw(q: ptr, expression: ptr) -> ptr
    module.add_function("mesh_query_order_by_raw",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_query_group_by_raw(q: ptr, expression: ptr) -> ptr
    module.add_function("mesh_query_group_by_raw",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 98: Repo Read Operations ───────────────────────────────

    // mesh_repo_all(pool: i64, query: ptr) -> ptr
    module.add_function("mesh_repo_all",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_one(pool: i64, query: ptr) -> ptr
    module.add_function("mesh_repo_one",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_get(pool: i64, table: ptr, id: ptr) -> ptr
    module.add_function("mesh_repo_get",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_get_by(pool: i64, table: ptr, field: ptr, value: ptr) -> ptr
    module.add_function("mesh_repo_get_by",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_count(pool: i64, query: ptr) -> ptr
    module.add_function("mesh_repo_count",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_exists(pool: i64, query: ptr) -> ptr
    module.add_function("mesh_repo_exists",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 98: Repo Write Operations ─────────────────────────────────

    // mesh_repo_insert(pool: i64, table: ptr, fields: ptr) -> ptr
    module.add_function("mesh_repo_insert",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_update(pool: i64, table: ptr, id: ptr, fields: ptr) -> ptr
    module.add_function("mesh_repo_update",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_delete(pool: i64, table: ptr, id: ptr) -> ptr
    module.add_function("mesh_repo_delete",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_transaction(pool: i64, fn_ptr: ptr, env_ptr: ptr) -> ptr
    module.add_function("mesh_repo_transaction",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 103: Extended Repo Write Operations ──────────────────────

    // mesh_repo_update_where(pool: i64, table: ptr, fields: ptr, query: ptr) -> ptr
    module.add_function("mesh_repo_update_where",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_delete_where(pool: i64, table: ptr, query: ptr) -> ptr
    module.add_function("mesh_repo_delete_where",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_query_raw(pool: i64, sql: ptr, params: ptr) -> ptr
    module.add_function("mesh_repo_query_raw",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_execute_raw(pool: i64, sql: ptr, params: ptr) -> ptr
    module.add_function("mesh_repo_execute_raw",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 100: Repo Preloading ─────────────────────────────────────

    // mesh_repo_preload(pool: i64, rows: ptr, associations: ptr, rel_meta: ptr) -> ptr
    module.add_function("mesh_repo_preload",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 99: Repo Changeset Operations ────────────────────────────

    // mesh_repo_insert_changeset(pool: i64, table: ptr, changeset: ptr) -> ptr
    module.add_function("mesh_repo_insert_changeset",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_repo_update_changeset(pool: i64, table: ptr, id: ptr, changeset: ptr) -> ptr
    module.add_function("mesh_repo_update_changeset",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 99: Changeset Operations ────────────────────────────────

    // mesh_changeset_cast(data: ptr, params: ptr, allowed: ptr) -> ptr
    module.add_function("mesh_changeset_cast",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_cast_with_types(data: ptr, params: ptr, allowed: ptr, field_types: ptr) -> ptr
    module.add_function("mesh_changeset_cast_with_types",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_validate_required(cs: ptr, fields: ptr) -> ptr
    module.add_function("mesh_changeset_validate_required",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_validate_length(cs: ptr, field: ptr, min: i64, max: i64) -> ptr
    module.add_function("mesh_changeset_validate_length",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_validate_format(cs: ptr, field: ptr, pattern: ptr) -> ptr
    module.add_function("mesh_changeset_validate_format",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_validate_inclusion(cs: ptr, field: ptr, allowed_values: ptr) -> ptr
    module.add_function("mesh_changeset_validate_inclusion",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_validate_number(cs: ptr, field: ptr, gt: i64, lt: i64, gte: i64, lte: i64) -> ptr
    module.add_function("mesh_changeset_validate_number",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_valid(cs: ptr) -> ptr
    module.add_function("mesh_changeset_valid",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_errors(cs: ptr) -> ptr
    module.add_function("mesh_changeset_errors",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_changes(cs: ptr) -> ptr
    module.add_function("mesh_changeset_changes",
        ptr_type.fn_type(&[ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_get_change(cs: ptr, field: ptr) -> ptr
    module.add_function("mesh_changeset_get_change",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_changeset_get_error(cs: ptr, field: ptr) -> ptr
    module.add_function("mesh_changeset_get_error",
        ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 101: Migration DDL Operations ────────────────────────────

    // mesh_migration_create_table(pool: i64, table: ptr, columns: ptr) -> ptr
    module.add_function("mesh_migration_create_table",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_drop_table(pool: i64, table: ptr) -> ptr
    module.add_function("mesh_migration_drop_table",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_add_column(pool: i64, table: ptr, col_def: ptr) -> ptr
    module.add_function("mesh_migration_add_column",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_drop_column(pool: i64, table: ptr, col: ptr) -> ptr
    module.add_function("mesh_migration_drop_column",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_rename_column(pool: i64, table: ptr, old: ptr, new: ptr) -> ptr
    module.add_function("mesh_migration_rename_column",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_create_index(pool: i64, table: ptr, cols: ptr, opts: ptr) -> ptr
    module.add_function("mesh_migration_create_index",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_drop_index(pool: i64, table: ptr, cols: ptr) -> ptr
    module.add_function("mesh_migration_drop_index",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_migration_execute(pool: i64, sql: ptr) -> ptr
    module.add_function("mesh_migration_execute",
        ptr_type.fn_type(&[i64_type.into(), ptr_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // ── Phase 68: Global Registry ──────────────────────────────────────

    // mesh_global_register(name_ptr: ptr, name_len: i64, pid: i64) -> i64
    module.add_function("mesh_global_register",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_global_whereis(name_ptr: ptr, name_len: i64) -> i64
    module.add_function("mesh_global_whereis",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));

    // mesh_global_unregister(name_ptr: ptr, name_len: i64) -> i64
    module.add_function("mesh_global_unregister",
        i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
        Some(inkwell::module::Linkage::External));
}

/// Get a runtime function by name from the module.
///
/// Panics if the function was not declared (call `declare_intrinsics` first).
pub fn get_intrinsic<'ctx>(module: &Module<'ctx>, name: &str) -> FunctionValue<'ctx> {
    module
        .get_function(name)
        .unwrap_or_else(|| panic!("Runtime function '{}' not declared", name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    #[test]
    fn test_declare_all_intrinsics() {
        let context = Context::create();
        let module = context.create_module("test");
        declare_intrinsics(&module);

        // Verify all expected functions exist
        assert!(module.get_function("mesh_rt_init").is_some());
        assert!(module.get_function("mesh_gc_alloc_actor").is_some());
        assert!(module.get_function("mesh_string_new").is_some());
        assert!(module.get_function("mesh_string_concat").is_some());
        assert!(module.get_function("mesh_int_to_string").is_some());
        assert!(module.get_function("mesh_float_to_string").is_some());
        assert!(module.get_function("mesh_bool_to_string").is_some());
        assert!(module.get_function("mesh_print").is_some());
        assert!(module.get_function("mesh_println").is_some());
        assert!(module.get_function("mesh_panic").is_some());

        // Actor runtime functions
        assert!(module.get_function("mesh_rt_init_actor").is_some());
        assert!(module.get_function("mesh_actor_spawn").is_some());
        assert!(module.get_function("mesh_actor_send").is_some());
        assert!(module.get_function("mesh_actor_receive").is_some());
        assert!(module.get_function("mesh_actor_self").is_some());
        assert!(module.get_function("mesh_actor_link").is_some());
        assert!(module.get_function("mesh_reduction_check").is_some());
        assert!(module.get_function("mesh_actor_set_terminate").is_some());
        assert!(module.get_function("mesh_rt_run_scheduler").is_some());

        // Supervisor runtime functions
        assert!(module.get_function("mesh_supervisor_start").is_some());
        assert!(module.get_function("mesh_supervisor_start_child").is_some());
        assert!(module.get_function("mesh_supervisor_terminate_child").is_some());
        assert!(module.get_function("mesh_supervisor_count_children").is_some());
        assert!(module.get_function("mesh_actor_trap_exit").is_some());
        assert!(module.get_function("mesh_actor_exit").is_some());

        // Standard library functions (Phase 8)
        assert!(module.get_function("mesh_file_read").is_some());
        assert!(module.get_function("mesh_file_write").is_some());
        assert!(module.get_function("mesh_file_append").is_some());
        assert!(module.get_function("mesh_file_exists").is_some());
        assert!(module.get_function("mesh_file_delete").is_some());
        assert!(module.get_function("mesh_string_length").is_some());
        assert!(module.get_function("mesh_string_slice").is_some());
        assert!(module.get_function("mesh_string_contains").is_some());
        assert!(module.get_function("mesh_string_starts_with").is_some());
        assert!(module.get_function("mesh_string_ends_with").is_some());
        assert!(module.get_function("mesh_string_trim").is_some());
        assert!(module.get_function("mesh_string_to_upper").is_some());
        assert!(module.get_function("mesh_string_to_lower").is_some());
        assert!(module.get_function("mesh_string_replace").is_some());
        assert!(module.get_function("mesh_string_eq").is_some());
        assert!(module.get_function("mesh_string_split").is_some());
        assert!(module.get_function("mesh_string_join").is_some());
        assert!(module.get_function("mesh_string_to_int").is_some());
        assert!(module.get_function("mesh_string_to_float").is_some());
        assert!(module.get_function("mesh_io_read_line").is_some());
        assert!(module.get_function("mesh_io_eprintln").is_some());
        assert!(module.get_function("mesh_env_get").is_some());
        assert!(module.get_function("mesh_env_args").is_some());

        // Collection functions (Phase 8 Plan 02)
        assert!(module.get_function("mesh_list_new").is_some());
        assert!(module.get_function("mesh_list_length").is_some());
        assert!(module.get_function("mesh_list_append").is_some());
        assert!(module.get_function("mesh_list_head").is_some());
        assert!(module.get_function("mesh_list_tail").is_some());
        assert!(module.get_function("mesh_list_get").is_some());
        assert!(module.get_function("mesh_list_concat").is_some());
        assert!(module.get_function("mesh_list_reverse").is_some());
        assert!(module.get_function("mesh_list_map").is_some());
        assert!(module.get_function("mesh_list_filter").is_some());
        assert!(module.get_function("mesh_list_reduce").is_some());
        assert!(module.get_function("mesh_list_from_array").is_some());
        assert!(module.get_function("mesh_list_builder_new").is_some());
        assert!(module.get_function("mesh_list_builder_push").is_some());
        // Phase 46: List sort, find, any, all, contains
        assert!(module.get_function("mesh_list_sort").is_some());
        assert!(module.get_function("mesh_list_find").is_some());
        assert!(module.get_function("mesh_list_any").is_some());
        assert!(module.get_function("mesh_list_all").is_some());
        assert!(module.get_function("mesh_list_contains").is_some());
        // Phase 47: List zip, flat_map, flatten, enumerate, take, drop, last, nth
        assert!(module.get_function("mesh_list_zip").is_some());
        assert!(module.get_function("mesh_list_flat_map").is_some());
        assert!(module.get_function("mesh_list_flatten").is_some());
        assert!(module.get_function("mesh_list_enumerate").is_some());
        assert!(module.get_function("mesh_list_take").is_some());
        assert!(module.get_function("mesh_list_drop").is_some());
        assert!(module.get_function("mesh_list_last").is_some());
        assert!(module.get_function("mesh_list_nth").is_some());
        assert!(module.get_function("mesh_map_new").is_some());
        assert!(module.get_function("mesh_map_new_typed").is_some());
        assert!(module.get_function("mesh_map_put").is_some());
        assert!(module.get_function("mesh_map_get").is_some());
        assert!(module.get_function("mesh_map_has_key").is_some());
        assert!(module.get_function("mesh_map_delete").is_some());
        assert!(module.get_function("mesh_map_size").is_some());
        assert!(module.get_function("mesh_map_keys").is_some());
        assert!(module.get_function("mesh_map_values").is_some());
        assert!(module.get_function("mesh_map_merge").is_some());
        assert!(module.get_function("mesh_map_to_list").is_some());
        assert!(module.get_function("mesh_map_from_list").is_some());
        assert!(module.get_function("mesh_map_entry_key").is_some());
        assert!(module.get_function("mesh_map_entry_value").is_some());
        assert!(module.get_function("mesh_set_new").is_some());
        assert!(module.get_function("mesh_set_add").is_some());
        assert!(module.get_function("mesh_set_remove").is_some());
        assert!(module.get_function("mesh_set_contains").is_some());
        assert!(module.get_function("mesh_set_size").is_some());
        assert!(module.get_function("mesh_set_union").is_some());
        assert!(module.get_function("mesh_set_intersection").is_some());
        assert!(module.get_function("mesh_set_element_at").is_some());
        assert!(module.get_function("mesh_set_difference").is_some());
        assert!(module.get_function("mesh_set_to_list").is_some());
        assert!(module.get_function("mesh_set_from_list").is_some());
        assert!(module.get_function("mesh_tuple_nth").is_some());
        assert!(module.get_function("mesh_tuple_first").is_some());
        assert!(module.get_function("mesh_tuple_second").is_some());
        assert!(module.get_function("mesh_tuple_size").is_some());
        assert!(module.get_function("mesh_range_new").is_some());
        assert!(module.get_function("mesh_range_to_list").is_some());
        assert!(module.get_function("mesh_range_map").is_some());
        assert!(module.get_function("mesh_range_filter").is_some());
        assert!(module.get_function("mesh_range_length").is_some());
        assert!(module.get_function("mesh_queue_new").is_some());
        assert!(module.get_function("mesh_queue_push").is_some());
        assert!(module.get_function("mesh_queue_pop").is_some());
        assert!(module.get_function("mesh_queue_peek").is_some());
        assert!(module.get_function("mesh_queue_size").is_some());
        assert!(module.get_function("mesh_queue_is_empty").is_some());

        // HTTP functions (Phase 8 Plan 05)
        assert!(module.get_function("mesh_http_router").is_some());
        assert!(module.get_function("mesh_http_route").is_some());
        assert!(module.get_function("mesh_http_serve").is_some());
        assert!(module.get_function("mesh_http_serve_tls").is_some());
        assert!(module.get_function("mesh_http_response_new").is_some());
        assert!(module.get_function("mesh_http_get").is_some());
        assert!(module.get_function("mesh_http_post").is_some());
        assert!(module.get_function("mesh_http_request_method").is_some());
        assert!(module.get_function("mesh_http_request_path").is_some());
        assert!(module.get_function("mesh_http_request_body").is_some());
        assert!(module.get_function("mesh_http_request_header").is_some());
        assert!(module.get_function("mesh_http_request_query").is_some());

        // Phase 51: Method-specific routing and path parameter extraction
        assert!(module.get_function("mesh_http_route_get").is_some());
        assert!(module.get_function("mesh_http_route_post").is_some());
        assert!(module.get_function("mesh_http_route_put").is_some());
        assert!(module.get_function("mesh_http_route_delete").is_some());
        assert!(module.get_function("mesh_http_request_param").is_some());

        // Phase 52: Middleware
        assert!(module.get_function("mesh_http_use_middleware").is_some());

        // Phase 53: SQLite
        assert!(module.get_function("mesh_sqlite_open").is_some());
        assert!(module.get_function("mesh_sqlite_close").is_some());
        assert!(module.get_function("mesh_sqlite_execute").is_some());
        assert!(module.get_function("mesh_sqlite_query").is_some());

        // Phase 54: PostgreSQL
        assert!(module.get_function("mesh_pg_connect").is_some());
        assert!(module.get_function("mesh_pg_close").is_some());
        assert!(module.get_function("mesh_pg_execute").is_some());
        assert!(module.get_function("mesh_pg_query").is_some());

        // Phase 57: PostgreSQL Transactions
        assert!(module.get_function("mesh_pg_begin").is_some());
        assert!(module.get_function("mesh_pg_commit").is_some());
        assert!(module.get_function("mesh_pg_rollback").is_some());
        assert!(module.get_function("mesh_pg_transaction").is_some());

        // Phase 57: SQLite Transactions
        assert!(module.get_function("mesh_sqlite_begin").is_some());
        assert!(module.get_function("mesh_sqlite_commit").is_some());
        assert!(module.get_function("mesh_sqlite_rollback").is_some());

        // Phase 57: Connection Pool
        assert!(module.get_function("mesh_pool_open").is_some());
        assert!(module.get_function("mesh_pool_close").is_some());
        assert!(module.get_function("mesh_pool_checkout").is_some());
        assert!(module.get_function("mesh_pool_checkin").is_some());
        assert!(module.get_function("mesh_pool_query").is_some());
        assert!(module.get_function("mesh_pool_execute").is_some());

        // Phase 58: Row Parsing & Struct-to-Row Mapping
        assert!(module.get_function("mesh_row_from_row_get").is_some());
        assert!(module.get_function("mesh_row_parse_int").is_some());
        assert!(module.get_function("mesh_row_parse_float").is_some());
        assert!(module.get_function("mesh_row_parse_bool").is_some());
        assert!(module.get_function("mesh_pg_query_as").is_some());
        assert!(module.get_function("mesh_pool_query_as").is_some());

        // Service runtime functions (Phase 9 Plan 03)
        assert!(module.get_function("mesh_service_call").is_some());
        assert!(module.get_function("mesh_service_reply").is_some());

        // Job runtime functions (Phase 9 Plan 04)
        assert!(module.get_function("mesh_job_async").is_some());
        assert!(module.get_function("mesh_job_await").is_some());
        assert!(module.get_function("mesh_job_await_timeout").is_some());
        assert!(module.get_function("mesh_job_map").is_some());

        // JSON functions (Phase 8 Plan 04)
        assert!(module.get_function("mesh_json_parse").is_some());
        assert!(module.get_function("mesh_json_encode").is_some());
        assert!(module.get_function("mesh_json_encode_string").is_some());
        assert!(module.get_function("mesh_json_encode_int").is_some());
        assert!(module.get_function("mesh_json_encode_bool").is_some());
        assert!(module.get_function("mesh_json_encode_map").is_some());
        assert!(module.get_function("mesh_json_encode_list").is_some());
        assert!(module.get_function("mesh_json_from_int").is_some());
        assert!(module.get_function("mesh_json_from_float").is_some());
        assert!(module.get_function("mesh_json_from_bool").is_some());
        assert!(module.get_function("mesh_json_from_string").is_some());

        // Phase 103: JSON field extraction
        assert!(module.get_function("mesh_json_get").is_some());
        assert!(module.get_function("mesh_json_get_nested").is_some());

        // Structured JSON functions (Phase 49)
        assert!(module.get_function("mesh_json_object_new").is_some());
        assert!(module.get_function("mesh_json_object_put").is_some());
        assert!(module.get_function("mesh_json_object_get").is_some());
        assert!(module.get_function("mesh_json_array_new").is_some());
        assert!(module.get_function("mesh_json_array_push").is_some());
        assert!(module.get_function("mesh_json_array_get").is_some());
        assert!(module.get_function("mesh_json_as_int").is_some());
        assert!(module.get_function("mesh_json_as_float").is_some());
        assert!(module.get_function("mesh_json_as_string").is_some());
        assert!(module.get_function("mesh_json_as_bool").is_some());
        assert!(module.get_function("mesh_json_null").is_some());
        assert!(module.get_function("mesh_json_from_list").is_some());
        assert!(module.get_function("mesh_json_from_map").is_some());
        assert!(module.get_function("mesh_json_to_list").is_some());
        assert!(module.get_function("mesh_json_to_map").is_some());

        // Hash runtime functions (Phase 21 Plan 01)
        assert!(module.get_function("mesh_hash_int").is_some());
        assert!(module.get_function("mesh_hash_float").is_some());
        assert!(module.get_function("mesh_hash_bool").is_some());
        assert!(module.get_function("mesh_hash_string").is_some());
        assert!(module.get_function("mesh_hash_combine").is_some());

        // Collection Display runtime functions (Phase 21 Plan 04)
        assert!(module.get_function("mesh_list_to_string").is_some());
        assert!(module.get_function("mesh_map_to_string").is_some());
        assert!(module.get_function("mesh_set_to_string").is_some());
        assert!(module.get_function("mesh_string_to_string").is_some());

        // List Eq/Ord runtime functions (Phase 27 Plan 01)
        assert!(module.get_function("mesh_list_eq").is_some());
        assert!(module.get_function("mesh_list_compare").is_some());

        // Timer functions (Phase 44 Plan 02)
        assert!(module.get_function("mesh_timer_sleep").is_some());
        assert!(module.get_function("mesh_timer_send_after").is_some());

        // WebSocket functions (Phase 60)
        assert!(module.get_function("mesh_ws_serve").is_some());
        assert!(module.get_function("mesh_ws_send").is_some());
        assert!(module.get_function("mesh_ws_send_binary").is_some());
        assert!(module.get_function("mesh_ws_serve_tls").is_some());

        // WebSocket Room functions (Phase 62)
        assert!(module.get_function("mesh_ws_join").is_some());
        assert!(module.get_function("mesh_ws_leave").is_some());
        assert!(module.get_function("mesh_ws_broadcast").is_some());
        assert!(module.get_function("mesh_ws_broadcast_except").is_some());

        // Phase 67: Node distribution & remote spawn
        assert!(module.get_function("mesh_node_start").is_some());
        assert!(module.get_function("mesh_node_connect").is_some());
        assert!(module.get_function("mesh_node_self").is_some());
        assert!(module.get_function("mesh_node_list").is_some());
        assert!(module.get_function("mesh_node_monitor").is_some());
        assert!(module.get_function("mesh_node_spawn").is_some());
        assert!(module.get_function("mesh_register_function").is_some());
        assert!(module.get_function("mesh_process_monitor").is_some());
        assert!(module.get_function("mesh_process_demonitor").is_some());
        assert!(module.get_function("mesh_actor_send_named").is_some());

        // Phase 68: Global Registry
        assert!(module.get_function("mesh_global_register").is_some());
        assert!(module.get_function("mesh_global_whereis").is_some());
        assert!(module.get_function("mesh_global_unregister").is_some());

        // Phase 76: Iterator runtime functions
        assert!(module.get_function("mesh_list_iter_new").is_some());
        assert!(module.get_function("mesh_list_iter_next").is_some());
        assert!(module.get_function("mesh_map_iter_new").is_some());
        assert!(module.get_function("mesh_map_iter_next").is_some());
        assert!(module.get_function("mesh_set_iter_new").is_some());
        assert!(module.get_function("mesh_set_iter_next").is_some());
        assert!(module.get_function("mesh_range_iter_new").is_some());
        assert!(module.get_function("mesh_range_iter_next").is_some());
        assert!(module.get_function("mesh_iter_from").is_some());

        // Phase 78: Lazy Combinators & Terminals
        assert!(module.get_function("mesh_iter_map").is_some());
        assert!(module.get_function("mesh_iter_filter").is_some());
        assert!(module.get_function("mesh_iter_take").is_some());
        assert!(module.get_function("mesh_iter_skip").is_some());
        assert!(module.get_function("mesh_iter_enumerate").is_some());
        assert!(module.get_function("mesh_iter_zip").is_some());
        assert!(module.get_function("mesh_iter_count").is_some());
        assert!(module.get_function("mesh_iter_sum").is_some());
        assert!(module.get_function("mesh_iter_any").is_some());
        assert!(module.get_function("mesh_iter_all").is_some());
        assert!(module.get_function("mesh_iter_find").is_some());
        assert!(module.get_function("mesh_iter_reduce").is_some());
        assert!(module.get_function("mesh_iter_generic_next").is_some());
        assert!(module.get_function("mesh_iter_map_next").is_some());
        assert!(module.get_function("mesh_iter_filter_next").is_some());
        assert!(module.get_function("mesh_iter_take_next").is_some());
        assert!(module.get_function("mesh_iter_skip_next").is_some());
        assert!(module.get_function("mesh_iter_enumerate_next").is_some());
        assert!(module.get_function("mesh_iter_zip_next").is_some());

        // Phase 79: Collect terminal operations
        assert!(module.get_function("mesh_list_collect").is_some());
        assert!(module.get_function("mesh_map_collect").is_some());
        assert!(module.get_function("mesh_set_collect").is_some());
        assert!(module.get_function("mesh_string_collect").is_some());

        // Phase 97: ORM SQL Generation
        assert!(module.get_function("mesh_orm_build_select").is_some());
        assert!(module.get_function("mesh_orm_build_insert").is_some());
        assert!(module.get_function("mesh_orm_build_update").is_some());
        assert!(module.get_function("mesh_orm_build_delete").is_some());

        // Phase 101: Migration DDL Operations
        assert!(module.get_function("mesh_migration_create_table").is_some());
        assert!(module.get_function("mesh_migration_drop_table").is_some());
        assert!(module.get_function("mesh_migration_add_column").is_some());
        assert!(module.get_function("mesh_migration_drop_column").is_some());
        assert!(module.get_function("mesh_migration_rename_column").is_some());
        assert!(module.get_function("mesh_migration_create_index").is_some());
        assert!(module.get_function("mesh_migration_drop_index").is_some());
        assert!(module.get_function("mesh_migration_execute").is_some());

        // Phase 98: Query Builder
        assert!(module.get_function("mesh_query_from").is_some());
        assert!(module.get_function("mesh_query_where").is_some());
        assert!(module.get_function("mesh_query_where_op").is_some());
        assert!(module.get_function("mesh_query_where_in").is_some());
        assert!(module.get_function("mesh_query_where_null").is_some());
        assert!(module.get_function("mesh_query_where_not_null").is_some());
        assert!(module.get_function("mesh_query_select").is_some());
        assert!(module.get_function("mesh_query_order_by").is_some());
        assert!(module.get_function("mesh_query_limit").is_some());
        assert!(module.get_function("mesh_query_offset").is_some());
        assert!(module.get_function("mesh_query_join").is_some());
        assert!(module.get_function("mesh_query_join_as").is_some());
        assert!(module.get_function("mesh_query_group_by").is_some());
        assert!(module.get_function("mesh_query_having").is_some());
        assert!(module.get_function("mesh_query_fragment").is_some());
        // Phase 106: Raw ORDER BY / GROUP BY
        assert!(module.get_function("mesh_query_order_by_raw").is_some());
        assert!(module.get_function("mesh_query_group_by_raw").is_some());
    }

    #[test]
    fn test_get_intrinsic() {
        let context = Context::create();
        let module = context.create_module("test");
        declare_intrinsics(&module);

        let init_fn = get_intrinsic(&module, "mesh_rt_init");
        assert_eq!(init_fn.get_name().to_str().unwrap(), "mesh_rt_init");
    }

    #[test]
    fn test_panic_is_noreturn() {
        let context = Context::create();
        let module = context.create_module("test");
        declare_intrinsics(&module);

        let panic_fn = get_intrinsic(&module, "mesh_panic");
        // Check that noreturn attribute is present
        let noreturn_id = inkwell::attributes::Attribute::get_named_enum_kind_id("noreturn");
        let attr = panic_fn.get_enum_attribute(inkwell::attributes::AttributeLoc::Function, noreturn_id);
        assert!(attr.is_some(), "mesh_panic should have noreturn attribute");
    }
}
