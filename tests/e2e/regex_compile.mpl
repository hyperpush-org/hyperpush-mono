fn main() do
  let result1 = Regex.compile("\\d+")
  case result1 do
    Ok(rx) -> println("compiled ok")
    Err(e) -> println("error")
  end
  let result2 = Regex.compile("(unclosed")
  case result2 do
    Ok(rx) -> println("should not succeed")
    Err(e) -> println("got error")
  end
end
