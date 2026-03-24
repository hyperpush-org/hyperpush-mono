actor boot_child() do
  println("child_boot")
end

supervisor BootSup do
  strategy: one_for_one
  max_restarts: 3
  max_seconds: 5

  child c1 do
    start: fn -> spawn(boot_child) end
    restart: temporary
    shutdown: 5000
  end
end

fn main() do
  let sup = BootSup()
  Timer.sleep(100)
  println("main_done")
end
