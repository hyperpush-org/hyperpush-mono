fn print_parts(parts :: List<String>) do
  let p0 = List.get(parts, 0)
  let p1 = List.get(parts, 1)
  let p2 = List.get(parts, 2)
  let p3 = List.get(parts, 3)
  println(p0)
  println(p1)
  println(p2)
  println(p3)
end

fn run_replace_split(r :: Regex) do
  let replaced = Regex.replace(r, "foo123bar456", "N")
  println(replaced)
  let parts = Regex.split(r, "a1b2c3d")
  print_parts(parts)
end

fn main() do
  let rx = Regex.compile("\\d+")
  case rx do
    Ok(r) -> run_replace_split(r)
    Err(e) -> println("compile failed")
  end
end
