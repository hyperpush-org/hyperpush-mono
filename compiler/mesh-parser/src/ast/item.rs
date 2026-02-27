//! Typed AST nodes for declarations and items.
//!
//! Covers: SourceFile, FnDef, ParamList, Param, TypeAnnotation, ModuleDef,
//! ImportDecl, FromImportDecl, ImportList, StructDef, StructField, LetBinding,
//! Visibility, Block, Name, NameRef, Path, SumTypeDef, VariantDef, VariantField.

use crate::ast::{ast_node, child_node, child_nodes, child_token, AstNode};
use crate::cst::{SyntaxNode, SyntaxToken};
use crate::syntax_kind::SyntaxKind;

// ── Source File ──────────────────────────────────────────────────────────

ast_node!(SourceFile, SOURCE_FILE);

impl SourceFile {
    /// All top-level items in the source file.
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.syntax.children().filter_map(Item::cast)
    }

    /// All top-level function definitions.
    pub fn fn_defs(&self) -> impl Iterator<Item = FnDef> + '_ {
        child_nodes(&self.syntax)
    }

    /// All top-level module definitions.
    pub fn modules(&self) -> impl Iterator<Item = ModuleDef> + '_ {
        child_nodes(&self.syntax)
    }
}

// ── Item enum ────────────────────────────────────────────────────────────

/// Any top-level or nested item declaration.
#[derive(Debug, Clone)]
pub enum Item {
    FnDef(FnDef),
    ModuleDef(ModuleDef),
    ImportDecl(ImportDecl),
    FromImportDecl(FromImportDecl),
    StructDef(StructDef),
    LetBinding(LetBinding),
    InterfaceDef(InterfaceDef),
    ImplDef(ImplDef),
    TypeAliasDef(TypeAliasDef),
    SumTypeDef(SumTypeDef),
    ActorDef(ActorDef),
    ServiceDef(ServiceDef),
    SupervisorDef(SupervisorDef),
}

impl Item {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::FN_DEF => Some(Item::FnDef(FnDef { syntax: node })),
            SyntaxKind::MODULE_DEF => Some(Item::ModuleDef(ModuleDef { syntax: node })),
            SyntaxKind::IMPORT_DECL => Some(Item::ImportDecl(ImportDecl { syntax: node })),
            SyntaxKind::FROM_IMPORT_DECL => {
                Some(Item::FromImportDecl(FromImportDecl { syntax: node }))
            }
            SyntaxKind::STRUCT_DEF => Some(Item::StructDef(StructDef { syntax: node })),
            SyntaxKind::LET_BINDING => Some(Item::LetBinding(LetBinding { syntax: node })),
            SyntaxKind::INTERFACE_DEF => {
                Some(Item::InterfaceDef(InterfaceDef { syntax: node }))
            }
            SyntaxKind::IMPL_DEF => Some(Item::ImplDef(ImplDef { syntax: node })),
            SyntaxKind::TYPE_ALIAS_DEF => {
                Some(Item::TypeAliasDef(TypeAliasDef { syntax: node }))
            }
            SyntaxKind::SUM_TYPE_DEF => {
                Some(Item::SumTypeDef(SumTypeDef { syntax: node }))
            }
            SyntaxKind::ACTOR_DEF => Some(Item::ActorDef(ActorDef { syntax: node })),
            SyntaxKind::SERVICE_DEF => {
                Some(Item::ServiceDef(ServiceDef { syntax: node }))
            }
            SyntaxKind::SUPERVISOR_DEF => {
                Some(Item::SupervisorDef(SupervisorDef { syntax: node }))
            }
            _ => None,
        }
    }
}

// ── Function Definition ──────────────────────────────────────────────────

ast_node!(FnDef, FN_DEF);

impl FnDef {
    /// The visibility modifier (`pub`), if present.
    pub fn visibility(&self) -> Option<Visibility> {
        child_node(&self.syntax)
    }

