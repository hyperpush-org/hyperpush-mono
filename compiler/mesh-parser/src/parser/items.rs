//! Item/declaration parsers for Mesh.
//!
//! Parses top-level and nested declarations: function definitions, module
//! definitions, import declarations, and struct definitions. Handles visibility
//! (pub keyword) and type annotations.

use crate::syntax_kind::SyntaxKind;

use super::expressions::{parse_fn_clause_param_list, parse_param_list};
use super::Parser;

// ── Visibility ───────────────────────────────────────────────────────────

/// If the current token is `pub`, parse it as a VISIBILITY node.
fn parse_optional_visibility(p: &mut Parser) {
    if p.at(SyntaxKind::PUB_KW) {
        let m = p.open();
        p.advance(); // pub
        p.close(m, SyntaxKind::VISIBILITY);
    }
}

// ── Function Definition ──────────────────────────────────────────────────

/// Parse a function definition.
///
/// Supports two body forms:
/// - Block body: `[pub] fn|def name(params) [-> ReturnType] [where ...] do body end`
/// - Expression body: `[pub] fn|def name(pattern_params) [when guard] = expr`
///
/// Pattern parameters (literals, wildcards, constructors, tuples) are supported
/// alongside regular named parameters in both forms.
pub(crate) fn parse_fn_def(p: &mut Parser) {
    let m = p.open();

    // Optional visibility.
    parse_optional_visibility(p);

    // Consume fn or def keyword.
    p.advance(); // FN_KW or DEF_KW

    // Function name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected function name");
        p.close(m, SyntaxKind::FN_DEF);
        return;
    }

    // Optional type parameters: <T, U>
    if p.at(SyntaxKind::LT) {
        parse_generic_param_list(p);
    }

    // Parameter list -- use clause-aware parsing to support patterns.
    if p.at(SyntaxKind::L_PAREN) {
        parse_fn_clause_param_list(p);
    }

    // Optional return type: -> Type
    if p.at(SyntaxKind::ARROW) {
        let ann = p.open();
        p.advance(); // ->
        parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }

    // Optional where clause: where T: Trait
    if p.at(SyntaxKind::WHERE_KW) {
        parse_where_clause(p);
    }

    // Optional guard clause: when <expr>
    if p.at(SyntaxKind::WHEN_KW) {
        let guard = p.open();
        p.advance(); // WHEN_KW
        super::expressions::expr(p);
        p.close(guard, SyntaxKind::GUARD_CLAUSE);
    }

    // Determine body form: `= expr` or `do ... end`
    if p.at(SyntaxKind::EQ) {
        // Expression body form: fn name(pattern) = expr
        p.advance(); // EQ

        let body = p.open();
        super::expressions::expr(p);
        p.close(body, SyntaxKind::FN_EXPR_BODY);
    } else if p.at(SyntaxKind::DO_KW) {
        // Block body form: fn name(params) do ... end
        let do_span = p.current_span();
        p.advance(); // DO_KW

        parse_item_block_body(p);

        if !p.at(SyntaxKind::END_KW) {
            p.error_with_related(
                "expected `end` to close function body",
                do_span,
                "`do` block started here",
            );
        } else {
            p.advance(); // END_KW
        }
    } else {
        p.error("expected `=` or `do` for function body");
    }

    p.close(m, SyntaxKind::FN_DEF);
}

// ── Module Definition ────────────────────────────────────────────────────

/// Parse a module definition: `[pub] module Name do items end`
pub(crate) fn parse_module_def(p: &mut Parser) {
    let m = p.open();

    // Optional visibility.
    parse_optional_visibility(p);

    p.advance(); // MODULE_KW

    // Module name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected module name");
        p.close(m, SyntaxKind::MODULE_DEF);
        return;
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse module items.
    if !p.has_error() {
        parse_item_block_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close module body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::MODULE_DEF);
}

// ── Import Declarations ──────────────────────────────────────────────────

/// Parse an import declaration: `import ModulePath`
///
/// Module path is dot-separated: `import Foo.Bar.Baz`
pub(crate) fn parse_import_decl(p: &mut Parser) {
    let m = p.open();
    p.advance(); // IMPORT_KW

    // Parse module path.
    parse_module_path(p);

    p.close(m, SyntaxKind::IMPORT_DECL);
}

/// Parse a from-import declaration: `from ModulePath import name1, name2`
///
/// Glob imports (`from Module import *`) are rejected.
pub(crate) fn parse_from_import_decl(p: &mut Parser) {
    let m = p.open();

    // "from" is an identifier (not a keyword), advance it.
    p.advance(); // "from" IDENT

    // Parse module path.
    parse_module_path(p);

    // Expect `import`.
    p.expect(SyntaxKind::IMPORT_KW);

    // Parse import list.
    if !p.has_error() {
        let list = p.open();

        // Reject glob import: `from Module import *`
        if p.at(SyntaxKind::STAR) {
            p.error("glob imports are not allowed; import names explicitly");
            p.close(list, SyntaxKind::IMPORT_LIST);
            p.close(m, SyntaxKind::FROM_IMPORT_DECL);
            return;
        }

        if p.at(SyntaxKind::IDENT) {
            let name = p.open();
            p.advance();
            p.close(name, SyntaxKind::NAME);

            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::NEWLINE) || p.at(SyntaxKind::EOF) {
                    break;
                }
                let name = p.open();
                p.expect(SyntaxKind::IDENT);
                p.close(name, SyntaxKind::NAME);
            }
        } else {
            p.error("expected import name");
        }

        p.close(list, SyntaxKind::IMPORT_LIST);
    }

    p.close(m, SyntaxKind::FROM_IMPORT_DECL);
}

