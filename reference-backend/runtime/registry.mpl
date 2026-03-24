struct RegistryState do
  pool :: PoolHandle
  database_url :: String
  poll_ms :: Int
end

service RuntimeRegistry do
  fn init(pool :: PoolHandle, database_url :: String, poll_ms :: Int) -> RegistryState do
    RegistryState { pool : pool, database_url : database_url, poll_ms : poll_ms }
  end
  
  call GetPool() :: PoolHandle do|state|
    (state, state.pool)
  end
  
  call GetDatabaseUrl() :: String do|state|
    (state, state.database_url)
  end
  
  call GetPollMs() :: Int do|state|
    (state, state.poll_ms)
  end
end

pub fn start_registry(pool :: PoolHandle, database_url :: String, poll_ms :: Int) do
  let registry_pid = RuntimeRegistry.start(pool, database_url, poll_ms)
  let _ = Process.register("reference_backend_registry", registry_pid)
  registry_pid
end

pub fn get_pool() do
  let registry_pid = Process.whereis("reference_backend_registry")
  RuntimeRegistry.get_pool(registry_pid)
end

pub fn get_database_url() -> String do
  let registry_pid = Process.whereis("reference_backend_registry")
  RuntimeRegistry.get_database_url(registry_pid)
end

pub fn get_poll_ms() -> Int do
  let registry_pid = Process.whereis("reference_backend_registry")
  RuntimeRegistry.get_poll_ms(registry_pid)
end