    /// The function name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The parameter list.
    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The return type annotation (`-> Type`), if present.
    pub fn return_type(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }

    /// The function body block (for `do ... end` form).
    ///
    /// Returns `None` for `= expr` form functions -- use `expr_body()` instead.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }

    /// The guard clause (`when expr`), if present.
    ///
    /// Used in multi-clause function definitions: `fn abs(n) when n < 0 = -n`
    pub fn guard(&self) -> Option<GuardClause> {
        child_node(&self.syntax)
    }

    /// The expression body for `= expr` form functions.
    ///
    /// Returns the expression from the FN_EXPR_BODY node. Returns `None` for
    /// `do ... end` form functions.
    pub fn expr_body(&self) -> Option<super::expr::Expr> {
        self.syntax
            .children()
            .find(|n| n.kind() == SyntaxKind::FN_EXPR_BODY)
            .and_then(|body_node| body_node.children().find_map(super::expr::Expr::cast))
    }

    /// Whether this function definition uses the `= expr` body form.
    ///
    /// Returns `true` for `fn fib(0) = 0`, `false` for `fn foo(x) do ... end`.
    /// Useful for the type checker to detect multi-clause function candidates.
    pub fn has_eq_body(&self) -> bool {
        self.syntax
            .children()
            .any(|n| n.kind() == SyntaxKind::FN_EXPR_BODY)
    }
}

// ── Parameter List ───────────────────────────────────────────────────────

ast_node!(ParamList, PARAM_LIST);

impl ParamList {
    /// All parameters in the list.
    pub fn params(&self) -> impl Iterator<Item = Param> + '_ {
        child_nodes(&self.syntax)
    }
}

// ── Parameter ────────────────────────────────────────────────────────────

ast_node!(Param, PARAM);

impl Param {
    /// The parameter name token (IDENT).
    ///
    /// For regular parameters like `fn foo(x)`, returns the IDENT token.
    /// For pattern parameters like `fn foo(0)`, returns `None` -- use `pattern()` instead.
    pub fn name(&self) -> Option<SyntaxToken> {
        child_token(&self.syntax, SyntaxKind::IDENT)
    }

    /// The type annotation, if present.
    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }

    /// The pattern child, if this parameter contains a pattern instead of a plain IDENT.
    ///
    /// Returns `Some(pattern)` for literal, wildcard, constructor, or tuple patterns
    /// (e.g., `fn fib(0)`, `fn foo(_)`, `fn bar(Some(x))`).
    /// Returns `None` for regular named parameters (e.g., `fn foo(x)`).
    pub fn pattern(&self) -> Option<super::pat::Pattern> {
        self.syntax
            .children()
            .find_map(super::pat::Pattern::cast)
    }
}

// ── Type Annotation ──────────────────────────────────────────────────────

ast_node!(TypeAnnotation, TYPE_ANNOTATION);

impl TypeAnnotation {
    /// The type name token(s). Returns the first IDENT in the annotation.
    pub fn type_name(&self) -> Option<SyntaxToken> {
        child_token(&self.syntax, SyntaxKind::IDENT)
    }
}

// ── Module Definition ────────────────────────────────────────────────────

ast_node!(ModuleDef, MODULE_DEF);

impl ModuleDef {
    /// The visibility modifier, if present.
    pub fn visibility(&self) -> Option<Visibility> {
        child_node(&self.syntax)
    }

    /// The module name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The items in the module body.
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        // Items are inside the BLOCK child.
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children().filter_map(Item::cast))
    }
}

// ── Import Declarations ──────────────────────────────────────────────────

ast_node!(ImportDecl, IMPORT_DECL);

impl ImportDecl {
    /// The module path being imported.
    pub fn module_path(&self) -> Option<Path> {
        child_node(&self.syntax)
    }
}

ast_node!(FromImportDecl, FROM_IMPORT_DECL);

impl FromImportDecl {
    /// The module path being imported from.
    pub fn module_path(&self) -> Option<Path> {
        child_node(&self.syntax)
    }

