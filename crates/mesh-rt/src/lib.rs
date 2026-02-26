//! Mesh runtime library.
//!
//! This crate provides the runtime support functions that compiled Mesh
//! programs call at runtime. It is compiled as both:
//!
//! - A static library (`libmesh_rt.a`) for linking into Mesh binaries
//! - A Rust library (`lib`) for unit testing
//!
//! ## Modules
//!
//! - [`gc`]: Arena/bump allocator for GC-managed memory (Phase 5: no collection)
//! - [`string`]: GC-managed string operations (create, concat, format, print)
//! - [`panic`]: Runtime panic handler with source locations
//! - [`actor`]: Actor runtime -- PCB, M:N scheduler, corosensei coroutines
//!
//! ## ABI Contract
//!
//! All public `extern "C"` functions in this crate form the runtime ABI.
//! Compiled Mesh programs call these functions directly via LLVM IR. The
//! function signatures must remain stable across Mesh compiler versions
//! (or at least across a single phase).

pub mod actor;
pub mod collections;
pub mod db;
pub mod env;
pub mod file;
pub mod gc;
pub mod hash;
pub mod http;
pub mod io;
pub mod ws;
pub mod dist;
pub mod json;
pub mod iter;
pub mod option;
pub mod panic;
pub mod regex;
pub mod string;

