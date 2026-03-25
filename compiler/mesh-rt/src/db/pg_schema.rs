//! PostgreSQL-specific schema helpers for Mesh.
//!
//! These helpers intentionally own PostgreSQL-only DDL that should not be
//! represented by the neutral `Migration.*` surface.

use crate::collections::list::{
    mesh_list_append, mesh_list_get, mesh_list_length, mesh_list_new,
};
use crate::collections::map::mesh_map_get;
use crate::db::pool::{mesh_pool_execute, mesh_pool_query};
use crate::io::{alloc_result, MeshResult};
use crate::string::{mesh_string_new, MeshString};

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn quote_qualified_ident(value: &str, helper_name: &str) -> Result<String, String> {
    let mut parts = Vec::new();
    for part in value.split('.') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Err(format!("{helper_name}: identifier `{value}` contains an empty segment"));
        }
        parts.push(quote_ident(trimmed));
    }
    Ok(parts.join("."))
}

unsafe fn list_to_strings(list_ptr: *mut u8) -> Vec<String> {
    let len = mesh_list_length(list_ptr);
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        let elem = mesh_list_get(list_ptr, i) as *const MeshString;
        if !elem.is_null() {
            result.push((*elem).as_str().to_string());
        }
    }
    result
}

unsafe fn rust_string_to_mesh(s: &str) -> *mut u8 {
    mesh_string_new(s.as_ptr(), s.len() as u64) as *mut u8
}

unsafe fn strings_to_mesh_list(values: &[String]) -> *mut u8 {
    let mut list = mesh_list_new();
    for value in values {
        list = mesh_list_append(list, rust_string_to_mesh(value) as u64);
    }
    list
}

fn err_result(message: &str) -> *mut u8 {
    unsafe { alloc_result(1, rust_string_to_mesh(message)) as *mut u8 }
}

fn ok_int_result(value: i64) -> *mut u8 {
    let boxed = Box::into_raw(Box::new(value)) as *mut u8;
    alloc_result(0, boxed) as *mut u8
}

enum PartitionedTableEntry {
    Column { name: String, sql: String },
    Constraint(String),
}

fn render_partitioned_table_entry(
    column: &str,
    helper_name: &str,
) -> Result<PartitionedTableEntry, String> {
    let trimmed = column.trim();
    if trimmed.is_empty() {
        return Err(format!("{helper_name}: invalid column definition `{column}`"));
    }

    if !trimmed.contains(':') {
        return Ok(PartitionedTableEntry::Constraint(trimmed.to_string()));
    }

    let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
    match parts.as_slice() {
        [name, sql_type, constraints] if !name.trim().is_empty() => Ok(PartitionedTableEntry::Column {
            name: name.trim().to_string(),
            sql: format!("{} {} {}", quote_ident(name.trim()), sql_type.trim(), constraints.trim()),
        }),
        [name, sql_type] if !name.trim().is_empty() => Ok(PartitionedTableEntry::Column {
            name: name.trim().to_string(),
            sql: format!("{} {}", quote_ident(name.trim()), sql_type.trim()),
        }),
        _ => Err(format!("{helper_name}: invalid column definition `{column}`")),
    }
}

pub(crate) fn build_create_extension_sql(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Pg.create_extension: extension name must not be empty".to_string());
    }
    Ok(format!(
        "CREATE EXTENSION IF NOT EXISTS {}",
        quote_ident(trimmed)
    ))
}

