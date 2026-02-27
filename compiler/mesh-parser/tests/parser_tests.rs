//! Parser integration tests using insta snapshots.
//!
//! Each test parses a Mesh expression/declaration/program, builds the CST,
//! and snapshots the debug tree output to verify correct structure.

use insta::assert_snapshot;
use mesh_parser::ast::expr::{BinaryExpr, ForInExpr, IfExpr, Literal};
use mesh_parser::ast::item::{
    FnDef, LetBinding, ServiceDef, SourceFile, StructDef, SumTypeDef,
};
use mesh_parser::SyntaxKind;
use mesh_parser::ast::pat::{AsPat, ConstructorPat, OrPat, Pattern};
use mesh_parser::{debug_tree, parse, parse_block, parse_expr, AstNode};

fn parse_and_debug(source: &str) -> String {
    let parse = parse_expr(source);
    format_parse(&parse)
}

fn block_and_debug(source: &str) -> String {
    let parse = parse_block(source);
    format_parse(&parse)
}

fn source_and_debug(source: &str) -> String {
    let parse = parse(source);
    format_parse(&parse)
}

fn format_parse(parse: &mesh_parser::Parse) -> String {
    let tree = debug_tree(&parse.syntax());
    if !parse.errors().is_empty() {
        format!(
            "{}\nerrors:\n{}",
            tree,
            parse
                .errors()
                .iter()
                .map(|e| format!("  - {} @{}..{}", e.message, e.span.start, e.span.end))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        tree
    }
}

// ── Literals ───────────────────────────────────────────────────────────

#[test]
fn literal_int() {
    assert_snapshot!(parse_and_debug("42"));
}

#[test]
fn literal_float() {
    assert_snapshot!(parse_and_debug("3.14"));
}

#[test]
fn literal_true() {
    assert_snapshot!(parse_and_debug("true"));
}

#[test]
fn literal_false() {
    assert_snapshot!(parse_and_debug("false"));
}

#[test]
fn literal_nil() {
    assert_snapshot!(parse_and_debug("nil"));
}

#[test]
fn literal_string() {
    assert_snapshot!(parse_and_debug("\"hello\""));
}

// ── Simple Binary Expressions ──────────────────────────────────────────

#[test]
fn binary_add() {
    assert_snapshot!(parse_and_debug("1 + 2"));
}

#[test]
fn binary_mul_add_precedence() {
    // * binds tighter than +, so: a * b + c => (a * b) + c
    assert_snapshot!(parse_and_debug("a * b + c"));
}

// ── Precedence Chain ───────────────────────────────────────────────────

#[test]
fn precedence_chain() {
    // 1 + 2 * 3 - 4 / 2 => (1 + (2 * 3)) - (4 / 2)
    assert_snapshot!(parse_and_debug("1 + 2 * 3 - 4 / 2"));
}

// ── Unary Prefix ───────────────────────────────────────────────────────

#[test]
fn unary_negate() {
    assert_snapshot!(parse_and_debug("-x"));
}

#[test]
fn unary_bang() {
    assert_snapshot!(parse_and_debug("!flag"));
}

#[test]
fn unary_not_keyword() {
    assert_snapshot!(parse_and_debug("not done"));
}

// ── Unary with Binary ──────────────────────────────────────────────────

#[test]
fn unary_with_binary() {
    // -x + y => (-x) + y (unary binds tighter)
    assert_snapshot!(parse_and_debug("-x + y"));
}

// ── Comparison ─────────────────────────────────────────────────────────

#[test]
fn comparison_eq() {
    assert_snapshot!(parse_and_debug("a == b"));
}

#[test]
fn comparison_lt_with_arithmetic() {
    // x < y + 1 => x < (y + 1) (+ binds tighter than <)
    assert_snapshot!(parse_and_debug("x < y + 1"));
}

// ── Logical ────────────────────────────────────────────────────────────

#[test]
fn logical_and_or() {
    // a and b or c => (a and b) or c (and binds tighter than or)
    assert_snapshot!(parse_and_debug("a and b or c"));
}

// ── Pipe ───────────────────────────────────────────────────────────────

#[test]
fn pipe_simple() {
    assert_snapshot!(parse_and_debug("x |> foo()"));
}

#[test]
fn pipe_chain() {
    // x |> foo() |> bar() => ((x |> foo()) |> bar()) -- left-associative
    assert_snapshot!(parse_and_debug("x |> foo() |> bar()"));
}

// ── Function Calls ─────────────────────────────────────────────────────

#[test]
fn call_no_args() {
    assert_snapshot!(parse_and_debug("foo()"));
}

#[test]
fn call_with_args() {
    assert_snapshot!(parse_and_debug("foo(1, 2, 3)"));
}

#[test]
fn call_with_expr_arg() {
    assert_snapshot!(parse_and_debug("foo(a, b + c)"));
}

// ── Nested Calls ───────────────────────────────────────────────────────

#[test]
fn nested_calls() {
    assert_snapshot!(parse_and_debug("foo(bar(x))"));
}

// ── Field Access ───────────────────────────────────────────────────────

#[test]
fn field_access_single() {
    assert_snapshot!(parse_and_debug("a.b"));
}

#[test]
fn field_access_chain() {
    // a.b.c => (a.b).c -- left-to-right
    assert_snapshot!(parse_and_debug("a.b.c"));
}

// ── Index Access ───────────────────────────────────────────────────────

#[test]
fn index_access() {
    assert_snapshot!(parse_and_debug("a[0]"));
}

#[test]
fn index_with_expr() {
    assert_snapshot!(parse_and_debug("a[i + 1]"));
}

// ── Mixed Postfix ──────────────────────────────────────────────────────

#[test]
fn mixed_postfix() {
    // a.b(c)[d] => ((a.b)(c))[d]
    assert_snapshot!(parse_and_debug("a.b(c)[d]"));
}

// ── Grouped Expression ─────────────────────────────────────────────────

#[test]
fn grouped_expression() {
    // (a + b) * c => (group(a + b)) * c
    assert_snapshot!(parse_and_debug("(a + b) * c"));
}

// ── String Interpolation ───────────────────────────────────────────────

#[test]
fn string_interpolation() {
    assert_snapshot!(parse_and_debug("\"hello ${name} world\""));
}

// ── Pipe with Calls ────────────────────────────────────────────────────

#[test]
fn pipe_with_calls() {
    assert_snapshot!(parse_and_debug("data |> map(f) |> filter(g)"));
}

// ── Error Cases ────────────────────────────────────────────────────────

#[test]
fn error_missing_lhs() {
    // + by itself should produce an error
    assert_snapshot!(parse_and_debug("+"));
}

// ── Range ──────────────────────────────────────────────────────────────

#[test]
fn range_operator() {
    assert_snapshot!(parse_and_debug("1..10"));
}

// ── Concatenation ──────────────────────────────────────────────────────

#[test]
fn concat_diamond() {
    assert_snapshot!(parse_and_debug("a <> b"));
}

#[test]
fn concat_plus_plus() {
    assert_snapshot!(parse_and_debug("a ++ b"));
}

// ── Tuple ──────────────────────────────────────────────────────────────

#[test]
fn tuple_expression() {
    assert_snapshot!(parse_and_debug("(1, 2, 3)"));
}

#[test]
fn empty_tuple() {
    assert_snapshot!(parse_and_debug("()"));
}

// ── Modulo ─────────────────────────────────────────────────────────────

#[test]
fn modulo_operator() {
    assert_snapshot!(parse_and_debug("a % b"));
}

// ── Let Bindings ──────────────────────────────────────────────────────

#[test]
fn let_simple() {
    assert_snapshot!(block_and_debug("let x = 5"));
}

#[test]
fn let_with_type_annotation() {
    assert_snapshot!(block_and_debug("let name :: String = \"hello\""));
}

#[test]
fn let_multiple_statements() {
    assert_snapshot!(block_and_debug("let x = 1\nlet y = 2"));
}

// ── Return ────────────────────────────────────────────────────────────

#[test]
fn return_with_value() {
    assert_snapshot!(block_and_debug("return x"));
}

#[test]
fn return_with_expr() {
    assert_snapshot!(block_and_debug("return x + 1"));
}

// ── If/Else ───────────────────────────────────────────────────────────

#[test]
fn if_simple() {
    assert_snapshot!(parse_and_debug("if true do\n  1\nend"));
}

#[test]
fn if_else() {
    assert_snapshot!(parse_and_debug("if x > 0 do\n  x\nelse\n  -x\nend"));
}

#[test]
fn if_else_if_else() {
    assert_snapshot!(parse_and_debug("if a do\n  1\nelse if b do\n  2\nelse\n  3\nend"));
}

#[test]
fn if_single_line() {
    assert_snapshot!(parse_and_debug("if true do 1 end"));
}

// ── Case/Match ────────────────────────────────────────────────────────

#[test]
fn case_simple() {
    assert_snapshot!(parse_and_debug("case x do\n  1 -> \"one\"\n  2 -> \"two\"\nend"));
}

#[test]
fn match_boolean() {
    assert_snapshot!(parse_and_debug("match value do\n  true -> 1\n  false -> 0\nend"));
}

// ── Closures ──────────────────────────────────────────────────────────

#[test]
fn closure_single_param() {
    assert_snapshot!(parse_and_debug("fn (x) -> x + 1 end"));
}

#[test]
fn closure_two_params() {
    assert_snapshot!(parse_and_debug("fn (x, y) -> x + y end"));
}

#[test]
fn closure_no_params() {
    assert_snapshot!(parse_and_debug("fn () -> 42 end"));
}

// ── Closures (Phase 12: bare params, do/end, multi-clause, guards) ──

#[test]
fn closure_bare_single_param() {
    assert_snapshot!(parse_and_debug("fn x -> x + 1 end"));
}

#[test]
fn closure_bare_two_params() {
    assert_snapshot!(parse_and_debug("fn x, y -> x + y end"));
}

#[test]
fn closure_bare_param_pattern_matching() {
    assert_snapshot!(parse_and_debug("fn 0 -> 42 end"));
}

#[test]
fn closure_do_end_body() {
    // Use a let binding so the closure is parsed in expression context
    // (at statement level, `fn x do...end` is parsed as a named fn def)
    assert_snapshot!(source_and_debug("let f = fn x do\n  let y = x * 2\n  y + 1\nend"));
}

#[test]
fn closure_do_end_no_params() {
    assert_snapshot!(parse_and_debug("fn do 42 end"));
}

#[test]
fn closure_multi_clause() {
    assert_snapshot!(parse_and_debug("fn 0 -> 42 | n -> n + 1 end"));
}

#[test]
fn closure_multi_clause_three() {
    assert_snapshot!(parse_and_debug("fn 0 -> 42 | 1 -> 99 | n -> n + 1 end"));
}

#[test]
fn closure_guard_clause() {
    assert_snapshot!(parse_and_debug("fn x when x > 0 -> x end"));
}

#[test]
fn closure_multi_clause_with_guards() {
    assert_snapshot!(parse_and_debug("fn x when x > 0 -> x | x -> 0 - x end"));
}

#[test]
fn closure_in_pipe_chain() {
    assert_snapshot!(source_and_debug("fn main() do\n  let list = (1, 2, 3)\n  list |> map(fn x -> x * 2 end)\nend"));
}

#[test]
fn closure_chained_pipes() {
    assert_snapshot!(parse_and_debug("list |> map(fn x -> x + 1 end) |> filter(fn x -> x > 3 end)"));
}

#[test]
fn closure_nested_do_end_in_body() {
    assert_snapshot!(parse_and_debug("fn x -> if x > 0 do x else 0 - x end end"));
}

#[test]
fn closure_paren_params_still_work() {
    // Confirms existing parenthesized syntax is unchanged
    assert_snapshot!(parse_and_debug("fn (x) -> x + 1 end"));
}

#[test]
fn closure_constructor_pattern() {
    assert_snapshot!(parse_and_debug("fn Some(x) -> x | None -> 0 end"));
}

#[test]
fn closure_wildcard_param() {
    assert_snapshot!(parse_and_debug("fn _ -> 42 end"));
}

#[test]
fn closure_tuple_pattern_param() {
    assert_snapshot!(parse_and_debug("fn (a, b) -> a + b end"));
}

#[test]
fn closure_pipe_inside_closure_body() {
    assert_snapshot!(parse_and_debug("fn x -> x |> to_string() end"));
}

#[test]
fn closure_missing_end_error() {
    assert_snapshot!(parse_and_debug("fn x -> x + 1"));
}

// ── Blocks ────────────────────────────────────────────────────────────

#[test]
fn block_multi_statement() {
    assert_snapshot!(block_and_debug("let x = 1\nx + 1"));
}

// ── Trailing Closures ────────────────────────────────────────────────

#[test]
fn trailing_closure_basic() {
    assert_snapshot!(parse_and_debug("run() do\n  42\nend"));
}

// ── Error Cases (compound) ───────────────────────────────────────────

#[test]
fn error_if_missing_end() {
    assert_snapshot!(parse_and_debug("if x do\n  1\n"));
}

#[test]
fn error_let_missing_ident() {
    assert_snapshot!(block_and_debug("let = 5"));
}

// ── Newline Significance ─────────────────────────────────────────────

#[test]
fn newlines_inside_parens_ignored() {
    assert_snapshot!(parse_and_debug("foo(\n  1,\n  2\n)"));
}

// ── Return bare (no value) ───────────────────────────────────────────

#[test]
fn return_bare() {
    assert_snapshot!(block_and_debug("return"));
}

// ── Case with when guard ─────────────────────────────────────────────

#[test]
fn case_with_when_guard() {
    assert_snapshot!(parse_and_debug("case x do\n  n when n > 0 -> n\n  _ -> 0\nend"));
}

// ═══════════════════════════════════════════════════════════════════════
// Plan 02-04: Declarations, Patterns, and Types
// ═══════════════════════════════════════════════════════════════════════

// ── Function Definitions ─────────────────────────────────────────────

#[test]
fn fn_def_simple() {
    assert_snapshot!(source_and_debug("fn greet(name) do\n  \"hello\"\nend"));
}

#[test]
fn fn_def_pub() {
    assert_snapshot!(source_and_debug("pub fn add(x, y) do\n  x + y\nend"));
}

#[test]
fn fn_def_typed_params_and_return() {
    assert_snapshot!(source_and_debug("fn typed(x :: Int, y :: Int) -> Int do\n  x + y\nend"));
}

#[test]
fn def_keyword() {
    assert_snapshot!(source_and_debug("def greet(name) do\n  \"hello\"\nend"));
}

#[test]
fn fn_def_no_params() {
    assert_snapshot!(source_and_debug("fn hello() do\n  \"world\"\nend"));
}

// ── Module Definitions ──────────────────────────────────────────────

#[test]
fn module_simple() {
    assert_snapshot!(source_and_debug("module Math do\n  pub fn add(x, y) do\n    x + y\n  end\nend"));
}

#[test]
fn module_nested() {
    assert_snapshot!(source_and_debug("module Outer do\n  module Inner do\n  end\nend"));
}

// ── Import Declarations ─────────────────────────────────────────────

#[test]
fn import_simple() {
    assert_snapshot!(source_and_debug("import Math"));
}

#[test]
fn import_dotted_path() {
    assert_snapshot!(source_and_debug("import Foo.Bar.Baz"));
}

#[test]
fn from_import() {
    assert_snapshot!(source_and_debug("from Math import sqrt, pow"));
}

// ── Struct Definitions ──────────────────────────────────────────────

#[test]
fn struct_simple() {
    assert_snapshot!(source_and_debug("struct Point do\n  x :: Float\n  y :: Float\nend"));
}

#[test]
fn struct_pub_with_generics() {
    assert_snapshot!(source_and_debug("pub struct Pair<A, B> do\n  first :: A\n  second :: B\nend"));
}

// ── Pattern Matching ────────────────────────────────────────────────

#[test]
fn case_with_literal_patterns() {
    assert_snapshot!(parse_and_debug("case x do\n  0 -> \"zero\"\n  _ -> \"other\"\nend"));
}

#[test]
fn let_tuple_destructure() {
    assert_snapshot!(source_and_debug("let (a, b) = pair"));
}

#[test]
fn case_with_tuple_patterns() {
    assert_snapshot!(parse_and_debug("case point do\n  (0, 0) -> \"origin\"\n  (x, y) -> \"other\"\nend"));
}

#[test]
fn case_with_negative_literal() {
    assert_snapshot!(parse_and_debug("case x do\n  -1 -> \"neg\"\n  0 -> \"zero\"\n  1 -> \"pos\"\nend"));
}

#[test]
fn case_with_string_pattern() {
    assert_snapshot!(parse_and_debug("case s do\n  \"hello\" -> 1\n  _ -> 0\nend"));
}

// ── Full Programs (integration) ─────────────────────────────────────

#[test]
fn full_program_module_with_fns() {
    let source = "\
module Math do
  pub fn add(x, y) do
    x + y
  end

  pub fn sub(x, y) do
    x - y
  end
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_program_imports_and_pipes() {
    let source = "\
import IO
from Math import sqrt

fn main() do
  42 |> sqrt() |> IO.puts()
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_program_struct_and_fn() {
    let source = "\
struct Point do
  x :: Float
  y :: Float
end

fn distance(p :: Point) -> Float do
  p.x + p.y
end";
    assert_snapshot!(source_and_debug(source));
}

// ── Error Cases (declarations) ──────────────────────────────────────

#[test]
fn error_fn_missing_name() {
    assert_snapshot!(source_and_debug("fn do end"));
}

#[test]
fn error_glob_import() {
    assert_snapshot!(source_and_debug("from Math import *"));
}

// ── Public parse() API end-to-end ───────────────────────────────────

#[test]
fn parse_api_simple_expression() {
    // parse() should work with simple expressions
    assert_snapshot!(source_and_debug("1 + 2"));
}

#[test]
fn parse_api_let_binding() {
    assert_snapshot!(source_and_debug("let x = 42"));
}

// ═══════════════════════════════════════════════════════════════════════
// Plan 02-05: Typed AST Wrappers and Comprehensive Tests
// ═══════════════════════════════════════════════════════════════════════

// ── Full Program Snapshot Tests ──────────────────────────────────────

#[test]
fn full_complete_module() {
    let source = "\
module StringUtils do
  pub fn upcase(s :: String) -> String do
    s
  end

  pub fn downcase(s :: String) -> String do
    s
  end

  fn helper(x) do
    x
  end
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_program_with_imports_pipes_closures() {
    let source = "\
import IO
from List import map, filter

fn main() do
  let nums = (1, 2, 3, 4, 5)
  nums |> filter(fn (x) -> x > 2 end) |> map(fn (x) -> x * 2 end)
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_nested_if_else_with_case() {
    let source = "\
fn classify(x) do
  if x > 0 do
    case x do
      1 -> \"one\"
      2 -> \"two\"
      _ -> \"many\"
    end
  else if x == 0 do
    \"zero\"
  else
    \"negative\"
  end
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_struct_definition_and_usage() {
    let source = "\
pub struct Point<T> do
  x :: T
  y :: T
end

fn origin() -> Point do
  let p = Point
  p
end

fn translate(p :: Point, dx :: Float, dy :: Float) -> Point do
  p
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_multiple_imports_and_nested_modules() {
    let source = "\
import IO
import Math
from String import split, join, trim

module App do
  module Config do
    fn default() do
      42
    end
  end

  pub fn run() do
    let cfg = Config.default()
    cfg |> IO.inspect()
  end
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_pattern_matching_program() {
    let source = "\
fn fibonacci(n) do
  case n do
    0 -> 0
    1 -> 1
    _ -> fibonacci(n - 1) + fibonacci(n - 2)
  end
end

fn classify_pair(pair) do
  case pair do
    (0, 0) -> \"origin\"
    (x, 0) -> \"x-axis\"
    (0, y) -> \"y-axis\"
    (x, y) -> \"general\"
  end
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_closure_and_higher_order() {
    let source = "\
fn apply(f, x) do
  f(x)
end

fn compose(f, g) do
  fn (x) -> f(g(x)) end
end

fn main() do
  let double = fn (x) -> x * 2 end
  let inc = fn (x) -> x + 1 end
  let double_then_inc = compose(inc, double)
  apply(double_then_inc, 5)
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn full_chained_pipes_and_field_access() {
    let source = "\
fn process(data) do
  data.items
    |> filter(fn (x) -> x.active end)
    |> map(fn (x) -> x.value end)
end";
    assert_snapshot!(source_and_debug(source));
}

// ── AST Accessor Tests ──────────────────────────────────────────────

#[test]
fn ast_fn_def_accessors() {
    let p = parse("pub fn add(x, y) do\n  x + y\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().expect("should have fn def");

    // Visibility
    assert!(fn_def.visibility().is_some(), "should have pub visibility");

    // Name
    let name = fn_def.name().expect("should have name");
    assert_eq!(name.text().unwrap(), "add");

    // Param list
    let param_list = fn_def.param_list().expect("should have param list");
    let params: Vec<_> = param_list.params().collect();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name().unwrap().text(), "x");
    assert_eq!(params[1].name().unwrap().text(), "y");

    // Body
    assert!(fn_def.body().is_some(), "should have body block");
}

#[test]
fn ast_fn_def_with_return_type() {
    let p = parse("fn typed(x :: Int) -> Int do\n  x\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().expect("should have fn def");

    // No visibility
    assert!(fn_def.visibility().is_none());

    // Return type
    let ret_type = fn_def.return_type().expect("should have return type");
    assert_eq!(ret_type.type_name().unwrap().text(), "Int");

    // Param with type annotation
    let param_list = fn_def.param_list().unwrap();
    let param = param_list.params().next().unwrap();
    assert_eq!(param.name().unwrap().text(), "x");
    let ann = param.type_annotation().expect("should have type annotation");
    assert_eq!(ann.type_name().unwrap().text(), "Int");
}

#[test]
fn ast_let_binding_accessors() {
    let p = parse("let x :: Int = 5");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let let_binding: LetBinding = tree
        .syntax()
        .children()
        .find_map(LetBinding::cast)
        .expect("should have let binding");

    // Name
    let name = let_binding.name().expect("should have name");
    assert_eq!(name.text().unwrap(), "x");

    // Type annotation
    let ann = let_binding.type_annotation().expect("should have type annotation");
    assert_eq!(ann.type_name().unwrap().text(), "Int");

    // Initializer
    let init = let_binding.initializer().expect("should have initializer");
    match init {
        mesh_parser::ast::expr::Expr::Literal(_) => {} // expected
        other => panic!("expected Literal, got {:?}", other),
    }
}

#[test]
fn ast_if_expr_accessors() {
    let p = parse_expr("if true do 1 else 2 end");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    let if_expr: IfExpr = root
        .children()
        .find_map(IfExpr::cast)
        .expect("should have if expr");

    // Condition
    let cond = if_expr.condition().expect("should have condition");
    match cond {
        mesh_parser::ast::expr::Expr::Literal(_) => {} // true
        other => panic!("expected Literal condition, got {:?}", other),
    }

    // Then branch
    assert!(if_expr.then_branch().is_some(), "should have then branch");

    // Else branch
    let else_br = if_expr.else_branch().expect("should have else branch");
    assert!(else_br.block().is_some(), "else should have a block");
}

#[test]
fn ast_struct_def_accessors() {
    let p = parse("pub struct Point do\n  x :: Float\n  y :: Float\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let struct_def: StructDef = tree
        .syntax()
        .children()
        .find_map(StructDef::cast)
        .expect("should have struct def");

    // Visibility
    assert!(struct_def.visibility().is_some());

    // Name
    assert_eq!(struct_def.name().unwrap().text().unwrap(), "Point");

    // Fields
    let fields: Vec<_> = struct_def.fields().collect();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name().unwrap().text().unwrap(), "x");
    assert_eq!(
        fields[0].type_annotation().unwrap().type_name().unwrap().text(),
        "Float"
    );
    assert_eq!(fields[1].name().unwrap().text().unwrap(), "y");
}

#[test]
fn ast_source_file_items() {
    let source = "\
fn foo() do 1 end
fn bar() do 2 end";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let fns: Vec<_> = tree.fn_defs().collect();
    assert_eq!(fns.len(), 2);
    assert_eq!(fns[0].name().unwrap().text().unwrap(), "foo");
    assert_eq!(fns[1].name().unwrap().text().unwrap(), "bar");
}

#[test]
fn ast_binary_expr_accessors() {
    let p = parse_expr("1 + 2");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    let binary: BinaryExpr = root
        .children()
        .find_map(BinaryExpr::cast)
        .expect("should have binary expr");

    // LHS
    let lhs = binary.lhs().expect("should have lhs");
    match lhs {
        mesh_parser::ast::expr::Expr::Literal(_) => {}
        other => panic!("expected Literal lhs, got {:?}", other),
    }

    // RHS
    let rhs = binary.rhs().expect("should have rhs");
    match rhs {
        mesh_parser::ast::expr::Expr::Literal(_) => {}
        other => panic!("expected Literal rhs, got {:?}", other),
    }

    // Operator
    let op = binary.op().expect("should have op token");
    assert_eq!(op.text(), "+");
}

#[test]
fn ast_literal_token() {
    let p = parse_expr("42");
    assert!(p.ok());
    let root = p.syntax();

    let lit: Literal = root
        .children()
        .find_map(Literal::cast)
        .expect("should have literal");

    let token = lit.token().expect("should have token");
    assert_eq!(token.text(), "42");
}

#[test]
fn ast_import_accessors() {
    let p = parse("from Foo.Bar import baz, qux");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let from_import: mesh_parser::ast::item::FromImportDecl = tree
        .syntax()
        .children()
        .find_map(mesh_parser::ast::item::FromImportDecl::cast)
        .expect("should have from import");

    let path = from_import.module_path().expect("should have module path");
    assert_eq!(path.segments(), vec!["Foo", "Bar"]);

    let import_list = from_import.import_list().expect("should have import list");
    let names: Vec<_> = import_list
        .names()
        .map(|n| n.text().unwrap())
        .collect();
    assert_eq!(names, vec!["baz", "qux"]);
}

#[test]
fn ast_module_accessors() {
    let source = "\
pub module Math do
  pub fn add(x, y) do
    x + y
  end
end";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let module: mesh_parser::ast::item::ModuleDef = tree
        .modules()
        .next()
        .expect("should have module");

    assert!(module.visibility().is_some());
    assert_eq!(module.name().unwrap().text().unwrap(), "Math");

    let items: Vec<_> = module.items().collect();
    assert_eq!(items.len(), 1);
    match &items[0] {
        mesh_parser::ast::item::Item::FnDef(_) => {}
        other => panic!("expected FnDef, got {:?}", other),
    }
}

#[test]
fn ast_parse_tree_convenience() {
    let p = parse("fn hello() do 42 end");
    assert!(p.ok());
    let tree: SourceFile = p.tree();
    let fn_def = tree.fn_defs().next().unwrap();
    assert_eq!(fn_def.name().unwrap().text().unwrap(), "hello");
}

// ── Error Message Quality Tests ─────────────────────────────────────

#[test]
fn error_fn_missing_end_references_do_span() {
    // fn foo() do 1 (missing end) -> error should reference the do span
    let p = parse("fn foo() do\n  1\n");
    assert!(!p.ok());
    let err = &p.errors()[0];
    assert!(
        err.message.contains("end"),
        "error should mention `end`: {}",
        err.message
    );
    assert!(
        err.related.is_some(),
        "error should have related span referencing `do`"
    );
    let (related_msg, _) = err.related.as_ref().unwrap();
    assert!(
        related_msg.contains("do"),
        "related message should mention `do`: {}",
        related_msg
    );
}

#[test]
fn error_glob_import_message() {
    // from Math import * -> error about glob imports
    let p = parse("from Math import *");
    assert!(!p.ok());
    let err = &p.errors()[0];
    assert!(
        err.message.contains("glob"),
        "error should mention glob: {}",
        err.message
    );
}

#[test]
fn error_let_missing_identifier_message() {
    // let = 5 -> error about missing identifier
    let p = parse("let = 5");
    assert!(!p.ok());
    let err = &p.errors()[0];
    assert!(
        err.message.contains("identifier") || err.message.contains("pattern"),
        "error should mention identifier or pattern: {}",
        err.message
    );
}

#[test]
fn error_if_missing_do() {
    // if true 1 end -> error about missing do
    let p = parse_expr("if true 1 end");
    assert!(!p.ok());
    let err = &p.errors()[0];
    assert!(
        err.message.contains("DO_KW") || err.message.contains("do") || err.message.contains("expected"),
        "error should mention do: {}",
        err.message
    );
}

#[test]
fn error_struct_missing_name() {
    let p = parse("struct do end");
    assert!(!p.ok());
    let err = &p.errors()[0];
    assert!(
        err.message.contains("name") || err.message.contains("struct"),
        "error should mention struct name: {}",
        err.message
    );
}

#[test]
fn error_module_missing_end() {
    let p = parse("module Foo do\n  fn bar() do 1 end\n");
    assert!(!p.ok());
    let err = &p.errors()[0];
    assert!(
        err.message.contains("end"),
        "error should mention `end`: {}",
        err.message
    );
}

// ── Lossless Round-Trip Tests ───────────────────────────────────────
//
// The lexer strips whitespace (by design -- whitespace tokens are not emitted).
// Newlines ARE preserved as tokens. The CST round-trip preserves all token text:
// stripping spaces from the source should match the CST text exactly.

fn strip_spaces(s: &str) -> String {
    // Remove spaces and indentation but preserve newlines and all other chars
    s.chars().filter(|c| *c != ' ').collect()
}

fn assert_lossless_roundtrip(source: &str) {
    let p = parse(source);
    let tree_text = p.syntax().text().to_string();
    let expected = strip_spaces(source);
    assert_eq!(
        tree_text, expected,
        "round-trip failed: CST text does not match source (modulo whitespace)"
    );
}

#[test]
fn lossless_simple_let() {
    assert_lossless_roundtrip("let x = 5");
}

#[test]
fn lossless_fn_def() {
    assert_lossless_roundtrip("fn add(x, y) do\n  x + y\nend");
}

#[test]
fn lossless_if_else() {
    assert_lossless_roundtrip("if true do\n  1\nelse\n  2\nend");
}

#[test]
fn lossless_case_expr() {
    assert_lossless_roundtrip("case x do\n  1 -> \"one\"\n  _ -> \"other\"\nend");
}

#[test]
fn lossless_struct_def() {
    assert_lossless_roundtrip("struct Point do\n  x :: Float\n  y :: Float\nend");
}

#[test]
fn lossless_import() {
    assert_lossless_roundtrip("import Foo.Bar");
}

#[test]
fn lossless_from_import() {
    assert_lossless_roundtrip("from Math import sqrt, pow");
}

#[test]
fn lossless_pipe_chain() {
    assert_lossless_roundtrip("fn main() do\n  x |> foo() |> bar()\nend");
}

#[test]
fn lossless_closure() {
    assert_lossless_roundtrip("fn (x) -> x + 1 end");
}

// ═══════════════════════════════════════════════════════════════════════
// Plan 03-01: Phase 3 Syntax (interface, impl, type alias, generics)
// ═══════════════════════════════════════════════════════════════════════

// ── Angle Bracket Generics ──────────────────────────────────────────

#[test]
fn struct_angle_bracket_generics() {
    assert_snapshot!(source_and_debug("struct Foo<T> do\n  x :: T\nend"));
}

// ── Interface Definition ────────────────────────────────────────────

#[test]
fn interface_simple() {
    assert_snapshot!(source_and_debug(
        "interface Printable do\n  fn to_string(self) -> String\nend"
    ));
}

#[test]
fn interface_with_generic() {
    assert_snapshot!(source_and_debug(
        "interface Container<T> do\n  fn get(self) -> T\n  fn set(self, value :: T)\nend"
    ));
}

// ── Interface Method with Default Body ──────────────────────────────

#[test]
fn interface_method_with_default_body() {
    let source = "interface Describable do\n  fn describe(self) -> String do\n    \"unknown\"\n  end\nend";
    let parse = parse(source);
    assert!(parse.errors().is_empty(), "Expected no parse errors, got: {:?}", parse.errors());
    let root = SourceFile::cast(parse.syntax()).unwrap();
    let items: Vec<_> = root.items().collect();
    assert_eq!(items.len(), 1);
    if let mesh_parser::ast::item::Item::InterfaceDef(iface) = &items[0] {
        let methods: Vec<_> = iface.methods().collect();
        assert_eq!(methods.len(), 1);
        let method = &methods[0];
        assert_eq!(method.name().and_then(|n| n.text()), Some("describe".to_string()));
        assert!(method.body().is_some(), "Expected default body to be Some");
    } else {
        panic!("Expected InterfaceDef");
    }
}

#[test]
fn interface_method_without_body() {
    let source = "interface Describable do\n  fn describe(self) -> String\nend";
    let parse = parse(source);
    assert!(parse.errors().is_empty(), "Expected no parse errors, got: {:?}", parse.errors());
    let root = SourceFile::cast(parse.syntax()).unwrap();
    let items: Vec<_> = root.items().collect();
    assert_eq!(items.len(), 1);
    if let mesh_parser::ast::item::Item::InterfaceDef(iface) = &items[0] {
        let methods: Vec<_> = iface.methods().collect();
        assert_eq!(methods.len(), 1);
        let method = &methods[0];
        assert_eq!(method.name().and_then(|n| n.text()), Some("describe".to_string()));
        assert!(method.body().is_none(), "Expected body to be None for signature-only method");
    } else {
        panic!("Expected InterfaceDef");
    }
}

// ── Impl Block ──────────────────────────────────────────────────────

#[test]
fn impl_simple() {
    assert_snapshot!(source_and_debug(
        "impl Printable for Int do\n  fn to_string(self) -> String do\n    \"int\"\n  end\nend"
    ));
}

// ── Type Alias ──────────────────────────────────────────────────────

#[test]
fn type_alias_simple() {
    assert_snapshot!(source_and_debug("type Alias = Int"));
}

#[test]
fn type_alias_generic() {
    assert_snapshot!(source_and_debug("type StringResult<T> = Result<T, String>"));
}

#[test]
fn type_alias_pub() {
    assert_snapshot!(source_and_debug("pub type Url = String"));
}

// ── Option Sugar ────────────────────────────────────────────────────

#[test]
fn option_sugar_in_type() {
    // Int? should produce a QUESTION token after IDENT "Int"
    assert_snapshot!(source_and_debug("fn foo(x :: Int?) do\n  x\nend"));
}

// ── Result Sugar ────────────────────────────────────────────────────

#[test]
fn result_sugar_in_type() {
    // T!E should produce BANG token between T and E types
    assert_snapshot!(source_and_debug("fn bar(x :: String!Error) do\n  x\nend"));
}

// ── Function with Where Clause ──────────────────────────────────────

#[test]
fn fn_with_where_clause() {
    assert_snapshot!(source_and_debug(
        "fn print<T>(x :: T) where T: Printable do\n  x\nend"
    ));
}

// ── Function with Generic Params ────────────────────────────────────

#[test]
fn fn_with_generic_params() {
    assert_snapshot!(source_and_debug("fn identity<T>(x :: T) -> T do\n  x\nend"));
}

#[test]
fn lossless_full_program() {
    let source = "\
module Math do
  pub fn add(x, y) do
    x + y
  end
end";
    assert_lossless_roundtrip(source);
}

// ═══════════════════════════════════════════════════════════════════════
// Plan 04-01: Sum Types, Constructor Patterns, Or/As Patterns
// ═══════════════════════════════════════════════════════════════════════

// ── Sum Type Definitions ────────────────────────────────────────────

#[test]
fn sum_type_simple() {
    assert_snapshot!(source_and_debug(
        "type Shape do\n  Circle(Float)\n  Rectangle(width :: Float, height :: Float)\n  Point\nend"
    ));
}

#[test]
fn sum_type_generic() {
    assert_snapshot!(source_and_debug(
        "type Option<T> do\n  Some(T)\n  None\nend"
    ));
}

#[test]
fn sum_type_multiple_positional() {
    assert_snapshot!(source_and_debug(
        "type Pair do\n  Pair(Int, Int)\nend"
    ));
}

// ── Constructor Patterns ────────────────────────────────────────────

#[test]
fn pattern_constructor_qualified() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Shape.Circle(r) -> r\n  _ -> 0\nend"
    ));
}

#[test]
fn pattern_constructor_unqualified() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Some(v) -> v\n  _ -> 0\nend"
    ));
}

#[test]
fn pattern_constructor_nested() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Some(Circle(r)) -> r\n  _ -> 0\nend"
    ));
}

#[test]
fn pattern_constructor_qualified_no_args() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Shape.Point -> 1\n  _ -> 0\nend"
    ));
}