    /// The list of imported names.
    pub fn import_list(&self) -> Option<ImportList> {
        child_node(&self.syntax)
    }
}

ast_node!(ImportList, IMPORT_LIST);

impl ImportList {
    /// All imported names.
    pub fn names(&self) -> impl Iterator<Item = Name> + '_ {
        child_nodes(&self.syntax)
    }
}

// ── Struct Definition ────────────────────────────────────────────────────

ast_node!(StructDef, STRUCT_DEF);

impl StructDef {
    /// The visibility modifier, if present.
    pub fn visibility(&self) -> Option<Visibility> {
        child_node(&self.syntax)
    }

    /// The struct name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The struct fields.
    pub fn fields(&self) -> impl Iterator<Item = StructField> + '_ {
        child_nodes(&self.syntax)
    }

    /// Whether this struct has an explicit `deriving(...)` clause.
    ///
    /// When `true`, only the traits listed in the clause are derived.
    /// When `false`, all default traits (Debug, Eq, Ord, Hash) are derived
    /// for backward compatibility.
    pub fn has_deriving_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|n| n.kind() == SyntaxKind::DERIVING_CLAUSE)
    }

    /// Returns the list of trait names from `deriving(Eq, Display, ...)`.
    ///
    /// Returns an empty Vec if no DERIVING_CLAUSE is present.
    /// A present but empty `deriving()` also returns an empty Vec
    /// (use `has_deriving_clause()` to distinguish).
    pub fn deriving_traits(&self) -> Vec<String> {
        self.syntax
            .children()
            .find(|n| n.kind() == SyntaxKind::DERIVING_CLAUSE)
            .map(|dc| {
                dc.children_with_tokens()
                    .filter_map(|it| it.into_token())
                    .filter(|t| t.kind() == SyntaxKind::IDENT && t.text() != "deriving")
                    .map(|t| t.text().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// All relationship declarations in the struct body.
    pub fn relationships(&self) -> Vec<RelationshipDecl> {
        self.syntax
            .children()
            .filter_map(RelationshipDecl::cast)
            .collect()
    }

    /// All schema option declarations in the struct body.
    pub fn schema_options(&self) -> Vec<SchemaOption> {
        self.syntax
            .children()
            .filter_map(SchemaOption::cast)
            .collect()
    }
}

ast_node!(StructField, STRUCT_FIELD);

impl StructField {
    /// The field name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The type annotation.
    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }
}

// ── Relationship Declaration ─────────────────────────────────────────────

ast_node!(RelationshipDecl, RELATIONSHIP_DECL);

impl RelationshipDecl {
    /// The relationship kind: "belongs_to", "has_many", or "has_one".
    pub fn kind_text(&self) -> Option<String> {
        // First IDENT child is the relationship kind.
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
    }

    /// The association name (from the atom literal, e.g., "user" from `:user`).
    pub fn assoc_name(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| t.kind() == SyntaxKind::ATOM_LITERAL)
            .map(|t| {
                let text = t.text().to_string();
                // Strip leading colon from atom literal.
                if text.starts_with(':') {
                    text[1..].to_string()
                } else {
                    text
                }
            })
    }

    /// The target type name (e.g., "User" from `belongs_to :user, User`).
    pub fn target_type(&self) -> Option<String> {
        // The target type is the IDENT after the COMMA token.
        let mut after_comma = false;
        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.as_token() {
                if token.kind() == SyntaxKind::COMMA {
                    after_comma = true;
                } else if after_comma && token.kind() == SyntaxKind::IDENT {
                    return Some(token.text().to_string());
                }
            }
        }
        None
    }
}

// ── Schema Option ────────────────────────────────────────────────────────

ast_node!(SchemaOption, SCHEMA_OPTION);

impl SchemaOption {
    /// The option name: "table", "primary_key", or "timestamps".
    pub fn option_name(&self) -> Option<String> {
        // First IDENT child is the option name.
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
    }

