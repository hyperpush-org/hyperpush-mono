//! Algorithm J inference engine for Mesh.
//!
//! Walks the Mesh AST, generates type constraints, and solves them via
//! unification. Implements Hindley-Milner type inference with:
//! - Let-polymorphism (generalize + instantiate)
//! - Occurs check (rejects infinite types)
//! - Level-based generalization (Remy's algorithm)
//! - Error provenance via ConstraintOrigin
//! - Trait system: interface definitions, impl blocks, where-clause enforcement
//! - Compiler-known traits for operator dispatch (Add, Eq, Ord, etc.)
//! - Struct definitions, struct literals, and field access (03-03)

use rowan::TextRange;
use mesh_parser::ast::expr::{
    BinaryExpr, BreakExpr, CallExpr, CaseExpr, ClosureExpr, ContinueExpr, Expr, FieldAccess,
    ForInExpr, IfExpr, LinkExpr, ListLiteral, Literal, MapLiteral, NameRef, PipeExpr, ReceiveExpr,
    ReturnExpr, SendExpr, SelfExpr, SpawnExpr, StructLiteral, StructUpdate, TryExpr, TupleExpr,
    UnaryExpr, WhileExpr,
};
use mesh_parser::ast::item::{
    ActorDef, Block, FnDef, InterfaceDef, ImplDef as AstImplDef, Item, LetBinding, ServiceDef,
    StructDef, SumTypeDef, SupervisorDef, TypeAliasDef,
};
use mesh_parser::ast::pat::Pattern;
use mesh_parser::ast::AstNode;
use mesh_parser::syntax_kind::SyntaxKind;
use mesh_parser::Parse;

use crate::builtins;
use crate::env::TypeEnv;
use crate::error::{ConstraintOrigin, TypeError};
use crate::exhaustiveness::{
    self, Pat as AbsPat, LitKind as AbsLitKind, TypeInfo as AbsTypeInfo,
    TypeRegistry as AbsTypeRegistry, ConstructorSig,
};
use crate::traits::{
    AssocTypeDef as TraitAssocTypeDef, ImplDef as TraitImplDef, ImplMethodSig, TraitDef,
    TraitMethodSig, TraitRegistry,
};
use crate::ty::{Scheme, Ty, TyCon, TyVar};
use crate::unify::InferCtx;
use crate::{ImportContext, TypeckResult};

use rustc_hash::FxHashMap;

/// Helper enum for tracking children in source order during multi-clause grouping.
enum ChildKind {
    /// An item, identified by its index in the original items list.
    ItemIndex(usize),
    /// A bare expression (not wrapped in an item).
    Expr(mesh_parser::SyntaxNode),
}

// ── Struct & Type Registry (03-03) ────────────────────────────────────

/// A registered struct definition with its fields and generic parameters.
#[derive(Clone, Debug)]
pub struct StructDefInfo {
    /// The struct's name.
    pub name: String,
    /// Names of generic type parameters (e.g., ["A", "B"] for `Pair<A, B>`).
    pub generic_params: Vec<String>,
    /// Field names and their types. Types may reference generic params.
    pub fields: Vec<(String, Ty)>,
}

/// A registered type alias.
#[derive(Clone, Debug)]
pub struct TypeAliasInfo {
    /// The alias name.
    #[allow(dead_code)]
    pub name: String,
    /// Names of generic type parameters.
    #[allow(dead_code)]
    pub generic_params: Vec<String>,
    /// The aliased type (may reference generic params).
    #[allow(dead_code)]
    pub aliased_type: Ty,
}

// ── Sum Type Registry (04-02) ──────────────────────────────────────────

/// A registered sum type definition with its variants and generic parameters.
#[derive(Clone, Debug)]
pub struct SumTypeDefInfo {
    /// The sum type's name (e.g. "Shape", "Option").
    pub name: String,
    /// Names of generic type parameters (e.g. ["T"] for `Option<T>`).
    pub generic_params: Vec<String>,
    /// Variant definitions.
    pub variants: Vec<VariantInfo>,
}

/// A single variant of a sum type.
#[derive(Clone, Debug)]
pub struct VariantInfo {
    /// The variant's name (e.g. "Circle", "Some").
    pub name: String,
    /// The variant's fields (positional or named).
    pub fields: Vec<VariantFieldInfo>,
}

/// A field in a variant -- either positional (unnamed) or named.
#[derive(Clone, Debug)]
pub enum VariantFieldInfo {
    /// Positional field (e.g. `Float` in `Circle(Float)`).
    Positional(Ty),
    /// Named field (e.g. `width :: Float` in `Rectangle(width :: Float, height :: Float)`).
    Named(String, Ty),
}

/// Registry for struct definitions, type aliases, and sum type definitions.
///
/// This is the central store of all type definitions in a Mesh program.
/// Codegen uses this to determine memory layouts for structs and ADTs.
#[derive(Clone, Debug, Default)]
pub struct TypeRegistry {
    /// Registered struct definitions, keyed by struct name.
    pub struct_defs: FxHashMap<String, StructDefInfo>,
    /// Registered type aliases, keyed by alias name.
    pub type_aliases: FxHashMap<String, TypeAliasInfo>,
    /// Registered sum type definitions, keyed by sum type name.
    pub sum_type_defs: FxHashMap<String, SumTypeDefInfo>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_struct(&mut self, info: StructDefInfo) {
        self.struct_defs.insert(info.name.clone(), info);
    }

    fn register_alias(&mut self, info: TypeAliasInfo) {
        self.type_aliases.insert(info.name.clone(), info);
    }

    pub fn register_sum_type(&mut self, info: SumTypeDefInfo) {
        self.sum_type_defs.insert(info.name.clone(), info);
    }

    fn lookup_struct(&self, name: &str) -> Option<&StructDefInfo> {
        self.struct_defs.get(name)
    }

    #[allow(dead_code)]
    fn lookup_alias(&self, name: &str) -> Option<&TypeAliasInfo> {
        self.type_aliases.get(name)
    }

    fn lookup_sum_type(&self, name: &str) -> Option<&SumTypeDefInfo> {
        self.sum_type_defs.get(name)
    }

    /// Look up a variant by its unqualified name (e.g. "Circle").
    /// Returns the parent sum type info and the variant info.
    #[allow(dead_code)]
    fn lookup_variant(&self, variant_name: &str) -> Option<(&SumTypeDefInfo, &VariantInfo)> {
        for sum_type in self.sum_type_defs.values() {
            for variant in &sum_type.variants {
                if variant.name == variant_name {
                    return Some((sum_type, variant));
                }
            }
        }
        None
    }

    /// Look up a variant by qualified name (e.g. "Shape" + "Circle").
    /// Returns the parent sum type info and the variant info.
    fn lookup_qualified_variant(
        &self,
        type_name: &str,
        variant_name: &str,
    ) -> Option<(&SumTypeDefInfo, &VariantInfo)> {
        if let Some(sum_type) = self.sum_type_defs.get(type_name) {
            for variant in &sum_type.variants {
                if variant.name == variant_name {
                    return Some((sum_type, variant));
                }
            }
        }
        None
    }
}

// ── Per-function metadata for where-clause enforcement (03-04) ────────

/// Per-function metadata for where-clause enforcement.
#[derive(Clone, Debug)]
struct FnConstraints {
    /// Where-clause constraints: (type_param_name, trait_name).
    where_constraints: Vec<(String, String)>,
    /// Type parameter names mapped to their inference type variables.
    type_params: FxHashMap<String, Ty>,
    /// For each function parameter (by index), the type parameter name it
    /// was annotated with (if any). Used to resolve type params from call-site
    /// argument types after instantiation + unification.
    param_type_param_names: Vec<Option<String>>,
}

// ── Standard Library Module Resolution (Phase 8) ──────────────────────

use std::collections::HashMap;

