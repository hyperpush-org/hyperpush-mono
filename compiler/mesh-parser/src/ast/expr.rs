//! Typed AST nodes for expressions.
//!
//! Covers all expression forms: literals, name references, binary/unary
//! operators, function calls, pipe expressions, field access, index access,
//! if/else, case/match, closures, blocks, strings, return, and tuples.

use crate::ast::item::{Block, GuardClause, ParamList};
use crate::ast::{ast_node, child_node, child_nodes, child_token, AstNode};
use crate::cst::{SyntaxNode, SyntaxToken};
use crate::syntax_kind::SyntaxKind;

// ── Expr enum ────────────────────────────────────────────────────────────

/// Any expression node.
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    NameRef(NameRef),
    BinaryExpr(BinaryExpr),
    UnaryExpr(UnaryExpr),
    CallExpr(CallExpr),
    PipeExpr(PipeExpr),
    FieldAccess(FieldAccess),
    IndexExpr(IndexExpr),
    IfExpr(IfExpr),
    CaseExpr(CaseExpr),
    ClosureExpr(ClosureExpr),
    Block(Block),
    StringExpr(StringExpr),
    ReturnExpr(ReturnExpr),
    TupleExpr(TupleExpr),
    StructLiteral(StructLiteral),
    MapLiteral(MapLiteral),
    ListLiteral(ListLiteral),
    // Loop expression types
    WhileExpr(WhileExpr),
    BreakExpr(BreakExpr),
    ContinueExpr(ContinueExpr),
    ForInExpr(ForInExpr),
    // Actor expression types
    SpawnExpr(SpawnExpr),
    SendExpr(SendExpr),
    ReceiveExpr(ReceiveExpr),
    SelfExpr(SelfExpr),
    LinkExpr(LinkExpr),
    // Error propagation
    TryExpr(TryExpr),
    // Atom literal
    AtomLiteral(AtomLiteral),
    // Struct update expression
    StructUpdate(StructUpdate),
    // Slot pipe expression
    SlotPipeExpr(SlotPipeExpr),
    // Regex literal expression: ~r/pattern/flags
    RegexExpr(RegexExpr),
    // Json object literal expression: json { key: value, ... }
    JsonExpr(JsonExpr),
}

