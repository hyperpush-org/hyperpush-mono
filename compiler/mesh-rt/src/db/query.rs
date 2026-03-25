//! Query builder runtime module for the Mesh runtime.
//!
//! Provides an immutable, pipe-composable Query struct that accumulates
//! SQL clauses. Each builder function allocates a new Query via
//! `mesh_gc_alloc_actor`, copies the previous state, and modifies the
//! relevant slots. The Query object is never mutated in place.
//!
//! ## Query object layout (14 slots, 112 bytes)
//!
//! | Slot | Offset | Name            | Type                   |
//! |------|--------|-----------------|------------------------|
//! |  0   |   0    | source          | *mut u8 (MeshString)   |
//! |  1   |   8    | select_fields   | *mut u8 (List<String>) |
//! |  2   |  16    | where_clauses   | *mut u8 (List<String>) |
//! |  3   |  24    | where_params    | *mut u8 (List<String>) |
//! |  4   |  32    | order_fields    | *mut u8 (List<String>) |
//! |  5   |  40    | limit_val       | i64 (-1 = no limit)    |
//! |  6   |  48    | offset_val      | i64 (-1 = no offset)   |
//! |  7   |  56    | join_clauses    | *mut u8 (List<String>) |
//! |  8   |  64    | group_fields    | *mut u8 (List<String>) |
//! |  9   |  72    | having_clauses  | *mut u8 (List<String>) |
//! | 10   |  80    | having_params   | *mut u8 (List<String>) |
//! | 11   |  88    | fragment_parts  | *mut u8 (List<String>) |
//! | 12   |  96    | fragment_params | *mut u8 (List<String>) |
//! | 13   | 104    | select_params   | *mut u8 (List<String>) |

use crate::collections::list::{mesh_list_append, mesh_list_get, mesh_list_length, mesh_list_new};
use crate::gc::mesh_gc_alloc_actor;
use crate::string::{mesh_string_new, MeshString};

// ── Constants ────────────────────────────────────────────────────────

const QUERY_SLOTS: usize = 14;
const QUERY_SIZE: usize = QUERY_SLOTS * 8; // 112 bytes

// Slot indices
const SLOT_SOURCE: usize = 0;
const SLOT_SELECT: usize = 1;
const SLOT_WHERE_CLAUSES: usize = 2;
const SLOT_WHERE_PARAMS: usize = 3;
const SLOT_ORDER: usize = 4;
const SLOT_LIMIT: usize = 5;
const SLOT_OFFSET: usize = 6;
const SLOT_JOIN: usize = 7;
const SLOT_GROUP: usize = 8;
const SLOT_HAVING_CLAUSES: usize = 9;
const SLOT_HAVING_PARAMS: usize = 10;
const SLOT_FRAGMENT_PARTS: usize = 11;
const SLOT_FRAGMENT_PARAMS: usize = 12;
const SLOT_SELECT_PARAMS: usize = 13;

// ── Slot access helpers ──────────────────────────────────────────────

unsafe fn query_get(q: *mut u8, slot: usize) -> *mut u8 {
    *(q.add(slot * 8) as *mut *mut u8)
}

unsafe fn query_set(q: *mut u8, slot: usize, val: *mut u8) {
    *(q.add(slot * 8) as *mut *mut u8) = val;
}

#[allow(dead_code)]
unsafe fn query_get_int(q: *mut u8, slot: usize) -> i64 {
    *(q.add(slot * 8) as *mut i64)
}

unsafe fn query_set_int(q: *mut u8, slot: usize, val: i64) {
    *(q.add(slot * 8) as *mut i64) = val;
}

// ── String helpers ───────────────────────────────────────────────────

/// Create a MeshString from a Rust &str and return as *mut u8.
unsafe fn rust_str_to_mesh(s: &str) -> *mut u8 {
    mesh_string_new(s.as_ptr(), s.len() as u64) as *mut u8
}

/// Read a MeshString pointer as a Rust &str.
unsafe fn mesh_str_ref(ptr: *mut u8) -> &'static str {
    let ms = ptr as *const MeshString;
    (*ms).as_str()
}

