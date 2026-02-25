//! SyntaxKind enum for the Mesh CST.
//!
//! This is a superset of `TokenKind` (mapped to SCREAMING_SNAKE_CASE) plus
//! composite node kinds for CST nodes produced by the parser.

use mesh_common::token::TokenKind;

/// Every kind of syntax element in the Mesh CST.
///
/// Token kinds (leaves) are mapped 1:1 from [`TokenKind`]. Composite node kinds
/// represent parser-produced tree nodes. The first two values are sentinels used
/// by the event-based parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // ── Sentinels ──────────────────────────────────────────────────────
    /// Placeholder kind for incomplete/unfinished parser events.
    TOMBSTONE = 0,
    /// Wrapper for tokens/nodes that couldn't be parsed.
    ERROR_NODE = 1,

    // ── Keywords (48) ──────────────────────────────────────────────────
    ACTOR_KW,
    AFTER_KW,
    ALIAS_KW,
    AND_KW,
    CALL_KW,
    CASE_KW,
    CAST_KW,
    COND_KW,
    DEF_KW,
    DO_KW,
    ELSE_KW,
    END_KW,
    FALSE_KW,
    FN_KW,
    FOR_KW,
    IF_KW,
    IMPL_KW,
    IMPORT_KW,
    INTERFACE_KW,
    IN_KW,
    LET_KW,
    LINK_KW,
    MATCH_KW,
    MODULE_KW,
    MONITOR_KW,
    NIL_KW,
    NOT_KW,
    OR_KW,
    PUB_KW,
    RECEIVE_KW,
    RETURN_KW,
    SELF_KW,
    SEND_KW,
    SERVICE_KW,
    SPAWN_KW,
    STRUCT_KW,
    SUPERVISOR_KW,
    TERMINATE_KW,
    TRAIT_KW,
    TRAP_KW,
    TRUE_KW,
    TYPE_KW,
    WHEN_KW,
    WHERE_KW,
    WHILE_KW,
    WITH_KW,
    BREAK_KW,
    CONTINUE_KW,

    // ── Operators (24) ─────────────────────────────────────────────────
    PLUS,
    MINUS,
    STAR,
    SLASH,
    PERCENT,
    EQ_EQ,
    NOT_EQ,
    LT,
    GT,
    LT_EQ,
    GT_EQ,
    AMP_AMP,
    PIPE_PIPE,
    BANG,
    /// `|>` pipe operator
    PIPE,
    /// `|N>` slot pipe operator (e.g. `|2>`, `|3>`)
    SLOT_PIPE,
    DOT_DOT,
    DIAMOND,
    PLUS_PLUS,
    EQ,
    ARROW,
    FAT_ARROW,
    COLON_COLON,
    QUESTION,
    /// `|` bare pipe for or-patterns
    BAR,

    // ── Delimiters (6) ─────────────────────────────────────────────────
    L_PAREN,
    R_PAREN,
    L_BRACKET,
    R_BRACKET,
    L_BRACE,
    R_BRACE,

    // ── Punctuation (5) ────────────────────────────────────────────────
    COMMA,
    DOT,
    COLON,
    SEMICOLON,
    NEWLINE,

    // ── Literals (8) ───────────────────────────────────────────────────
    INT_LITERAL,
    FLOAT_LITERAL,
    STRING_START,
    STRING_END,
    STRING_CONTENT,
    INTERPOLATION_START,
    INTERPOLATION_END,
    ATOM_LITERAL,

    // ── Identifiers and comments (4) ───────────────────────────────────
    IDENT,
    COMMENT,
    DOC_COMMENT,
    MODULE_DOC_COMMENT,

    // ── Special (2) ────────────────────────────────────────────────────
    EOF,
    /// Lexer error token
    ERROR,

    // ── Whitespace (parser-only, not from TokenKind) ───────────────────
    /// Whitespace trivia. The lexer skips whitespace, but this kind
    /// exists for potential future use in lossless CST reconstruction.
    WHITESPACE,

    // ── Composite node kinds (~43) ─────────────────────────────────────
    /// Root node of a parsed source file.
    SOURCE_FILE,
    /// Function definition: `fn name(params) do ... end`
    FN_DEF,
    /// Let binding: `let x = expr`
    LET_BINDING,
    /// Return expression: `return expr`
    RETURN_EXPR,
    /// If expression: `if cond do ... else ... end`
    IF_EXPR,
    /// Else branch of an if expression.
    ELSE_BRANCH,
    /// Case/match expression: `case expr do ... end`
    CASE_EXPR,
    /// Single arm in a case/match expression.
    MATCH_ARM,
    /// Binary expression: `a + b`, `a == b`, etc.
    BINARY_EXPR,
    /// Unary expression: `-x`, `!x`, `not x`
    UNARY_EXPR,
    /// Function call: `f(args)`
    CALL_EXPR,
    /// Pipe expression: `x |> f`
    PIPE_EXPR,
    /// Slot pipe expression: `x |N> f(a)` -- pipes x as the N-th argument
    SLOT_PIPE_EXPR,
    /// Field access: `expr.field`
    FIELD_ACCESS,
    /// Index expression: `expr[index]`
    INDEX_EXPR,
    /// Block: sequence of expressions/statements.
    BLOCK,
    /// Parameter list: `(a, b, c)`
    PARAM_LIST,
    /// Single parameter in a parameter list.
    PARAM,
    /// Argument list: `(a, b, c)`
    ARG_LIST,
    /// Module definition: `module Name do ... end`
    MODULE_DEF,
    /// Import declaration: `import Module`
    IMPORT_DECL,
    /// From-import declaration: `from Module import a, b`
    FROM_IMPORT_DECL,
    /// Struct definition: `struct Name do ... end`
    STRUCT_DEF,
    /// Single field in a struct definition.
    STRUCT_FIELD,
    /// Closure expression: `fn x -> x + 1 end`
    CLOSURE_EXPR,
    /// Literal expression (int, float, bool, nil).
    LITERAL,
    /// Atom literal expression: `:name`, `:email`, `:asc`.
    ATOM_EXPR,
    /// Name in a definition position.
    NAME,
    /// Name reference (identifier used as expression).
    NAME_REF,
    /// Qualified path: `Module.name`
    PATH,
    /// Type annotation: `:: Type`
    TYPE_ANNOTATION,
    /// Visibility modifier: `pub`
    VISIBILITY,
    /// Wildcard pattern: `_`
    WILDCARD_PAT,
    /// Identifier pattern: `x`
    IDENT_PAT,
    /// Literal pattern: `42`, `"hello"`, `true`
    LITERAL_PAT,
    /// Tuple pattern: `(a, b, c)`
    TUPLE_PAT,
    /// Struct pattern: `Point { x, y }`
    STRUCT_PAT,
    /// Interpolated string expression: `"hello ${name}"`
    STRING_EXPR,
    /// Interpolation segment: `${expr}`
    INTERPOLATION,
    /// Trailing closure: `do |params| ... end` after a call
    TRAILING_CLOSURE,
    /// Tuple expression: `(a, b, c)`
    TUPLE_EXPR,
    /// Type parameter list: `[A, B]`
    TYPE_PARAM_LIST,
    /// Import list: `import a, b, c` items
    IMPORT_LIST,
    /// Struct literal: `Point { x: 1, y: 2 }`
    STRUCT_LITERAL,
    /// Single field in a struct literal.
    STRUCT_LITERAL_FIELD,
    /// Interface definition: `interface Printable do ... end`
    INTERFACE_DEF,
    /// Method signature in an interface definition.
    INTERFACE_METHOD,
    /// Impl block: `impl Printable for Int do ... end`
    IMPL_DEF,
    /// Associated type declaration inside an interface: `type Item`
    ASSOC_TYPE_DEF,
    /// Associated type binding inside an impl: `type Item = Int`
    ASSOC_TYPE_BINDING,
    /// Type alias: `type Name = ExistingType`
    TYPE_ALIAS_DEF,
    /// Where clause: `where T: Trait`
    WHERE_CLAUSE,
    /// Trait bound: `T: TraitName`
    TRAIT_BOUND,
    /// Generic parameter list: `<A, B>`
    GENERIC_PARAM_LIST,
    /// Generic argument list in type application: `<Int, String>`
    GENERIC_ARG_LIST,
    /// Option type sugar: `Int?` => `Option<Int>`
    OPTION_TYPE,
    /// Result type sugar: `T!E` => `Result<T, E>`
    RESULT_TYPE,
    /// Function type annotation: `Fun(Int, String) -> Bool`
    FUN_TYPE,

    // ── Sum type / ADT node kinds ────────────────────────────────────────
    /// Sum type definition: `type Shape do ... end`
    SUM_TYPE_DEF,
    /// Variant definition inside a sum type: `Circle(Float)` or `Rectangle(width: Float, height: Float)`
    VARIANT_DEF,
    /// Named field inside a variant: `width: Float`
    VARIANT_FIELD,
    /// Constructor pattern: `Shape.Circle(r)` or `Some(x)` in patterns
    CONSTRUCTOR_PAT,
    /// Or-pattern: `Circle(_) | Point`
    OR_PAT,
    /// As-pattern: `Circle(_) as c`
    AS_PAT,
    /// Cons pattern: `head :: tail` for list destructuring
    CONS_PAT,
    /// Guard clause: `when r > 0.0`
    GUARD_CLAUSE,
    /// Deriving clause: `deriving(Eq, Display, ...)`
    DERIVING_CLAUSE,
    /// Expression body for `fn name(pattern) = expr` form.
    FN_EXPR_BODY,
    /// A single clause in a multi-clause closure: `params [when guard] -> body`
    CLOSURE_CLAUSE,
    /// Map literal: `%{key => value, ...}`
    MAP_LITERAL,
    /// Single entry in a map literal: `key => value`
    MAP_ENTRY,
    /// Struct update expression: `%{base | field: value, ...}`
    STRUCT_UPDATE_EXPR,
    /// List literal: `[expr, expr, ...]`
    LIST_LITERAL,
    /// Relationship declaration in a struct body: `belongs_to :name, Type`
    RELATIONSHIP_DECL,
    /// Schema option declaration in a struct body: `table "name"`, `primary_key :id`, `timestamps true`
    SCHEMA_OPTION,

    // ── Loop node kinds ───────────────────────────────────────────────
    /// While expression: `while cond do body end`
    WHILE_EXPR,
    /// Break expression: `break`
    BREAK_EXPR,
    /// Continue expression: `continue`
    CONTINUE_EXPR,
    /// For-in expression: for binding in iterable do body end
    FOR_IN_EXPR,
    /// Destructuring binding in for-in: `{k, v}`
    DESTRUCTURE_BINDING,

    // ── Actor node kinds ──────────────────────────────────────────────
    /// Actor block declaration: `actor Name do ... end`
    ACTOR_DEF,
    /// Spawn expression: `spawn(func, args...)`
    SPAWN_EXPR,
    /// Send expression: `send(pid, message)`
    SEND_EXPR,
    /// Receive block expression: `receive do ... end`
    RECEIVE_EXPR,
    /// Individual arm within a receive block
    RECEIVE_ARM,
    /// Self expression: `self()` -- get own PID
    SELF_EXPR,
    /// Link expression: `link(pid)`
    LINK_EXPR,
    /// Timeout clause in receive: `after timeout -> body`
    AFTER_CLAUSE,
    /// Terminate callback clause in actor block: `terminate do ... end`
    TERMINATE_CLAUSE,

    // ── Error propagation node kinds ────────────────────────────────
    /// Try expression: `expr?` for Result/Option propagation
    TRY_EXPR,

    // ── Service node kinds ──────────────────────────────────────────
    /// Service block declaration: `service Name do ... end`
    SERVICE_DEF,
    /// Call handler in a service: `call Name(args) :: ReturnType do |state| ... end`
    CALL_HANDLER,
    /// Cast handler in a service: `cast Name(args) do |state| ... end`
    CAST_HANDLER,

    // ── Supervisor node kinds ────────────────────────────────────────
    /// Supervisor block declaration: `supervisor Name do ... end`
    SUPERVISOR_DEF,
    /// Child spec inside a supervisor: `child Name do ... end`
    CHILD_SPEC_DEF,
    /// Strategy clause in supervisor: `strategy: one_for_one`
    STRATEGY_CLAUSE,
    /// Restart limit in supervisor: `max_restarts: 3`
    RESTART_LIMIT,
    /// Seconds limit in supervisor: `max_seconds: 5`
    SECONDS_LIMIT,
}