// ── Or Patterns ─────────────────────────────────────────────────────

#[test]
fn pattern_or_simple() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Circle(_) | Point -> 1\n  _ -> 0\nend"
    ));
}

#[test]
fn pattern_or_triple() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  1 | 2 | 3 -> true\n  _ -> false\nend"
    ));
}

// ── As Patterns ─────────────────────────────────────────────────────

#[test]
fn pattern_as_simple() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Circle(r) as c -> c\n  _ -> x\nend"
    ));
}

// ── Combined Patterns ───────────────────────────────────────────────

#[test]
fn pattern_or_with_constructors() {
    assert_snapshot!(parse_and_debug(
        "case x do\n  Some(1) | Some(2) -> true\n  _ -> false\nend"
    ));
}

// ── Lossless Round-Trip for Sum Types ───────────────────────────────

#[test]
fn lossless_sum_type() {
    assert_lossless_roundtrip("type Shape do\n  Circle(Float)\n  Point\nend");
}

// ── AST Accessor Tests (Phase 04-01) ─────────────────────────────────

#[test]
fn ast_sum_type_def_accessors() {
    let p = parse("type Shape do\n  Circle(Float)\n  Rectangle(width :: Float, height :: Float)\n  Point\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let sum_type: SumTypeDef = tree
        .syntax()
        .children()
        .find_map(SumTypeDef::cast)
        .expect("should have sum type def");

    // Name
    assert_eq!(sum_type.name().unwrap().text().unwrap(), "Shape");

    // No visibility
    assert!(sum_type.visibility().is_none());

    // Variants
    let variants: Vec<_> = sum_type.variants().collect();
    assert_eq!(variants.len(), 3);

    // First variant: Circle(Float) - positional
    assert_eq!(variants[0].name().unwrap().text(), "Circle");
    let pos_types: Vec<_> = variants[0].positional_types().collect();
    assert_eq!(pos_types.len(), 1);
    assert_eq!(pos_types[0].type_name().unwrap().text(), "Float");

    // Second variant: Rectangle(width :: Float, height :: Float) - named fields
    assert_eq!(variants[1].name().unwrap().text(), "Rectangle");
    let fields: Vec<_> = variants[1].fields().collect();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name().unwrap().text().unwrap(), "width");
    assert_eq!(fields[0].type_annotation().unwrap().type_name().unwrap().text(), "Float");
    assert_eq!(fields[1].name().unwrap().text().unwrap(), "height");

    // Third variant: Point - nullary
    assert_eq!(variants[2].name().unwrap().text(), "Point");
    assert_eq!(variants[2].fields().count(), 0);
    assert_eq!(variants[2].positional_types().count(), 0);
}