/// Parse a dot-separated module path: `Foo.Bar.Baz`
fn parse_module_path(p: &mut Parser) {
    let m = p.open();

    if p.at(SyntaxKind::IDENT) {
        p.advance(); // first segment

        while p.at(SyntaxKind::DOT) {
            p.advance(); // .
            p.expect(SyntaxKind::IDENT);
        }
    } else {
        p.error("expected module name");
    }

    p.close(m, SyntaxKind::PATH);
}

// ── Struct Definition ────────────────────────────────────────────────────

/// Parse a struct definition: `[pub] struct Name [TypeParams] do fields end`
pub(crate) fn parse_struct_def(p: &mut Parser) {
    let m = p.open();

    // Optional visibility.
    parse_optional_visibility(p);

    p.advance(); // STRUCT_KW

    // Struct name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected struct name");
        p.close(m, SyntaxKind::STRUCT_DEF);
        return;
    }

    // Optional type parameters: <A, B>
    if p.at(SyntaxKind::LT) {
        parse_generic_param_list(p);
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse fields and relationship declarations.
    if !p.has_error() {
        loop {
            p.eat_newlines();

            if p.at(SyntaxKind::END_KW) || p.at(SyntaxKind::EOF) {
                break;
            }

            // Check for schema option declarations: table, primary_key, timestamps
            // These must come before field declarations in struct bodies.
            if p.at(SyntaxKind::IDENT) {
                let text = p.current_text().to_string();
                if text == "table" || text == "primary_key" || text == "timestamps" {
                    parse_schema_option(p);
                    if p.has_error() {
                        break;
                    }
                    continue;
                }
            }

            // Check for relationship declarations: belongs_to, has_many, has_one
            if p.at(SyntaxKind::IDENT) {
                let text = p.current_text().to_string();
                if text == "belongs_to" || text == "has_many" || text == "has_one" {
                    parse_relationship_decl(p);
                    if p.has_error() {
                        break;
                    }
                    continue;
                }
            }

            parse_struct_field(p);

            if p.has_error() {
                break;
            }
        }
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close struct body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    // Optional deriving clause: end deriving(Trait1, Trait2, ...)
    if p.at(SyntaxKind::IDENT) && p.current_text() == "deriving" {
        parse_deriving_clause(p);
    }

    p.close(m, SyntaxKind::STRUCT_DEF);
}

/// Parse a struct field: `name :: Type`
fn parse_struct_field(p: &mut Parser) {
    let m = p.open();

    // Field name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected field name");
        p.close(m, SyntaxKind::STRUCT_FIELD);
        return;
    }

    // Type annotation: :: Type
    if p.at(SyntaxKind::COLON_COLON) {
        let ann = p.open();
        p.advance(); // ::
        parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }

    p.close(m, SyntaxKind::STRUCT_FIELD);
}

/// Parse a relationship declaration inside a struct body.
///
/// Syntax: `belongs_to :name, Type` / `has_many :name, Type` / `has_one :name, Type`
///
/// These are contextual identifiers (not keywords) recognized only inside struct bodies.
/// They produce RELATIONSHIP_DECL nodes containing the relationship kind identifier,
/// atom literal (association name), and type identifier (target type).
fn parse_relationship_decl(p: &mut Parser) {
    let m = p.open();

    // Bump the relationship kind identifier (belongs_to, has_many, has_one).
    p.advance(); // IDENT

    // Expect atom literal: :name
    if p.at(SyntaxKind::ATOM_LITERAL) {
        p.advance();
    } else {
        p.error("expected atom literal (e.g., :user) after relationship kind");
        p.close(m, SyntaxKind::RELATIONSHIP_DECL);
        return;
    }

    // Expect comma separator.
    p.expect(SyntaxKind::COMMA);

    // Expect target type name.
    if !p.has_error() {
        if p.at(SyntaxKind::IDENT) {
            p.advance();
        } else {
            p.error("expected target type name after comma in relationship declaration");
        }
    }

    p.close(m, SyntaxKind::RELATIONSHIP_DECL);
}