pub(crate) fn build_create_range_partitioned_table_sql(
    table: &str,
    columns: &[String],
    partition_column: &str,
) -> Result<String, String> {
    let table = table.trim();
    if table.is_empty() {
        return Err("Pg.create_range_partitioned_table: table name must not be empty".to_string());
    }
    let partition_column = partition_column.trim();
    if partition_column.is_empty() {
        return Err(
            "Pg.create_range_partitioned_table: partition column must not be empty".to_string(),
        );
    }

    let rendered_entries: Vec<PartitionedTableEntry> = columns
        .iter()
        .map(|column| render_partitioned_table_entry(column, "Pg.create_range_partitioned_table"))
        .collect::<Result<_, _>>()?;
    if !rendered_entries
        .iter()
        .any(|entry| matches!(entry, PartitionedTableEntry::Column { .. }))
    {
        return Err(
            "Pg.create_range_partitioned_table: at least one column is required".to_string(),
        );
    }
    if !rendered_entries.iter().any(|entry| {
        matches!(
            entry,
            PartitionedTableEntry::Column { name, .. } if name == partition_column
        )
    }) {
        return Err(format!(
            "Pg.create_range_partitioned_table: partition column `{partition_column}` is missing from `{table}`"
        ));
    }

    let column_sql = rendered_entries
        .iter()
        .map(|entry| match entry {
            PartitionedTableEntry::Column { sql, .. } => sql.as_str(),
            PartitionedTableEntry::Constraint(sql) => sql.as_str(),
        })
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        "CREATE TABLE IF NOT EXISTS {} ({}) PARTITION BY RANGE ({})",
        quote_ident(table),
        column_sql,
        quote_ident(partition_column)
    ))
}

pub(crate) fn build_create_gin_index_sql(
    table: &str,
    index_name: &str,
    column: &str,
    opclass: &str,
) -> Result<String, String> {
    let table = table.trim();
    let index_name = index_name.trim();
    let column = column.trim();
    let opclass = opclass.trim();
    if table.is_empty() {
        return Err("Pg.create_gin_index: table name must not be empty".to_string());
    }
    if index_name.is_empty() {
        return Err("Pg.create_gin_index: index name must not be empty".to_string());
    }
    if column.is_empty() {
        return Err("Pg.create_gin_index: column name must not be empty".to_string());
    }
    if opclass.is_empty() {
        return Err("Pg.create_gin_index: opclass must not be empty".to_string());
    }

    Ok(format!(
        "CREATE INDEX IF NOT EXISTS {} ON {} USING GIN ({} {})",
        quote_ident(index_name),
        quote_ident(table),
        quote_ident(column),
        quote_qualified_ident(opclass, "Pg.create_gin_index")?
    ))
}

pub(crate) fn build_create_daily_partition_sql(
    parent_table: &str,
    offset_days: i64,
) -> Result<String, String> {
    let parent_table = parent_table.trim();
    if parent_table.is_empty() {
        return Err("Pg.create_daily_partitions_ahead: parent table must not be empty".to_string());
    }
    if offset_days < 0 {
        return Err(format!(
            "Pg.create_daily_partitions_ahead: days must be non-negative, got {offset_days}"
        ));
    }

    Ok(format!(
        "DO $mesh$ DECLARE parent_name text := {parent}; part_date date := current_date + {offset}; part_name text := parent_name || '_' || to_char(part_date, 'YYYYMMDD'); BEGIN EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)', part_name, parent_name, part_date, part_date + 1); END $mesh$;",
        parent = quote_literal(parent_table),
        offset = offset_days,
    ))
}

pub(crate) fn build_list_daily_partitions_before_sql() -> &'static str {
    "SELECT c.relname::text AS partition_name \
FROM pg_inherits i \
JOIN pg_class c ON c.oid = i.inhrelid \
JOIN pg_class p ON p.oid = i.inhparent \
WHERE p.relname = $1 \
  AND left(c.relname, char_length($1) + 1) = $1 || '_' \
  AND char_length(c.relname) = char_length($1) + 9 \
  AND substring(c.relname from char_length($1) + 2 for 8) ~ '^[0-9]{8}$' \
  AND to_date(substring(c.relname from char_length($1) + 2 for 8), 'YYYYMMDD') < (current_date - $2::int) \
ORDER BY c.relname"
}

pub(crate) fn build_drop_partition_sql(partition_name: &str) -> Result<String, String> {
    let trimmed = partition_name.trim();
    if trimmed.is_empty() {
        return Err("Pg.drop_partition: partition name must not be empty".to_string());
    }
    Ok(format!("DROP TABLE IF EXISTS {}", quote_ident(trimmed)))
}