/// Concatenate two MeshString pointers, returning a new MeshString as *mut u8.
#[allow(dead_code)]
unsafe fn mesh_concat(a: *mut u8, b: *mut u8) -> *mut u8 {
    crate::string::mesh_string_concat(a as *const MeshString, b as *const MeshString) as *mut u8
}

// ── Atom-to-SQL mapping ──────────────────────────────────────────────

fn atom_to_sql_op(atom: &str) -> &str {
    match atom {
        "eq" => "=",
        "neq" => "!=",
        "lt" => "<",
        "gt" => ">",
        "lte" => "<=",
        "gte" => ">=",
        "like" => "LIKE",
        "ilike" => "ILIKE",
        _ => "=", // default to equality
    }
}

fn atom_to_direction(atom: &str) -> &str {
    match atom {
        "asc" => "ASC",
        "desc" => "DESC",
        _ => "ASC",
    }
}

fn atom_to_join_type(atom: &str) -> &str {
    match atom {
        "inner" => "INNER",
        "left" => "LEFT",
        "right" => "RIGHT",
        _ => "INNER",
    }
}

// ── Query allocation helpers ─────────────────────────────────────────

/// Allocate a fresh Query with all pointer slots set to empty lists
/// and integer slots set to -1.
unsafe fn alloc_query() -> *mut u8 {
    let q = mesh_gc_alloc_actor(QUERY_SIZE as u64, 8);
    std::ptr::write_bytes(q, 0, QUERY_SIZE);
    // Initialize list slots to empty lists
    let empty = mesh_list_new();
    query_set(q, SLOT_SELECT, empty);
    query_set(q, SLOT_WHERE_CLAUSES, mesh_list_new());
    query_set(q, SLOT_WHERE_PARAMS, mesh_list_new());
    query_set(q, SLOT_ORDER, mesh_list_new());
    query_set(q, SLOT_JOIN, mesh_list_new());
    query_set(q, SLOT_GROUP, mesh_list_new());
    query_set(q, SLOT_HAVING_CLAUSES, mesh_list_new());
    query_set(q, SLOT_HAVING_PARAMS, mesh_list_new());
    query_set(q, SLOT_FRAGMENT_PARTS, mesh_list_new());
    query_set(q, SLOT_FRAGMENT_PARAMS, mesh_list_new());
    query_set(q, SLOT_SELECT_PARAMS, mesh_list_new());
    // Integer slots: -1 means "not set"
    query_set_int(q, SLOT_LIMIT, -1);
    query_set_int(q, SLOT_OFFSET, -1);
    q
}

/// Clone a Query: allocate new 112 bytes and copy all data from source.
unsafe fn clone_query(src: *mut u8) -> *mut u8 {
    let dst = mesh_gc_alloc_actor(QUERY_SIZE as u64, 8);
    std::ptr::copy_nonoverlapping(src, dst, QUERY_SIZE);
    dst
}

// ── Extern C builder functions ───────────────────────────────────────

/// Create a new Query from a table name string.
///
/// `Query.from("users")` -> opaque Query pointer
#[no_mangle]
pub extern "C" fn mesh_query_from(table: *mut u8) -> *mut u8 {
    unsafe {
        let q = alloc_query();
        query_set(q, SLOT_SOURCE, table);
        q
    }
}

/// Add an equality WHERE clause: `field = value`.
///
/// `Query.where(q, :name, "Alice")` -> new Query with WHERE name = $N
#[no_mangle]
pub extern "C" fn mesh_query_where(q: *mut u8, field: *mut u8, value: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let clause = format!("{} =", field_str);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        let wp = query_get(new_q, SLOT_WHERE_PARAMS);
        query_set(new_q, SLOT_WHERE_PARAMS, mesh_list_append(wp, value as u64));
        new_q
    }
}

/// Add an operator WHERE clause: `field op value`.
///
/// `Query.where_op(q, :age, :gt, "21")` -> new Query with WHERE age > $N
#[no_mangle]
pub extern "C" fn mesh_query_where_op(
    q: *mut u8,
    field: *mut u8,
    op: *mut u8,
    value: *mut u8,
) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let op_str = mesh_str_ref(op);
        let sql_op = atom_to_sql_op(op_str);
        let clause = format!("{} {}", field_str, sql_op);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        let wp = query_get(new_q, SLOT_WHERE_PARAMS);
        query_set(new_q, SLOT_WHERE_PARAMS, mesh_list_append(wp, value as u64));
        new_q
    }
}

