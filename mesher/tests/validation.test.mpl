# Unit tests for Ingestion.Validation pure functions.
# No database or actor dependencies.
# Run with: meshc test mesher/tests/

from Ingestion.Validation import validate_level, validate_payload_size, validate_bulk_count

describe("validate_level") do
  test("accepts fatal") do
    let result = validate_level("fatal")
    case result do
      Ok(v) -> assert_eq(v, "valid")
      Err(_) -> assert(false)
    end
  end

  test("accepts error") do
    let result = validate_level("error")
    case result do
      Ok(v) -> assert_eq(v, "valid")
      Err(_) -> assert(false)
    end
  end

  test("accepts warning") do
    let result = validate_level("warning")
    case result do
      Ok(v) -> assert_eq(v, "valid")
      Err(_) -> assert(false)
    end
  end

  test("accepts info") do
    let result = validate_level("info")
    case result do
      Ok(v) -> assert_eq(v, "valid")
      Err(_) -> assert(false)
    end
  end

  test("accepts debug") do
    let result = validate_level("debug")
    case result do
      Ok(v) -> assert_eq(v, "valid")
      Err(_) -> assert(false)
    end
  end

  test("rejects critical") do
    let result = validate_level("critical")
    case result do
      Err(_) -> assert(true)
      Ok(_) -> assert(false)
    end
  end

  test("rejects empty string") do
    let result = validate_level("")
    case result do
      Err(_) -> assert(true)
      Ok(_) -> assert(false)
    end
  end
end

describe("validate_payload_size") do
  test("accepts body within limit") do
    let result = validate_payload_size("hello", 100)
    case result do
      Ok(v) -> assert_eq(v, "ok")
      Err(_) -> assert(false)
    end
  end

  test("accepts body exactly at limit") do
    let result = validate_payload_size("hello", 5)
    case result do
      Ok(v) -> assert_eq(v, "ok")
      Err(_) -> assert(false)
    end
  end

  test("rejects body over limit") do
    let result = validate_payload_size("hello world", 5)
    case result do
      Err(_) -> assert(true)
      Ok(_) -> assert(false)
    end
  end
end

describe("validate_bulk_count") do
  test("accepts count within limit") do
    let result = validate_bulk_count(50, 100)
    case result do
      Ok(v) -> assert_eq(v, "ok")
      Err(_) -> assert(false)
    end
  end

  test("accepts count exactly at limit") do
    let result = validate_bulk_count(100, 100)
    case result do
      Ok(v) -> assert_eq(v, "ok")
      Err(_) -> assert(false)
    end
  end

  test("rejects count over limit") do
    let result = validate_bulk_count(101, 100)
    case result do
      Err(_) -> assert(true)
      Ok(_) -> assert(false)
    end
  end
end