#[no_mangle]
pub extern "C" fn mesh_pg_create_extension(pool: u64, name: *const MeshString) -> *mut u8 {
    unsafe {
        match build_create_extension_sql((*name).as_str()) {
            Ok(sql) => {
                let sql_ptr = rust_string_to_mesh(&sql) as *const MeshString;
                mesh_pool_execute(pool, sql_ptr, mesh_list_new())
            }
            Err(message) => err_result(&message),
        }
    }
}

#[no_mangle]
pub extern "C" fn mesh_pg_create_range_partitioned_table(
    pool: u64,
    table: *const MeshString,
    columns: *mut u8,
    partition_column: *const MeshString,
) -> *mut u8 {
    unsafe {
        let cols = list_to_strings(columns);
        match build_create_range_partitioned_table_sql(
            (*table).as_str(),
            &cols,
            (*partition_column).as_str(),
        ) {
            Ok(sql) => {
                let sql_ptr = rust_string_to_mesh(&sql) as *const MeshString;
                mesh_pool_execute(pool, sql_ptr, mesh_list_new())
            }
            Err(message) => err_result(&message),
        }
    }
}

#[no_mangle]
pub extern "C" fn mesh_pg_create_gin_index(
    pool: u64,
    table: *const MeshString,
    index_name: *const MeshString,
    column: *const MeshString,
    opclass: *const MeshString,
) -> *mut u8 {
    unsafe {
        match build_create_gin_index_sql(
            (*table).as_str(),
            (*index_name).as_str(),
            (*column).as_str(),
            (*opclass).as_str(),
        ) {
            Ok(sql) => {
                let sql_ptr = rust_string_to_mesh(&sql) as *const MeshString;
                mesh_pool_execute(pool, sql_ptr, mesh_list_new())
            }
            Err(message) => err_result(&message),
        }
    }
}

#[no_mangle]
pub extern "C" fn mesh_pg_create_daily_partitions_ahead(
    pool: u64,
    parent_table: *const MeshString,
    days: i64,
) -> *mut u8 {
    unsafe {
        let parent_table = (*parent_table).as_str();
        if days < 0 {
            return err_result(&format!(
                "Pg.create_daily_partitions_ahead: days must be non-negative, got {days}"
            ));
        }

        for offset in 0..days {
            let sql = match build_create_daily_partition_sql(parent_table, offset) {
                Ok(sql) => sql,
                Err(message) => return err_result(&message),
            };
            let sql_ptr = rust_string_to_mesh(&sql) as *const MeshString;
            let exec_result = mesh_pool_execute(pool, sql_ptr, mesh_list_new());
            let result = &*(exec_result as *const MeshResult);
            if result.tag != 0 {
                return exec_result;
            }
        }

        ok_int_result(0)
    }
}

#[no_mangle]
pub extern "C" fn mesh_pg_list_daily_partitions_before(
    pool: u64,
    parent_table: *const MeshString,
    max_days: i64,
) -> *mut u8 {
    unsafe {
        let parent_table = (*parent_table).as_str();
        if max_days < 0 {
            return err_result(&format!(
                "Pg.list_daily_partitions_before: max_days must be non-negative, got {max_days}"
            ));
        }

        let sql = build_list_daily_partitions_before_sql();
        let params = strings_to_mesh_list(&[parent_table.to_string(), max_days.to_string()]);
        let sql_ptr = rust_string_to_mesh(sql) as *const MeshString;
        let query_result = mesh_pool_query(pool, sql_ptr, params);
        let result = &*(query_result as *const MeshResult);
        if result.tag != 0 {
            return query_result;
        }

        let rows = result.value;
        let len = mesh_list_length(rows);
        let partition_name_key = rust_string_to_mesh("partition_name") as u64;
        let mut partitions = mesh_list_new();
        for i in 0..len {
            let row = mesh_list_get(rows, i) as *mut u8;
            let partition_name = mesh_map_get(row, partition_name_key);
            if partition_name == 0 {
                return err_result(
                    "Pg.list_daily_partitions_before: query row missing partition_name",
                );
            }
            partitions = mesh_list_append(partitions, partition_name);
        }

        alloc_result(0, partitions) as *mut u8
    }
}