/// Add a WHERE IN clause: `field IN (values...)`.
///
/// `Query.where_in(q, :status, ["active", "pending"])` -> new Query with WHERE status IN ($N, $M)
#[no_mangle]
pub extern "C" fn mesh_query_where_in(q: *mut u8, field: *mut u8, values: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let list_len = mesh_list_length(values);
        let clause = format!("{} IN:{}", field_str, list_len);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        // Append each value from the list to where_params
        let mut wp = query_get(new_q, SLOT_WHERE_PARAMS);
        for i in 0..list_len {
            let elem = mesh_list_get(values, i);
            wp = mesh_list_append(wp, elem);
        }
        query_set(new_q, SLOT_WHERE_PARAMS, wp);
        new_q
    }
}

/// Add a WHERE NOT IN clause: `field NOT IN (values...)`.
///
/// `Query.where_not_in(q, :status, ["archived", "deleted"])` -> new Query with WHERE status NOT IN ($N, $M)
#[no_mangle]
pub extern "C" fn mesh_query_where_not_in(q: *mut u8, field: *mut u8, values: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let list_len = mesh_list_length(values);
        let clause = format!("{} NOT_IN:{}", field_str, list_len);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        // Append each value from the list to where_params
        let mut wp = query_get(new_q, SLOT_WHERE_PARAMS);
        for i in 0..list_len {
            let elem = mesh_list_get(values, i);
            wp = mesh_list_append(wp, elem);
        }
        query_set(new_q, SLOT_WHERE_PARAMS, wp);
        new_q
    }
}

/// Add a WHERE BETWEEN clause: `field BETWEEN low AND high`.
///
/// `Query.where_between(q, :age, "18", "65")` -> new Query with WHERE age BETWEEN $N AND $M
#[no_mangle]
pub extern "C" fn mesh_query_where_between(
    q: *mut u8,
    field: *mut u8,
    low: *mut u8,
    high: *mut u8,
) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let clause = format!("{} BETWEEN", field_str);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        let mut wp = query_get(new_q, SLOT_WHERE_PARAMS);
        wp = mesh_list_append(wp, low as u64);
        wp = mesh_list_append(wp, high as u64);
        query_set(new_q, SLOT_WHERE_PARAMS, wp);
        new_q
    }
}

/// Add a WHERE OR clause: `(field1 = $N OR field2 = $M ...)`.
///
/// `Query.where_or(q, [:status, :level], ["active", "error"])` -> new Query with WHERE (status = $N OR level = $M)
#[no_mangle]
pub extern "C" fn mesh_query_where_or(q: *mut u8, fields: *mut u8, values: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_count = mesh_list_length(fields);
        // Build OR clause encoding: "OR:field1,field2,...:N"
        let mut field_names = Vec::new();
        for i in 0..field_count {
            let f = mesh_list_get(fields, i) as *mut u8;
            field_names.push(mesh_str_ref(f).to_string());
        }
        let clause = format!("OR:{}:{}", field_names.join(","), field_count);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        // Append values to where_params
        let mut wp = query_get(new_q, SLOT_WHERE_PARAMS);
        let val_count = mesh_list_length(values);
        for i in 0..val_count {
            let elem = mesh_list_get(values, i);
            wp = mesh_list_append(wp, elem);
        }
        query_set(new_q, SLOT_WHERE_PARAMS, wp);
        new_q
    }
}

/// Add a WHERE IS NULL clause.
///
/// `Query.where_null(q, :deleted_at)` -> new Query with WHERE deleted_at IS NULL
#[no_mangle]
pub extern "C" fn mesh_query_where_null(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let clause = format!("{} IS NULL", field_str);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        new_q
    }
}

