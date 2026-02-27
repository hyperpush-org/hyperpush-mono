# Phase 128 TRYFROM-02: 42.try_into() in a context expecting Result<PositiveInt, String>
# exercises the synthetic TryInto impl derived from impl TryFrom<Int> for PositiveInt.
# No explicit TryInto impl is written -- only TryFrom is user-defined.

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
  let r :: Result<PositiveInt, String> = 42.try_into()
  case r do
    Ok(p) -> println("${p.value}")
    Err(e) -> println("error: ${e}")
  end
  let r2 :: Result<PositiveInt, String> = (-5).try_into()
  case r2 do
    Ok(p) -> println("${p.value}")
    Err(e) -> println("${e}")
  end
end
