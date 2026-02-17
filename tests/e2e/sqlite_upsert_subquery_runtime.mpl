# E2E test for SQLite upsert, DELETE RETURNING, and subquery WHERE
# Phase 109: Verifies UPS-01, UPS-02, UPS-03

fn run_db() -> Int!String do
  let db = Sqlite.open(":memory:")?

  let _ = Sqlite.execute(db, "CREATE TABLE organizations (id TEXT PRIMARY KEY, name TEXT)", [])?
  let _ = Sqlite.execute(db, "CREATE TABLE projects (id TEXT PRIMARY KEY, org_id TEXT, name TEXT, status TEXT)", [])?

  let _ = Sqlite.execute(db, "INSERT INTO organizations VALUES (?, ?)", ["org1", "Acme Corp"])?
  let _ = Sqlite.execute(db, "INSERT INTO organizations VALUES (?, ?)", ["org2", "Beta Inc"])?
  let _ = Sqlite.execute(db, "INSERT INTO projects VALUES (?, ?, ?, ?)", ["p1", "org1", "Alpha", "active"])?
  let _ = Sqlite.execute(db, "INSERT INTO projects VALUES (?, ?, ?, ?)", ["p2", "org1", "Beta", "active"])?
  let _ = Sqlite.execute(db, "INSERT INTO projects VALUES (?, ?, ?, ?)", ["p3", "org2", "Gamma", "archived"])?

  # UPS-01 upsert insert (p4 does not exist yet)
  let ins_rows = Sqlite.query(db, "INSERT INTO projects (id, org_id, name, status) VALUES (?, ?, ?, ?) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, status = EXCLUDED.status RETURNING id, org_id, name, status", ["p4", "org2", "Delta", "active"])?
  let ins_row = List.head(ins_rows)
  println("upsert_insert:" <> Map.get(ins_row, "name"))

  # UPS-01 upsert update (p4 already exists, triggers DO UPDATE SET)
  let upd_rows = Sqlite.query(db, "INSERT INTO projects (id, org_id, name, status) VALUES (?, ?, ?, ?) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, status = EXCLUDED.status RETURNING id, org_id, name, status", ["p4", "org2", "Delta-Updated", "active"])?
  let upd_row = List.head(upd_rows)
  println("upsert_update:" <> Map.get(upd_row, "name"))

  # verify count stays at 4 (no duplicate from upsert)
  let count_rows = Sqlite.query(db, "SELECT count(*) AS cnt FROM projects", [])?
  let cr = List.head(count_rows)
  println("upsert_count:" <> Map.get(cr, "cnt"))

  # UPS-02 delete returning (delete p3 and get its data back)
  let del_rows = Sqlite.query(db, "DELETE FROM projects WHERE id = ? RETURNING id, org_id, name, status", ["p3"])?
  let del_row = List.head(del_rows)
  println("delete_ret_name:" <> Map.get(del_row, "name"))

  # verify p3 is actually gone
  let chk = Sqlite.query(db, "SELECT count(*) AS cnt FROM projects WHERE id = ?", ["p3"])?
  let chk_row = List.head(chk)
  println("delete_after_count:" <> Map.get(chk_row, "cnt"))

  # UPS-03 subquery (find projects belonging to Acme Corp via subquery)
  let sq = Sqlite.query(db, "SELECT name FROM projects WHERE org_id IN (SELECT id FROM organizations WHERE name = ?) ORDER BY name ASC", ["Acme Corp"])?
  let sq_first = List.head(sq)
  println("subquery_first:" <> Map.get(sq_first, "name"))

  Sqlite.close(db)
  Ok(0)
end

fn main() do
  let r = run_db()
  case r do
    Ok(_) -> println("done")
    Err(msg) -> println("error: " <> msg)
  end
end