/// Build the stdlib module namespace registry.
///
/// Maps module names (e.g., "String", "IO", "Env") to their exported
/// function names and type schemes. This is used by both `from X import y`
/// and `X.y` resolution paths.
fn stdlib_modules() -> HashMap<String, HashMap<String, Scheme>> {
    let mut modules: HashMap<String, HashMap<String, Scheme>> = HashMap::new();

    // ── String module ──────────────────────────────────────────────
    let mut string_mod = HashMap::new();
    string_mod.insert(
        "length".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::int())),
    );
    string_mod.insert(
        "slice".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string(), Ty::int(), Ty::int()], Ty::string())),
    );
    string_mod.insert(
        "contains".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string(), Ty::string()], Ty::bool())),
    );
    string_mod.insert(
        "starts_with".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string(), Ty::string()], Ty::bool())),
    );
    string_mod.insert(
        "ends_with".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string(), Ty::string()], Ty::bool())),
    );
    string_mod.insert(
        "trim".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::string())),
    );
    string_mod.insert(
        "to_upper".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::string())),
    );
    string_mod.insert(
        "to_lower".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::string())),
    );
    string_mod.insert(
        "replace".to_string(),
        Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::string(), Ty::string()],
            Ty::string(),
        )),
    );
    string_mod.insert(
        "split".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string(), Ty::string()], Ty::list(Ty::string()))),
    );
    string_mod.insert(
        "join".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::list(Ty::string()), Ty::string()], Ty::string())),
    );
    string_mod.insert(
        "to_int".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::option(Ty::int()))),
    );
    string_mod.insert(
        "to_float".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::option(Ty::float()))),
    );
    // Phase 77: String.from(T) -> String (polymorphic -- accepts Int, Float, Bool)
    {
        let from_input_var = TyVar(91100);
        string_mod.insert("from".to_string(), Scheme {
            vars: vec![from_input_var],
            ty: Ty::fun(vec![Ty::Var(from_input_var)], Ty::string()),
        });
    }
    // Phase 79: String.collect(iter) -> String
    string_mod.insert("collect".to_string(), Scheme::mono(
        Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], Ty::string())
    ));
    modules.insert("String".to_string(), string_mod);

    // ── IO module ──────────────────────────────────────────────────
    let mut io_mod = HashMap::new();
    io_mod.insert(
        "read_line".to_string(),
        Scheme::mono(Ty::fun(vec![], Ty::result(Ty::string(), Ty::string()))),
    );
    io_mod.insert(
        "eprintln".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::Tuple(vec![]))),
    );
    modules.insert("IO".to_string(), io_mod);

    // ── Env module ─────────────────────────────────────────────────
    let mut env_mod = HashMap::new();
    env_mod.insert(
        "get".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::option(Ty::string()))),
    );
    modules.insert("Env".to_string(), env_mod);

    // ── File module ─────────────────────────────────────────────────
    let mut file_mod = HashMap::new();
    file_mod.insert(
        "read".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::result(Ty::string(), Ty::string()))),
    );
    file_mod.insert(
        "write".to_string(),
        Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::string()],
            Ty::result(Ty::Tuple(vec![]), Ty::string()),
        )),
    );
    file_mod.insert(
        "append".to_string(),
        Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::string()],
            Ty::result(Ty::Tuple(vec![]), Ty::string()),
        )),
    );
    file_mod.insert(
        "exists".to_string(),
        Scheme::mono(Ty::fun(vec![Ty::string()], Ty::bool())),
    );
    file_mod.insert(
        "delete".to_string(),
        Scheme::mono(Ty::fun(
            vec![Ty::string()],
            Ty::result(Ty::Tuple(vec![]), Ty::string()),
        )),
    );
    modules.insert("File".to_string(), file_mod);

    // ── Collection modules (Phase 8 Plan 02) ─────────────────────────

    // List module -- polymorphic: List<T> with type variables for element type.
    let t_var = TyVar(91000);
    let u_var = TyVar(91001);
    let t = Ty::Var(t_var);
    let u = Ty::Var(u_var);
    let list_t = Ty::list(t.clone());
    let list_u = Ty::list(u.clone());
    let t_to_u = Ty::fun(vec![t.clone()], u.clone());
    let t_to_bool = Ty::fun(vec![t.clone()], Ty::bool());
    let u_t_to_u = Ty::fun(vec![u.clone(), t.clone()], u.clone());

    let mut list_mod = HashMap::new();
    list_mod.insert("new".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![], list_t.clone()) });
    list_mod.insert("length".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone()], Ty::int()) });
    list_mod.insert("append".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t.clone()], list_t.clone()) });
    list_mod.insert("head".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone()], t.clone()) });
    list_mod.insert("tail".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone()], list_t.clone()) });
    list_mod.insert("get".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), Ty::int()], t.clone()) });
    list_mod.insert("concat".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), list_t.clone()], list_t.clone()) });
    list_mod.insert("reverse".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone()], list_t.clone()) });
    list_mod.insert("map".to_string(), Scheme { vars: vec![t_var, u_var], ty: Ty::fun(vec![list_t.clone(), t_to_u], list_u) });
    list_mod.insert("filter".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t_to_bool.clone()], list_t.clone()) });
    list_mod.insert("reduce".to_string(), Scheme { vars: vec![t_var, u_var], ty: Ty::fun(vec![list_t.clone(), u.clone(), u_t_to_u], u) });
    // Phase 46: sort, find, any, all, contains
    let t_t_to_int = Ty::fun(vec![t.clone(), t.clone()], Ty::int());
    list_mod.insert("sort".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t_t_to_int], list_t.clone()) });
    list_mod.insert("find".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t_to_bool.clone()], Ty::option(t.clone())) });
    list_mod.insert("any".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t_to_bool.clone()], Ty::bool()) });
    list_mod.insert("all".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t_to_bool.clone()], Ty::bool()) });
    list_mod.insert("contains".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), t.clone()], Ty::bool()) });
    // Phase 47: zip, flat_map, flatten, enumerate, take, drop, last, nth
    let u = Ty::Var(u_var);
    let list_u2 = Ty::list(u.clone());
    list_mod.insert("zip".to_string(), Scheme { vars: vec![t_var, u_var], ty: Ty::fun(vec![list_t.clone(), list_u2.clone()], Ty::list(Ty::Tuple(vec![t.clone(), u.clone()]))) });
    let t_to_list_u = Ty::fun(vec![t.clone()], list_u2.clone());
    list_mod.insert("flat_map".to_string(), Scheme { vars: vec![t_var, u_var], ty: Ty::fun(vec![list_t.clone(), t_to_list_u], list_u2.clone()) });
    list_mod.insert("flatten".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![Ty::list(list_t.clone())], list_t.clone()) });
    list_mod.insert("enumerate".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone()], Ty::list(Ty::Tuple(vec![Ty::int(), t.clone()]))) });
    list_mod.insert("take".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), Ty::int()], list_t.clone()) });
    list_mod.insert("drop".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), Ty::int()], list_t.clone()) });
    list_mod.insert("last".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone()], t.clone()) });
    list_mod.insert("nth".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![list_t.clone(), Ty::int()], t.clone()) });
    // Phase 79: List.collect(iter) -> List<T>
    list_mod.insert("collect".to_string(), Scheme { vars: vec![t_var], ty: Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], list_t.clone()) });
    modules.insert("List".to_string(), list_mod);

    // Map module -- polymorphic: Map<K, V> with type variables for key/value.
    let k_var = TyVar(90000);
    let v_var = TyVar(90001);
    let k = Ty::Var(k_var);
    let v = Ty::Var(v_var);
    let map_kv = Ty::map(k.clone(), v.clone());

    let mut map_mod = HashMap::new();
    map_mod.insert("new".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![], map_kv.clone()) });
    map_mod.insert("put".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone(), k.clone(), v.clone()], map_kv.clone()) });
    map_mod.insert("get".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone(), k.clone()], v.clone()) });
    map_mod.insert("has_key".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone(), k.clone()], Ty::bool()) });
    map_mod.insert("delete".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone(), k.clone()], map_kv.clone()) });
    map_mod.insert("size".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone()], Ty::int()) });
    map_mod.insert("keys".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone()], Ty::list(k.clone())) });
    map_mod.insert("values".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone()], Ty::list(v.clone())) });
    // Phase 47: merge, to_list, from_list
    map_mod.insert("merge".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone(), map_kv.clone()], map_kv.clone()) });
    map_mod.insert("to_list".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![map_kv.clone()], Ty::list(Ty::Tuple(vec![k.clone(), v.clone()]))) });
    map_mod.insert("from_list".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![Ty::list(Ty::Tuple(vec![k.clone(), v.clone()]))], map_kv.clone()) });
    // Phase 79: Map.collect(iter) -> Map<K, V>
    map_mod.insert("collect".to_string(), Scheme { vars: vec![k_var, v_var], ty: Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], map_kv.clone()) });
    modules.insert("Map".to_string(), map_mod);

    let set_t = Ty::set_untyped();
    let mut set_mod = HashMap::new();
    set_mod.insert("new".to_string(), Scheme::mono(Ty::fun(vec![], set_t.clone())));
    set_mod.insert("add".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone(), Ty::int()], set_t.clone())));
    set_mod.insert("remove".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone(), Ty::int()], set_t.clone())));
    set_mod.insert("contains".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone(), Ty::int()], Ty::bool())));
    set_mod.insert("size".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone()], Ty::int())));
    set_mod.insert("union".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone(), set_t.clone()], set_t.clone())));
    set_mod.insert("intersection".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone(), set_t.clone()], set_t.clone())));
    // Phase 47: difference, to_list, from_list
    set_mod.insert("difference".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone(), set_t.clone()], set_t.clone())));
    set_mod.insert("to_list".to_string(), Scheme::mono(Ty::fun(vec![set_t.clone()], Ty::list(Ty::int()))));
    set_mod.insert("from_list".to_string(), Scheme::mono(Ty::fun(vec![Ty::list(Ty::int())], set_t.clone())));
    // Phase 79: Set.collect(iter) -> Set
    set_mod.insert("collect".to_string(), Scheme::mono(Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], set_t.clone())));
    modules.insert("Set".to_string(), set_mod);

    let mut tuple_mod = HashMap::new();
    tuple_mod.insert("nth".to_string(), Scheme::mono(Ty::fun(vec![Ty::Con(TyCon::new("Tuple")), Ty::int()], Ty::int())));
    tuple_mod.insert("first".to_string(), Scheme::mono(Ty::fun(vec![Ty::Con(TyCon::new("Tuple"))], Ty::int())));
    tuple_mod.insert("second".to_string(), Scheme::mono(Ty::fun(vec![Ty::Con(TyCon::new("Tuple"))], Ty::int())));
    tuple_mod.insert("size".to_string(), Scheme::mono(Ty::fun(vec![Ty::Con(TyCon::new("Tuple"))], Ty::int())));
    modules.insert("Tuple".to_string(), tuple_mod);

    let range_t = Ty::range();
    let mut range_mod = HashMap::new();
    range_mod.insert("new".to_string(), Scheme::mono(Ty::fun(vec![Ty::int(), Ty::int()], range_t.clone())));
    range_mod.insert("to_list".to_string(), Scheme::mono(Ty::fun(vec![range_t.clone()], Ty::list(Ty::int()))));
    range_mod.insert("map".to_string(), Scheme::mono(Ty::fun(vec![range_t.clone(), Ty::fun(vec![Ty::int()], Ty::int())], Ty::list(Ty::int()))));
    range_mod.insert("filter".to_string(), Scheme::mono(Ty::fun(vec![range_t.clone(), Ty::fun(vec![Ty::int()], Ty::bool())], Ty::list(Ty::int()))));
    range_mod.insert("length".to_string(), Scheme::mono(Ty::fun(vec![range_t.clone()], Ty::int())));
    modules.insert("Range".to_string(), range_mod);

    let queue_t = Ty::queue();
    let mut queue_mod = HashMap::new();
    queue_mod.insert("new".to_string(), Scheme::mono(Ty::fun(vec![], queue_t.clone())));
    queue_mod.insert("push".to_string(), Scheme::mono(Ty::fun(vec![queue_t.clone(), Ty::int()], queue_t.clone())));
    queue_mod.insert("pop".to_string(), Scheme::mono(Ty::fun(vec![queue_t.clone()], Ty::Con(TyCon::new("Tuple")))));
    queue_mod.insert("peek".to_string(), Scheme::mono(Ty::fun(vec![queue_t.clone()], Ty::int())));
    queue_mod.insert("size".to_string(), Scheme::mono(Ty::fun(vec![queue_t.clone()], Ty::int())));
    queue_mod.insert("is_empty".to_string(), Scheme::mono(Ty::fun(vec![queue_t.clone()], Ty::bool())));
    modules.insert("Queue".to_string(), queue_mod);

    // ── JSON module (Phase 8 Plan 04) ─────────────────────────────────
    let json_t = Ty::Con(TyCon::new("Json"));
    let mut json_mod = HashMap::new();
    json_mod.insert("parse".to_string(), Scheme::mono(Ty::fun(vec![Ty::string()], Ty::result(json_t.clone(), Ty::string()))));
    json_mod.insert("encode".to_string(), Scheme::mono(Ty::fun(vec![json_t.clone()], Ty::string())));
    json_mod.insert("encode_string".to_string(), Scheme::mono(Ty::fun(vec![Ty::string()], Ty::string())));
    json_mod.insert("encode_int".to_string(), Scheme::mono(Ty::fun(vec![Ty::int()], Ty::string())));
    json_mod.insert("encode_bool".to_string(), Scheme::mono(Ty::fun(vec![Ty::bool()], Ty::string())));
    json_mod.insert("encode_map".to_string(), Scheme::mono(Ty::fun(vec![Ty::map_untyped()], Ty::string())));
    json_mod.insert("encode_list".to_string(), Scheme::mono(Ty::fun(vec![Ty::list_untyped()], Ty::string())));
    // Phase 103: JSON field extraction (no DB roundtrip)
    // Json.get(json_string, key) -> String
    json_mod.insert("get".to_string(), Scheme::mono(Ty::fun(vec![Ty::string(), Ty::string()], Ty::string())));
    // Json.get_nested(json_string, path1, path2) -> String
    json_mod.insert("get_nested".to_string(), Scheme::mono(Ty::fun(vec![Ty::string(), Ty::string(), Ty::string()], Ty::string())));

    modules.insert("JSON".to_string(), json_mod.clone());

    // Also register "Json" as an alias for the JSON module.
    // Mesh users write Json.encode(...) for struct serde (Phase 49).
    // Override `encode` to accept any type (struct with ToJson impl will be
    // dispatched at codegen time via ToJson__to_json__StructName).
    let json_encode_tv = TyVar(9999);
    let mut json_mod_alias = json_mod;
    json_mod_alias.insert("encode".to_string(), Scheme {
        vars: vec![json_encode_tv],
        ty: Ty::fun(vec![Ty::Var(json_encode_tv)], Ty::string()),
    });
    modules.insert("Json".to_string(), json_mod_alias);

    // ── HTTP module (Phase 8 Plan 05) ────────────────────────────────
    let request_t = Ty::Con(TyCon::new("Request"));
    let response_t = Ty::Con(TyCon::new("Response"));
    let router_t = Ty::Con(TyCon::new("Router"));

    let mut http_mod = HashMap::new();
    http_mod.insert("router".to_string(), Scheme::mono(Ty::fun(vec![], router_t.clone())));
    http_mod.insert("route".to_string(), Scheme::mono(Ty::fun(
        vec![router_t.clone(), Ty::string(), Ty::fun(vec![request_t.clone()], response_t.clone())],
        router_t.clone(),
    )));
    http_mod.insert("serve".to_string(), Scheme::mono(Ty::fun(vec![router_t.clone(), Ty::int()], Ty::Tuple(vec![]))));
    // Phase 56: HTTPS TLS server
    http_mod.insert("serve_tls".to_string(), Scheme::mono(Ty::fun(vec![router_t.clone(), Ty::int(), Ty::string(), Ty::string()], Ty::Tuple(vec![]))));
    http_mod.insert("response".to_string(), Scheme::mono(Ty::fun(vec![Ty::int(), Ty::string()], response_t.clone())));
    // Phase 88: response_with_headers(Int, String, Map<K, V>) -> Response
    {
        let k_var = TyVar(92000);
        let v_var = TyVar(92001);
        let k = Ty::Var(k_var);
        let v = Ty::Var(v_var);
        let map_kv = Ty::map(k, v);
        http_mod.insert("response_with_headers".to_string(), Scheme {
            vars: vec![k_var, v_var],
            ty: Ty::fun(vec![Ty::int(), Ty::string(), map_kv], response_t.clone()),
        });
    }
    http_mod.insert("get".to_string(), Scheme::mono(Ty::fun(vec![Ty::string()], Ty::result(Ty::string(), Ty::string()))));
    http_mod.insert("post".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::string()],
        Ty::result(Ty::string(), Ty::string()),
    )));
    // Phase 51: Method-specific routing
    http_mod.insert("on_get".to_string(), Scheme::mono(Ty::fun(
        vec![router_t.clone(), Ty::string(), Ty::fun(vec![request_t.clone()], response_t.clone())],
        router_t.clone(),
    )));
    http_mod.insert("on_post".to_string(), Scheme::mono(Ty::fun(
        vec![router_t.clone(), Ty::string(), Ty::fun(vec![request_t.clone()], response_t.clone())],
        router_t.clone(),
    )));
    http_mod.insert("on_put".to_string(), Scheme::mono(Ty::fun(
        vec![router_t.clone(), Ty::string(), Ty::fun(vec![request_t.clone()], response_t.clone())],
        router_t.clone(),
    )));
    http_mod.insert("on_delete".to_string(), Scheme::mono(Ty::fun(
        vec![router_t.clone(), Ty::string(), Ty::fun(vec![request_t.clone()], response_t.clone())],
        router_t.clone(),
    )));
    // Phase 52: Middleware
    http_mod.insert("use".to_string(), Scheme::mono(Ty::fun(
        vec![
            router_t.clone(),
            Ty::fun(
                vec![request_t.clone(), Ty::fun(vec![request_t.clone()], response_t.clone())],
                response_t.clone(),
            ),
        ],
        router_t.clone(),
    )));
    modules.insert("HTTP".to_string(), http_mod);

    // ── Request module (Phase 8 Plan 05) ─────────────────────────────
    let mut request_mod = HashMap::new();
    request_mod.insert("method".to_string(), Scheme::mono(Ty::fun(vec![request_t.clone()], Ty::string())));
    request_mod.insert("path".to_string(), Scheme::mono(Ty::fun(vec![request_t.clone()], Ty::string())));
    request_mod.insert("body".to_string(), Scheme::mono(Ty::fun(vec![request_t.clone()], Ty::string())));
    request_mod.insert("header".to_string(), Scheme::mono(Ty::fun(
        vec![request_t.clone(), Ty::string()],
        Ty::option(Ty::string()),
    )));
    request_mod.insert("query".to_string(), Scheme::mono(Ty::fun(
        vec![request_t.clone(), Ty::string()],
        Ty::option(Ty::string()),
    )));
    // Phase 51: Path parameter accessor (returns Option<String> -- runtime returns MeshOption)
    request_mod.insert("param".to_string(), Scheme::mono(Ty::fun(
        vec![request_t.clone(), Ty::string()],
        Ty::option(Ty::string()),
    )));
    modules.insert("Request".to_string(), request_mod);

    // ── Job module (Phase 9 Plan 02) ─────────────────────────────────
    // Job provides fire-and-forget async computation with Pid-based futures.
    // Uses synthetic TyVars for polymorphic type schemes -- these are replaced
    // with fresh vars during instantiation and never enter the unification table.
    let job_t = TyVar(u32::MAX - 10);  // Synthetic type var T
    let job_a = TyVar(u32::MAX - 11);  // Synthetic type var A
    let job_b = TyVar(u32::MAX - 12);  // Synthetic type var B
    let ty_t = Ty::Var(job_t);
    let ty_a = Ty::Var(job_a);
    let ty_b = Ty::Var(job_b);

    let mut job_mod = HashMap::new();

    // Job.async: fn(fn() -> T) -> Pid<T>
    job_mod.insert("async".to_string(), Scheme {
        vars: vec![job_t],
        ty: Ty::fun(vec![Ty::fun(vec![], ty_t.clone())], Ty::pid(ty_t.clone())),
    });

    // Job.await: fn(Pid<T>) -> Result<T, String>
    job_mod.insert("await".to_string(), Scheme {
        vars: vec![job_t],
        ty: Ty::fun(vec![Ty::pid(ty_t.clone())], Ty::result(ty_t.clone(), Ty::string())),
    });

    // Job.await_timeout: fn(Pid<T>, Int) -> Result<T, String>
    job_mod.insert("await_timeout".to_string(), Scheme {
        vars: vec![job_t],
        ty: Ty::fun(vec![Ty::pid(ty_t.clone()), Ty::int()], Ty::result(ty_t, Ty::string())),
    });

    // Job.map: fn(List<A>, fn(A) -> B) -> List<Result<B, String>>
    job_mod.insert("map".to_string(), Scheme {
        vars: vec![job_a, job_b],
        ty: Ty::fun(
            vec![Ty::list(ty_a.clone()), Ty::fun(vec![ty_a], ty_b.clone())],
            Ty::list(Ty::result(ty_b, Ty::string())),
        ),
    });

    modules.insert("Job".to_string(), job_mod);

    // ── Math module (Phase 43 Plan 01) ─────────────────────────────────
    // Polymorphic abs/min/max using TyVar(92000) for type variable t.
    let math_t_var = TyVar(92000);
    let math_t = Ty::Var(math_t_var);

    let mut math_mod = HashMap::new();
    math_mod.insert("abs".to_string(), Scheme { vars: vec![math_t_var], ty: Ty::fun(vec![math_t.clone()], math_t.clone()) });
    math_mod.insert("min".to_string(), Scheme { vars: vec![math_t_var], ty: Ty::fun(vec![math_t.clone(), math_t.clone()], math_t.clone()) });
    math_mod.insert("max".to_string(), Scheme { vars: vec![math_t_var], ty: Ty::fun(vec![math_t.clone(), math_t.clone()], math_t.clone()) });
    math_mod.insert("pi".to_string(), Scheme::mono(Ty::float()));
    // ── pow/sqrt/floor/ceil/round (Phase 43 Plan 02) ──────────────────
    math_mod.insert("pow".to_string(), Scheme::mono(Ty::fun(vec![Ty::float(), Ty::float()], Ty::float())));
    math_mod.insert("sqrt".to_string(), Scheme::mono(Ty::fun(vec![Ty::float()], Ty::float())));
    math_mod.insert("floor".to_string(), Scheme::mono(Ty::fun(vec![Ty::float()], Ty::int())));
    math_mod.insert("ceil".to_string(), Scheme::mono(Ty::fun(vec![Ty::float()], Ty::int())));
    math_mod.insert("round".to_string(), Scheme::mono(Ty::fun(vec![Ty::float()], Ty::int())));
    modules.insert("Math".to_string(), math_mod);

    // ── Int module (Phase 43 Plan 01) ──────────────────────────────────
    let mut int_mod = HashMap::new();
    int_mod.insert("to_float".to_string(), Scheme::mono(Ty::fun(vec![Ty::int()], Ty::float())));
    int_mod.insert("to_string".to_string(), Scheme::mono(Ty::fun(vec![Ty::int()], Ty::string())));
    modules.insert("Int".to_string(), int_mod);

    // ── Float module (Phase 43 Plan 01) ────────────────────────────────
    let mut float_mod = HashMap::new();
    float_mod.insert("to_int".to_string(), Scheme::mono(Ty::fun(vec![Ty::float()], Ty::int())));
    // Phase 77: Float.from(Int) -> Float
    float_mod.insert("from".to_string(), Scheme::mono(Ty::fun(vec![Ty::int()], Ty::float())));
    modules.insert("Float".to_string(), float_mod);

    // ── Timer module (Phase 44 Plan 02) ───────────────────────────────
    let timer_t_var = TyVar(u32::MAX - 20);  // Synthetic type var T for Timer
    let timer_t = Ty::Var(timer_t_var);

    let mut timer_mod = HashMap::new();
    // Timer.sleep: fn(Int) -> Unit
    timer_mod.insert("sleep".to_string(), Scheme::mono(Ty::fun(vec![Ty::int()], Ty::Tuple(vec![]))));
    // Timer.send_after: fn(Pid<T>, Int, T) -> Unit
    timer_mod.insert("send_after".to_string(), Scheme {
        vars: vec![timer_t_var],
        ty: Ty::fun(vec![Ty::pid(timer_t.clone()), Ty::int(), timer_t], Ty::Tuple(vec![])),
    });
    modules.insert("Timer".to_string(), timer_mod);

    // ── Sqlite module (Phase 53) ──────────────────────────────────────
    let sqlite_conn_t = Ty::Con(TyCon::new("SqliteConn"));

    let mut sqlite_mod = HashMap::new();
    // Sqlite.open: fn(String) -> Result<SqliteConn, String>
    sqlite_mod.insert("open".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::result(sqlite_conn_t.clone(), Ty::string()),
    )));
    // Sqlite.close: fn(SqliteConn) -> Unit
    sqlite_mod.insert("close".to_string(), Scheme::mono(Ty::fun(
        vec![sqlite_conn_t.clone()],
        Ty::Tuple(vec![]),
    )));
    // Sqlite.execute: fn(SqliteConn, String, List<String>) -> Result<Int, String>
    sqlite_mod.insert("execute".to_string(), Scheme::mono(Ty::fun(
        vec![sqlite_conn_t.clone(), Ty::string(), Ty::list(Ty::string())],
        Ty::result(Ty::int(), Ty::string()),
    )));
    // Sqlite.query: fn(SqliteConn, String, List<String>) -> Result<List<Map<String, String>>, String>
    sqlite_mod.insert("query".to_string(), Scheme::mono(Ty::fun(
        vec![sqlite_conn_t.clone(), Ty::string(), Ty::list(Ty::string())],
        Ty::result(Ty::list(Ty::map(Ty::string(), Ty::string())), Ty::string()),
    )));
    // Sqlite.begin: fn(SqliteConn) -> Result<Unit, String>
    sqlite_mod.insert("begin".to_string(), Scheme::mono(Ty::fun(
        vec![sqlite_conn_t.clone()],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    // Sqlite.commit: fn(SqliteConn) -> Result<Unit, String>
    sqlite_mod.insert("commit".to_string(), Scheme::mono(Ty::fun(
        vec![sqlite_conn_t.clone()],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    // Sqlite.rollback: fn(SqliteConn) -> Result<Unit, String>
    sqlite_mod.insert("rollback".to_string(), Scheme::mono(Ty::fun(
        vec![sqlite_conn_t.clone()],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    modules.insert("Sqlite".to_string(), sqlite_mod);

    // ── Pg module (Phase 54) ──────────────────────────────────────────
    let pg_conn_t = Ty::Con(TyCon::new("PgConn"));

    let mut pg_mod = HashMap::new();
    // Pg.connect: fn(String) -> Result<PgConn, String>
    pg_mod.insert("connect".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::result(pg_conn_t.clone(), Ty::string()),
    )));
    // Pg.close: fn(PgConn) -> Unit
    pg_mod.insert("close".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone()],
        Ty::Tuple(vec![]),
    )));
    // Pg.execute: fn(PgConn, String, List<String>) -> Result<Int, String>
    pg_mod.insert("execute".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone(), Ty::string(), Ty::list(Ty::string())],
        Ty::result(Ty::int(), Ty::string()),
    )));
    // Pg.query: fn(PgConn, String, List<String>) -> Result<List<Map<String, String>>, String>
    pg_mod.insert("query".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone(), Ty::string(), Ty::list(Ty::string())],
        Ty::result(Ty::list(Ty::map(Ty::string(), Ty::string())), Ty::string()),
    )));
    // Pg.begin: fn(PgConn) -> Result<Unit, String>
    pg_mod.insert("begin".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone()],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    // Pg.commit: fn(PgConn) -> Result<Unit, String>
    pg_mod.insert("commit".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone()],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    // Pg.rollback: fn(PgConn) -> Result<Unit, String>
    pg_mod.insert("rollback".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone()],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    // Pg.transaction: fn(PgConn, fn(PgConn) -> Result<Unit, String>) -> Result<Unit, String>
    pg_mod.insert("transaction".to_string(), Scheme::mono(Ty::fun(
        vec![pg_conn_t.clone(), Ty::fun(vec![pg_conn_t.clone()], Ty::result(Ty::Tuple(vec![]), Ty::string()))],
        Ty::result(Ty::Tuple(vec![]), Ty::string()),
    )));
    // Pg.query_as: forall a. fn(PgConn, String, List<String>, fn(Map<String, String>) -> Result<a, String>) -> Result<List<Result<a, String>>, String>
    {
        let a = TyVar(99990);
        let a_ty = Ty::Var(a);
        let from_row_fn = Ty::fun(vec![Ty::map(Ty::string(), Ty::string())], Ty::result(a_ty.clone(), Ty::string()));
        let ret = Ty::result(Ty::list(Ty::result(a_ty, Ty::string())), Ty::string());
        pg_mod.insert("query_as".to_string(), Scheme {
            vars: vec![a],
            ty: Ty::fun(vec![pg_conn_t.clone(), Ty::string(), Ty::list(Ty::string()), from_row_fn], ret),
        });
    }
    modules.insert("Pg".to_string(), pg_mod);

    // ── Pool module (Phase 57) ──────────────────────────────────────
    let pool_handle_t = Ty::Con(TyCon::new("PoolHandle"));

    let mut pool_mod = HashMap::new();
    // Pool.open: fn(String, Int, Int, Int) -> Result<PoolHandle, String>
    pool_mod.insert("open".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::int(), Ty::int(), Ty::int()],
        Ty::result(pool_handle_t.clone(), Ty::string()),
    )));
    // Pool.close: fn(PoolHandle) -> Unit
    pool_mod.insert("close".to_string(), Scheme::mono(Ty::fun(
        vec![pool_handle_t.clone()],
        Ty::Tuple(vec![]),
    )));
    // Pool.checkout: fn(PoolHandle) -> Result<PgConn, String>
    pool_mod.insert("checkout".to_string(), Scheme::mono(Ty::fun(
        vec![pool_handle_t.clone()],
        Ty::result(pg_conn_t.clone(), Ty::string()),
    )));
    // Pool.checkin: fn(PoolHandle, PgConn) -> Unit
    pool_mod.insert("checkin".to_string(), Scheme::mono(Ty::fun(
        vec![pool_handle_t.clone(), pg_conn_t.clone()],
        Ty::Tuple(vec![]),
    )));
    // Pool.query: fn(PoolHandle, String, List<String>) -> Result<List<Map<String, String>>, String>
    pool_mod.insert("query".to_string(), Scheme::mono(Ty::fun(
        vec![pool_handle_t.clone(), Ty::string(), Ty::list(Ty::string())],
        Ty::result(Ty::list(Ty::map(Ty::string(), Ty::string())), Ty::string()),
    )));
    // Pool.execute: fn(PoolHandle, String, List<String>) -> Result<Int, String>
    pool_mod.insert("execute".to_string(), Scheme::mono(Ty::fun(
        vec![pool_handle_t.clone(), Ty::string(), Ty::list(Ty::string())],
        Ty::result(Ty::int(), Ty::string()),
    )));
    // Pool.query_as: forall a. fn(PoolHandle, String, List<String>, fn(Map<String, String>) -> Result<a, String>) -> Result<List<Result<a, String>>, String>
    {
        let a = TyVar(99991);
        let a_ty = Ty::Var(a);
        let from_row_fn = Ty::fun(vec![Ty::map(Ty::string(), Ty::string())], Ty::result(a_ty.clone(), Ty::string()));
        let ret = Ty::result(Ty::list(Ty::result(a_ty, Ty::string())), Ty::string());
        pool_mod.insert("query_as".to_string(), Scheme {
            vars: vec![a],
            ty: Ty::fun(vec![pool_handle_t.clone(), Ty::string(), Ty::list(Ty::string()), from_row_fn], ret),
        });
    }
    modules.insert("Pool".to_string(), pool_mod);

    // ── Node module (Phase 67) ──────────────────────────────────────
    let node_spawn_t = TyVar(u32::MAX - 30);  // Synthetic type var T for Node.spawn
    let _node_t = Ty::Var(node_spawn_t);

    let mut node_mod = HashMap::new();
    // Node.start: fn(String, String) -> Int  (name, cookie -> 0 on success)
    node_mod.insert("start".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::string()],
        Ty::int(),
    )));
    // Node.connect: fn(String) -> Int  (node_name -> 0 on success)
    node_mod.insert("connect".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::int(),
    )));
    // Node.self: fn() -> String  (returns node name)
    node_mod.insert("self".to_string(), Scheme::mono(Ty::fun(vec![], Ty::string())));
    // Node.list: fn() -> List<String>  (returns connected node names)
    node_mod.insert("list".to_string(), Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));
    // Node.monitor: fn(String) -> Int  (node_name -> monitor ref)
    node_mod.insert("monitor".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::int(),
    )));
    // Node.spawn/spawn_link are NOT defined here because they are variadic
    // (node_name, func_ref, args...). The type checker handles them specially
    // in infer_field_access by returning a fresh function type that matches
    // the actual call site arguments.
    modules.insert("Node".to_string(), node_mod);

    // ── Process module (Phase 67) ───────────────────────────────────
    let mut process_mod = HashMap::new();
    // Process.monitor: fn(Int) -> Int  (target_pid -> monitor ref)
    process_mod.insert("monitor".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::int()],
        Ty::int(),
    )));
    // Process.demonitor: fn(Int) -> Int  (monitor_ref -> 0 on success)
    process_mod.insert("demonitor".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::int()],
        Ty::int(),
    )));
    // Process.register: fn(String, Pid<()>) -> Int  (name, pid -> 0 success, 1 error)
    process_mod.insert("register".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::Con(TyCon::new("Pid"))],
        Ty::int(),
    )));
    // Process.whereis: fn(String) -> Pid<()>  (name -> pid, 0 if not found)
    process_mod.insert("whereis".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::Con(TyCon::new("Pid")),
    )));
    modules.insert("Process".to_string(), process_mod);

    // ── Global module (Phase 68) ─────────────────────────────────────
    let mut global_mod = HashMap::new();
    // Global.register: fn(String, Pid<()>) -> Int  (name, pid -> 0 success, 1 error)
    // Pid type matches Process.register for consistency; runtime treats Pid as u64.
    global_mod.insert("register".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::Con(TyCon::new("Pid"))],
        Ty::int(),
    )));
    // Global.whereis: fn(String) -> Pid<()>  (name -> pid, 0 if not found)
    // Pid type matches Process.whereis for consistency; runtime treats Pid as u64.
    global_mod.insert("whereis".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::Con(TyCon::new("Pid")),
    )));
    // Global.unregister: fn(String) -> Int  (name -> 0 success, 1 not found)
    global_mod.insert("unregister".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string()],
        Ty::int(),
    )));
    modules.insert("Global".to_string(), global_mod);

    // ── Ws (WebSocket) module (Phase 88) ─────────────────────────
    let mut ws_mod = HashMap::new();
    // Ws.send: fn(Int, String) -> Int  (conn_handle, message -> 0 success)
    ws_mod.insert("send".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::int(), Ty::string()],
        Ty::int(),
    )));
    // Ws.serve: fn(fn(Int, String, Map<String,String>) -> Int, fn(Int, String) -> (), fn(Int, Int, String) -> (), Int) -> ()
    ws_mod.insert("serve".to_string(), Scheme::mono(Ty::fun(
        vec![
            Ty::fun(vec![Ty::int(), Ty::string(), Ty::map(Ty::string(), Ty::string())], Ty::int()),
            Ty::fun(vec![Ty::int(), Ty::string()], Ty::Tuple(vec![])),
            Ty::fun(vec![Ty::int(), Ty::int(), Ty::string()], Ty::Tuple(vec![])),
            Ty::int(),
        ],
        Ty::Tuple(vec![]),
    )));
    // ── WebSocket Room functions (Phase 90) ────────────────────────
    // Ws.join: fn(Int, String) -> Int  (conn_handle, room_name -> 0 success)
    ws_mod.insert("join".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::int(), Ty::string()],
        Ty::int(),
    )));
    // Ws.leave: fn(Int, String) -> Int  (conn_handle, room_name -> 0 success)
    ws_mod.insert("leave".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::int(), Ty::string()],
        Ty::int(),
    )));
    // Ws.broadcast: fn(String, String) -> Int  (room_name, message -> failure_count)
    ws_mod.insert("broadcast".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::string()],
        Ty::int(),
    )));
    // Ws.broadcast_except: fn(String, String, Int) -> Int  (room, msg, except_conn -> failure_count)
    ws_mod.insert("broadcast_except".to_string(), Scheme::mono(Ty::fun(
        vec![Ty::string(), Ty::string(), Ty::int()],
        Ty::int(),
    )));
    modules.insert("Ws".to_string(), ws_mod);

    // ── Iter module (Phase 76 + Phase 78) ──────────────────────────
    {
        let iter_t_var = TyVar(91200);
        let iter_t = Ty::Var(iter_t_var);
        let mut iter_mod = HashMap::new();
        // Iter.from: fn(List<T>) -> ListIterator (polymorphic over element type)
        iter_mod.insert("from".to_string(), Scheme {
            vars: vec![iter_t_var],
            ty: Ty::fun(vec![Ty::list(iter_t)], Ty::Con(TyCon::new("ListIterator"))),
        });

        // ── Phase 78: Lazy Combinators ──────────────────────────────
        // Iter.map: fn(Ptr, fn(T) -> U) -> Ptr
        {
            let t = TyVar(91201);
            let u = TyVar(91202);
            iter_mod.insert("map".to_string(), Scheme {
                vars: vec![t, u],
                ty: Ty::fun(vec![
                    Ty::Con(TyCon::new("Ptr")),
                    Ty::fun(vec![Ty::Var(t)], Ty::Var(u)),
                ], Ty::Con(TyCon::new("Ptr"))),
            });
        }
        // Iter.filter: fn(Ptr, fn(T) -> Bool) -> Ptr
        {
            let t = TyVar(91203);
            iter_mod.insert("filter".to_string(), Scheme {
                vars: vec![t],
                ty: Ty::fun(vec![
                    Ty::Con(TyCon::new("Ptr")),
                    Ty::fun(vec![Ty::Var(t)], Ty::bool()),
                ], Ty::Con(TyCon::new("Ptr"))),
            });
        }
        // Iter.take: fn(Ptr, Int) -> Ptr
        iter_mod.insert("take".to_string(), Scheme::mono(
            Ty::fun(vec![Ty::Con(TyCon::new("Ptr")), Ty::int()], Ty::Con(TyCon::new("Ptr")))
        ));
        // Iter.skip: fn(Ptr, Int) -> Ptr
        iter_mod.insert("skip".to_string(), Scheme::mono(
            Ty::fun(vec![Ty::Con(TyCon::new("Ptr")), Ty::int()], Ty::Con(TyCon::new("Ptr")))
        ));
        // Iter.enumerate: fn(Ptr) -> Ptr
        iter_mod.insert("enumerate".to_string(), Scheme::mono(
            Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], Ty::Con(TyCon::new("Ptr")))
        ));
        // Iter.zip: fn(Ptr, Ptr) -> Ptr
        iter_mod.insert("zip".to_string(), Scheme::mono(
            Ty::fun(vec![Ty::Con(TyCon::new("Ptr")), Ty::Con(TyCon::new("Ptr"))], Ty::Con(TyCon::new("Ptr")))
        ));

        // ── Phase 78: Terminals ─────────────────────────────────────
        // Iter.count: fn(Ptr) -> Int
        iter_mod.insert("count".to_string(), Scheme::mono(
            Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], Ty::int())
        ));
        // Iter.sum: fn(Ptr) -> Int
        iter_mod.insert("sum".to_string(), Scheme::mono(
            Ty::fun(vec![Ty::Con(TyCon::new("Ptr"))], Ty::int())
        ));
        // Iter.any: fn(Ptr, fn(T) -> Bool) -> Bool
        {
            let t = TyVar(91204);
            iter_mod.insert("any".to_string(), Scheme {
                vars: vec![t],
                ty: Ty::fun(vec![
                    Ty::Con(TyCon::new("Ptr")),
                    Ty::fun(vec![Ty::Var(t)], Ty::bool()),
                ], Ty::bool()),
            });
        }
        // Iter.all: fn(Ptr, fn(T) -> Bool) -> Bool
        {
            let t = TyVar(91205);
            iter_mod.insert("all".to_string(), Scheme {
                vars: vec![t],
                ty: Ty::fun(vec![
                    Ty::Con(TyCon::new("Ptr")),
                    Ty::fun(vec![Ty::Var(t)], Ty::bool()),
                ], Ty::bool()),
            });
        }
        // Iter.find: fn(Ptr, fn(T) -> Bool) -> Ptr (MeshOption at runtime)
        {
            let t = TyVar(91206);
            iter_mod.insert("find".to_string(), Scheme {
                vars: vec![t],
                ty: Ty::fun(vec![
                    Ty::Con(TyCon::new("Ptr")),
                    Ty::fun(vec![Ty::Var(t)], Ty::bool()),
                ], Ty::Con(TyCon::new("Ptr"))),
            });
        }
        // Iter.reduce: fn(Ptr, T, fn(T, T) -> T) -> T
        {
            let t = TyVar(91207);
            iter_mod.insert("reduce".to_string(), Scheme {
                vars: vec![t],
                ty: Ty::fun(vec![
                    Ty::Con(TyCon::new("Ptr")),
                    Ty::Var(t),
                    Ty::fun(vec![Ty::Var(t), Ty::Var(t)], Ty::Var(t)),
                ], Ty::Var(t)),
            });
        }

        modules.insert("Iter".to_string(), iter_mod);
    }

    // ── Orm module (Phase 97) ───────────────────────────────────────
    {
        let mut orm_mod = HashMap::new();
        // Orm.build_select: fn(String, List<String>, List<String>, List<String>, Int, Int) -> String
        orm_mod.insert("build_select".to_string(), Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::list(Ty::string()), Ty::list(Ty::string()), Ty::list(Ty::string()), Ty::int(), Ty::int()],
            Ty::string(),
        )));
        // Orm.build_insert: fn(String, List<String>, List<String>) -> String
        orm_mod.insert("build_insert".to_string(), Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::list(Ty::string()), Ty::list(Ty::string())],
            Ty::string(),
        )));
        // Orm.build_update: fn(String, List<String>, List<String>, List<String>) -> String
        orm_mod.insert("build_update".to_string(), Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::list(Ty::string()), Ty::list(Ty::string()), Ty::list(Ty::string())],
            Ty::string(),
        )));
        // Orm.build_delete: fn(String, List<String>, List<String>) -> String
        orm_mod.insert("build_delete".to_string(), Scheme::mono(Ty::fun(
            vec![Ty::string(), Ty::list(Ty::string()), Ty::list(Ty::string())],
            Ty::string(),
        )));
        modules.insert("Orm".to_string(), orm_mod);
    }

    // ── Query module (Phase 98) ─────────────────────────────────────
    {
        let ptr_t = Ty::Con(TyCon::new("Ptr"));
        let atom_t = Ty::Con(TyCon::new("Atom"));
        let mut query_mod = HashMap::new();
        // Query.from(String) -> Ptr
        query_mod.insert("from".to_string(), Scheme::mono(Ty::fun(
            vec![Ty::string()],
            ptr_t.clone(),
        )));
        // Query.where(Ptr, Atom, String) -> Ptr  (3-arg equality: field = value)
        query_mod.insert("where".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.where_op(Ptr, Atom, Atom, String) -> Ptr  (4-arg operator: field op value)
        query_mod.insert("where_op".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), atom_t.clone(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.where_in(Ptr, Atom, Ptr) -> Ptr  (field IN values_list)
        query_mod.insert("where_in".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Query.where_null(Ptr, Atom) -> Ptr  (field IS NULL)
        query_mod.insert("where_null".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.where_not_null(Ptr, Atom) -> Ptr  (field IS NOT NULL)
        query_mod.insert("where_not_null".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.where_not_in(Ptr, Atom, List<String>) -> Ptr  (field NOT IN (values...))
        query_mod.insert("where_not_in".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Query.where_between(Ptr, Atom, String, String) -> Ptr  (field BETWEEN low AND high)
        query_mod.insert("where_between".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::string(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.where_or(Ptr, List<Atom>, List<String>) -> Ptr  ((field1 = $N OR field2 = $M))
        query_mod.insert("where_or".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::list(atom_t.clone()), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Query.select(Ptr, List<String>) -> Ptr  (select fields list)
        query_mod.insert("select".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Query.order_by(Ptr, Atom, Atom) -> Ptr  (field atom, direction atom)
        query_mod.insert("order_by".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.limit(Ptr, Int) -> Ptr
        query_mod.insert("limit".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::int()],
            ptr_t.clone(),
        )));
        // Query.offset(Ptr, Int) -> Ptr
        query_mod.insert("offset".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::int()],
            ptr_t.clone(),
        )));
        // Query.join(Ptr, Atom, String, String) -> Ptr  (type atom, table, on_clause)
        query_mod.insert("join".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::string(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.join_as(Ptr, Atom, String, String, String) -> Ptr  (type atom, table, alias, on_clause)
        query_mod.insert("join_as".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::string(), Ty::string(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.group_by(Ptr, Atom) -> Ptr  (field atom)
        query_mod.insert("group_by".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.having(Ptr, String, String) -> Ptr  (clause, value)
        query_mod.insert("having".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::string(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.fragment(Ptr, String, List<String>) -> Ptr  (raw sql, params list)
        query_mod.insert("fragment".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::string(), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Query.select_raw(Ptr, List<String>) -> Ptr  (raw SQL expressions list)
        query_mod.insert("select_raw".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Query.where_raw(Ptr, String, List<String>) -> Ptr  (raw clause, params list)
        query_mod.insert("where_raw".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::string(), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Query.order_by_raw(Ptr, String) -> Ptr  (raw ORDER BY expression)
        query_mod.insert("order_by_raw".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::string()],
            ptr_t.clone(),
        )));
        // Query.group_by_raw(Ptr, String) -> Ptr  (raw GROUP BY expression)
        query_mod.insert("group_by_raw".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), Ty::string()],
            ptr_t.clone(),
        )));
        // ── Phase 108: Aggregate SELECT functions ─────────────────────────
        // Query.select_count(Ptr) -> Ptr  (count all rows)
        query_mod.insert("select_count".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Query.select_count_field(Ptr, Atom) -> Ptr  (count specific field)
        query_mod.insert("select_count_field".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.select_sum(Ptr, Atom) -> Ptr
        query_mod.insert("select_sum".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.select_avg(Ptr, Atom) -> Ptr
        query_mod.insert("select_avg".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.select_min(Ptr, Atom) -> Ptr
        query_mod.insert("select_min".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // Query.select_max(Ptr, Atom) -> Ptr
        query_mod.insert("select_max".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            ptr_t.clone(),
        )));
        // ── Phase 109: Subquery WHERE ─────────────────────────────────────
        // Query.where_sub(Ptr, Atom, Ptr) -> Ptr  (query, field, sub_query)
        query_mod.insert("where_sub".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        modules.insert("Query".to_string(), query_mod);
    }

    // ── Repo module (Phase 98) ──────────────────────────────────────
    {
        let ptr_t = Ty::Con(TyCon::new("Ptr"));
        let pool_t = Ty::Con(TyCon::new("PoolHandle"));
        let mut repo_mod = HashMap::new();
        // Repo.all(PoolHandle, Ptr) -> Result<List<Map<String,String>>, String>  (pool, query)
        repo_mod.insert("all".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), ptr_t.clone()],
            Ty::result(Ty::list(Ty::map(Ty::string(), Ty::string())), Ty::string()),
        )));
        // Repo.one(PoolHandle, Ptr) -> Ptr  (pool, query -> Result<Map<String,String>, String>)
        repo_mod.insert("one".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Repo.get(PoolHandle, String, String) -> Result<Map<String,String>, String>  (pool, table, id)
        repo_mod.insert("get".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string()],
            Ty::result(Ty::map(Ty::string(), Ty::string()), Ty::string()),
        )));
        // Repo.get_by(PoolHandle, String, String, String) -> Result<Map<String,String>, String>  (pool, table, field, value)
        repo_mod.insert("get_by".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string(), Ty::string()],
            Ty::result(Ty::map(Ty::string(), Ty::string()), Ty::string()),
        )));
        // Repo.count(PoolHandle, Ptr) -> Ptr  (pool, query -> Result<Int, String>)
        repo_mod.insert("count".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Repo.exists(PoolHandle, Ptr) -> Ptr  (pool, query -> Result<Bool, String>)
        repo_mod.insert("exists".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Repo.insert(PoolHandle, String, Map<String,String>) -> Result<Map<String,String>, String>  (pool, table, fields_map)
        repo_mod.insert("insert".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::map(Ty::string(), Ty::string())],
            Ty::result(Ty::map(Ty::string(), Ty::string()), Ty::string()),
        )));
        // Repo.update(PoolHandle, String, String, Ptr) -> Ptr  (pool, table, id, fields_map -> Result<Map<String,String>, String>)
        repo_mod.insert("update".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Repo.delete(PoolHandle, String, String) -> Result<Map<String,String>, String>  (pool, table, id)
        repo_mod.insert("delete".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string()],
            Ty::result(Ty::map(Ty::string(), Ty::string()), Ty::string()),
        )));
        // Repo.transaction(PoolHandle, fn(Ptr) -> Ptr) -> Ptr  (pool, callback -> Result<Ptr, String>)
        // Closure calling convention: MIR lowerer splits closure into (fn_ptr, env_ptr)
        repo_mod.insert("transaction".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::fun(vec![ptr_t.clone()], ptr_t.clone())],
            ptr_t.clone(),
        )));
        // ── Phase 103: Extended Repo Write Operations ────────────────────
        // Repo.update_where(PoolHandle, String, Map<String,String>, Ptr) -> Result<Map<String,String>, String>  (pool, table, fields_map, query)
        repo_mod.insert("update_where".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::map(Ty::string(), Ty::string()), ptr_t.clone()],
            Ty::result(Ty::map(Ty::string(), Ty::string()), Ty::string()),
        )));
        // Repo.delete_where(PoolHandle, String, Ptr) -> Result<Int, String>  (pool, table, query)
        repo_mod.insert("delete_where".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), ptr_t.clone()],
            Ty::result(Ty::int(), Ty::string()),
        )));
        // Repo.query_raw(PoolHandle, String, List<String>) -> Result<List<Map<String,String>>, String>
        repo_mod.insert("query_raw".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::list(Ty::string())],
            Ty::result(Ty::list(Ty::map(Ty::string(), Ty::string())), Ty::string()),
        )));
        // Repo.execute_raw(PoolHandle, String, List<String>) -> Result<Int, String>
        repo_mod.insert("execute_raw".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::list(Ty::string())],
            Ty::result(Ty::int(), Ty::string()),
        )));
        // ── Phase 109: Upsert, RETURNING, Subquery ────────────────────────
        // Repo.insert_or_update(PoolHandle, String, Map<String,String>, List<String>, List<String>) -> Ptr
        repo_mod.insert("insert_or_update".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::map(Ty::string(), Ty::string()), Ty::list(Ty::string()), Ty::list(Ty::string())],
            ptr_t.clone(),
        )));
        // Repo.delete_where_returning(PoolHandle, String, Ptr) -> Ptr
        repo_mod.insert("delete_where_returning".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // ── Phase 99: Repo Changeset Operations ────────────────────────
        // Repo.insert_changeset(PoolHandle, String, Ptr) -> Ptr  (pool, table, changeset -> Result<Map, Changeset>)
        repo_mod.insert("insert_changeset".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // Repo.update_changeset(PoolHandle, String, String, Ptr) -> Ptr  (pool, table, id, changeset -> Result<Map, Changeset>)
        repo_mod.insert("update_changeset".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        // ── Phase 100: Repo Preloading ───────────────────────────────────
        // Repo.preload(PoolHandle, Ptr, Ptr, Ptr) -> Ptr
        // (pool, rows: List<Map>, associations: List<String>, rel_meta: List<String>) -> Result<List<Map>, String>
        repo_mod.insert("preload".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), ptr_t.clone(), ptr_t.clone(), ptr_t.clone()],
            ptr_t.clone(),
        )));
        modules.insert("Repo".to_string(), repo_mod);
    }

    // ── Changeset module (Phase 99) ──────────────────────────────────
    {
        let ptr_t = Ty::Con(TyCon::new("Ptr"));
        let atom_t = Ty::Con(TyCon::new("Atom"));
        let map_ss = Ty::map(Ty::string(), Ty::string());
        let list_atom = Ty::list(atom_t.clone());
        let list_str = Ty::list(Ty::string());
        let mut cs_mod = HashMap::new();

        // Changeset.cast(Map<String,String>, Map<String,String>, List<Atom>) -> Ptr
        cs_mod.insert("cast".to_string(), Scheme::mono(Ty::fun(
            vec![map_ss.clone(), map_ss.clone(), list_atom.clone()],
            ptr_t.clone(),
        )));
        // Changeset.cast_with_types(Map<String,String>, Map<String,String>, List<Atom>, List<String>) -> Ptr
        cs_mod.insert("cast_with_types".to_string(), Scheme::mono(Ty::fun(
            vec![map_ss.clone(), map_ss.clone(), list_atom.clone(), list_str.clone()],
            ptr_t.clone(),
        )));

        // Changeset.validate_required(Ptr, List<Atom>) -> Ptr
        cs_mod.insert("validate_required".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), list_atom.clone()],
            ptr_t.clone(),
        )));
        // Changeset.validate_length(Ptr, Atom, Int, Int) -> Ptr
        cs_mod.insert("validate_length".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::int(), Ty::int()],
            ptr_t.clone(),
        )));
        // Changeset.validate_format(Ptr, Atom, String) -> Ptr
        cs_mod.insert("validate_format".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::string()],
            ptr_t.clone(),
        )));
        // Changeset.validate_inclusion(Ptr, Atom, List<String>) -> Ptr
        cs_mod.insert("validate_inclusion".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), list_str.clone()],
            ptr_t.clone(),
        )));
        // Changeset.validate_number(Ptr, Atom, Int, Int, Int, Int) -> Ptr
        cs_mod.insert("validate_number".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone(), Ty::int(), Ty::int(), Ty::int(), Ty::int()],
            ptr_t.clone(),
        )));

        // Changeset.valid(Ptr) -> Bool
        cs_mod.insert("valid".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone()],
            Ty::bool(),
        )));
        // Changeset.errors(Ptr) -> Map<String,String>
        cs_mod.insert("errors".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone()],
            map_ss.clone(),
        )));
        // Changeset.changes(Ptr) -> Map<String,String>
        cs_mod.insert("changes".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone()],
            map_ss.clone(),
        )));
        // Changeset.get_change(Ptr, Atom) -> String
        cs_mod.insert("get_change".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            Ty::string(),
        )));
        // Changeset.get_error(Ptr, Atom) -> String
        cs_mod.insert("get_error".to_string(), Scheme::mono(Ty::fun(
            vec![ptr_t.clone(), atom_t.clone()],
            Ty::string(),
        )));

        modules.insert("Changeset".to_string(), cs_mod);
    }

    // ── Migration module (Phase 101) ──────────────────────────────────
    {
        let pool_t = Ty::Con(TyCon::new("PoolHandle"));
        let result_t = Ty::result(Ty::int(), Ty::string());
        let mut migration_mod = HashMap::new();

        // Migration.create_table: fn(PoolHandle, String, List<String>) -> Result<Int, String>
        migration_mod.insert("create_table".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::list(Ty::string())],
            result_t.clone(),
        )));
        // Migration.drop_table: fn(PoolHandle, String) -> Result<Int, String>
        migration_mod.insert("drop_table".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string()],
            result_t.clone(),
        )));
        // Migration.add_column: fn(PoolHandle, String, String) -> Result<Int, String>
        migration_mod.insert("add_column".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string()],
            result_t.clone(),
        )));
        // Migration.drop_column: fn(PoolHandle, String, String) -> Result<Int, String>
        migration_mod.insert("drop_column".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string()],
            result_t.clone(),
        )));
        // Migration.rename_column: fn(PoolHandle, String, String, String) -> Result<Int, String>
        migration_mod.insert("rename_column".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::string(), Ty::string()],
            result_t.clone(),
        )));
        // Migration.create_index: fn(PoolHandle, String, List<String>, String) -> Result<Int, String>
        migration_mod.insert("create_index".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::list(Ty::string()), Ty::string()],
            result_t.clone(),
        )));
        // Migration.drop_index: fn(PoolHandle, String, List<String>) -> Result<Int, String>
        migration_mod.insert("drop_index".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string(), Ty::list(Ty::string())],
            result_t.clone(),
        )));
        // Migration.execute: fn(PoolHandle, String) -> Result<Int, String>
        migration_mod.insert("execute".to_string(), Scheme::mono(Ty::fun(
            vec![pool_t.clone(), Ty::string()],
            result_t.clone(),
        )));

        modules.insert("Migration".to_string(), migration_mod);
    }

    modules
}

/// Set of module names recognized by the stdlib for qualified access.
const STDLIB_MODULE_NAMES: &[&str] = &[
    "String", "IO", "Env", "File", "List", "Map", "Set", "Tuple", "Range", "Queue", "HTTP", "JSON", "Json", "Request", "Job",
    "Math", "Int", "Float", "Timer", "Sqlite", "Pg", "Pool",
    "Node", "Process",  // Phase 67
    "Global",  // Phase 68
    "Iter",  // Phase 76
    "Ws",  // Phase 88
    "Orm",  // Phase 97
    "Query",  // Phase 98
    "Repo",  // Phase 98
    "Changeset",  // Phase 99
    "Migration",  // Phase 101
];

/// Check if a name is a known stdlib module.
fn is_stdlib_module(name: &str) -> bool {
    STDLIB_MODULE_NAMES.contains(&name)
}

/// Infer types for a parsed Mesh program.
///
/// This is the main entry point for single-module type checking.
/// Delegates to `infer_with_imports` with an empty ImportContext.
pub fn infer(parse: &Parse) -> TypeckResult {
    infer_with_imports(parse, &ImportContext::empty())
}

/// Infer types for a parsed Mesh program with pre-resolved imports.
///
/// This is the multi-module entry point. The ImportContext contains
/// symbols from already-type-checked dependency modules, which are
/// pre-seeded into the type environments before inference begins.
pub fn infer_with_imports(parse: &Parse, import_ctx: &ImportContext) -> TypeckResult {
    let mut ctx = InferCtx::new();
    ctx.current_module = import_ctx.current_module.clone();
    let mut env = TypeEnv::new();
    let mut trait_registry = TraitRegistry::new();
    let mut type_registry = TypeRegistry::new();
    builtins::register_builtins(&mut ctx, &mut env, &mut trait_registry);
    register_builtin_sum_types(&mut ctx, &mut env, &mut type_registry);

    // Pre-seed with imported trait defs (XMOD-05: globally visible)
    for trait_def in &import_ctx.all_trait_defs {
        trait_registry.register_trait(trait_def.clone());
    }
    for impl_def in &import_ctx.all_trait_impls {
        let _ = trait_registry.register_impl(impl_def.clone());
    }

    // Pre-seed TypeRegistry and TypeEnv with imported struct definitions
    for (mod_namespace, mod_exports) in &import_ctx.module_exports {
        for (name, struct_def) in &mod_exports.struct_defs {
            type_registry.register_struct(struct_def.clone());
            // Register struct constructor in env with display_prefix set to source module
            let tycon = TyCon::with_module(name, mod_namespace.as_str());
            let struct_ty = if struct_def.generic_params.is_empty() {
                Ty::App(Box::new(Ty::Con(tycon)), vec![])
            } else {
                let type_args: Vec<Ty> = struct_def.generic_params.iter()
                    .map(|_| ctx.fresh_var()).collect();
                Ty::App(Box::new(Ty::Con(tycon)), type_args)
            };
            env.insert(name.clone(), Scheme::mono(struct_ty));
        }
    }

    // Pre-seed with imported sum type definitions
    for mod_exports in import_ctx.module_exports.values() {
        for (_name, sum_type_def) in &mod_exports.sum_type_defs {
            type_registry.register_sum_type(sum_type_def.clone());
            register_variant_constructors(
                &mut ctx, &mut env,
                &sum_type_def.name,
                &sum_type_def.generic_params,
                &sum_type_def.variants,
            );
        }
    }

    let mut types = FxHashMap::default();
    let mut result_type = None;
    let mut fn_constraints: FxHashMap<String, FnConstraints> = FxHashMap::default();
    let mut default_method_bodies: FxHashMap<(String, String), TextRange> = FxHashMap::default();

    let tree = parse.tree();

    // Collect all children and separate items from bare expressions.
    // Items are grouped to detect multi-clause functions before inference.
    let mut children_ordered: Vec<(TextRange, ChildKind)> = Vec::new();
    let mut items_for_grouping: Vec<Item> = Vec::new();

    for child in tree.syntax().children() {
        let range = child.text_range();
        if let Some(item) = Item::cast(child.clone()) {
            children_ordered.push((range, ChildKind::ItemIndex(items_for_grouping.len())));
            items_for_grouping.push(item);
        } else if let Some(_expr) = Expr::cast(child.clone()) {
            children_ordered.push((range, ChildKind::Expr(child)));
        }
    }

    // Group consecutive same-name, same-arity FnDef items.
    let grouped = group_multi_clause_fns(items_for_grouping);

    // Check for non-consecutive same-name function definitions.
    check_non_consecutive_clauses(&grouped, &mut ctx);

    // Build a map from original item index to grouped item index.
    // Each grouped item knows which original item indices it consumed.
    let mut item_idx_to_grouped: FxHashMap<usize, usize> = FxHashMap::default();
    {
        let mut original_idx = 0;
        for (grouped_idx, gi) in grouped.iter().enumerate() {
            match gi {
                GroupedItem::Single(_) => {
                    item_idx_to_grouped.insert(original_idx, grouped_idx);
                    original_idx += 1;
                }
                GroupedItem::MultiClause { clauses } => {
                    for _ in 0..clauses.len() {
                        item_idx_to_grouped.insert(original_idx, grouped_idx);
                        original_idx += 1;
                    }
                }
            }
        }
    }

    // Process in source order, but skip duplicate grouped item references.
    let mut processed_grouped: rustc_hash::FxHashSet<usize> = rustc_hash::FxHashSet::default();

    for (_range, child_kind) in &children_ordered {
        match child_kind {
            ChildKind::ItemIndex(orig_idx) => {
                if let Some(&grouped_idx) = item_idx_to_grouped.get(orig_idx) {
                    if processed_grouped.contains(&grouped_idx) {
                        continue; // Already processed as part of a multi-clause group.
                    }
                    processed_grouped.insert(grouped_idx);

                    match &grouped[grouped_idx] {
                        GroupedItem::Single(item) => {
                            let ty = infer_item(
                                &mut ctx,
                                &mut env,
                                item,
                                &mut types,
                                &mut type_registry,
                                &mut trait_registry,
                                &mut fn_constraints,
                                &mut default_method_bodies,
                                import_ctx,
                            );
                            if let Some(ty) = ty {
                                result_type = Some(ty);
                            }
                        }
                        GroupedItem::MultiClause { clauses } => {
                            match infer_multi_clause_fn(
                                &mut ctx,
                                &mut env,
                                clauses,
                                &mut types,
                                &type_registry,
                                &trait_registry,
                                &mut fn_constraints,
                                import_ctx,
                            ) {
                                Ok(ty) => {
                                    result_type = Some(ty);
                                }
                                Err(_) => {
                                    // Error already recorded in ctx.errors
                                }
                            }
                        }
                    }
                }
            }
            ChildKind::Expr(child_node) => {
                if let Some(expr) = Expr::cast(child_node.clone()) {
                    match infer_expr(
                        &mut ctx,
                        &mut env,
                        &expr,
                        &mut types,
                        &type_registry,
                        &trait_registry,
                        &fn_constraints,
                    ) {
                        Ok(ty) => {
                            let resolved = ctx.resolve(ty.clone());
                            types.insert(expr.syntax().text_range(), resolved.clone());
                            result_type = Some(resolved);
                        }
                        Err(_) => {
                            // Error already recorded in ctx.errors
                        }
                    }
                }
            }
        }
    }

    // Resolve all types in the type table through the union-find.
    let resolved_types: FxHashMap<TextRange, Ty> = types
        .into_iter()
        .map(|(range, ty)| (range, ctx.resolve(ty)))
        .collect();

    // Resolve the result type as well.
    let resolved_result = result_type.map(|ty| ctx.resolve(ty));

    // Extract qualified module function names for the MIR lowerer.
    let qualified_modules_for_codegen: FxHashMap<String, Vec<String>> = ctx.qualified_modules
        .iter()
        .map(|(mod_name, functions)| {
            (mod_name.clone(), functions.keys().cloned().collect())
        })
        .collect();

    TypeckResult {
        types: resolved_types,
        errors: ctx.errors,
        warnings: ctx.warnings,
        result_type: resolved_result,
        type_registry,
        trait_registry,
        default_method_bodies,
        qualified_modules: qualified_modules_for_codegen,
        imported_functions: ctx.imported_functions,
        imported_service_methods: ctx.imported_service_methods,
        local_service_exports: ctx.local_service_exports,
    }
}