impl Expr {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::LITERAL => Some(Expr::Literal(Literal { syntax: node })),
            SyntaxKind::NAME_REF => Some(Expr::NameRef(NameRef { syntax: node })),
            SyntaxKind::BINARY_EXPR => Some(Expr::BinaryExpr(BinaryExpr { syntax: node })),
            SyntaxKind::UNARY_EXPR => Some(Expr::UnaryExpr(UnaryExpr { syntax: node })),
            SyntaxKind::CALL_EXPR => Some(Expr::CallExpr(CallExpr { syntax: node })),
            SyntaxKind::PIPE_EXPR => Some(Expr::PipeExpr(PipeExpr { syntax: node })),
            SyntaxKind::FIELD_ACCESS => Some(Expr::FieldAccess(FieldAccess { syntax: node })),
            SyntaxKind::INDEX_EXPR => Some(Expr::IndexExpr(IndexExpr { syntax: node })),
            SyntaxKind::IF_EXPR => Some(Expr::IfExpr(IfExpr { syntax: node })),
            SyntaxKind::CASE_EXPR => Some(Expr::CaseExpr(CaseExpr { syntax: node })),
            SyntaxKind::CLOSURE_EXPR => Some(Expr::ClosureExpr(ClosureExpr { syntax: node })),
            SyntaxKind::BLOCK => Some(Expr::Block(Block { syntax: node })),
            SyntaxKind::STRING_EXPR => Some(Expr::StringExpr(StringExpr { syntax: node })),
            SyntaxKind::RETURN_EXPR => Some(Expr::ReturnExpr(ReturnExpr { syntax: node })),
            SyntaxKind::TUPLE_EXPR => Some(Expr::TupleExpr(TupleExpr { syntax: node })),
            SyntaxKind::STRUCT_LITERAL => {
                Some(Expr::StructLiteral(StructLiteral { syntax: node }))
            }
            SyntaxKind::MAP_LITERAL => {
                Some(Expr::MapLiteral(MapLiteral { syntax: node }))
            }
            SyntaxKind::LIST_LITERAL => {
                Some(Expr::ListLiteral(ListLiteral { syntax: node }))
            }
            // Loop expressions
            SyntaxKind::WHILE_EXPR => Some(Expr::WhileExpr(WhileExpr { syntax: node })),
            SyntaxKind::BREAK_EXPR => Some(Expr::BreakExpr(BreakExpr { syntax: node })),
            SyntaxKind::CONTINUE_EXPR => {
                Some(Expr::ContinueExpr(ContinueExpr { syntax: node }))
            }
            SyntaxKind::FOR_IN_EXPR => Some(Expr::ForInExpr(ForInExpr { syntax: node })),
            // Actor expressions
            SyntaxKind::SPAWN_EXPR => Some(Expr::SpawnExpr(SpawnExpr { syntax: node })),
            SyntaxKind::SEND_EXPR => Some(Expr::SendExpr(SendExpr { syntax: node })),
            SyntaxKind::RECEIVE_EXPR => Some(Expr::ReceiveExpr(ReceiveExpr { syntax: node })),
            SyntaxKind::SELF_EXPR => Some(Expr::SelfExpr(SelfExpr { syntax: node })),
            SyntaxKind::LINK_EXPR => Some(Expr::LinkExpr(LinkExpr { syntax: node })),
            SyntaxKind::TRY_EXPR => Some(Expr::TryExpr(TryExpr { syntax: node })),
            SyntaxKind::ATOM_EXPR => Some(Expr::AtomLiteral(AtomLiteral { syntax: node })),
            SyntaxKind::STRUCT_UPDATE_EXPR => {
                Some(Expr::StructUpdate(StructUpdate { syntax: node }))
            }
            SyntaxKind::SLOT_PIPE_EXPR => {
                Some(Expr::SlotPipeExpr(SlotPipeExpr { syntax: node }))
            }
            SyntaxKind::REGEX_EXPR => Some(Expr::RegexExpr(RegexExpr { syntax: node })),
            SyntaxKind::JSON_EXPR => Some(Expr::JsonExpr(JsonExpr { syntax: node })),
            _ => None,
        }
    }

    /// Access the underlying syntax node regardless of variant.
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Expr::Literal(n) => &n.syntax,
            Expr::NameRef(n) => &n.syntax,
            Expr::BinaryExpr(n) => &n.syntax,
            Expr::UnaryExpr(n) => &n.syntax,
            Expr::CallExpr(n) => &n.syntax,
            Expr::PipeExpr(n) => &n.syntax,
            Expr::FieldAccess(n) => &n.syntax,
            Expr::IndexExpr(n) => &n.syntax,
            Expr::IfExpr(n) => &n.syntax,
            Expr::CaseExpr(n) => &n.syntax,
            Expr::ClosureExpr(n) => &n.syntax,
            Expr::Block(n) => AstNode::syntax(n),
            Expr::StringExpr(n) => &n.syntax,
            Expr::ReturnExpr(n) => &n.syntax,
            Expr::TupleExpr(n) => &n.syntax,
            Expr::StructLiteral(n) => &n.syntax,
            Expr::MapLiteral(n) => &n.syntax,
            Expr::ListLiteral(n) => &n.syntax,
            Expr::WhileExpr(n) => &n.syntax,
            Expr::BreakExpr(n) => &n.syntax,
            Expr::ContinueExpr(n) => &n.syntax,
            Expr::ForInExpr(n) => &n.syntax,
            Expr::SpawnExpr(n) => &n.syntax,
            Expr::SendExpr(n) => &n.syntax,
            Expr::ReceiveExpr(n) => &n.syntax,
            Expr::SelfExpr(n) => &n.syntax,
            Expr::LinkExpr(n) => &n.syntax,
            Expr::TryExpr(n) => &n.syntax,
            Expr::AtomLiteral(n) => &n.syntax,
            Expr::StructUpdate(n) => &n.syntax,
            Expr::SlotPipeExpr(n) => &n.syntax,
            Expr::RegexExpr(n) => &n.syntax,
            Expr::JsonExpr(n) => &n.syntax,
        }
    }
}

// ── Literal ──────────────────────────────────────────────────────────────

ast_node!(Literal, LITERAL);

impl Literal {
    /// The literal token (INT_LITERAL, FLOAT_LITERAL, TRUE_KW, FALSE_KW, NIL_KW).
    pub fn token(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .next()
    }
}