    /// The string value (for `table` option), with quotes stripped.
    /// Returns the STRING_CONTENT text if present.
    pub fn string_value(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| t.kind() == SyntaxKind::STRING_CONTENT)
            .map(|t| t.text().to_string())
    }

    /// The atom value (for `primary_key` option), with leading colon stripped.
    pub fn atom_value(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| t.kind() == SyntaxKind::ATOM_LITERAL)
            .map(|t| {
                let text = t.text().to_string();
                if text.starts_with(':') {
                    text[1..].to_string()
                } else {
                    text
                }
            })
    }

    /// The boolean value (for `timestamps` option).
    pub fn bool_value(&self) -> Option<bool> {
        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.as_token() {
                match token.kind() {
                    SyntaxKind::TRUE_KW => return Some(true),
                    SyntaxKind::FALSE_KW => return Some(false),
                    SyntaxKind::IDENT if token.text() == "true" => return Some(true),
                    SyntaxKind::IDENT if token.text() == "false" => return Some(false),
                    _ => {}
                }
            }
        }
        None
    }
}

// ── Let Binding ──────────────────────────────────────────────────────────

ast_node!(LetBinding, LET_BINDING);

impl LetBinding {
    /// The binding name (for simple `let x = ...`).
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The pattern, if the binding uses destructuring.
    pub fn pattern(&self) -> Option<super::pat::Pattern> {
        self.syntax
            .children()
            .find_map(super::pat::Pattern::cast)
    }

    /// The type annotation, if present.
    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }

    /// The initializer expression.
    pub fn initializer(&self) -> Option<super::expr::Expr> {
        // The initializer is the expression child after the = token.
        // We find the first expression-like child node.
        self.syntax
            .children()
            .find_map(super::expr::Expr::cast)
    }
}

// ── Visibility ───────────────────────────────────────────────────────────

ast_node!(Visibility, VISIBILITY);

impl Visibility {
    /// The `pub` keyword token.
    pub fn pub_kw(&self) -> Option<SyntaxToken> {
        child_token(&self.syntax, SyntaxKind::PUB_KW)
    }
}

// ── Block ────────────────────────────────────────────────────────────────

ast_node!(Block, BLOCK);

impl Block {
    /// Statements and expressions in the block.
    pub fn stmts(&self) -> impl Iterator<Item = Item> + '_ {
        self.syntax.children().filter_map(Item::cast)
    }

    /// The tail expression (last expression that is the block's value).
    /// This is the last child that can be cast to an Expr.
    pub fn tail_expr(&self) -> Option<super::expr::Expr> {
        self.syntax
            .children()
            .filter_map(super::expr::Expr::cast)
            .last()
    }
}

// ── Name and NameRef ─────────────────────────────────────────────────────

ast_node!(Name, NAME);

impl Name {
    /// The identifier text.
    pub fn text(&self) -> Option<String> {
        child_token(&self.syntax, SyntaxKind::IDENT).map(|t| t.text().to_string())
    }
}

ast_node!(NameRef, NAME_REF);

impl NameRef {
    /// The identifier text.
    pub fn text(&self) -> Option<String> {
        child_token(&self.syntax, SyntaxKind::IDENT).map(|t| t.text().to_string())
    }
}

// ── Path ─────────────────────────────────────────────────────────────────

ast_node!(Path, PATH);

impl Path {
    /// All segment identifiers in the path.
    pub fn segments(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
            .collect()
    }
}

// ── Interface Definition ────────────────────────────────────────────────

ast_node!(InterfaceDef, INTERFACE_DEF);

impl InterfaceDef {
    /// The visibility modifier, if present.
    pub fn visibility(&self) -> Option<Visibility> {
        child_node(&self.syntax)
    }

    /// The interface name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The method signatures in the interface.
    pub fn methods(&self) -> impl Iterator<Item = InterfaceMethod> + '_ {
        child_nodes(&self.syntax)
    }

    /// The associated type declarations in the interface.
    pub fn assoc_types(&self) -> impl Iterator<Item = AssocTypeDef> + '_ {
        child_nodes(&self.syntax)
    }
}