/// Add a WHERE IS NOT NULL clause.
///
/// `Query.where_not_null(q, :name)` -> new Query with WHERE name IS NOT NULL
#[no_mangle]
pub extern "C" fn mesh_query_where_not_null(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let clause = format!("{} IS NOT NULL", field_str);
        let clause_mesh = rust_str_to_mesh(&clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );
        new_q
    }
}

/// Set the SELECT fields for the query.
///
/// `Query.select(q, ["id", "name"])` -> new Query with SELECT id, name
#[no_mangle]
pub extern "C" fn mesh_query_select(q: *mut u8, fields: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        query_set(new_q, SLOT_SELECT, fields);
        query_set(new_q, SLOT_SELECT_PARAMS, mesh_list_new());
        new_q
    }
}

/// Add an ORDER BY clause.
///
/// `Query.order_by(q, :name, :asc)` -> new Query with ORDER BY name ASC
#[no_mangle]
pub extern "C" fn mesh_query_order_by(q: *mut u8, field: *mut u8, direction: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let dir_str = mesh_str_ref(direction);
        let dir_sql = atom_to_direction(dir_str);
        let order = format!("{} {}", field_str, dir_sql);
        let order_mesh = rust_str_to_mesh(&order);
        let of = query_get(new_q, SLOT_ORDER);
        query_set(new_q, SLOT_ORDER, mesh_list_append(of, order_mesh as u64));
        new_q
    }
}

/// Add a raw ORDER BY expression (no quoting).
///
/// `Query.order_by_raw(q, "random()")` -> new Query with ORDER BY random()
#[no_mangle]
pub extern "C" fn mesh_query_order_by_raw(q: *mut u8, expression: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let expr_str = mesh_str_ref(expression);
        let raw_order = format!("RAW:{}", expr_str);
        let raw_mesh = rust_str_to_mesh(&raw_order);
        let of = query_get(new_q, SLOT_ORDER);
        query_set(new_q, SLOT_ORDER, mesh_list_append(of, raw_mesh as u64));
        new_q
    }
}

/// Set the LIMIT for the query.
///
/// `Query.limit(q, 10)` -> new Query with LIMIT 10
#[no_mangle]
pub extern "C" fn mesh_query_limit(q: *mut u8, n: i64) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        query_set_int(new_q, SLOT_LIMIT, n);
        new_q
    }
}

/// Set the OFFSET for the query.
///
/// `Query.offset(q, 20)` -> new Query with OFFSET 20
#[no_mangle]
pub extern "C" fn mesh_query_offset(q: *mut u8, n: i64) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        query_set_int(new_q, SLOT_OFFSET, n);
        new_q
    }
}

/// Add a JOIN clause.
///
/// `Query.join(q, :inner, "posts", "users.id = posts.user_id")` -> new Query with INNER JOIN
#[no_mangle]
pub extern "C" fn mesh_query_join(
    q: *mut u8,
    join_type: *mut u8,
    table: *mut u8,
    on_clause: *mut u8,
) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let jt_str = mesh_str_ref(join_type);
        let tbl_str = mesh_str_ref(table);
        let on_str = mesh_str_ref(on_clause);
        let jt_sql = atom_to_join_type(jt_str);
        let join = format!("{}:{}:{}", jt_sql, tbl_str, on_str);
        let join_mesh = rust_str_to_mesh(&join);
        let jc = query_get(new_q, SLOT_JOIN);
        query_set(new_q, SLOT_JOIN, mesh_list_append(jc, join_mesh as u64));
        new_q
    }
}

/// Add a JOIN clause with table alias.
///
/// `Query.join_as(q, :inner, "projects", "p", "p.id = issues.project_id")` -> INNER JOIN projects p ON ...
#[no_mangle]
pub extern "C" fn mesh_query_join_as(
    q: *mut u8,
    join_type: *mut u8,
    table: *mut u8,
    alias: *mut u8,
    on_clause: *mut u8,
) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let jt_str = mesh_str_ref(join_type);
        let tbl_str = mesh_str_ref(table);
        let alias_str = mesh_str_ref(alias);
        let on_str = mesh_str_ref(on_clause);
        let jt_sql = atom_to_join_type(jt_str);
        let join = format!("ALIAS:{}:{}:{}:{}", jt_sql, tbl_str, alias_str, on_str);
        let join_mesh = rust_str_to_mesh(&join);
        let jc = query_get(new_q, SLOT_JOIN);
        query_set(new_q, SLOT_JOIN, mesh_list_append(jc, join_mesh as u64));
        new_q
    }
}