impl SyntaxKind {
    /// Whether this kind represents trivia (tokens that don't affect parsing).
    ///
    /// Trivia tokens are whitespace, newlines, and comments. They are preserved
    /// in the CST but skipped by the parser's lookahead methods.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::WHITESPACE
                | SyntaxKind::NEWLINE
                | SyntaxKind::COMMENT
                | SyntaxKind::DOC_COMMENT
                | SyntaxKind::MODULE_DOC_COMMENT
        )
    }
}

impl From<TokenKind> for SyntaxKind {
    fn from(kind: TokenKind) -> Self {
        match kind {
            // Keywords
            TokenKind::Actor => SyntaxKind::ACTOR_KW,
            TokenKind::After => SyntaxKind::AFTER_KW,
            TokenKind::Alias => SyntaxKind::ALIAS_KW,
            TokenKind::And => SyntaxKind::AND_KW,
            TokenKind::Call => SyntaxKind::CALL_KW,
            TokenKind::Case => SyntaxKind::CASE_KW,
            TokenKind::Cast => SyntaxKind::CAST_KW,
            TokenKind::Cond => SyntaxKind::COND_KW,
            TokenKind::Def => SyntaxKind::DEF_KW,
            TokenKind::Do => SyntaxKind::DO_KW,
            TokenKind::Else => SyntaxKind::ELSE_KW,
            TokenKind::End => SyntaxKind::END_KW,
            TokenKind::False => SyntaxKind::FALSE_KW,
            TokenKind::Fn => SyntaxKind::FN_KW,
            TokenKind::For => SyntaxKind::FOR_KW,
            TokenKind::If => SyntaxKind::IF_KW,
            TokenKind::Impl => SyntaxKind::IMPL_KW,
            TokenKind::Import => SyntaxKind::IMPORT_KW,
            TokenKind::Interface => SyntaxKind::INTERFACE_KW,
            TokenKind::In => SyntaxKind::IN_KW,
            TokenKind::Let => SyntaxKind::LET_KW,
            TokenKind::Link => SyntaxKind::LINK_KW,
            TokenKind::Match => SyntaxKind::MATCH_KW,
            TokenKind::Module => SyntaxKind::MODULE_KW,
            TokenKind::Monitor => SyntaxKind::MONITOR_KW,
            TokenKind::Nil => SyntaxKind::NIL_KW,
            TokenKind::Not => SyntaxKind::NOT_KW,
            TokenKind::Or => SyntaxKind::OR_KW,
            TokenKind::Pub => SyntaxKind::PUB_KW,
            TokenKind::Receive => SyntaxKind::RECEIVE_KW,
            TokenKind::Return => SyntaxKind::RETURN_KW,
            TokenKind::SelfKw => SyntaxKind::SELF_KW,
            TokenKind::Send => SyntaxKind::SEND_KW,
            TokenKind::Service => SyntaxKind::SERVICE_KW,
            TokenKind::Spawn => SyntaxKind::SPAWN_KW,
            TokenKind::Struct => SyntaxKind::STRUCT_KW,
            TokenKind::Supervisor => SyntaxKind::SUPERVISOR_KW,
            TokenKind::Terminate => SyntaxKind::TERMINATE_KW,
            TokenKind::Trait => SyntaxKind::TRAIT_KW,
            TokenKind::Trap => SyntaxKind::TRAP_KW,
            TokenKind::True => SyntaxKind::TRUE_KW,
            TokenKind::Type => SyntaxKind::TYPE_KW,
            TokenKind::When => SyntaxKind::WHEN_KW,
            TokenKind::Where => SyntaxKind::WHERE_KW,
            TokenKind::While => SyntaxKind::WHILE_KW,
            TokenKind::With => SyntaxKind::WITH_KW,
            TokenKind::Break => SyntaxKind::BREAK_KW,
            TokenKind::Continue => SyntaxKind::CONTINUE_KW,
            // Operators
            TokenKind::Plus => SyntaxKind::PLUS,
            TokenKind::Minus => SyntaxKind::MINUS,
            TokenKind::Star => SyntaxKind::STAR,
            TokenKind::Slash => SyntaxKind::SLASH,
            TokenKind::Percent => SyntaxKind::PERCENT,
            TokenKind::EqEq => SyntaxKind::EQ_EQ,
            TokenKind::NotEq => SyntaxKind::NOT_EQ,
            TokenKind::Lt => SyntaxKind::LT,
            TokenKind::Gt => SyntaxKind::GT,
            TokenKind::LtEq => SyntaxKind::LT_EQ,
            TokenKind::GtEq => SyntaxKind::GT_EQ,
            TokenKind::AmpAmp => SyntaxKind::AMP_AMP,
            TokenKind::PipePipe => SyntaxKind::PIPE_PIPE,
            TokenKind::Bang => SyntaxKind::BANG,
            TokenKind::Pipe => SyntaxKind::PIPE,
            TokenKind::SlotPipe(_) => SyntaxKind::SLOT_PIPE,
            TokenKind::DotDot => SyntaxKind::DOT_DOT,
            TokenKind::Diamond => SyntaxKind::DIAMOND,
            TokenKind::PlusPlus => SyntaxKind::PLUS_PLUS,
            TokenKind::Eq => SyntaxKind::EQ,
            TokenKind::Arrow => SyntaxKind::ARROW,
            TokenKind::FatArrow => SyntaxKind::FAT_ARROW,
            TokenKind::ColonColon => SyntaxKind::COLON_COLON,
            TokenKind::Question => SyntaxKind::QUESTION,
            TokenKind::Bar => SyntaxKind::BAR,
            // Delimiters
            TokenKind::LParen => SyntaxKind::L_PAREN,
            TokenKind::RParen => SyntaxKind::R_PAREN,
            TokenKind::LBracket => SyntaxKind::L_BRACKET,
            TokenKind::RBracket => SyntaxKind::R_BRACKET,
            TokenKind::LBrace => SyntaxKind::L_BRACE,
            TokenKind::RBrace => SyntaxKind::R_BRACE,
            // Punctuation
            TokenKind::Comma => SyntaxKind::COMMA,
            TokenKind::Dot => SyntaxKind::DOT,
            TokenKind::Colon => SyntaxKind::COLON,
            TokenKind::Semicolon => SyntaxKind::SEMICOLON,
            TokenKind::Newline => SyntaxKind::NEWLINE,
            // Literals
            TokenKind::IntLiteral => SyntaxKind::INT_LITERAL,
            TokenKind::FloatLiteral => SyntaxKind::FLOAT_LITERAL,
            TokenKind::StringStart => SyntaxKind::STRING_START,
            TokenKind::StringEnd => SyntaxKind::STRING_END,
            TokenKind::StringContent => SyntaxKind::STRING_CONTENT,
            TokenKind::InterpolationStart => SyntaxKind::INTERPOLATION_START,
            TokenKind::InterpolationEnd => SyntaxKind::INTERPOLATION_END,
            TokenKind::Atom => SyntaxKind::ATOM_LITERAL,
            // Identifiers and comments
            TokenKind::Ident => SyntaxKind::IDENT,
            TokenKind::Comment => SyntaxKind::COMMENT,
            TokenKind::DocComment => SyntaxKind::DOC_COMMENT,
            TokenKind::ModuleDocComment => SyntaxKind::MODULE_DOC_COMMENT,
            // Special
            TokenKind::Eof => SyntaxKind::EOF,
            TokenKind::Error => SyntaxKind::ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_token_kinds_convert_to_syntax_kind() {
        // Exhaustive test: every TokenKind variant must convert without panic.
        let all_kinds = [
            // Keywords (48)
            TokenKind::Actor,
            TokenKind::After,
            TokenKind::Alias,
            TokenKind::And,
            TokenKind::Call,
            TokenKind::Case,
            TokenKind::Cast,
            TokenKind::Cond,
            TokenKind::Def,
            TokenKind::Do,
            TokenKind::Else,
            TokenKind::End,
            TokenKind::False,
            TokenKind::Fn,
            TokenKind::For,
            TokenKind::If,
            TokenKind::Impl,
            TokenKind::Import,
            TokenKind::Interface,
            TokenKind::In,
            TokenKind::Let,
            TokenKind::Link,
            TokenKind::Match,
            TokenKind::Module,
            TokenKind::Monitor,
            TokenKind::Nil,
            TokenKind::Not,
            TokenKind::Or,
            TokenKind::Pub,
            TokenKind::Receive,
            TokenKind::Return,
            TokenKind::SelfKw,
            TokenKind::Send,
            TokenKind::Service,
            TokenKind::Spawn,
            TokenKind::Struct,
            TokenKind::Supervisor,
            TokenKind::Terminate,
            TokenKind::Trait,
            TokenKind::Trap,
            TokenKind::True,
            TokenKind::Type,
            TokenKind::When,
            TokenKind::Where,
            TokenKind::While,
            TokenKind::With,
            TokenKind::Break,
            TokenKind::Continue,
            // Operators (24)
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::EqEq,
            TokenKind::NotEq,
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
            TokenKind::AmpAmp,
            TokenKind::PipePipe,
            TokenKind::Bang,
            TokenKind::Pipe,
            TokenKind::SlotPipe(0),
            TokenKind::DotDot,
            TokenKind::Diamond,
            TokenKind::PlusPlus,
            TokenKind::Eq,
            TokenKind::Arrow,
            TokenKind::FatArrow,
            TokenKind::ColonColon,
            TokenKind::Question,
            TokenKind::Bar,
            // Delimiters (6)
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::LBrace,
            TokenKind::RBrace,
            // Punctuation (5)
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::Colon,
            TokenKind::Semicolon,
            TokenKind::Newline,
            // Literals (8)
            TokenKind::IntLiteral,
            TokenKind::FloatLiteral,
            TokenKind::StringStart,
            TokenKind::StringEnd,
            TokenKind::StringContent,
            TokenKind::InterpolationStart,
            TokenKind::InterpolationEnd,
            TokenKind::Atom,
            // Identifiers and comments (4)
            TokenKind::Ident,
            TokenKind::Comment,
            TokenKind::DocComment,
            TokenKind::ModuleDocComment,
            // Special (2)
            TokenKind::Eof,
            TokenKind::Error,
        ];

        assert_eq!(all_kinds.len(), 98, "must test all 98 TokenKind variants");

        for kind in all_kinds {
            let _syntax_kind: SyntaxKind = kind.into();
        }
    }

    #[test]
    fn trivia_kinds_identified() {
        assert!(SyntaxKind::WHITESPACE.is_trivia());
        assert!(SyntaxKind::NEWLINE.is_trivia());
        assert!(SyntaxKind::COMMENT.is_trivia());
        assert!(SyntaxKind::DOC_COMMENT.is_trivia());
        assert!(SyntaxKind::MODULE_DOC_COMMENT.is_trivia());

        assert!(!SyntaxKind::IDENT.is_trivia());
        assert!(!SyntaxKind::LET_KW.is_trivia());
        assert!(!SyntaxKind::PLUS.is_trivia());
        assert!(!SyntaxKind::SOURCE_FILE.is_trivia());
    }

    #[test]
    fn sentinel_kinds_are_first_values() {
        assert_eq!(SyntaxKind::TOMBSTONE as u16, 0);
        assert_eq!(SyntaxKind::ERROR_NODE as u16, 1);
    }

    #[test]
    fn syntax_kind_has_enough_variants() {
        // 2 sentinels + 90 token kinds + 1 WHITESPACE + 56 node kinds
        // Verify we have at least the expected count of composite node kinds.
        let node_kinds = [
            SyntaxKind::SOURCE_FILE,
            SyntaxKind::FN_DEF,
            SyntaxKind::LET_BINDING,
            SyntaxKind::RETURN_EXPR,
            SyntaxKind::IF_EXPR,
            SyntaxKind::ELSE_BRANCH,
            SyntaxKind::CASE_EXPR,
            SyntaxKind::MATCH_ARM,
            SyntaxKind::BINARY_EXPR,
            SyntaxKind::UNARY_EXPR,
            SyntaxKind::CALL_EXPR,
            SyntaxKind::PIPE_EXPR,
            SyntaxKind::SLOT_PIPE_EXPR,
            SyntaxKind::FIELD_ACCESS,
            SyntaxKind::INDEX_EXPR,
            SyntaxKind::BLOCK,
            SyntaxKind::PARAM_LIST,
            SyntaxKind::PARAM,
            SyntaxKind::ARG_LIST,
            SyntaxKind::MODULE_DEF,
            SyntaxKind::IMPORT_DECL,
            SyntaxKind::FROM_IMPORT_DECL,
            SyntaxKind::STRUCT_DEF,
            SyntaxKind::STRUCT_FIELD,
            SyntaxKind::CLOSURE_EXPR,
            SyntaxKind::LITERAL,
            SyntaxKind::ATOM_EXPR,
            SyntaxKind::NAME,
            SyntaxKind::NAME_REF,
            SyntaxKind::PATH,
            SyntaxKind::TYPE_ANNOTATION,
            SyntaxKind::VISIBILITY,
            SyntaxKind::WILDCARD_PAT,
            SyntaxKind::IDENT_PAT,
            SyntaxKind::LITERAL_PAT,
            SyntaxKind::TUPLE_PAT,
            SyntaxKind::STRUCT_PAT,
            SyntaxKind::STRING_EXPR,
            SyntaxKind::INTERPOLATION,
            SyntaxKind::TRAILING_CLOSURE,
            SyntaxKind::TUPLE_EXPR,
            SyntaxKind::TYPE_PARAM_LIST,
            SyntaxKind::IMPORT_LIST,
            SyntaxKind::STRUCT_LITERAL,
            SyntaxKind::STRUCT_LITERAL_FIELD,
            SyntaxKind::INTERFACE_DEF,
            SyntaxKind::INTERFACE_METHOD,
            SyntaxKind::IMPL_DEF,
            SyntaxKind::ASSOC_TYPE_DEF,
            SyntaxKind::ASSOC_TYPE_BINDING,
            SyntaxKind::TYPE_ALIAS_DEF,
            SyntaxKind::WHERE_CLAUSE,
            SyntaxKind::TRAIT_BOUND,
            SyntaxKind::GENERIC_PARAM_LIST,
            SyntaxKind::GENERIC_ARG_LIST,
            SyntaxKind::OPTION_TYPE,
            SyntaxKind::RESULT_TYPE,
            SyntaxKind::FUN_TYPE,
            SyntaxKind::SUM_TYPE_DEF,
            SyntaxKind::VARIANT_DEF,
            SyntaxKind::VARIANT_FIELD,
            SyntaxKind::CONSTRUCTOR_PAT,
            SyntaxKind::OR_PAT,
            SyntaxKind::AS_PAT,
            SyntaxKind::CONS_PAT,
            SyntaxKind::GUARD_CLAUSE,
            SyntaxKind::DERIVING_CLAUSE,
            SyntaxKind::FN_EXPR_BODY,
            SyntaxKind::CLOSURE_CLAUSE,
            SyntaxKind::MAP_LITERAL,
            SyntaxKind::MAP_ENTRY,
            SyntaxKind::STRUCT_UPDATE_EXPR,
            SyntaxKind::LIST_LITERAL,
            SyntaxKind::RELATIONSHIP_DECL,
            SyntaxKind::SCHEMA_OPTION,
            // Loop node kinds
            SyntaxKind::WHILE_EXPR,
            SyntaxKind::BREAK_EXPR,
            SyntaxKind::CONTINUE_EXPR,
            SyntaxKind::FOR_IN_EXPR,
            SyntaxKind::DESTRUCTURE_BINDING,
            // Actor node kinds
            SyntaxKind::ACTOR_DEF,
            SyntaxKind::SPAWN_EXPR,
            SyntaxKind::SEND_EXPR,
            SyntaxKind::RECEIVE_EXPR,
            SyntaxKind::RECEIVE_ARM,
            SyntaxKind::SELF_EXPR,
            SyntaxKind::LINK_EXPR,
            SyntaxKind::AFTER_CLAUSE,
            SyntaxKind::TERMINATE_CLAUSE,
            // Error propagation node kinds
            SyntaxKind::TRY_EXPR,
            // Service node kinds
            SyntaxKind::SERVICE_DEF,
            SyntaxKind::CALL_HANDLER,
            SyntaxKind::CAST_HANDLER,
            // Supervisor node kinds
            SyntaxKind::SUPERVISOR_DEF,
            SyntaxKind::CHILD_SPEC_DEF,
            SyntaxKind::STRATEGY_CLAUSE,
            SyntaxKind::RESTART_LIMIT,
            SyntaxKind::SECONDS_LIMIT,
        ];
        assert!(
            node_kinds.len() >= 88,
            "expected at least 88 composite node kinds, got {}",
            node_kinds.len()
        );
    }
}