ast_node!(InterfaceMethod, INTERFACE_METHOD);

impl InterfaceMethod {
    /// The method name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The parameter list.
    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The return type annotation, if present.
    pub fn return_type(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }

    /// The default method body block (for `do ... end` form), if present.
    ///
    /// Returns `None` for signature-only methods, `Some(Block)` for methods
    /// with a default implementation.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }
}

// ── Impl Definition ─────────────────────────────────────────────────────

ast_node!(ImplDef, IMPL_DEF);

impl ImplDef {
    /// The trait path being implemented.
    pub fn trait_path(&self) -> Option<Path> {
        child_node(&self.syntax)
    }

    /// The function definitions in the impl block.
    pub fn methods(&self) -> impl Iterator<Item = FnDef> + '_ {
        // Methods are inside the BLOCK child.
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children().filter_map(FnDef::cast))
    }

    /// The associated type bindings in the impl block.
    pub fn assoc_type_bindings(&self) -> impl Iterator<Item = AssocTypeBinding> + '_ {
        // Associated type bindings are inside the BLOCK child.
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children().filter_map(AssocTypeBinding::cast))
    }
}

// ── Associated Type Declaration ──────────────────────────────────────────

ast_node!(AssocTypeDef, ASSOC_TYPE_DEF);

impl AssocTypeDef {
    /// The associated type name (e.g., "Item").
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }
}

// ── Associated Type Binding ─────────────────────────────────────────────

ast_node!(AssocTypeBinding, ASSOC_TYPE_BINDING);

impl AssocTypeBinding {
    /// The associated type name (e.g., "Item").
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The concrete type node bound to this associated type.
    ///
    /// For `type Item = Int`, this returns the SyntaxNode containing the type
    /// expression after the `=` sign. The node will typically be an IDENT token
    /// or a more complex type expression (generic application, etc.).
    pub fn type_node(&self) -> Option<SyntaxNode> {
        // The type expression follows the EQ token. Look for a child that is
        // not NAME, not a keyword token, and not EQ -- it will be the type ref.
        // In practice, parse_type emits IDENT tokens (and optional GENERIC_ARG_LIST).
        // The simplest approach: return any child node that isn't NAME.
        self.syntax.children().find(|n| {
            n.kind() != SyntaxKind::NAME
        })
    }
}

// ── Type Alias ──────────────────────────────────────────────────────────

ast_node!(TypeAliasDef, TYPE_ALIAS_DEF);

impl TypeAliasDef {
    /// The visibility modifier (`pub`), if present.
    pub fn visibility(&self) -> Option<Visibility> {
        child_node(&self.syntax)
    }

    /// The alias name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The target type name: the first IDENT token appearing after the `=` sign.
    ///
    /// For `type Url = String`, returns `Some("String")`.
    /// For `type Pair<A, B> = (A, B)`, returns `None` (no bare IDENT after `=`).
    pub fn target_type_name(&self) -> Option<String> {
        let mut past_eq = false;
        for child in self.syntax.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::EQ {
                        past_eq = true;
                        continue;
                    }
                    if past_eq && t.kind() == SyntaxKind::IDENT {
                        return Some(t.text().to_string());
                    }
                }
                rowan::NodeOrToken::Node(_) => {
                    if past_eq {
                        // Complex type node after `=` — not a bare name
                        return None;
                    }
                }
            }
        }
        None
    }
}

// ── Sum Type Definition ──────────────────────────────────────────────────

ast_node!(SumTypeDef, SUM_TYPE_DEF);

impl SumTypeDef {
    /// The visibility modifier, if present.
    pub fn visibility(&self) -> Option<Visibility> {
        child_node(&self.syntax)
    }

