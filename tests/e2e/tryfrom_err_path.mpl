# Phase 128 TRYFROM-01: try_from with failing validation returns Err

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

fn main() do
  let r = PositiveInt.try_from(-5)
  case r do
    Ok(p) -> println("${p.value}")
    Err(e) -> println("${e}")
  end
end