// ── Name Reference ───────────────────────────────────────────────────────

ast_node!(NameRef, NAME_REF);

impl NameRef {
    /// The identifier text (also handles `self` keyword used as method receiver).
    pub fn text(&self) -> Option<String> {
        child_token(&self.syntax, SyntaxKind::IDENT)
            .or_else(|| child_token(&self.syntax, SyntaxKind::SELF_KW))
            .map(|t| t.text().to_string())
    }
}

// ── Binary Expression ────────────────────────────────────────────────────

ast_node!(BinaryExpr, BINARY_EXPR);

impl BinaryExpr {
    /// The left-hand side expression.
    pub fn lhs(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The right-hand side expression.
    pub fn rhs(&self) -> Option<Expr> {
        self.syntax.children().filter_map(Expr::cast).nth(1)
    }

    /// The operator token.
    pub fn op(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::PLUS
                        | SyntaxKind::MINUS
                        | SyntaxKind::STAR
                        | SyntaxKind::SLASH
                        | SyntaxKind::PERCENT
                        | SyntaxKind::EQ_EQ
                        | SyntaxKind::NOT_EQ
                        | SyntaxKind::LT
                        | SyntaxKind::GT
                        | SyntaxKind::LT_EQ
                        | SyntaxKind::GT_EQ
                        | SyntaxKind::AND_KW
                        | SyntaxKind::OR_KW
                        | SyntaxKind::AMP_AMP
                        | SyntaxKind::PIPE_PIPE
                        | SyntaxKind::DOT_DOT
                        | SyntaxKind::DIAMOND
                        | SyntaxKind::PLUS_PLUS
                )
            })
    }
}

// ── Unary Expression ─────────────────────────────────────────────────────

ast_node!(UnaryExpr, UNARY_EXPR);

impl UnaryExpr {
    /// The operator token.
    pub fn op(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::MINUS | SyntaxKind::BANG | SyntaxKind::NOT_KW
                )
            })
    }

    /// The operand expression.
    pub fn operand(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }
}

// ── Call Expression ──────────────────────────────────────────────────────

ast_node!(CallExpr, CALL_EXPR);

impl CallExpr {
    /// The callee expression (function being called).
    pub fn callee(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The argument list.
    pub fn arg_list(&self) -> Option<ArgList> {
        child_node(&self.syntax)
    }
}

ast_node!(ArgList, ARG_LIST);

impl ArgList {
    /// All argument expressions.
    pub fn args(&self) -> impl Iterator<Item = Expr> + '_ {
        self.syntax.children().filter_map(Expr::cast)
    }
}

// ── Pipe Expression ──────────────────────────────────────────────────────

ast_node!(PipeExpr, PIPE_EXPR);

impl PipeExpr {
    /// The left-hand side (input to the pipe).
    pub fn lhs(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The right-hand side (function receiving the piped value).
    pub fn rhs(&self) -> Option<Expr> {
        self.syntax.children().filter_map(Expr::cast).nth(1)
    }
}

// ── Slot Pipe Expression ─────────────────────────────────────────────────

ast_node!(SlotPipeExpr, SLOT_PIPE_EXPR);

impl SlotPipeExpr {
    /// The left-hand side (value being piped).
    pub fn lhs(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The slot position N (1-indexed) from the `|N>` token.
    pub fn slot(&self) -> Option<u32> {
        // Find the SLOT_PIPE token and extract N from text like "|2>"
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|t| t.kind() == SyntaxKind::SLOT_PIPE)
            .and_then(|t| {
                let text = t.text();
                // text is "|N>" -- strip '|' prefix and '>' suffix
                text.strip_prefix('|')
                    .and_then(|s| s.strip_suffix('>'))
                    .and_then(|n| n.parse::<u32>().ok())
            })
    }