/// Add a GROUP BY field.
///
/// `Query.group_by(q, :category)` -> new Query with GROUP BY category
#[no_mangle]
pub extern "C" fn mesh_query_group_by(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let gf = query_get(new_q, SLOT_GROUP);
        query_set(new_q, SLOT_GROUP, mesh_list_append(gf, field as u64));
        new_q
    }
}

/// Add a raw GROUP BY expression (no quoting).
///
/// `Query.group_by_raw(q, "date_trunc('hour', received_at)")` -> new Query with raw GROUP BY
#[no_mangle]
pub extern "C" fn mesh_query_group_by_raw(q: *mut u8, expression: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let expr_str = mesh_str_ref(expression);
        let raw_group = format!("RAW:{}", expr_str);
        let raw_mesh = rust_str_to_mesh(&raw_group);
        let gf = query_get(new_q, SLOT_GROUP);
        query_set(new_q, SLOT_GROUP, mesh_list_append(gf, raw_mesh as u64));
        new_q
    }
}

/// Add a HAVING clause.
///
/// `Query.having(q, "count(*) >", "5")` -> new Query with HAVING count(*) > $N
#[no_mangle]
pub extern "C" fn mesh_query_having(q: *mut u8, clause: *mut u8, value: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let hc = query_get(new_q, SLOT_HAVING_CLAUSES);
        query_set(
            new_q,
            SLOT_HAVING_CLAUSES,
            mesh_list_append(hc, clause as u64),
        );
        let hp = query_get(new_q, SLOT_HAVING_PARAMS);
        query_set(
            new_q,
            SLOT_HAVING_PARAMS,
            mesh_list_append(hp, value as u64),
        );
        new_q
    }
}

/// Set SELECT fields using raw SQL expressions (no quoting/escaping).
///
/// `Query.select_raw(q, ["count(*)::text AS count", "level"])` -> new Query with raw SELECT expressions
///
/// Each expression is stored with a "RAW:" prefix so the SQL builder emits it verbatim.
/// Can be mixed with Query.select -- normal fields get quoted, RAW: fields don't.
#[no_mangle]
pub extern "C" fn mesh_query_select_raw(q: *mut u8, expressions: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let expr_len = mesh_list_length(expressions);
        let mut sf = query_get(new_q, SLOT_SELECT);
        for i in 0..expr_len {
            let elem = mesh_list_get(expressions, i) as *mut u8;
            let expr_str = mesh_str_ref(elem);
            let raw_expr = format!("RAW:{}", expr_str);
            let raw_mesh = rust_str_to_mesh(&raw_expr);
            sf = mesh_list_append(sf, raw_mesh as u64);
        }
        query_set(new_q, SLOT_SELECT, sf);
        new_q
    }
}

/// Add a raw SQL WHERE clause with optional parameter binding.
///
/// `Query.where_raw(q, "expires_at > now()", [])` -> new Query with raw WHERE clause
/// `Query.where_raw(q, "status IN (?, ?)", ["active", "pending"])` -> with param binding
///
/// The clause is stored with a "RAW:" prefix. `?` placeholders in the clause are
/// replaced with the next sequential `$N` by the SQL builder. Parameters are appended
/// to the where_params list.
#[no_mangle]
pub extern "C" fn mesh_query_where_raw(q: *mut u8, clause: *mut u8, params: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let clause_str = mesh_str_ref(clause);
        let raw_clause = format!("RAW:{}", clause_str);
        let raw_mesh = rust_str_to_mesh(&raw_clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, raw_mesh as u64),
        );
        // Append all params to where_params
        let mut wp = query_get(new_q, SLOT_WHERE_PARAMS);
        let param_len = mesh_list_length(params);
        for i in 0..param_len {
            let elem = mesh_list_get(params, i);
            wp = mesh_list_append(wp, elem);
        }
        query_set(new_q, SLOT_WHERE_PARAMS, wp);
        new_q
    }
}

