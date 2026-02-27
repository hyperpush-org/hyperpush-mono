# Phase 128 TRYFROM-03: ? operator works on try_from result inside Result-returning fn

struct PositiveInt do
  value :: Int
end

impl TryFrom<Int> for PositiveInt do
  fn try_from(n :: Int) -> Result<PositiveInt, String> do
    if n > 0 do
      Ok(PositiveInt { value: n })
    else
      Err("must be positive")
    end
  end
end

fn double_positive(n :: Int) -> Int!String do
  let p = PositiveInt.try_from(n)?
  Ok(p.value * 2)
end

fn main() do
  let r1 = double_positive(21)
  case r1 do
    Ok(v) -> println("${v}")
    Err(e) -> println("error: ${e}")
  end
  let r2 = double_positive(-1)
  case r2 do
    Ok(v) -> println("${v}")
    Err(e) -> println("${e}")
  end
end