    /// The right-hand side (function call receiving the piped value at slot N).
    pub fn rhs(&self) -> Option<Expr> {
        self.syntax.children().filter_map(Expr::cast).nth(1)
    }
}

// ── Field Access ─────────────────────────────────────────────────────────

ast_node!(FieldAccess, FIELD_ACCESS);

impl FieldAccess {
    /// The expression being accessed.
    pub fn base(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The field name token.
    pub fn field(&self) -> Option<SyntaxToken> {
        // The field is after the DOT token; find the last IDENT or keyword-as-field.
        // Keywords like `self` and `monitor` are valid as field names in
        // module-qualified access (e.g., Node.self, Process.monitor).
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .filter(|t| matches!(t.kind(), SyntaxKind::IDENT | SyntaxKind::SELF_KW | SyntaxKind::MONITOR_KW | SyntaxKind::SPAWN_KW | SyntaxKind::LINK_KW | SyntaxKind::SEND_KW | SyntaxKind::WHERE_KW | SyntaxKind::CAST_KW))
            .last()
    }
}

// ── Index Expression ─────────────────────────────────────────────────────

ast_node!(IndexExpr, INDEX_EXPR);

impl IndexExpr {
    /// The expression being indexed.
    pub fn base(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The index expression (inside brackets).
    pub fn index(&self) -> Option<Expr> {
        self.syntax.children().filter_map(Expr::cast).nth(1)
    }
}

// ── If Expression ────────────────────────────────────────────────────────

ast_node!(IfExpr, IF_EXPR);

impl IfExpr {
    /// The condition expression.
    pub fn condition(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The then-branch block.
    pub fn then_branch(&self) -> Option<Block> {
        child_node(&self.syntax)
    }

    /// The else branch, if present.
    pub fn else_branch(&self) -> Option<ElseBranch> {
        child_node(&self.syntax)
    }
}

ast_node!(ElseBranch, ELSE_BRANCH);

impl ElseBranch {
    /// The else block (for plain `else ... end`).
    pub fn block(&self) -> Option<Block> {
        child_node(&self.syntax)
    }

    /// The chained `if` expression (for `else if ...`).
    pub fn if_expr(&self) -> Option<IfExpr> {
        child_node(&self.syntax)
    }
}

// ── Case/Match Expression ────────────────────────────────────────────────

ast_node!(CaseExpr, CASE_EXPR);

impl CaseExpr {
    /// The scrutinee expression being matched.
    pub fn scrutinee(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The match arms.
    pub fn arms(&self) -> impl Iterator<Item = MatchArm> + '_ {
        child_nodes(&self.syntax)
    }
}

ast_node!(MatchArm, MATCH_ARM);

impl MatchArm {
    /// The pattern being matched.
    pub fn pattern(&self) -> Option<super::pat::Pattern> {
        self.syntax
            .children()
            .find_map(super::pat::Pattern::cast)
    }

    /// The guard expression (after `when`), if present.
    pub fn guard(&self) -> Option<Expr> {
        // The guard is the second expression child (after the pattern
        // and before the arrow). If there's a when guard, there will be
        // expression nodes between the pattern and the body.
        // We look for the WHEN_KW token, then the next Expr child.
        let has_when = self
            .syntax
            .children_with_tokens()
            .any(|it| it.kind() == SyntaxKind::WHEN_KW);
        if has_when {
            // First expr child is the guard, second is the body.
            self.syntax.children().filter_map(Expr::cast).next()
        } else {
            None
        }
    }

    /// The body expression (after `->`).
    pub fn body(&self) -> Option<Expr> {
        let has_when = self
            .syntax
            .children_with_tokens()
            .any(|it| it.kind() == SyntaxKind::WHEN_KW);
        if has_when {
            // With guard: second expr is body
            self.syntax.children().filter_map(Expr::cast).nth(1)
        } else {
            // Without guard: first expr is body
            self.syntax.children().filter_map(Expr::cast).next()
        }
    }
}

// ── Closure Expression ───────────────────────────────────────────────────

ast_node!(ClosureExpr, CLOSURE_EXPR);

impl ClosureExpr {
    /// The parameter list, if present.
    ///
    /// For single-clause closures, returns the PARAM_LIST child.
    /// For multi-clause closures, returns the first clause's PARAM_LIST
    /// (direct child of CLOSURE_EXPR, not inside a CLOSURE_CLAUSE).
    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The closure body block.
    ///
    /// Returns the BLOCK child for both arrow closures (`-> expr`) and
    /// do/end closures (`do ... end`).
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }

    /// The guard clause on the first/only clause, if present.
    pub fn guard(&self) -> Option<GuardClause> {
        child_node(&self.syntax)
    }

    /// Whether this is a multi-clause closure.
    ///
    /// Multi-clause closures have CLOSURE_CLAUSE children for the 2nd+ clauses.
    pub fn is_multi_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|c| c.kind() == SyntaxKind::CLOSURE_CLAUSE)
    }

