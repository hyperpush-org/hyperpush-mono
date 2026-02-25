//! Pratt expression parser for Mesh.
//!
//! Implements operator precedence parsing using binding power tables.
//! Handles all Mesh expression forms: literals, identifiers, binary/unary
//! operators, function calls, field access, indexing, pipe, grouping,
//! string interpolation, compound expressions (if/else, case/match,
//! closures, blocks), and basic statements (let bindings, return).

use mesh_common::span::Span;

use crate::syntax_kind::SyntaxKind;

use super::{MarkClosed, Parser};

// ── Binding Power Tables ───────────────────────────────────────────────

/// Returns (left_bp, right_bp) for infix operators.
///
/// Left < right means left-associative (the usual case).
/// Left > right would mean right-associative.
/// Returns `None` if the token is not an infix operator.
fn infix_binding_power(op: SyntaxKind) -> Option<(u8, u8)> {
    match op {
        // Pipe: lowest expression precedence, left-associative
        SyntaxKind::PIPE => Some((3, 4)),
        // Slot pipe: same precedence as pipe, left-associative
        SyntaxKind::SLOT_PIPE => Some((3, 4)),

        // Logical OR: left-associative
        SyntaxKind::OR_KW | SyntaxKind::PIPE_PIPE => Some((5, 6)),

        // Logical AND: left-associative
        SyntaxKind::AND_KW | SyntaxKind::AMP_AMP => Some((7, 8)),

        // Equality: left-associative
        SyntaxKind::EQ_EQ | SyntaxKind::NOT_EQ => Some((9, 10)),

        // Comparison: left-associative
        SyntaxKind::LT | SyntaxKind::GT | SyntaxKind::LT_EQ | SyntaxKind::GT_EQ => Some((11, 12)),

        // Range: left-associative
        SyntaxKind::DOT_DOT => Some((13, 14)),

        // Concatenation: left-associative
        SyntaxKind::DIAMOND | SyntaxKind::PLUS_PLUS => Some((15, 16)),

        // Additive: left-associative
        SyntaxKind::PLUS | SyntaxKind::MINUS => Some((17, 18)),

        // Multiplicative: left-associative
        SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT => Some((19, 20)),

        _ => None,
    }
}

/// Returns ((), right_bp) for prefix operators.
///
/// Returns `None` if the token is not a prefix operator.
fn prefix_binding_power(op: SyntaxKind) -> Option<((), u8)> {
    match op {
        SyntaxKind::MINUS => Some(((), 23)),
        SyntaxKind::BANG => Some(((), 23)),
        SyntaxKind::NOT_KW => Some(((), 23)),
        _ => None,
    }
}

/// Postfix operations (call, field access, indexing) have implicit binding
/// power of 25, tighter than all prefix and infix operators.
const POSTFIX_BP: u8 = 25;

// ── Expression Entry Point ─────────────────────────────────────────────

/// Parse an expression at the default (lowest) binding power.
pub(crate) fn expr(p: &mut Parser) {
    expr_bp(p, 0);
}

/// Parse an expression with the given minimum binding power.
///
/// This is the core Pratt parsing loop. It first parses an atom or prefix
/// expression (the LHS), then loops over postfix and infix operators,
/// consuming them as long as their binding power exceeds `min_bp`.
fn expr_bp(p: &mut Parser, min_bp: u8) -> Option<MarkClosed> {
    // Parse the left-hand side (atom or prefix).
    let mut lhs = lhs(p)?;

    // Postfix and infix loop.
    loop {
        if p.has_error() {
            break;
        }

        let current = p.current();

        // ── Postfix: struct literal ──
        // After an identifier (NAME_REF), `{` starts a struct literal: `Point { x: 1, y: 2 }`
        if current == SyntaxKind::L_BRACE && POSTFIX_BP >= min_bp {
            let m = p.open_before(lhs);
            parse_struct_literal_body(p);
            lhs = p.close(m, SyntaxKind::STRUCT_LITERAL);
            continue;
        }

        // ── Postfix: function call (possibly with trailing closure) ──
        if current == SyntaxKind::L_PAREN && POSTFIX_BP >= min_bp {
            let m = p.open_before(lhs);
            parse_arg_list(p);
            // Check for trailing closure: `foo() do ... end`
            if p.at(SyntaxKind::DO_KW) {
                parse_trailing_closure(p);
            }
            lhs = p.close(m, SyntaxKind::CALL_EXPR);
            continue;
        }

        // ── Postfix: field access ──
        if current == SyntaxKind::DOT && POSTFIX_BP >= min_bp {
            let m = p.open_before(lhs);
            p.advance(); // .
            // Accept IDENT or keywords that are valid as field names
            // (e.g., Node.self, Node.monitor, Node.spawn, Process.monitor, Ws.send,
            //  Changeset.cast).
            if !p.eat(SyntaxKind::IDENT)
                && !p.eat(SyntaxKind::SELF_KW)
                && !p.eat(SyntaxKind::MONITOR_KW)
                && !p.eat(SyntaxKind::SPAWN_KW)
                && !p.eat(SyntaxKind::LINK_KW)
                && !p.eat(SyntaxKind::SEND_KW)
                && !p.eat(SyntaxKind::WHERE_KW)
                && !p.eat(SyntaxKind::CAST_KW) {
                p.error("expected IDENT");
            }
            lhs = p.close(m, SyntaxKind::FIELD_ACCESS);
            continue;
        }

        // ── Postfix: index access ──
        if current == SyntaxKind::L_BRACKET && POSTFIX_BP >= min_bp {
            let m = p.open_before(lhs);
            p.advance(); // [
            expr_bp(p, 0);
            p.expect(SyntaxKind::R_BRACKET);
            lhs = p.close(m, SyntaxKind::INDEX_EXPR);
            continue;
        }

        // ── Postfix: try operator ──
        if current == SyntaxKind::QUESTION && POSTFIX_BP >= min_bp {
            let m = p.open_before(lhs);
            p.advance(); // ?
            lhs = p.close(m, SyntaxKind::TRY_EXPR);
            continue;
        }

        // ── Infix operators ──
        if let Some((l_bp, r_bp)) = infix_binding_power(current) {
            if l_bp < min_bp {
                break;
            }

            let m = p.open_before(lhs);
            p.advance(); // operator

            expr_bp(p, r_bp);

            let kind = match current {
                SyntaxKind::PIPE => SyntaxKind::PIPE_EXPR,
                SyntaxKind::SLOT_PIPE => SyntaxKind::SLOT_PIPE_EXPR,
                _ => SyntaxKind::BINARY_EXPR,
            };
            lhs = p.close(m, kind);
            continue;
        }

        // ── Multi-line pipe continuation ──
        // At the top level (outside delimiters), newlines are significant.
        // When the current token is NEWLINE and the next non-newline token
        // is PIPE (|>), treat the newline as a continuation rather than a
        // statement terminator. This allows multi-line pipe chains like:
        //   users
        //     |> filter(fn u -> u.active end)
        //     |> map(fn u -> u.name end)
        if current == SyntaxKind::NEWLINE
            && matches!(
                p.peek_past_newlines(),
                SyntaxKind::PIPE | SyntaxKind::SLOT_PIPE
            )
        {
            // PIPE / SLOT_PIPE have binding power (3, 4). Check if we can continue.
            if 3 >= min_bp {
                // Skip the newlines so the Pratt loop sees the PIPE/SLOT_PIPE operator.
                p.skip_newlines_for_continuation();
                continue;
            }
        }

        // Nothing matched -- exit the loop.
        break;
    }

    Some(lhs)
}