/// Parse a schema option declaration inside a struct body.
///
/// Syntax:
/// - `table "custom_table_name"` (STRING value)
/// - `primary_key :custom_pk` (ATOM value)
/// - `timestamps true` or `timestamps false` (IDENT value)
///
/// These are contextual identifiers (not keywords) recognized only inside struct bodies.
/// They produce SCHEMA_OPTION nodes containing the option name identifier and its value.
fn parse_schema_option(p: &mut Parser) {
    let m = p.open();

    // Read the option name (table, primary_key, timestamps).
    let option_name = p.current_text().to_string();
    p.advance(); // IDENT (option name)

    match option_name.as_str() {
        "table" => {
            // Expect a string value: "custom_table_name"
            if p.at(SyntaxKind::STRING_START) {
                // Consume STRING_START, STRING_CONTENT, STRING_END
                p.advance(); // STRING_START
                if p.at(SyntaxKind::STRING_CONTENT) {
                    p.advance(); // STRING_CONTENT
                }
                if p.at(SyntaxKind::STRING_END) {
                    p.advance(); // STRING_END
                } else {
                    p.error("unterminated string in table option");
                }
            } else {
                p.error("expected string literal after `table` (e.g., table \"people\")");
            }
        }
        "primary_key" => {
            // Expect an atom literal: :custom_pk
            if p.at(SyntaxKind::ATOM_LITERAL) {
                p.advance(); // ATOM_LITERAL
            } else {
                p.error("expected atom literal after `primary_key` (e.g., primary_key :uuid)");
            }
        }
        "timestamps" => {
            // Expect an IDENT "true" or "false"
            if p.at(SyntaxKind::TRUE_KW) {
                p.advance(); // TRUE_KW
            } else if p.at(SyntaxKind::FALSE_KW) {
                p.advance(); // FALSE_KW
            } else if p.at(SyntaxKind::IDENT) {
                let val = p.current_text().to_string();
                if val == "true" || val == "false" {
                    p.advance(); // IDENT
                } else {
                    p.error("expected `true` or `false` after `timestamps`");
                }
            } else {
                p.error("expected `true` or `false` after `timestamps`");
            }
        }
        _ => {
            p.error(&format!("unknown schema option `{}`", option_name));
        }
    }

    p.close(m, SyntaxKind::SCHEMA_OPTION);

    // Skip optional newline after the option.
    p.eat(SyntaxKind::NEWLINE);
}

/// Parse a deriving clause: `deriving(Trait1, Trait2, ...)`
///
/// Called after `end` in struct and sum type definitions. The `deriving` identifier
/// is parsed as a contextual keyword (regular IDENT whose text is "deriving").
fn parse_deriving_clause(p: &mut Parser) {
    let dc = p.open();
    p.advance(); // "deriving" IDENT
    p.expect(SyntaxKind::L_PAREN);
    loop {
        if p.at(SyntaxKind::R_PAREN) || p.at(SyntaxKind::EOF) {
            break;
        }
        if p.at(SyntaxKind::IDENT) {
            p.advance(); // trait name
        } else {
            p.error("expected trait name in deriving clause");
            break;
        }
        if !p.eat(SyntaxKind::COMMA) {
            break;
        }
    }
    p.expect(SyntaxKind::R_PAREN);
    p.close(dc, SyntaxKind::DERIVING_CLAUSE);
}

/// Parse a generic parameter list: `<A, B, C>`
fn parse_generic_param_list(p: &mut Parser) {
    let m = p.open();
    p.advance(); // <

    if !p.at(SyntaxKind::GT) {
        p.expect(SyntaxKind::IDENT);
        while p.eat(SyntaxKind::COMMA) {
            if p.at(SyntaxKind::GT) {
                break;
            }
            p.expect(SyntaxKind::IDENT);
        }
    }

    p.expect(SyntaxKind::GT);
    p.close(m, SyntaxKind::GENERIC_PARAM_LIST);
}

// ── Type Parsing ─────────────────────────────────────────────────────────

/// Parse a type expression: `Ident`, `Ident<A, B>`, `Mod.Type`, `Int?`, `T!E`
///
/// Supports:
/// - Simple types: `Int`, `String`
/// - Qualified types: `Foo.Bar`
/// - Generic applications: `List<Int>`, `Result<String, Error>`
/// - Option sugar: `Int?` (desugars to `Option<Int>`)
/// - Result sugar: `T!E` (desugars to `Result<T, E>`)
pub(crate) fn parse_type(p: &mut Parser) {
    // Tuple type: (A, B, C)
    if p.at(SyntaxKind::L_PAREN) {
        p.advance(); // (
        if !p.at(SyntaxKind::R_PAREN) {
            parse_type(p);
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::R_PAREN) {
                    break;
                }
                parse_type(p);
            }
        }
        p.expect(SyntaxKind::R_PAREN);
        return;
    }

    // Function type: Fun(ParamTypes) -> ReturnType
    if p.at(SyntaxKind::IDENT) && p.current_text() == "Fun" && p.nth(1) == SyntaxKind::L_PAREN {
        let m = p.open();
        p.advance(); // Fun
        p.advance(); // (
        // Parse comma-separated parameter types (may be empty for Fun() -> T)
        if !p.at(SyntaxKind::R_PAREN) {
            parse_type(p);
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::R_PAREN) {
                    break;
                }
                parse_type(p);
            }
        }
        p.expect(SyntaxKind::R_PAREN);
        p.expect(SyntaxKind::ARROW);
        if !p.has_error() {
            parse_type(p); // return type
        }
        p.close(m, SyntaxKind::FUN_TYPE);
        return;
    }

    if !p.at(SyntaxKind::IDENT) {
        p.error("expected type name");
        return;
    }

    // Parse the base type: Ident possibly with dots and generic args.
    // We emit the tokens directly (no wrapping node for simple types).
    p.advance(); // type name IDENT

    // Optional dot-separated path: Foo.Bar
    while p.at(SyntaxKind::DOT) {
        p.advance(); // .
        p.expect(SyntaxKind::IDENT);
    }

    // Optional generic arguments: <A, B>
    if p.at(SyntaxKind::LT) {
        let args = p.open();
        p.advance(); // <
        if !p.at(SyntaxKind::GT) {
            parse_type(p);
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::GT) {
                    break;
                }
                parse_type(p);
            }
        }
        p.expect(SyntaxKind::GT);
        p.close(args, SyntaxKind::GENERIC_ARG_LIST);
    }

    // Option sugar: Type? => OPTION_TYPE wrapping the base type
    if p.at(SyntaxKind::QUESTION) {
        p.advance(); // ?
        // The QUESTION token is emitted; the type checker will interpret
        // the preceding type + QUESTION as Option<Type>.
    }

    // Result sugar: Type!ErrorType => RESULT_TYPE wrapping both types
    if p.at(SyntaxKind::BANG) {
        p.advance(); // !
        parse_type(p); // error type
        // The BANG token followed by another type is emitted; the type checker
        // will interpret this as Result<Type, ErrorType>.
    }
}