    /// Returns additional clauses (2nd, 3rd, ...) for multi-clause closures.
    ///
    /// The first clause's data is stored as direct children of CLOSURE_EXPR
    /// (param_list, guard, body). Additional clauses are CLOSURE_CLAUSE children.
    pub fn clauses(&self) -> impl Iterator<Item = ClosureClause> + '_ {
        self.syntax.children().filter_map(ClosureClause::cast)
    }
}

// ── Closure Clause (multi-clause closures) ──────────────────────────────

ast_node!(ClosureClause, CLOSURE_CLAUSE);

impl ClosureClause {
    /// The parameter list for this clause.
    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The guard clause, if present.
    pub fn guard(&self) -> Option<GuardClause> {
        child_node(&self.syntax)
    }

    /// The body block.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }

    /// The body as an expression (first Expr child).
    pub fn body_expr(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }
}

// ── String Expression ────────────────────────────────────────────────────

ast_node!(StringExpr, STRING_EXPR);

// ── Return Expression ────────────────────────────────────────────────────

ast_node!(ReturnExpr, RETURN_EXPR);

impl ReturnExpr {
    /// The return value expression, if present.
    pub fn value(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }
}

// ── Tuple Expression ─────────────────────────────────────────────────────

ast_node!(TupleExpr, TUPLE_EXPR);

impl TupleExpr {
    /// The elements of the tuple.
    pub fn elements(&self) -> impl Iterator<Item = Expr> + '_ {
        self.syntax.children().filter_map(Expr::cast)
    }
}

// ── Struct Literal Expression ───────────────────────────────────────────

ast_node!(StructLiteral, STRUCT_LITERAL);

impl StructLiteral {
    /// The struct name (NAME_REF child).
    pub fn name_ref(&self) -> Option<NameRef> {
        child_node(&self.syntax)
    }

    /// The struct literal fields.
    pub fn fields(&self) -> impl Iterator<Item = StructLiteralField> + '_ {
        child_nodes(&self.syntax)
    }
}

ast_node!(StructLiteralField, STRUCT_LITERAL_FIELD);

impl StructLiteralField {
    /// The field name.
    pub fn name(&self) -> Option<super::item::Name> {
        child_node(&self.syntax)
    }

    /// The field value expression.
    pub fn value(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }
}

// ── Map Literal Expression ───────────────────────────────────────────────

ast_node!(MapLiteral, MAP_LITERAL);

impl MapLiteral {
    /// The map entries.
    pub fn entries(&self) -> impl Iterator<Item = MapEntry> + '_ {
        child_nodes(&self.syntax)
    }
}

ast_node!(MapEntry, MAP_ENTRY);

impl MapEntry {
    /// The key expression (first child expression).
    pub fn key(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The value expression (second child expression, after `=>` or `:`).
    pub fn value(&self) -> Option<Expr> {
        self.syntax.children().filter_map(Expr::cast).nth(1)
    }

    /// Whether this entry is a keyword argument entry (uses `:` not `=>`).
    ///
    /// Keyword args like `name: "Alice"` produce MAP_ENTRY nodes with a COLON
    /// separator instead of FAT_ARROW. The key is a NAME_REF whose text should
    /// be treated as a string literal key.
    pub fn is_keyword_entry(&self) -> bool {
        child_token(&self.syntax, SyntaxKind::COLON).is_some()
            && child_token(&self.syntax, SyntaxKind::FAT_ARROW).is_none()
    }

    /// For keyword entries, return the key name as a string.
    ///
    /// Returns the text of the NAME_REF key for keyword argument entries.
    pub fn keyword_key_text(&self) -> Option<String> {
        if !self.is_keyword_entry() {
            return None;
        }
        self.key().map(|k| k.syntax().text().to_string())
    }
}

// ── Struct Update Expression ─────────────────────────────────────────────

ast_node!(StructUpdate, STRUCT_UPDATE_EXPR);

impl StructUpdate {
    /// The base expression (the struct being updated) -- first child expression.
    pub fn base_expr(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The override fields (`name: value` pairs) -- reuses StructLiteralField AST nodes.
    pub fn override_fields(&self) -> Vec<StructLiteralField> {
        child_nodes(&self.syntax).collect()
    }
}

// ── List Literal Expression ──────────────────────────────────────────────

ast_node!(ListLiteral, LIST_LITERAL);

impl ListLiteral {
    /// The element expressions in the list literal.
    pub fn elements(&self) -> impl Iterator<Item = Expr> + '_ {
        self.syntax.children().filter_map(Expr::cast)
    }
}

// ── While Expression ─────────────────────────────────────────────────

ast_node!(WhileExpr, WHILE_EXPR);

impl WhileExpr {
    /// The condition expression.
    pub fn condition(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The loop body block.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }
}

// ── Break Expression ─────────────────────────────────────────────────

ast_node!(BreakExpr, BREAK_EXPR);

// ── Continue Expression ──────────────────────────────────────────────

ast_node!(ContinueExpr, CONTINUE_EXPR);

// ── For-In Expression ───────────────────────────────────────────────

ast_node!(ForInExpr, FOR_IN_EXPR);

impl ForInExpr {
    /// The loop variable name (NAME child) for simple bindings.
    pub fn binding_name(&self) -> Option<super::item::Name> {
        child_node(&self.syntax)
    }