// ── Atom / Prefix Parsing (LHS) ───────────────────────────────────────

/// Parse the left-hand side of an expression: an atom or a prefix operator.
fn lhs(p: &mut Parser) -> Option<MarkClosed> {
    let current = p.current();

    // ── Prefix operators ──
    if let Some(((), r_bp)) = prefix_binding_power(current) {
        let m = p.open();
        p.advance(); // operator
        expr_bp(p, r_bp);
        return Some(p.close(m, SyntaxKind::UNARY_EXPR));
    }

    // ── Atoms ──
    match current {
        // Numeric literals
        SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL => {
            let m = p.open();
            p.advance();
            Some(p.close(m, SyntaxKind::LITERAL))
        }

        // Boolean and nil literals
        SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW | SyntaxKind::NIL_KW => {
            let m = p.open();
            p.advance();
            Some(p.close(m, SyntaxKind::LITERAL))
        }

        // Atom literal
        SyntaxKind::ATOM_LITERAL => {
            let m = p.open();
            p.advance();
            Some(p.close(m, SyntaxKind::ATOM_EXPR))
        }

        // Identifier
        SyntaxKind::IDENT => {
            let m = p.open();
            p.advance();
            Some(p.close(m, SyntaxKind::NAME_REF))
        }

        // String (may contain interpolation)
        SyntaxKind::STRING_START => Some(parse_string_expr(p)),

        // Grouped expression or tuple
        SyntaxKind::L_PAREN => {
            let m = p.open();
            p.advance(); // (

            // Empty parens: unit/empty tuple
            if p.at(SyntaxKind::R_PAREN) {
                p.advance(); // )
                return Some(p.close(m, SyntaxKind::TUPLE_EXPR));
            }

            // Parse the first expression.
            expr_bp(p, 0);

            // If followed by a comma, it's a tuple.
            if p.at(SyntaxKind::COMMA) {
                // Parse remaining tuple elements.
                while p.eat(SyntaxKind::COMMA) {
                    if p.at(SyntaxKind::R_PAREN) {
                        break; // trailing comma
                    }
                    expr_bp(p, 0);
                }
                p.expect(SyntaxKind::R_PAREN);
                Some(p.close(m, SyntaxKind::TUPLE_EXPR))
            } else {
                // Just a grouped expression -- no wrapper node needed,
                // but we keep the parens in the CST via the open/close.
                p.expect(SyntaxKind::R_PAREN);
                Some(p.close(m, SyntaxKind::TUPLE_EXPR))
            }
        }

        // List literal: [expr, expr, ...]
        SyntaxKind::L_BRACKET => {
            let m = p.open();
            p.advance(); // consume [
            if p.current() != SyntaxKind::R_BRACKET {
                expr_bp(p, 0);
                while p.current() == SyntaxKind::COMMA {
                    p.advance(); // consume ,
                    if p.current() == SyntaxKind::R_BRACKET {
                        break; // trailing comma
                    }
                    expr_bp(p, 0);
                }
            }
            p.expect(SyntaxKind::R_BRACKET);
            Some(p.close(m, SyntaxKind::LIST_LITERAL))
        }

        // Compound expression atoms
        SyntaxKind::IF_KW => Some(parse_if_expr(p)),
        SyntaxKind::CASE_KW | SyntaxKind::MATCH_KW => Some(parse_case_expr(p)),
        SyntaxKind::FN_KW => Some(parse_closure(p)),

        // Map literal: %{key => value, ...}
        SyntaxKind::PERCENT => {
            if p.nth(1) == SyntaxKind::L_BRACE {
                Some(parse_map_literal(p))
            } else {
                // Bare % is not valid in lhs position (modulo is an infix op)
                p.error("expected expression");
                None
            }
        }

        // Loop expression atoms
        SyntaxKind::WHILE_KW => Some(parse_while_expr(p)),
        SyntaxKind::FOR_KW => Some(parse_for_in_expr(p)),
        SyntaxKind::BREAK_KW => Some(parse_break_expr(p)),
        SyntaxKind::CONTINUE_KW => Some(parse_continue_expr(p)),

        // Actor expression atoms
        SyntaxKind::SPAWN_KW => Some(parse_spawn_expr(p)),
        SyntaxKind::SEND_KW => Some(parse_send_expr(p)),
        SyntaxKind::RECEIVE_KW => Some(parse_receive_expr(p)),
        SyntaxKind::SELF_KW => {
            // `self()` is the actor self-call; `self.x` or bare `self` is a
            // method-receiver reference (impl method bodies).  Disambiguate
            // by looking at the next token.
            if p.nth(1) == SyntaxKind::L_PAREN {
                Some(parse_self_expr(p))
            } else {
                // Treat as NAME_REF so postfix `.field` works.
                let m = p.open();
                p.advance(); // SELF_KW
                Some(p.close(m, SyntaxKind::NAME_REF))
            }
        }
        SyntaxKind::LINK_KW => Some(parse_link_expr(p)),

        _ => {
            p.error("expected expression");
            None
        }
    }
}