// ── Interface Definition ─────────────────────────────────────────────────

/// Parse an interface definition: `[pub] interface Name [<T>] do method_sigs end`
pub(crate) fn parse_interface_def(p: &mut Parser) {
    let m = p.open();

    // Optional visibility.
    parse_optional_visibility(p);

    p.advance(); // INTERFACE_KW

    // Interface name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected interface name");
        p.close(m, SyntaxKind::INTERFACE_DEF);
        return;
    }

    // Optional type parameters: <T>
    if p.at(SyntaxKind::LT) {
        parse_generic_param_list(p);
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse method signatures and associated type declarations.
    if !p.has_error() {
        loop {
            p.eat_newlines();

            if p.at(SyntaxKind::END_KW) || p.at(SyntaxKind::EOF) {
                break;
            }

            if p.at(SyntaxKind::TYPE_KW) {
                parse_assoc_type_decl(p);
            } else {
                parse_interface_method(p);
            }

            if p.has_error() {
                break;
            }
        }
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close interface body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::INTERFACE_DEF);
}

/// Parse a method signature inside an interface: `fn name(params) [-> ReturnType] [do body end]`
///
/// The body is optional. Methods without `do ... end` are signature-only.
/// Methods with `do ... end` provide a default implementation.
fn parse_interface_method(p: &mut Parser) {
    let m = p.open();

    if !p.at(SyntaxKind::FN_KW) && !p.at(SyntaxKind::DEF_KW) {
        p.error("expected method signature (fn)");
        p.close(m, SyntaxKind::INTERFACE_METHOD);
        return;
    }

    p.advance(); // FN_KW or DEF_KW

    // Method name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected method name");
        p.close(m, SyntaxKind::INTERFACE_METHOD);
        return;
    }

    // Parameter list.
    if p.at(SyntaxKind::L_PAREN) {
        parse_param_list(p);
    }

    // Optional return type: -> Type
    if p.at(SyntaxKind::ARROW) {
        let ann = p.open();
        p.advance(); // ->
        parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }

    // Optional default body: do ... end
    if p.at(SyntaxKind::DO_KW) {
        let do_span = p.current_span();
        p.advance(); // DO_KW

        parse_item_block_body(p);

        if !p.at(SyntaxKind::END_KW) {
            p.error_with_related(
                "expected `end` to close default method body",
                do_span,
                "`do` block started here",
            );
        } else {
            p.advance(); // END_KW
        }
    }

    p.close(m, SyntaxKind::INTERFACE_METHOD);
}

/// Parse an associated type declaration inside an interface: `type Item`
fn parse_assoc_type_decl(p: &mut Parser) {
    let m = p.open();
    p.advance(); // TYPE_KW

    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected associated type name");
    }

    p.close(m, SyntaxKind::ASSOC_TYPE_DEF);
}

/// Parse an associated type binding inside an impl: `type Item = ConcreteType`
fn parse_assoc_type_binding(p: &mut Parser) {
    let m = p.open();
    p.advance(); // TYPE_KW

    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected associated type name");
        p.close(m, SyntaxKind::ASSOC_TYPE_BINDING);
        return;
    }

    p.expect(SyntaxKind::EQ); // =

    if !p.has_error() {
        parse_type(p); // The concrete type
    }

    p.close(m, SyntaxKind::ASSOC_TYPE_BINDING);
}

// ── Impl Definition ─────────────────────────────────────────────────────

