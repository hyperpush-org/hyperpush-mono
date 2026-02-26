fn print_caps(groups :: List<String>) do
  let full = List.get(groups, 0)
  let g1 = List.get(groups, 1)
  let g2 = List.get(groups, 2)
  println(full)
  println(g1)
  println(g2)
end

fn run_captures(r :: Regex) do
  let caps = Regex.captures(r, "hello world")
  case caps do
    Some(groups) -> print_caps(groups)
    None -> println("no captures")
  end
  let no_caps = Regex.captures(r, "123")
  case no_caps do
    Some(g) -> println("unexpected match")
    None -> println("no match")
  end
end

fn main() do
  let rx = Regex.compile("(\\w+) (\\w+)")
  case rx do
    Ok(r) -> run_captures(r)
    Err(e) -> println("compile failed")
  end
end