// Re-export key functions for convenient Rust-side access and testing.
pub use actor::{
    mesh_actor_link, mesh_actor_receive, mesh_actor_register, mesh_actor_self, mesh_actor_send,
    mesh_actor_send_named,
    mesh_actor_set_terminate, mesh_actor_spawn, mesh_actor_whereis, mesh_reduction_check,
    mesh_rt_init_actor, mesh_rt_run_scheduler,
    mesh_timer_sleep, mesh_timer_send_after,
    mesh_process_monitor, mesh_process_demonitor,
    mesh_node_monitor,
    mesh_global_register, mesh_global_whereis, mesh_global_unregister,
};
pub use actor::service::{mesh_service_call, mesh_service_reply};
pub use db::orm::{
    mesh_orm_build_select, mesh_orm_build_insert, mesh_orm_build_update, mesh_orm_build_delete,
};
pub use db::query::{
    mesh_query_from, mesh_query_where, mesh_query_where_op, mesh_query_where_in,
    mesh_query_where_not_in, mesh_query_where_between, mesh_query_where_or,
    mesh_query_where_null, mesh_query_where_not_null, mesh_query_select, mesh_query_order_by,
    mesh_query_limit, mesh_query_offset, mesh_query_join, mesh_query_join_as, mesh_query_group_by,
    mesh_query_having, mesh_query_fragment,
    mesh_query_select_raw, mesh_query_where_raw,
    mesh_query_order_by_raw, mesh_query_group_by_raw,
    mesh_query_select_count, mesh_query_select_count_field,
    mesh_query_select_sum, mesh_query_select_avg,
    mesh_query_select_min, mesh_query_select_max,
    mesh_query_where_sub,
};
pub use db::changeset::{
    mesh_changeset_cast, mesh_changeset_cast_with_types,
    mesh_changeset_validate_required, mesh_changeset_validate_length,
    mesh_changeset_validate_format, mesh_changeset_validate_inclusion,
    mesh_changeset_validate_number,
    mesh_changeset_valid, mesh_changeset_errors, mesh_changeset_changes,
    mesh_changeset_get_change, mesh_changeset_get_error,
};
pub use db::json::{
    mesh_json_get, mesh_json_get_nested,
};
pub use db::migration::{
    mesh_migration_create_table, mesh_migration_drop_table,
    mesh_migration_add_column, mesh_migration_drop_column,
    mesh_migration_rename_column, mesh_migration_create_index,
    mesh_migration_drop_index, mesh_migration_execute,
};
pub use db::repo::{
    mesh_repo_all, mesh_repo_one, mesh_repo_get, mesh_repo_get_by,
    mesh_repo_count, mesh_repo_exists,
    mesh_repo_insert, mesh_repo_update, mesh_repo_delete, mesh_repo_transaction,
    mesh_repo_insert_changeset, mesh_repo_update_changeset,
    mesh_repo_preload,
    mesh_repo_update_where, mesh_repo_delete_where,
    mesh_repo_insert_or_update, mesh_repo_delete_where_returning,
    mesh_repo_query_raw, mesh_repo_execute_raw,
};
pub use db::pg::{
    mesh_pg_connect, mesh_pg_close, mesh_pg_execute, mesh_pg_query,
};
pub use db::pool::{
    mesh_pool_open, mesh_pool_checkout, mesh_pool_checkin, mesh_pool_query,
    mesh_pool_execute, mesh_pool_close,
};
pub use db::sqlite::{
    mesh_sqlite_open, mesh_sqlite_close, mesh_sqlite_execute, mesh_sqlite_query,
};
pub use collections::list::{
    mesh_list_all, mesh_list_any, mesh_list_append, mesh_list_concat, mesh_list_contains,
    mesh_list_drop, mesh_list_enumerate, mesh_list_filter, mesh_list_find, mesh_list_flat_map,
    mesh_list_flatten, mesh_list_from_array, mesh_list_get, mesh_list_head, mesh_list_last,
    mesh_list_length, mesh_list_map, mesh_list_new, mesh_list_nth, mesh_list_reduce,
    mesh_list_reverse, mesh_list_sort, mesh_list_tail, mesh_list_take, mesh_list_zip,
};
pub use collections::map::{
    mesh_map_delete, mesh_map_from_list, mesh_map_get, mesh_map_has_key, mesh_map_keys,
    mesh_map_merge, mesh_map_new, mesh_map_put, mesh_map_size, mesh_map_to_list, mesh_map_values,
};
pub use collections::queue::{
    mesh_queue_is_empty, mesh_queue_new, mesh_queue_peek, mesh_queue_pop, mesh_queue_push,
    mesh_queue_size,
};
pub use collections::range::{
    mesh_range_filter, mesh_range_length, mesh_range_map, mesh_range_new, mesh_range_to_list,
};
pub use collections::set::{
    mesh_set_add, mesh_set_contains, mesh_set_difference, mesh_set_from_list,
    mesh_set_intersection, mesh_set_new, mesh_set_remove, mesh_set_size, mesh_set_to_list,
    mesh_set_union,
};
pub use collections::tuple::{mesh_tuple_first, mesh_tuple_nth, mesh_tuple_second, mesh_tuple_size};
pub use env::{mesh_env_args, mesh_env_get, mesh_env_get_with_default, mesh_env_get_int};
pub use regex::{
    mesh_regex_from_literal, mesh_regex_compile, mesh_regex_match,
    mesh_regex_captures, mesh_regex_replace, mesh_regex_split,
};
pub use iter::{
    mesh_iter_generic_next,
    mesh_iter_map, mesh_iter_map_next,
    mesh_iter_filter, mesh_iter_filter_next,
    mesh_iter_take, mesh_iter_take_next,
    mesh_iter_skip, mesh_iter_skip_next,
    mesh_iter_enumerate, mesh_iter_enumerate_next,
    mesh_iter_zip, mesh_iter_zip_next,
    mesh_iter_count, mesh_iter_sum, mesh_iter_any, mesh_iter_all, mesh_iter_find, mesh_iter_reduce,
    mesh_list_collect, mesh_map_collect, mesh_map_collect_string_keys, mesh_set_collect, mesh_string_collect,
};
pub use option::{MeshOption, alloc_option};
pub use file::{
    mesh_file_append, mesh_file_delete, mesh_file_exists, mesh_file_read, mesh_file_write,
};
pub use gc::{mesh_gc_alloc, mesh_gc_alloc_actor, mesh_rt_init};
pub use hash::{mesh_hash_bool, mesh_hash_combine, mesh_hash_float, mesh_hash_int, mesh_hash_string};
pub use http::{
    mesh_http_get, mesh_http_post, mesh_http_request_body, mesh_http_request_header,
    mesh_http_request_method, mesh_http_request_path, mesh_http_request_query, mesh_http_response_new,
    mesh_http_route, mesh_http_router, mesh_http_serve,
};
pub use io::{mesh_io_eprintln, mesh_io_read_line};
pub use json::{
    mesh_json_encode, mesh_json_encode_bool, mesh_json_encode_int, mesh_json_encode_list,
    mesh_json_encode_map, mesh_json_encode_string, mesh_json_from_bool, mesh_json_from_float,
    mesh_json_from_int, mesh_json_from_string, mesh_json_parse,
};
pub use dist::node::{mesh_node_self, mesh_node_list, mesh_node_start, mesh_node_connect, mesh_register_function, mesh_node_spawn};
pub use panic::mesh_panic;
pub use string::{
    mesh_bool_to_string, mesh_float_to_string, mesh_int_to_string, mesh_print, mesh_println,
    mesh_string_concat, mesh_string_contains, mesh_string_ends_with, mesh_string_eq,
    mesh_string_join, mesh_string_length, mesh_string_new, mesh_string_replace, mesh_string_slice,
    mesh_string_split, mesh_string_starts_with, mesh_string_to_float, mesh_string_to_int,
    mesh_string_to_lower, mesh_string_to_upper, mesh_string_trim,
    MeshString,
};