/// Parse an impl block: `impl TraitName for TypeName [where ...] do fn_defs end`
pub(crate) fn parse_impl_def(p: &mut Parser) {
    let m = p.open();

    p.advance(); // IMPL_KW

    // Trait name (possibly qualified: Foo.Bar).
    if p.at(SyntaxKind::IDENT) {
        parse_module_path(p);
    } else {
        p.error("expected trait name");
        p.close(m, SyntaxKind::IMPL_DEF);
        return;
    }

    // Optional generic arguments on the trait: impl Trait<T> for ...
    if p.at(SyntaxKind::LT) {
        let args = p.open();
        p.advance(); // <
        if !p.at(SyntaxKind::GT) {
            parse_type(p);
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::GT) {
                    break;
                }
                parse_type(p);
            }
        }
        p.expect(SyntaxKind::GT);
        p.close(args, SyntaxKind::GENERIC_ARG_LIST);
    }

    // Expect `for`.
    p.expect(SyntaxKind::FOR_KW);

    // Type name (possibly qualified).
    if !p.has_error() {
        if p.at(SyntaxKind::IDENT) {
            parse_module_path(p);
        } else {
            p.error("expected type name");
        }
    }

    // Optional where clause.
    if p.at(SyntaxKind::WHERE_KW) {
        parse_where_clause(p);
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse method definitions and associated type bindings.
    if !p.has_error() {
        parse_impl_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close impl body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::IMPL_DEF);
}