// ── Map Literal ───────────────────────────────────────────────────

/// Parse a map literal or struct update expression.
///
/// Map literal: `%{key1 => value1, key2 => value2, ...}`
/// Struct update: `%{base_expr | field: value, ...}`
///
/// Disambiguation: After `%{`, parse the first expression and save its mark.
/// If the next token is `BAR` (`|`), this is a struct update expression.
/// If `FAT_ARROW` (`=>`), use `open_before` to retroactively wrap the key
/// in a MAP_ENTRY node and continue as a map literal. Empty `%{}` is a map.
fn parse_map_literal(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // PERCENT
    p.expect(SyntaxKind::L_BRACE);

    // Handle empty map literal: %{}
    if p.at(SyntaxKind::R_BRACE) {
        p.advance(); // R_BRACE
        return p.close(m, SyntaxKind::MAP_LITERAL);
    }

    // Parse the first expression (could be map key or struct update base).
    // Save the MarkClosed so we can retroactively wrap it in MAP_ENTRY if needed.
    let first_expr_mark = match expr_bp(p, 0) {
        Some(mark) => mark,
        None => {
            p.expect(SyntaxKind::R_BRACE);
            return p.close(m, SyntaxKind::MAP_LITERAL);
        }
    };

    if p.has_error() {
        p.expect(SyntaxKind::R_BRACE);
        return p.close(m, SyntaxKind::MAP_LITERAL);
    }

    // Disambiguate: BAR means struct update, FAT_ARROW means map literal.
    if p.at(SyntaxKind::BAR) {
        // ── Struct Update: %{base | field: value, ...} ──
        p.advance(); // BAR

        // Parse comma-separated `name: expr` override fields (reuse STRUCT_LITERAL_FIELD).
        loop {
            if p.at(SyntaxKind::R_BRACE) || p.at(SyntaxKind::EOF) {
                break;
            }

            let field = p.open();

            // Field name.
            if p.at(SyntaxKind::IDENT) {
                let name = p.open();
                p.advance(); // field name
                p.close(name, SyntaxKind::NAME);
            } else {
                p.error("expected field name in struct update");
                p.close(field, SyntaxKind::STRUCT_LITERAL_FIELD);
                break;
            }

            // Colon.
            p.expect(SyntaxKind::COLON);

            // Field value expression.
            if !p.has_error() {
                expr_bp(p, 0);
            }

            p.close(field, SyntaxKind::STRUCT_LITERAL_FIELD);

            if p.has_error() {
                break;
            }

            // Separator: comma or implicit (newlines inside braces are insignificant).
            if !p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::R_BRACE) || p.at(SyntaxKind::EOF) {
                    break;
                }
            }
        }

        p.expect(SyntaxKind::R_BRACE);
        return p.close(m, SyntaxKind::STRUCT_UPDATE_EXPR);
    }

    // ── Map Literal: retroactively wrap first key expression in MAP_ENTRY ──
    {
        let entry = p.open_before(first_expr_mark);
        p.expect(SyntaxKind::FAT_ARROW); // =>
        if !p.has_error() {
            expr_bp(p, 0); // value expression
        }
        p.close(entry, SyntaxKind::MAP_ENTRY);
    }

    if p.has_error() {
        p.expect(SyntaxKind::R_BRACE);
        return p.close(m, SyntaxKind::MAP_LITERAL);
    }

    // Parse remaining map entries.
    while p.eat(SyntaxKind::COMMA) {
        if p.at(SyntaxKind::R_BRACE) || p.at(SyntaxKind::EOF) {
            break; // trailing comma
        }

        let entry = p.open();
        expr_bp(p, 0); // key expression
        p.expect(SyntaxKind::FAT_ARROW); // =>
        if !p.has_error() {
            expr_bp(p, 0); // value expression
        }
        p.close(entry, SyntaxKind::MAP_ENTRY);

        if p.has_error() {
            break;
        }
    }

    p.expect(SyntaxKind::R_BRACE);
    p.close(m, SyntaxKind::MAP_LITERAL)
}

// ── Struct Literal ────────────────────────────────────────────────

/// Parse a struct literal body: `{ field: expr, field: expr }`
///
/// Each field is `name: expr`, separated by commas or newlines.
/// Produces STRUCT_LITERAL_FIELD children inside the struct literal.
fn parse_struct_literal_body(p: &mut Parser) {
    p.advance(); // L_BRACE

    loop {
        // Skip insignificant newlines inside braces (tracked by delimiter depth).
        if p.at(SyntaxKind::R_BRACE) || p.at(SyntaxKind::EOF) {
            break;
        }

        let field = p.open();

        // Field name.
        if p.at(SyntaxKind::IDENT) {
            let name = p.open();
            p.advance(); // field name
            p.close(name, SyntaxKind::NAME);
        } else {
            p.error("expected field name in struct literal");
            p.close(field, SyntaxKind::STRUCT_LITERAL_FIELD);
            break;
        }

        // Colon.
        p.expect(SyntaxKind::COLON);

        // Field value expression.
        if !p.has_error() {
            expr_bp(p, 0);
        }

        p.close(field, SyntaxKind::STRUCT_LITERAL_FIELD);

        if p.has_error() {
            break;
        }

        // Separator: comma or implicit (newlines inside braces are insignificant).
        if !p.eat(SyntaxKind::COMMA) {
            // No comma -- if next is not `}`, that's fine (newlines handle separation).
            if p.at(SyntaxKind::R_BRACE) || p.at(SyntaxKind::EOF) {
                break;
            }
        }
    }

    p.expect(SyntaxKind::R_BRACE);
}