#[no_mangle]
pub extern "C" fn mesh_pg_drop_partition(
    pool: u64,
    partition_name: *const MeshString,
) -> *mut u8 {
    unsafe {
        match build_drop_partition_sql((*partition_name).as_str()) {
            Ok(sql) => {
                let sql_ptr = rust_string_to_mesh(&sql) as *const MeshString;
                mesh_pool_execute(pool, sql_ptr, mesh_list_new())
            }
            Err(message) => err_result(&message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_pg_schema_build_create_extension_sql_quotes_identifier() {
        let sql = build_create_extension_sql("pgcrypto").expect("extension SQL should build");
        assert_eq!(sql, "CREATE EXTENSION IF NOT EXISTS \"pgcrypto\"");
    }

    #[test]
    fn migration_pg_schema_build_create_range_partitioned_table_sql_renders_partition_clause() {
        let sql = build_create_range_partitioned_table_sql(
            "events",
            &[
                "id:UUID:NOT NULL DEFAULT gen_random_uuid()".to_string(),
                "received_at:TIMESTAMPTZ:NOT NULL DEFAULT now()".to_string(),
            ],
            "received_at",
        )
        .expect("partitioned table SQL should build");
        assert_eq!(
            sql,
            "CREATE TABLE IF NOT EXISTS \"events\" (\"id\" UUID NOT NULL DEFAULT gen_random_uuid(), \"received_at\" TIMESTAMPTZ NOT NULL DEFAULT now()) PARTITION BY RANGE (\"received_at\")"
        );
    }

    #[test]
    fn migration_pg_schema_build_create_range_partitioned_table_sql_rejects_missing_partition_column() {
        let err = build_create_range_partitioned_table_sql(
            "events",
            &["id:UUID:PRIMARY KEY".to_string()],
            "received_at",
        )
        .expect_err("missing partition column should be rejected");
        assert_eq!(
            err,
            "Pg.create_range_partitioned_table: partition column `received_at` is missing from `events`"
        );
    }

    #[test]
    fn migration_pg_schema_build_create_range_partitioned_table_sql_allows_table_constraints() {
        let sql = build_create_range_partitioned_table_sql(
            "events",
            &[
                "id:UUID:NOT NULL DEFAULT gen_random_uuid()".to_string(),
                "received_at:TIMESTAMPTZ:NOT NULL DEFAULT now()".to_string(),
                "PRIMARY KEY (id, received_at)".to_string(),
            ],
            "received_at",
        )
        .expect("table constraints should be accepted for partitioned tables");
        assert!(sql.contains("PRIMARY KEY (id, received_at)"));
    }

    #[test]
    fn migration_pg_schema_build_create_gin_index_sql_renders_opclass() {
        let sql = build_create_gin_index_sql(
            "events",
            "idx_events_tags",
            "tags",
            "jsonb_path_ops",
        )
        .expect("GIN index SQL should build");
        assert_eq!(
            sql,
            "CREATE INDEX IF NOT EXISTS \"idx_events_tags\" ON \"events\" USING GIN (\"tags\" \"jsonb_path_ops\")"
        );
    }

    #[test]
    fn migration_pg_schema_build_create_daily_partition_sql_uses_database_clock() {
        let sql = build_create_daily_partition_sql("events", 3)
            .expect("daily partition SQL should build");
        assert!(sql.contains("part_date date := current_date + 3"));
        assert!(sql.contains("to_char(part_date, 'YYYYMMDD')"));
        assert!(sql.contains("PARTITION OF %I FOR VALUES FROM (%L) TO (%L)"));
    }

    #[test]
    fn migration_pg_schema_build_list_daily_partitions_before_sql_uses_catalog_query() {
        let sql = build_list_daily_partitions_before_sql();
        assert!(sql.contains("FROM pg_inherits"));
        assert!(sql.contains("current_date - $2::int"));
        assert!(sql.contains("left(c.relname, char_length($1) + 1) = $1 || '_'"));
    }

    #[test]
    fn migration_pg_schema_build_drop_partition_sql_quotes_identifier() {
        let sql = build_drop_partition_sql("events_20260216")
            .expect("drop partition SQL should build");
        assert_eq!(sql, "DROP TABLE IF EXISTS \"events_20260216\"");
    }
}
