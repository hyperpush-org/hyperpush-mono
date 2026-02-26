fn main() do
  let rx = ~r/\d+/
  let matched = Regex.is_match(rx, "hello123")
  if matched do
    println("digit found")
  else
    println("no digit")
  end
  let rx2 = ~r/[a-z]+/i
  let matched2 = Regex.is_match(rx2, "HELLO")
  if matched2 do
    println("case insensitive match")
  else
    println("no match")
  end
end