// ── Argument List ──────────────────────────────────────────────────────

/// Check if the current position starts a keyword argument: `IDENT COLON`
/// where COLON is not part of `COLON_COLON` (type annotation).
fn at_keyword_arg(p: &Parser) -> bool {
    p.current() == SyntaxKind::IDENT && p.nth(1) == SyntaxKind::COLON
}

/// Parse an argument list: `(expr, expr, ...)`.
///
/// Supports keyword arguments at the end of the list:
/// - `f(name: "Alice", age: 30)` desugars to `f(%{"name" => "Alice", "age" => 30})`
/// - `f(x, name: "Alice")` desugars to `f(x, %{"name" => "Alice"})`
///
/// Keyword arguments are detected by the `IDENT COLON` pattern (not `IDENT COLON_COLON`).
/// Once a keyword argument is seen, all remaining arguments must be keyword arguments.
/// The keyword arguments are wrapped in a synthetic MAP_LITERAL node appended as
/// the final argument.
fn parse_arg_list(p: &mut Parser) {
    let m = p.open();
    p.advance(); // (

    // Parse comma-separated arguments, detecting keyword args.
    if !p.at(SyntaxKind::R_PAREN) {
        // Check if the very first argument is a keyword arg.
        if at_keyword_arg(p) {
            // Entire arg list is keyword args (zero positional, one map arg).
            parse_keyword_args_as_map(p);
        } else {
            // Parse first positional argument.
            expr_bp(p, 0);

            // Parse remaining arguments.
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::R_PAREN) {
                    break; // trailing comma
                }
                // Check if this argument starts keyword args.
                if at_keyword_arg(p) {
                    parse_keyword_args_as_map(p);
                    break; // keyword args are always last
                }
                expr_bp(p, 0);
            }
        }
    }

    p.expect(SyntaxKind::R_PAREN);
    p.close(m, SyntaxKind::ARG_LIST);
}

/// Parse keyword arguments and wrap them in a MAP_LITERAL node.
///
/// Called when the parser has detected `IDENT COLON` at the current position.
/// Parses all remaining `name: expr` pairs, wrapping each in a MAP_ENTRY
/// and the whole set in a MAP_LITERAL. The key identifier is wrapped in a
/// NAME_REF node; the MIR lowerer converts these to string literals.
fn parse_keyword_args_as_map(p: &mut Parser) {
    let map_m = p.open();

    // Parse first keyword entry.
    parse_keyword_entry(p);

    // Parse remaining keyword entries.
    while p.eat(SyntaxKind::COMMA) {
        if p.at(SyntaxKind::R_PAREN) {
            break; // trailing comma
        }
        parse_keyword_entry(p);
    }

    p.close(map_m, SyntaxKind::MAP_LITERAL);
}

/// Parse a single keyword argument entry: `name: expr`.
///
/// Produces a MAP_ENTRY node with the identifier wrapped in NAME_REF as key
/// and the parsed expression as value.
fn parse_keyword_entry(p: &mut Parser) {
    let entry = p.open();

    // Key: wrap the identifier in a NAME_REF node.
    let key = p.open();
    p.advance(); // IDENT
    p.close(key, SyntaxKind::NAME_REF);

    // Consume the colon separator.
    p.advance(); // COLON

    // Value expression.
    if !p.has_error() {
        expr_bp(p, 0);
    }

    p.close(entry, SyntaxKind::MAP_ENTRY);
}

// ── String Expression ──────────────────────────────────────────────────

/// Parse a string expression, which may contain interpolation segments.
///
/// String tokens from the lexer look like:
///   STRING_START  STRING_CONTENT?  (INTERPOLATION_START expr INTERPOLATION_END STRING_CONTENT?)*  STRING_END
fn parse_string_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // STRING_START

    loop {
        match p.current() {
            SyntaxKind::STRING_CONTENT => {
                p.advance();
            }
            SyntaxKind::INTERPOLATION_START => {
                let interp = p.open();
                p.advance(); // ${
                expr_bp(p, 0);
                p.expect(SyntaxKind::INTERPOLATION_END);
                p.close(interp, SyntaxKind::INTERPOLATION);
            }
            SyntaxKind::STRING_END => {
                p.advance();
                break;
            }
            SyntaxKind::EOF => {
                p.error("unterminated string");
                break;
            }
            _ => {
                // Unexpected token inside string -- error recovery.
                p.error("unexpected token in string");
                break;
            }
        }
    }

    p.close(m, SyntaxKind::STRING_EXPR)
}

// ── Block Parsing ─────────────────────────────────────────────────────