/// Register Option<T> and Result<T, E> as proper sum types in the type registry.
///
/// This replaces the old approach of registering constructors only in the type
/// environment. By registering them as SumTypeDefInfo entries, exhaustiveness
/// checking works for Option/Result: `case opt do Some(x) -> x end` triggers
/// a non-exhaustive error because None is missing.
///
/// Uses enter_level/leave_level to ensure fresh type variables are created
/// at a higher level than current, so they get properly generalized into
/// polymorphic schemes (forall).
fn register_builtin_sum_types(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    type_registry: &mut TypeRegistry,
) {
    // ── Option<T> ──────────────────────────────────────────────────────
    //
    // type Option<T> do
    //   Some(T)
    //   None
    // end

    let option_generic_params = vec!["T".to_string()];
    let option_variants = vec![
        VariantInfo {
            name: "Some".to_string(),
            fields: vec![VariantFieldInfo::Positional(Ty::Con(TyCon::new("T")))],
        },
        VariantInfo {
            name: "None".to_string(),
            fields: vec![],
        },
    ];

    type_registry.register_sum_type(SumTypeDefInfo {
        name: "Option".to_string(),
        generic_params: option_generic_params.clone(),
        variants: option_variants.clone(),
    });

    register_variant_constructors(
        ctx,
        env,
        "Option",
        &option_generic_params,
        &option_variants,
    );

    // ── Result<T, E> ───────────────────────────────────────────────────
    //
    // type Result<T, E> do
    //   Ok(T)
    //   Err(E)
    // end

    let result_generic_params = vec!["T".to_string(), "E".to_string()];
    let result_variants = vec![
        VariantInfo {
            name: "Ok".to_string(),
            fields: vec![VariantFieldInfo::Positional(Ty::Con(TyCon::new("T")))],
        },
        VariantInfo {
            name: "Err".to_string(),
            fields: vec![VariantFieldInfo::Positional(Ty::Con(TyCon::new("E")))],
        },
    ];

    type_registry.register_sum_type(SumTypeDefInfo {
        name: "Result".to_string(),
        generic_params: result_generic_params.clone(),
        variants: result_variants.clone(),
    });

    register_variant_constructors(
        ctx,
        env,
        "Result",
        &result_generic_params,
        &result_variants,
    );

    // ── Ordering (Less | Equal | Greater) ────────────────────────────────
    //
    // type Ordering do
    //   Less
    //   Equal
    //   Greater
    // end
    //
    // Non-generic sum type used as the return type for compare().

    let ordering_variants = vec![
        VariantInfo {
            name: "Less".to_string(),
            fields: vec![],
        },
        VariantInfo {
            name: "Equal".to_string(),
            fields: vec![],
        },
        VariantInfo {
            name: "Greater".to_string(),
            fields: vec![],
        },
    ];

    type_registry.register_sum_type(SumTypeDefInfo {
        name: "Ordering".to_string(),
        generic_params: vec![],
        variants: ordering_variants.clone(),
    });

    register_variant_constructors(ctx, env, "Ordering", &[], &ordering_variants);
}

/// Register variant constructors for a sum type as polymorphic functions in env.
///
/// This is the shared logic extracted from `register_sum_type_def` so that both
/// user-defined sum types (parsed from source) and built-in sum types (Option,
/// Result) use the same variant registration mechanism.
pub fn register_variant_constructors(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    type_name: &str,
    generic_params: &[String],
    variants: &[VariantInfo],
) {
    for variant in variants {
        let field_types: Vec<Ty> = variant
            .fields
            .iter()
            .map(|f| match f {
                VariantFieldInfo::Positional(ty) => ty.clone(),
                VariantFieldInfo::Named(_, ty) => ty.clone(),
            })
            .collect();

        ctx.enter_level();

        // Create fresh type vars for generic params.
        let type_param_vars: Vec<Ty> = generic_params.iter().map(|_| ctx.fresh_var()).collect();

        // Substitute generic params in field types.
        let substituted_fields: Vec<Ty> = field_types
            .iter()
            .map(|fty| substitute_type_params(fty, generic_params, &type_param_vars))
            .collect();

        // The result type of the constructor: SumTypeName<T1, T2, ...>
        let result_ty = if type_param_vars.is_empty() {
            Ty::App(Box::new(Ty::Con(TyCon::new(type_name))), vec![])
        } else {
            Ty::App(
                Box::new(Ty::Con(TyCon::new(type_name))),
                type_param_vars.clone(),
            )
        };

        let ctor_ty = if substituted_fields.is_empty() {
            // Nullary constructor: not a function, just the type itself.
            result_ty.clone()
        } else {
            // Constructor with fields: function from fields to result type.
            Ty::Fun(substituted_fields, Box::new(result_ty.clone()))
        };

        ctx.leave_level();
        let scheme = ctx.generalize(ctor_ty);

        // Register under qualified name: Option.Some, Result.Ok
        let qualified_name = format!("{}.{}", type_name, variant.name);
        env.insert(qualified_name, scheme.clone());

        // Register under unqualified name: Some, None, Ok, Err
        env.insert(variant.name.clone(), scheme);
    }
}

// ── Multi-Clause Function Grouping (11-02) ────────────────────────────

/// A grouped item: either a single item or a multi-clause function group.
enum GroupedItem {
    /// A non-FnDef item, or a standalone single-clause FnDef.
    Single(Item),
    /// Consecutive same-name, same-arity FnDef items grouped together.
    MultiClause {
        /// All clauses (first clause contains visibility, generics, return type).
        clauses: Vec<FnDef>,
    },
}

/// Group consecutive same-name, same-arity FnDef items from a list of items.
///
/// Rules:
/// - Group by name AND arity (param count). Different arities are separate functions.
/// - Only group CONSECUTIVE FnDef items. Non-fn items break the grouping.
/// - A single FnDef with `= expr` body is treated as a 1-clause multi-clause function.
/// - A single FnDef with `do/end` body remains a Single item (regular function).
/// - Multiple consecutive FnDef nodes with the same name produce a MultiClause group.
fn group_multi_clause_fns(items: Vec<Item>) -> Vec<GroupedItem> {
    let mut result: Vec<GroupedItem> = Vec::new();
    let mut i = 0;

    while i < items.len() {
        match &items[i] {
            Item::FnDef(fn_def) => {
                let name = fn_def.name().and_then(|n| n.text()).unwrap_or_default();
                let arity = fn_def
                    .param_list()
                    .map(|pl| pl.params().count())
                    .unwrap_or(0);

                // Collect consecutive FnDef items with the same name and arity.
                let mut clauses = vec![fn_def.clone()];
                let mut j = i + 1;
                while j < items.len() {
                    if let Item::FnDef(next_fn) = &items[j] {
                        let next_name =
                            next_fn.name().and_then(|n| n.text()).unwrap_or_default();
                        let next_arity = next_fn
                            .param_list()
                            .map(|pl| pl.params().count())
                            .unwrap_or(0);
                        if next_name == name && next_arity == arity {
                            clauses.push(next_fn.clone());
                            j += 1;
                            continue;
                        }
                    }
                    break;
                }

                if clauses.len() == 1 {
                    // Single FnDef -- check if it's an `= expr` form.
                    if clauses[0].has_eq_body() {
                        // Single-clause multi-clause function (still valid).
                        result.push(GroupedItem::MultiClause { clauses });
                    } else {
                        // Regular do/end function -- keep as Single.
                        result.push(GroupedItem::Single(items[i].clone()));
                    }
                } else {
                    // Multiple clauses with same name/arity.
                    result.push(GroupedItem::MultiClause { clauses });
                }

                i = j;
            }
            _ => {
                result.push(GroupedItem::Single(items[i].clone()));
                i += 1;
            }
        }
    }

    result
}

/// Check for non-consecutive same-name function definitions after grouping.
///
/// If a function name/arity appears in multiple groups, emit a `NonConsecutiveClauses` error.
fn check_non_consecutive_clauses(grouped: &[GroupedItem], ctx: &mut InferCtx) {
    // Track seen function name/arity -> span of first group.
    let mut seen: FxHashMap<(String, usize), TextRange> = FxHashMap::default();

    for gi in grouped {
        let (name, arity, span) = match gi {
            GroupedItem::Single(Item::FnDef(fn_def)) => {
                let name = fn_def.name().and_then(|n| n.text()).unwrap_or_default();
                let arity = fn_def
                    .param_list()
                    .map(|pl| pl.params().count())
                    .unwrap_or(0);
                (name, arity, fn_def.syntax().text_range())
            }
            GroupedItem::MultiClause { clauses } => {
                let first = &clauses[0];
                let name = first.name().and_then(|n| n.text()).unwrap_or_default();
                let arity = first
                    .param_list()
                    .map(|pl| pl.params().count())
                    .unwrap_or(0);
                (name, arity, first.syntax().text_range())
            }
            _ => continue,
        };

        if name.is_empty() {
            continue;
        }

        let key = (name.clone(), arity);
        if let Some(first_span) = seen.get(&key) {
            ctx.errors.push(TypeError::NonConsecutiveClauses {
                fn_name: name,
                arity,
                first_span: *first_span,
                second_span: span,
            });
        } else {
            seen.insert(key, span);
        }
    }
}

/// Check if a clause is a "catch-all" -- all parameters are wildcards or simple variable bindings.
///
/// A catch-all clause has no literal, constructor, or tuple patterns in any parameter.
fn is_catch_all_clause(fn_def: &FnDef) -> bool {
    let param_list = match fn_def.param_list() {
        Some(pl) => pl,
        None => return true, // No params = catch-all (vacuously)
    };

    for param in param_list.params() {
        if let Some(pat) = param.pattern() {
            // Has a pattern child -- check if it's just a variable or wildcard.
            match pat {
                Pattern::Wildcard(_) | Pattern::Ident(_) => {
                    // Simple binding or wildcard -- still catch-all for this param.
                }
                _ => {
                    // Literal, constructor, tuple, or, as -- NOT catch-all.
                    return false;
                }
            }
        }
        // No pattern child means it's a plain IDENT parameter -- catch-all for this param.
    }

    // Also check if there's a guard -- a guarded clause is NOT catch-all.
    if fn_def.guard().is_some() {
        return false;
    }

    true
}

/// Infer a multi-clause function group.
///
/// Groups consecutive FnDef nodes with the same name/arity and type-checks them
/// as a single function with pattern matching on the parameters.
///
/// This conceptually desugars:
/// ```text
/// fn fib(0) = 0
/// fn fib(1) = 1
/// fn fib(n) = fib(n - 1) + fib(n - 2)
/// ```
/// into the equivalent of:
/// ```text
/// fn fib(__p0) do
///   case __p0 do
///     0 -> 0
///     1 -> 1
///     n -> fib(n - 1) + fib(n - 2)
///   end
/// end
/// ```
fn infer_multi_clause_fn(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    clauses: &[FnDef],
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
    _import_ctx: &ImportContext,
) -> Result<Ty, TypeError> {
    assert!(!clauses.is_empty());

    let first = &clauses[0];
    let fn_name = first
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let arity = first
        .param_list()
        .map(|pl| pl.params().count())
        .unwrap_or(0);

    // ── Step 1: Validate clause properties ─────────────────────────────

    // Check that non-first clauses don't have visibility, generics, return type.
    for (_idx, clause) in clauses.iter().enumerate().skip(1) {
        if clause.visibility().is_some() {
            ctx.warnings.push(TypeError::NonFirstClauseAnnotation {
                fn_name: fn_name.clone(),
                what: "visibility".to_string(),
                span: clause.syntax().text_range(),
            });
        }
        // Check for generic params on non-first clause.
        let has_generics = clause
            .syntax()
            .children()
            .any(|n| n.kind() == SyntaxKind::GENERIC_PARAM_LIST);
        if has_generics {
            ctx.warnings.push(TypeError::NonFirstClauseAnnotation {
                fn_name: fn_name.clone(),
                what: "generic parameters".to_string(),
                span: clause.syntax().text_range(),
            });
        }
        if clause.return_type().is_some() {
            ctx.warnings.push(TypeError::NonFirstClauseAnnotation {
                fn_name: fn_name.clone(),
                what: "return type annotation".to_string(),
                span: clause.syntax().text_range(),
            });
        }

        // Verify arity consistency.
        let clause_arity = clause
            .param_list()
            .map(|pl| pl.params().count())
            .unwrap_or(0);
        if clause_arity != arity {
            ctx.errors.push(TypeError::ClauseArityMismatch {
                fn_name: fn_name.clone(),
                expected_arity: arity,
                found_arity: clause_arity,
                span: clause.syntax().text_range(),
            });
        }

        // Check for where clause on non-first clause.
        let has_where = clause
            .syntax()
            .children()
            .any(|n| n.kind() == SyntaxKind::WHERE_CLAUSE);
        if has_where {
            ctx.warnings.push(TypeError::NonFirstClauseAnnotation {
                fn_name: fn_name.clone(),
                what: "where clause".to_string(),
                span: clause.syntax().text_range(),
            });
        }
    }

    // Check catch-all ordering: catch-all must be the last clause.
    if clauses.len() > 1 {
        for (idx, clause) in clauses.iter().enumerate() {
            if idx < clauses.len() - 1 && is_catch_all_clause(clause) {
                ctx.errors.push(TypeError::CatchAllNotLast {
                    fn_name: fn_name.clone(),
                    arity,
                    span: clause.syntax().text_range(),
                });
            }
        }
    }

    // ── Step 2: Set up function type infrastructure ────────────────────

    ctx.enter_level();

    // Pre-register the function name with a fresh type variable for recursion.
    let self_var = ctx.fresh_var();
    env.insert(fn_name.clone(), Scheme::mono(self_var.clone()));

    // Extract generic type parameters from the FIRST clause only.
    let mut type_params: FxHashMap<String, Ty> = FxHashMap::default();
    for child in first.syntax().children() {
        if child.kind() == SyntaxKind::GENERIC_PARAM_LIST {
            for tok in child.children_with_tokens() {
                if let Some(token) = tok.as_token() {
                    if token.kind() == SyntaxKind::IDENT {
                        let param_name = token.text().to_string();
                        let param_ty = ctx.fresh_var();
                        type_params.insert(param_name, param_ty);
                    }
                }
            }
        }
    }

    // Extract where-clause constraints from the first clause.
    let where_constraints = extract_where_constraints(first);

    // Create fresh type variables for each parameter position.
    let param_types: Vec<Ty> = (0..arity).map(|_| ctx.fresh_var()).collect();

    // Parse return type annotation from the first clause.
    // Use resolve_type_annotation which handles generic args and sugar types
    // (e.g., Result<Int, String>, Int!String, Int?) correctly.
    let return_type_annotation = first.return_type().and_then(|ann| {
        // First check if it's a type parameter name.
        if let Some(type_name) = resolve_type_name_str(&ann) {
            if let Some(tp_ty) = type_params.get(&type_name) {
                return Some(tp_ty.clone());
            }
        }
        // Use full annotation resolution for generic/sugar types.
        resolve_type_annotation(ctx, &ann, type_registry)
            .or_else(|| {
                resolve_type_name_str(&ann).map(|name| name_to_type(&name))
            })
    });

    // Store fn constraints if any.
    if !where_constraints.is_empty() || !type_params.is_empty() {
        let param_type_param_names: Vec<Option<String>> = (0..arity).map(|_| None).collect();
        fn_constraints.insert(
            fn_name.clone(),
            FnConstraints {
                where_constraints: where_constraints.clone(),
                type_params: type_params.clone(),
                param_type_param_names,
            },
        );
    }

    // ── Step 3: Infer each clause (like case arms) ─────────────────────

    ctx.push_fn_return_type(return_type_annotation.clone());

    let mut result_ty: Option<Ty> = None;
    let mut arm_patterns: Vec<AbsPat> = Vec::new();
    let mut arm_has_guard: Vec<bool> = Vec::new();
    let mut arm_spans: Vec<TextRange> = Vec::new();

    for clause in clauses {
        env.push_scope();

        // Insert type params into scope.
        for (name, ty) in &type_params {
            env.insert(name.clone(), Scheme::mono(ty.clone()));
        }

        // Process each parameter's pattern and unify with param type.
        let param_list = clause.param_list();
        let params: Vec<_> = param_list.iter().flat_map(|pl| pl.params()).collect();

        let mut clause_abs_pats: Vec<AbsPat> = Vec::new();

        for (param_idx, param) in params.iter().enumerate() {
            if param_idx >= arity {
                break;
            }

            if let Some(pat) = param.pattern() {
                // Pattern parameter -- infer the pattern type and unify.
                let pat_ty = infer_pattern(ctx, env, &pat, types, type_registry)?;
                ctx.unify(
                    pat_ty,
                    param_types[param_idx].clone(),
                    ConstraintOrigin::Builtin,
                )?;

                // Convert to abstract pattern for exhaustiveness.
                let abs_pat = ast_pattern_to_abstract(&pat, env, type_registry);
                clause_abs_pats.push(abs_pat);
            } else if let Some(name_tok) = param.name() {
                // Regular named parameter -- treat as wildcard pattern, bind the name.
                let name_text = name_tok.text().to_string();
                env.insert(name_text, Scheme::mono(param_types[param_idx].clone()));
                clause_abs_pats.push(AbsPat::Wildcard);
            } else {
                clause_abs_pats.push(AbsPat::Wildcard);
            }
        }

        // For exhaustiveness: combine param patterns into a single abstract pattern.
        // For single-param functions, use the pattern directly.
        // For multi-param functions, combine into a tuple pattern.
        let combined_pat = if clause_abs_pats.len() == 1 {
            clause_abs_pats.into_iter().next().unwrap()
        } else if clause_abs_pats.is_empty() {
            AbsPat::Wildcard
        } else {
            AbsPat::Constructor {
                name: "Tuple".to_string(),
                type_name: "Tuple".to_string(),
                args: clause_abs_pats,
            }
        };
        arm_patterns.push(combined_pat);

        // Process guard expression if present.
        let has_guard = clause.guard().is_some();
        arm_has_guard.push(has_guard);
        arm_spans.push(clause.syntax().text_range());

        if let Some(guard_clause) = clause.guard() {
            if let Some(guard_expr) = guard_clause.expr() {
                // For multi-clause function guards: accept arbitrary Bool expressions.
                // Do NOT call validate_guard_expr -- just type-check and verify Bool.
                let guard_ty = infer_expr(
                    ctx,
                    env,
                    &guard_expr,
                    types,
                    type_registry,
                    trait_registry,
                    fn_constraints,
                )?;
                let _ = ctx.unify(guard_ty, Ty::bool(), ConstraintOrigin::Builtin);
            }
        }

        // Infer the body expression.
        let body_ty = if let Some(expr_body) = clause.expr_body() {
            // `= expr` form
            infer_expr(
                ctx,
                env,
                &expr_body,
                types,
                type_registry,
                trait_registry,
                fn_constraints,
            )?
        } else if let Some(body) = clause.body() {
            // `do ... end` form (rare for multi-clause but allowed for single clause)
            infer_block(
                ctx,
                env,
                &body,
                types,
                type_registry,
                trait_registry,
                fn_constraints,
            )?
        } else {
            Ty::Tuple(vec![])
        };

        // Unify body type with previous clause body types.
        if let Some(ref prev_ty) = result_ty {
            ctx.unify(prev_ty.clone(), body_ty.clone(), ConstraintOrigin::Builtin)?;
        } else {
            result_ty = Some(body_ty.clone());
        }

        // Unify with return type annotation if present.
        if let Some(ref ret_ann) = return_type_annotation {
            ctx.unify(body_ty, ret_ann.clone(), ConstraintOrigin::Builtin)?;
        }

        env.pop_scope();
    }

    ctx.pop_fn_return_type();

    // ── Step 4: Exhaustiveness and redundancy checking ─────────────────

    // Build scrutinee type for exhaustiveness checking.
    let scrutinee_ty = if param_types.len() == 1 {
        ctx.resolve(param_types[0].clone())
    } else if param_types.is_empty() {
        Ty::Tuple(vec![])
    } else {
        Ty::Tuple(param_types.iter().map(|t| ctx.resolve(t.clone())).collect())
    };

    let scrutinee_type_info = type_to_type_info(&scrutinee_ty, type_registry);
    let abs_registry = build_abs_type_registry(type_registry);

    // For exhaustiveness: exclude guarded arms.
    let unguarded_patterns: Vec<AbsPat> = arm_patterns
        .iter()
        .zip(arm_has_guard.iter())
        .filter(|(_, has_guard)| !**has_guard)
        .map(|(pat, _)| pat.clone())
        .collect();

    if let Some(witnesses) = exhaustiveness::check_exhaustiveness(
        &unguarded_patterns,
        &scrutinee_type_info,
        &abs_registry,
    ) {
        let missing: Vec<String> = witnesses.iter().map(format_abstract_pat).collect();
        let err = TypeError::NonExhaustiveMatch {
            scrutinee_type: format!("{}", scrutinee_ty),
            missing_patterns: missing,
            span: first.syntax().text_range(),
        };
        ctx.warnings.push(err);
    }

    // Redundancy checking.
    let redundant_indices =
        exhaustiveness::check_redundancy(&arm_patterns, &scrutinee_type_info, &abs_registry);
    for idx in redundant_indices {
        let warn = TypeError::RedundantArm {
            arm_index: idx,
            span: arm_spans
                .get(idx)
                .copied()
                .unwrap_or(first.syntax().text_range()),
        };
        ctx.warnings.push(warn);
    }

    // ── Step 5: Build function type and register ───────────────────────

    let ret_ty = return_type_annotation
        .or(result_ty)
        .unwrap_or_else(|| Ty::Tuple(vec![]));
    let fn_ty = Ty::Fun(param_types, Box::new(ret_ty));

    ctx.unify(self_var, fn_ty.clone(), ConstraintOrigin::Builtin)?;

    ctx.leave_level();
    let scheme = ctx.generalize(fn_ty.clone());
    env.insert(fn_name, scheme);

    let resolved = ctx.resolve(fn_ty);
    types.insert(first.syntax().text_range(), resolved.clone());

    Ok(resolved)
}

// ── Item Inference ─────────────────────────────────────────────────────

