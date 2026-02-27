# Phase 128 TRYFROM-01: impl TryFrom<Int> for PositiveInt
# PositiveInt.try_from(42) returns Ok(PositiveInt { value: 42 })

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
  let r = PositiveInt.try_from(42)
  case r do
    Ok(p) -> println("${p.value}")
    Err(e) -> println("error: ${e}")
  end
end