    /// The sum type name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The variant definitions in the sum type.
    pub fn variants(&self) -> impl Iterator<Item = VariantDef> + '_ {
        child_nodes(&self.syntax)
    }

    /// Whether this sum type has an explicit `deriving(...)` clause.
    ///
    /// When `true`, only the traits listed in the clause are derived.
    /// When `false`, all default traits (Debug, Eq, Ord) are derived
    /// for backward compatibility.
    pub fn has_deriving_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|n| n.kind() == SyntaxKind::DERIVING_CLAUSE)
    }

    /// Returns the list of trait names from `deriving(Eq, Display, ...)`.
    ///
    /// Returns an empty Vec if no DERIVING_CLAUSE is present.
    /// A present but empty `deriving()` also returns an empty Vec
    /// (use `has_deriving_clause()` to distinguish).
    pub fn deriving_traits(&self) -> Vec<String> {
        self.syntax
            .children()
            .find(|n| n.kind() == SyntaxKind::DERIVING_CLAUSE)
            .map(|dc| {
                dc.children_with_tokens()
                    .filter_map(|it| it.into_token())
                    .filter(|t| t.kind() == SyntaxKind::IDENT && t.text() != "deriving")
                    .map(|t| t.text().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ── Variant Definition ──────────────────────────────────────────────────

ast_node!(VariantDef, VARIANT_DEF);

impl VariantDef {
    /// The variant name IDENT token.
    pub fn name(&self) -> Option<SyntaxToken> {
        child_token(&self.syntax, SyntaxKind::IDENT)
    }

    /// Named fields (VARIANT_FIELD children) in the variant.
    ///
    /// For `Rectangle(width :: Float, height :: Float)`, this yields the named fields.
    pub fn fields(&self) -> impl Iterator<Item = VariantField> + '_ {
        child_nodes(&self.syntax)
    }

    /// Positional type annotations (TYPE_ANNOTATION children) in the variant.
    ///
    /// For `Circle(Float)` or `Pair(Int, Int)`, this yields the positional types.
    pub fn positional_types(&self) -> impl Iterator<Item = TypeAnnotation> + '_ {
        child_nodes(&self.syntax)
    }
}

// ── Variant Field ───────────────────────────────────────────────────────

ast_node!(VariantField, VARIANT_FIELD);

impl VariantField {
    /// The field name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The field type annotation.
    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }
}

// ── Actor Definition ────────────────────────────────────────────────────

ast_node!(ActorDef, ACTOR_DEF);

impl ActorDef {
    /// The actor name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The parameter list (state arguments), if present.
    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The actor body block.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }

    /// The optional terminate clause for cleanup logic.
    pub fn terminate_clause(&self) -> Option<TerminateClause> {
        // The terminate clause is inside the BLOCK child of the actor body.
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children())
            .find_map(TerminateClause::cast)
    }
}

// ── Terminate Clause ─────────────────────────────────────────────────────

ast_node!(TerminateClause, TERMINATE_CLAUSE);

impl TerminateClause {
    /// The body block of the terminate clause.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }
}

// ── Guard Clause ────────────────────────────────────────────────────────

ast_node!(GuardClause, GUARD_CLAUSE);

impl GuardClause {
    /// The guard expression (the condition after `when`).
    ///
    /// For `fn abs(n) when n < 0 = -n`, this returns the expression `n < 0`.
    pub fn expr(&self) -> Option<super::expr::Expr> {
        self.syntax
            .children()
            .find_map(super::expr::Expr::cast)
    }
}

// ── FnExprBody ──────────────────────────────────────────────────────────

ast_node!(FnExprBody, FN_EXPR_BODY);

impl FnExprBody {
    /// The body expression.
    ///
    /// For `fn fib(0) = 0`, this returns the expression `0`.
    pub fn expr(&self) -> Option<super::expr::Expr> {
        self.syntax
            .children()
            .find_map(super::expr::Expr::cast)
    }
}

// ── Supervisor Definition ──────────────────────────────────────────────

ast_node!(SupervisorDef, SUPERVISOR_DEF);