#[test]
fn ast_constructor_pat_qualified() {
    let p = parse_expr("case x do\n  Shape.Circle(r) -> r\n  _ -> 0\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    // Find the CONSTRUCTOR_PAT via tree traversal
    fn find_constructor_pat(node: &mesh_parser::SyntaxNode) -> Option<ConstructorPat> {
        if let Some(pat) = ConstructorPat::cast(node.clone()) {
            return Some(pat);
        }
        for child in node.children() {
            if let Some(pat) = find_constructor_pat(&child) {
                return Some(pat);
            }
        }
        None
    }

    let ctor = find_constructor_pat(&root).expect("should have constructor pattern");

    assert!(ctor.is_qualified());
    assert_eq!(ctor.type_name().unwrap().text(), "Shape");
    assert_eq!(ctor.variant_name().unwrap().text(), "Circle");

    let fields: Vec<_> = ctor.fields().collect();
    assert_eq!(fields.len(), 1);
    match &fields[0] {
        Pattern::Ident(ident) => assert_eq!(ident.name().unwrap().text(), "r"),
        other => panic!("expected ident pattern, got {:?}", other),
    }
}

#[test]
fn ast_constructor_pat_unqualified() {
    let p = parse_expr("case x do\n  Some(v) -> v\n  _ -> 0\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    fn find_constructor_pat(node: &mesh_parser::SyntaxNode) -> Option<ConstructorPat> {
        if let Some(pat) = ConstructorPat::cast(node.clone()) {
            return Some(pat);
        }
        for child in node.children() {
            if let Some(pat) = find_constructor_pat(&child) {
                return Some(pat);
            }
        }
        None
    }

    let ctor = find_constructor_pat(&root).expect("should have constructor pattern");

    assert!(!ctor.is_qualified());
    assert!(ctor.type_name().is_none());
    assert_eq!(ctor.variant_name().unwrap().text(), "Some");

    let fields: Vec<_> = ctor.fields().collect();
    assert_eq!(fields.len(), 1);
}

#[test]
fn ast_or_pat_accessors() {
    let p = parse_expr("case x do\n  1 | 2 | 3 -> true\n  _ -> false\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    fn find_or_pat(node: &mesh_parser::SyntaxNode) -> Option<OrPat> {
        if let Some(pat) = OrPat::cast(node.clone()) {
            return Some(pat);
        }
        for child in node.children() {
            if let Some(pat) = find_or_pat(&child) {
                return Some(pat);
            }
        }
        None
    }

    let or_pat = find_or_pat(&root).expect("should have or-pattern");
    let alts: Vec<_> = or_pat.alternatives().collect();
    assert_eq!(alts.len(), 3, "should have 3 alternatives");
}

#[test]
fn ast_as_pat_accessors() {
    let p = parse_expr("case x do\n  Circle(r) as c -> c\n  _ -> x\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    fn find_as_pat(node: &mesh_parser::SyntaxNode) -> Option<AsPat> {
        if let Some(pat) = AsPat::cast(node.clone()) {
            return Some(pat);
        }
        for child in node.children() {
            if let Some(pat) = find_as_pat(&child) {
                return Some(pat);
            }
        }
        None
    }

    let as_pat = find_as_pat(&root).expect("should have as-pattern");

    // Inner pattern
    let inner = as_pat.pattern().expect("should have inner pattern");
    match inner {
        Pattern::Constructor(_) => {} // expected
        other => panic!("expected constructor pattern, got {:?}", other),
    }

    // Binding name
    assert_eq!(as_pat.binding_name().unwrap().text(), "c");
}

#[test]
fn ast_sum_type_in_items() {
    let source = "type Shape do\n  Circle(Float)\n  Point\nend\n\nfn foo() do\n  1\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let items: Vec<_> = tree.items().collect();
    assert_eq!(items.len(), 2);
    match &items[0] {
        mesh_parser::ast::item::Item::SumTypeDef(_) => {} // expected
        other => panic!("expected SumTypeDef, got {:?}", other),
    }
    match &items[1] {
        mesh_parser::ast::item::Item::FnDef(_) => {} // expected
        other => panic!("expected FnDef, got {:?}", other),
    }
}

// =======================================================================
// Plan 06-02: Actor Syntax (actor blocks, spawn/send/receive/self/link, terminate)
// =======================================================================

// -- Actor Block Definition --

#[test]
fn actor_def_simple() {
    assert_snapshot!(source_and_debug("actor Counter do\n  0\nend"));
}

#[test]
fn actor_def_with_params() {
    assert_snapshot!(source_and_debug("actor Counter(state) do\n  state + 1\nend"));
}

#[test]
fn actor_def_with_terminate_clause() {
    let source = "\
actor Worker(state) do
  state
  terminate do
    state
  end
end";
    assert_snapshot!(source_and_debug(source));
}

// -- Spawn Expression --

#[test]
fn spawn_expr_simple() {
    assert_snapshot!(parse_and_debug("spawn(counter, 0)"));
}

#[test]
fn spawn_expr_no_args() {
    assert_snapshot!(parse_and_debug("spawn(worker)"));
}

// -- Send Expression --

#[test]
fn send_expr_simple() {
    assert_snapshot!(parse_and_debug("send(pid, 42)"));
}

// -- Receive Expression --

#[test]
fn receive_expr_single_arm() {
    assert_snapshot!(parse_and_debug("receive do\n  x -> x\nend"));
}

#[test]
fn receive_expr_multiple_arms() {
    assert_snapshot!(parse_and_debug("receive do\n  1 -> 10\n  2 -> 20\nend"));
}

#[test]
fn receive_expr_with_after() {
    assert_snapshot!(parse_and_debug("receive do\n  x -> x\nafter 5000 -> 0\nend"));
}

// -- Self Expression --

#[test]
fn self_expr() {
    assert_snapshot!(parse_and_debug("self()"));
}

// -- Link Expression --

#[test]
fn link_expr_simple() {
    assert_snapshot!(parse_and_debug("link(other_pid)"));
}

// -- Actor in Context --

#[test]
fn actor_with_receive_and_send() {
    let source = "\
actor EchoServer do
  receive do
    msg -> send(msg, msg)
  end
end";
    assert_snapshot!(source_and_debug(source));
}

// -- AST Accessor Tests --

#[test]
fn ast_actor_def_accessors() {
    use mesh_parser::ast::item::ActorDef;
    let p = parse("actor Counter(state) do\n  state\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let actor: ActorDef = tree
        .syntax()
        .children()
        .find_map(ActorDef::cast)
        .expect("should have actor def");

    assert_eq!(actor.name().unwrap().text().unwrap(), "Counter");
    assert!(actor.param_list().is_some(), "should have param list");
    assert!(actor.body().is_some(), "should have body block");
    assert!(actor.terminate_clause().is_none(), "no terminate clause");
}

#[test]
fn ast_actor_def_with_terminate() {
    use mesh_parser::ast::item::ActorDef;
    let source = "actor Worker do\n  0\n  terminate do\n    1\n  end\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let actor: ActorDef = tree
        .syntax()
        .children()
        .find_map(ActorDef::cast)
        .expect("should have actor def");

    assert_eq!(actor.name().unwrap().text().unwrap(), "Worker");
    let tc = actor.terminate_clause().expect("should have terminate clause");
    assert!(tc.body().is_some(), "terminate clause should have body");
}

#[test]
fn ast_actor_in_items() {
    let source = "actor Foo do 1 end\n\nfn bar() do 2 end";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let items: Vec<_> = tree.items().collect();
    assert_eq!(items.len(), 2);
    match &items[0] {
        mesh_parser::ast::item::Item::ActorDef(_) => {}
        other => panic!("expected ActorDef, got {:?}", other),
    }
    match &items[1] {
        mesh_parser::ast::item::Item::FnDef(_) => {}
        other => panic!("expected FnDef, got {:?}", other),
    }
}

// ── Service Definition ──────────────────────────────────────────────

#[test]
fn service_def_simple() {
    assert_snapshot!(source_and_debug(
        "service Counter do\n  fn init(start_val :: Int) -> Int do\n    start_val\n  end\nend"
    ));
}

#[test]
fn service_def_with_call_handler() {
    assert_snapshot!(source_and_debug(
        "service Counter do\n  call GetCount() :: Int do |state|\n    (state, state)\n  end\nend"
    ));
}

#[test]
fn service_def_with_cast_handler() {
    assert_snapshot!(source_and_debug(
        "service Counter do\n  cast Reset() do |state|\n    0\n  end\nend"
    ));
}

#[test]
fn service_def_full() {
    let source = "\
service Counter do
  fn init(start_val :: Int) -> Int do
    start_val
  end

  call GetCount() :: Int do |state|
    (state, state)
  end

  call Increment(amount :: Int) :: Int do |state|
    (state + amount, state + amount)
  end

  cast Reset() do |state|
    0
  end
end";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn ast_service_def_accessors() {
    let source = "\
service Counter do
  fn init(start_val :: Int) -> Int do
    start_val
  end

  call GetCount() :: Int do |state|
    (state, state)
  end

  call Increment(amount :: Int) :: Int do |state|
    (state + amount, state + amount)
  end

  cast Reset() do |state|
    0
  end
end";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let items: Vec<_> = tree.items().collect();
    assert_eq!(items.len(), 1);

    match &items[0] {
        mesh_parser::ast::item::Item::ServiceDef(svc) => {
            // Name
            assert_eq!(svc.name().unwrap().text().unwrap(), "Counter");

            // Init fn
            let init = svc.init_fn().expect("should have init fn");
            assert_eq!(init.name().unwrap().text().unwrap(), "init");

            // Call handlers
            let calls = svc.call_handlers();
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0].name().unwrap().text().unwrap(), "GetCount");
            assert_eq!(calls[0].state_param_name().unwrap(), "state");
            assert!(calls[0].return_type().is_some());
            assert_eq!(calls[1].name().unwrap().text().unwrap(), "Increment");
            assert_eq!(calls[1].state_param_name().unwrap(), "state");
            assert!(calls[1].params().is_some());

            // Cast handlers
            let casts = svc.cast_handlers();
            assert_eq!(casts.len(), 1);
            assert_eq!(casts[0].name().unwrap().text().unwrap(), "Reset");
            assert_eq!(casts[0].state_param_name().unwrap(), "state");
        }
        other => panic!("expected ServiceDef, got {:?}", other),
    }
}