/// Infer the type of a top-level or nested item.
/// Returns the type of the item (for let bindings, the type of the initializer;
/// for function defs, the function type).
fn infer_item(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    item: &Item,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &mut TypeRegistry,
    trait_registry: &mut TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
    default_method_bodies: &mut FxHashMap<(String, String), TextRange>,
    import_ctx: &ImportContext,
) -> Option<Ty> {
    match item {
        Item::LetBinding(let_) => {
            infer_let_binding(ctx, env, let_, types, type_registry, trait_registry, fn_constraints)
                .ok()
        }
        Item::FnDef(fn_) => {
            infer_fn_def(ctx, env, fn_, types, type_registry, trait_registry, fn_constraints).ok()
        }
        Item::StructDef(struct_def) => {
            register_struct_def(ctx, env, struct_def, type_registry, trait_registry);
            None
        }
        Item::TypeAliasDef(alias_def) => {
            register_type_alias(alias_def, type_registry);
            None
        }
        Item::InterfaceDef(iface) => {
            infer_interface_def(ctx, env, iface, trait_registry, default_method_bodies);
            None
        }
        Item::ImplDef(impl_) => {
            infer_impl_def(ctx, env, impl_, types, type_registry, trait_registry, fn_constraints);
            None
        }
        // Module declarations -- skip module def, handle imports.
        Item::ModuleDef(_) => None,
        Item::ImportDecl(ref import_decl) => {
            // Resolve import: check user modules first, then stdlib.
            if let Some(path) = import_decl.module_path() {
                let segments = path.segments();
                let full_name = segments.join(".");
                let last_segment = segments.last().cloned().unwrap_or_default();

                // Check user-defined modules via ImportContext
                if let Some(mod_exports) = import_ctx.module_exports.get(&last_segment) {
                    // Register the module namespace for qualified access
                    ctx.qualified_modules.insert(last_segment.clone(), mod_exports.functions.clone());
                    // Also register struct constructor types for qualified access
                    for (name, struct_def) in &mod_exports.struct_defs {
                        let tycon = TyCon::with_module(name, last_segment.as_str());
                        let struct_ty = if struct_def.generic_params.is_empty() {
                            Ty::App(Box::new(Ty::Con(tycon)), vec![])
                        } else {
                            let type_args: Vec<Ty> = struct_def.generic_params.iter()
                                .map(|_| ctx.fresh_var()).collect();
                            Ty::App(Box::new(Ty::Con(tycon)), type_args)
                        };
                        ctx.qualified_modules.entry(last_segment.clone())
                            .or_default()
                            .insert(name.clone(), Scheme::mono(struct_ty));
                    }
                    // Sum type variant constructors are already in mod_exports.functions
                    // (registered during the exporting module's type check).

                    // Register service definitions for qualified access (ServiceName.method)
                    for (service_name, service_info) in &mod_exports.service_defs {
                        // Register each helper function type in the environment
                        for (method_name, scheme) in &service_info.helpers {
                            let qualified = format!("{}.{}", service_name, method_name);
                            env.insert(qualified, scheme.clone());
                            // Also add to qualified_modules for MIR lowering
                            ctx.qualified_modules.entry(service_name.clone())
                                .or_default()
                                .insert(method_name.clone(), scheme.clone());
                        }
                        // Register the service name as a type constructor for field access
                        env.insert(service_name.clone(), Scheme::mono(Ty::Con(TyCon::new(service_name))));
                        // Register method mappings for MIR lowering
                        ctx.imported_service_methods.insert(
                            service_name.clone(),
                            service_info.methods.clone(),
                        );
                    }
                } else if is_stdlib_module(&last_segment) {
                    // Stdlib module -- already handled in infer_field_access.
                    // No action needed (backward compat).
                } else {
                    // IMPORT-06: Module not found
                    ctx.errors.push(TypeError::ImportModuleNotFound {
                        module_name: full_name,
                        span: import_decl.syntax().text_range(),
                        suggestion: None,
                    });
                }
            }
            None
        }
        Item::FromImportDecl(ref from_import) => {
            if let Some(path) = from_import.module_path() {
                let segments = path.segments();
                let full_name = segments.join(".");
                let last_segment = segments.last().cloned().unwrap_or_default();

                // Check user-defined modules first
                if let Some(mod_exports) = import_ctx.module_exports.get(&last_segment) {
                    if let Some(import_list) = from_import.import_list() {
                        for name_node in import_list.names() {
                            if let Some(name) = name_node.text() {
                                // Check functions
                                if let Some(scheme) = mod_exports.functions.get(&name) {
                                    env.insert(name.clone(), scheme.clone());
                                    ctx.imported_functions.push(name.clone());
                                }
                                // Check struct constructors
                                else if let Some(struct_def) = mod_exports.struct_defs.get(&name) {
                                    let tycon = TyCon::with_module(&name, last_segment.as_str());
                                    let struct_ty = if struct_def.generic_params.is_empty() {
                                        Ty::App(Box::new(Ty::Con(tycon)), vec![])
                                    } else {
                                        let type_args: Vec<Ty> = struct_def.generic_params.iter()
                                            .map(|_| ctx.fresh_var()).collect();
                                        Ty::App(Box::new(Ty::Con(tycon)), type_args)
                                    };
                                    env.insert(name.clone(), Scheme::mono(struct_ty));
                                    // Also register the struct in type_registry
                                    type_registry.register_struct(struct_def.clone());

                                    // Re-register Schema metadata functions for cross-module access.
                                    // When a struct has deriving(Schema), its __table__, __fields__, etc.
                                    // are only in the defining module's env. Re-register them here so
                                    // importing modules can call Organization.__table__() etc.
                                    let has_schema = import_ctx.all_trait_impls.iter().any(|imp| {
                                        imp.trait_name == "Schema" && imp.impl_type_name == name
                                    });
                                    if has_schema {
                                        // __table__ :: () -> String
                                        env.insert(format!("{}.__table__", name), Scheme::mono(Ty::fun(vec![], Ty::string())));
                                        // __fields__ :: () -> List<String>
                                        env.insert(format!("{}.__fields__", name), Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));
                                        // __primary_key__ :: () -> String
                                        env.insert(format!("{}.__primary_key__", name), Scheme::mono(Ty::fun(vec![], Ty::string())));
                                        // __relationships__ :: () -> List<String>
                                        env.insert(format!("{}.__relationships__", name), Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));
                                        // __field_types__ :: () -> List<String>
                                        env.insert(format!("{}.__field_types__", name), Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));
                                        // __relationship_meta__ :: () -> List<String>
                                        env.insert(format!("{}.__relationship_meta__", name), Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));
                                        // Per-field column accessors: __{field}_col__ :: () -> String
                                        for (field_name, _) in &struct_def.fields {
                                            env.insert(format!("{}.__{}_col__", name, field_name), Scheme::mono(Ty::fun(vec![], Ty::string())));
                                        }
                                    }
                                }
                                // Check sum type names (importing sum type brings constructors)
                                else if let Some(sum_def) = mod_exports.sum_type_defs.get(&name) {
                                    type_registry.register_sum_type(sum_def.clone());
                                    register_variant_constructors(
                                        ctx, env,
                                        &sum_def.name,
                                        &sum_def.generic_params,
                                        &sum_def.variants,
                                    );
                                }
                                // Check actor definitions (importing an actor brings its spawn function)
                                else if let Some(scheme) = mod_exports.actor_defs.get(&name) {
                                    env.insert(name.clone(), scheme.clone());
                                    ctx.imported_functions.push(name.clone());
                                }
                                // Check service definitions (importing a service brings its helpers)
                                else if let Some(service_info) = mod_exports.service_defs.get(&name) {
                                    // Register each helper function type in the environment
                                    for (method_name, scheme) in &service_info.helpers {
                                        let qualified = format!("{}.{}", name, method_name);
                                        env.insert(qualified, scheme.clone());
                                        ctx.qualified_modules.entry(name.clone())
                                            .or_default()
                                            .insert(method_name.clone(), scheme.clone());
                                    }
                                    // Register the service name as a type constructor
                                    env.insert(name.clone(), Scheme::mono(Ty::Con(TyCon::new(&name))));
                                    // Register method mappings for MIR lowering
                                    ctx.imported_service_methods.insert(
                                        name.clone(),
                                        service_info.methods.clone(),
                                    );
                                }
                                else {
                                    // Check if item exists but is private (VIS-03)
                                    if mod_exports.private_names.contains(&name) {
                                        ctx.errors.push(TypeError::PrivateItem {
                                            module_name: full_name.clone(),
                                            name: name.clone(),
                                            span: name_node.syntax().text_range(),
                                        });
                                    } else {
                                        // IMPORT-07: Name not found in module
                                        let available: Vec<String> = mod_exports.functions.keys()
                                            .chain(mod_exports.struct_defs.keys())
                                            .chain(mod_exports.sum_type_defs.keys())
                                            .chain(mod_exports.service_defs.keys())
                                            .chain(mod_exports.actor_defs.keys())
                                            .cloned()
                                            .collect();
                                        ctx.errors.push(TypeError::ImportNameNotFound {
                                            module_name: full_name.clone(),
                                            name: name.clone(),
                                            span: name_node.syntax().text_range(),
                                            available,
                                        });
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Fall back to stdlib modules (backward compat)
                    let modules = stdlib_modules();
                    if let Some(first_segment) = segments.first() {
                        if let Some(mod_fns) = modules.get(first_segment.as_str()) {
                            if let Some(import_list) = from_import.import_list() {
                                for name_node in import_list.names() {
                                    if let Some(name) = name_node.text() {
                                        if let Some(scheme) = mod_fns.get(&name) {
                                            env.insert(name.clone(), scheme.clone());
                                            let prefixed = format!(
                                                "{}_{}",
                                                first_segment.to_lowercase(),
                                                name
                                            );
                                            env.insert(prefixed, scheme.clone());
                                        }
                                    }
                                }
                            }
                        } else {
                            // Not a user module, not a stdlib module -> error
                            ctx.errors.push(TypeError::ImportModuleNotFound {
                                module_name: full_name,
                                span: from_import.syntax().text_range(),
                                suggestion: None,
                            });
                        }
                    }
                }
            }
            None
        }
        Item::SumTypeDef(sum_def) => {
            register_sum_type_def(ctx, env, sum_def, type_registry, trait_registry);
            None
        }
        Item::ActorDef(actor_def) => {
            infer_actor_def(ctx, env, actor_def, types, type_registry, trait_registry, fn_constraints).ok()
        }
        Item::ServiceDef(service_def) => {
            infer_service_def(ctx, env, &service_def, types, type_registry, trait_registry, fn_constraints).ok()
        }
        Item::SupervisorDef(sup_def) => {
            infer_supervisor_def(ctx, env, sup_def, types, type_registry, trait_registry, fn_constraints).ok()
        }
    }
}

// ── Struct Registration (03-03) ────────────────────────────────────────

/// Register a struct definition: extract field names/types and generic params.
fn register_struct_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    struct_def: &StructDef,
    type_registry: &mut TypeRegistry,
    trait_registry: &mut TraitRegistry,
) {
    let name = struct_def
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed>".to_string());

    // Extract generic type parameters.
    let generic_params: Vec<String> = struct_def
        .syntax()
        .children()
        .filter(|n| n.kind() == SyntaxKind::GENERIC_PARAM_LIST)
        .flat_map(|gpl| {
            gpl.children_with_tokens()
                .filter_map(|t| t.into_token())
                .filter(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
        })
        .collect();

    // Extract fields.
    let mut fields = Vec::new();
    for field in struct_def.fields() {
        let field_name = field
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let field_ty = field
            .type_annotation()
            .and_then(|ann| resolve_type_annotation(ctx, &ann, type_registry))
            .unwrap_or_else(|| ctx.fresh_var());

        fields.push((field_name, field_ty));
    }

    // Register a constructor function: StructName(field1, field2, ...) -> StructName
    // If in multi-module mode, set display_prefix on the TyCon for error messages.
    let tycon = if let Some(ref module) = ctx.current_module {
        TyCon::with_module(&name, module.as_str())
    } else {
        TyCon::new(&name)
    };
    let struct_ty = if generic_params.is_empty() {
        Ty::App(Box::new(Ty::Con(tycon)), vec![])
    } else {
        let type_args: Vec<Ty> = generic_params.iter().map(|_| ctx.fresh_var()).collect();
        Ty::App(Box::new(Ty::Con(tycon)), type_args)
    };

    env.insert(name.clone(), Scheme::mono(struct_ty));

    // Conditional auto-registration of trait impls based on deriving clause.
    // No deriving clause = backward compat (derive all default traits).
    // Explicit deriving(...) = only derive listed traits.
    let has_deriving = struct_def.has_deriving_clause();
    let derive_list = struct_def.deriving_traits();
    let derive_all = !has_deriving; // no clause = derive all defaults

    // Validate derive trait names.
    let valid_derives = ["Eq", "Ord", "Display", "Debug", "Hash", "Json", "Row", "Schema"];
    for trait_name in &derive_list {
        if !valid_derives.contains(&trait_name.as_str()) {
            ctx.errors.push(TypeError::UnsupportedDerive {
                trait_name: trait_name.clone(),
                type_name: name.clone(),
            });
        }
    }

    // Check trait dependencies: Ord requires Eq.
    if has_deriving && derive_list.iter().any(|t| t == "Ord") && !derive_list.iter().any(|t| t == "Eq") {
        ctx.errors.push(TypeError::MissingDerivePrerequisite {
            trait_name: "Ord".to_string(),
            requires: "Eq".to_string(),
            type_name: name.clone(),
        });
        // Register the struct type info so the rest of compilation doesn't crash,
        // but skip trait impl registration to avoid generating broken MIR.
        type_registry.register_struct(StructDefInfo {
            name,
            generic_params,
            fields,
        });
        return;
    }

    // Build the impl type: non-generic uses Ty::Con, generic uses Ty::App.
    let impl_ty = if generic_params.is_empty() {
        Ty::Con(TyCon::new(&name))
    } else {
        let base_ty = Ty::Con(TyCon::new(&name));
        let param_tys: Vec<Ty> = generic_params.iter().map(|p| Ty::Con(TyCon::new(p))).collect();
        Ty::App(Box::new(base_ty), param_tys)
    };

    // Schema metadata functions -- only via explicit deriving(Schema), structs only.
    // Registers __table__, __fields__, __primary_key__, __relationships__ as static
    // functions callable via StructName.__table__() syntax.
    if derive_list.iter().any(|t| t == "Schema") {
        // __table__ :: () -> String
        let table_fn_name = format!("{}.__table__", name);
        env.insert(table_fn_name, Scheme::mono(Ty::fun(vec![], Ty::string())));

        // __fields__ :: () -> List<String>
        let fields_fn_name = format!("{}.__fields__", name);
        env.insert(fields_fn_name, Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));

        // __primary_key__ :: () -> String
        let pk_fn_name = format!("{}.__primary_key__", name);
        env.insert(pk_fn_name, Scheme::mono(Ty::fun(vec![], Ty::string())));

        // __relationships__ :: () -> List<String>
        // Each relationship encoded as "kind:name:target" string.
        let rels_fn_name = format!("{}.__relationships__", name);
        env.insert(rels_fn_name, Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));

        // __field_types__ :: () -> List<String>
        // Each entry is "field_name:SQL_TYPE".
        let ft_fn_name = format!("{}.__field_types__", name);
        env.insert(ft_fn_name, Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));

        // __relationship_meta__ :: () -> List<String>
        // Each relationship encoded as "kind:name:target:fk:target_table" string.
        let meta_fn_name = format!("{}.__relationship_meta__", name);
        env.insert(meta_fn_name, Scheme::mono(Ty::fun(vec![], Ty::list(Ty::string()))));

        // Per-field column accessor functions: __{field}_col__ :: () -> String
        for field in struct_def.fields() {
            let field_name = field
                .name()
                .and_then(|n| n.text())
                .unwrap_or_else(|| "<unnamed>".to_string());
            let col_fn_name = format!("{}.__{}_col__", name, field_name);
            env.insert(col_fn_name, Scheme::mono(Ty::fun(vec![], Ty::string())));
        }

        // Register Schema trait impl so it's exported via collect_exports
        // and available for cross-module import resolution.
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Schema".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: FxHashMap::default(),
            associated_types: FxHashMap::default(),
        });
    }

    // Debug impl
    if derive_all || derive_list.iter().any(|t| t == "Debug") {
        let mut debug_methods = FxHashMap::default();
        debug_methods.insert(
            "inspect".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 0,
                return_type: Some(Ty::string()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Debug".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: debug_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Eq impl
    if derive_all || derive_list.iter().any(|t| t == "Eq") {
        let mut eq_methods = FxHashMap::default();
        eq_methods.insert(
            "eq".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 1,
                return_type: Some(Ty::bool()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Eq".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: eq_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Ord impl
    if derive_all || derive_list.iter().any(|t| t == "Ord") {
        let mut ord_methods = FxHashMap::default();
        ord_methods.insert(
            "lt".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 1,
                return_type: Some(Ty::bool()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Ord".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: ord_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Hash impl
    if derive_all || derive_list.iter().any(|t| t == "Hash") {
        let mut hash_methods = FxHashMap::default();
        hash_methods.insert(
            "hash".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 0,
                return_type: Some(Ty::int()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Hash".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: hash_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Display impl (only via explicit deriving, never auto-derived)
    if derive_list.iter().any(|t| t == "Display") {
        let mut display_methods = FxHashMap::default();
        display_methods.insert(
            "to_string".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 0,
                return_type: Some(Ty::string()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Display".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: display_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Json impl (ToJson + FromJson) -- only via explicit deriving(Json)
    if derive_list.iter().any(|t| t == "Json") {
        // Validate all fields are JSON-serializable BEFORE registering impls.
        let mut json_valid = true;
        for (field_name, field_ty) in &fields {
            if !is_json_serializable(field_ty, type_registry, trait_registry) {
                ctx.errors.push(TypeError::NonSerializableField {
                    struct_name: name.clone(),
                    field_name: field_name.clone(),
                    field_type: format!("{}", field_ty),
                });
                json_valid = false;
            }
        }

        if json_valid {
            let mut to_json_methods = FxHashMap::default();
            to_json_methods.insert(
                "to_json".to_string(),
                ImplMethodSig {
                    has_self: true,
                    param_count: 0,
                    return_type: Some(Ty::Con(TyCon::new("Json"))),
                },
            );
            let _ = trait_registry.register_impl(TraitImplDef {
                trait_name: "ToJson".to_string(),
                trait_type_args: vec![],
                impl_type: impl_ty.clone(),
                impl_type_name: name.clone(),
                methods: to_json_methods,
                associated_types: FxHashMap::default(),
            });

            let mut from_json_methods = FxHashMap::default();
            from_json_methods.insert(
                "from_json".to_string(),
                ImplMethodSig {
                    has_self: false,
                    param_count: 1,
                    return_type: Some(Ty::result(
                        Ty::Con(TyCon::new(&name)),
                        Ty::string(),
                    )),
                },
            );
            let _ = trait_registry.register_impl(TraitImplDef {
                trait_name: "FromJson".to_string(),
                trait_type_args: vec![],
                impl_type: impl_ty.clone(),
                impl_type_name: name.clone(),
                methods: from_json_methods,
                associated_types: FxHashMap::default(),
            });
        }
    }

    // Row impl (FromRow) -- only via explicit deriving(Row), structs only
    if derive_list.iter().any(|t| t == "Row") {
        let mut row_valid = true;
        for (field_name, field_ty) in &fields {
            if !is_row_mappable(field_ty) {
                ctx.errors.push(TypeError::NonMappableField {
                    struct_name: name.clone(),
                    field_name: field_name.clone(),
                    field_type: format!("{}", field_ty),
                });
                row_valid = false;
            }
        }

        if row_valid {
            let mut from_row_methods = FxHashMap::default();
            from_row_methods.insert(
                "from_row".to_string(),
                ImplMethodSig {
                    has_self: false,
                    param_count: 1, // takes a Map<String, String>
                    return_type: Some(Ty::result(
                        Ty::Con(TyCon::new(&name)),
                        Ty::string(),
                    )),
                },
            );
            let _ = trait_registry.register_impl(TraitImplDef {
                trait_name: "FromRow".to_string(),
                trait_type_args: vec![],
                impl_type: impl_ty.clone(),
                impl_type_name: name.clone(),
                methods: from_row_methods,
                associated_types: FxHashMap::default(),
            });
        }
    }

    type_registry.register_struct(StructDefInfo {
        name,
        generic_params,
        fields,
    });
}

/// Check if a type is JSON-serializable for deriving(Json) validation.
/// Serializable types: Int, Float, Bool, String, structs with ToJson impl,
/// Option<T> where T is serializable, List<T> where T is serializable,
/// Map<String, V> where V is serializable.
fn is_json_serializable(ty: &Ty, _type_registry: &TypeRegistry, trait_registry: &TraitRegistry) -> bool {
    match ty {
        Ty::Con(con) => match con.name.as_str() {
            "Int" | "Float" | "Bool" | "String" => true,
            name => {
                // Generic type params (single uppercase letter like T, U, V, K, A, B)
                // are treated as serializable at definition time. Invalid instantiations
                // will fail at link time with missing ToJson__to_json__<Type> errors.
                if name.len() == 1 && name.chars().next().map_or(false, |c| c.is_ascii_uppercase()) {
                    return true;
                }
                // Check if this type has a ToJson impl registered
                let ty_for_lookup = Ty::Con(TyCon::new(name));
                trait_registry.has_impl("ToJson", &ty_for_lookup)
            }
        },
        Ty::App(base, args) => {
            if let Ty::Con(con) = base.as_ref() {
                match con.name.as_str() {
                    "Option" => args.first().map_or(false, |t| is_json_serializable(t, _type_registry, trait_registry)),
                    "List" => args.first().map_or(false, |t| is_json_serializable(t, _type_registry, trait_registry)),
                    "Map" => {
                        // Map key must be String for JSON objects
                        let key_ok = args.first().map_or(false, |t| {
                            matches!(t, Ty::Con(c) if c.name == "String")
                        });
                        let val_ok = args.get(1).map_or(false, |t| is_json_serializable(t, _type_registry, trait_registry));
                        key_ok && val_ok
                    }
                    _ => false,
                }
            } else {
                false
            }
        },
        _ => false, // Type variables, functions, etc. are not serializable
    }
}

/// Check if a type is row-mappable for deriving(Row) validation.
/// Row-mappable: Int, Float, Bool, String, Option<T> where T is row-mappable primitive.
/// NOT mappable: nested structs, sum types (except Option), List, Map, Ptr.
fn is_row_mappable(ty: &Ty) -> bool {
    match ty {
        Ty::Con(con) => matches!(con.name.as_str(), "Int" | "Float" | "Bool" | "String"),
        Ty::App(base, args) => {
            if let Ty::Con(con) = base.as_ref() {
                if con.name == "Option" {
                    args.first().map_or(false, |t| is_row_mappable(t))
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Register a type alias.
fn register_type_alias(alias_def: &TypeAliasDef, type_registry: &mut TypeRegistry) {
    let name = alias_def
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed>".to_string());

    let generic_params: Vec<String> = alias_def
        .syntax()
        .children()
        .filter(|n| n.kind() == SyntaxKind::GENERIC_PARAM_LIST)
        .flat_map(|gpl| {
            gpl.children_with_tokens()
                .filter_map(|t| t.into_token())
                .filter(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
        })
        .collect();

    // Parse the aliased type from tokens after the `=` sign.
    let aliased_type = parse_alias_type(alias_def.syntax(), &generic_params);

    type_registry.register_alias(TypeAliasInfo {
        name,
        generic_params,
        aliased_type,
    });
}

/// Parse the aliased type from a TYPE_ALIAS_DEF node.
/// Collects tokens after the `=` sign and parses them as a type.
fn parse_alias_type(node: &mesh_parser::SyntaxNode, _generic_params: &[String]) -> Ty {
    let mut tokens: Vec<(SyntaxKind, String)> = Vec::new();
    let mut past_eq = false;

    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Token(t) => {
                let kind = t.kind();
                if kind == SyntaxKind::EQ {
                    past_eq = true;
                    continue;
                }
                if past_eq {
                    match kind {
                        SyntaxKind::IDENT | SyntaxKind::LT | SyntaxKind::GT
                        | SyntaxKind::COMMA | SyntaxKind::QUESTION | SyntaxKind::BANG
                        | SyntaxKind::L_PAREN | SyntaxKind::R_PAREN
                        | SyntaxKind::ARROW => {
                            tokens.push((kind, t.text().to_string()));
                        }
                        _ => {}
                    }
                }
            }
            rowan::NodeOrToken::Node(n) => {
                if past_eq {
                    collect_annotation_tokens(&n, &mut tokens);
                }
            }
        }
    }

    if tokens.is_empty() {
        return Ty::Never;
    }

    // Parse the tokens, treating generic_params as type variables
    // (they'll be represented as Ty::Con("A"), Ty::Con("B") etc.)
    parse_type_tokens(&tokens, &mut 0)
}

// ── Sum Type Registration (04-02) ──────────────────────────────────────

/// Register a sum type definition: extract variants, fields, and generic params.
/// Each variant constructor is registered as a polymorphic function in the env.
fn register_sum_type_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    sum_def: &SumTypeDef,
    type_registry: &mut TypeRegistry,
    trait_registry: &mut TraitRegistry,
) {
    let name = sum_def
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed>".to_string());

    // Extract generic type parameters.
    let generic_params: Vec<String> = sum_def
        .syntax()
        .children()
        .filter(|n| n.kind() == SyntaxKind::GENERIC_PARAM_LIST)
        .flat_map(|gpl| {
            gpl.children_with_tokens()
                .filter_map(|t| t.into_token())
                .filter(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
        })
        .collect();

    // Extract variants.
    let mut variants = Vec::new();
    for variant_def in sum_def.variants() {
        let variant_name = variant_def
            .name()
            .map(|t| t.text().to_string())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let mut fields = Vec::new();

        // Check for named fields first (VARIANT_FIELD children).
        let named_fields: Vec<_> = variant_def.fields().collect();
        if !named_fields.is_empty() {
            for field in named_fields {
                let field_name = field
                    .name()
                    .and_then(|n| n.text())
                    .unwrap_or_else(|| "<unnamed>".to_string());
                let field_ty = field
                    .type_annotation()
                    .and_then(|ann| resolve_type_annotation(ctx, &ann, type_registry))
                    .unwrap_or_else(|| ctx.fresh_var());
                fields.push(VariantFieldInfo::Named(field_name, field_ty));
            }
        } else {
            // Positional types (TYPE_ANNOTATION children directly under VARIANT_DEF).
            for type_ann in variant_def.positional_types() {
                let field_ty =
                    resolve_type_annotation(ctx, &type_ann, type_registry)
                        .unwrap_or_else(|| ctx.fresh_var());
                fields.push(VariantFieldInfo::Positional(field_ty));
            }
        }

        variants.push(VariantInfo {
            name: variant_name,
            fields,
        });
    }

    // Register the sum type info.
    let sum_info = SumTypeDefInfo {
        name: name.clone(),
        generic_params: generic_params.clone(),
        variants: variants.clone(),
    };
    type_registry.register_sum_type(sum_info);

    // Register each variant constructor using the shared mechanism.
    register_variant_constructors(ctx, env, &name, &generic_params, &variants);

    // Conditional auto-registration of trait impls based on deriving clause.
    // No deriving clause = backward compat (derive all default traits).
    // Explicit deriving(...) = only derive listed traits.
    let has_deriving = sum_def.has_deriving_clause();
    let derive_list = sum_def.deriving_traits();
    let derive_all = !has_deriving; // no clause = derive all defaults

    // Validate derive trait names.
    let valid_derives = ["Eq", "Ord", "Display", "Debug", "Hash", "Json", "Row", "Schema"];
    for trait_name in &derive_list {
        if !valid_derives.contains(&trait_name.as_str()) {
            ctx.errors.push(TypeError::UnsupportedDerive {
                trait_name: trait_name.clone(),
                type_name: name.clone(),
            });
        }
    }

    // Schema is only supported on structs, not sum types.
    if derive_list.iter().any(|t| t == "Schema") {
        ctx.errors.push(TypeError::UnsupportedDerive {
            trait_name: "Schema".to_string(),
            type_name: name.clone(),
        });
    }

    // Check trait dependencies: Ord requires Eq.
    if has_deriving && derive_list.iter().any(|t| t == "Ord") && !derive_list.iter().any(|t| t == "Eq") {
        ctx.errors.push(TypeError::MissingDerivePrerequisite {
            trait_name: "Ord".to_string(),
            requires: "Eq".to_string(),
            type_name: name.clone(),
        });
        // Sum type and variant constructors are already registered above.
        // Skip trait impl registration to avoid generating broken MIR.
        return;
    }

    // Build the impl type: non-generic uses Ty::Con, generic uses Ty::App.
    let impl_ty = if generic_params.is_empty() {
        Ty::Con(TyCon::new(&name))
    } else {
        let base_ty = Ty::Con(TyCon::new(&name));
        let param_tys: Vec<Ty> = generic_params.iter().map(|p| Ty::Con(TyCon::new(p))).collect();
        Ty::App(Box::new(base_ty), param_tys)
    };

    // Debug impl
    if derive_all || derive_list.iter().any(|t| t == "Debug") {
        let mut debug_methods = FxHashMap::default();
        debug_methods.insert(
            "inspect".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 0,
                return_type: Some(Ty::string()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Debug".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: debug_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Eq impl
    if derive_all || derive_list.iter().any(|t| t == "Eq") {
        let mut eq_methods = FxHashMap::default();
        eq_methods.insert(
            "eq".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 1,
                return_type: Some(Ty::bool()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Eq".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: eq_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Ord impl
    if derive_all || derive_list.iter().any(|t| t == "Ord") {
        let mut ord_methods = FxHashMap::default();
        ord_methods.insert(
            "lt".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 1,
                return_type: Some(Ty::bool()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Ord".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: ord_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Hash impl (only via explicit deriving for sum types, never auto-derived)
    if derive_list.iter().any(|t| t == "Hash") {
        let mut hash_methods = FxHashMap::default();
        hash_methods.insert(
            "hash".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 0,
                return_type: Some(Ty::int()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Hash".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: hash_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Display impl (only via explicit deriving, never auto-derived)
    if derive_list.iter().any(|t| t == "Display") {
        let mut display_methods = FxHashMap::default();
        display_methods.insert(
            "to_string".to_string(),
            ImplMethodSig {
                has_self: true,
                param_count: 0,
                return_type: Some(Ty::string()),
            },
        );
        let _ = trait_registry.register_impl(TraitImplDef {
            trait_name: "Display".to_string(),
            trait_type_args: vec![],
            impl_type: impl_ty.clone(),
            impl_type_name: name.clone(),
            methods: display_methods,
            associated_types: FxHashMap::default(),
        });
    }

    // Json impl (ToJson + FromJson) -- only via explicit deriving(Json)
    if derive_list.iter().any(|t| t == "Json") {
        // Validate all variant field types are JSON-serializable.
        let mut json_valid = true;
        for variant in &variants {
            for (field_idx, field) in variant.fields.iter().enumerate() {
                let field_ty = match field {
                    VariantFieldInfo::Positional(ty) => ty,
                    VariantFieldInfo::Named(_, ty) => ty,
                };
                if !is_json_serializable(field_ty, type_registry, trait_registry) {
                    let field_ident = match field {
                        VariantFieldInfo::Positional(_) => format!("{}::{}", variant.name, field_idx),
                        VariantFieldInfo::Named(fname, _) => format!("{}::{}", variant.name, fname),
                    };
                    ctx.errors.push(TypeError::NonSerializableField {
                        struct_name: name.clone(),
                        field_name: field_ident,
                        field_type: format!("{}", field_ty),
                    });
                    json_valid = false;
                }
            }
        }

        if json_valid {
            let mut to_json_methods = FxHashMap::default();
            to_json_methods.insert(
                "to_json".to_string(),
                ImplMethodSig {
                    has_self: true,
                    param_count: 0,
                    return_type: Some(Ty::Con(TyCon::new("Json"))),
                },
            );
            let _ = trait_registry.register_impl(TraitImplDef {
                trait_name: "ToJson".to_string(),
                trait_type_args: vec![],
                impl_type: impl_ty.clone(),
                impl_type_name: name.clone(),
                methods: to_json_methods,
                associated_types: FxHashMap::default(),
            });

            let mut from_json_methods = FxHashMap::default();
            from_json_methods.insert(
                "from_json".to_string(),
                ImplMethodSig {
                    has_self: false,
                    param_count: 1,
                    return_type: Some(Ty::result(
                        Ty::Con(TyCon::new(&name)),
                        Ty::string(),
                    )),
                },
            );
            let _ = trait_registry.register_impl(TraitImplDef {
                trait_name: "FromJson".to_string(),
                trait_type_args: vec![],
                impl_type: impl_ty,
                impl_type_name: name.clone(),
                methods: from_json_methods,
                associated_types: FxHashMap::default(),
            });
        }
    }
}

// ── Interface/Impl Registration (03-04) ───────────────────────────────

/// Process an interface definition: register the trait in the registry.
/// Also stores default method body syntax nodes for later MIR lowering.
fn infer_interface_def(
    _ctx: &mut InferCtx,
    _env: &mut TypeEnv,
    iface: &InterfaceDef,
    trait_registry: &mut TraitRegistry,
    default_method_bodies: &mut FxHashMap<(String, String), TextRange>,
) {
    let trait_name = iface
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed>".to_string());

    let mut methods = Vec::new();
    for method in iface.methods() {
        let method_name = method
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let mut has_self = false;
        let mut param_count = 0;

        if let Some(param_list) = method.param_list() {
            for param in param_list.params() {
                let is_self = param
                    .syntax()
                    .children_with_tokens()
                    .any(|tok| {
                        tok.as_token()
                            .map(|t| t.kind() == SyntaxKind::SELF_KW)
                            .unwrap_or(false)
                    });
                if is_self {
                    has_self = true;
                } else {
                    param_count += 1;
                }
            }
        }

        let return_type = method.return_type().and_then(|ann| resolve_type_name(&ann));

        let has_default_body = method.body().is_some();

        // Store the default method body text range for MIR lowering.
        if has_default_body {
            default_method_bodies.insert(
                (trait_name.clone(), method_name.clone()),
                method.syntax().text_range(),
            );
        }

        methods.push(TraitMethodSig {
            name: method_name,
            has_self,
            param_count,
            return_type,
            has_default_body,
        });
    }

    // Collect associated type declarations from the interface.
    let mut associated_types = Vec::new();
    for assoc in iface.assoc_types() {
        if let Some(name) = assoc.name().and_then(|n| n.text()) {
            associated_types.push(TraitAssocTypeDef { name });
        }
    }

    trait_registry.register_trait(TraitDef {
        name: trait_name,
        methods,
        associated_types,
    });
}

/// Extract the concrete type from an associated type binding node.
///
/// The ASSOC_TYPE_BINDING node contains: TYPE_KW, NAME, EQ, then the type tokens.
/// For simple types (Int, String, T), there's a bare IDENT token after EQ.
/// For generic types, there's IDENT + GENERIC_ARG_LIST.
/// We collect all significant tokens after EQ and parse them into a Ty.
fn resolve_assoc_type_binding(
    binding: &mesh_parser::ast::item::AssocTypeBinding,
    type_registry: &TypeRegistry,
) -> Option<Ty> {
    let mut tokens: Vec<(SyntaxKind, String)> = Vec::new();
    let mut past_eq = false;
    for child in binding.syntax().children_with_tokens() {
        match child {
            rowan::NodeOrToken::Token(t) => {
                let kind = t.kind();
                if kind == SyntaxKind::EQ {
                    past_eq = true;
                    continue;
                }
                if past_eq {
                    match kind {
                        SyntaxKind::IDENT | SyntaxKind::LT | SyntaxKind::GT
                        | SyntaxKind::COMMA | SyntaxKind::QUESTION | SyntaxKind::BANG
                        | SyntaxKind::L_PAREN | SyntaxKind::R_PAREN
                        | SyntaxKind::ARROW | SyntaxKind::DOT => {
                            tokens.push((kind, t.text().to_string()));
                        }
                        _ => {}
                    }
                }
            }
            rowan::NodeOrToken::Node(n) => {
                if past_eq {
                    // Recurse into child nodes (e.g., GENERIC_ARG_LIST)
                    collect_annotation_tokens(&n, &mut tokens);
                }
            }
        }
    }
    if tokens.is_empty() {
        return None;
    }
    let mut start = 0;
    let ty = parse_type_tokens(&tokens, &mut start);
    Some(resolve_alias(ty, type_registry))
}

/// Resolve a Self.X reference in a type annotation to the concrete associated type.
///
/// Returns Some(Ty) if the annotation contains a Self.X pattern where X is found
/// in the assoc_types map. Returns None if no Self.X pattern is found.
///
/// Note: uppercase `Self` is parsed as an IDENT token (not SELF_KW, which is
/// lowercase `self`).
fn resolve_self_assoc_type(
    ann: &mesh_parser::ast::item::TypeAnnotation,
    assoc_types: &FxHashMap<String, Ty>,
) -> Option<Ty> {
    // Look for Self.X pattern in annotation tokens:
    // IDENT("Self") followed by DOT followed by IDENT(name)
    // Filter out whitespace/trivia tokens first.
    let tokens: Vec<_> = ann
        .syntax()
        .children_with_tokens()
        .filter_map(|t| t.into_token())
        .filter(|t| !t.kind().is_trivia())
        .collect();

    // Skip leading ARROW (return type annotation prefix)
    let mut i = 0;
    while i < tokens.len() && tokens[i].kind() == SyntaxKind::ARROW {
        i += 1;
    }

    // Look for IDENT("Self") DOT IDENT pattern
    while i + 2 < tokens.len() {
        if tokens[i].kind() == SyntaxKind::IDENT
            && tokens[i].text() == "Self"
            && tokens[i + 1].kind() == SyntaxKind::DOT
            && tokens[i + 2].kind() == SyntaxKind::IDENT
        {
            let assoc_name = tokens[i + 2].text().to_string();
            return assoc_types.get(&assoc_name).cloned();
        }
        i += 1;
    }
    None
}

/// Process an impl definition: register the impl and type-check methods.
fn infer_impl_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    impl_: &AstImplDef,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &mut TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
) {
    // Extract trait name from the first PATH child.
    let paths: Vec<_> = impl_
        .syntax()
        .children()
        .filter(|n| n.kind() == SyntaxKind::PATH)
        .collect();

    let trait_name = paths
        .first()
        .and_then(|path| {
            path.children_with_tokens()
                .filter_map(|t| t.into_token())
                .find(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
        })
        .unwrap_or_else(|| "<unknown>".to_string());

    // Extract trait type arguments from GENERIC_ARG_LIST (e.g., <Int> in From<Int>).
    // GENERIC_ARG_LIST is a direct child of IMPL_DEF, appearing after the trait PATH.
    let trait_type_args: Vec<Ty> = impl_
        .syntax()
        .children()
        .filter(|n| n.kind() == SyntaxKind::GENERIC_ARG_LIST)
        .flat_map(|gal| {
            gal.children_with_tokens()
                .filter_map(|t| t.into_token())
                .filter(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| name_to_type(&t.text().to_string()))
                .collect::<Vec<_>>()
        })
        .collect();

    // Extract type name from the second PATH child (after `for`).
    let impl_type_name = paths
        .get(1)
        .and_then(|path| {
            path.children_with_tokens()
                .filter_map(|t| t.into_token())
                .find(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
        })
        .unwrap_or_else(|| "<unknown>".to_string());

    let impl_type = name_to_type(&impl_type_name);

    // Collect associated type bindings from the impl block.
    let mut assoc_types: FxHashMap<String, Ty> = FxHashMap::default();
    for binding in impl_.assoc_type_bindings() {
        if let Some(name) = binding.name().and_then(|n| n.text()) {
            if let Some(concrete_ty) = resolve_assoc_type_binding(&binding, type_registry) {
                assoc_types.insert(name, concrete_ty);
            }
        }
    }

    // Collect methods from the impl block.
    let mut impl_methods = FxHashMap::default();

    for method in impl_.methods() {
        let method_name = method
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let mut has_self = false;
        let mut param_count = 0;

        if let Some(param_list) = method.param_list() {
            for param in param_list.params() {
                let is_self = param
                    .syntax()
                    .children_with_tokens()
                    .any(|tok| {
                        tok.as_token()
                            .map(|t| t.kind() == SyntaxKind::SELF_KW)
                            .unwrap_or(false)
                    });
                if is_self {
                    has_self = true;
                } else {
                    param_count += 1;
                }
            }
        }

        let return_type = method.return_type().and_then(|ann| {
            // Try Self.Item resolution first, then fall back to standard type resolution.
            resolve_self_assoc_type(&ann, &assoc_types)
                .or_else(|| resolve_type_name(&ann))
        });

        impl_methods.insert(
            method_name.clone(),
            ImplMethodSig {
                has_self,
                param_count,
                return_type: return_type.clone(),
            },
        );

        // Also infer the method body to check it type-checks.
        env.push_scope();
        env.insert("self".into(), Scheme::mono(impl_type.clone()));

        // Collect all param types for fn_ty (self + non-self params).
        // Only prepend impl_type when method has self parameter (e.g., instance methods).
        // Static trait methods like From.from() do NOT take self.
        let mut all_param_tys: Vec<Ty> = if has_self {
            vec![impl_type.clone()]
        } else {
            vec![]
        };

        if let Some(param_list) = method.param_list() {
            for param in param_list.params() {
                let is_self = param
                    .syntax()
                    .children_with_tokens()
                    .any(|tok| {
                        tok.as_token()
                            .map(|t| t.kind() == SyntaxKind::SELF_KW)
                            .unwrap_or(false)
                    });
                if !is_self {
                    if let Some(name_tok) = param.name() {
                        let name_text = name_tok.text().to_string();
                        let param_ty = param
                            .type_annotation()
                            .and_then(|ann| {
                                // Try Self.Item resolution first for impl method params.
                                resolve_self_assoc_type(&ann, &assoc_types)
                                    .or_else(|| resolve_type_name(&ann))
                            })
                            .unwrap_or_else(|| ctx.fresh_var());
                        env.insert(name_text, Scheme::mono(param_ty.clone()));
                        all_param_tys.push(param_ty);
                    }
                }
            }
        }

        if let Some(body) = method.body() {
            match infer_block(
                ctx,
                env,
                &body,
                types,
                type_registry,
                &*trait_registry,
                fn_constraints,
            ) {
                Ok(body_ty) => {
                    if let Some(ref ret_ty) = return_type {
                        let _ = ctx.unify(body_ty, ret_ty.clone(), ConstraintOrigin::Builtin);
                    }
                }
                Err(_) => { /* error already recorded */ }
            }
        }

        env.pop_scope();

        // Register the method as a callable function so `to_string(42)` works.
        // Include all params (self + non-self) so the MIR lowerer can bind them.
        let fn_ty = {
            let ret = return_type.clone().unwrap_or_else(|| Ty::Tuple(vec![]));
            Ty::Fun(all_param_tys, Box::new(ret))
        };
        // Store the function type in the types map so the MIR lowerer can look it up.
        types.insert(method.syntax().text_range(), fn_ty.clone());
        if env.lookup(&method_name).is_none() {
            env.insert(method_name.clone(), Scheme::mono(fn_ty));
        }
    }

    // Register the impl and collect validation errors.
    let errors = trait_registry.register_impl(TraitImplDef {
        trait_name,
        trait_type_args,
        impl_type,
        impl_type_name,
        methods: impl_methods,
        associated_types: assoc_types,
    });

    ctx.errors.extend(errors);
}

/// Infer a let binding: `let x = expr`
fn infer_let_binding(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    let_: &LetBinding,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    ctx.enter_level();

    let init_expr = let_.initializer().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;

    let init_ty = infer_expr(ctx, env, &init_expr, types, type_registry, trait_registry, fn_constraints)?;

    // If there is a type annotation, resolve and unify with the inferred type.
    // When annotation is present and unification succeeds, use the annotation
    // type for the binding (the annotation declares the variable's type).
    let binding_ty = if let Some(annotation) = let_.type_annotation() {
        if let Some(ann_ty) = resolve_type_annotation(ctx, &annotation, type_registry) {
            let origin = ConstraintOrigin::Annotation {
                annotation_span: annotation.syntax().text_range(),
            };
            ctx.unify(init_ty.clone(), ann_ty.clone(), origin)?;
            ann_ty
        } else {
            init_ty.clone()
        }
    } else {
        init_ty.clone()
    };

    ctx.leave_level();
    let scheme = ctx.generalize(binding_ty);

    if let Some(name) = let_.name() {
        if let Some(name_text) = name.text() {
            // Propagate where-clause constraints if RHS is a NameRef
            // to a constrained function (fixes TSND-01 soundness bug).
            if let Expr::NameRef(ref name_ref) = init_expr {
                if let Some(source_name) = name_ref.text() {
                    if let Some(source_constraints) = fn_constraints.get(&source_name).cloned() {
                        fn_constraints.insert(name_text.clone(), source_constraints);
                    }
                }
            }
            env.insert(name_text, scheme);
        }
    } else if let Some(pat) = let_.pattern() {
        let pat_ty = infer_pattern(ctx, env, &pat, types, type_registry)?;
        ctx.unify(
            pat_ty,
            init_ty.clone(),
            ConstraintOrigin::LetBinding {
                binding_span: let_.syntax().text_range(),
            },
        )?;
    }

    let resolved = ctx.resolve(init_ty);
    types.insert(let_.syntax().text_range(), resolved.clone());

    Ok(resolved)
}

/// Infer a named function definition: `fn name(params) [-> RetType] [where T: Trait] do body end`
fn infer_fn_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    fn_: &FnDef,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let fn_name = fn_
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<anonymous>".to_string());

    ctx.enter_level();

    let self_var = ctx.fresh_var();
    env.insert(fn_name.clone(), Scheme::mono(self_var.clone()));

    // Extract generic type parameters if present.
    let mut type_params: FxHashMap<String, Ty> = FxHashMap::default();
    for child in fn_.syntax().children() {
        if child.kind() == SyntaxKind::GENERIC_PARAM_LIST {
            for tok in child.children_with_tokens() {
                if let Some(token) = tok.as_token() {
                    if token.kind() == SyntaxKind::IDENT {
                        let param_name = token.text().to_string();
                        let param_ty = ctx.fresh_var();
                        type_params.insert(param_name, param_ty);
                    }
                }
            }
        }
    }

    // Extract where-clause constraints.
    let where_constraints = extract_where_constraints(fn_);

    env.push_scope();

    // Insert type params into the scope.
    for (name, ty) in &type_params {
        env.insert(name.clone(), Scheme::mono(ty.clone()));
    }

    let mut param_types = Vec::new();
    let mut param_type_param_names: Vec<Option<String>> = Vec::new();

    if let Some(param_list) = fn_.param_list() {
        for param in param_list.params() {
            let (param_ty, tp_name) = if let Some(ann) = param.type_annotation() {
                if let Some(type_name) = resolve_type_name_str(&ann) {
                    if let Some(tp_ty) = type_params.get(&type_name) {
                        (tp_ty.clone(), Some(type_name))
                    } else {
                        // Try full annotation resolution first (handles generic args
                        // like List<String>, Map<String, String>, Result<T, E>).
                        // Fall back to simple name_to_type if that fails.
                        if let Some(full_ty) = resolve_type_annotation(ctx, &ann, type_registry) {
                            (full_ty, None)
                        } else {
                            (name_to_type(&type_name), None)
                        }
                    }
                } else {
                    // No simple type name -- try full annotation resolution.
                    if let Some(full_ty) = resolve_type_annotation(ctx, &ann, type_registry) {
                        (full_ty, None)
                    } else {
                        (ctx.fresh_var(), None)
                    }
                }
            } else {
                (ctx.fresh_var(), None)
            };

            if let Some(name_tok) = param.name() {
                let name_text = name_tok.text().to_string();
                env.insert(name_text, Scheme::mono(param_ty.clone()));
            }
            param_types.push(param_ty);
            param_type_param_names.push(tp_name);
        }
    }

    if !where_constraints.is_empty() || !type_params.is_empty() {
        fn_constraints.insert(
            fn_name.clone(),
            FnConstraints {
                where_constraints: where_constraints.clone(),
                type_params: type_params.clone(),
                param_type_param_names,
            },
        );
    }

    // Parse return type annotation.
    // Use resolve_type_annotation which handles generic args and sugar types
    // (e.g., Result<Int, String>, Int!String, Int?) correctly.
    let return_type_annotation = fn_.return_type().and_then(|ann| {
        // First check if it's a type parameter name.
        if let Some(type_name) = resolve_type_name_str(&ann) {
            if let Some(tp_ty) = type_params.get(&type_name) {
                return Some(tp_ty.clone());
            }
        }
        // Use full annotation resolution for generic/sugar types.
        resolve_type_annotation(ctx, &ann, type_registry)
            .or_else(|| {
                resolve_type_name_str(&ann).map(|name| name_to_type(&name))
            })
    });

    ctx.push_fn_return_type(return_type_annotation.clone());
    let body_ty = if let Some(body) = fn_.body() {
        infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
    } else {
        Ty::Tuple(vec![])
    };
    ctx.pop_fn_return_type();

    if let Some(ref ret_ann) = return_type_annotation {
        ctx.unify(body_ty.clone(), ret_ann.clone(), ConstraintOrigin::Builtin)?;
    }

    env.pop_scope();

    let ret_ty = return_type_annotation.unwrap_or(body_ty);
    let fn_ty = Ty::Fun(param_types, Box::new(ret_ty));

    ctx.unify(self_var, fn_ty.clone(), ConstraintOrigin::Builtin)?;

    ctx.leave_level();
    let scheme = ctx.generalize(fn_ty.clone());

    env.insert(fn_name, scheme);

    let resolved = ctx.resolve(fn_ty);
    types.insert(fn_.syntax().text_range(), resolved.clone());

    Ok(resolved)
}

// ── Expression Inference ───────────────────────────────────────────────

/// Infer the type of an expression.
fn infer_expr(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    expr: &Expr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let ty = match expr {
        Expr::Literal(lit) => infer_literal(lit),
        Expr::NameRef(name_ref) => infer_name_ref(ctx, env, name_ref)?,
        Expr::BinaryExpr(bin) => {
            infer_binary(ctx, env, bin, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::UnaryExpr(un) => {
            infer_unary(ctx, env, un, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::CallExpr(call) => {
            infer_call(ctx, env, call, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::PipeExpr(pipe) => {
            infer_pipe(ctx, env, pipe, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::IfExpr(if_) => {
            infer_if(ctx, env, if_, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::ClosureExpr(closure) => {
            infer_closure(ctx, env, closure, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::Block(block) => {
            infer_block(ctx, env, block, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::TupleExpr(tuple) => {
            infer_tuple(ctx, env, tuple, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::CaseExpr(case) => {
            infer_case(ctx, env, case, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::ReturnExpr(ret) => {
            infer_return(ctx, env, ret, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::StringExpr(se) => {
            // Recurse into interpolation expressions so their types are recorded.
            for child in se.syntax().children() {
                if child.kind() == SyntaxKind::INTERPOLATION {
                    for inner in child.children() {
                        if let Some(inner_expr) = Expr::cast(inner) {
                            let _ = infer_expr(
                                ctx,
                                env,
                                &inner_expr,
                                types,
                                type_registry,
                                trait_registry,
                                fn_constraints,
                            );
                        }
                    }
                }
            }
            Ty::string()
        }
        Expr::FieldAccess(fa) => {
            infer_field_access(ctx, env, fa, types, type_registry, trait_registry, fn_constraints, false)?
        }
        Expr::StructLiteral(sl) => {
            infer_struct_literal(ctx, env, sl, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::MapLiteral(map_lit) => {
            infer_map_literal(ctx, env, map_lit, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::ListLiteral(lit) => {
            infer_list_literal(ctx, env, &lit, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::IndexExpr(_) => ctx.fresh_var(),
        // Loop expressions.
        Expr::WhileExpr(w) => {
            infer_while(ctx, env, w, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::BreakExpr(b) => {
            infer_break(ctx, b)?
        }
        Expr::ContinueExpr(c) => {
            infer_continue(ctx, c)?
        }
        Expr::ForInExpr(for_in) => {
            infer_for_in(ctx, env, &for_in, types, type_registry, trait_registry, fn_constraints)?
        }
        // Actor expressions.
        Expr::SpawnExpr(spawn) => {
            infer_spawn(ctx, env, spawn, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::SendExpr(send) => {
            infer_send(ctx, env, send, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::ReceiveExpr(recv) => {
            infer_receive(ctx, env, recv, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::SelfExpr(self_expr) => {
            infer_self_expr(ctx, env, self_expr)?
        }
        Expr::LinkExpr(link) => {
            infer_link(ctx, env, link, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::TryExpr(try_expr) => {
            infer_try_expr(ctx, env, try_expr, types, type_registry, trait_registry, fn_constraints)?
        }
        Expr::AtomLiteral(_atom) => {
            // Atoms have a distinct type from String -- they are opaque typed values.
            Ty::Con(TyCon::new("Atom"))
        }
        Expr::StructUpdate(update) => {
            infer_struct_update(ctx, env, update, types, type_registry, trait_registry, fn_constraints)?
        }
    };

    let resolved = ctx.resolve(ty.clone());
    types.insert(expr.syntax().text_range(), resolved.clone());

    Ok(ty)
}

/// Infer the type of a literal expression.
fn infer_literal(lit: &Literal) -> Ty {
    if let Some(token) = lit.token() {
        match token.kind() {
            SyntaxKind::INT_LITERAL => Ty::int(),
            SyntaxKind::FLOAT_LITERAL => Ty::float(),
            SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW => Ty::bool(),
            SyntaxKind::NIL_KW => Ty::Tuple(vec![]),
            SyntaxKind::STRING_START => Ty::string(),
            _ => Ty::Tuple(vec![]),
        }
    } else {
        Ty::Tuple(vec![])
    }
}

/// Infer the type of a name reference (variable lookup).
fn infer_name_ref(
    ctx: &mut InferCtx,
    env: &TypeEnv,
    name_ref: &NameRef,
) -> Result<Ty, TypeError> {
    let name = name_ref
        .text()
        .unwrap_or_else(|| "<unknown>".to_string());

    match env.lookup(&name) {
        Some(scheme) => Ok(ctx.instantiate(scheme)),
        None => {
            let err = TypeError::UnboundVariable {
                name,
                span: name_ref.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            Err(err)
        }
    }
}

/// Infer the type of a binary expression with trait-based operator dispatch.
fn infer_binary(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    bin: &BinaryExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let lhs_expr = bin.lhs().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;
    let rhs_expr = bin.rhs().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;

    let lhs_ty = infer_expr(ctx, env, &lhs_expr, types, type_registry, trait_registry, fn_constraints)?;
    let rhs_ty = infer_expr(ctx, env, &rhs_expr, types, type_registry, trait_registry, fn_constraints)?;

    let op = bin.op();
    let op_kind = op.as_ref().map(|t| t.kind());

    let origin = ConstraintOrigin::BinOp {
        op_span: bin.syntax().text_range(),
    };

    match op_kind {
        // Arithmetic: dispatch via compiler-known traits
        Some(SyntaxKind::PLUS) => {
            infer_trait_binary_op(ctx, "Add", &lhs_ty, &rhs_ty, trait_registry, &origin)
        }
        Some(SyntaxKind::MINUS) => {
            infer_trait_binary_op(ctx, "Sub", &lhs_ty, &rhs_ty, trait_registry, &origin)
        }
        Some(SyntaxKind::STAR) => {
            infer_trait_binary_op(ctx, "Mul", &lhs_ty, &rhs_ty, trait_registry, &origin)
        }
        Some(SyntaxKind::SLASH) => {
            infer_trait_binary_op(ctx, "Div", &lhs_ty, &rhs_ty, trait_registry, &origin)
        }
        Some(SyntaxKind::PERCENT) => {
            infer_trait_binary_op(ctx, "Mod", &lhs_ty, &rhs_ty, trait_registry, &origin)
        }

        // Equality: dispatch via Eq trait, return Bool
        Some(SyntaxKind::EQ_EQ | SyntaxKind::NOT_EQ) => {
            ctx.unify(lhs_ty.clone(), rhs_ty, origin.clone())?;
            let resolved = ctx.resolve(lhs_ty);
            if !is_type_var(&resolved) && !trait_registry.has_impl("Eq", &resolved) {
                let err = TypeError::TraitNotSatisfied {
                    ty: resolved,
                    trait_name: "Eq".to_string(),
                    origin,
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }
            Ok(Ty::bool())
        }

        // Ordering: dispatch via Ord trait, return Bool
        Some(SyntaxKind::LT | SyntaxKind::GT | SyntaxKind::LT_EQ | SyntaxKind::GT_EQ) => {
            ctx.unify(lhs_ty.clone(), rhs_ty, origin.clone())?;
            let resolved = ctx.resolve(lhs_ty);
            if !is_type_var(&resolved) && !trait_registry.has_impl("Ord", &resolved) {
                let err = TypeError::TraitNotSatisfied {
                    ty: resolved,
                    trait_name: "Ord".to_string(),
                    origin,
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }
            Ok(Ty::bool())
        }

        // Logical: unify both sides with Bool, return Bool
        Some(
            SyntaxKind::AND_KW | SyntaxKind::OR_KW | SyntaxKind::AMP_AMP | SyntaxKind::PIPE_PIPE,
        ) => {
            ctx.unify(lhs_ty, Ty::bool(), origin.clone())?;
            ctx.unify(rhs_ty, Ty::bool(), origin)?;
            Ok(Ty::bool())
        }

        // Concatenation operators: unify both sides, return same type
        Some(SyntaxKind::DIAMOND | SyntaxKind::PLUS_PLUS) => {
            ctx.unify(lhs_ty.clone(), rhs_ty, origin)?;
            Ok(lhs_ty)
        }

        // Unknown op: return a fresh variable
        _ => {
            let result = ctx.fresh_var();
            Ok(result)
        }
    }
}

/// Infer a binary operator using trait dispatch.
fn infer_trait_binary_op(
    ctx: &mut InferCtx,
    trait_name: &str,
    lhs_ty: &Ty,
    rhs_ty: &Ty,
    trait_registry: &TraitRegistry,
    origin: &ConstraintOrigin,
) -> Result<Ty, TypeError> {
    ctx.unify(lhs_ty.clone(), rhs_ty.clone(), origin.clone())?;

    let resolved = ctx.resolve(lhs_ty.clone());

    if is_type_var(&resolved) {
        return Ok(resolved);
    }

    if trait_registry.has_impl(trait_name, &resolved) {
        // Resolve the Output associated type for the result type.
        // For Int + Int, Output = Int; for Float * Float, Output = Float.
        // Falls back to the operand type if no Output is defined (backward compat).
        if let Some(output_ty) = trait_registry.resolve_associated_type(trait_name, "Output", &resolved) {
            Ok(output_ty)
        } else {
            Ok(resolved)
        }
    } else {
        let err = TypeError::TraitNotSatisfied {
            ty: resolved,
            trait_name: trait_name.to_string(),
            origin: origin.clone(),
        };
        ctx.errors.push(err.clone());
        Err(err)
    }
}

/// Infer the type of a unary expression.
fn infer_unary(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    un: &UnaryExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let operand = un.operand().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;

    let operand_ty = infer_expr(ctx, env, &operand, types, type_registry, trait_registry, fn_constraints)?;

    let op_kind = un.op().map(|t| t.kind());

    match op_kind {
        Some(SyntaxKind::MINUS) => {
            let resolved = ctx.resolve(operand_ty.clone());

            // Deferred: type variable, return as-is.
            if is_type_var(&resolved) {
                return Ok(resolved);
            }

            // Fast path: primitive Int/Float -- no trait check needed.
            if matches!(&resolved, Ty::Con(ref tc) if tc.name == "Int" || tc.name == "Float") {
                return Ok(resolved);
            }

            // User type: check Neg trait.
            if trait_registry.has_impl("Neg", &resolved) {
                if let Some(output_ty) = trait_registry.resolve_associated_type("Neg", "Output", &resolved) {
                    Ok(output_ty)
                } else {
                    Ok(resolved)
                }
            } else {
                let err = TypeError::TraitNotSatisfied {
                    ty: resolved,
                    trait_name: "Neg".to_string(),
                    origin: ConstraintOrigin::Builtin,
                };
                ctx.errors.push(err.clone());
                Err(err)
            }
        }
        Some(SyntaxKind::BANG | SyntaxKind::NOT_KW) => {
            ctx.unify(operand_ty, Ty::bool(), ConstraintOrigin::Builtin)?;
            Ok(Ty::bool())
        }
        _ => Ok(operand_ty),
    }
}

/// Infer the type of a function call expression with where-clause enforcement.
fn infer_call(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    call: &CallExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let callee_expr = call.callee().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;

    // Try normal callee inference first. For FieldAccess callees, this goes through
    // the standard infer_field_access path (modules, services, variants, struct fields).
    // If that fails AND the callee is a FieldAccess, retry in method-call context.
    let callee_ty = match infer_expr(ctx, env, &callee_expr, types, type_registry, trait_registry, fn_constraints) {
        Ok(ty) => ty,
        Err(first_err) => {
            // If callee is a FieldAccess and normal inference failed, try method resolution.
            if let Expr::FieldAccess(ref fa) = callee_expr {
                // Remove the error that was pushed during the failed attempt.
                // The failed infer_field_access(false) pushed a NoSuchField error.
                if let Some(pos) = ctx.errors.iter().rposition(|e| {
                    matches!(e, TypeError::NoSuchField { .. })
                }) {
                    ctx.errors.remove(pos);
                }

                // Try method-call context.
                match infer_field_access(ctx, env, fa, types, type_registry, trait_registry, fn_constraints, true) {
                    Ok(callee_ty) => {
                        // Method resolved! Build expected function type with receiver.
                        let mut arg_types = Vec::new();
                        // Infer the receiver type (base expression of the FieldAccess).
                        if let Some(base) = fa.base() {
                            let receiver_ty = infer_expr(ctx, env, &base, types, type_registry, trait_registry, fn_constraints)?;
                            arg_types.push(receiver_ty);
                        }
                        // Infer explicit argument types.
                        if let Some(arg_list) = call.arg_list() {
                            for arg in arg_list.args() {
                                let arg_ty = infer_expr(ctx, env, &arg, types, type_registry, trait_registry, fn_constraints)?;
                                arg_types.push(arg_ty);
                            }
                        }

                        let ret_var = ctx.fresh_var();
                        let expected_fn_ty = Ty::Fun(arg_types, Box::new(ret_var.clone()));

                        let origin = ConstraintOrigin::FnArg {
                            call_site: call.syntax().text_range(),
                            param_idx: 0,
                        };
                        ctx.unify(callee_ty, expected_fn_ty, origin)?;

                        let result = ctx.resolve(ret_var);
                        types.insert(call.syntax().text_range(), result.clone());
                        return Ok(result);
                    }
                    Err(method_err) => return Err(method_err),
                }
            } else {
                return Err(first_err);
            }
        }
    };

    let mut arg_types = Vec::new();
    if let Some(arg_list) = call.arg_list() {
        for arg in arg_list.args() {
            let arg_ty = infer_expr(ctx, env, &arg, types, type_registry, trait_registry, fn_constraints)?;
            arg_types.push(arg_ty);
        }
    }

    let ret_var = ctx.fresh_var();
    let expected_fn_ty = Ty::Fun(arg_types.clone(), Box::new(ret_var.clone()));

    let origin = ConstraintOrigin::FnArg {
        call_site: call.syntax().text_range(),
        param_idx: 0,
    };
    ctx.unify(callee_ty, expected_fn_ty, origin.clone())?;

    // Check where-clause constraints at the call site.
    // After unification, arg_types hold the resolved concrete types for each
    // parameter. Use param_type_param_names to map from arg position back to
    // type parameter name, then check trait constraints on the resolved types.
    if let Expr::NameRef(name_ref) = &callee_expr {
        if let Some(fn_name) = name_ref.text() {
            if let Some(constraints) = fn_constraints.get(&fn_name) {
                if !constraints.where_constraints.is_empty() {
                    let mut resolved_type_args: FxHashMap<String, Ty> = FxHashMap::default();

                    // Build type param -> resolved type mapping from call-site args.
                    for (i, tp_name_opt) in constraints.param_type_param_names.iter().enumerate() {
                        if let Some(tp_name) = tp_name_opt {
                            if i < arg_types.len() {
                                let resolved = ctx.resolve(arg_types[i].clone());
                                resolved_type_args.insert(tp_name.clone(), resolved);
                            }
                        }
                    }

                    // Fallback: also try definition-time vars (may work for non-generic cases).
                    for (param_name, param_ty) in &constraints.type_params {
                        if !resolved_type_args.contains_key(param_name) {
                            let resolved = ctx.resolve(param_ty.clone());
                            resolved_type_args.insert(param_name.clone(), resolved);
                        }
                    }

                    let errors = trait_registry.check_where_constraints(
                        &constraints.where_constraints,
                        &resolved_type_args,
                        origin.clone(),
                    );
                    ctx.errors.extend(errors.clone());

                    if let Some(first_err) = errors.into_iter().next() {
                        return Err(first_err);
                    }
                }
            }
        }
    }

    // Check constraints on function-typed ARGUMENTS (higher-order case).
    // When apply(show, 42) is called, show has fn_constraints but f inside
    // apply's body does not. Check show's constraints HERE at the outer call site
    // where unification has connected type variables to concrete argument types.
    if let Some(arg_list) = call.arg_list() {
        for (arg_idx, arg) in arg_list.args().enumerate() {
            if let Expr::NameRef(ref name_ref) = arg {
                if let Some(arg_fn_name) = name_ref.text() {
                    if let Some(arg_constraints) = fn_constraints.get(&arg_fn_name) {
                        if !arg_constraints.where_constraints.is_empty() && arg_idx < arg_types.len() {
                            // Resolve the argument's type (a function type after unification)
                            let resolved_arg_ty = ctx.resolve(arg_types[arg_idx].clone());

                            if let Ty::Fun(ref param_tys, _) = resolved_arg_ty {
                                let mut resolved_type_args: FxHashMap<String, Ty> = FxHashMap::default();

                                // Map parameter positions to type param names, resolve types
                                for (j, tp_name_opt) in arg_constraints.param_type_param_names.iter().enumerate() {
                                    if let Some(tp_name) = tp_name_opt {
                                        if j < param_tys.len() {
                                            let resolved = ctx.resolve(param_tys[j].clone());
                                            resolved_type_args.insert(tp_name.clone(), resolved);
                                        }
                                    }
                                }

                                // Fallback: definition-time vars (may be connected via unification)
                                for (param_name, param_ty) in &arg_constraints.type_params {
                                    if !resolved_type_args.contains_key(param_name) {
                                        let resolved = ctx.resolve(param_ty.clone());
                                        resolved_type_args.insert(param_name.clone(), resolved);
                                    }
                                }

                                // Only check constraints where type resolved to concrete (not Ty::Var)
                                let checkable: Vec<(String, String)> = arg_constraints.where_constraints
                                    .iter()
                                    .filter(|(pn, _)| {
                                        resolved_type_args.get(pn)
                                            .map(|ty| !matches!(ty, Ty::Var(_)))
                                            .unwrap_or(false)
                                    })
                                    .cloned()
                                    .collect();

                                if !checkable.is_empty() {
                                    let errors = trait_registry.check_where_constraints(
                                        &checkable,
                                        &resolved_type_args,
                                        origin.clone(),
                                    );
                                    ctx.errors.extend(errors);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ret_var)
}

/// Infer the type of a pipe expression: `lhs |> rhs`
fn infer_pipe(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    pipe: &PipeExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let lhs = pipe.lhs().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;
    let rhs = pipe.rhs().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;

    let lhs_ty = infer_expr(ctx, env, &lhs, types, type_registry, trait_registry, fn_constraints)?;

    let ret_var = ctx.fresh_var();

    match &rhs {
        Expr::CallExpr(call) => {
            // Pipe-aware call inference: `x |> f(a, b)` desugars to `f(x, a, b)`.
            // We infer the callee and explicit args separately, then prepend lhs_ty
            // to construct the full expected function type -- matching MIR lowering.
            let callee_expr = call.callee().ok_or_else(|| {
                let err = TypeError::Mismatch {
                    expected: Ty::Never,
                    found: Ty::Never,
                    origin: ConstraintOrigin::Builtin,
                };
                ctx.errors.push(err.clone());
                err
            })?;

            let callee_ty = infer_expr(ctx, env, &callee_expr, types, type_registry, trait_registry, fn_constraints)?;

            // Infer explicit argument types from the call's arg list.
            let mut arg_types = Vec::new();
            if let Some(arg_list) = call.arg_list() {
                for arg in arg_list.args() {
                    let arg_ty = infer_expr(ctx, env, &arg, types, type_registry, trait_registry, fn_constraints)?;
                    arg_types.push(arg_ty);
                }
            }

            // Build full arg list: [lhs_ty, ...explicit_arg_types]
            let mut full_args = vec![lhs_ty];
            full_args.extend(arg_types.clone());

            let expected_fn_ty = Ty::Fun(full_args.clone(), Box::new(ret_var.clone()));

            let origin = ConstraintOrigin::FnArg {
                call_site: call.syntax().text_range(),
                param_idx: 0,
            };
            ctx.unify(callee_ty, expected_fn_ty, origin.clone())?;

            // Record type for the CallExpr node so MIR lowering can resolve it.
            let resolved_call = ctx.resolve(ret_var.clone());
            types.insert(call.syntax().text_range(), resolved_call);

            // Check where-clause constraints at the call site (mirrors infer_call).
            if let Expr::NameRef(name_ref) = &callee_expr {
                if let Some(fn_name) = name_ref.text() {
                    if let Some(constraints) = fn_constraints.get(&fn_name) {
                        if !constraints.where_constraints.is_empty() {
                            let mut resolved_type_args: FxHashMap<String, Ty> = FxHashMap::default();

                            // Build type param -> resolved type mapping from full arg list
                            // (including the piped argument at position 0).
                            for (i, tp_name_opt) in constraints.param_type_param_names.iter().enumerate() {
                                if let Some(tp_name) = tp_name_opt {
                                    if i < full_args.len() {
                                        let resolved = ctx.resolve(full_args[i].clone());
                                        resolved_type_args.insert(tp_name.clone(), resolved);
                                    }
                                }
                            }

                            // Fallback: definition-time vars.
                            for (param_name, param_ty) in &constraints.type_params {
                                if !resolved_type_args.contains_key(param_name) {
                                    let resolved = ctx.resolve(param_ty.clone());
                                    resolved_type_args.insert(param_name.clone(), resolved);
                                }
                            }

                            let errors = trait_registry.check_where_constraints(
                                &constraints.where_constraints,
                                &resolved_type_args,
                                origin.clone(),
                            );
                            ctx.errors.extend(errors.clone());

                            if let Some(first_err) = errors.into_iter().next() {
                                return Err(first_err);
                            }
                        }
                    }
                }
            }

            // Check constraints on function-typed ARGUMENTS (higher-order case, pipe variant).
            // For `value |> apply(show)`, the explicit args are [show] in arg_types.
            // The piped lhs is NOT in arg_list.args(), so we iterate only explicit args.
            if let Some(arg_list) = call.arg_list() {
                for (arg_idx, arg) in arg_list.args().enumerate() {
                    if let Expr::NameRef(ref name_ref) = arg {
                        if let Some(arg_fn_name) = name_ref.text() {
                            if let Some(arg_constraints) = fn_constraints.get(&arg_fn_name) {
                                if !arg_constraints.where_constraints.is_empty() && arg_idx < arg_types.len() {
                                    let resolved_arg_ty = ctx.resolve(arg_types[arg_idx].clone());

                                    if let Ty::Fun(ref param_tys, _) = resolved_arg_ty {
                                        let mut resolved_type_args: FxHashMap<String, Ty> = FxHashMap::default();

                                        for (j, tp_name_opt) in arg_constraints.param_type_param_names.iter().enumerate() {
                                            if let Some(tp_name) = tp_name_opt {
                                                if j < param_tys.len() {
                                                    let resolved = ctx.resolve(param_tys[j].clone());
                                                    resolved_type_args.insert(tp_name.clone(), resolved);
                                                }
                                            }
                                        }

                                        for (param_name, param_ty) in &arg_constraints.type_params {
                                            if !resolved_type_args.contains_key(param_name) {
                                                let resolved = ctx.resolve(param_ty.clone());
                                                resolved_type_args.insert(param_name.clone(), resolved);
                                            }
                                        }

                                        let checkable: Vec<(String, String)> = arg_constraints.where_constraints
                                            .iter()
                                            .filter(|(pn, _)| {
                                                resolved_type_args.get(pn)
                                                    .map(|ty| !matches!(ty, Ty::Var(_)))
                                                    .unwrap_or(false)
                                            })
                                            .cloned()
                                            .collect();

                                        if !checkable.is_empty() {
                                            let errors = trait_registry.check_where_constraints(
                                                &checkable,
                                                &resolved_type_args,
                                                origin.clone(),
                                            );
                                            ctx.errors.extend(errors);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {
            // Existing behavior: infer rhs as function, unify with Fun([lhs_ty], ret).
            let rhs_ty = infer_expr(ctx, env, &rhs, types, type_registry, trait_registry, fn_constraints)?;
            let expected_fn = Ty::Fun(vec![lhs_ty], Box::new(ret_var.clone()));
            ctx.unify(rhs_ty, expected_fn, ConstraintOrigin::Builtin)?;
        }
    }

    Ok(ret_var)
}

/// Infer the type of an if expression.
fn infer_if(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    if_: &IfExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    if let Some(cond) = if_.condition() {
        let cond_ty = infer_expr(ctx, env, &cond, types, type_registry, trait_registry, fn_constraints)?;
        ctx.unify(cond_ty, Ty::bool(), ConstraintOrigin::Builtin)?;
    }

    let then_ty = if let Some(then_block) = if_.then_branch() {
        infer_block(ctx, env, &then_block, types, type_registry, trait_registry, fn_constraints)?
    } else {
        Ty::Tuple(vec![])
    };

    if let Some(else_branch) = if_.else_branch() {
        let else_ty = if let Some(else_if) = else_branch.if_expr() {
            infer_if(ctx, env, &else_if, types, type_registry, trait_registry, fn_constraints)?
        } else if let Some(else_block) = else_branch.block() {
            infer_block(ctx, env, &else_block, types, type_registry, trait_registry, fn_constraints)?
        } else {
            Ty::Tuple(vec![])
        };

        let origin = ConstraintOrigin::IfBranches {
            if_span: if_.syntax().text_range(),
            then_span: if_
                .then_branch()
                .map(|b| b.syntax().text_range())
                .unwrap_or_else(|| if_.syntax().text_range()),
            else_span: else_branch.syntax().text_range(),
        };
        ctx.unify(then_ty.clone(), else_ty, origin)?;

        Ok(then_ty)
    } else {
        Ok(then_ty)
    }
}

/// Infer the type of a while expression: `while cond do body end`
///
/// The condition must be Bool. The body is inferred with loop_depth incremented.
/// While expressions always return Unit (WHILE-03).
fn infer_while(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    w: &WhileExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    // Infer and unify condition with Bool.
    if let Some(cond) = w.condition() {
        let cond_ty = infer_expr(ctx, env, &cond, types, type_registry, trait_registry, fn_constraints)?;
        ctx.unify(cond_ty, Ty::bool(), ConstraintOrigin::Builtin)?;
    }

    // Infer body with incremented loop_depth.
    ctx.enter_loop();
    if let Some(body) = w.body() {
        let _ = infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?;
    }
    ctx.exit_loop();

    // While always returns Unit regardless of body type (WHILE-03).
    Ok(Ty::Tuple(vec![]))
}

/// Infer the type of a break expression.
///
/// Break is a control flow transfer -- type is Never.
/// Produces BreakOutsideLoop error if not inside a loop (BRKC-04).
fn infer_break(
    ctx: &mut InferCtx,
    b: &BreakExpr,
) -> Result<Ty, TypeError> {
    if !ctx.in_loop() {
        let span = b.syntax().text_range();
        ctx.errors.push(TypeError::BreakOutsideLoop { span });
    }
    Ok(Ty::Never)
}

/// Infer the type of a continue expression.
///
/// Continue is a control flow transfer -- type is Never.
/// Produces ContinueOutsideLoop error if not inside a loop (BRKC-04).
fn infer_continue(
    ctx: &mut InferCtx,
    c: &ContinueExpr,
) -> Result<Ty, TypeError> {
    if !ctx.in_loop() {
        let span = c.syntax().text_range();
        ctx.errors.push(TypeError::ContinueOutsideLoop { span });
    }
    Ok(Ty::Never)
}

/// Infer the type of a for-in expression: `for i in 0..10 do body end`
///
/// Handles range (DotDot), List<T>, Map<K,V>, and Set<T> iterables.
/// Binds loop variable(s) to the element type(s).
/// Returns List<body_ty> (comprehension semantics).
fn infer_for_in(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    for_in: &ForInExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    // Infer the iterable expression.
    let iter_ty = if let Some(iterable) = for_in.iterable() {
        let ty = infer_expr(ctx, env, &iterable, types, type_registry, trait_registry, fn_constraints)?;
        ctx.resolve(ty)
    } else {
        return Ok(Ty::Tuple(vec![]));
    };

    // Check if iterable is a DotDot range.
    let is_range = if let Some(iterable) = for_in.iterable() {
        if let Expr::BinaryExpr(ref bin) = iterable {
            let is_dot_dot = bin.op().map(|t| t.kind()) == Some(SyntaxKind::DOT_DOT);
            if is_dot_dot {
                let origin = ConstraintOrigin::BinOp {
                    op_span: bin.syntax().text_range(),
                };
                if let Some(lhs) = bin.lhs() {
                    if let Some(lhs_ty) = types.get(&lhs.syntax().text_range()) {
                        ctx.unify(lhs_ty.clone(), Ty::int(), origin.clone())?;
                    }
                }
                if let Some(rhs) = bin.rhs() {
                    if let Some(rhs_ty) = types.get(&rhs.syntax().text_range()) {
                        ctx.unify(rhs_ty.clone(), Ty::int(), origin)?;
                    }
                }
            }
            is_dot_dot
        } else {
            false
        }
    } else {
        false
    };

    // Push a new scope for the loop variable(s).
    env.push_scope();

    if is_range {
        // Range iteration: bind loop variable as Int.
        let var_name = for_in
            .binding_name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());
        env.insert(var_name, Scheme::mono(Ty::int()));
    } else {
        // Collection iteration: detect type and bind accordingly.
        match extract_collection_elem_type(&iter_ty) {
            CollectionType::List(elem_ty) => {
                let var_name = for_in
                    .binding_name()
                    .and_then(|n| n.text())
                    .unwrap_or_else(|| "_".to_string());
                env.insert(var_name, Scheme::mono(elem_ty));
            }
            CollectionType::Map(key_ty, val_ty) => {
                // Expect destructuring binding {k, v}.
                if let Some(destr) = for_in.destructure_binding() {
                    let names = destr.names();
                    if names.len() >= 1 {
                        let k_name = names[0].text().unwrap_or_else(|| "_".to_string());
                        env.insert(k_name, Scheme::mono(key_ty));
                    }
                    if names.len() >= 2 {
                        let v_name = names[1].text().unwrap_or_else(|| "_".to_string());
                        env.insert(v_name, Scheme::mono(val_ty));
                    }
                } else {
                    // Single binding over map: bind as Int (key) as fallback.
                    let var_name = for_in
                        .binding_name()
                        .and_then(|n| n.text())
                        .unwrap_or_else(|| "_".to_string());
                    env.insert(var_name, Scheme::mono(key_ty));
                }
            }
            CollectionType::Set(elem_ty) => {
                let var_name = for_in
                    .binding_name()
                    .and_then(|n| n.text())
                    .unwrap_or_else(|| "_".to_string());
                env.insert(var_name, Scheme::mono(elem_ty));
            }
            CollectionType::Unknown => {
                let var_name = for_in
                    .binding_name()
                    .and_then(|n| n.text())
                    .unwrap_or_else(|| "_".to_string());

                // Check if the type implements Iterable (collection -> iterator).
                if trait_registry.has_impl("Iterable", &iter_ty) {
                    if let Some(item_ty) = trait_registry.resolve_associated_type("Iterable", "Item", &iter_ty) {
                        env.insert(var_name, Scheme::mono(item_ty));
                    } else {
                        // Iterable impl exists but no Item type resolved -- fallback to Int
                        env.insert(var_name, Scheme::mono(Ty::int()));
                    }
                }
                // Check if the type directly implements Iterator (type IS an iterator).
                else if trait_registry.has_impl("Iterator", &iter_ty) {
                    if let Some(item_ty) = trait_registry.resolve_associated_type("Iterator", "Item", &iter_ty) {
                        env.insert(var_name, Scheme::mono(item_ty));
                    } else {
                        env.insert(var_name, Scheme::mono(Ty::int()));
                    }
                }
                // True fallback: bind as Int (existing behavior).
                else {
                    env.insert(var_name, Scheme::mono(Ty::int()));
                }
            }
        }
    }

    // Enter loop context (enables break/continue validation).
    ctx.enter_loop();

    // Infer filter condition if present (FILT-01).
    if let Some(filter_expr) = for_in.filter() {
        let filter_ty = infer_expr(
            ctx,
            env,
            &filter_expr,
            types,
            type_registry,
            trait_registry,
            fn_constraints,
        )?;
        let origin = ConstraintOrigin::BinOp {
            op_span: filter_expr.syntax().text_range(),
        };
        ctx.unify(filter_ty, Ty::bool(), origin)?;
    }

    // Infer body -- its type becomes the List element type.
    let body_ty = if let Some(body) = for_in.body() {
        infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
    } else {
        Ty::Tuple(vec![])
    };

    ctx.exit_loop();

    // Pop the loop variable scope.
    env.pop_scope();

    // For-in returns List<body_ty> (comprehension semantics).
    Ok(Ty::list(body_ty))
}

/// Helper enum for classifying collection iterable types.
enum CollectionType {
    List(Ty),
    Map(Ty, Ty),
    Set(Ty),
    Unknown,
}

/// Extract the element type from a collection type (List<T>, Map<K,V>, Set<T>).
fn extract_collection_elem_type(ty: &Ty) -> CollectionType {
    match ty {
        Ty::App(con, args) => {
            if let Ty::Con(ref tc) = **con {
                match tc.name.as_str() {
                    "List" if !args.is_empty() => CollectionType::List(args[0].clone()),
                    "Map" if args.len() >= 2 => CollectionType::Map(args[0].clone(), args[1].clone()),
                    "Set" if !args.is_empty() => CollectionType::Set(args[0].clone()),
                    _ => CollectionType::Unknown,
                }
            } else {
                CollectionType::Unknown
            }
        }
        Ty::Con(tc) => match tc.name.as_str() {
            "List" => CollectionType::List(Ty::int()),
            "Map" => CollectionType::Map(Ty::int(), Ty::int()),
            "Set" => CollectionType::Set(Ty::int()),
            _ => CollectionType::Unknown,
        },
        _ => CollectionType::Unknown,
    }
}

/// Infer the type of a closure expression: `fn (params) -> body end`
///
/// Handles three forms:
/// 1. Single-clause arrow: `fn x -> expr end` or `fn(x) -> expr end`
/// 2. Single-clause do/end: `fn x do stmts end`
/// 3. Multi-clause: `fn 0 -> "zero" | n -> to_string(n) end`
fn infer_closure(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    closure: &ClosureExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    // Check if this is a multi-clause closure.
    if closure.is_multi_clause() {
        return infer_multi_clause_closure(
            ctx, env, closure, types, type_registry, trait_registry, fn_constraints,
        );
    }

    // Single-clause closure: existing path.
    env.push_scope();

    let mut param_types = Vec::new();

    if let Some(param_list) = closure.param_list() {
        for param in param_list.params() {
            let param_ty = if let Some(ann) = param.type_annotation() {
                if let Some(type_name) = resolve_type_name_str(&ann) {
                    name_to_type(&type_name)
                } else {
                    ctx.fresh_var()
                }
            } else {
                ctx.fresh_var()
            };
            if let Some(name_tok) = param.name() {
                let name_text = name_tok.text().to_string();
                env.insert(name_text, Scheme::mono(param_ty.clone()));
            }
            param_types.push(param_ty);
        }
    }

    // Reset loop_depth inside closure body (BRKC-05: break/continue cannot cross closure boundary).
    let saved_loop_depth = ctx.enter_closure();
    ctx.push_fn_return_type(None); // Closure return type is inferred
    let body_ty = if let Some(body) = closure.body() {
        infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
    } else {
        Ty::Tuple(vec![])
    };
    ctx.pop_fn_return_type();
    ctx.exit_closure(saved_loop_depth);

    env.pop_scope();

    Ok(Ty::Fun(param_types, Box::new(body_ty)))
}

/// Infer the type of a multi-clause closure.
///
/// Multi-clause closures like `fn 0 -> "zero" | n -> to_string(n) end` are
/// desugared during type inference: each clause is treated like a match arm.
/// The first clause's params/guard/body are direct children of CLOSURE_EXPR,
/// and subsequent clauses are CLOSURE_CLAUSE children.
fn infer_multi_clause_closure(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    closure: &ClosureExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    // Get arity from the first clause's param list.
    let arity = closure
        .param_list()
        .map(|pl| pl.params().count())
        .unwrap_or(0);

    // Create fresh type variables for each parameter position.
    let param_types: Vec<Ty> = (0..arity).map(|_| ctx.fresh_var()).collect();

    // ── Process the first clause (inline in CLOSURE_EXPR) ───────────────

    env.push_scope();

    if let Some(param_list) = closure.param_list() {
        for (param_idx, param) in param_list.params().enumerate() {
            if param_idx >= arity {
                break;
            }
            if let Some(pat) = param.pattern() {
                // Pattern parameter: infer type and unify with param position.
                let pat_ty = infer_pattern(ctx, env, &pat, types, type_registry)?;
                ctx.unify(pat_ty, param_types[param_idx].clone(), ConstraintOrigin::Builtin)?;
            } else if let Some(name_tok) = param.name() {
                // Regular named parameter: bind as wildcard.
                let name_text = name_tok.text().to_string();
                env.insert(name_text, Scheme::mono(param_types[param_idx].clone()));
            }
        }
    }

    // Process guard expression if present.
    if let Some(guard_clause) = closure.guard() {
        if let Some(guard_expr) = guard_clause.expr() {
            let guard_ty = infer_expr(
                ctx, env, &guard_expr, types, type_registry, trait_registry, fn_constraints,
            )?;
            let _ = ctx.unify(guard_ty, Ty::bool(), ConstraintOrigin::Builtin);
        }
    }

    // Infer the body (reset loop_depth inside closure -- BRKC-05).
    let saved_loop_depth = ctx.enter_closure();
    ctx.push_fn_return_type(None); // Multi-clause closure return type is inferred
    let first_body_ty = if let Some(body) = closure.body() {
        infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
    } else {
        Ty::Tuple(vec![])
    };
    ctx.pop_fn_return_type();
    ctx.exit_closure(saved_loop_depth);
    let mut result_ty: Option<Ty> = Some(first_body_ty);

    env.pop_scope();

    // ── Process subsequent clauses (CLOSURE_CLAUSE children) ────────────

    for clause in closure.clauses() {
        env.push_scope();

        if let Some(param_list) = clause.param_list() {
            for (param_idx, param) in param_list.params().enumerate() {
                if param_idx >= arity {
                    break;
                }
                if let Some(pat) = param.pattern() {
                    let pat_ty = infer_pattern(ctx, env, &pat, types, type_registry)?;
                    ctx.unify(pat_ty, param_types[param_idx].clone(), ConstraintOrigin::Builtin)?;
                } else if let Some(name_tok) = param.name() {
                    let name_text = name_tok.text().to_string();
                    env.insert(name_text, Scheme::mono(param_types[param_idx].clone()));
                }
            }
        }

        // Process guard if present.
        if let Some(guard_clause) = clause.guard() {
            if let Some(guard_expr) = guard_clause.expr() {
                let guard_ty = infer_expr(
                    ctx, env, &guard_expr, types, type_registry, trait_registry, fn_constraints,
                )?;
                let _ = ctx.unify(guard_ty, Ty::bool(), ConstraintOrigin::Builtin);
            }
        }

        // Infer body (reset loop_depth inside closure -- BRKC-05).
        let saved_loop_depth = ctx.enter_closure();
        ctx.push_fn_return_type(None); // Multi-clause closure return type is inferred
        let body_ty = if let Some(body) = clause.body() {
            infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
        } else {
            Ty::Tuple(vec![])
        };
        ctx.pop_fn_return_type();
        ctx.exit_closure(saved_loop_depth);

        // Unify body type with previous clauses.
        if let Some(ref prev_ty) = result_ty {
            ctx.unify(prev_ty.clone(), body_ty, ConstraintOrigin::Builtin)?;
        } else {
            result_ty = Some(body_ty);
        }

        env.pop_scope();
    }

    let body_ty = result_ty.unwrap_or_else(|| Ty::Tuple(vec![]));

    Ok(Ty::Fun(param_types, Box::new(body_ty)))
}

/// Infer the type of a block.
fn infer_block(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    block: &Block,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let mut last_ty = Ty::Tuple(vec![]);
    let mut local_fn_constraints = fn_constraints.clone();

    // Process ALL children in source order. This handles:
    // - Items (let bindings, fn defs) as declarations
    // - Expressions (function calls, etc.) as expression-statements
    // Processing in order ensures let bindings are in scope for subsequent exprs.
    //
    // Multi-clause function grouping (11-02): collect items first, group
    // consecutive same-name FnDef nodes, then process in source order.
    let mut processed_ranges: Vec<TextRange> = Vec::new();

    // Collect children in order, separating items and expressions.
    let mut block_children: Vec<(TextRange, BlockChildKind)> = Vec::new();
    let mut block_items: Vec<Item> = Vec::new();

    for child in block.syntax().children() {
        let range = child.text_range();
        if let Some(item) = Item::cast(child.clone()) {
            block_children.push((range, BlockChildKind::ItemIdx(block_items.len())));
            block_items.push(item);
        } else if let Some(_expr) = Expr::cast(child.clone()) {
            block_children.push((range, BlockChildKind::ExprNode(child)));
        }
    }

    // Group multi-clause functions.
    let grouped = group_multi_clause_fns(block_items);

    // Check for non-consecutive same-name function definitions.
    check_non_consecutive_clauses(&grouped, ctx);

    // Build original-item-index to grouped-item-index mapping.
    let mut block_item_to_grouped: FxHashMap<usize, usize> = FxHashMap::default();
    {
        let mut orig_idx = 0;
        for (gi, grouped_item) in grouped.iter().enumerate() {
            match grouped_item {
                GroupedItem::Single(_) => {
                    block_item_to_grouped.insert(orig_idx, gi);
                    orig_idx += 1;
                }
                GroupedItem::MultiClause { clauses } => {
                    for _ in 0..clauses.len() {
                        block_item_to_grouped.insert(orig_idx, gi);
                        orig_idx += 1;
                    }
                }
            }
        }
    }

    let mut block_processed_grouped: rustc_hash::FxHashSet<usize> = rustc_hash::FxHashSet::default();

    for (range, child_kind) in &block_children {
        match child_kind {
            BlockChildKind::ItemIdx(orig_idx) => {
                if let Some(&gi) = block_item_to_grouped.get(orig_idx) {
                    if block_processed_grouped.contains(&gi) {
                        continue;
                    }
                    block_processed_grouped.insert(gi);
                    processed_ranges.push(*range);

                    match &grouped[gi] {
                        GroupedItem::Single(item) => {
                            match item {
                                Item::LetBinding(let_) => {
                                    if let Ok(ty) = infer_let_binding(
                                        ctx,
                                        env,
                                        let_,
                                        types,
                                        type_registry,
                                        trait_registry,
                                        &mut local_fn_constraints,
                                    ) {
                                        last_ty = ty;
                                    }
                                }
                                Item::FnDef(fn_) => {
                                    if let Ok(ty) = infer_fn_def(
                                        ctx,
                                        env,
                                        fn_,
                                        types,
                                        type_registry,
                                        trait_registry,
                                        &mut local_fn_constraints,
                                    ) {
                                        last_ty = ty;
                                    }
                                }
                                _ => {
                                    // Other items (interface, impl, struct, etc.)
                                }
                            }
                        }
                        GroupedItem::MultiClause { clauses } => {
                            if let Ok(ty) = infer_multi_clause_fn(
                                ctx,
                                env,
                                clauses,
                                types,
                                type_registry,
                                trait_registry,
                                &mut local_fn_constraints,
                                &ImportContext::empty(),
                            ) {
                                last_ty = ty;
                            }
                        }
                    }
                }
            }
            BlockChildKind::ExprNode(child_node) => {
                if let Some(expr) = Expr::cast(child_node.clone()) {
                    processed_ranges.push(*range);
                    match infer_expr(ctx, env, &expr, types, type_registry, trait_registry, &local_fn_constraints) {
                        Ok(ty) => {
                            last_ty = ty;
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }

    // Legacy: also process the tail expression if it wasn't already handled.
    // This can happen if the tail expression's syntax node wasn't a direct child.
    if let Some(tail) = block.tail_expr() {
        let tail_range = tail.syntax().text_range();
        let already_processed = processed_ranges.iter().any(|r| *r == tail_range);

        if !already_processed {
            match infer_expr(ctx, env, &tail, types, type_registry, trait_registry, &local_fn_constraints) {
                Ok(ty) => {
                    last_ty = ty;
                }
                Err(_) => {}
            }
        }
    }

    Ok(last_ty)
}

/// Helper for block child classification.
enum BlockChildKind {
    ItemIdx(usize),
    ExprNode(mesh_parser::SyntaxNode),
}

/// Infer the type of a tuple expression.
fn infer_tuple(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    tuple: &TupleExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let mut elem_types = Vec::new();
    for elem in tuple.elements() {
        let ty = infer_expr(ctx, env, &elem, types, type_registry, trait_registry, fn_constraints)?;
        elem_types.push(ty);
    }

    if elem_types.len() == 1 {
        Ok(elem_types.into_iter().next().unwrap())
    } else {
        Ok(Ty::Tuple(elem_types))
    }
}

// ── AST-to-Abstract Pattern Conversion (04-04) ────────────────────────

/// Convert an AST pattern to the abstract `Pat` used by the exhaustiveness algorithm.
///
/// Variable bindings become wildcards (they match anything). Literals, constructors,
/// and or-patterns are mapped directly to their abstract equivalents.
fn ast_pattern_to_abstract(
    pat: &Pattern,
    env: &TypeEnv,
    type_registry: &TypeRegistry,
) -> AbsPat {
    match pat {
        Pattern::Wildcard(_) => AbsPat::Wildcard,
        Pattern::Ident(ident) => {
            // Check if this identifier is a known constructor (nullary variant).
            if let Some(name_tok) = ident.name() {
                let name_text = name_tok.text().to_string();
                if let Some(_scheme) = env.lookup(&name_text) {
                    // Check if this resolves to a sum type constructor by looking
                    // at the type registry for a variant with this name.
                    if let Some((sum_info, _variant)) = type_registry.lookup_variant(&name_text) {
                        return AbsPat::Constructor {
                            name: name_text,
                            type_name: sum_info.name.clone(),
                            args: vec![],
                        };
                    }
                }
            }
            // Regular variable binding -> wildcard for exhaustiveness.
            AbsPat::Wildcard
        }
        Pattern::Literal(lit) => {
            if let Some(token) = lit.token() {
                match token.kind() {
                    SyntaxKind::INT_LITERAL => AbsPat::Literal {
                        value: token.text().to_string(),
                        ty: AbsLitKind::Int,
                    },
                    SyntaxKind::FLOAT_LITERAL => AbsPat::Literal {
                        value: token.text().to_string(),
                        ty: AbsLitKind::Float,
                    },
                    SyntaxKind::TRUE_KW => AbsPat::Literal {
                        value: "true".to_string(),
                        ty: AbsLitKind::Bool,
                    },
                    SyntaxKind::FALSE_KW => AbsPat::Literal {
                        value: "false".to_string(),
                        ty: AbsLitKind::Bool,
                    },
                    SyntaxKind::STRING_START => {
                        // Extract actual string content from the LITERAL_PAT node's children
                        let mut content = String::new();
                        for child in lit.syntax().children_with_tokens() {
                            if child.kind() == SyntaxKind::STRING_CONTENT {
                                if let Some(tok) = child.as_token() {
                                    content.push_str(tok.text());
                                }
                            }
                        }
                        AbsPat::Literal {
                            value: content,
                            ty: AbsLitKind::String,
                        }
                    }
                    _ => AbsPat::Wildcard,
                }
            } else {
                AbsPat::Wildcard
            }
        }
        Pattern::Tuple(tuple_pat) => {
            let args: Vec<AbsPat> = tuple_pat
                .patterns()
                .map(|sub| ast_pattern_to_abstract(&sub, env, type_registry))
                .collect();
            AbsPat::Constructor {
                name: "Tuple".to_string(),
                type_name: "Tuple".to_string(),
                args,
            }
        }
        Pattern::Constructor(ctor_pat) => {
            let variant_name = ctor_pat
                .variant_name()
                .map(|t| t.text().to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            // Determine the type name.
            let type_name = if ctor_pat.is_qualified() {
                ctor_pat
                    .type_name()
                    .map(|t| t.text().to_string())
                    .unwrap_or_default()
            } else {
                // Look up unqualified variant in the type registry.
                type_registry
                    .lookup_variant(&variant_name)
                    .map(|(sum, _)| sum.name.clone())
                    .unwrap_or_default()
            };

            let args: Vec<AbsPat> = ctor_pat
                .fields()
                .map(|sub| ast_pattern_to_abstract(&sub, env, type_registry))
                .collect();

            AbsPat::Constructor {
                name: variant_name,
                type_name,
                args,
            }
        }
        Pattern::Or(or_pat) => {
            let alts: Vec<AbsPat> = or_pat
                .alternatives()
                .map(|alt| ast_pattern_to_abstract(&alt, env, type_registry))
                .collect();
            AbsPat::Or { alternatives: alts }
        }
        Pattern::As(as_pat) => {
            // For exhaustiveness, an as-pattern is equivalent to its inner pattern.
            if let Some(inner) = as_pat.pattern() {
                ast_pattern_to_abstract(&inner, env, type_registry)
            } else {
                AbsPat::Wildcard
            }
        }
        Pattern::Cons(_) => {
            // Cons patterns match non-empty lists. For exhaustiveness checking,
            // treat as a wildcard (conservative: won't incorrectly claim exhaustive).
            // Lists are infinite types so cons alone is never exhaustive anyway.
            AbsPat::Wildcard
        }
    }
}

/// Convert a resolved scrutinee type to the abstract `TypeInfo` used by exhaustiveness.
fn type_to_type_info(ty: &Ty, type_registry: &TypeRegistry) -> AbsTypeInfo {
    let resolved = match ty {
        Ty::App(con, _) => {
            if let Ty::Con(tc) = con.as_ref() {
                Some(tc.name.clone())
            } else {
                None
            }
        }
        Ty::Con(tc) => Some(tc.name.clone()),
        _ => None,
    };

    if let Some(ref name) = resolved {
        // Check if it's Bool.
        if name == "Bool" {
            return AbsTypeInfo::Bool;
        }

        // Check if it's a registered sum type.
        if let Some(sum_info) = type_registry.lookup_sum_type(name) {
            let variants: Vec<ConstructorSig> = sum_info
                .variants
                .iter()
                .map(|v| ConstructorSig {
                    name: v.name.clone(),
                    arity: v.fields.len(),
                })
                .collect();
            return AbsTypeInfo::SumType { variants };
        }
    }

    // Int, Float, String, or unknown -> infinite type.
    AbsTypeInfo::Infinite
}

/// Build an exhaustiveness `TypeRegistry` from the infer `TypeRegistry`.
///
/// This populates the abstract type registry with all known sum types
/// so that nested pattern checking can look up inner types.
fn build_abs_type_registry(type_registry: &TypeRegistry) -> AbsTypeRegistry {
    let mut abs_reg = AbsTypeRegistry::new();

    for (name, sum_info) in &type_registry.sum_type_defs {
        let variants: Vec<ConstructorSig> = sum_info
            .variants
            .iter()
            .map(|v| ConstructorSig {
                name: v.name.clone(),
                arity: v.fields.len(),
            })
            .collect();
        abs_reg.register(name.clone(), AbsTypeInfo::SumType { variants });
    }

    // Also register Bool for nested bool patterns.
    abs_reg.register("Bool", AbsTypeInfo::Bool);

    // Register Option and Result as sum types if they exist.
    // These are built-in but not in our type_registry, so add them.
    if abs_reg.lookup("Option").is_none() {
        abs_reg.register(
            "Option",
            AbsTypeInfo::SumType {
                variants: vec![
                    ConstructorSig {
                        name: "Some".to_string(),
                        arity: 1,
                    },
                    ConstructorSig {
                        name: "None".to_string(),
                        arity: 0,
                    },
                ],
            },
        );
    }
    if abs_reg.lookup("Result").is_none() {
        abs_reg.register(
            "Result",
            AbsTypeInfo::SumType {
                variants: vec![
                    ConstructorSig {
                        name: "Ok".to_string(),
                        arity: 1,
                    },
                    ConstructorSig {
                        name: "Err".to_string(),
                        arity: 1,
                    },
                ],
            },
        );
    }

    abs_reg
}

/// Format an abstract pattern as a human-readable string for error messages.
fn format_abstract_pat(pat: &AbsPat) -> String {
    match pat {
        AbsPat::Wildcard => "_".to_string(),
        AbsPat::Constructor { name, args, .. } => {
            if args.is_empty() {
                name.clone()
            } else {
                let args_str: Vec<String> = args.iter().map(format_abstract_pat).collect();
                format!("{}({})", name, args_str.join(", "))
            }
        }
        AbsPat::Literal { value, .. } => value.clone(),
        AbsPat::Or { alternatives } => {
            let alts_str: Vec<String> = alternatives.iter().map(format_abstract_pat).collect();
            alts_str.join(" | ")
        }
    }
}

// ── Guard Expression Validation (04-04) ────────────────────────────────

/// Validate that a guard expression only uses allowed constructs:
/// comparisons, boolean operators, literals, and name references.
///
/// Guards must be simple boolean expressions. Function calls, assignments,
/// and other complex expressions are disallowed.
fn validate_guard_expr(expr: &Expr) -> Result<(), String> {
    match expr {
        Expr::Literal(_) | Expr::NameRef(_) | Expr::AtomLiteral(_) => Ok(()),
        Expr::BinaryExpr(bin) => {
            // Allow comparisons and boolean ops.
            if let Some(op) = bin.op() {
                match op.kind() {
                    SyntaxKind::EQ_EQ
                    | SyntaxKind::NOT_EQ
                    | SyntaxKind::LT
                    | SyntaxKind::GT
                    | SyntaxKind::LT_EQ
                    | SyntaxKind::GT_EQ
                    | SyntaxKind::AND_KW
                    | SyntaxKind::OR_KW
                    | SyntaxKind::AMP_AMP
                    | SyntaxKind::PIPE_PIPE => {}
                    _ => {
                        return Err(format!(
                            "operator `{}` not allowed in guard",
                            op.text()
                        ));
                    }
                }
            }
            if let Some(lhs) = bin.lhs() {
                validate_guard_expr(&lhs)?;
            }
            if let Some(rhs) = bin.rhs() {
                validate_guard_expr(&rhs)?;
            }
            Ok(())
        }
        Expr::UnaryExpr(un) => {
            // Allow `not` / `!`
            if let Some(op) = un.op() {
                match op.kind() {
                    SyntaxKind::BANG | SyntaxKind::NOT_KW => {}
                    _ => {
                        return Err(format!(
                            "operator `{}` not allowed in guard",
                            op.text()
                        ));
                    }
                }
            }
            if let Some(operand) = un.operand() {
                validate_guard_expr(&operand)?;
            }
            Ok(())
        }
        Expr::TupleExpr(_) => {
            // Allow parenthesized grouping.
            Ok(())
        }
        Expr::CallExpr(call) => {
            // Allow calls to builtins (functions referenced by name).
            if let Some(callee) = call.callee() {
                match callee {
                    Expr::NameRef(_) => Ok(()),
                    _ => Err("only named function calls allowed in guard".to_string()),
                }
            } else {
                Ok(())
            }
        }
        _ => Err(format!("expression not allowed in guard")),
    }
}

/// Infer the type of a case/match expression.
///
/// After type-checking all arms, runs exhaustiveness and redundancy analysis.
/// Guarded arms are excluded from the exhaustiveness matrix (they may not match).
fn infer_case(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    case: &CaseExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let scrutinee_ty = if let Some(scrutinee) = case.scrutinee() {
        infer_expr(ctx, env, &scrutinee, types, type_registry, trait_registry, fn_constraints)?
    } else {
        ctx.fresh_var()
    };

    let mut result_ty: Option<Ty> = None;

    // Collect patterns and guard info for exhaustiveness checking.
    let mut arm_patterns: Vec<AbsPat> = Vec::new();
    let mut arm_has_guard: Vec<bool> = Vec::new();
    let mut arm_spans: Vec<TextRange> = Vec::new();

    for arm in case.arms() {
        env.push_scope();

        if let Some(pat) = arm.pattern() {
            let pat_ty = infer_pattern(ctx, env, &pat, types, type_registry)?;
            ctx.unify(pat_ty, scrutinee_ty.clone(), ConstraintOrigin::Builtin)?;

            // Convert to abstract pattern for exhaustiveness.
            let abs_pat = ast_pattern_to_abstract(&pat, env, type_registry);
            arm_patterns.push(abs_pat);
        } else {
            arm_patterns.push(AbsPat::Wildcard);
        }

        // Check for guard expression.
        let has_guard = arm.guard().is_some();
        arm_has_guard.push(has_guard);
        arm_spans.push(arm.syntax().text_range());

        // Validate and type-check guard if present.
        if let Some(guard_expr) = arm.guard() {
            // Validate guard uses only allowed constructs.
            if let Err(reason) = validate_guard_expr(&guard_expr) {
                let err = TypeError::InvalidGuardExpression {
                    reason,
                    span: guard_expr.syntax().text_range(),
                };
                ctx.errors.push(err);
            }

            // Type-check the guard -- it must be Bool.
            let guard_ty = infer_expr(
                ctx, env, &guard_expr, types, type_registry, trait_registry, fn_constraints,
            )?;
            let _ = ctx.unify(guard_ty, Ty::bool(), ConstraintOrigin::Builtin);
        }

        if let Some(body) = arm.body() {
            let body_ty = infer_expr(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?;
            if let Some(ref prev_ty) = result_ty {
                ctx.unify(
                    prev_ty.clone(),
                    body_ty.clone(),
                    ConstraintOrigin::Builtin,
                )?;
            } else {
                result_ty = Some(body_ty);
            }
        }

        env.pop_scope();
    }

    // ── Exhaustiveness and redundancy checking ─────────────────────────
    let resolved_scrutinee = ctx.resolve(scrutinee_ty.clone());
    let scrutinee_type_info = type_to_type_info(&resolved_scrutinee, type_registry);
    let abs_registry = build_abs_type_registry(type_registry);

    // For exhaustiveness: exclude guarded arms (they may not match).
    let unguarded_patterns: Vec<AbsPat> = arm_patterns
        .iter()
        .zip(arm_has_guard.iter())
        .filter(|(_, has_guard)| !**has_guard)
        .map(|(pat, _)| pat.clone())
        .collect();

    if let Some(witnesses) = exhaustiveness::check_exhaustiveness(
        &unguarded_patterns,
        &scrutinee_type_info,
        &abs_registry,
    ) {
        let missing: Vec<String> = witnesses.iter().map(format_abstract_pat).collect();
        let err = TypeError::NonExhaustiveMatch {
            scrutinee_type: format!("{}", resolved_scrutinee),
            missing_patterns: missing,
            span: case.syntax().text_range(),
        };
        ctx.errors.push(err);
    }

    // For redundancy: check all arms (including guarded ones).
    let redundant_indices =
        exhaustiveness::check_redundancy(&arm_patterns, &scrutinee_type_info, &abs_registry);
    for idx in redundant_indices {
        let warn = TypeError::RedundantArm {
            arm_index: idx,
            span: arm_spans.get(idx).copied().unwrap_or(case.syntax().text_range()),
        };
        ctx.warnings.push(warn);
    }

    Ok(result_ty.unwrap_or_else(|| Ty::Tuple(vec![])))
}

/// Infer the type of a return expression.
fn infer_return(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    ret: &ReturnExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    if let Some(value) = ret.value() {
        let _ty = infer_expr(ctx, env, &value, types, type_registry, trait_registry, fn_constraints)?;
    }
    Ok(Ty::Never)
}

// ── Method Resolution (30-01) ──────────────────────────────────────────

/// Build the full function type for a trait method, including the self parameter.
/// Returns `Ty::Fun([self_type, ...param_types], return_type)`.
///
/// Searches the trait registry's impl blocks to find the matching method
/// signature for the given self type. Uses `param_count` to create fresh
/// type variables for non-self parameters (since `ImplMethodSig` does not
/// store individual parameter types).
fn build_method_fn_type(
    trait_registry: &TraitRegistry,
    method_name: &str,
    self_ty: &Ty,
    ret_ty: &Ty,
    ctx: &mut InferCtx,
) -> Ty {
    // Look up the method signature to determine parameter count.
    if let Some(method_sig) = trait_registry.find_method_sig(method_name, self_ty) {
        let mut param_types = vec![self_ty.clone()]; // self parameter
        // Add fresh type vars for remaining params (we only know the count).
        for _ in 0..method_sig.param_count {
            param_types.push(ctx.fresh_var());
        }
        return Ty::Fun(param_types, Box::new(ret_ty.clone()));
    }
    // Fallback: if we can't find the full signature, construct a unary function
    // (self -> return_type). infer_call's unification will catch arity mismatches.
    Ty::Fun(vec![self_ty.clone()], Box::new(ret_ty.clone()))
}

// ── Struct/Field Inference (03-03) ─────────────────────────────────────

/// Infer the type of a field access expression: `expr.field_name`
fn infer_field_access(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    fa: &FieldAccess,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
    is_method_call: bool,
) -> Result<Ty, TypeError> {
    let base_expr = fa.base().ok_or_else(|| {
        let err = TypeError::Mismatch {
            expected: Ty::Never,
            found: Ty::Never,
            origin: ConstraintOrigin::Builtin,
        };
        ctx.errors.push(err.clone());
        err
    })?;

    let field_name = match fa.field() {
        Some(tok) => tok.text().to_string(),
        None => "<unknown>".to_string(),
    };

    // Check if base is a NameRef pointing to a module name for qualified access.
    // e.g. Vector.add (user module), String.length (stdlib) -- module-qualified function reference.
    if let Expr::NameRef(ref name_ref) = base_expr {
        if let Some(base_name) = name_ref.text() {
            // Check user-defined modules first (from import context via ctx.qualified_modules)
            if let Some(mod_fns) = ctx.qualified_modules.get(&base_name) {
                if let Some(scheme) = mod_fns.get(&field_name) {
                    let scheme = scheme.clone();
                    let ty = ctx.instantiate(&scheme);
                    return Ok(ty);
                }
                // Module exists but field not found -- fall through to other checks
            }

            // Then check stdlib modules (existing behavior)
            if is_stdlib_module(&base_name) {
                let modules = stdlib_modules();
                if let Some(mod_exports) = modules.get(&base_name) {
                    if let Some(scheme) = mod_exports.get(&field_name) {
                        let ty = ctx.instantiate(scheme);
                        return Ok(ty);
                    }
                }
            }

            // Node.spawn and Node.spawn_link are variadic -- return a permissive
            // type that matches any call site arguments (returns Int = remote PID).
            if base_name == "Node" && (field_name == "spawn" || field_name == "spawn_link") {
                // Build a function type that accepts whatever arguments are at the call site.
                // The type is: fn(any_args...) -> Int. We use fresh type variables for all
                // parameters since the actual types are checked at codegen time.
                // The caller (infer_call) will create arg types and unify; by returning
                // a fresh variable, unification will succeed with any argument list.
                return Ok(ctx.fresh_var());
            }

            // Check if base is a user-defined service module (e.g. Counter.get_count).
            // Service helper functions are registered in env as "ServiceName.method_name".
            {
                let qualified = format!("{}.{}", base_name, field_name);
                if let Some(scheme) = env.lookup(&qualified) {
                    // Only treat as service module if it's not also a sum type variant.
                    if type_registry
                        .lookup_qualified_variant(&base_name, &field_name)
                        .is_none()
                    {
                        let ty = ctx.instantiate(scheme);
                        return Ok(ty);
                    }
                }
            }

            // Check if base is a sum type name for variant construction.
            // e.g. Shape.Circle -- Shape is a sum type, Circle is a variant.
            if let Some((_sum_info, _variant_info)) =
                type_registry.lookup_qualified_variant(&base_name, &field_name)
            {
                let qualified = format!("{}.{}", base_name, field_name);
                if let Some(scheme) = env.lookup(&qualified) {
                    let ty = ctx.instantiate(scheme);
                    return Ok(ty);
                }
            }

            // Check if base is a struct type name with a static trait method.
            // e.g. User.from_json -- User is a struct type, from_json is a FromJson method.
            if field_name == "from_json" {
                if let Some(_struct_info) = type_registry.lookup_struct(&base_name) {
                    // Check if this struct has FromJson impl
                    let struct_ty = Ty::Con(TyCon::new(&base_name));
                    if trait_registry.has_impl("FromJson", &struct_ty) {
                        // from_json :: String -> Result<StructName, String>
                        let result_ty = Ty::result(struct_ty, Ty::string());
                        return Ok(Ty::fun(vec![Ty::string()], result_ty));
                    }
                }
                // Also check sum types: Shape.from_json
                if let Some(_sum_info) = type_registry.lookup_sum_type(&base_name) {
                    let sum_ty = Ty::Con(TyCon::new(&base_name));
                    if trait_registry.has_impl("FromJson", &sum_ty) {
                        // from_json :: String -> Result<SumTypeName, String>
                        let result_ty = Ty::result(sum_ty, Ty::string());
                        return Ok(Ty::fun(vec![Ty::string()], result_ty));
                    }
                }
            }

            // Check if base is a struct type name with FromRow trait method.
            // e.g. User.from_row -- User is a struct with deriving(Row).
            if field_name == "from_row" {
                if let Some(_struct_info) = type_registry.lookup_struct(&base_name) {
                    let struct_ty = Ty::Con(TyCon::new(&base_name));
                    if trait_registry.has_impl("FromRow", &struct_ty) {
                        // from_row :: Map<String, String> -> Result<StructName, String>
                        let map_ty = Ty::map(Ty::string(), Ty::string());
                        let result_ty = Ty::result(struct_ty, Ty::string());
                        return Ok(Ty::fun(vec![map_ty], result_ty));
                    }
                }
            }

            // Check if base is a struct type name with Schema metadata functions.
            // e.g. User.__table__() -- User is a struct with deriving(Schema).
            if field_name == "__table__" || field_name == "__fields__"
                || field_name == "__primary_key__" || field_name == "__relationships__"
                || field_name == "__field_types__" || field_name == "__relationship_meta__"
                || (field_name.starts_with("__") && field_name.ends_with("_col__"))
            {
                let qualified = format!("{}.{}", base_name, field_name);
                if let Some(scheme) = env.lookup(&qualified) {
                    let ty = ctx.instantiate(scheme);
                    return Ok(ty);
                }
            }
        }
    }

    let base_ty = infer_expr(ctx, env, &base_expr, types, type_registry, trait_registry, fn_constraints)?;
    let resolved_base = ctx.resolve(base_ty);

    let struct_name = match &resolved_base {
        Ty::App(con, _) => {
            if let Ty::Con(tc) = con.as_ref() {
                Some(tc.name.clone())
            } else {
                None
            }
        }
        Ty::Con(tc) => Some(tc.name.clone()),
        _ => None,
    };

    if let Some(name) = struct_name {
        if let Some(struct_info) = type_registry.lookup_struct(&name) {
            let struct_info = struct_info.clone();
            // Get the type arguments from the resolved base type.
            let type_args = match &resolved_base {
                Ty::App(_, args) => args.clone(),
                _ => vec![],
            };
            for (fname, fty) in &struct_info.fields {
                if *fname == field_name {
                    // Substitute generic params with actual type args.
                    let resolved_field = substitute_type_params(
                        fty,
                        &struct_info.generic_params,
                        &type_args,
                    );
                    return Ok(resolved_field);
                }
            }
            // Field not found in struct -- try method resolution if in method call context.
            if is_method_call {
                let matching_traits = trait_registry.find_method_traits(&field_name, &resolved_base);
                if matching_traits.len() > 1 {
                    let err = TypeError::AmbiguousMethod {
                        method_name: field_name.clone(),
                        candidate_traits: matching_traits,
                        ty: resolved_base.clone(),
                        span: fa.syntax().text_range(),
                    };
                    ctx.errors.push(err.clone());
                    return Err(err);
                }
                if let Some(ret_ty) = trait_registry.resolve_trait_method(&field_name, &resolved_base) {
                    let method_fn_ty = build_method_fn_type(trait_registry, &field_name, &resolved_base, &ret_ty, ctx);
                    return Ok(method_fn_ty);
                }
                // Method not found -- emit NoSuchMethod (not NoSuchField).
                let err = TypeError::NoSuchMethod {
                    ty: resolved_base,
                    method_name: field_name,
                    span: fa.syntax().text_range(),
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }
            // Not a method call -- emit NoSuchField.
            let err = TypeError::NoSuchField {
                ty: resolved_base,
                field_name,
                span: fa.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
    }

    // Base type is not a struct (or no struct def found).
    if is_method_call {
        let matching_traits = trait_registry.find_method_traits(&field_name, &resolved_base);
        if matching_traits.len() > 1 {
            let err = TypeError::AmbiguousMethod {
                method_name: field_name.clone(),
                candidate_traits: matching_traits,
                ty: resolved_base.clone(),
                span: fa.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
        if let Some(ret_ty) = trait_registry.resolve_trait_method(&field_name, &resolved_base) {
            let method_fn_ty = build_method_fn_type(trait_registry, &field_name, &resolved_base, &ret_ty, ctx);
            return Ok(method_fn_ty);
        }

        // ── Stdlib module method fallback (Phase 31) ──────────────────
        // After trait method resolution fails, check if the method name
        // matches a stdlib module function for the receiver type.
        // e.g. "hello".length() -> String.length(str), my_list.length() -> List.length(list)
        let module_name = match &resolved_base {
            t if *t == Ty::string() => Some("String"),
            t if *t == Ty::range() => Some("Range"),
            t if *t == Ty::set_untyped() => Some("Set"),
            Ty::App(con, _) => {
                if let Ty::Con(c) = con.as_ref() {
                    match c.name.as_str() {
                        "List" => Some("List"),
                        "Map" => Some("Map"),
                        "Set" => Some("Set"),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(mod_name) = module_name {
            let modules = stdlib_modules();
            if let Some(mod_fns) = modules.get(mod_name) {
                if let Some(scheme) = mod_fns.get(&field_name) {
                    let fn_ty = ctx.instantiate(scheme);
                    return Ok(fn_ty);
                }
            }
        }

        let err = TypeError::NoSuchMethod {
            ty: resolved_base,
            method_name: field_name,
            span: fa.syntax().text_range(),
        };
        ctx.errors.push(err.clone());
        return Err(err);
    }

    // ── Non-struct concrete type NoSuchField (Phase 31) ──────────────
    // For known concrete non-struct types (Ty::Con, Ty::App), return
    // Err(NoSuchField) to trigger the retry mechanism in infer_call.
    // Only return Ok(fresh_var) for truly unresolved types (Ty::Var).
    match &resolved_base {
        Ty::Con(_) | Ty::App(_, _) => {
            let err = TypeError::NoSuchField {
                ty: resolved_base,
                field_name,
                span: fa.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
        _ => {} // Ty::Var, Ty::Fun, etc. -- leave as fresh_var for unresolved types
    }

    Ok(ctx.fresh_var())
}

/// Infer the type of a struct literal: `StructName { field1: expr1, ... }`
///
/// 1. Look up the struct definition.
/// 2. Create fresh type variables for generic parameters.
/// 3. For each field in the literal, infer value type and unify with expected.
/// 4. Check all required fields are present.
/// 5. Return the struct type with inferred generic arguments.
fn infer_struct_literal(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    sl: &StructLiteral,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let struct_name = sl
        .name_ref()
        .and_then(|nr| nr.text())
        .unwrap_or_else(|| "<unknown>".to_string());

    let struct_def = match type_registry.lookup_struct(&struct_name) {
        Some(def) => def.clone(),
        None => {
            // Unknown struct -- infer field values anyway, return a basic type.
            for field in sl.fields() {
                if let Some(value) = field.value() {
                    let _ = infer_expr(ctx, env, &value, types, type_registry, trait_registry, fn_constraints);
                }
            }
            return Ok(Ty::struct_ty(&struct_name, vec![]));
        }
    };

    // Create fresh type variables for generic params.
    let generic_vars: Vec<Ty> = struct_def
        .generic_params
        .iter()
        .map(|_| ctx.fresh_var())
        .collect();

    // Track provided fields.
    let mut provided_fields: Vec<String> = Vec::new();

    for field in sl.fields() {
        let field_name = match field.name().and_then(|n| n.text()) {
            Some(n) => n,
            None => continue,
        };

        // Find expected field type.
        let expected_ty = struct_def
            .fields
            .iter()
            .find(|(name, _)| *name == field_name)
            .map(|(_, ty)| {
                substitute_type_params(ty, &struct_def.generic_params, &generic_vars)
            });

        let expected_ty = match expected_ty {
            Some(ty) => ty,
            None => {
                let err = TypeError::UnknownField {
                    struct_name: struct_name.clone(),
                    field_name: field_name.clone(),
                    span: field.syntax().text_range(),
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }
        };

        // Infer field value.
        if let Some(value) = field.value() {
            let value_ty = infer_expr(ctx, env, &value, types, type_registry, trait_registry, fn_constraints)?;
            ctx.unify(
                value_ty,
                expected_ty,
                ConstraintOrigin::Annotation {
                    annotation_span: field.syntax().text_range(),
                },
            )?;
        }

        provided_fields.push(field_name);
    }

    // Check for missing fields.
    for (field_name, _) in &struct_def.fields {
        if !provided_fields.contains(field_name) {
            let err = TypeError::MissingField {
                struct_name: struct_name.clone(),
                field_name: field_name.clone(),
                span: sl.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
    }

    // Build the struct type with display_prefix if applicable.
    // Look up the env entry for this struct to preserve its display_prefix
    // (set during import resolution or local struct registration).
    let tycon = match env.lookup(&struct_name) {
        Some(scheme) => {
            match &scheme.ty {
                Ty::App(inner, _) => {
                    if let Ty::Con(tc) = inner.as_ref() {
                        tc.clone()
                    } else {
                        TyCon::new(&struct_name)
                    }
                }
                Ty::Con(tc) => tc.clone(),
                _ => TyCon::new(&struct_name),
            }
        }
        None => TyCon::new(&struct_name),
    };

    Ok(Ty::App(
        Box::new(Ty::Con(tycon)),
        generic_vars,
    ))
}

// ── Struct Update Inference ────────────────────────────────────────────

/// Infer the type of a struct update expression: `%{base | field: value, ...}`
///
/// The base expression must resolve to a struct type. Each override field must
/// exist in the struct definition and its value must unify with the field's type.
/// Returns the same struct type as the base expression.
fn infer_struct_update(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    update: &StructUpdate,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    // Infer the type of the base expression.
    let base_expr = match update.base_expr() {
        Some(e) => e,
        None => return Ok(ctx.fresh_var()),
    };
    let base_ty = infer_expr(ctx, env, &base_expr, types, type_registry, trait_registry, fn_constraints)?;
    let resolved_base = ctx.resolve(base_ty.clone());

    // Extract the struct name from the resolved type.
    let struct_name = match &resolved_base {
        Ty::Con(tc) => tc.name.clone(),
        Ty::App(inner, _) => {
            if let Ty::Con(tc) = inner.as_ref() {
                tc.name.clone()
            } else {
                // Not a struct type -- emit a "no such field" error as a proxy.
                let err = TypeError::NoSuchField {
                    ty: resolved_base.clone(),
                    field_name: "<struct update>".to_string(),
                    span: base_expr.syntax().text_range(),
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }
        }
        _ => {
            let err = TypeError::NoSuchField {
                ty: resolved_base.clone(),
                field_name: "<struct update>".to_string(),
                span: base_expr.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
    };

    // Look up the struct definition.
    let struct_def = match type_registry.lookup_struct(&struct_name) {
        Some(def) => def.clone(),
        None => {
            // Infer override values anyway.
            for field in update.override_fields() {
                if let Some(value) = field.value() {
                    let _ = infer_expr(ctx, env, &value, types, type_registry, trait_registry, fn_constraints);
                }
            }
            return Ok(resolved_base);
        }
    };

    // Create fresh type variables for generic params (matching base type args).
    let generic_vars: Vec<Ty> = match &resolved_base {
        Ty::App(_, args) => args.clone(),
        _ => struct_def.generic_params.iter().map(|_| ctx.fresh_var()).collect(),
    };

    // Validate and infer each override field.
    for field in update.override_fields() {
        let field_name = match field.name().and_then(|n| n.text()) {
            Some(n) => n,
            None => continue,
        };

        // Verify the field exists in the struct.
        let expected_ty = struct_def
            .fields
            .iter()
            .find(|(name, _)| *name == field_name)
            .map(|(_, ty)| substitute_type_params(ty, &struct_def.generic_params, &generic_vars));

        let expected_ty = match expected_ty {
            Some(ty) => ty,
            None => {
                let err = TypeError::UnknownField {
                    struct_name: struct_name.clone(),
                    field_name: field_name.clone(),
                    span: field.syntax().text_range(),
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }
        };

        // Infer the override value and unify with expected field type.
        if let Some(value) = field.value() {
            let value_ty = infer_expr(ctx, env, &value, types, type_registry, trait_registry, fn_constraints)?;
            ctx.unify(
                value_ty,
                expected_ty,
                ConstraintOrigin::Annotation {
                    annotation_span: field.syntax().text_range(),
                },
            )?;
        }
    }

    // Return the same type as the base expression.
    Ok(base_ty)
}

// ── Map Literal Inference ──────────────────────────────────────────────

/// Infer the type of a map literal: `%{k1 => v1, k2 => v2, ...}`
///
/// Creates fresh type variables for K and V, then unifies each entry's key
/// and value types against them. Returns `Map<K, V>`.
fn infer_map_literal(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    map_lit: &MapLiteral,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let k_ty = ctx.fresh_var();
    let v_ty = ctx.fresh_var();

    for entry in map_lit.entries() {
        if let Some(key_expr) = entry.key() {
            // For keyword argument entries (name: value), the key is a NAME_REF
            // representing the identifier text as a string key, not a variable reference.
            let key_inferred = if entry.is_keyword_entry() {
                Ty::string()
            } else {
                infer_expr(ctx, env, &key_expr, types, type_registry, trait_registry, fn_constraints)?
            };
            ctx.unify(
                key_inferred,
                k_ty.clone(),
                ConstraintOrigin::Annotation {
                    annotation_span: entry.syntax().text_range(),
                },
            )?;
        }
        if let Some(val_expr) = entry.value() {
            let val_inferred = infer_expr(ctx, env, &val_expr, types, type_registry, trait_registry, fn_constraints)?;
            ctx.unify(
                val_inferred,
                v_ty.clone(),
                ConstraintOrigin::Annotation {
                    annotation_span: entry.syntax().text_range(),
                },
            )?;
        }
    }

    Ok(Ty::map(k_ty, v_ty))
}

// ── List Literal Inference ──────────────────────────────────────────────

/// Infer the type of a list literal: `[e1, e2, ...]`
///
/// Creates a fresh type variable for the element type, then unifies each
/// element's inferred type against it. Returns `List<T>`.
fn infer_list_literal(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    lit: &ListLiteral,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let elem_ty = ctx.fresh_var();
    for elem in lit.elements() {
        let t = infer_expr(ctx, env, &elem, types, type_registry, trait_registry, fn_constraints)?;
        ctx.unify(
            t,
            elem_ty.clone(),
            ConstraintOrigin::Annotation {
                annotation_span: elem.syntax().text_range(),
            },
        )?;
    }
    let resolved = ctx.resolve(elem_ty);
    let result_ty = Ty::list(resolved);
    types.insert(lit.syntax().text_range(), result_ty.clone());
    Ok(result_ty)
}

// ── Pattern Inference ──────────────────────────────────────────────────

/// Infer the type of a pattern, binding any variables into the environment.
fn infer_pattern(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    pat: &Pattern,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
) -> Result<Ty, TypeError> {
    match pat {
        Pattern::Ident(ident) => {
            if let Some(name_tok) = ident.name() {
                let name_text = name_tok.text().to_string();

                // Check if this identifier is a known nullary variant constructor.
                // In Mesh, bare uppercase names like `Red`, `None`, `Point` in pattern
                // position should resolve to constructors, not create fresh bindings.
                if let Some(scheme) = env.lookup(&name_text) {
                    let candidate = ctx.instantiate(scheme);
                    let resolved = ctx.resolve(candidate.clone());
                    // If the name resolves to a sum type (nullary constructor), use it.
                    let is_sum_type = matches!(&resolved, Ty::App(con, _) if matches!(con.as_ref(), Ty::Con(_)));
                    if is_sum_type {
                        types.insert(pat.syntax().text_range(), candidate.clone());
                        return Ok(candidate);
                    }
                }

                // Regular identifier pattern: create a fresh binding.
                let ty = ctx.fresh_var();
                env.insert(name_text, Scheme::mono(ty.clone()));
                types.insert(pat.syntax().text_range(), ty.clone());
                Ok(ty)
            } else {
                let ty = ctx.fresh_var();
                types.insert(pat.syntax().text_range(), ty.clone());
                Ok(ty)
            }
        }
        Pattern::Wildcard(_) => {
            let ty = ctx.fresh_var();
            types.insert(pat.syntax().text_range(), ty.clone());
            Ok(ty)
        }
        Pattern::Literal(lit) => {
            let ty = if let Some(token) = lit.token() {
                match token.kind() {
                    SyntaxKind::INT_LITERAL => Ty::int(),
                    SyntaxKind::FLOAT_LITERAL => Ty::float(),
                    SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW => Ty::bool(),
                    SyntaxKind::NIL_KW => Ty::Tuple(vec![]),
                    SyntaxKind::STRING_START => Ty::string(),
                    _ => ctx.fresh_var(),
                }
            } else {
                ctx.fresh_var()
            };
            types.insert(pat.syntax().text_range(), ty.clone());
            Ok(ty)
        }
        Pattern::Tuple(tuple_pat) => {
            let mut elem_types = Vec::new();
            for sub_pat in tuple_pat.patterns() {
                let ty = infer_pattern(ctx, env, &sub_pat, types, type_registry)?;
                elem_types.push(ty);
            }
            let ty = Ty::Tuple(elem_types);
            types.insert(pat.syntax().text_range(), ty.clone());
            Ok(ty)
        }
        Pattern::Constructor(ctor_pat) => {
            infer_constructor_pattern(ctx, env, ctor_pat, pat, types, type_registry)
        }
        Pattern::Or(or_pat) => {
            infer_or_pattern(ctx, env, or_pat, pat, types, type_registry)
        }
        Pattern::As(as_pat) => {
            infer_as_pattern(ctx, env, as_pat, pat, types, type_registry)
        }
        Pattern::Cons(cons_pat) => {
            infer_cons_pattern(ctx, env, cons_pat, pat, types, type_registry)
        }
    }
}

/// Infer a constructor pattern: `Circle(r)` or `Shape.Circle(r)`.
///
/// 1. Look up the variant constructor (qualified or unqualified) in the env.
/// 2. Instantiate the constructor scheme to get fresh type vars.
/// 3. If it's a function type, unify sub-pattern types with param types.
/// 4. Return the result type (the sum type).
fn infer_constructor_pattern(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    ctor_pat: &mesh_parser::ast::pat::ConstructorPat,
    pat: &Pattern,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
) -> Result<Ty, TypeError> {
    let variant_name = ctor_pat
        .variant_name()
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    // Build the lookup name -- qualified or unqualified.
    let lookup_name = if ctor_pat.is_qualified() {
        let type_name = ctor_pat
            .type_name()
            .map(|t| t.text().to_string())
            .unwrap_or_else(|| "<unknown>".to_string());
        format!("{}.{}", type_name, variant_name)
    } else {
        variant_name.clone()
    };

    // Look up the constructor in the environment.
    let ctor_scheme = match env.lookup(&lookup_name) {
        Some(scheme) => scheme.clone(),
        None => {
            // Try to find in type registry for better error message.
            let err = TypeError::UnknownVariant {
                name: lookup_name,
                span: pat.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
    };

    let ctor_ty = ctx.instantiate(&ctor_scheme);

    // Collect sub-patterns from the constructor.
    let sub_patterns: Vec<Pattern> = ctor_pat.fields().collect();

    match ctor_ty {
        Ty::Fun(param_types, ret) => {
            // Constructor with fields: unify sub-pattern types with param types.
            if sub_patterns.len() != param_types.len() {
                let err = TypeError::ArityMismatch {
                    expected: param_types.len(),
                    found: sub_patterns.len(),
                    origin: ConstraintOrigin::Builtin,
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }

            for (sub_pat, expected_ty) in sub_patterns.iter().zip(param_types.iter()) {
                let sub_ty = infer_pattern(ctx, env, sub_pat, types, type_registry)?;
                ctx.unify(sub_ty, expected_ty.clone(), ConstraintOrigin::Builtin)?;
            }

            types.insert(pat.syntax().text_range(), (*ret).clone());
            Ok(*ret)
        }
        _ => {
            // Nullary constructor (not a function): no sub-patterns expected.
            if !sub_patterns.is_empty() {
                let err = TypeError::ArityMismatch {
                    expected: 0,
                    found: sub_patterns.len(),
                    origin: ConstraintOrigin::Builtin,
                };
                ctx.errors.push(err.clone());
                return Err(err);
            }

            types.insert(pat.syntax().text_range(), ctor_ty.clone());
            Ok(ctor_ty)
        }
    }
}

/// Infer an or-pattern: `Circle(_) | Point`.
///
/// 1. Infer each alternative in a temporary scope.
/// 2. Unify all alternatives (they must match the same type).
/// 3. Validate that all alternatives bind the same set of variable names.
/// 4. Re-bind variables from the first alternative into the current scope.
fn infer_or_pattern(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    or_pat: &mesh_parser::ast::pat::OrPat,
    pat: &Pattern,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
) -> Result<Ty, TypeError> {
    let alternatives: Vec<Pattern> = or_pat.alternatives().collect();

    if alternatives.is_empty() {
        let ty = ctx.fresh_var();
        types.insert(pat.syntax().text_range(), ty.clone());
        return Ok(ty);
    }

    // Collect binding names using semantic-aware collection (needs env).
    let first_names = collect_pattern_binding_names(&alternatives[0], env);

    // Infer first alternative in a temporary scope.
    env.push_scope();
    let first_ty = infer_pattern(ctx, env, &alternatives[0], types, type_registry)?;

    // Save the bindings from the first alternative to re-apply later.
    let first_bindings: Vec<(String, Scheme)> = first_names
        .iter()
        .filter_map(|name| {
            env.lookup(name).map(|scheme| (name.clone(), scheme.clone()))
        })
        .collect();
    env.pop_scope();

    // Infer remaining alternatives, unify types, validate bindings.
    for alt in alternatives.iter().skip(1) {
        let alt_names = collect_pattern_binding_names(alt, env);

        env.push_scope();
        let alt_ty = infer_pattern(ctx, env, alt, types, type_registry)?;
        ctx.unify(first_ty.clone(), alt_ty, ConstraintOrigin::Builtin)?;
        env.pop_scope();

        // Validate same variable names are bound.
        let mut first_sorted = first_names.clone();
        first_sorted.sort();
        let mut alt_sorted = alt_names;
        alt_sorted.sort();

        if first_sorted != alt_sorted {
            let err = TypeError::OrPatternBindingMismatch {
                expected_bindings: first_sorted,
                found_bindings: alt_sorted,
                span: pat.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
    }

    // Re-bind the variables from the first alternative into the current scope.
    for (name, scheme) in first_bindings {
        env.insert(name, scheme);
    }

    types.insert(pat.syntax().text_range(), first_ty.clone());
    Ok(first_ty)
}

/// Collect all variable names that would be *bound* by a pattern (recursively).
///
/// This is semantically aware: ident patterns that resolve to known constructors
/// in the environment are NOT counted as bindings.
fn collect_pattern_binding_names(pat: &Pattern, env: &TypeEnv) -> Vec<String> {
    let mut names = Vec::new();
    collect_binding_names_recursive(pat, &mut names, env);
    names
}

fn collect_binding_names_recursive(pat: &Pattern, names: &mut Vec<String>, env: &TypeEnv) {
    match pat {
        Pattern::Ident(ident) => {
            if let Some(name_tok) = ident.name() {
                let name_text = name_tok.text().to_string();
                // Check if this name is a known constructor (not a variable binding).
                // If the name already exists in the env, it may be a constructor.
                // We use the same heuristic as infer_pattern: if it resolves to
                // a sum type (App(Con(_), _)), it's a constructor, not a binding.
                let is_constructor = env.lookup(&name_text).is_some();
                if !is_constructor {
                    names.push(name_text);
                }
            }
        }
        Pattern::Wildcard(_) | Pattern::Literal(_) => {}
        Pattern::Tuple(tuple_pat) => {
            for sub in tuple_pat.patterns() {
                collect_binding_names_recursive(&sub, names, env);
            }
        }
        Pattern::Constructor(ctor) => {
            for sub in ctor.fields() {
                collect_binding_names_recursive(&sub, names, env);
            }
        }
        Pattern::Or(or_pat) => {
            // For binding collection, use the first alternative.
            if let Some(first) = or_pat.alternatives().next() {
                collect_binding_names_recursive(&first, names, env);
            }
        }
        Pattern::As(as_pat) => {
            if let Some(inner) = as_pat.pattern() {
                collect_binding_names_recursive(&inner, names, env);
            }
            if let Some(binding) = as_pat.binding_name() {
                names.push(binding.text().to_string());
            }
        }
        Pattern::Cons(cons_pat) => {
            if let Some(head) = cons_pat.head() {
                collect_binding_names_recursive(&head, names, env);
            }
            if let Some(tail) = cons_pat.tail() {
                collect_binding_names_recursive(&tail, names, env);
            }
        }
    }
}

/// Infer an as-pattern: `Circle(r) as c`.
///
/// 1. Infer the inner pattern.
/// 2. Bind the "as" name to the inner pattern's type.
fn infer_as_pattern(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    as_pat: &mesh_parser::ast::pat::AsPat,
    pat: &Pattern,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
) -> Result<Ty, TypeError> {
    // Infer the inner pattern.
    let inner_ty = if let Some(inner_pat) = as_pat.pattern() {
        infer_pattern(ctx, env, &inner_pat, types, type_registry)?
    } else {
        ctx.fresh_var()
    };

    // Bind the "as" name to the whole matched value's type.
    if let Some(binding_name_tok) = as_pat.binding_name() {
        let binding_name = binding_name_tok.text().to_string();
        env.insert(binding_name, Scheme::mono(inner_ty.clone()));
    }

    types.insert(pat.syntax().text_range(), inner_ty.clone());
    Ok(inner_ty)
}

/// Infer a cons pattern: `head :: tail`.
///
/// The cons pattern destructures a List<T> into:
/// - head: T (the first element)
/// - tail: List<T> (the remaining list)
fn infer_cons_pattern(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    cons_pat: &mesh_parser::ast::pat::ConsPat,
    pat: &Pattern,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
) -> Result<Ty, TypeError> {
    // The overall type is List<T> where T is a fresh variable.
    let elem_ty = ctx.fresh_var();
    let list_ty = Ty::list(elem_ty.clone());

    // Infer head pattern and unify with element type T.
    if let Some(head_pat) = cons_pat.head() {
        let head_ty = infer_pattern(ctx, env, &head_pat, types, type_registry)?;
        ctx.unify(head_ty, elem_ty.clone(), ConstraintOrigin::Builtin)?;
    }

    // Infer tail pattern and unify with List<T>.
    if let Some(tail_pat) = cons_pat.tail() {
        let tail_ty = infer_pattern(ctx, env, &tail_pat, types, type_registry)?;
        ctx.unify(tail_ty, list_ty.clone(), ConstraintOrigin::Builtin)?;
    }

    types.insert(pat.syntax().text_range(), list_ty.clone());
    Ok(list_ty)
}


// ── Actor Inference (06-04) ─────────────────────────────────────────────

/// Well-known environment key for tracking the current actor's message type.
/// When inside an actor block, this is bound to `Scheme::mono(M)` where M is
/// the actor's message type. Used by `self()` and `receive` to know the
/// current actor context.
const ACTOR_MSG_TYPE_KEY: &str = "__actor_msg_type__";

/// Infer an actor definition:
///
/// ```mesh
/// actor counter(state :: Int) do
///   receive do
///     n :: Int -> counter(state + n)
///   end
/// end
/// ```
///
/// The actor's message type M is inferred from the receive block's patterns.
/// The actor is registered in the environment as a function: `actor_name :: (StateType) -> Pid<M>`.
fn infer_actor_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    actor_def: &ActorDef,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let actor_name = actor_def
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed_actor>".to_string());

    ctx.enter_level();

    // Create a fresh type variable for the message type M.
    let msg_ty = ctx.fresh_var();

    // Pre-bind the actor name as a self-recursive function (for tail calls).
    let self_var = ctx.fresh_var();
    env.insert(actor_name.clone(), Scheme::mono(self_var.clone()));

    env.push_scope();

    // Bind the actor message type for self() and receive.
    env.insert(ACTOR_MSG_TYPE_KEY.into(), Scheme::mono(msg_ty.clone()));

    // Infer parameter types.
    let mut param_types = Vec::new();
    if let Some(param_list) = actor_def.param_list() {
        for param in param_list.params() {
            let param_ty = if let Some(ann) = param.type_annotation() {
                if let Some(type_name) = resolve_type_name_str(&ann) {
                    name_to_type(&type_name)
                } else {
                    ctx.fresh_var()
                }
            } else {
                ctx.fresh_var()
            };
            if let Some(name_tok) = param.name() {
                let name_text = name_tok.text().to_string();
                env.insert(name_text, Scheme::mono(param_ty.clone()));
            }
            param_types.push(param_ty);
        }
    }

    // Infer the actor body.
    let _body_ty = if let Some(body) = actor_def.body() {
        infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
    } else {
        Ty::Tuple(vec![])
    };

    env.pop_scope();

    // The actor function type: (StateTypes...) -> Pid<M>
    let pid_ty = Ty::pid(msg_ty);
    let fn_ty = Ty::Fun(param_types, Box::new(pid_ty.clone()));

    // Unify with the pre-bound self-recursive variable.
    ctx.unify(self_var, fn_ty.clone(), ConstraintOrigin::Builtin)?;

    ctx.leave_level();
    let scheme = ctx.generalize(fn_ty.clone());
    env.insert(actor_name, scheme);

    let resolved = ctx.resolve(fn_ty);
    types.insert(actor_def.syntax().text_range(), resolved.clone());

    Ok(resolved)
}

/// Infer the type of a supervisor definition.
///
/// The supervisor is registered as a function: `supervisor_name :: () -> Pid<Unit>`.
/// Supervisors don't receive user messages; they manage child processes.
///
/// Validates child specs at compile time:
/// - Strategy must be one_for_one, one_for_all, rest_for_one, or simple_one_for_one
/// - Child start functions must return Pid (checked via spawn reference)
/// - Restart types must be permanent, transient, or temporary
/// - Shutdown values must be positive integers or brutal_kill
/// - Child names must be unique within the supervisor
fn infer_supervisor_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    sup_def: &SupervisorDef,
    types: &mut FxHashMap<TextRange, Ty>,
    _type_registry: &TypeRegistry,
    _trait_registry: &TraitRegistry,
    _fn_constraints: &mut FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let sup_name = sup_def
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed_supervisor>".to_string());

    // ── Strategy validation ──────────────────────────────────────────
    if let Some(strategy_node) = sup_def.strategy() {
        let idents: Vec<_> = strategy_node
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .collect();
        // The first IDENT is "strategy", the second is the value.
        if idents.len() >= 2 {
            let strategy_text = idents[1].text().to_string();
            match strategy_text.as_str() {
                "one_for_one" | "one_for_all" | "rest_for_one" | "simple_one_for_one" => {}
                _ => {
                    ctx.errors.push(TypeError::InvalidStrategy {
                        found: strategy_text,
                        span: idents[1].text_range(),
                    });
                }
            }
        }
    }

    // ── Child spec validation ────────────────────────────────────────
    let child_specs = sup_def.child_specs();
    let mut seen_child_names: Vec<String> = Vec::new();

    for child_node in &child_specs {
        // Extract child name.
        let child_name = child_node
            .children()
            .find(|c| c.kind() == SyntaxKind::NAME)
            .and_then(|n| {
                n.children_with_tokens()
                    .filter_map(|c| c.into_token())
                    .find(|t| t.kind() == SyntaxKind::IDENT)
                    .map(|t| t.text().to_string())
            })
            .unwrap_or_else(|| "<unnamed_child>".to_string());

        // Check for duplicate child names.
        if seen_child_names.contains(&child_name) {
            ctx.errors.push(TypeError::InvalidStrategy {
                found: format!("duplicate child name `{}`", child_name),
                span: child_node.text_range(),
            });
        }
        seen_child_names.push(child_name.clone());

        // Walk the BLOCK child for key-value validation.
        let block = child_node
            .children()
            .find(|c| c.kind() == SyntaxKind::BLOCK);

        if let Some(block) = block {
            let tokens: Vec<_> = block
                .descendants_with_tokens()
                .filter_map(|c| c.into_token())
                .collect();

            let mut i = 0;
            let mut found_start = false;

            while i < tokens.len() {
                let text = tokens[i].text();

                if text == "start" {
                    found_start = true;
                    // Validate that the start expression references a spawn call.
                    // Walk forward to find SPAWN_KW -- if it's there, the start fn returns Pid.
                    // If no SPAWN_KW is found before the next key or end, the start fn
                    // may not return Pid. We check for the spawn keyword as evidence.
                    let mut j = i + 1;
                    let mut has_spawn = false;
                    while j < tokens.len() {
                        if tokens[j].kind() == SyntaxKind::SPAWN_KW {
                            has_spawn = true;
                            break;
                        }
                        // Stop at next key boundary.
                        if tokens[j].text() == "restart"
                            || tokens[j].text() == "shutdown"
                        {
                            break;
                        }
                        j += 1;
                    }

                    if !has_spawn {
                        // Find the span of the start value for error reporting.
                        // Skip "start" and ":" to find the expression start.
                        let mut val_start = i + 1;
                        while val_start < tokens.len()
                            && tokens[val_start].kind() == SyntaxKind::COLON
                        {
                            val_start += 1;
                        }
                        let span = if val_start < j && val_start < tokens.len() {
                            // Span from first value token to last before next key.
                            let start = tokens[val_start].text_range().start();
                            let end = tokens[(j - 1).min(tokens.len() - 1)]
                                .text_range()
                                .end();
                            TextRange::new(start, end)
                        } else {
                            tokens[i].text_range()
                        };

                        ctx.errors.push(TypeError::InvalidChildStart {
                            child_name: child_name.clone(),
                            found: Ty::Con(crate::ty::TyCon::new("unknown")),
                            span,
                        });
                    }
                } else if text == "restart" {
                    // Validate restart type.
                    let mut j = i + 1;
                    while j < tokens.len() {
                        if tokens[j].kind() == SyntaxKind::IDENT
                            && tokens[j].text() != "restart"
                        {
                            let restart_text = tokens[j].text().to_string();
                            match restart_text.as_str() {
                                "permanent" | "transient" | "temporary" => {}
                                _ => {
                                    ctx.errors.push(TypeError::InvalidRestartType {
                                        found: restart_text,
                                        child_name: child_name.clone(),
                                        span: tokens[j].text_range(),
                                    });
                                }
                            }
                            break;
                        }
                        if tokens[j].kind() == SyntaxKind::COLON {
                            j += 1;
                            continue;
                        }
                        break;
                    }
                } else if text == "shutdown" {
                    // Validate shutdown value.
                    let mut j = i + 1;
                    while j < tokens.len() {
                        if tokens[j].kind() == SyntaxKind::COLON {
                            j += 1;
                            continue;
                        }
                        if tokens[j].kind() == SyntaxKind::INT_LITERAL {
                            // Valid: positive integer.
                            if let Ok(val) = tokens[j].text().parse::<i64>() {
                                if val <= 0 {
                                    ctx.errors.push(TypeError::InvalidShutdownValue {
                                        found: tokens[j].text().to_string(),
                                        child_name: child_name.clone(),
                                        span: tokens[j].text_range(),
                                    });
                                }
                            }
                            break;
                        }
                        if tokens[j].kind() == SyntaxKind::IDENT {
                            let shutdown_text = tokens[j].text().to_string();
                            if shutdown_text == "brutal_kill" {
                                // Valid.
                            } else {
                                ctx.errors.push(TypeError::InvalidShutdownValue {
                                    found: shutdown_text,
                                    child_name: child_name.clone(),
                                    span: tokens[j].text_range(),
                                });
                            }
                            break;
                        }
                        break;
                    }
                }

                i += 1;
            }

            // If no start clause was found, that's also an error (but the parser
            // should catch this, so we only flag it if we need to).
            let _ = found_start;
        }
    }

    // ── Register supervisor type ─────────────────────────────────────
    // Supervisors are zero-arg functions that return Pid<Unit>.
    let pid_ty = Ty::pid(Ty::Tuple(vec![]));
    let fn_ty = Ty::Fun(vec![], Box::new(pid_ty.clone()));

    let scheme = ctx.generalize(fn_ty.clone());
    env.insert(sup_name, scheme);

    let resolved = ctx.resolve(fn_ty);
    types.insert(sup_def.syntax().text_range(), resolved.clone());

    Ok(resolved)
}

/// Convert a PascalCase name to snake_case.
///
/// Examples: "GetCount" -> "get_count", "Increment" -> "increment",
/// "ResetAll" -> "reset_all".
fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Infer the type of a service definition.
///
/// Services define a typed client-server abstraction. The type checker:
/// 1. Infers init function return type and unifies with state type variable.
/// 2. For each call handler: validates state param, infers reply type from
///    annotation, ensures body returns (new_state, reply) tuple.
/// 3. For each cast handler: validates state param, ensures body returns new_state.
/// 4. Registers module-qualified helper functions (ServiceName.method_name).
///
/// The service is registered as a module with:
/// - start(init_args...) -> Pid<Unit>
/// - Per call handler: snake_name(pid, args...) -> reply_ty
/// - Per cast handler: snake_name(pid, args...) -> Unit
fn infer_service_def(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    service_def: &ServiceDef,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &mut FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let service_name = service_def
        .name()
        .and_then(|n| n.text())
        .unwrap_or_else(|| "<unnamed_service>".to_string());

    ctx.enter_level();

    // Create a fresh type variable for the service state.
    let state_ty = ctx.fresh_var();

    // Pid type for callers: Pid<Unit> (internal message dispatching uses type_tags,
    // callers don't see message types directly).
    let pid_ty = Ty::pid(Ty::Tuple(vec![]));

    env.push_scope();

    // ── Infer init function ──────────────────────────────────────────
    let mut init_param_types: Vec<Ty> = Vec::new();

    if let Some(init_fn) = service_def.init_fn() {
        env.push_scope();

        // Infer init parameters.
        if let Some(param_list) = init_fn.param_list() {
            for param in param_list.params() {
                let param_ty = if let Some(ann) = param.type_annotation() {
                    if let Some(type_name) = resolve_type_name_str(&ann) {
                        name_to_type(&type_name)
                    } else {
                        ctx.fresh_var()
                    }
                } else {
                    ctx.fresh_var()
                };
                if let Some(name_tok) = param.name() {
                    let name_text = name_tok.text().to_string();
                    env.insert(name_text, Scheme::mono(param_ty.clone()));
                }
                // Record param type in the types map so MIR lowering can resolve it.
                types.insert(param.syntax().text_range(), param_ty.clone());
                init_param_types.push(param_ty);
            }
        }

        // Infer init body -- its return type is the initial state.
        let init_body_ty = if let Some(body) = init_fn.body() {
            infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
        } else {
            Ty::Tuple(vec![])
        };

        // Unify init return type with state_ty.
        ctx.unify(
            init_body_ty,
            state_ty.clone(),
            ConstraintOrigin::Builtin,
        )?;

        // Record the init function's type so MIR lowering can resolve parameter types.
        let init_fn_ty = Ty::Fun(init_param_types.clone(), Box::new(ctx.resolve(state_ty.clone())));
        types.insert(init_fn.syntax().text_range(), init_fn_ty);

        env.pop_scope();
    }

    // ── Infer call handlers ──────────────────────────────────────────
    let call_handlers = service_def.call_handlers();
    let mut call_handler_info: Vec<(String, Vec<Ty>, Ty)> = Vec::new(); // (variant_name, param_types, reply_ty)

    for handler in &call_handlers {
        let variant_name = handler
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed_call>".to_string());

        env.push_scope();

        // Bind state parameter.
        if let Some(state_name) = handler.state_param_name() {
            env.insert(state_name, Scheme::mono(state_ty.clone()));
        }

        // Infer call handler parameters (the variant's arguments).
        let mut handler_param_types = Vec::new();
        if let Some(param_list) = handler.params() {
            for param in param_list.params() {
                let param_ty = if let Some(ann) = param.type_annotation() {
                    if let Some(type_name) = resolve_type_name_str(&ann) {
                        name_to_type(&type_name)
                    } else {
                        ctx.fresh_var()
                    }
                } else {
                    ctx.fresh_var()
                };
                if let Some(name_tok) = param.name() {
                    let name_text = name_tok.text().to_string();
                    env.insert(name_text, Scheme::mono(param_ty.clone()));
                }
                // Record param type for MIR lowering.
                types.insert(param.syntax().text_range(), param_ty.clone());
                handler_param_types.push(param_ty);
            }
        }

        // Parse return type annotation (:: Type).
        let reply_ty = if let Some(ann) = handler.return_type() {
            if let Some(type_name) = resolve_type_name_str(&ann) {
                name_to_type(&type_name)
            } else {
                // Try full type annotation resolution (for generic types).
                resolve_type_annotation(ctx, &ann, type_registry)
                    .unwrap_or_else(|| ctx.fresh_var())
            }
        } else {
            ctx.fresh_var()
        };

        // Infer call handler body -- should return (new_state, reply) tuple.
        let body_ty = if let Some(body) = handler.body() {
            infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
        } else {
            Ty::Tuple(vec![state_ty.clone(), reply_ty.clone()])
        };

        // Body should return a tuple of (new_state, reply).
        let expected_body_ty = Ty::Tuple(vec![state_ty.clone(), reply_ty.clone()]);
        ctx.unify(
            body_ty,
            expected_body_ty,
            ConstraintOrigin::Builtin,
        )?;

        env.pop_scope();

        call_handler_info.push((variant_name, handler_param_types, reply_ty));
    }

    // ── Infer cast handlers ──────────────────────────────────────────
    let cast_handlers = service_def.cast_handlers();
    let mut cast_handler_info: Vec<(String, Vec<Ty>)> = Vec::new(); // (variant_name, param_types)

    for handler in &cast_handlers {
        let variant_name = handler
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed_cast>".to_string());

        env.push_scope();

        // Bind state parameter.
        if let Some(state_name) = handler.state_param_name() {
            env.insert(state_name, Scheme::mono(state_ty.clone()));
        }

        // Infer cast handler parameters.
        let mut handler_param_types = Vec::new();
        if let Some(param_list) = handler.params() {
            for param in param_list.params() {
                let param_ty = if let Some(ann) = param.type_annotation() {
                    if let Some(type_name) = resolve_type_name_str(&ann) {
                        name_to_type(&type_name)
                    } else {
                        ctx.fresh_var()
                    }
                } else {
                    ctx.fresh_var()
                };
                if let Some(name_tok) = param.name() {
                    let name_text = name_tok.text().to_string();
                    env.insert(name_text, Scheme::mono(param_ty.clone()));
                }
                // Record param type for MIR lowering.
                types.insert(param.syntax().text_range(), param_ty.clone());
                handler_param_types.push(param_ty);
            }
        }

        // Infer cast handler body -- returns new_state.
        let body_ty = if let Some(body) = handler.body() {
            infer_block(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?
        } else {
            state_ty.clone()
        };

        // Unify body return with state type.
        ctx.unify(
            body_ty,
            state_ty.clone(),
            ConstraintOrigin::Builtin,
        )?;

        env.pop_scope();

        cast_handler_info.push((variant_name, handler_param_types));
    }

    env.pop_scope();

    // ── Register service module helper functions ──────────────────────

    // Register ServiceName.start(init_args...) -> Pid<Unit>
    let start_fn_ty = Ty::Fun(init_param_types, Box::new(pid_ty.clone()));
    let start_qualified = format!("{}.start", service_name);
    env.insert(start_qualified, Scheme::mono(start_fn_ty.clone()));

    // Register call helper functions: ServiceName.snake_name(pid, args...) -> reply_ty
    for (variant_name, param_types, reply_ty) in &call_handler_info {
        let snake_name = to_snake_case(variant_name);
        let mut fn_params = vec![pid_ty.clone()];
        fn_params.extend(param_types.iter().cloned());
        let resolved_reply = ctx.resolve(reply_ty.clone());
        let fn_ty = Ty::Fun(fn_params, Box::new(resolved_reply));
        let qualified = format!("{}.{}", service_name, snake_name);
        env.insert(qualified, Scheme::mono(fn_ty));
    }

    // Register cast helper functions: ServiceName.snake_name(pid, args...) -> Unit
    for (variant_name, param_types) in &cast_handler_info {
        let snake_name = to_snake_case(variant_name);
        let mut fn_params = vec![pid_ty.clone()];
        fn_params.extend(param_types.iter().cloned());
        let fn_ty = Ty::Fun(fn_params, Box::new(Ty::Tuple(vec![])));
        let qualified = format!("{}.{}", service_name, snake_name);
        env.insert(qualified, Scheme::mono(fn_ty));
    }

    // ── Build service export info for cross-module export ──────────────
    {
        use crate::ServiceExportInfo;
        use rustc_hash::FxHashMap as FxMap;

        let name_lower = service_name.to_lowercase();
        let mut info = ServiceExportInfo {
            name: service_name.clone(),
            helpers: FxMap::default(),
            methods: Vec::new(),
        };

        // Start helper
        let resolved_start = ctx.resolve(start_fn_ty.clone());
        info.helpers.insert("start".to_string(), Scheme::mono(resolved_start));
        info.methods.push(("start".to_string(), format!("__service_{}_start", name_lower)));

        // Call handler helpers
        for (variant_name, param_types, reply_ty) in &call_handler_info {
            let snake_name = to_snake_case(variant_name);
            let mut fn_params = vec![pid_ty.clone()];
            fn_params.extend(param_types.iter().cloned());
            let resolved_reply = ctx.resolve(reply_ty.clone());
            let fn_ty = Ty::Fun(fn_params, Box::new(resolved_reply));
            let resolved_fn = ctx.resolve(fn_ty);
            info.helpers.insert(snake_name.clone(), Scheme::mono(resolved_fn));
            info.methods.push((snake_name, format!("__service_{}_call_{}", name_lower, to_snake_case(variant_name))));
        }

        // Cast handler helpers
        for (variant_name, param_types) in &cast_handler_info {
            let snake_name = to_snake_case(variant_name);
            let mut fn_params = vec![pid_ty.clone()];
            fn_params.extend(param_types.iter().cloned());
            let fn_ty = Ty::Fun(fn_params, Box::new(Ty::Tuple(vec![])));
            let resolved_fn = ctx.resolve(fn_ty);
            info.helpers.insert(snake_name.clone(), Scheme::mono(resolved_fn));
            info.methods.push((snake_name, format!("__service_{}_cast_{}", name_lower, to_snake_case(variant_name))));
        }

        ctx.local_service_exports.insert(service_name.clone(), info);
    }

    ctx.leave_level();

    // The service itself is a module-like entity. Register the name so it can be
    // used as a module qualifier in ServiceName.method() calls.
    // Register as a simple type constructor for recognition in field access.
    env.insert(service_name.clone(), Scheme::mono(Ty::Con(TyCon::new(&service_name))));

    let resolved = ctx.resolve(start_fn_ty);
    types.insert(service_def.syntax().text_range(), resolved.clone());

    Ok(resolved)
}

/// Infer the type of a spawn expression: `spawn(actor_fn, initial_state...)`.
///
/// The first argument must be a function. Its return type determines the Pid
/// type. Returns `Pid<M>` where M is inferred from the actor function.
fn infer_spawn(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    spawn: &SpawnExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let arg_list = spawn.arg_list();
    let mut args: Vec<Expr> = Vec::new();
    if let Some(al) = &arg_list {
        args = al.args().collect();
    }

    if args.is_empty() {
        // spawn() with no args -- return fresh Pid.
        return Ok(Ty::pid(ctx.fresh_var()));
    }

    // First arg is the actor function reference.
    let actor_fn_expr = &args[0];
    let actor_fn_ty = infer_expr(ctx, env, actor_fn_expr, types, type_registry, trait_registry, fn_constraints)?;

    // Remaining args are initial state.
    let mut state_arg_types = Vec::new();
    for arg in args.iter().skip(1) {
        let arg_ty = infer_expr(ctx, env, arg, types, type_registry, trait_registry, fn_constraints)?;
        state_arg_types.push(arg_ty);
    }

    // The actor function should be: (StateTypes...) -> Pid<M>
    // Create the expected function type and unify.
    let msg_var = ctx.fresh_var();
    let pid_ret = Ty::pid(msg_var.clone());
    let expected_fn_ty = Ty::Fun(state_arg_types, Box::new(pid_ret.clone()));

    let resolved_fn = ctx.resolve(actor_fn_ty.clone());
    match resolved_fn {
        Ty::Fun(_, _) | Ty::Var(_) => {
            let origin = ConstraintOrigin::FnArg {
                call_site: spawn.syntax().text_range(),
                param_idx: 0,
            };
            ctx.unify(actor_fn_ty, expected_fn_ty, origin)?;
        }
        _ => {
            let err = TypeError::SpawnNonFunction {
                found: resolved_fn,
                span: spawn.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            return Err(err);
        }
    }

    Ok(pid_ret)
}

/// Infer the type of a send expression: `send(pid, message)`.
///
/// If pid is `Pid<M>`, validates that message has type M.
/// If pid is untyped `Pid`, accepts any message type.
/// Returns Unit (fire-and-forget).
fn infer_send(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    send: &SendExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let arg_list = send.arg_list();
    let mut args: Vec<Expr> = Vec::new();
    if let Some(al) = &arg_list {
        args = al.args().collect();
    }

    if args.len() < 2 {
        // Not enough arguments -- return Unit, error handled elsewhere.
        return Ok(Ty::Tuple(vec![]));
    }

    let pid_expr = &args[0];
    let msg_expr = &args[1];

    let pid_ty = infer_expr(ctx, env, pid_expr, types, type_registry, trait_registry, fn_constraints)?;
    let msg_ty = infer_expr(ctx, env, msg_expr, types, type_registry, trait_registry, fn_constraints)?;

    let resolved_pid = ctx.resolve(pid_ty);

    match &resolved_pid {
        // Typed Pid<M>: validate message type matches M.
        Ty::App(con, args) if matches!(con.as_ref(), Ty::Con(tc) if tc.name == "Pid") => {
            if let Some(expected_msg) = args.first() {
                let result = ctx.unify(
                    msg_ty.clone(),
                    expected_msg.clone(),
                    ConstraintOrigin::Builtin,
                );
                if result.is_err() {
                    let resolved_expected = ctx.resolve(expected_msg.clone());
                    let resolved_found = ctx.resolve(msg_ty);
                    let err = TypeError::SendTypeMismatch {
                        expected: resolved_expected,
                        found: resolved_found,
                        span: send.syntax().text_range(),
                    };
                    ctx.errors.push(err.clone());
                    return Err(err);
                }
            }
        }
        // Untyped Pid: accept any message type (escape hatch).
        Ty::Con(tc) if tc.name == "Pid" => {
            // No validation needed.
        }
        // Type variable: constrain to Pid<msg_ty>.
        Ty::Var(_) => {
            let _ = ctx.unify(
                resolved_pid,
                Ty::pid(msg_ty),
                ConstraintOrigin::Builtin,
            );
        }
        _ => {
            // Not a Pid at all -- type mismatch will be caught by usage context.
        }
    }

    Ok(Ty::Tuple(vec![]))
}

/// Infer the type of a receive expression.
///
/// Each arm pattern contributes to inferring the message type M.
/// All arms must return the same type. Optional after clause for timeouts.
fn infer_receive(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    recv: &ReceiveExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    // Check if we're inside an actor block.
    let in_actor = env.lookup(ACTOR_MSG_TYPE_KEY).is_some();
    if !in_actor {
        let err = TypeError::ReceiveOutsideActor {
            span: recv.syntax().text_range(),
        };
        ctx.errors.push(err.clone());
        return Err(err);
    }

    // Get the actor's message type from context.
    let actor_msg_ty = env
        .lookup(ACTOR_MSG_TYPE_KEY)
        .map(|s| ctx.instantiate(s))
        .unwrap_or_else(|| ctx.fresh_var());

    let mut result_ty: Option<Ty> = None;

    for arm in recv.arms() {
        env.push_scope();

        if let Some(pat) = arm.pattern() {
            let pat_ty = infer_pattern(ctx, env, &pat, types, type_registry)?;
            // Unify pattern type with actor message type.
            ctx.unify(pat_ty, actor_msg_ty.clone(), ConstraintOrigin::Builtin)?;
        }

        if let Some(body) = arm.body() {
            let body_ty = infer_expr(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?;
            if let Some(ref prev_ty) = result_ty {
                ctx.unify(prev_ty.clone(), body_ty.clone(), ConstraintOrigin::Builtin)?;
            } else {
                result_ty = Some(body_ty);
            }
        }

        env.pop_scope();
    }

    // Handle after (timeout) clause.
    if let Some(after) = recv.after_clause() {
        if let Some(timeout_expr) = after.timeout() {
            let timeout_ty = infer_expr(ctx, env, &timeout_expr, types, type_registry, trait_registry, fn_constraints)?;
            let _ = ctx.unify(timeout_ty, Ty::int(), ConstraintOrigin::Builtin);
        }
        if let Some(body) = after.body() {
            let body_ty = infer_expr(ctx, env, &body, types, type_registry, trait_registry, fn_constraints)?;
            if let Some(ref prev_ty) = result_ty {
                ctx.unify(prev_ty.clone(), body_ty.clone(), ConstraintOrigin::Builtin)?;
            } else {
                result_ty = Some(body_ty);
            }
        }
    }

    Ok(result_ty.unwrap_or_else(|| Ty::Tuple(vec![])))
}

/// Infer the type of a self() expression.
///
/// Returns `Pid<M>` where M is the current actor's message type.
/// Errors if called outside an actor block.
fn infer_self_expr(
    ctx: &mut InferCtx,
    env: &TypeEnv,
    self_expr: &SelfExpr,
) -> Result<Ty, TypeError> {
    match env.lookup(ACTOR_MSG_TYPE_KEY) {
        Some(scheme) => {
            let msg_ty = ctx.instantiate(scheme);
            Ok(Ty::pid(msg_ty))
        }
        None => {
            let err = TypeError::SelfOutsideActor {
                span: self_expr.syntax().text_range(),
            };
            ctx.errors.push(err.clone());
            Err(err)
        }
    }
}

/// Infer the type of a link expression: `link(pid)`.
///
/// The argument must be a Pid (typed or untyped). Returns Unit.
fn infer_link(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    link: &LinkExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    if let Some(arg_list) = link.arg_list() {
        for arg in arg_list.args() {
            let _arg_ty = infer_expr(ctx, env, &arg, types, type_registry, trait_registry, fn_constraints)?;
            // We could validate that arg_ty is a Pid, but for now we just
            // infer the type. A future refinement could add a type error.
        }
    }
    Ok(Ty::Tuple(vec![]))
}

// ── Try (?) expression inference ───────────────────────────────────────

/// Infer the type of a try expression (`expr?`).
///
/// The `?` operator works on `Result<T, E>` and `Option<T>` values:
/// - For `Result<T, E>`: extracts `T` on success, propagates `E` to enclosing function
/// - For `Option<T>`: extracts `T` on Some, propagates None to enclosing function
///
/// Validates that the enclosing function returns a compatible `Result` or `Option` type.
fn infer_try_expr(
    ctx: &mut InferCtx,
    env: &mut TypeEnv,
    try_expr: &TryExpr,
    types: &mut FxHashMap<TextRange, Ty>,
    type_registry: &TypeRegistry,
    trait_registry: &TraitRegistry,
    fn_constraints: &FxHashMap<String, FnConstraints>,
) -> Result<Ty, TypeError> {
    let span = try_expr.syntax().text_range();

    // 1. Get and infer the operand expression.
    let operand = match try_expr.operand() {
        Some(op) => op,
        None => return Ok(ctx.fresh_var()),
    };
    let operand_ty = infer_expr(ctx, env, &operand, types, type_registry, trait_registry, fn_constraints)?;
    let resolved = ctx.resolve(operand_ty.clone());

    // 2. Determine whether operand is Result<T, E> or Option<T>.
    enum TryKind {
        Result { ok_ty: Ty, err_ty: Ty },
        Option { inner_ty: Ty },
    }

    let try_kind = match &resolved {
        Ty::App(con, args) => {
            match con.as_ref() {
                Ty::Con(tc) if tc.name == "Result" && args.len() == 2 => {
                    Some(TryKind::Result {
                        ok_ty: args[0].clone(),
                        err_ty: args[1].clone(),
                    })
                }
                Ty::Con(tc) if tc.name == "Option" && args.len() == 1 => {
                    Some(TryKind::Option {
                        inner_ty: args[0].clone(),
                    })
                }
                _ => None,
            }
        }
        // Handle the case where operand is an unresolved type variable --
        // try unifying with Result<T, E> first (most common usage).
        Ty::Var(_) => {
            let fresh_t = ctx.fresh_var();
            let fresh_e = ctx.fresh_var();
            let result_ty = Ty::result(fresh_t.clone(), fresh_e.clone());
            if ctx.unify(resolved.clone(), result_ty, ConstraintOrigin::Builtin).is_ok() {
                Some(TryKind::Result {
                    ok_ty: fresh_t,
                    err_ty: fresh_e,
                })
            } else {
                // Fall back to Option<T>.
                let fresh_t2 = ctx.fresh_var();
                let option_ty = Ty::option(fresh_t2.clone());
                if ctx.unify(resolved.clone(), option_ty, ConstraintOrigin::Builtin).is_ok() {
                    Some(TryKind::Option { inner_ty: fresh_t2 })
                } else {
                    None
                }
            }
        }
        _ => None,
    };

    let try_kind = match try_kind {
        Some(k) => k,
        None => {
            // Operand is not Result or Option.
            ctx.errors.push(TypeError::TryOnNonResultOption {
                operand_ty: resolved.clone(),
                span,
            });
            return Ok(ctx.fresh_var());
        }
    };

    // 3. Check the enclosing function's return type.
    let fn_ret = ctx.current_fn_return_type().cloned();

    match &try_kind {
        TryKind::Result { ok_ty, err_ty } => {
            // Validate fn return type is Result<_, E> with compatible error type.
            if let Some(ref fn_ret_ty) = fn_ret {
                let fn_ret_resolved = ctx.resolve(fn_ret_ty.clone());
                match &fn_ret_resolved {
                    Ty::App(con, args) if matches!(con.as_ref(), Ty::Con(tc) if tc.name == "Result") && args.len() == 2 => {
                        // Try direct unification first (preserves existing behavior).
                        let err_resolved = ctx.resolve(err_ty.clone());
                        let fn_err_resolved = ctx.resolve(args[1].clone());
                        // Save error count before unify -- unify pushes errors internally
                        // and we need to undo them if a From impl exists.
                        let err_count_before = ctx.errors.len();
                        if ctx.unify(err_resolved.clone(), fn_err_resolved.clone(), ConstraintOrigin::Builtin).is_err() {
                            // Direct unification failed -- check for From impl.
                            // Only attempt From lookup when both types are concrete (not inference variables).
                            let err_is_concrete = !matches!(&err_resolved, Ty::Var(_));
                            let fn_err_is_concrete = !matches!(&fn_err_resolved, Ty::Var(_));
                            if err_is_concrete && fn_err_is_concrete {
                                if trait_registry.has_impl_with_type_args("From", &[err_resolved.clone()], &fn_err_resolved) {
                                    // From impl exists -- type check passes. Remove the
                                    // unification error that unify() pushed internally.
                                    ctx.errors.truncate(err_count_before);
                                } else {
                                    // No From impl either -- replace the unification error
                                    // with a more descriptive TryIncompatibleReturn error.
                                    ctx.errors.truncate(err_count_before);
                                    ctx.errors.push(TypeError::TryIncompatibleReturn {
                                        operand_ty: resolved.clone(),
                                        fn_return_ty: fn_ret_resolved.clone(),
                                        span,
                                    });
                                }
                            }
                            // If either type is still a variable, let inference continue
                            // (keep the unification error from ctx.unify).
                        }
                    }
                    Ty::Var(_) => {
                        // fn return type is not yet resolved -- unify it with Result<fresh, E>.
                        let fresh_ok = ctx.fresh_var();
                        let result_ret = Ty::result(fresh_ok, err_ty.clone());
                        let _ = ctx.unify(fn_ret_resolved, result_ret, ConstraintOrigin::Builtin);
                    }
                    _ => {
                        // fn return type is incompatible with ?.
                        ctx.errors.push(TypeError::TryIncompatibleReturn {
                            operand_ty: resolved.clone(),
                            fn_return_ty: fn_ret_resolved,
                            span,
                        });
                    }
                }
            }
            // else: no fn return type on stack (top-level expression) -- allow for now,
            // future plan can enforce ? only inside functions.

            Ok(ok_ty.clone())
        }
        TryKind::Option { inner_ty } => {
            // Validate fn return type is Option<_>.
            if let Some(ref fn_ret_ty) = fn_ret {
                let fn_ret_resolved = ctx.resolve(fn_ret_ty.clone());
                match &fn_ret_resolved {
                    Ty::App(con, _args) if matches!(con.as_ref(), Ty::Con(tc) if tc.name == "Option") => {
                        // Compatible -- Option fn return type.
                    }
                    Ty::Var(_) => {
                        // fn return type is not yet resolved -- unify with Option<fresh>.
                        let fresh_inner = ctx.fresh_var();
                        let option_ret = Ty::option(fresh_inner);
                        let _ = ctx.unify(fn_ret_resolved, option_ret, ConstraintOrigin::Builtin);
                    }
                    _ => {
                        // fn return type is incompatible with ?.
                        ctx.errors.push(TypeError::TryIncompatibleReturn {
                            operand_ty: resolved.clone(),
                            fn_return_ty: fn_ret_resolved,
                            span,
                        });
                    }
                }
            }

            Ok(inner_ty.clone())
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Extract where-clause constraints from a function definition.
fn extract_where_constraints(fn_: &FnDef) -> Vec<(String, String)> {
    let mut constraints = Vec::new();

    for child in fn_.syntax().children() {
        if child.kind() == SyntaxKind::WHERE_CLAUSE {
            for bound in child.children() {
                if bound.kind() == SyntaxKind::TRAIT_BOUND {
                    let tokens: Vec<_> = bound
                        .children_with_tokens()
                        .filter_map(|t| t.into_token())
                        .filter(|t| t.kind() == SyntaxKind::IDENT)
                        .collect();

                    if tokens.len() >= 2 {
                        let type_param = tokens[0].text().to_string();
                        let trait_name = tokens[1].text().to_string();
                        constraints.push((type_param, trait_name));
                    }
                }
            }
        }
    }

    constraints
}

/// Resolve a type annotation to a Ty, from the annotation's type name.
fn resolve_type_name(ann: &mesh_parser::ast::item::TypeAnnotation) -> Option<Ty> {
    let name = resolve_type_name_str(ann)?;
    Some(name_to_type(&name))
}

/// Extract the type name string from a type annotation.
fn resolve_type_name_str(ann: &mesh_parser::ast::item::TypeAnnotation) -> Option<String> {
    ann.type_name().map(|t| t.text().to_string())
}

/// Resolve a type annotation using the type registry (supports struct types, aliases).
fn resolve_type_annotation(
    _ctx: &mut InferCtx,
    ann: &mesh_parser::ast::item::TypeAnnotation,
    type_registry: &TypeRegistry,
) -> Option<Ty> {
    // Collect all significant tokens from the annotation to parse the full type.
    let mut tokens: Vec<(SyntaxKind, String)> = Vec::new();
    collect_annotation_tokens(ann.syntax(), &mut tokens);
    if tokens.is_empty() {
        return None;
    }
    // Skip leading ARROW (`->`) or COLON_COLON (`::`) prefix tokens that are
    // part of the TYPE_ANNOTATION node but not part of the type itself.
    let mut start = 0;
    if start < tokens.len() && matches!(tokens[start].0, SyntaxKind::ARROW) {
        start += 1;
    }
    if start >= tokens.len() {
        return None;
    }
    let ty = parse_type_tokens(&tokens, &mut start);
    Some(resolve_alias(ty, type_registry))
}

/// Collect significant tokens (IDENT, LT, GT, COMMA, QUESTION, BANG,
/// L_PAREN, R_PAREN) from a TYPE_ANNOTATION node tree.
fn collect_annotation_tokens(
    node: &mesh_parser::SyntaxNode,
    tokens: &mut Vec<(SyntaxKind, String)>,
) {
    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Token(t) => {
                let kind = t.kind();
                match kind {
                    SyntaxKind::IDENT | SyntaxKind::LT | SyntaxKind::GT
                    | SyntaxKind::COMMA | SyntaxKind::QUESTION | SyntaxKind::BANG
                    | SyntaxKind::L_PAREN | SyntaxKind::R_PAREN
                    | SyntaxKind::ARROW => {
                        tokens.push((kind, t.text().to_string()));
                    }
                    _ => {}
                }
            }
            rowan::NodeOrToken::Node(n) => {
                collect_annotation_tokens(&n, tokens);
            }
        }
    }
}

/// Parse a Ty from a flat list of significant tokens.
fn parse_type_tokens(tokens: &[(SyntaxKind, String)], pos: &mut usize) -> Ty {
    if *pos >= tokens.len() {
        return Ty::Never;
    }

    // Tuple: (A, B)
    if tokens[*pos].0 == SyntaxKind::L_PAREN {
        *pos += 1;
        let mut elems = Vec::new();
        while *pos < tokens.len() && tokens[*pos].0 != SyntaxKind::R_PAREN {
            elems.push(parse_type_tokens(tokens, pos));
            if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::COMMA {
                *pos += 1;
            }
        }
        if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::R_PAREN {
            *pos += 1;
        }
        let base = Ty::Tuple(elems);
        return apply_type_sugar(tokens, pos, base);
    }

    if tokens[*pos].0 != SyntaxKind::IDENT {
        return Ty::Never;
    }

    let name = tokens[*pos].1.clone();
    *pos += 1;

    // Function type: Fun(ParamTypes) -> ReturnType
    if name == "Fun" && *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::L_PAREN {
        *pos += 1; // skip (
        let mut param_tys = Vec::new();
        while *pos < tokens.len() && tokens[*pos].0 != SyntaxKind::R_PAREN {
            param_tys.push(parse_type_tokens(tokens, pos));
            if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::COMMA {
                *pos += 1;
            }
        }
        if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::R_PAREN {
            *pos += 1; // skip )
        }
        // Expect ->
        if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::ARROW {
            *pos += 1; // skip ->
        }
        let ret_ty = parse_type_tokens(tokens, pos);
        return Ty::Fun(param_tys, Box::new(ret_ty));
    }

    // Generic args: Name<A, B>
    let base = if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::LT {
        *pos += 1;
        let mut args = Vec::new();
        while *pos < tokens.len() && tokens[*pos].0 != SyntaxKind::GT {
            args.push(parse_type_tokens(tokens, pos));
            if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::COMMA {
                *pos += 1;
            }
        }
        if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::GT {
            *pos += 1;
        }
        Ty::App(Box::new(Ty::Con(TyCon::new(&name))), args)
    } else {
        name_to_type(&name)
    };

    apply_type_sugar(tokens, pos, base)
}

/// Apply sugar postfix: `?` for Option, `!` for Result.
fn apply_type_sugar(tokens: &[(SyntaxKind, String)], pos: &mut usize, base: Ty) -> Ty {
    if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::QUESTION {
        *pos += 1;
        Ty::option(base)
    } else if *pos < tokens.len() && tokens[*pos].0 == SyntaxKind::BANG {
        *pos += 1;
        let err_ty = parse_type_tokens(tokens, pos);
        Ty::result(base, err_ty)
    } else {
        base
    }
}

/// Recursively resolve type aliases.
fn resolve_alias(ty: Ty, type_registry: &TypeRegistry) -> Ty {
    match ty {
        Ty::App(con, args) => {
            if let Ty::Con(ref tc) = *con {
                if let Some(alias) = type_registry.lookup_alias(&tc.name) {
                    let resolved_args: Vec<Ty> = args
                        .into_iter()
                        .map(|a| resolve_alias(a, type_registry))
                        .collect();
                    return substitute_type_params(
                        &alias.aliased_type,
                        &alias.generic_params,
                        &resolved_args,
                    );
                }
            }
            let resolved_args: Vec<Ty> = args
                .into_iter()
                .map(|a| resolve_alias(a, type_registry))
                .collect();
            Ty::App(con, resolved_args)
        }
        Ty::Con(ref tc) => {
            if let Some(alias) = type_registry.lookup_alias(&tc.name) {
                if alias.generic_params.is_empty() {
                    return resolve_alias(alias.aliased_type.clone(), type_registry);
                }
            }
            ty
        }
        Ty::Fun(params, ret) => {
            let p: Vec<Ty> = params.into_iter().map(|p| resolve_alias(p, type_registry)).collect();
            Ty::Fun(p, Box::new(resolve_alias(*ret, type_registry)))
        }
        Ty::Tuple(elems) => {
            let e: Vec<Ty> = elems.into_iter().map(|e| resolve_alias(e, type_registry)).collect();
            Ty::Tuple(e)
        }
        _ => ty,
    }
}

/// Substitute named type parameters with concrete types.
fn substitute_type_params(ty: &Ty, param_names: &[String], param_values: &[Ty]) -> Ty {
    match ty {
        Ty::Con(tc) => {
            if let Some(idx) = param_names.iter().position(|p| *p == tc.name) {
                if idx < param_values.len() {
                    return param_values[idx].clone();
                }
            }
            ty.clone()
        }
        Ty::App(con, args) => {
            let new_con = substitute_type_params(con, param_names, param_values);
            let new_args: Vec<Ty> = args
                .iter()
                .map(|a| substitute_type_params(a, param_names, param_values))
                .collect();
            Ty::App(Box::new(new_con), new_args)
        }
        Ty::Fun(params, ret) => {
            let p: Vec<Ty> = params.iter().map(|p| substitute_type_params(p, param_names, param_values)).collect();
            Ty::Fun(p, Box::new(substitute_type_params(ret, param_names, param_values)))
        }
        Ty::Tuple(elems) => {
            let e: Vec<Ty> = elems.iter().map(|e| substitute_type_params(e, param_names, param_values)).collect();
            Ty::Tuple(e)
        }
        _ => ty.clone(),
    }
}

/// Convert a type name string to a Ty.
fn name_to_type(name: &str) -> Ty {
    match name {
        "Int" => Ty::int(),
        "Float" => Ty::float(),
        "String" => Ty::string(),
        "Bool" => Ty::bool(),
        other => Ty::Con(TyCon::new(other)),
    }
}

/// Check if a type is an unresolved type variable.
fn is_type_var(ty: &Ty) -> bool {
    matches!(ty, Ty::Var(_))
}