/// Parse a block body: a sequence of statements separated by newlines
/// or semicolons.
///
/// A block is parsed until END_KW, ELSE_KW, or EOF is encountered.
/// Each statement is either a let binding, return expression, or an
/// expression-statement.
pub(crate) fn parse_block_body(p: &mut Parser) {
    let m = p.open();

    loop {
        // Eat leading newlines/semicolons between statements.
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        // Check if we've reached a block terminator.
        match p.current() {
            SyntaxKind::END_KW | SyntaxKind::ELSE_KW | SyntaxKind::EOF => break,
            _ => {}
        }

        // Parse a statement or item.
        super::parse_item_or_stmt(p);

        if p.has_error() {
            break;
        }

        // After a statement, expect a separator or block terminator.
        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {
                // Will be eaten at top of loop.
            }
            SyntaxKind::END_KW | SyntaxKind::ELSE_KW | SyntaxKind::EOF => {
                // Block terminator -- stop.
            }
            _ => {
                // If we're not at a separator or terminator, that's ok --
                // the next iteration will try to parse another statement
                // or hit an error.
            }
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

// ── Let Binding ───────────────────────────────────────────────────────

/// Parse a let binding: `let pattern [:: Type] = expr`
pub(crate) fn parse_let_binding(p: &mut Parser) {
    let m = p.open();
    p.advance(); // LET_KW

    // Parse the pattern: identifier, tuple, wildcard, etc.
    if p.at(SyntaxKind::L_PAREN) {
        // Tuple pattern: let (a, b) = expr
        super::patterns::parse_pattern(p);
    } else if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance(); // identifier
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected identifier or pattern after `let`");
    }

    // Optional type annotation: `:: Type`
    if p.at(SyntaxKind::COLON_COLON) {
        let ann = p.open();
        p.advance(); // ::
        super::items::parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }

    // Expect `=` and initializer.
    p.expect(SyntaxKind::EQ);
    if !p.has_error() {
        expr(p);
    }

    p.close(m, SyntaxKind::LET_BINDING);
}

// ── Return Expression ─────────────────────────────────────────────────

/// Parse a return expression: `return [expr]`
pub(crate) fn parse_return_expr(p: &mut Parser) {
    let m = p.open();
    p.advance(); // RETURN_KW

    // If next token looks like an expression start, parse the value.
    if looks_like_expr_start(p) {
        expr(p);
    }

    p.close(m, SyntaxKind::RETURN_EXPR);
}

/// Whether the current token could start an expression.
/// Used to determine if `return` has a value.
fn looks_like_expr_start(p: &Parser) -> bool {
    match p.current() {
        SyntaxKind::NEWLINE
        | SyntaxKind::END_KW
        | SyntaxKind::ELSE_KW
        | SyntaxKind::EOF
        | SyntaxKind::SEMICOLON => false,
        _ => true,
    }
}

// ── If/Else Expression ────────────────────────────────────────────────

/// Parse an if expression: `if cond do body [else [if ...] body] end`
fn parse_if_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // IF_KW

    // Parse condition.
    expr(p);

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse then-body.
    parse_block_body(p);

    // Check for else.
    if p.at(SyntaxKind::ELSE_KW) {
        let else_m = p.open();
        p.advance(); // ELSE_KW

        if p.at(SyntaxKind::IF_KW) {
            // else-if chain: parse nested if expression.
            parse_if_expr(p);
        } else {
            // else block.
            parse_block_body(p);
            p.expect(SyntaxKind::END_KW);
        }

        p.close(else_m, SyntaxKind::ELSE_BRANCH);
    } else {
        // No else -- expect `end`.
        if !p.at(SyntaxKind::END_KW) {
            p.error_with_related(
                "expected `end` to close `do` block",
                do_span,
                "`do` block started here",
            );
        } else {
            p.advance(); // END_KW
        }
    }

    p.close(m, SyntaxKind::IF_EXPR)
}

// ── Case/Match Expression ─────────────────────────────────────────────

/// Parse a case/match expression: `case expr do pattern -> body ... end`
fn parse_case_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // CASE_KW or MATCH_KW

    // Parse scrutinee.
    expr(p);

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse match arms until END_KW.
    loop {
        p.eat_newlines();

        if p.at(SyntaxKind::END_KW) || p.at(SyntaxKind::EOF) {
            break;
        }

        parse_match_arm(p);

        if p.has_error() {
            break;
        }
    }

    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close `case` block",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::CASE_EXPR)
}

/// Parse a single match arm: `pattern [when guard] -> body`
fn parse_match_arm(p: &mut Parser) {
    let m = p.open();

    // Parse pattern.
    super::patterns::parse_pattern(p);

    // Optional `when` guard.
    if p.at(SyntaxKind::WHEN_KW) {
        p.advance(); // WHEN_KW
        expr(p);
    }

    // Expect `->`.
    p.expect(SyntaxKind::ARROW);

    // Parse arm body: a single expression.
    if !p.has_error() {
        expr(p);
    }

    p.close(m, SyntaxKind::MATCH_ARM);
}

// ── Closure Expression ────────────────────────────────────────────────

/// Parse a closure expression.
///
/// Supports multiple syntax forms:
/// - Parenthesized params: `fn(x, y) -> body end` or `fn (x) -> body end`
/// - Bare params: `fn x -> body end` or `fn x, y -> body end`
/// - No params: `fn -> body end` or `fn do body end`
/// - do/end body: `fn x do body end`
/// - Multi-clause: `fn 0 -> "zero" | n -> to_string(n) end`
/// - Guard clause: `fn x when x > 0 -> x | x -> -x end`
/// - Pattern params: `fn Some(x) -> x | None -> 0 end`
///
/// Multi-clause closures: the first clause's children (params, guard, arrow,
/// body) are direct children of CLOSURE_EXPR. Subsequent clauses are wrapped
/// in CLOSURE_CLAUSE nodes. The AST layer detects multi-clause by the
/// presence of CLOSURE_CLAUSE children.
///
/// For arrow bodies (`->`), the body expression is wrapped in a BLOCK node
/// (using `expr()` internally to stop at BAR for multi-clause detection).
/// For do/end bodies, `parse_block_body()` produces the BLOCK as usual.
fn parse_closure(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    let fn_span = p.current_span();
    p.advance(); // FN_KW

    // Dispatch on what follows fn:
    // - L_PAREN: parenthesized params
    // - ARROW: no params, arrow body
    // - DO_KW: no params, do/end body
    // - IDENT/LITERAL/MINUS/_: bare params or pattern params

    let has_paren_params = p.at(SyntaxKind::L_PAREN);
    let has_arrow_immediately = p.at(SyntaxKind::ARROW);
    let has_do_immediately = p.at(SyntaxKind::DO_KW);
    let has_bare_params = looks_like_bare_closure_params(p);

    // No-params closures: `fn -> body end` or `fn do body end`
    if has_arrow_immediately || has_do_immediately {
        parse_closure_body_block(p, fn_span);
        expect_closure_end(p, fn_span);
        return p.close(m, SyntaxKind::CLOSURE_EXPR);
    }

    // Parse params
    if has_paren_params {
        parse_fn_clause_param_list(p);
    } else if has_bare_params {
        parse_bare_closure_params(p);
    } else {
        p.error("expected closure parameters, `->`, or `do`");
        return p.close(m, SyntaxKind::CLOSURE_EXPR);
    }

    // Optional guard clause
    if p.at(SyntaxKind::WHEN_KW) {
        let guard = p.open();
        p.advance(); // WHEN_KW
        expr(p);
        p.close(guard, SyntaxKind::GUARD_CLAUSE);
    }

    // Parse body
    if p.at(SyntaxKind::ARROW) {
        p.advance(); // ARROW

        // Wrap body expression in BLOCK for consistency with downstream code.
        // Use expr() instead of parse_block_body() so the Pratt parser exits
        // at BAR (not an infix operator), enabling multi-clause detection.
        if !p.has_error() {
            let block = p.open();
            expr(p);
            p.close(block, SyntaxKind::BLOCK);
        }

        // Multi-clause: BAR follows the body expression
        if p.at(SyntaxKind::BAR) {
            // First clause's children are already direct children of CLOSURE_EXPR.
            // Parse subsequent clauses wrapped in CLOSURE_CLAUSE nodes.
            while p.at(SyntaxKind::BAR) {
                parse_closure_clause(p);
                if p.has_error() {
                    break;
                }
            }

            expect_closure_end(p, fn_span);
            return p.close(m, SyntaxKind::CLOSURE_EXPR);
        }

        // Single-clause arrow closure
        expect_closure_end(p, fn_span);
        return p.close(m, SyntaxKind::CLOSURE_EXPR);
    } else if p.at(SyntaxKind::DO_KW) {
        // do/end body: `fn x do body end` -- single `end` for both block and closure.
        let do_span = p.current_span();
        p.advance(); // DO_KW
        parse_block_body(p);
        if !p.at(SyntaxKind::END_KW) {
            p.error_with_related(
                "unclosed closure -- expected `end`",
                do_span,
                "`do` block started here",
            );
        } else {
            p.advance(); // END_KW
        }
        return p.close(m, SyntaxKind::CLOSURE_EXPR);
    } else {
        p.error("expected `->` or `do` after closure parameters");
        return p.close(m, SyntaxKind::CLOSURE_EXPR);
    }
}

