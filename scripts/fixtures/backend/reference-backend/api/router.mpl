from Api.Health import handle_health
from Api.Jobs import handle_create_job, handle_get_job

pub fn build_router() do
  let router = HTTP.router()
    |> HTTP.on_get("/health", handle_health)
    |> HTTP.on_post("/jobs", handle_create_job)
    |> HTTP.on_get("/jobs/:id", handle_get_job)
  router
end