/// Parse the body of an impl block: associated type bindings and method definitions.
///
/// Unlike `parse_item_block_body`, this specifically handles `type Item = T`
/// as associated type bindings rather than top-level type aliases/sum types.
fn parse_impl_body(p: &mut Parser) {
    let m = p.open();

    loop {
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        match p.current() {
            SyntaxKind::END_KW | SyntaxKind::EOF => break,
            SyntaxKind::TYPE_KW => {
                parse_assoc_type_binding(p);
            }
            SyntaxKind::FN_KW | SyntaxKind::DEF_KW => {
                parse_fn_def(p);
            }
            SyntaxKind::PUB_KW => {
                // pub fn ...
                match p.nth(1) {
                    SyntaxKind::FN_KW | SyntaxKind::DEF_KW => {
                        parse_fn_def(p);
                    }
                    _ => {
                        p.error("expected `fn` or `type` in impl body");
                        break;
                    }
                }
            }
            _ => {
                p.error("expected `fn`, `type`, or `end` in impl body");
                break;
            }
        }

        if p.has_error() {
            break;
        }

        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {}
            SyntaxKind::END_KW | SyntaxKind::EOF => {}
            _ => {}
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

// ── Sum Type Definition ──────────────────────────────────────────────────

/// Parse a sum type definition: `[pub] type Name [<T>] do Variant1(Type) ... end`
///
/// Each variant is either:
/// - Nullary: `Point` (no fields)
/// - Positional: `Circle(Float)` or `Pair(Int, Int)`
/// - Named: `Rectangle(width :: Float, height :: Float)`
pub(crate) fn parse_sum_type_def(p: &mut Parser) {
    let m = p.open();

    // Optional visibility.
    parse_optional_visibility(p);

    p.advance(); // TYPE_KW

    // Type name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected sum type name");
        p.close(m, SyntaxKind::SUM_TYPE_DEF);
        return;
    }

    // Optional type parameters: <T, U>
    if p.at(SyntaxKind::LT) {
        parse_generic_param_list(p);
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse variant definitions.
    if !p.has_error() {
        loop {
            p.eat_newlines();

            if p.at(SyntaxKind::END_KW) || p.at(SyntaxKind::EOF) {
                break;
            }

            parse_variant_def(p);

            if p.has_error() {
                break;
            }
        }
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close sum type body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    // Optional deriving clause: end deriving(Trait1, Trait2, ...)
    if p.at(SyntaxKind::IDENT) && p.current_text() == "deriving" {
        parse_deriving_clause(p);
    }

    p.close(m, SyntaxKind::SUM_TYPE_DEF);
}

/// Parse a single variant definition inside a sum type.
///
/// Variants:
/// - `VariantName` (nullary)
/// - `VariantName(Type1, Type2)` (positional)
/// - `VariantName(name1 :: Type1, name2 :: Type2)` (named fields)
fn parse_variant_def(p: &mut Parser) {
    let m = p.open();

    // Variant name.
    if p.at(SyntaxKind::IDENT) {
        p.advance(); // variant name IDENT
    } else {
        p.error("expected variant name");
        p.close(m, SyntaxKind::VARIANT_DEF);
        return;
    }

    // Optional field list: (fields...)
    if p.at(SyntaxKind::L_PAREN) {
        p.advance(); // (

        if !p.at(SyntaxKind::R_PAREN) {
            parse_variant_field_or_type(p);
            while p.eat(SyntaxKind::COMMA) {
                if p.at(SyntaxKind::R_PAREN) {
                    break; // trailing comma
                }
                parse_variant_field_or_type(p);
            }
        }

        p.expect(SyntaxKind::R_PAREN);
    }

    p.close(m, SyntaxKind::VARIANT_DEF);
}

/// Parse either a named field (`name :: Type`) or a positional type in a variant.
///
/// Distinguishes by checking if IDENT is followed by COLON_COLON (named field)
/// or something else (positional type).
fn parse_variant_field_or_type(p: &mut Parser) {
    // If IDENT followed by ::, it's a named field
    if p.at(SyntaxKind::IDENT) && p.nth(1) == SyntaxKind::COLON_COLON {
        let field = p.open();
        let name = p.open();
        p.advance(); // field name
        p.close(name, SyntaxKind::NAME);
        let ann = p.open();
        p.advance(); // ::
        parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
        p.close(field, SyntaxKind::VARIANT_FIELD);
    } else {
        // Positional type
        let ann = p.open();
        parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }
}

// ── Type Alias ──────────────────────────────────────────────────────────

/// Parse a type alias: `[pub] type Name [<T>] = Type`
pub(crate) fn parse_type_alias(p: &mut Parser) {
    let m = p.open();

    // Optional visibility (handles `pub type Alias = T`).
    parse_optional_visibility(p);

    p.advance(); // TYPE_KW

    // Type name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected type alias name");
        p.close(m, SyntaxKind::TYPE_ALIAS_DEF);
        return;
    }

    // Optional type parameters: <T>
    if p.at(SyntaxKind::LT) {
        parse_generic_param_list(p);
    }

    // Expect `=`.
    p.expect(SyntaxKind::EQ);

    // Parse the aliased type.
    if !p.has_error() {
        parse_type(p);
    }

    p.close(m, SyntaxKind::TYPE_ALIAS_DEF);
}

// ── Where Clause ────────────────────────────────────────────────────────

/// Parse a where clause: `where T: Trait, U: OtherTrait`
fn parse_where_clause(p: &mut Parser) {
    let m = p.open();
    p.advance(); // WHERE_KW

    // Parse comma-separated trait bounds: T: Trait
    parse_trait_bound(p);
    while p.eat(SyntaxKind::COMMA) {
        parse_trait_bound(p);
    }

    p.close(m, SyntaxKind::WHERE_CLAUSE);
}

/// Parse a single trait bound: `T: TraitName`
fn parse_trait_bound(p: &mut Parser) {
    let m = p.open();

    // Type parameter name.
    p.expect(SyntaxKind::IDENT);

    // Colon.
    p.expect(SyntaxKind::COLON);

    // Trait name (possibly qualified).
    if !p.has_error() {
        if p.at(SyntaxKind::IDENT) {
            p.advance();
            while p.at(SyntaxKind::DOT) {
                p.advance(); // .
                p.expect(SyntaxKind::IDENT);
            }
        } else {
            p.error("expected trait name");
        }
    }

    p.close(m, SyntaxKind::TRAIT_BOUND);
}

// ── Item Block Body ──────────────────────────────────────────────────────

/// Parse a block body that can contain items (fn, module, struct) as well
/// as statements/expressions.
///
/// This is used for module bodies and function bodies.
fn parse_item_block_body(p: &mut Parser) {
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

        // Parse an item or statement.
        super::parse_item_or_stmt(p);

        if p.has_error() {
            break;
        }

        // After a statement, handle separators.
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
            _ => {}
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

// ── Actor Definition ────────────────────────────────────────────────────

/// Parse an actor block definition: `actor Name(params) do body [terminate do ... end] end`
///
/// The actor block is a first-class language construct. Inside the body,
/// expressions can include receive blocks, send calls, spawn calls, and
/// a self() expression. An optional `terminate do ... end` clause defines
/// cleanup logic that runs before the actor fully terminates.
pub(crate) fn parse_actor_def(p: &mut Parser) {
    let m = p.open();

    p.advance(); // ACTOR_KW

    // Actor name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected actor name");
        p.close(m, SyntaxKind::ACTOR_DEF);
        return;
    }

    // Optional parameter list (state arguments).
    if p.at(SyntaxKind::L_PAREN) {
        parse_param_list(p);
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse actor body (statements and expressions).
    if !p.has_error() {
        parse_actor_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close actor body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::ACTOR_DEF);
}

/// Parse the body of an actor block.
///
/// The body can contain statements/expressions and an optional
/// `terminate do ... end` clause. Only one terminate clause is allowed.
fn parse_actor_body(p: &mut Parser) {
    let m = p.open();
    let mut seen_terminate = false;

    loop {
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        match p.current() {
            SyntaxKind::END_KW | SyntaxKind::EOF => break,
            SyntaxKind::TERMINATE_KW => {
                if seen_terminate {
                    p.error("only one `terminate` clause is allowed per actor block");
                    break;
                }
                seen_terminate = true;
                parse_terminate_clause(p);
            }
            _ => {
                super::parse_item_or_stmt(p);
            }
        }

        if p.has_error() {
            break;
        }

        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {}
            SyntaxKind::END_KW | SyntaxKind::EOF => {}
            _ => {}
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

/// Parse a terminate clause: `terminate do body end`
///
/// Defines cleanup logic that runs before the actor fully terminates.
fn parse_terminate_clause(p: &mut Parser) {
    let m = p.open();
    p.advance(); // TERMINATE_KW

    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    if !p.has_error() {
        super::expressions::parse_block_body(p);
    }

    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close `terminate` block",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::TERMINATE_CLAUSE);
}

// ── Supervisor Definition ─────────────────────────────────────────────

/// Parse a supervisor block definition: `[pub] supervisor Name do body end`
///
/// The supervisor body contains:
/// - `strategy:` clause (one_for_one, one_for_all, rest_for_one, simple_one_for_one)
/// - `max_restarts:` clause (integer)
/// - `max_seconds:` clause (integer)
/// - `child Name do ... end` blocks with start, restart, shutdown settings
pub(crate) fn parse_supervisor_def(p: &mut Parser) {
    let m = p.open();

    // Optional visibility.
    parse_optional_visibility(p);

    p.advance(); // SUPERVISOR_KW

    // Supervisor name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected supervisor name");
        p.close(m, SyntaxKind::SUPERVISOR_DEF);
        return;
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse supervisor body.
    if !p.has_error() {
        parse_supervisor_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close supervisor body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::SUPERVISOR_DEF);
}

/// Parse the body of a supervisor block.
///
/// Parses key-value clauses (strategy, max_restarts, max_seconds) and
/// child spec blocks (`child Name do ... end`).
fn parse_supervisor_body(p: &mut Parser) {
    let m = p.open();

    loop {
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        match p.current() {
            SyntaxKind::END_KW | SyntaxKind::EOF => break,
            SyntaxKind::IDENT => {
                let text = p.current_text().to_string();
                match text.as_str() {
                    "strategy" => parse_strategy_clause(p),
                    "max_restarts" => parse_restart_limit(p),
                    "max_seconds" => parse_seconds_limit(p),
                    "child" => parse_child_spec(p),
                    _ => {
                        p.error(&format!("unexpected `{}` in supervisor body", text));
                        break;
                    }
                }
            }
            _ => {
                p.error("expected `strategy`, `max_restarts`, `max_seconds`, `child`, or `end` in supervisor body");
                break;
            }
        }

        if p.has_error() {
            break;
        }

        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {}
            SyntaxKind::END_KW | SyntaxKind::EOF => {}
            _ => {}
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

/// Parse `strategy: one_for_one` (or one_for_all, rest_for_one, simple_one_for_one).
fn parse_strategy_clause(p: &mut Parser) {
    let m = p.open();
    p.advance(); // "strategy" IDENT

    p.expect(SyntaxKind::COLON);

    // Strategy value must be a known identifier.
    if p.at(SyntaxKind::IDENT) {
        p.advance();
    } else {
        p.error("expected strategy name (one_for_one, one_for_all, rest_for_one, simple_one_for_one)");
    }

    p.close(m, SyntaxKind::STRATEGY_CLAUSE);
}

/// Parse `max_restarts: 3`.
fn parse_restart_limit(p: &mut Parser) {
    let m = p.open();
    p.advance(); // "max_restarts" IDENT

    p.expect(SyntaxKind::COLON);

    if p.at(SyntaxKind::INT_LITERAL) {
        p.advance();
    } else {
        p.error("expected integer for max_restarts");
    }

    p.close(m, SyntaxKind::RESTART_LIMIT);
}

/// Parse `max_seconds: 5`.
fn parse_seconds_limit(p: &mut Parser) {
    let m = p.open();
    p.advance(); // "max_seconds" IDENT

    p.expect(SyntaxKind::COLON);

    if p.at(SyntaxKind::INT_LITERAL) {
        p.advance();
    } else {
        p.error("expected integer for max_seconds");
    }

    p.close(m, SyntaxKind::SECONDS_LIMIT);
}

/// Parse a child spec: `child Name do ... end`
///
/// The child body contains key-value pairs:
/// - `start: fn -> spawn(Actor, args) end`
/// - `restart: permanent | transient | temporary`
/// - `shutdown: 5000 | brutal_kill`
fn parse_child_spec(p: &mut Parser) {
    let m = p.open();
    p.advance(); // "child" IDENT

    // Child name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected child spec name");
        p.close(m, SyntaxKind::CHILD_SPEC_DEF);
        return;
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse child body key-value pairs.
    if !p.has_error() {
        parse_child_spec_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close child spec body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::CHILD_SPEC_DEF);
}

/// Parse child spec body key-value pairs.
///
/// Keys: `start`, `restart`, `shutdown`.
fn parse_child_spec_body(p: &mut Parser) {
    let m = p.open();

    loop {
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        match p.current() {
            SyntaxKind::END_KW | SyntaxKind::EOF => break,
            SyntaxKind::IDENT => {
                let text = p.current_text().to_string();
                match text.as_str() {
                    "start" | "restart" | "shutdown" => {
                        // Key: value pair. Advance key, expect colon.
                        p.advance(); // key IDENT
                        p.expect(SyntaxKind::COLON);

                        if p.has_error() {
                            break;
                        }

                        // Parse the value: could be an expression (fn -> ... end),
                        // an identifier (permanent, brutal_kill), or an integer (5000).
                        match text.as_str() {
                            "start" => {
                                // Parse as an expression (typically a closure).
                                super::expressions::expr(p);
                            }
                            "restart" => {
                                // Expect an identifier: permanent, transient, temporary.
                                if p.at(SyntaxKind::IDENT) {
                                    p.advance();
                                } else {
                                    p.error("expected restart strategy (permanent, transient, temporary)");
                                }
                            }
                            "shutdown" => {
                                // Either an integer or `brutal_kill`.
                                if p.at(SyntaxKind::INT_LITERAL) {
                                    p.advance();
                                } else if p.at(SyntaxKind::IDENT) {
                                    p.advance(); // brutal_kill
                                } else {
                                    p.error("expected shutdown timeout (integer or brutal_kill)");
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        p.error(&format!("unexpected `{}` in child spec body", text));
                        break;
                    }
                }
            }
            _ => {
                p.error("expected `start`, `restart`, `shutdown`, or `end` in child spec body");
                break;
            }
        }

        if p.has_error() {
            break;
        }

        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {}
            SyntaxKind::END_KW | SyntaxKind::EOF => {}
            _ => {}
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

// ── Service Definition ─────────────────────────────────────────────

/// Parse a service block definition: `service Name do ... end`
///
/// The service body contains:
/// - `fn init(params) :: ReturnType do ... end` (initialization function)
/// - `call Name(params) :: ReturnType do |state| ... end` (synchronous handlers)
/// - `cast Name(params) do |state| ... end` (asynchronous handlers)
pub(crate) fn parse_service_def(p: &mut Parser) {
    let m = p.open();

    p.advance(); // SERVICE_KW

    // Service name.
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected service name");
        p.close(m, SyntaxKind::SERVICE_DEF);
        return;
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    // Parse service body.
    if !p.has_error() {
        parse_service_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close service body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::SERVICE_DEF);
}

/// Parse the body of a service block.
///
/// Parses `fn` definitions (init), `call` handlers, and `cast` handlers.
fn parse_service_body(p: &mut Parser) {
    let m = p.open();

    loop {
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        match p.current() {
            SyntaxKind::END_KW | SyntaxKind::EOF => break,
            SyntaxKind::FN_KW | SyntaxKind::DEF_KW => {
                // Parse fn init(...) as a regular function definition.
                super::parse_item_or_stmt(p);
            }
            SyntaxKind::CALL_KW => {
                parse_call_handler(p);
            }
            SyntaxKind::CAST_KW => {
                parse_cast_handler(p);
            }
            _ => {
                p.error("expected `fn`, `call`, `cast`, or `end` in service body");
                break;
            }
        }

        if p.has_error() {
            break;
        }

        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {}
            SyntaxKind::END_KW | SyntaxKind::EOF => {}
            _ => {}
        }
    }

    p.close(m, SyntaxKind::BLOCK);
}

/// Parse a call handler: `call Name(params) :: ReturnType do |state| body end`
///
/// Call handlers are synchronous request handlers in a service. They receive
/// the current state and must return a tuple of `{new_state, reply}`.
fn parse_call_handler(p: &mut Parser) {
    let m = p.open();
    p.advance(); // CALL_KW

    // Handler name (variant name, e.g., GetCount).
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected call handler name");
        p.close(m, SyntaxKind::CALL_HANDLER);
        return;
    }

    // Optional parameter list: (params)
    if p.at(SyntaxKind::L_PAREN) {
        parse_param_list(p);
    }

    // Return type annotation: :: Type
    if p.at(SyntaxKind::COLON_COLON) {
        let ann = p.open();
        p.advance(); // ::
        parse_type(p);
        p.close(ann, SyntaxKind::TYPE_ANNOTATION);
    }

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    if p.has_error() {
        p.close(m, SyntaxKind::CALL_HANDLER);
        return;
    }

    // Parse |state| parameter.
    if p.at(SyntaxKind::BAR) {
        p.advance(); // |
        if p.at(SyntaxKind::IDENT) {
            let name = p.open();
            p.advance();
            p.close(name, SyntaxKind::NAME);
        } else {
            p.error("expected state parameter name");
        }
        p.expect(SyntaxKind::BAR);
    }

    // Parse handler body.
    if !p.has_error() {
        super::expressions::parse_block_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close call handler body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::CALL_HANDLER);
}

/// Parse a cast handler: `cast Name(params) do |state| body end`
///
/// Cast handlers are asynchronous (fire-and-forget) handlers in a service.
/// They receive the current state and must return the new state.
fn parse_cast_handler(p: &mut Parser) {
    let m = p.open();
    p.advance(); // CAST_KW

    // Handler name (variant name, e.g., Reset).
    if p.at(SyntaxKind::IDENT) {
        let name = p.open();
        p.advance();
        p.close(name, SyntaxKind::NAME);
    } else {
        p.error("expected cast handler name");
        p.close(m, SyntaxKind::CAST_HANDLER);
        return;
    }

    // Optional parameter list: (params)
    if p.at(SyntaxKind::L_PAREN) {
        parse_param_list(p);
    }

    // No return type for cast (fire-and-forget, returns Unit).

    // Expect `do`.
    let do_span = p.current_span();
    p.expect(SyntaxKind::DO_KW);

    if p.has_error() {
        p.close(m, SyntaxKind::CAST_HANDLER);
        return;
    }

    // Parse |state| parameter.
    if p.at(SyntaxKind::BAR) {
        p.advance(); // |
        if p.at(SyntaxKind::IDENT) {
            let name = p.open();
            p.advance();
            p.close(name, SyntaxKind::NAME);
        } else {
            p.error("expected state parameter name");
        }
        p.expect(SyntaxKind::BAR);
    }

    // Parse handler body.
    if !p.has_error() {
        super::expressions::parse_block_body(p);
    }

    // Expect `end`.
    if !p.at(SyntaxKind::END_KW) {
        p.error_with_related(
            "expected `end` to close cast handler body",
            do_span,
            "`do` block started here",
        );
    } else {
        p.advance(); // END_KW
    }

    p.close(m, SyntaxKind::CAST_HANDLER);
}