/// Parse a single additional closure clause: `| params [when guard] -> body`
///
/// Called for the 2nd, 3rd, ... clauses in a multi-clause closure.
/// Wraps the clause in a CLOSURE_CLAUSE node.
fn parse_closure_clause(p: &mut Parser) {
    let clause = p.open();
    p.advance(); // BAR

    // Parse clause params
    if p.at(SyntaxKind::L_PAREN) {
        parse_fn_clause_param_list(p);
    } else if looks_like_bare_closure_params(p) {
        parse_bare_closure_params(p);
    }

    // Optional guard
    if p.at(SyntaxKind::WHEN_KW) {
        let guard = p.open();
        p.advance(); // WHEN_KW
        expr(p);
        p.close(guard, SyntaxKind::GUARD_CLAUSE);
    }

    // Arrow and body
    if p.at(SyntaxKind::ARROW) {
        p.advance(); // ARROW
        if !p.has_error() {
            let block = p.open();
            expr(p);
            p.close(block, SyntaxKind::BLOCK);
        }
    } else if p.at(SyntaxKind::DO_KW) {
        let do_span = p.current_span();
        p.advance(); // DO_KW
        parse_block_body(p);
        if !p.at(SyntaxKind::END_KW) {
            p.error_with_related(
                "unclosed closure clause -- expected `end`",
                do_span,
                "`do` block started here",
            );
        } else {
            p.advance(); // END_KW (for do/end block)
        }
    } else {
        p.error("expected `->` or `do` after closure clause parameters");
    }

    p.close(clause, SyntaxKind::CLOSURE_CLAUSE);
}

/// Parse a closure body for the no-params variant.
/// Handles both `-> body` and `do body` forms. Produces a BLOCK.
fn parse_closure_body_block(p: &mut Parser, _fn_span: Span) {
    if p.at(SyntaxKind::ARROW) {
        p.advance(); // ARROW
        if !p.has_error() {
            parse_block_body(p);
        }
    } else if p.at(SyntaxKind::DO_KW) {
        p.advance(); // DO_KW
        parse_block_body(p);
    } else {
        p.error("expected `->` or `do` after `fn`");
    }
}

/// Expect END_KW to close a closure, emitting an error pointing back to fn if missing.
fn expect_closure_end(p: &mut Parser, fn_span: Span) {
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "unclosed closure -- expected `end`",
            fn_span,
            "closure started here",
        );
    } else {
        p.advance(); // END_KW
    }
}

/// Check whether the current position looks like bare closure parameters.
///
/// After `fn`, bare params can be:
/// - IDENT followed by `,`, `->`, `when`, `do`, `::` (regular param or typed param)
/// - INT_LITERAL/FLOAT_LITERAL/TRUE_KW/FALSE_KW/NIL_KW (pattern params)
/// - MINUS followed by INT_LITERAL/FLOAT_LITERAL (negative literal pattern)
/// - IDENT `_` (wildcard)
/// - Uppercase IDENT followed by `(` (constructor pattern like `Some(x)`)
fn looks_like_bare_closure_params(p: &Parser) -> bool {
    match p.current() {
        // Regular identifier param
        SyntaxKind::IDENT => {
            let text = p.current_text();
            if text == "_" {
                return true; // wildcard
            }
            // Uppercase IDENT followed by L_PAREN -> constructor pattern
            if text.starts_with(|c: char| c.is_uppercase()) && p.nth(1) == SyntaxKind::L_PAREN {
                return true;
            }
            // Lowercase ident -> bare param (check next token to be sure)
            let next = p.nth(1);
            matches!(
                next,
                SyntaxKind::COMMA
                    | SyntaxKind::ARROW
                    | SyntaxKind::WHEN_KW
                    | SyntaxKind::DO_KW
                    | SyntaxKind::COLON_COLON
            )
        }
        // Literal patterns
        SyntaxKind::INT_LITERAL
        | SyntaxKind::FLOAT_LITERAL
        | SyntaxKind::TRUE_KW
        | SyntaxKind::FALSE_KW
        | SyntaxKind::NIL_KW
        | SyntaxKind::STRING_START => true,
        // Negative literal pattern
        SyntaxKind::MINUS
            if matches!(
                p.nth(1),
                SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL
            ) =>
        {
            true
        }
        _ => false,
    }
}