#[test]
fn ast_service_in_items() {
    let source = "service Foo do\n  cast Bar() do |s|\n    s\n  end\nend\n\nfn baz() do 1 end";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let items: Vec<_> = tree.items().collect();
    assert_eq!(items.len(), 2);
    match &items[0] {
        mesh_parser::ast::item::Item::ServiceDef(_) => {}
        other => panic!("expected ServiceDef, got {:?}", other),
    }
    match &items[1] {
        mesh_parser::ast::item::Item::FnDef(_) => {}
        other => panic!("expected FnDef, got {:?}", other),
    }
}

// ── Multi-Clause Function Definitions ────────────────────────────────

#[test]
fn fn_expr_body_literal_param() {
    // fn fib(0) = 0
    let source = "fn fib(0) = 0";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_ident_param() {
    // fn fib(n) = fib(n - 1) + fib(n - 2)
    let source = "fn fib(n) = fib(n - 1) + fib(n - 2)";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_with_guard() {
    // fn abs(n) when n < 0 = -n
    let source = "fn abs(n) when n < 0 = -n";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_constructor_pattern() {
    // fn foo(Some(x)) = x
    let source = "fn foo(Some(x)) = x";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_wildcard() {
    // fn foo(_) = 0
    let source = "fn foo(_) = 0";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_multiple_params() {
    // fn add(0, y) = y
    let source = "fn add(0, y) = y";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_negative_literal() {
    // fn neg(-1) = 1
    let source = "fn neg(-1) = 1";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_expr_body_bool_pattern() {
    // fn to_int(true) = 1
    let source = "fn to_int(true) = 1";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn fn_existing_do_end_still_works() {
    // Existing syntax must continue to work
    let source = "fn foo(x) do\n  x + 1\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
}

#[test]
fn fn_existing_typed_params_still_works() {
    // fn bar(x :: Int) do x end
    let source = "fn bar(x :: Int) do\n  x\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
}

#[test]
fn fn_multi_clause_consecutive() {
    // Multiple clauses for the same function
    let source = "fn fib(0) = 0\nfn fib(1) = 1\nfn fib(n) = fib(n - 1) + fib(n - 2)";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_defs: Vec<_> = tree.fn_defs().collect();
    assert_eq!(fn_defs.len(), 3);

    // All three should have the same name
    for f in &fn_defs {
        assert_eq!(f.name().unwrap().text().unwrap(), "fib");
    }
}

#[test]
fn fn_guard_clause_produces_guard_node() {
    // fn abs(n) when n < 0 = -n
    let source = "fn abs(n) when n < 0 = -n";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // Check that GUARD_CLAUSE child exists
    let has_guard = fn_def
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::GUARD_CLAUSE);
    assert!(has_guard, "expected GUARD_CLAUSE child in FnDef");
}

#[test]
fn fn_expr_body_produces_fn_expr_body_node() {
    // fn fib(0) = 0
    let source = "fn fib(0) = 0";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // Check that FN_EXPR_BODY child exists
    let has_expr_body = fn_def
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::FN_EXPR_BODY);
    assert!(has_expr_body, "expected FN_EXPR_BODY child in FnDef");
}

#[test]
fn fn_do_end_has_no_fn_expr_body() {
    // fn foo(x) do x + 1 end
    let source = "fn foo(x) do\n  x + 1\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // No FN_EXPR_BODY node for do/end functions
    let has_expr_body = fn_def
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::FN_EXPR_BODY);
    assert!(!has_expr_body, "do/end function should not have FN_EXPR_BODY");
}

#[test]
fn fn_param_literal_has_pattern_child() {
    // fn fib(0) = 0 -- the param should contain a LITERAL_PAT
    let source = "fn fib(0) = 0";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();
    let param_list = fn_def.param_list().unwrap();
    let param = param_list.params().next().unwrap();

    // The param should have a LITERAL_PAT child
    let has_literal_pat = param
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::LITERAL_PAT);
    assert!(has_literal_pat, "expected LITERAL_PAT child in param for fn fib(0)");
}

#[test]
fn fn_param_wildcard_has_pattern_child() {
    // fn foo(_) = 0
    let source = "fn foo(_) = 0";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();
    let param_list = fn_def.param_list().unwrap();
    let param = param_list.params().next().unwrap();

    let has_wildcard_pat = param
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::WILDCARD_PAT);
    assert!(has_wildcard_pat, "expected WILDCARD_PAT child in param for fn foo(_)");
}

#[test]
fn fn_param_constructor_has_pattern_child() {
    // fn foo(Some(x)) = x
    let source = "fn foo(Some(x)) = x";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();
    let param_list = fn_def.param_list().unwrap();
    let param = param_list.params().next().unwrap();

    let has_constructor_pat = param
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::CONSTRUCTOR_PAT);
    assert!(has_constructor_pat, "expected CONSTRUCTOR_PAT child in param for fn foo(Some(x))");
}

#[test]
fn fn_guard_with_function_call() {
    // Guards can include function calls (arbitrary Bool expr)
    let source = "fn process(x) when is_valid(x) = x";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
}

#[test]
fn fn_guard_with_complex_expr() {
    // Guards can be complex boolean expressions
    let source = "fn check(n) when n > 0 and n < 100 = n";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
}

#[test]
fn fn_tuple_pattern_param() {
    // fn swap((a, b)) = (b, a)
    let source = "fn swap((a, b)) = (b, a)";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();
    let param_list = fn_def.param_list().unwrap();
    let param = param_list.params().next().unwrap();

    let has_tuple_pat = param
        .syntax()
        .children()
        .any(|n| n.kind() == SyntaxKind::TUPLE_PAT);
    assert!(has_tuple_pat, "expected TUPLE_PAT child in param for fn swap((a, b))");
}

// ── AST Accessor Tests for Multi-Clause Functions ────────────────────

#[test]
fn ast_fn_def_guard_accessor() {
    let source = "fn abs(n) when n < 0 = -n";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // guard() should return Some
    let guard = fn_def.guard();
    assert!(guard.is_some(), "FnDef::guard() should return Some for guarded function");

    // guard().expr() should return the guard expression
    let guard_expr = guard.unwrap().expr();
    assert!(guard_expr.is_some(), "GuardClause::expr() should return the guard expression");
}

#[test]
fn ast_fn_def_no_guard() {
    let source = "fn fib(0) = 0";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // guard() should return None for non-guarded functions
    assert!(fn_def.guard().is_none(), "FnDef::guard() should return None");
}

#[test]
fn ast_fn_def_expr_body_accessor() {
    let source = "fn fib(0) = 0";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // expr_body() should return the body expression
    let expr_body = fn_def.expr_body();
    assert!(expr_body.is_some(), "FnDef::expr_body() should return Some for = expr form");

    // has_eq_body() should be true
    assert!(fn_def.has_eq_body(), "FnDef::has_eq_body() should be true for = expr form");

    // body() should return None (no do/end block)
    assert!(fn_def.body().is_none(), "FnDef::body() should return None for = expr form");
}

#[test]
fn ast_fn_def_do_end_accessors() {
    let source = "fn foo(x) do\n  x + 1\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();

    // body() should return Some for do/end form
    assert!(fn_def.body().is_some(), "FnDef::body() should return Some for do/end form");

    // has_eq_body() should be false
    assert!(!fn_def.has_eq_body(), "FnDef::has_eq_body() should be false for do/end form");

    // expr_body() should return None
    assert!(fn_def.expr_body().is_none(), "FnDef::expr_body() should return None for do/end form");
}

#[test]
fn ast_param_pattern_accessor() {
    // Test literal pattern param
    let source = "fn fib(0) = 0";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();
    let param = fn_def.param_list().unwrap().params().next().unwrap();

    // pattern() should return Some for literal pattern
    assert!(param.pattern().is_some(), "Param::pattern() should return Some for literal pattern");

    // name() may return None for pattern params (no direct IDENT child in some cases)
}

#[test]
fn ast_param_ident_no_pattern() {
    // Regular ident param should have no pattern
    let source = "fn foo(x) do\n  x\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    let tree = p.tree();
    let fn_def: FnDef = tree.fn_defs().next().unwrap();
    let param = fn_def.param_list().unwrap().params().next().unwrap();

    // pattern() should return None for plain ident params
    assert!(param.pattern().is_none(), "Param::pattern() should return None for ident param");

    // name() should return the IDENT
    assert!(param.name().is_some(), "Param::name() should return Some for ident param");
    assert_eq!(param.name().unwrap().text(), "x");
}

// ═══════════════════════════════════════════════════════════════════════
// Phase 22-01: Deriving clause parsing and AST accessors
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn struct_deriving_clause_snapshot() {
    let source = "struct Point do\n  x :: Int\nend deriving(Eq, Display)";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn struct_no_deriving_clause_snapshot() {
    // Existing behavior: no deriving clause
    let source = "struct Point do\n  x :: Int\nend";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn sum_type_deriving_clause_snapshot() {
    let source = "type Shape do\n  Circle\nend deriving(Eq)";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn struct_deriving_empty_clause() {
    // deriving() with empty parens derives nothing
    let source = "struct Empty do\n  x :: Int\nend deriving()";
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn ast_struct_deriving_traits_accessor() {
    let source = "struct Point do\n  x :: Int\nend deriving(Eq, Display)";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();
    let struct_def: StructDef = tree
        .syntax()
        .children()
        .find_map(StructDef::cast)
        .expect("should have struct def");

    assert!(struct_def.has_deriving_clause());
    let traits = struct_def.deriving_traits();
    assert_eq!(traits, vec!["Eq".to_string(), "Display".to_string()]);
}

#[test]
fn ast_struct_no_deriving_clause() {
    let source = "struct Point do\n  x :: Int\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();
    let struct_def: StructDef = tree
        .syntax()
        .children()
        .find_map(StructDef::cast)
        .expect("should have struct def");

    assert!(!struct_def.has_deriving_clause());
    assert!(struct_def.deriving_traits().is_empty());
}

#[test]
fn ast_sum_type_deriving_traits_accessor() {
    let source = "type Shape do\n  Circle\n  Square\nend deriving(Eq)";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();
    let sum_def: SumTypeDef = tree
        .syntax()
        .children()
        .find_map(SumTypeDef::cast)
        .expect("should have sum type def");

    assert!(sum_def.has_deriving_clause());
    let traits = sum_def.deriving_traits();
    assert_eq!(traits, vec!["Eq".to_string()]);
}

#[test]
fn ast_struct_deriving_empty() {
    let source = "struct Point do\n  x :: Int\nend deriving()";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();
    let struct_def: StructDef = tree
        .syntax()
        .children()
        .find_map(StructDef::cast)
        .expect("should have struct def");

    assert!(struct_def.has_deriving_clause());
    assert!(struct_def.deriving_traits().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════
// Phase 36-02: For-in with when filter clause
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn for_in_when_filter_snapshot() {
    assert_snapshot!(parse_and_debug("for x in list when x > 0 do\n  x\nend"));
}

#[test]
fn for_in_when_filter_ast_accessors() {
    let p = parse_expr("for x in list when x > 0 do\n  x\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    let for_in: ForInExpr = root
        .children()
        .find_map(ForInExpr::cast)
        .expect("should have ForInExpr");

    // WHEN_KW token is present as a direct child
    let has_when = for_in
        .syntax()
        .children_with_tokens()
        .any(|it| it.kind() == SyntaxKind::WHEN_KW);
    assert!(has_when, "ForInExpr should have WHEN_KW token");

    // filter() returns Some(expr)
    let filter = for_in.filter();
    assert!(filter.is_some(), "ForInExpr::filter() should return Some");

    // iterable() still returns the correct expression
    let iterable = for_in.iterable();
    assert!(iterable.is_some(), "ForInExpr::iterable() should return Some");

    // body() returns the block
    let body = for_in.body();
    assert!(body.is_some(), "ForInExpr::body() should return Some");
}

#[test]
fn for_in_without_when_filter_returns_none() {
    let p = parse_expr("for x in list do\n  x\nend");
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    let for_in: ForInExpr = root
        .children()
        .find_map(ForInExpr::cast)
        .expect("should have ForInExpr");

    // No WHEN_KW token
    let has_when = for_in
        .syntax()
        .children_with_tokens()
        .any(|it| it.kind() == SyntaxKind::WHEN_KW);
    assert!(!has_when, "ForInExpr without when should not have WHEN_KW token");

    // filter() returns None
    assert!(for_in.filter().is_none(), "ForInExpr::filter() should return None without when clause");

    // iterable() and body() still work
    assert!(for_in.iterable().is_some(), "iterable() should still work");
    assert!(for_in.body().is_some(), "body() should still work");
}

// ═══════════════════════════════════════════════════════════════════════
// Phase 74-01: Associated Type Parser and AST Support
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn interface_with_assoc_type_decl() {
    let source = "interface Iterator do\n  type Item\n  fn next(self) -> Int\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn impl_with_assoc_type_binding() {
    let source = "impl Iterator for MyList do\n  type Item = Int\n  fn next(self) -> Int do\n    42\n  end\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    assert_snapshot!(source_and_debug(source));
}

#[test]
fn interface_assoc_type_cst_has_assoc_type_def() {
    let source = "interface Foo do\n  type Item\n  fn next(self) -> Int\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    // Find ASSOC_TYPE_DEF in the CST
    fn find_kind(node: &mesh_parser::SyntaxNode, target: SyntaxKind) -> bool {
        if node.kind() == target {
            return true;
        }
        node.children().any(|c| find_kind(&c, target))
    }

    assert!(
        find_kind(&root, SyntaxKind::ASSOC_TYPE_DEF),
        "CST should contain ASSOC_TYPE_DEF node"
    );
}

#[test]
fn impl_assoc_type_cst_has_assoc_type_binding() {
    let source = "impl Iterator for MyList do\n  type Item = Int\n  fn next(self) -> Int do\n    42\n  end\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let root = p.syntax();

    fn find_kind(node: &mesh_parser::SyntaxNode, target: SyntaxKind) -> bool {
        if node.kind() == target {
            return true;
        }
        node.children().any(|c| find_kind(&c, target))
    }

    assert!(
        find_kind(&root, SyntaxKind::ASSOC_TYPE_BINDING),
        "CST should contain ASSOC_TYPE_BINDING node"
    );
}

#[test]
fn ast_interface_def_assoc_types_accessor() {
    use mesh_parser::ast::item::{AssocTypeDef, InterfaceDef};
    let source = "interface Iterator do\n  type Item\n  fn next(self) -> Int\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let iface: InterfaceDef = tree
        .syntax()
        .children()
        .find_map(InterfaceDef::cast)
        .expect("should have interface def");

    let assoc_types: Vec<AssocTypeDef> = iface.assoc_types().collect();
    assert_eq!(assoc_types.len(), 1);
    assert_eq!(assoc_types[0].name().unwrap().text().unwrap(), "Item");

    // Methods should still work
    let methods: Vec<_> = iface.methods().collect();
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].name().unwrap().text().unwrap(), "next");
}

#[test]
fn ast_impl_def_assoc_type_bindings_accessor() {
    use mesh_parser::ast::item::{AssocTypeBinding, ImplDef};
    let source = "impl Iterator for MyList do\n  type Item = Int\n  fn next(self) -> Int do\n    42\n  end\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
    let tree = p.tree();

    let impl_def: ImplDef = tree
        .syntax()
        .children()
        .find_map(ImplDef::cast)
        .expect("should have impl def");

    let bindings: Vec<AssocTypeBinding> = impl_def.assoc_type_bindings().collect();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].name().unwrap().text().unwrap(), "Item");

    // Methods should still work
    let methods: Vec<_> = impl_def.methods().collect();
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].name().unwrap().text().unwrap(), "next");
}

#[test]
fn interface_multiple_assoc_types() {
    let source = "interface Numeric do\n  type Output\n  type Input\n  fn compute(self) -> Int\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());

    use mesh_parser::ast::item::InterfaceDef;
    let tree = p.tree();
    let iface: InterfaceDef = tree
        .syntax()
        .children()
        .find_map(InterfaceDef::cast)
        .expect("should have interface def");

    let assoc_types: Vec<_> = iface.assoc_types().collect();
    assert_eq!(assoc_types.len(), 2);
    assert_eq!(assoc_types[0].name().unwrap().text().unwrap(), "Output");
    assert_eq!(assoc_types[1].name().unwrap().text().unwrap(), "Input");
}

#[test]
fn impl_assoc_type_generic_binding() {
    // Test binding to a generic type: type Item = List<Int>
    let source = "impl Foo for Bar do\n  type Item = List<Int>\n  fn baz(self) do\n    42\n  end\nend";
    let p = parse(source);
    assert!(p.ok(), "parse errors: {:?}", p.errors());
}

// ── Multi-line pipe (PIPE-01) ──────────────────────────────────────────

#[test]
fn pipe_multiline_leading() {
    // |> at start of continuation line (already worked, regression test)
    assert_snapshot!(parse_and_debug("x\n  |> foo()"));
}

#[test]
fn pipe_multiline_trailing() {
    // |> at end of line (new in Phase 126)
    assert_snapshot!(parse_and_debug("x |>\n  foo()"));
}

#[test]
fn pipe_multiline_chain_leading() {
    assert_snapshot!(parse_and_debug("x\n  |> foo()\n  |> bar()"));
}

#[test]
fn pipe_multiline_chain_trailing() {
    assert_snapshot!(parse_and_debug("x |>\n  foo() |>\n  bar()"));
}

#[test]
fn slot_pipe_multiline_leading() {
    // |2> at start of continuation line
    assert_snapshot!(parse_and_debug("x\n  |2> foo(1)"));
}

#[test]
fn slot_pipe_multiline_trailing() {
    // |2> at end of line (new in Phase 126)
    assert_snapshot!(parse_and_debug("x |2>\n  foo(1)"));
}