// ── Aggregate SELECT functions ───────────────────────────────────────

/// Add SELECT count(*) to the query.
///
/// `Query.select_count(q)` -> new Query with count(*) in SELECT
#[no_mangle]
pub extern "C" fn mesh_query_select_count(q: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let raw = rust_str_to_mesh("RAW:count(*)");
        let sf = query_get(new_q, SLOT_SELECT);
        query_set(new_q, SLOT_SELECT, mesh_list_append(sf, raw as u64));
        new_q
    }
}

/// Add SELECT count("field") to the query.
///
/// `Query.select_count_field(q, :assignee_id)` -> new Query with count("assignee_id") in SELECT
#[no_mangle]
pub extern "C" fn mesh_query_select_count_field(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let raw = format!("RAW:count(\"{}\")", field_str.replace('"', "\"\""));
        let raw_mesh = rust_str_to_mesh(&raw);
        let sf = query_get(new_q, SLOT_SELECT);
        query_set(new_q, SLOT_SELECT, mesh_list_append(sf, raw_mesh as u64));
        new_q
    }
}

/// Add SELECT sum("field") to the query.
///
/// `Query.select_sum(q, :amount)` -> new Query with sum("amount") in SELECT
#[no_mangle]
pub extern "C" fn mesh_query_select_sum(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let raw = format!("RAW:sum(\"{}\")", field_str.replace('"', "\"\""));
        let raw_mesh = rust_str_to_mesh(&raw);
        let sf = query_get(new_q, SLOT_SELECT);
        query_set(new_q, SLOT_SELECT, mesh_list_append(sf, raw_mesh as u64));
        new_q
    }
}

/// Add SELECT avg("field") to the query.
///
/// `Query.select_avg(q, :price)` -> new Query with avg("price") in SELECT
#[no_mangle]
pub extern "C" fn mesh_query_select_avg(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let raw = format!("RAW:avg(\"{}\")", field_str.replace('"', "\"\""));
        let raw_mesh = rust_str_to_mesh(&raw);
        let sf = query_get(new_q, SLOT_SELECT);
        query_set(new_q, SLOT_SELECT, mesh_list_append(sf, raw_mesh as u64));
        new_q
    }
}

/// Add SELECT min("field") to the query.
///
/// `Query.select_min(q, :created_at)` -> new Query with min("created_at") in SELECT
#[no_mangle]
pub extern "C" fn mesh_query_select_min(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let raw = format!("RAW:min(\"{}\")", field_str.replace('"', "\"\""));
        let raw_mesh = rust_str_to_mesh(&raw);
        let sf = query_get(new_q, SLOT_SELECT);
        query_set(new_q, SLOT_SELECT, mesh_list_append(sf, raw_mesh as u64));
        new_q
    }
}

/// Add SELECT max("field") to the query.
///
/// `Query.select_max(q, :created_at)` -> new Query with max("created_at") in SELECT
#[no_mangle]
pub extern "C" fn mesh_query_select_max(q: *mut u8, field: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);
        let raw = format!("RAW:max(\"{}\")", field_str.replace('"', "\"\""));
        let raw_mesh = rust_str_to_mesh(&raw);
        let sf = query_get(new_q, SLOT_SELECT);
        query_set(new_q, SLOT_SELECT, mesh_list_append(sf, raw_mesh as u64));
        new_q
    }
}

