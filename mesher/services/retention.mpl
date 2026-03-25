# Retention cleaner actor for periodic data cleanup.
# Runs daily to delete expired events per-project based on retention_days setting,
# then drops any event partitions older than the maximum retention period (90 days).
# Follows Timer.sleep + recursive call pattern (established in pipeline.mpl).

from Storage.Queries import delete_expired_events, get_all_project_retention
from Storage.Schema import get_expired_partitions, drop_partition

# Log helpers (extracted for single-expression case arms, decision [88-02]).

fn log_cleanup_result(deleted :: Int) do
  println("[Mesher] Retention cleanup: deleted #{deleted} expired events")
  0
end

fn log_cleanup_error(e :: String) do
  println("[Mesher] Retention cleanup error: #{e}")
  0
end

fn log_project_cleanup_error(project_id :: String, e :: String) do
  println("[Mesher] Retention event cleanup failed for project #{project_id}: #{e}")
  0
end

fn log_partition_list_error(e :: String) do
  println("[Mesher] Retention partition listing failed: #{e}")
  0
end

fn log_partition_drop(name :: String) do
  println("[Mesher] Dropped expired partition: #{name}")
  0
end

fn log_partition_drop_error(name :: String, e :: String) do
  println("[Mesher] Retention partition drop failed for #{name}: #{e}")
  0
end

fn stop_project_cleanup(project_id :: String, e :: String) -> Int ! String do
  let message = "Retention event cleanup failed for project #{project_id}: #{e}"
  log_project_cleanup_error(project_id, e)
  Err(message)
end

fn stop_partition_listing(e :: String) -> Int ! String do
  let message = "Retention partition listing failed: #{e}"
  log_partition_list_error(e)
  Err(message)
end

fn stop_partition_drop(partition_name :: String, e :: String) -> Int ! String do
  let message = "Retention partition drop failed for #{partition_name}: #{e}"
  log_partition_drop_error(partition_name, e)
  Err(message)
end

# Loop through projects list by index, deleting expired events for each.
# Accumulates total deleted count across all projects.

fn cleanup_projects_loop(pool :: PoolHandle, projects, i :: Int, total :: Int, deleted :: Int) -> Int ! String do
  if i < total do
    let row = List.get(projects, i)
    let id = Map.get(row, "id")
    let retention_days_str = Map.get(row, "retention_days")
    let count_result = delete_expired_events(pool, id, retention_days_str)
    case count_result do
      Ok( count) -> cleanup_projects_loop(pool, projects, i + 1, total, deleted + count)
      Err( e) -> stop_project_cleanup(id, e)
    end
  else
    Ok(deleted)
  end
end

fn continue_partition_drop(pool :: PoolHandle,
partitions,
i :: Int,
total :: Int,
partition_name :: String) -> Int ! String do
  log_partition_drop(partition_name)
  drop_partitions_loop(pool, partitions, i + 1, total)
end

# Loop through expired partitions, dropping each one.

fn drop_partitions_loop(pool :: PoolHandle, partitions, i :: Int, total :: Int) -> Int ! String do
  if i < total do
    let partition_name = List.get(partitions, i)
    let drop_result = drop_partition(pool, partition_name)
    case drop_result do
      Ok( _) -> continue_partition_drop(pool, partitions, i, total, partition_name)
      Err( e) -> stop_partition_drop(partition_name, e)
    end
  else
    Ok(total)
  end
end

fn finish_partition_cleanup(pool :: PoolHandle, deleted :: Int, partitions) -> Int ! String do
  drop_partitions_loop(pool, partitions, 0, List.length(partitions)) ?
  Ok(deleted)
end

# Orchestration: run per-project deletion then global partition cleanup.

fn run_retention_cleanup(pool :: PoolHandle) -> Int ! String do
  let projects = get_all_project_retention(pool) ?
  let deleted = cleanup_projects_loop(pool, projects, 0, List.length(projects), 0) ?
  let partitions_result = get_expired_partitions(pool, 90)
  case partitions_result do
    Ok( partitions) -> finish_partition_cleanup(pool, deleted, partitions)
    Err( e) -> stop_partition_listing(e)
  end
end

# Retention cleaner actor -- runs every 24 hours (86400000ms).
# Iterates all projects to delete expired events per their retention_days setting,
# then drops any partitions older than 90 days (the maximum retention period).

actor retention_cleaner(pool :: PoolHandle) do
  Timer.sleep(86400000)
  
  let result = run_retention_cleanup(pool)
  
  case result do
    Ok( n) -> log_cleanup_result(n)
    Err( e) -> log_cleanup_error(e)
  end
  
  retention_cleaner(pool)
end
