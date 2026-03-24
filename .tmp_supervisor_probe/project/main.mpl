actor crasher() do
  let x = 1 / 0
  println("after ${x}")
end

fn main() do
  spawn(crasher)
  Timer.sleep(100)
  println("main done")
end