impl SupervisorDef {
    /// The supervisor name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The strategy clause node, if present.
    pub fn strategy(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .flat_map(|n| {
                if n.kind() == SyntaxKind::BLOCK {
                    n.children().collect::<Vec<_>>()
                } else {
                    vec![n]
                }
            })
            .find(|c| c.kind() == SyntaxKind::STRATEGY_CLAUSE)
    }

    /// The max_restarts clause node, if present.
    pub fn max_restarts(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .flat_map(|n| {
                if n.kind() == SyntaxKind::BLOCK {
                    n.children().collect::<Vec<_>>()
                } else {
                    vec![n]
                }
            })
            .find(|c| c.kind() == SyntaxKind::RESTART_LIMIT)
    }

    /// The max_seconds clause node, if present.
    pub fn max_seconds(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .flat_map(|n| {
                if n.kind() == SyntaxKind::BLOCK {
                    n.children().collect::<Vec<_>>()
                } else {
                    vec![n]
                }
            })
            .find(|c| c.kind() == SyntaxKind::SECONDS_LIMIT)
    }

    /// The child spec nodes inside the supervisor body.
    pub fn child_specs(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .flat_map(|n| {
                if n.kind() == SyntaxKind::BLOCK {
                    n.children().collect::<Vec<_>>()
                } else {
                    vec![n]
                }
            })
            .filter(|c| c.kind() == SyntaxKind::CHILD_SPEC_DEF)
            .collect()
    }
}

// ── Service Definition ──────────────────────────────────────────────

ast_node!(ServiceDef, SERVICE_DEF);

impl ServiceDef {
    /// The service name.
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The init function definition (fn init(...) ... end), if present.
    pub fn init_fn(&self) -> Option<FnDef> {
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children())
            .find_map(FnDef::cast)
    }

    /// All call handlers in the service body.
    pub fn call_handlers(&self) -> Vec<CallHandler> {
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children())
            .filter_map(CallHandler::cast)
            .collect()
    }

    /// All cast handlers in the service body.
    pub fn cast_handlers(&self) -> Vec<CastHandler> {
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK)
            .flat_map(|block| block.children())
            .filter_map(CastHandler::cast)
            .collect()
    }
}

// ── Call Handler ─────────────────────────────────────────────────────

ast_node!(CallHandler, CALL_HANDLER);

impl CallHandler {
    /// The call handler variant name (e.g., "GetCount").
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The parameter list, if present.
    pub fn params(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The return type annotation (:: Type), if present.
    pub fn return_type(&self) -> Option<TypeAnnotation> {
        child_node(&self.syntax)
    }

    /// The state parameter name from the |state| pattern.
    pub fn state_param_name(&self) -> Option<String> {
        // First NAME is the handler name, second NAME (before BLOCK) is state param.
        let names: Vec<_> = self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::NAME)
            .collect();
        if names.len() >= 2 {
            return Name::cast(names[1].clone()).and_then(|n| n.text());
        }
        None
    }

    /// The body block of the call handler.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }
}

// ── Cast Handler ─────────────────────────────────────────────────────

ast_node!(CastHandler, CAST_HANDLER);

impl CastHandler {
    /// The cast handler variant name (e.g., "Reset").
    pub fn name(&self) -> Option<Name> {
        child_node(&self.syntax)
    }

    /// The parameter list, if present.
    pub fn params(&self) -> Option<ParamList> {
        child_node(&self.syntax)
    }

    /// The state parameter name from the |state| pattern.
    pub fn state_param_name(&self) -> Option<String> {
        // First NAME is the handler name, second NAME (before BLOCK) is state param.
        let names: Vec<_> = self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::NAME)
            .collect();
        if names.len() >= 2 {
            return Name::cast(names[1].clone()).and_then(|n| n.text());
        }
        None
    }

    /// The body block of the cast handler.
    pub fn body(&self) -> Option<Block> {
        child_node(&self.syntax)
    }
}
