fn run_match(r :: Regex) do
  let m1 = Regex.is_match(r, "hello")
  if m1 do
    println("matched")
  else
    println("no match")
  end
  let m2 = Regex.is_match(r, "hello world")
  if m2 do
    println("matched")
  else
    println("no match")
  end
end

fn main() do
  let rx = Regex.compile("^hello$")
  case rx do
    Ok(r) -> run_match(r)
    Err(e) -> println("compile failed")
  end
end