/// Parse bare (non-parenthesized) closure parameters.
///
/// Opens a PARAM_LIST, parses comma-separated params using
/// parse_fn_clause_param, and closes. Stops at `->`, `when`, or `do`.
fn parse_bare_closure_params(p: &mut Parser) {
    let m = p.open();

    parse_fn_clause_param(p);
    while p.eat(SyntaxKind::COMMA) {
        // Stop if next token terminates param list
        if p.at(SyntaxKind::ARROW) || p.at(SyntaxKind::WHEN_KW) || p.at(SyntaxKind::DO_KW) {
            break;
        }
        parse_fn_clause_param(p);
    }

    p.close(m, SyntaxKind::PARAM_LIST);
}

// ── Parameter List ────────────────────────────────────────────────────

/// Parse a parameter list: `(param, param, ...)`
///
/// Each parameter is `name [:: Type]`.
pub(crate) fn parse_param_list(p: &mut Parser) {
    let m = p.open();
    p.advance(); // L_PAREN

    if !p.at(SyntaxKind::R_PAREN) {
        parse_param(p);
        while p.eat(SyntaxKind::COMMA) {
            if p.at(SyntaxKind::R_PAREN) {
                break; // trailing comma
            }
            parse_param(p);
        }
    }

    p.expect(SyntaxKind::R_PAREN);
    p.close(m, SyntaxKind::PARAM_LIST);
}

/// Parse a single parameter: `name [:: Type]` or `self`
fn parse_param(p: &mut Parser) {
    let m = p.open();

    // Parameter name: IDENT or self keyword.
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::SELF_KW) {
        p.advance();
    } else {
        p.error("expected parameter name");
    }

    // Optional type annotation: `:: Type`
    if p.at(SyntaxKind::COLON_COLON) {
        let ann = p.open();
        p.advance(); // ::
        super::items::parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }

    p.close(m, SyntaxKind::PARAM);
}

// ── Multi-Clause Function Parameter Parsing ──────────────────────────

/// Parse a parameter list for multi-clause function definitions: `(param, param, ...)`
///
/// Each parameter may be a pattern (literal, wildcard, constructor, tuple)
/// or a regular named parameter with optional type annotation.
pub(crate) fn parse_fn_clause_param_list(p: &mut Parser) {
    let m = p.open();
    p.advance(); // L_PAREN

    if !p.at(SyntaxKind::R_PAREN) {
        parse_fn_clause_param(p);
        while p.eat(SyntaxKind::COMMA) {
            if p.at(SyntaxKind::R_PAREN) {
                break; // trailing comma
            }
            parse_fn_clause_param(p);
        }
    }

    p.expect(SyntaxKind::R_PAREN);
    p.close(m, SyntaxKind::PARAM_LIST);
}

/// Parse a single function parameter that may be a pattern.
///
/// Detection logic:
/// - Literals (int, float, true, false, nil) -> pattern param
/// - `-` followed by number -> negative literal pattern param
/// - `_` -> wildcard pattern param
/// - `(` -> tuple pattern param
/// - Uppercase IDENT followed by `(` -> constructor pattern param
/// - Lowercase IDENT (not `_`) -> regular named param with optional `:: Type`
/// - `self` -> regular param
pub(crate) fn parse_fn_clause_param(p: &mut Parser) {
    let m = p.open();

    match p.current() {
        // Literal patterns: 0, 1, 3.14, true, false, nil
        SyntaxKind::INT_LITERAL
        | SyntaxKind::FLOAT_LITERAL
        | SyntaxKind::TRUE_KW
        | SyntaxKind::FALSE_KW
        | SyntaxKind::NIL_KW => {
            super::patterns::parse_pattern(p);
        }

        // Negative literal pattern: -1, -3.14
        SyntaxKind::MINUS
            if matches!(
                p.nth(1),
                SyntaxKind::INT_LITERAL | SyntaxKind::FLOAT_LITERAL
            ) =>
        {
            super::patterns::parse_pattern(p);
        }

        // String literal pattern
        SyntaxKind::STRING_START => {
            super::patterns::parse_pattern(p);
        }

        // Tuple pattern: (a, b)
        SyntaxKind::L_PAREN => {
            super::patterns::parse_pattern(p);
        }

        SyntaxKind::IDENT => {
            let text = p.current_text().to_string();

            if text == "_" {
                // Wildcard pattern
                super::patterns::parse_pattern(p);
            } else if text.starts_with(|c: char| c.is_uppercase())
                && p.nth(1) == SyntaxKind::L_PAREN
            {
                // Constructor pattern: Some(x), Ok(val)
                super::patterns::parse_pattern(p);
            } else if text.starts_with(|c: char| c.is_uppercase())
                && p.nth(1) == SyntaxKind::DOT
            {
                // Qualified constructor pattern: Shape.Circle(r)
                super::patterns::parse_pattern(p);
            } else {
                // Regular identifier parameter with optional type annotation
                p.advance(); // ident

                // Optional type annotation: `:: Type`
                if p.at(SyntaxKind::COLON_COLON) {
                    let ann = p.open();
                    p.advance(); // ::
                    super::items::parse_type(p);
                    p.close(ann, SyntaxKind::TYPE_ANNOTATION);
                }
            }
        }

        // self keyword as parameter
        SyntaxKind::SELF_KW => {
            p.advance();
        }

        _ => {
            p.error("expected parameter name or pattern");
        }
    }

    p.close(m, SyntaxKind::PARAM);
}

// ── Trailing Closure ──────────────────────────────────────────────────