    /// The destructuring binding ({k, v}) for map iteration.
    pub fn destructure_binding(&self) -> Option<DestructureBinding> {
        child_node(&self.syntax)
    }

    /// The iterable expression (e.g., 0..10).
    pub fn iterable(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The filter expression (after `when`), if present.
    pub fn filter(&self) -> Option<Expr> {
        let has_when = self
            .syntax
            .children_with_tokens()
            .any(|it| it.kind() == SyntaxKind::WHEN_KW);
        if has_when {
            // With `when`: first expr = iterable, second expr = filter
            self.syntax.children().filter_map(Expr::cast).nth(1)
        } else {
            None
        }
    }

    /// The loop body block.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }
}

// ── Destructure Binding ─────────────────────────────────────────────

ast_node!(DestructureBinding, DESTRUCTURE_BINDING);

impl DestructureBinding {
    /// The variable names inside the destructuring: `{k, v}` -> [k, v].
    pub fn names(&self) -> Vec<super::item::Name> {
        child_nodes(&self.syntax).collect()
    }
}

// ── Actor Expression Types ──────────────────────────────────────────────

ast_node!(SpawnExpr, SPAWN_EXPR);

impl SpawnExpr {
    /// The argument list (function reference + initial state args).
    pub fn arg_list(&self) -> Option<ArgList> {
        child_node(&self.syntax)
    }
}

ast_node!(SendExpr, SEND_EXPR);

impl SendExpr {
    /// The argument list (target pid + message).
    pub fn arg_list(&self) -> Option<ArgList> {
        child_node(&self.syntax)
    }
}

ast_node!(ReceiveExpr, RECEIVE_EXPR);

impl ReceiveExpr {
    /// The receive arms.
    pub fn arms(&self) -> impl Iterator<Item = ReceiveArm> + '_ {
        child_nodes(&self.syntax)
    }

    /// The optional after (timeout) clause.
    pub fn after_clause(&self) -> Option<AfterClause> {
        child_node(&self.syntax)
    }
}

ast_node!(ReceiveArm, RECEIVE_ARM);

impl ReceiveArm {
    /// The pattern being matched.
    pub fn pattern(&self) -> Option<super::pat::Pattern> {
        self.syntax
            .children()
            .find_map(super::pat::Pattern::cast)
    }

