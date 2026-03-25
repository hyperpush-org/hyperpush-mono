# PostgreSQL partition lifecycle helpers for Mesher monitoring platform.
# Schema DDL is managed by migration files (mesher/migrations/).
# This module owns runtime partition create/list/drop behavior for the events table.
# Create daily partitions for the next N days using PostgreSQL current_date.

pub fn create_partitions_ahead(pool :: PoolHandle, days :: Int) -> Int ! String do
  Pg.create_daily_partitions_ahead(pool, "events", days)
end

# List event partitions older than the given retention window in days.
# Uses PostgreSQL catalog introspection and current_date inside the runtime helper.

pub fn get_expired_partitions(pool :: PoolHandle, max_days :: Int) -> List < String > ! String do
  Pg.list_daily_partitions_before(pool, "events", max_days)
end

# Drop a single event partition by name via the quoted PostgreSQL helper.

pub fn drop_partition(pool :: PoolHandle, partition_name :: String) -> Int ! String do
  Pg.drop_partition(pool, partition_name)
end