/// Parse a trailing closure: `do [|params|] body end`
///
/// Attached to the preceding CALL_EXPR as a TRAILING_CLOSURE child.
fn parse_trailing_closure(p: &mut Parser) {
    let m = p.open();
    let do_span = p.current_span();
    p.advance(); // DO_KW

    // Optional closure params: `do |x, y| ... end`
    // The lexer emits bare `|` as BAR tokens.
    if p.at(SyntaxKind::BAR) {
        p.advance(); // opening |
        // Parse params between pipes.
        let params = p.open();
        if !p.at(SyntaxKind::BAR) {
            parse_param(p);
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::BAR) {
                    break;
                }
                parse_param(p);
            }
        }
        p.close(params, SyntaxKind::PARAM_LIST);
        // Expect closing |
        if p.at(SyntaxKind::BAR) {
            p.advance(); // closing |
        } else {
            p.error("expected `|` to close trailing closure parameters");
        }
    }

    // Parse block body.
    parse_block_body(p);

    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close `do` block",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::TRAILING_CLOSURE);
}

// ── While/Break/Continue Expression Parsing ──────────────────────────

/// Parse a while expression: `while cond do body end`
fn parse_while_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // WHILE_KW

    // Parse condition.
    expr(p);

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse body.
    parse_block_body(p);

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close `do` block",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::WHILE_EXPR)
}

/// Parse a break expression: `break`
fn parse_break_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // BREAK_KW
    p.close(m, SyntaxKind::BREAK_EXPR)
}

/// Parse a continue expression: `continue`
fn parse_continue_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // CONTINUE_KW
    p.close(m, SyntaxKind::CONTINUE_EXPR)
}

// ── For-In Expression Parsing ─────────────────────────────────────────

/// Parse a for-in expression: `for binding in iterable do body end`
///
/// Supports two binding forms:
/// - Simple: `for x in iterable do body end`
/// - Destructuring: `for {k, v} in map do body end`
fn parse_for_in_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // FOR_KW

    // Parse binding: either a single IDENT (NAME) or {k, v} (DESTRUCTURE_BINDING).
    if p.at(SyntaxKind::L_BRACE) {
        // Destructuring binding: {k, v}
        let dm = p.open();
        p.advance(); // {
        if p.at(SyntaxKind::IDENT) {
            let n = p.open();
            p.advance();
            p.close(n, SyntaxKind::NAME);
        }
        while p.eat(SyntaxKind::COMMA) {
            if p.at(SyntaxKind::R_BRACE) {
                break;
            }
            if p.at(SyntaxKind::IDENT) {
                let n = p.open();
                p.advance();
                p.close(n, SyntaxKind::NAME);
            }
        }
        p.expect(SyntaxKind::R_BRACE);
        p.close(dm, SyntaxKind::DESTRUCTURE_BINDING);
    } else if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance(); // identifier
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected loop variable name or {key, value} destructuring after `for`");
    }

    // Expect `in`.
    p.expect(SyntaxKind::IN_KW);

    // Parse iterable expression (e.g., 0..10 parses as BINARY_EXPR with DOT_DOT).
    if !p.has_error() {
        expr(p);
    }

    // Optional filter clause: `when condition`
    if p.at(SyntaxKind::WHEN_KW) {
        p.advance(); // WHEN_KW
        expr(p); // filter expression
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse body.
    parse_block_body(p);

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close `for` block",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::FOR_IN_EXPR)
}

// ── Actor Expression Parsing ──────────────────────────────────────────

/// Parse a spawn expression: `spawn(func, args...)`
fn parse_spawn_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // SPAWN_KW

    // Expect argument list.
    if p.at(SyntaxKind::L_PAREN) {
        parse_arg_list(p);
    } else {
        p.error("expected `(` after `spawn`");
    }

    p.close(m, SyntaxKind::SPAWN_EXPR)
}

/// Parse a send expression: `send(target, message)`
fn parse_send_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // SEND_KW

    // Expect argument list.
    if p.at(SyntaxKind::L_PAREN) {
        parse_arg_list(p);
    } else {
        p.error("expected `(` after `send`");
    }

    p.close(m, SyntaxKind::SEND_EXPR)
}

/// Parse a receive expression: `receive do pattern -> body ... [after timeout -> body] end`
fn parse_receive_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // RECEIVE_KW

    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse receive arms until END_KW, AFTER_KW, or EOF.
    loop {
        p.eat_newlines();

        if p.at(SyntaxKind::END_KW) || p.at(SyntaxKind::EOF) {
            break;
        }

        // Check for `after` clause.
        if p.at(SyntaxKind::AFTER_KW) {
            parse_after_clause(p);
            break;
        }

        parse_receive_arm(p);

        if p.has_error() {
            break;
        }
    }

    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close `receive` block",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::RECEIVE_EXPR)
}

/// Parse a single receive arm: `pattern -> body`
fn parse_receive_arm(p: &mut Parser) {
    let m = p.open();

    // Parse pattern.
    super::patterns::parse_pattern(p);

    // Optional `when` guard.
    if p.at(SyntaxKind::WHEN_KW) {
        p.advance(); // WHEN_KW
        expr(p);
    }

    // Expect `->`.
    p.expect(SyntaxKind::ARROW);

    // Parse arm body: a single expression.
    if !p.has_error() {
        expr(p);
    }

    p.close(m, SyntaxKind::RECEIVE_ARM);
}

/// Parse an after clause in a receive block: `after timeout -> body`
fn parse_after_clause(p: &mut Parser) {
    let m = p.open();
    p.advance(); // AFTER_KW

    // Parse timeout expression.
    expr(p);

    // Expect `->`.
    p.expect(SyntaxKind::ARROW);

    // Parse timeout body.
    if !p.has_error() {
        expr(p);
    }

    p.close(m, SyntaxKind::AFTER_CLAUSE);
}

/// Parse a self expression: `self()`
fn parse_self_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // SELF_KW

    // Expect `()` for self() call syntax.
    p.expect(SyntaxKind::L_PAREN);
    p.expect(SyntaxKind::R_PAREN);

    p.close(m, SyntaxKind::SELF_EXPR)
}

/// Parse a link expression: `link(pid)`
fn parse_link_expr(p: &mut Parser) -> MarkClosed {
    let m = p.open();
    p.advance(); // LINK_KW

    // Expect argument list.
    if p.at(SyntaxKind::L_PAREN) {
        parse_arg_list(p);
    } else {
        p.error("expected `(` after `link`");
    }

    p.close(m, SyntaxKind::LINK_EXPR)
}