/// Add a WHERE IN subquery clause.
///
/// `Query.where_sub(q, :field, sub_query)` -> new Query with WHERE field IN (SELECT ...)
///
/// The sub_query is another Query that gets serialized to a SELECT SQL string.
/// Its parameters are appended to the outer query's where_params.
#[no_mangle]
pub extern "C" fn mesh_query_where_sub(q: *mut u8, field: *mut u8, sub_query: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let field_str = mesh_str_ref(field);

        // Build subquery SQL from the sub_query's slots
        let sub_source_ptr = query_get(sub_query, SLOT_SOURCE);
        let sub_source = mesh_str_ref(sub_source_ptr);
        let sub_select = list_to_sub_strings(query_get(sub_query, SLOT_SELECT));
        let sub_where_clauses = list_to_sub_strings(query_get(sub_query, SLOT_WHERE_CLAUSES));
        // Build the subquery SELECT SQL
        let mut sub_sql = String::from("SELECT ");
        if sub_select.is_empty() {
            sub_sql.push('*');
        } else {
            let cols: Vec<String> = sub_select
                .iter()
                .map(|f| {
                    if f.starts_with("RAW:") {
                        f[4..].to_string()
                    } else {
                        format!("\"{}\"", f.replace('"', "\"\""))
                    }
                })
                .collect();
            sub_sql.push_str(&cols.join(", "));
        }
        sub_sql.push_str(&format!(" FROM \"{}\"", sub_source.replace('"', "\"\"")));

        // WHERE conditions: use ? placeholders (will be renumbered by outer query's SQL builder)
        if !sub_where_clauses.is_empty() {
            sub_sql.push_str(" WHERE ");
            let mut conditions = Vec::new();
            for clause in &sub_where_clauses {
                if clause.starts_with("RAW:") {
                    // Pass raw clauses through as-is
                    conditions.push(clause[4..].to_string());
                } else if let Some(space_pos) = clause.find(' ') {
                    let col = &clause[..space_pos];
                    let op = clause[space_pos + 1..].trim();
                    if op == "IS NULL" || op == "IS NOT NULL" {
                        conditions.push(format!("\"{}\" {}", col.replace('"', "\"\""), op));
                    } else {
                        conditions.push(format!("\"{}\" {} ?", col.replace('"', "\"\""), op));
                    }
                } else {
                    conditions.push(format!("\"{}\" = ?", clause.replace('"', "\"\"")));
                }
            }
            sub_sql.push_str(&conditions.join(" AND "));
        }

        // Store as RAW: clause in where_clauses
        let raw_clause = format!(
            "RAW:\"{}\" IN ({})",
            field_str.replace('"', "\"\""),
            sub_sql
        );
        let clause_mesh = rust_str_to_mesh(&raw_clause);
        let wc = query_get(new_q, SLOT_WHERE_CLAUSES);
        query_set(
            new_q,
            SLOT_WHERE_CLAUSES,
            mesh_list_append(wc, clause_mesh as u64),
        );

        // Append subquery's where_params to outer query's where_params
        let mut wp = query_get(new_q, SLOT_WHERE_PARAMS);
        let sub_param_count = mesh_list_length(query_get(sub_query, SLOT_WHERE_PARAMS));
        for i in 0..sub_param_count {
            let elem = mesh_list_get(query_get(sub_query, SLOT_WHERE_PARAMS), i);
            wp = mesh_list_append(wp, elem);
        }
        query_set(new_q, SLOT_WHERE_PARAMS, wp);

        new_q
    }
}

/// Helper: read a list of MeshStrings into a Vec<String>.
unsafe fn list_to_sub_strings(list_ptr: *mut u8) -> Vec<String> {
    let len = mesh_list_length(list_ptr);
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        let elem = mesh_list_get(list_ptr, i) as *mut u8;
        if !elem.is_null() {
            result.push(mesh_str_ref(elem).to_string());
        }
    }
    result
}

/// Add a raw SQL fragment.
///
/// `Query.fragment(q, "WHERE custom_fn($1)", params)` -> new Query with raw fragment
#[no_mangle]
pub extern "C" fn mesh_query_fragment(q: *mut u8, sql: *mut u8, params: *mut u8) -> *mut u8 {
    unsafe {
        let new_q = clone_query(q);
        let fp = query_get(new_q, SLOT_FRAGMENT_PARTS);
        query_set(new_q, SLOT_FRAGMENT_PARTS, mesh_list_append(fp, sql as u64));
        // Append each param from the params list to fragment_params
        let mut fpar = query_get(new_q, SLOT_FRAGMENT_PARAMS);
        let param_len = mesh_list_length(params);
        for i in 0..param_len {
            let elem = mesh_list_get(params, i);
            fpar = mesh_list_append(fpar, elem);
        }
        query_set(new_q, SLOT_FRAGMENT_PARAMS, fpar);
        new_q
    }
}