    /// The body expression (after `->`).
    pub fn body(&self) -> Option<Expr> {
        let has_when = self
            .syntax
            .children_with_tokens()
            .any(|it| it.kind() == SyntaxKind::WHEN_KW);
        if has_when {
            self.syntax.children().filter_map(Expr::cast).nth(1)
        } else {
            self.syntax.children().filter_map(Expr::cast).next()
        }
    }
}

ast_node!(AfterClause, AFTER_CLAUSE);

impl AfterClause {
    /// The timeout expression.
    pub fn timeout(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    /// The timeout body expression.
    pub fn body(&self) -> Option<Expr> {
        self.syntax.children().filter_map(Expr::cast).nth(1)
    }
}

ast_node!(SelfExpr, SELF_EXPR);

ast_node!(LinkExpr, LINK_EXPR);

impl LinkExpr {
    /// The argument list (target pid).
    pub fn arg_list(&self) -> Option<ArgList> {
        child_node(&self.syntax)
    }
}

// ── Try Expression ──────────────────────────────────────────────────────

ast_node!(TryExpr, TRY_EXPR);

impl TryExpr {
    /// The operand expression (the expression before `?`).
    pub fn operand(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }
}

// ── Atom Literal Expression ─────────────────────────────────────────────

ast_node!(AtomLiteral, ATOM_EXPR);

impl AtomLiteral {
    /// Extract the atom name (without the leading `:`).
    ///
    /// For `:name`, returns `Some("name")`.
    pub fn atom_text(&self) -> Option<String> {
        child_token(&self.syntax, SyntaxKind::ATOM_LITERAL).map(|t| {
            let text = t.text().to_string();
            // Strip leading ':'
            text.strip_prefix(':').unwrap_or(&text).to_string()
        })
    }
}

// ── Regex Literal Expression ─────────────────────────────────────────────

ast_node!(RegexExpr, REGEX_EXPR);

impl RegexExpr {
    /// Returns the regex pattern string (content between `/` delimiters).
    ///
    /// Parses the source token text `~r/pattern/flags` by scanning for the
    /// first unescaped `/` after the `~r/` prefix.
    pub fn pattern(&self) -> Option<String> {
        child_token(&self.syntax, SyntaxKind::REGEX_LITERAL).map(|t| {
            let text = t.text();
            extract_regex_pattern(text)
        })
    }

    /// Returns the flags string (e.g. "ims", "i", "").
    ///
    /// Parses the source token text `~r/pattern/flags` to extract everything
    /// after the closing `/`.
    pub fn flags(&self) -> String {
        child_token(&self.syntax, SyntaxKind::REGEX_LITERAL)
            .map(|t| {
                let text = t.text();
                extract_regex_flags(text)
            })
            .unwrap_or_default()
    }
}

/// Extract the pattern from a regex literal source text like `~r/pattern/flags`.
///
/// Scans char-by-char after `~r/`, tracking backslash escapes.
/// The first unescaped `/` ends the pattern.
fn extract_regex_pattern(text: &str) -> String {
    let Some(rest) = text.strip_prefix("~r/") else {
        return String::new();
    };
    let mut pattern = String::new();
    let mut prev_was_backslash = false;
    for c in rest.chars() {
        if c == '/' && !prev_was_backslash {
            break;
        }
        pattern.push(c);
        if c == '\\' && !prev_was_backslash {
            prev_was_backslash = true;
        } else {
            prev_was_backslash = false;
        }
    }
    pattern
}

/// Extract the flags from a regex literal source text like `~r/pattern/flags`.
///
/// Finds the closing unescaped `/` and returns everything after it.
fn extract_regex_flags(text: &str) -> String {
    let Some(rest) = text.strip_prefix("~r/") else {
        return String::new();
    };
    let mut prev_was_backslash = false;
    let mut slash_pos = None;
    for (i, c) in rest.char_indices() {
        if c == '/' && !prev_was_backslash {
            slash_pos = Some(i);
            break;
        }
        if c == '\\' && !prev_was_backslash {
            prev_was_backslash = true;
        } else {
            prev_was_backslash = false;
        }
    }
    match slash_pos {
        Some(pos) => rest[pos + 1..].to_string(),
        None => String::new(),
    }
}

// ── Json Literal Expression ──────────────────────────────────────────────

ast_node!(JsonExpr, JSON_EXPR);

impl JsonExpr {
    /// Iterate over the fields of this json literal.
    pub fn fields(&self) -> impl Iterator<Item = JsonField> + '_ {
        self.syntax.children().filter_map(JsonField::cast)
    }
}

/// A single field in a json literal: `key: value`
#[derive(Debug, Clone)]
pub struct JsonField {
    pub(crate) syntax: crate::cst::SyntaxNode,
}

impl JsonField {
    /// Key name (bare identifier, e.g. `status` in `json { status: "ok" }`)
    pub fn key_text(&self) -> Option<String> {
        child_token(&self.syntax, SyntaxKind::IDENT).map(|t| t.text().to_string())
    }

    /// Value expression after the colon.
    pub fn value(&self) -> Option<Expr> {
        self.syntax.children().find_map(Expr::cast)
    }

    pub fn cast(node: crate::cst::SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::JSON_FIELD {
            Some(JsonField { syntax: node })
        } else {
            None
        }
    }
}
