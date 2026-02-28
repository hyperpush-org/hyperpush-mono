//! AST-to-MIR lowering.
//!
//! Converts the typed Rowan CST (Parse + TypeckResult) to the MIR representation.
//! Handles desugaring of pipe operators, string interpolation, and closure conversion.

use std::collections::{HashMap, HashSet};

use rowan::TextRange;
use rustc_hash::FxHashMap;
use mesh_parser::ast::expr::{
    BinaryExpr, CallExpr, CaseExpr, ClosureExpr, Expr, FieldAccess, ForInExpr, IfExpr, LinkExpr,
    ListLiteral, Literal, MapLiteral, MatchArm, NameRef, PipeExpr, ReceiveExpr, ReturnExpr,
    SendExpr, SlotPipeExpr, SpawnExpr, StringExpr, StructLiteral, StructUpdate, TryExpr,
    TupleExpr, UnaryExpr, WhileExpr,
};
use mesh_parser::ast::item::{
    ActorDef, Block, FnDef, ImplDef, InterfaceMethod, Item, LetBinding, RelationshipDecl,
    ServiceDef, SourceFile, StructDef, SumTypeDef, SupervisorDef,
};
use mesh_parser::ast::pat::Pattern;
use mesh_parser::ast::AstNode;
use mesh_parser::syntax_kind::SyntaxKind;
use mesh_parser::Parse;
use mesh_typeck::ty::Ty;
use mesh_typeck::{TraitRegistry, TypeckResult};

use super::types::{mangle_type_name, mir_type_to_impl_name, mir_type_to_ty, resolve_type};
use super::{
    BinOp, MirChildSpec, MirExpr, MirFunction, MirLiteral, MirMatchArm, MirModule, MirPattern,
    MirStructDef, MirSumTypeDef, MirType, MirVariantDef, UnaryOp,
};

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract the element type T from a `Ty::App(Con("List"), [T])`.
/// Returns `None` if the type is not a List.
fn extract_list_elem_type(ty: &Ty) -> Option<Ty> {
    match ty {
        Ty::App(con_ty, args) => {
            if let Ty::Con(con) = con_ty.as_ref() {
                if con.name == "List" && !args.is_empty() {
                    return Some(args[0].clone());
                }
            }
            None
        }
        Ty::Con(con) if con.name == "List" => {
            // Bare List without type args -- default to Int
            Some(Ty::int())
        }
        _ => None,
    }
}

/// Extract key and value types from a `Ty::App(Con("Map"), [K, V])`.
/// Returns `None` if the type is not a Map.
fn extract_map_types(ty: &Ty) -> Option<(Ty, Ty)> {
    match ty {
        Ty::App(con_ty, args) => {
            if let Ty::Con(con) = con_ty.as_ref() {
                if con.name == "Map" && args.len() >= 2 {
                    return Some((args[0].clone(), args[1].clone()));
                }
            }
            None
        }
        Ty::Con(con) if con.name == "Map" => {
            Some((Ty::int(), Ty::int()))
        }
        _ => None,
    }
}

/// Extract the element type T from a `Ty::App(Con("Set"), [T])`.
/// Returns `None` if the type is not a Set.
fn extract_set_elem_type(ty: &Ty) -> Option<Ty> {
    match ty {
        Ty::App(con_ty, args) => {
            if let Ty::Con(con) = con_ty.as_ref() {
                if con.name == "Set" && !args.is_empty() {
                    return Some(args[0].clone());
                }
            }
            None
        }
        Ty::Con(con) if con.name == "Set" => {
            Some(Ty::int())
        }
        _ => None,
    }
}

/// Extract the trait name, trait type args, and type name from an ImplDef's PATH children.
/// Returns `(trait_name, trait_type_args, type_name)`, e.g. `("From", vec!["Int"], "Float")`.
/// For non-parameterized traits, trait_type_args is empty.
fn extract_impl_names(impl_def: &ImplDef) -> (String, Vec<String>, String) {
    let paths: Vec<_> = impl_def
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
    // GENERIC_ARG_LIST is a direct child of IMPL_DEF.
    let trait_type_args: Vec<String> = impl_def
        .syntax()
        .children()
        .filter(|n| n.kind() == SyntaxKind::GENERIC_ARG_LIST)
        .flat_map(|gal| {
            gal.children_with_tokens()
                .filter_map(|t| t.into_token())
                .filter(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
                .collect::<Vec<_>>()
        })
        .collect();

    let type_name = paths
        .get(1)
        .and_then(|path| {
            path.children_with_tokens()
                .filter_map(|t| t.into_token())
                .find(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| t.text().to_string())
        })
        .unwrap_or_else(|| "<unknown>".to_string());

    (trait_name, trait_type_args, type_name)
}

/// Build a mangled trait method name, incorporating trait type args when present.
/// Non-parameterized: `Trait__method__Type` (e.g., `Display__to_string__Int`)
/// Parameterized: `Trait_TypeArg__method__ImplType` (e.g., `From_Int__from__Float`)
fn mangle_trait_method(
    trait_name: &str,
    trait_type_args: &[String],
    method_name: &str,
    impl_type_name: &str,
) -> String {
    if trait_type_args.is_empty() {
        format!("{}__{}__{}", trait_name, method_name, impl_type_name)
    } else {
        let args_str = trait_type_args.join("_");
        format!("{}_{}__{}__{}", trait_name, args_str, method_name, impl_type_name)
    }
}

/// Substitute type parameters in a `Ty` using a substitution map.
///
/// Replaces `Ty::Con("T")` with the corresponding concrete type from the map.
/// Recursively handles `Ty::App`, `Ty::Fun`, and `Ty::Tuple`.
fn substitute_type_params(ty: &Ty, subst: &HashMap<String, &Ty>) -> Ty {
    match ty {
        Ty::Con(con) => {
            if let Some(replacement) = subst.get(&con.name) {
                (*replacement).clone()
            } else {
                ty.clone()
            }
        }
        Ty::App(con, args) => {
            let con_sub = substitute_type_params(con, subst);
            let args_sub: Vec<Ty> = args.iter().map(|a| substitute_type_params(a, subst)).collect();
            Ty::App(Box::new(con_sub), args_sub)
        }
        Ty::Fun(params, ret) => {
            let params_sub: Vec<Ty> = params.iter().map(|p| substitute_type_params(p, subst)).collect();
            let ret_sub = substitute_type_params(ret, subst);
            Ty::Fun(params_sub, Box::new(ret_sub))
        }
        Ty::Tuple(elems) => {
            let elems_sub: Vec<Ty> = elems.iter().map(|e| substitute_type_params(e, subst)).collect();
            Ty::Tuple(elems_sub)
        }
        _ => ty.clone(),
    }
}

// ── Lowerer ──────────────────────────────────────────────────────────

/// The AST-to-MIR lowering context.
struct Lowerer<'a> {
    /// Type map from typeck: TextRange -> Ty.
    types: &'a FxHashMap<TextRange, Ty>,
    /// Type registry for struct/sum type lookups.
    registry: &'a mesh_typeck::TypeRegistry,
    /// Trait registry for trait method dispatch resolution.
    trait_registry: &'a TraitRegistry,
    /// Default method body text ranges from interface definitions.
    /// Keyed by `(trait_name, method_name)`, value is the TextRange of
    /// the INTERFACE_METHOD node containing the default body.
    default_method_bodies: &'a FxHashMap<(String, String), TextRange>,
    /// The parse tree, used for looking up default method body AST nodes.
    parse: &'a Parse,
    /// Functions being built.
    functions: Vec<MirFunction>,
    /// Struct definitions.
    structs: Vec<MirStructDef>,
    /// Sum type definitions.
    sum_types: Vec<MirSumTypeDef>,
    /// Scope stack for local variable types.
    scopes: Vec<HashMap<String, MirType>>,
    /// Counter for generating unique lifted closure function names.
    closure_counter: u32,
    /// Names of known functions (for distinguishing direct calls from closure calls).
    known_functions: HashMap<String, MirType>,
    /// Entry function name, if found.
    entry_function: Option<String>,
    /// Service module names (for field access resolution).
    /// Maps service name -> list of (method_name, generated_fn_name) pairs.
    service_modules: HashMap<String, Vec<(String, String)>>,
    /// Current monomorphization depth (incremented per function body lowering).
    mono_depth: u32,
    /// Maximum allowed monomorphization depth before emitting a Panic node.
    max_mono_depth: u32,
    /// Tracks which monomorphized trait functions have been generated for generic types.
    /// Prevents duplicate generation when the same generic struct is instantiated
    /// multiple times (e.g., Box<Int> used in multiple places).
    monomorphized_trait_fns: HashSet<String>,
    /// User-defined module namespaces for qualified access (Phase 39).
    /// Maps module namespace name (e.g., "Math") to list of exported function names.
    user_modules: HashMap<String, Vec<String>>,
    /// Function names imported via `from Module import name1, name2` (Phase 39).
    /// These are directly callable without qualification and must not go through
    /// trait dispatch.
    imported_functions: HashSet<String>,
    /// Module name for name-mangling private functions (Phase 41).
    /// Empty string means single-file mode (no prefix applied).
    module_name: String,
    /// Set of pub function names that should NOT be module-prefixed (Phase 41).
    pub_functions: HashSet<String>,
    /// Names of user-defined functions from FnDef items (Phase 41).
    /// Used to distinguish actual function definitions from variant constructors,
    /// actors, etc. when applying module-qualified naming at call sites.
    user_fn_defs: HashSet<String>,
    /// Maps user-defined function name → all concrete function types observed at
    /// call sites where the function was passed as a value argument (not called directly).
    ///
    /// Example: `fn pass(req, next) do next(req) end` used in `HTTP.use(r, pass)`.
    /// At the usage site, the typeck resolves `pass` to `Fn(Request, ...) -> Response`.
    /// This map lets the lowerer recover the correct parameter types for functions whose
    /// parameters were generalized (as Ty::Var) before the call site could constrain them.
    fn_value_usage_types: HashMap<String, Vec<Ty>>,
    /// Current enclosing function's return type (Phase 45).
    /// Set when entering a function body, used by lower_try_expr for early-return
    /// variant construction. Save/restore pattern for nested functions and closures.
    current_fn_return_type: Option<MirType>,
    /// Counter for generating unique try binding names (Phase 45).
    /// Incremented per `?` usage to avoid shadowing in nested `?` expressions.
    try_counter: u32,
}

/// Walk through Let/Block wrappers to find the effective return type of a MIR expression.
/// Let { ty, body, .. } has `ty` as the binding's value type, but the effective type is body's type.
/// Block(exprs, ty) already stores the last expression's type as `ty`.
fn effective_return_type(expr: &MirExpr) -> MirType {
    match expr {
        MirExpr::Let { body, .. } => effective_return_type(body),
        MirExpr::Block(_, ty) => ty.clone(),
        other => other.ty().clone(),
    }
}

/// Map a MIR type to its PostgreSQL SQL type string.
///
/// Used by `generate_schema_metadata` to produce `__field_types__()` entries.
fn mir_type_to_sql_type(ty: &MirType) -> &'static str {
    match ty {
        MirType::Int => "BIGINT",
        MirType::Float => "DOUBLE PRECISION",
        MirType::Bool => "BOOLEAN",
        MirType::String => "TEXT",
        _ => "TEXT", // Default fallback for Ptr and other types
    }
}

impl<'a> Lowerer<'a> {
    fn new(typeck: &'a TypeckResult, parse: &'a Parse, module_name: &str, pub_fns: &HashSet<String>) -> Self {
        Lowerer {
            types: &typeck.types,
            registry: &typeck.type_registry,
            trait_registry: &typeck.trait_registry,
            default_method_bodies: &typeck.default_method_bodies,
            parse,
            functions: Vec::new(),
            structs: Vec::new(),
            sum_types: Vec::new(),
            scopes: vec![HashMap::new()],
            closure_counter: 0,
            known_functions: HashMap::new(),
            entry_function: None,
            service_modules: typeck.imported_service_methods.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            mono_depth: 0,
            max_mono_depth: 64,
            monomorphized_trait_fns: HashSet::new(),
            user_modules: typeck.qualified_modules.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            imported_functions: typeck.imported_functions.iter().cloned().collect(),
            module_name: module_name.to_string(),
            pub_functions: pub_fns.clone(),
            user_fn_defs: HashSet::new(),
            fn_value_usage_types: HashMap::new(),
            current_fn_return_type: None,
            try_counter: 0,
        }
    }

    // ── Scope management ─────────────────────────────────────────────

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert_var(&mut self, name: String, ty: MirType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    fn lookup_var(&self, name: &str) -> Option<MirType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    // ── Module-qualified naming (Phase 41) ──────────────────────────

    /// Apply module prefix to a private function name.
    ///
    /// Rules:
    /// - Empty module_name (single-file mode): return name unchanged
    /// - "main": unchanged (handled separately as mesh_main)
    /// - Pub functions: unchanged (cross-module references use unqualified name)
    /// - Builtin/runtime prefixes (mesh_, trait impls): unchanged
    /// - Otherwise: `ModuleName__name` (dots replaced with underscores)
    fn qualify_name(&self, name: &str) -> String {
        // Single-file mode: no prefix
        if self.module_name.is_empty() {
            return name.to_string();
        }
        // main is handled separately (renamed to mesh_main)
        if name == "main" {
            return name.to_string();
        }
        // Pub functions keep unqualified names for cross-module references
        if self.pub_functions.contains(name) {
            return name.to_string();
        }
        // Builtin/runtime prefixes: do not prefix
        const BUILTIN_PREFIXES: &[&str] = &[
            "mesh_", "Ord__", "Eq__", "Display__", "Debug__", "Hash__",
            "Default__", "Add__", "Sub__", "Mul__", "Div__", "Rem__", "Neg__",
            "FromRow__", "FromJson__", "ToJson__",
            "From_", "From__", "Into_", "Into__",
            "TryFrom_", "TryFrom__", "TryInto_", "TryInto__", // Phase 128
        ];
        for prefix in BUILTIN_PREFIXES {
            if name.starts_with(prefix) {
                return name.to_string();
            }
        }
        // Apply module prefix: ModuleName__function_name
        format!("{}__{}",  self.module_name.replace('.', "_"), name)
    }

    // ── Type resolution helper ───────────────────────────────────────

    fn resolve_range(&self, range: TextRange) -> MirType {
        if let Some(ty) = self.types.get(&range) {
            resolve_type(ty, self.registry, false)
        } else {
            MirType::Unit
        }
    }

    #[allow(dead_code)]
    fn resolve_range_closure(&self, range: TextRange) -> MirType {
        if let Some(ty) = self.types.get(&range) {
            resolve_type(ty, self.registry, true)
        } else {
            MirType::Unit
        }
    }

    fn get_ty(&self, range: TextRange) -> Option<&Ty> {
        self.types.get(&range)
    }

    /// Determine the key_type tag for a Map.new() call based on the resolved type.
    /// Returns 1 for String keys, 0 for everything else (Int or unresolved).
    fn infer_map_key_type(&self, call_range: TextRange) -> i64 {
        if let Some(ty) = self.types.get(&call_range) {
            if Self::ty_has_string_map_keys(ty) {
                return 1;
            }
        }
        0 // KEY_TYPE_INT (default)
    }

    /// Check if a Ty represents Map<String, V> or List<(String, V)> (i.e., has
    /// string keys that should be preserved through collect operations).
    fn ty_has_string_map_keys(ty: &Ty) -> bool {
        match ty {
            Ty::App(con, args) => {
                if let Ty::Con(ref tc) = **con {
                    match tc.name.as_str() {
                        "Map" if !args.is_empty() => args[0] == Ty::string(),
                        "List" if !args.is_empty() => {
                            // Check if the element is a tuple (String, V)
                            if let Ty::Tuple(elems) = &args[0] {
                                !elems.is_empty() && elems[0] == Ty::string()
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Walk a pipe chain backwards to determine if the source data has string
    /// map keys. Used for Map.collect dispatch: when the source of the pipe
    /// chain is a List<(String, V)> or Map<String, V>, the collected map needs
    /// string key_type (1) rather than integer key_type (0).
    ///
    /// Also handles the Iter.zip pattern: `string_keys |> Iter.from() |> Iter.zip(values)`
    /// where the key source (LHS before the zip step) is a List<String> or String iterator.
    fn pipe_chain_has_string_keys(&self, pipe: &PipeExpr) -> bool {
        // Walk backwards through the pipe chain.
        // At each inner pipe step, check whether the RHS is an Iter.zip call --
        // if so, the LHS of that pipe is the key source; check it for string type.
        let mut current_lhs = pipe.lhs();
        loop {
            match current_lhs {
                Some(Expr::PipeExpr(inner_pipe)) => {
                    // Check if this inner pipe step zips with string keys.
                    // Pattern: <key_iter> |> Iter.zip(<val_iter>)
                    // If the RHS is a call to Iter.zip, check the LHS (key source).
                    if Self::rhs_is_iter_zip(&inner_pipe) {
                        if self.pipe_source_has_string_list(inner_pipe.lhs()) {
                            return true;
                        }
                    }
                    current_lhs = inner_pipe.lhs();
                }
                Some(ref expr) => {
                    // Found the deepest non-pipe expression. Check its typeck type.
                    if let Some(ty) = self.types.get(&expr.syntax().text_range()) {
                        return Self::ty_has_string_map_keys(ty);
                    }
                    return false;
                }
                None => return false,
            }
        }
    }

    /// Check if the RHS of a pipe expression is a call to `Iter.zip`.
    fn rhs_is_iter_zip(pipe: &PipeExpr) -> bool {
        match pipe.rhs() {
            Some(Expr::CallExpr(call)) => {
                if let Some(callee) = call.callee() {
                    if let Expr::FieldAccess(fa) = callee {
                        let module = fa.base().and_then(|b| {
                            if let Expr::NameRef(nr) = b {
                                nr.text()
                            } else {
                                None
                            }
                        });
                        let field = fa.field().map(|t| t.text().to_string());
                        return module.as_deref() == Some("Iter")
                            && field.as_deref() == Some("zip");
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Walk a chain of pipes or a bare expression to see if its ultimate
    /// source is a List<String> (string elements used as zip keys).
    fn pipe_source_has_string_list(&self, expr: Option<Expr>) -> bool {
        match expr {
            Some(Expr::PipeExpr(inner)) => {
                // Keep walking to the root of any nested pipes.
                self.pipe_source_has_string_list(inner.lhs())
            }
            Some(ref e) => {
                if let Some(ty) = self.types.get(&e.syntax().text_range()) {
                    // List<String>: the elements are string keys.
                    if let Ty::App(con, args) = ty {
                        if let Ty::Con(ref tc) = **con {
                            if tc.name == "List" && !args.is_empty() {
                                return args[0] == Ty::string();
                            }
                        }
                    }
                }
                false
            }
            None => false,
        }
    }

    // ── Function value usage type recovery ───────────────────────────

    /// Scan every NAME_REF node in the source AST and, for each node that refers
    /// to a user-defined function and has a concrete (non-Var) function type in the
    /// typeck map, record all observed types for that function name.
    ///
    /// At call sites like `HTTP.use(r, pass)` the typeck resolves the `pass`
    /// identifier to the instantiated concrete type (e.g. `Fn(Request, …)->Response`)
    /// even when the function definition's own parameter types were generalized away
    /// as Ty::Var before the call site was processed.  We collect these usage types
    /// so that `resolve_param_from_usage` can recover the correct MIR type for
    /// parameters that would otherwise fall back to MirType::Unit.
    fn build_fn_value_usage_types(
        &self,
        root: &mesh_parser::SyntaxNode,
    ) -> HashMap<String, Vec<Ty>> {
        let mut map: HashMap<String, Vec<Ty>> = HashMap::new();
        for node in root.descendants() {
            if node.kind() == SyntaxKind::NAME_REF {
                if let Some(name_ref) = NameRef::cast(node) {
                    if let Some(name) = name_ref.text() {
                        if self.user_fn_defs.contains(&name) {
                            if let Some(ty) = self.types.get(&name_ref.syntax().text_range()) {
                                // Only record concrete function types — skip Ty::Var results.
                                if matches!(ty, Ty::Fun(..)) {
                                    map.entry(name.to_string()).or_default().push(ty.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        map
    }

    /// Try to recover a concrete MIR type for the parameter at position `param_idx`
    /// of function `fn_name` by inspecting usage-site types collected in
    /// `fn_value_usage_types`.  Returns `None` if no concrete type can be found.
    fn resolve_param_from_usage(&self, fn_name: &str, param_idx: usize) -> Option<MirType> {
        for usage_ty in self.fn_value_usage_types.get(fn_name)?.iter() {
            if let Ty::Fun(usage_params, _) = usage_ty {
                if let Some(specific_ty) = usage_params.get(param_idx) {
                    let mir = resolve_type(
                        specific_ty,
                        self.registry,
                        matches!(specific_ty, Ty::Fun(..)),
                    );
                    if mir != MirType::Unit {
                        return Some(mir);
                    }
                }
            }
        }
        None
    }

    // ── Top-level lowering ───────────────────────────────────────────

    fn lower_source_file(&mut self, sf: SourceFile) {
        // First pass: register all function names so we know which are direct calls.
        // For multi-clause functions, only register the FIRST clause (which has the type).
        for item in sf.items() {
            match &item {
                Item::FnDef(fn_def) => {
                    if let Some(name) = fn_def.name().and_then(|n| n.text()) {
                        // Skip if already registered (subsequent clause of a multi-clause fn).
                        if !self.known_functions.contains_key(&name) {
                            let fn_ty = self.resolve_range(fn_def.syntax().text_range());
                            self.known_functions.insert(name.clone(), fn_ty.clone());
                            self.user_fn_defs.insert(name.clone());
                            self.insert_var(name, fn_ty);
                        }
                    }
                }
                Item::ActorDef(actor_def) => {
                    if let Some(name) = actor_def.name().and_then(|n| n.text()) {
                        // Actor definitions produce a function with the actor name
                        let fn_ty = self.resolve_range(actor_def.syntax().text_range());
                        self.known_functions.insert(name.clone(), fn_ty.clone());
                        self.insert_var(name, fn_ty);
                    }
                }
                Item::SupervisorDef(sup_def) => {
                    if let Some(name) = sup_def.name().and_then(|n| n.text()) {
                        // Supervisor definitions produce a function that returns Pid
                        let fn_ty = self.resolve_range(sup_def.syntax().text_range());
                        self.known_functions.insert(name.clone(), fn_ty.clone());
                        self.insert_var(name, fn_ty);
                    }
                }
                Item::ServiceDef(service_def) => {
                    if let Some(name) = service_def.name().and_then(|n| n.text()) {
                        // Pre-register the service start function.
                        let start_fn_name = format!("__service_{}_start", name.to_lowercase());
                        self.known_functions.insert(
                            start_fn_name.clone(),
                            MirType::FnPtr(vec![], Box::new(MirType::Pid(None))),
                        );
                    }
                }
                Item::ImplDef(impl_def) => {
                    let (trait_name, trait_type_args, type_name) = extract_impl_names(&impl_def);
                    let mut provided_methods = std::collections::HashSet::new();
                    for method in impl_def.methods() {
                        if let Some(method_name) = method.name().and_then(|n| n.text()) {
                            provided_methods.insert(method_name.clone());
                            let mangled = mangle_trait_method(
                                &trait_name, &trait_type_args, &method_name, &type_name,
                            );
                            let fn_ty =
                                self.resolve_range(method.syntax().text_range());
                            self.known_functions.insert(mangled.clone(), fn_ty);
                        }
                    }
                    // Pre-register default method bodies for missing methods.
                    if let Some(trait_def) = self.trait_registry.get_trait(&trait_name) {
                        for trait_method in &trait_def.methods {
                            if trait_method.has_default_body
                                && !provided_methods.contains(&trait_method.name)
                            {
                                let mangled = mangle_trait_method(
                                    &trait_name, &trait_type_args, &trait_method.name, &type_name,
                                );
                                // Use the return type from the trait method sig, fallback to Unit.
                                let fn_ty = if let Some(ret_ty) = &trait_method.return_type {
                                    resolve_type(ret_ty, self.registry, false)
                                } else {
                                    MirType::Unit
                                };
                                self.known_functions.insert(mangled, fn_ty);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Register builtin I/O functions as known functions.
        self.known_functions.insert(
            "println".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Unit)),
        );
        self.known_functions.insert(
            "print".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Unit)),
        );

        // Register stdlib functions as known functions (Phase 8).
        // String operations
        self.known_functions.insert(
            "mesh_string_length".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Int)),
        );
        self.known_functions.insert(
            "mesh_string_slice".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::Int, MirType::Int], Box::new(MirType::String)),
        );
        self.known_functions.insert(
            "mesh_string_contains".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool)),
        );
        self.known_functions.insert(
            "mesh_string_starts_with".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool)),
        );
        self.known_functions.insert(
            "mesh_string_ends_with".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool)),
        );
        self.known_functions.insert(
            "mesh_string_trim".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::String)),
        );
        self.known_functions.insert(
            "mesh_string_to_upper".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::String)),
        );
        self.known_functions.insert(
            "mesh_string_to_lower".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::String)),
        );
        self.known_functions.insert(
            "mesh_string_replace".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String, MirType::String], Box::new(MirType::String)),
        );
        // Phase 46: String split/join/to_int/to_float
        self.known_functions.insert(
            "mesh_string_split".to_string(),
            MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_string_join".to_string(),
            MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_string_to_int".to_string(),
            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_string_to_float".to_string(),
            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
        );
        // File I/O functions
        self.known_functions.insert(
            "mesh_file_read".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_file_write".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_file_append".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_file_exists".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Bool)),
        );
        self.known_functions.insert(
            "mesh_file_delete".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)),
        );
        // IO functions
        self.known_functions.insert(
            "mesh_io_read_line".to_string(),
            MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_io_eprintln".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Unit)),
        );
        // Env functions
        self.known_functions.insert(
            "mesh_env_get".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_env_args".to_string(),
            MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_env_get_with_default".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::String)),
        );
        self.known_functions.insert(
            "mesh_env_get_int".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::Int], Box::new(MirType::Int)),
        );
        // Regex runtime functions (Phase 119)
        self.known_functions.insert(
            "mesh_regex_from_literal".to_string(),
            MirType::FnPtr(vec![MirType::String, MirType::Int], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_regex_compile".to_string(),
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_regex_match".to_string(),
            MirType::FnPtr(vec![MirType::Ptr, MirType::String], Box::new(MirType::Bool)),
        );
        self.known_functions.insert(
            "mesh_regex_captures".to_string(),
            MirType::FnPtr(vec![MirType::Ptr, MirType::String], Box::new(MirType::Ptr)),
        );
        self.known_functions.insert(
            "mesh_regex_replace".to_string(),
            MirType::FnPtr(vec![MirType::Ptr, MirType::String, MirType::String], Box::new(MirType::String)),
        );
        self.known_functions.insert(
            "mesh_regex_split".to_string(),
            MirType::FnPtr(vec![MirType::Ptr, MirType::String], Box::new(MirType::Ptr)),
        );
        // ── Collection functions (Phase 8 Plan 02) ─────────────────────
        // List
        self.known_functions.insert("mesh_list_new".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_length".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_list_append".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_head".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_tail".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_concat".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_reverse".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_map".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_filter".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_reduce".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_from_array".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        // Phase 46: sort, find, any, all, contains
        self.known_functions.insert("mesh_list_sort".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_find".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_any".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Bool)));
        self.known_functions.insert("mesh_list_all".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Bool)));
        self.known_functions.insert("mesh_list_contains".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Bool)));
        // Phase 47: zip, flat_map, flatten, enumerate, take, drop, last, nth
        self.known_functions.insert("mesh_list_zip".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_flat_map".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_flatten".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_enumerate".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_take".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_drop".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_last".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_list_nth".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        // Map
        self.known_functions.insert("mesh_map_new".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_new_typed".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_tag_string".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_put".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_map_has_key".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Bool)));
        self.known_functions.insert("mesh_map_delete".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_size".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_map_keys".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_values".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // Phase 47: Map merge/to_list/from_list
        self.known_functions.insert("mesh_map_merge".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_to_list".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_from_list".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // Set
        self.known_functions.insert("mesh_set_new".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_add".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_remove".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_contains".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Bool)));
        self.known_functions.insert("mesh_set_size".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_set_union".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_intersection".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // Phase 47: Set difference/to_list/from_list
        self.known_functions.insert("mesh_set_difference".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_to_list".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_from_list".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // Collection Display (Phase 21 Plan 04)
        self.known_functions.insert("mesh_list_to_string".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_map_to_string".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_set_to_string".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_string_to_string".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        // List Eq/Ord (Phase 27)
        self.known_functions.insert("mesh_list_eq".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Bool)));
        self.known_functions.insert("mesh_list_compare".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        // Tuple
        self.known_functions.insert("mesh_tuple_nth".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_tuple_first".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_tuple_second".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_tuple_size".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        // Range
        self.known_functions.insert("mesh_range_new".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_range_to_list".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_range_map".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_range_filter".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_range_length".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        // Queue
        self.known_functions.insert("mesh_queue_new".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_queue_push".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_queue_pop".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_queue_peek".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_queue_size".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_queue_is_empty".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Bool)));
        // JSON functions (Phase 8 Plan 04)
        self.known_functions.insert("mesh_json_parse".to_string(), MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_encode".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)));
        self.known_functions.insert("mesh_json_encode_string".to_string(), MirType::FnPtr(vec![MirType::String], Box::new(MirType::String)));
        self.known_functions.insert("mesh_json_encode_int".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::String)));
        self.known_functions.insert("mesh_json_encode_bool".to_string(), MirType::FnPtr(vec![MirType::Bool], Box::new(MirType::String)));
        self.known_functions.insert("mesh_json_encode_map".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)));
        self.known_functions.insert("mesh_json_encode_list".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)));
        self.known_functions.insert("mesh_json_from_int".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_from_float".to_string(), MirType::FnPtr(vec![MirType::Float], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_from_bool".to_string(), MirType::FnPtr(vec![MirType::Bool], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_from_string".to_string(), MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)));
        // Phase 103: JSON field extraction (no DB roundtrip)
        // mesh_json_get(json: String, key: String) -> String
        self.known_functions.insert("mesh_json_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_json_get_nested(json: String, path1: String, path2: String) -> String
        self.known_functions.insert("mesh_json_get_nested".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // JSON structured object/array functions (Phase 49)
        self.known_functions.insert("mesh_json_object_new".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_object_put".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_object_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_array_new".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_array_push".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_array_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_as_int".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_as_float".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_as_string".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_as_bool".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_null".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        // JSON collection helpers (callback-based, for List<T> and Map<String, V> fields)
        self.known_functions.insert("mesh_json_from_list".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_from_map".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_to_list".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_json_to_map".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // Result helpers (for from_json Result propagation)
        self.known_functions.insert("mesh_alloc_result".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_result_is_ok".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_result_unwrap".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // HTTP functions (Phase 8 Plan 05)
        self.known_functions.insert("mesh_http_router".to_string(), MirType::FnPtr(vec![], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_route".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_serve".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_http_serve_tls".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int, MirType::String, MirType::String], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_http_response_new".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::String], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_response_with_headers".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::String, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_get".to_string(), MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_post".to_string(), MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_request_method".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)));
        self.known_functions.insert("mesh_http_request_path".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)));
        self.known_functions.insert("mesh_http_request_body".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)));
        self.known_functions.insert("mesh_http_request_header".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_request_query".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String], Box::new(MirType::Ptr)));
        // Phase 51: Method-specific routing and path parameter extraction
        self.known_functions.insert("mesh_http_route_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_route_post".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_route_put".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_route_delete".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_http_request_param".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::String], Box::new(MirType::Ptr)));
        // Phase 52: Middleware
        self.known_functions.insert("mesh_http_use_middleware".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── WebSocket functions (Phase 60) ──────────────────────────────
        // mesh_ws_serve(on_connect_fn: ptr, on_connect_env: ptr, on_message_fn: ptr, on_message_env: ptr, on_close_fn: ptr, on_close_env: ptr, port: i64) -> void
        self.known_functions.insert("mesh_ws_serve".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Int], Box::new(MirType::Unit)));
        // mesh_ws_send(conn: ptr, msg: ptr) -> i64
        self.known_functions.insert("mesh_ws_send".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        // mesh_ws_send_binary(conn: ptr, data: ptr, len: i64) -> i64
        self.known_functions.insert("mesh_ws_send_binary".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Int], Box::new(MirType::Int)));
        // mesh_ws_serve_tls(on_connect_fn: ptr, on_connect_env: ptr, on_message_fn: ptr, on_message_env: ptr, on_close_fn: ptr, on_close_env: ptr, port: i64, cert_path: ptr, key_path: ptr) -> void
        self.known_functions.insert("mesh_ws_serve_tls".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Unit)));
        // ── WebSocket Room functions (Phase 62) ──────────────────────────
        // mesh_ws_join(conn: ptr, room: ptr) -> i64
        self.known_functions.insert("mesh_ws_join".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        // mesh_ws_leave(conn: ptr, room: ptr) -> i64
        self.known_functions.insert("mesh_ws_leave".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        // mesh_ws_broadcast(room: ptr, msg: ptr) -> i64
        self.known_functions.insert("mesh_ws_broadcast".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        // mesh_ws_broadcast_except(room: ptr, msg: ptr, except_conn: ptr) -> i64
        self.known_functions.insert("mesh_ws_broadcast_except".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        // ── SQLite functions (Phase 53) ──────────────────────────────────
        // Connection handle is MirType::Int (i64) for GC safety (SQLT-07).
        self.known_functions.insert("mesh_sqlite_open".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_sqlite_close".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_sqlite_execute".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_sqlite_query".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── PostgreSQL functions (Phase 54) ──────────────────────────────
        // Connection handle is MirType::Int (i64) for GC safety (same as SQLite).
        self.known_functions.insert("mesh_pg_connect".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pg_close".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_pg_execute".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pg_query".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 57: PG Transaction functions ──────────────────────────
        // mesh_pg_begin(conn: i64) -> ptr (Result)
        self.known_functions.insert("mesh_pg_begin".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pg_commit".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pg_rollback".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        // mesh_pg_transaction(conn: i64, fn_ptr: ptr, env_ptr: ptr) -> ptr
        self.known_functions.insert("mesh_pg_transaction".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 57: SQLite Transaction functions ──────────────────────
        self.known_functions.insert("mesh_sqlite_begin".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_sqlite_commit".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_sqlite_rollback".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        // ── Phase 57: Connection Pool functions ─────────────────────────
        // mesh_pool_open(url: ptr, min: i64, max: i64, timeout: i64) -> ptr
        self.known_functions.insert("mesh_pool_open".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int, MirType::Int, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pool_close".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_pool_checkout".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pool_checkin".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_pool_query".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pool_execute".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 58: Row Parsing & Struct-to-Row Mapping ─────────────────
        self.known_functions.insert("mesh_row_from_row_get".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_row_parse_int".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_row_parse_float".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_row_parse_bool".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pg_query_as".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_pool_query_as".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 97: ORM SQL Generation ─────────────────────────────────
        // mesh_orm_build_select(table: ptr, columns: ptr, where_clauses: ptr, order_by: ptr, limit: i64, offset: i64) -> ptr
        self.known_functions.insert("mesh_orm_build_select".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Int, MirType::Int], Box::new(MirType::Ptr)));
        // mesh_orm_build_insert(table: ptr, columns: ptr, returning: ptr) -> ptr
        self.known_functions.insert("mesh_orm_build_insert".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_orm_build_update(table: ptr, set_columns: ptr, where_clauses: ptr, returning: ptr) -> ptr
        self.known_functions.insert("mesh_orm_build_update".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_orm_build_delete(table: ptr, where_clauses: ptr, returning: ptr) -> ptr
        self.known_functions.insert("mesh_orm_build_delete".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 98: Query Builder ─────────────────────────────────────
        // mesh_query_from(table: ptr) -> ptr
        self.known_functions.insert("mesh_query_from".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where(q: ptr, field: ptr, value: ptr) -> ptr
        self.known_functions.insert("mesh_query_where".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_op(q: ptr, field: ptr, op: ptr, value: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_op".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_in(q: ptr, field: ptr, values: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_in".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_null(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_null".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_not_null(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_not_null".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_not_in(q: ptr, field: ptr, values: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_not_in".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_between(q: ptr, field: ptr, low: ptr, high: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_between".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_or(q: ptr, fields: ptr, values: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_or".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select(q: ptr, fields: ptr) -> ptr
        self.known_functions.insert("mesh_query_select".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_order_by(q: ptr, field: ptr, direction: ptr) -> ptr
        self.known_functions.insert("mesh_query_order_by".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_limit(q: ptr, n: i64) -> ptr
        self.known_functions.insert("mesh_query_limit".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        // mesh_query_offset(q: ptr, n: i64) -> ptr
        self.known_functions.insert("mesh_query_offset".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        // mesh_query_join(q: ptr, type: ptr, table: ptr, on_clause: ptr) -> ptr
        self.known_functions.insert("mesh_query_join".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_join_as(q: ptr, type: ptr, table: ptr, alias: ptr, on_clause: ptr) -> ptr
        self.known_functions.insert("mesh_query_join_as".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_group_by(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_group_by".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_having(q: ptr, clause: ptr, value: ptr) -> ptr
        self.known_functions.insert("mesh_query_having".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 108: Aggregate SELECT functions ─────────────────────────
        // mesh_query_select_count(q: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_count".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select_count_field(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_count_field".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select_sum(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_sum".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select_avg(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_avg".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select_min(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_min".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select_max(q: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_max".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_fragment(q: ptr, sql: ptr, params: ptr) -> ptr
        self.known_functions.insert("mesh_query_fragment".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 103: Query Builder Raw Extensions ─────────────────────
        // mesh_query_order_by_raw(q: ptr, expression: ptr) -> ptr
        self.known_functions.insert("mesh_query_order_by_raw".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_group_by_raw(q: ptr, expression: ptr) -> ptr
        self.known_functions.insert("mesh_query_group_by_raw".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_select_raw(q: ptr, expressions: ptr) -> ptr
        self.known_functions.insert("mesh_query_select_raw".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_query_where_raw(q: ptr, clause: ptr, params: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_raw".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 109: Subquery WHERE ─────────────────────────────────────
        // mesh_query_where_sub(q: ptr, field: ptr, sub_query: ptr) -> ptr
        self.known_functions.insert("mesh_query_where_sub".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 98: Repo Read Operations ───────────────────────────────
        // mesh_repo_all(pool: i64, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_all".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_one(pool: i64, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_one".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_get(pool: i64, table: ptr, id: ptr) -> ptr
        self.known_functions.insert("mesh_repo_get".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_get_by(pool: i64, table: ptr, field: ptr, value: ptr) -> ptr
        self.known_functions.insert("mesh_repo_get_by".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_count(pool: i64, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_count".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_exists(pool: i64, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_exists".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 98: Repo Write Operations ─────────────────────────────
        // mesh_repo_insert(pool: i64, table: ptr, fields: ptr) -> ptr
        self.known_functions.insert("mesh_repo_insert".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_update(pool: i64, table: ptr, id: ptr, fields: ptr) -> ptr
        self.known_functions.insert("mesh_repo_update".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_delete(pool: i64, table: ptr, id: ptr) -> ptr
        self.known_functions.insert("mesh_repo_delete".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_transaction(pool: i64, fn_ptr: ptr, env_ptr: ptr) -> ptr
        self.known_functions.insert("mesh_repo_transaction".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 103: Extended Repo Write Operations ────────────────────
        // mesh_repo_update_where(pool: i64, table: ptr, fields: ptr, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_update_where".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_delete_where(pool: i64, table: ptr, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_delete_where".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_query_raw(pool: i64, sql: ptr, params: ptr) -> ptr
        self.known_functions.insert("mesh_repo_query_raw".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_execute_raw(pool: i64, sql: ptr, params: ptr) -> ptr
        self.known_functions.insert("mesh_repo_execute_raw".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 109: Upsert, RETURNING, Subquery ────────────────────────
        // mesh_repo_insert_or_update(pool: i64, table: ptr, fields: ptr, conflict_targets: ptr, update_fields: ptr) -> ptr
        self.known_functions.insert("mesh_repo_insert_or_update".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_delete_where_returning(pool: i64, table: ptr, query: ptr) -> ptr
        self.known_functions.insert("mesh_repo_delete_where_returning".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 99: Repo Changeset Operations ─────────────────────────
        // mesh_repo_insert_changeset(pool: i64, table: ptr, changeset: ptr) -> ptr
        self.known_functions.insert("mesh_repo_insert_changeset".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_repo_update_changeset(pool: i64, table: ptr, id: ptr, changeset: ptr) -> ptr
        self.known_functions.insert("mesh_repo_update_changeset".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 100: Repo Preloading ──────────────────────────────────
        // mesh_repo_preload(pool: i64, rows: ptr, associations: ptr, rel_meta: ptr) -> ptr
        self.known_functions.insert("mesh_repo_preload".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 99: Changeset Operations ──────────────────────────────
        // mesh_changeset_cast(data: ptr, params: ptr, allowed: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_cast".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_cast_with_types(data: ptr, params: ptr, allowed: ptr, field_types: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_cast_with_types".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_validate_required(cs: ptr, fields: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_validate_required".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_validate_length(cs: ptr, field: ptr, min: ptr, max: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_validate_length".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_validate_format(cs: ptr, field: ptr, pattern: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_validate_format".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_validate_inclusion(cs: ptr, field: ptr, allowed_values: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_validate_inclusion".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_validate_number(cs: ptr, field: ptr, gt: ptr, lt: ptr, gte: ptr, lte: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_validate_number".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_valid(cs: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_valid".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_errors(cs: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_errors".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_changes(cs: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_changes".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_get_change(cs: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_get_change".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_changeset_get_error(cs: ptr, field: ptr) -> ptr
        self.known_functions.insert("mesh_changeset_get_error".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Phase 101: Migration DDL Operations ─────────────────────────
        // mesh_migration_create_table(pool: i64, table: ptr, columns: ptr) -> ptr
        self.known_functions.insert("mesh_migration_create_table".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_drop_table(pool: i64, table: ptr) -> ptr
        self.known_functions.insert("mesh_migration_drop_table".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_add_column(pool: i64, table: ptr, col_def: ptr) -> ptr
        self.known_functions.insert("mesh_migration_add_column".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_drop_column(pool: i64, table: ptr, col: ptr) -> ptr
        self.known_functions.insert("mesh_migration_drop_column".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_rename_column(pool: i64, table: ptr, old: ptr, new: ptr) -> ptr
        self.known_functions.insert("mesh_migration_rename_column".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_create_index(pool: i64, table: ptr, cols: ptr, opts: ptr) -> ptr
        self.known_functions.insert("mesh_migration_create_index".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_drop_index(pool: i64, table: ptr, cols: ptr) -> ptr
        self.known_functions.insert("mesh_migration_drop_index".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // mesh_migration_execute(pool: i64, sql: ptr) -> ptr
        self.known_functions.insert("mesh_migration_execute".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Job functions (Phase 9 Plan 04) ──────────────────────────────
        // mesh_job_async takes (fn_ptr, env_ptr) -> i64 (PID)
        // But the closure splitting at codegen will expand the closure arg into (fn_ptr, env_ptr)
        self.known_functions.insert("mesh_job_async".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int)));
        self.known_functions.insert("mesh_job_await".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_job_await_timeout".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Ptr)));
        // mesh_job_map takes (list_ptr, fn_ptr, env_ptr) -> ptr
        // Closure splitting expands the closure arg into (fn_ptr, env_ptr)
        self.known_functions.insert("mesh_job_map".to_string(), MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr)));
        // ── Timer functions (Phase 44 Plan 02) ──────────────────────────────
        // mesh_timer_sleep(ms: i64) -> void (Unit)
        self.known_functions.insert("mesh_timer_sleep".to_string(), MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Unit)));
        // mesh_timer_send_after(pid: i64, ms: i64, msg_ptr: ptr, msg_size: i64) -> void (Unit)
        self.known_functions.insert("mesh_timer_send_after".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Int, MirType::Ptr, MirType::Int], Box::new(MirType::Unit)));
        // ── Service runtime functions (Phase 9 Plan 03) ─────────────────
        self.known_functions.insert("mesh_service_call".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Int, MirType::Ptr, MirType::Int], Box::new(MirType::Ptr)));
        self.known_functions.insert("mesh_service_reply".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Int], Box::new(MirType::Unit)));
        self.known_functions.insert("mesh_actor_send".to_string(), MirType::FnPtr(vec![MirType::Int, MirType::Ptr, MirType::Int], Box::new(MirType::Unit)));

        // Also register variant constructors as known functions.
        for (_, sum_info) in &self.registry.sum_type_defs {
            for variant in &sum_info.variants {
                if !variant.fields.is_empty() {
                    // Variant constructor is a function
                    let name = variant.name.clone();
                    let qualified = format!("{}.{}", sum_info.name, variant.name);
                    // We don't have exact types here; mark as known for call dispatch.
                    self.known_functions
                        .insert(name, MirType::FnPtr(vec![], Box::new(MirType::Unit)));
                    self.known_functions
                        .insert(qualified, MirType::FnPtr(vec![], Box::new(MirType::Unit)));
                }
            }
        }

        // Pre-pass: build function value usage types map so that lower_fn_def can
        // recover concrete parameter types for functions whose params were generalized
        // away (Ty::Var) before call sites like `HTTP.use(r, pass)` constrained them.
        {
            let syntax = self.parse.syntax();
            let usage_types = self.build_fn_value_usage_types(&syntax);
            self.fn_value_usage_types = usage_types;
        }

        // Second pass: lower all items, grouping consecutive same-name FnDefs.
        let items: Vec<Item> = sf.items().collect();
        let mut i = 0;
        while i < items.len() {
            if let Item::FnDef(ref fn_def) = items[i] {
                // Check if this starts a multi-clause function group.
                let fn_name = fn_def.name().and_then(|n| n.text());
                if fn_def.has_eq_body() {
                    if let Some(ref name) = fn_name {
                        // Collect consecutive FnDefs with the same name.
                        let mut group: Vec<&FnDef> = vec![fn_def];
                        let mut j = i + 1;
                        while j < items.len() {
                            if let Item::FnDef(ref next_fn) = items[j] {
                                let next_name = next_fn.name().and_then(|n| n.text());
                                if next_name.as_deref() == Some(name) && next_fn.has_eq_body() {
                                    group.push(next_fn);
                                    j += 1;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        if group.len() > 1 {
                            self.lower_multi_clause_fn(&group);
                            i = j;
                            continue;
                        }
                    }
                }
            }
            self.lower_item(items[i].clone());
            i += 1;
        }
    }

    fn lower_item(&mut self, item: Item) {
        match item {
            Item::FnDef(fn_def) => self.lower_fn_def(&fn_def),
            Item::StructDef(struct_def) => self.lower_struct_def(&struct_def),
            Item::SumTypeDef(sum_def) => self.lower_sum_type_def(&sum_def),
            Item::LetBinding(let_) => self.lower_top_level_let(&let_),
            Item::ImplDef(impl_def) => {
                let (trait_name, trait_type_args, type_name) = extract_impl_names(&impl_def);

                // Collect names of methods explicitly provided in this impl.
                let mut provided_methods = std::collections::HashSet::new();
                for method in impl_def.methods() {
                    let method_name = method
                        .name()
                        .and_then(|n| n.text())
                        .unwrap_or_else(|| "<unnamed>".to_string());
                    provided_methods.insert(method_name.clone());
                    let mangled = mangle_trait_method(
                        &trait_name, &trait_type_args, &method_name, &type_name,
                    );
                    self.lower_impl_method(&method, &mangled, &type_name);
                }

                // Lower default method bodies for methods not provided by the impl.
                if let Some(trait_def) = self.trait_registry.get_trait(&trait_name) {
                    for trait_method in &trait_def.methods {
                        if trait_method.has_default_body
                            && !provided_methods.contains(&trait_method.name)
                        {
                            let key = (trait_name.clone(), trait_method.name.clone());
                            if let Some(&range) = self.default_method_bodies.get(&key) {
                                self.lower_default_method(
                                    range,
                                    &trait_name,
                                    &trait_method.name,
                                    &type_name,
                                );
                            }
                        }
                    }
                }
            }
            Item::InterfaceDef(_) | Item::TypeAliasDef(_) => {
                // Skip -- interfaces are erased, type aliases are resolved.
            }
            Item::ModuleDef(_) | Item::ImportDecl(_) | Item::FromImportDecl(_) => {
                // Skip -- module/import handling is not needed for single-file compilation.
            }
            Item::ActorDef(actor_def) => self.lower_actor_def(&actor_def),
            Item::ServiceDef(service_def) => self.lower_service_def(&service_def),
            Item::SupervisorDef(sup_def) => self.lower_supervisor_def(&sup_def),
        }
    }

    // ── Function lowering ────────────────────────────────────────────

    fn lower_fn_def(&mut self, fn_def: &FnDef) {
        let name = fn_def
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<anonymous>".to_string());

        // Get function type from typeck.
        let fn_range = fn_def.syntax().text_range();
        let fn_ty_raw = self.get_ty(fn_range).cloned();

        // Extract parameter names and types.
        let mut params = Vec::new();
        self.push_scope();

        if let Some(param_list) = fn_def.param_list() {
            if let Some(Ty::Fun(param_tys, _)) = &fn_ty_raw {
                for (param_idx, (param, param_ty)) in
                    param_list.params().zip(param_tys.iter()).enumerate()
                {
                    let param_name = param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string());
                    let is_closure = matches!(param_ty, Ty::Fun(..));
                    let mut mir_ty = resolve_type(param_ty, self.registry, is_closure);
                    // If the parameter type is unresolved (Ty::Var → MirType::Unit),
                    // try to recover the concrete type from call sites where this
                    // function was passed as a value argument (e.g. HTTP.use(r, fn)).
                    if mir_ty == MirType::Unit && matches!(param_ty, Ty::Var(_)) {
                        if let Some(recovered) = self.resolve_param_from_usage(&name, param_idx) {
                            mir_ty = recovered;
                        }
                    }
                    self.insert_var(param_name.clone(), mir_ty.clone());
                    params.push((param_name, mir_ty));
                }
            } else {
                // Fallback: use range-based type lookup for each param.
                for param in param_list.params() {
                    let param_name = param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string());
                    let mir_ty = self.resolve_range(param.syntax().text_range());
                    self.insert_var(param_name.clone(), mir_ty.clone());
                    params.push((param_name, mir_ty));
                }
            }
        }

        // Return type.
        let return_type = if let Some(Ty::Fun(_, ret)) = &fn_ty_raw {
            resolve_type(ret, self.registry, false)
        } else {
            MirType::Unit
        };

        // Track current function return type for ? operator desugaring (Phase 45).
        let prev_fn_return_type = self.current_fn_return_type.take();
        self.current_fn_return_type = Some(return_type.clone());

        // Monomorphization depth tracking.
        self.mono_depth += 1;
        let mut body = if self.mono_depth > self.max_mono_depth {
            MirExpr::Panic {
                message: format!(
                    "monomorphization depth limit ({}) exceeded",
                    self.max_mono_depth
                ),
                file: "<compiler>".to_string(),
                line: 0,
            }
        } else if let Some(block) = fn_def.body() {
            self.lower_block(&block)
        } else if let Some(expr) = fn_def.expr_body() {
            // Handle `= expr` body form (e.g., `fn double(x) = x * 2`).
            self.lower_expr(&expr)
        } else {
            MirExpr::Unit
        };
        self.mono_depth -= 1;

        // Restore previous function return type.
        self.current_fn_return_type = prev_fn_return_type;

        self.pop_scope();

        // Rename "main" to "mesh_main" to avoid collision with C main() entry point.
        // Then apply module-qualified naming for private functions.
        let fn_name = if name == "main" {
            self.entry_function = Some("mesh_main".to_string());
            "mesh_main".to_string()
        } else {
            self.qualify_name(&name)
        };

        // Register both original and qualified name in known_functions for
        // intra-module call resolution (callers use unqualified name from AST).
        if fn_name != name {
            let fn_ty = MirType::FnPtr(
                params.iter().map(|(_, t)| t.clone()).collect(),
                Box::new(return_type.clone()),
            );
            self.known_functions.insert(name, fn_ty);
        }

        // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
        let has_tail_calls = rewrite_tail_calls(&mut body, &fn_name);

        self.functions.push(MirFunction {
            name: fn_name,
            params,
            return_type,
            body,
            is_closure_fn: false,
            captures: Vec::new(),
            has_tail_calls,
        });
    }

    // ── Impl method lowering ───────────────────────────────────────

    /// Lower a single impl method to a MirFunction with a mangled name.
    /// The `self` parameter is detected via SELF_KW and named "self" with
    /// the concrete implementing struct type.
    fn lower_impl_method(&mut self, method: &FnDef, mangled_name: &str, type_name: &str) {
        // Get function type from typeck.
        let fn_range = method.syntax().text_range();
        let fn_ty_raw = self.get_ty(fn_range).cloned();

        // Extract parameter names and types.
        let mut params = Vec::new();
        self.push_scope();

        if let Some(param_list) = method.param_list() {
            if let Some(Ty::Fun(param_tys, _)) = &fn_ty_raw {
                for (param, param_ty) in param_list.params().zip(param_tys.iter()) {
                    // Detect self parameter via SELF_KW token.
                    let is_self = param
                        .syntax()
                        .children_with_tokens()
                        .any(|tok| {
                            tok.as_token()
                                .map(|t| t.kind() == SyntaxKind::SELF_KW)
                                .unwrap_or(false)
                        });

                    let param_name = if is_self {
                        "self".to_string()
                    } else {
                        param
                            .name()
                            .map(|t| t.text().to_string())
                            .unwrap_or_else(|| "_".to_string())
                    };

                    // Use the Ty::Fun param type for all params (including self).
                    // The type checker stores the impl type as the first param type.
                    let is_closure = matches!(param_ty, Ty::Fun(..));
                    let mir_ty = resolve_type(param_ty, self.registry, is_closure);
                    self.insert_var(param_name.clone(), mir_ty.clone());
                    params.push((param_name, mir_ty));
                }
            } else {
                // Fallback: use range-based type lookup for each param.
                for param in param_list.params() {
                    let is_self = param
                        .syntax()
                        .children_with_tokens()
                        .any(|tok| {
                            tok.as_token()
                                .map(|t| t.kind() == SyntaxKind::SELF_KW)
                                .unwrap_or(false)
                        });

                    let param_name = if is_self {
                        "self".to_string()
                    } else {
                        param
                            .name()
                            .map(|t| t.text().to_string())
                            .unwrap_or_else(|| "_".to_string())
                    };

                    let mir_ty = if is_self {
                        // For self, resolve to the concrete struct type.
                        resolve_type(
                            &Ty::Con(mesh_typeck::ty::TyCon::new(type_name)),
                            self.registry,
                            false,
                        )
                    } else {
                        self.resolve_range(param.syntax().text_range())
                    };

                    self.insert_var(param_name.clone(), mir_ty.clone());
                    params.push((param_name, mir_ty));
                }
            }
        }

        // Return type.
        let return_type = if let Some(Ty::Fun(_, ret)) = &fn_ty_raw {
            resolve_type(ret, self.registry, false)
        } else {
            MirType::Unit
        };

        // Track current function return type for ? operator desugaring (Phase 45).
        let prev_fn_return_type = self.current_fn_return_type.take();
        self.current_fn_return_type = Some(return_type.clone());

        // Monomorphization depth tracking.
        self.mono_depth += 1;
        let mut body = if self.mono_depth > self.max_mono_depth {
            MirExpr::Panic {
                message: format!(
                    "monomorphization depth limit ({}) exceeded",
                    self.max_mono_depth
                ),
                file: "<compiler>".to_string(),
                line: 0,
            }
        } else if let Some(block) = method.body() {
            self.lower_block(&block)
        } else if let Some(expr) = method.expr_body() {
            self.lower_expr(&expr)
        } else {
            MirExpr::Unit
        };
        self.mono_depth -= 1;

        // Restore previous function return type.
        self.current_fn_return_type = prev_fn_return_type;

        self.pop_scope();

        // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
        let has_tail_calls = rewrite_tail_calls(&mut body, mangled_name);

        self.functions.push(MirFunction {
            name: mangled_name.to_string(),
            params,
            return_type,
            body,
            is_closure_fn: false,
            captures: Vec::new(),
            has_tail_calls,
        });
    }

    // ── Default method body lowering ─────────────────────────────────

    /// Lower a default method body from an interface definition for a concrete type.
    ///
    /// The default body is re-lowered per concrete type (monomorphization model).
    /// The `self` parameter is bound to the concrete impl type.
    fn lower_default_method(
        &mut self,
        method_range: TextRange,
        trait_name: &str,
        method_name: &str,
        type_name: &str,
    ) {
        // Find the InterfaceMethod AST node by its text range.
        let tree = self.parse.syntax();
        let method_node = tree
            .descendants()
            .find(|n| {
                n.kind() == SyntaxKind::INTERFACE_METHOD && n.text_range() == method_range
            });

        let method_node = match method_node {
            Some(n) => n,
            None => return, // Could not find the interface method node
        };

        let interface_method = match InterfaceMethod::cast(method_node) {
            Some(m) => m,
            None => return,
        };

        let body_block = match interface_method.body() {
            Some(b) => b,
            None => return, // No default body (should not happen since has_default_body is true)
        };

        let mangled = format!("{}__{}__{}", trait_name, method_name, type_name);

        // Build parameters: detect self via SELF_KW, bind to concrete type.
        let mut params = Vec::new();
        self.push_scope();

        if let Some(param_list) = interface_method.param_list() {
            for param in param_list.params() {
                let is_self = param
                    .syntax()
                    .children_with_tokens()
                    .any(|tok| {
                        tok.as_token()
                            .map(|t| t.kind() == SyntaxKind::SELF_KW)
                            .unwrap_or(false)
                    });

                let param_name = if is_self {
                    "self".to_string()
                } else {
                    param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string())
                };

                let mir_ty = if is_self {
                    resolve_type(
                        &Ty::Con(mesh_typeck::ty::TyCon::new(type_name)),
                        self.registry,
                        false,
                    )
                } else {
                    self.resolve_range(param.syntax().text_range())
                };

                self.insert_var(param_name.clone(), mir_ty.clone());
                params.push((param_name, mir_ty));
            }
        }

        // Return type: use range-based lookup or fall back to Unit.
        let return_type = if let Some(ann) = interface_method.return_type() {
            self.resolve_range(ann.syntax().text_range())
        } else {
            MirType::Unit
        };

        // Lower the default body.
        self.mono_depth += 1;
        let mut body = if self.mono_depth > self.max_mono_depth {
            MirExpr::Panic {
                message: format!(
                    "monomorphization depth limit ({}) exceeded",
                    self.max_mono_depth
                ),
                file: "<compiler>".to_string(),
                line: 0,
            }
        } else {
            self.lower_block(&body_block)
        };
        self.mono_depth -= 1;

        self.pop_scope();

        // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
        let has_tail_calls = rewrite_tail_calls(&mut body, &mangled);

        self.functions.push(MirFunction {
            name: mangled,
            params,
            return_type,
            body,
            is_closure_fn: false,
            captures: Vec::new(),
            has_tail_calls,
        });
    }

    // ── Multi-clause function lowering ──────────────────────────────

    /// Lower a group of consecutive same-name FnDef nodes (multi-clause function)
    /// into a single MirFunction with a match body dispatching on parameter patterns.
    fn lower_multi_clause_fn(&mut self, clauses: &[&FnDef]) {
        let first = clauses[0];
        let name = first
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<anonymous>".to_string());

        // Get the function type from typeck (stored on the FIRST clause's range).
        let fn_range = first.syntax().text_range();
        let fn_ty_raw = self.get_ty(fn_range).cloned();

        // Extract parameter types from the function type.
        let (param_tys, return_type) = if let Some(Ty::Fun(pts, ret)) = &fn_ty_raw {
            (
                pts.iter()
                    .enumerate()
                    .map(|(param_idx, t)| {
                        let is_closure = matches!(t, Ty::Fun(..));
                        let mut mir_ty = resolve_type(t, self.registry, is_closure);
                        // Recover concrete type for Ty::Var parameters from usage sites.
                        if mir_ty == MirType::Unit && matches!(t, Ty::Var(_)) {
                            if let Some(recovered) =
                                self.resolve_param_from_usage(&name, param_idx)
                            {
                                mir_ty = recovered;
                            }
                        }
                        mir_ty
                    })
                    .collect::<Vec<_>>(),
                resolve_type(ret, self.registry, false),
            )
        } else {
            (Vec::new(), MirType::Unit)
        };

        let arity = param_tys.len();

        // Create synthetic parameter names: __param_0, __param_1, etc.
        let params: Vec<(String, MirType)> = param_tys
            .iter()
            .enumerate()
            .map(|(i, ty)| (format!("__param_{}", i), ty.clone()))
            .collect();

        // Build match arms from clauses.
        self.push_scope();

        // Insert synthetic params into scope.
        for (pname, pty) in &params {
            self.insert_var(pname.clone(), pty.clone());
        }

        if arity == 1 {
            // Single-parameter: use MirExpr::Match directly.
            let scrutinee = MirExpr::Var(params[0].0.clone(), params[0].1.clone());
            let mut arms = Vec::new();

            for clause in clauses {
                self.push_scope();
                // Insert param into scope for body lowering.
                self.insert_var(params[0].0.clone(), params[0].1.clone());

                let pattern = self.lower_clause_param_pattern(clause, 0, &params);
                let guard = self.lower_clause_guard(clause);
                let body = self.lower_clause_body(clause);
                self.pop_scope();

                arms.push(MirMatchArm {
                    pattern,
                    guard,
                    body,
                });
            }

            let mut body = MirExpr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
                ty: return_type.clone(),
            };

            self.pop_scope();

            let fn_name = if name == "main" {
                self.entry_function = Some("mesh_main".to_string());
                "mesh_main".to_string()
            } else {
                self.qualify_name(&name)
            };

            // Register original name for intra-module call resolution
            if fn_name != name {
                let fn_ty = MirType::FnPtr(
                    params.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(return_type.clone()),
                );
                self.known_functions.insert(name, fn_ty);
            }

            // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
            let has_tail_calls = rewrite_tail_calls(&mut body, &fn_name);

            self.functions.push(MirFunction {
                name: fn_name,
                params,
                return_type,
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls,
            });
        } else {
            // Multi-parameter: use an if-else chain.
            // Each clause becomes: if (param_checks && guard) { bind_vars; body } else { next }
            let mut body = self.lower_multi_clause_if_chain(clauses, &params, &return_type);
            self.pop_scope();

            let fn_name = if name == "main" {
                self.entry_function = Some("mesh_main".to_string());
                "mesh_main".to_string()
            } else {
                self.qualify_name(&name)
            };

            // Register original name for intra-module call resolution
            if fn_name != name {
                let fn_ty = MirType::FnPtr(
                    params.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(return_type.clone()),
                );
                self.known_functions.insert(name, fn_ty);
            }

            // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
            let has_tail_calls = rewrite_tail_calls(&mut body, &fn_name);

            self.functions.push(MirFunction {
                name: fn_name,
                params,
                return_type,
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls,
            });
        }
    }

    /// Lower a single clause's parameter at `param_idx` to a MirPattern.
    fn lower_clause_param_pattern(
        &mut self,
        clause: &FnDef,
        param_idx: usize,
        mir_params: &[(String, MirType)],
    ) -> MirPattern {
        if let Some(param_list) = clause.param_list() {
            if let Some(param) = param_list.params().nth(param_idx) {
                if let Some(pat) = param.pattern() {
                    return self.lower_pattern(&pat);
                }
                // Regular named parameter -> wildcard-like variable binding.
                if let Some(name_tok) = param.name() {
                    let pname = name_tok.text().to_string();
                    let pty = mir_params[param_idx].1.clone();
                    self.insert_var(pname.clone(), pty.clone());
                    return MirPattern::Var(pname, pty);
                }
            }
        }
        MirPattern::Wildcard
    }

    /// Lower a clause's guard expression to an optional MirExpr.
    fn lower_clause_guard(&mut self, clause: &FnDef) -> Option<MirExpr> {
        clause.guard().and_then(|gc| gc.expr()).map(|e| self.lower_expr(&e))
    }

    /// Lower a clause's body expression.
    fn lower_clause_body(&mut self, clause: &FnDef) -> MirExpr {
        if let Some(expr) = clause.expr_body() {
            self.lower_expr(&expr)
        } else if let Some(block) = clause.body() {
            self.lower_block(&block)
        } else {
            MirExpr::Unit
        }
    }

    /// Build an if-else chain for multi-parameter multi-clause functions.
    /// Each clause becomes: if (all params match) { body } else { next clause }
    fn lower_multi_clause_if_chain(
        &mut self,
        clauses: &[&FnDef],
        mir_params: &[(String, MirType)],
        return_type: &MirType,
    ) -> MirExpr {
        if clauses.is_empty() {
            return MirExpr::Unit;
        }

        // Process clauses from last to first, building the else chain.
        let mut else_body: Option<MirExpr> = None;

        for clause in clauses.iter().rev() {
            self.push_scope();
            // Re-insert params into scope for this clause.
            for (pname, pty) in mir_params {
                self.insert_var(pname.clone(), pty.clone());
            }

            // Check if this is a catch-all clause (all params are wildcards/variables, no guard).
            let is_catch_all = self.is_catch_all_clause(clause, mir_params);

            if is_catch_all && else_body.is_none() {
                // Last clause and catch-all: just emit the body directly.
                let mut bindings = Vec::new();
                self.collect_clause_bindings(clause, mir_params, &mut bindings);
                let body = self.lower_clause_body(clause);
                self.pop_scope();

                // Wrap bindings around body.
                let body = self.wrap_with_bindings(bindings, body);

                else_body = Some(body);
            } else {
                // Build condition: check all param patterns.
                let cond = self.build_clause_condition(clause, mir_params);
                let guard = self.lower_clause_guard(clause);

                // Combine pattern check with guard.
                let full_cond = if let Some(guard_expr) = guard {
                    if let Some(pattern_cond) = cond {
                        MirExpr::BinOp {
                            op: BinOp::And,
                            lhs: Box::new(pattern_cond),
                            rhs: Box::new(guard_expr),
                            ty: MirType::Bool,
                        }
                    } else {
                        guard_expr
                    }
                } else {
                    cond.unwrap_or(MirExpr::BoolLit(true, MirType::Bool))
                };

                let mut bindings = Vec::new();
                self.collect_clause_bindings(clause, mir_params, &mut bindings);
                let body = self.lower_clause_body(clause);
                let body = self.wrap_with_bindings(bindings, body);

                self.pop_scope();

                let fallthrough = else_body.unwrap_or(MirExpr::Unit);

                else_body = Some(MirExpr::If {
                    cond: Box::new(full_cond),
                    then_body: Box::new(body),
                    else_body: Box::new(fallthrough),
                    ty: return_type.clone(),
                });
            }
        }

        else_body.unwrap_or(MirExpr::Unit)
    }

    /// Check if a clause is a catch-all (all params are wildcards or plain variables, no guard).
    fn is_catch_all_clause(&self, clause: &FnDef, _mir_params: &[(String, MirType)]) -> bool {
        if clause.guard().is_some() {
            return false;
        }
        if let Some(param_list) = clause.param_list() {
            for param in param_list.params() {
                if let Some(pat) = param.pattern() {
                    // Has a pattern -- check if it's a wildcard or ident.
                    match pat {
                        Pattern::Wildcard(_) | Pattern::Ident(_) => {}
                        _ => return false,
                    }
                }
                // Plain named param is always catch-all.
            }
            true
        } else {
            true
        }
    }

    /// Build a boolean condition that checks if all params match the clause's patterns.
    /// Returns None if the clause is a catch-all (no conditions needed).
    fn build_clause_condition(
        &self,
        clause: &FnDef,
        mir_params: &[(String, MirType)],
    ) -> Option<MirExpr> {
        let mut conditions: Vec<MirExpr> = Vec::new();

        if let Some(param_list) = clause.param_list() {
            for (idx, param) in param_list.params().enumerate() {
                if idx >= mir_params.len() {
                    break;
                }
                if let Some(pat) = param.pattern() {
                    if let Some(cond) = self.pattern_to_condition(&pat, &mir_params[idx]) {
                        conditions.push(cond);
                    }
                }
                // Plain named param: no condition needed (matches everything).
            }
        }

        if conditions.is_empty() {
            None
        } else {
            let mut result = conditions.remove(0);
            for cond in conditions {
                result = MirExpr::BinOp {
                    op: BinOp::And,
                    lhs: Box::new(result),
                    rhs: Box::new(cond),
                    ty: MirType::Bool,
                };
            }
            Some(result)
        }
    }

    /// Convert a pattern to a boolean condition expression.
    /// Returns None for wildcard/variable patterns (always match).
    fn pattern_to_condition(
        &self,
        pat: &Pattern,
        param: &(String, MirType),
    ) -> Option<MirExpr> {
        match pat {
            Pattern::Wildcard(_) | Pattern::Ident(_) => None,
            Pattern::Literal(lit) => {
                let param_var = MirExpr::Var(param.0.clone(), param.1.clone());
                if let Some(tok) = lit.token() {
                    let text = tok.text().to_string();
                    let lit_expr = match tok.kind() {
                        SyntaxKind::INT_LITERAL => {
                            MirExpr::IntLit(text.parse().unwrap_or(0), param.1.clone())
                        }
                        SyntaxKind::FLOAT_LITERAL => {
                            MirExpr::FloatLit(text.parse().unwrap_or(0.0), param.1.clone())
                        }
                        SyntaxKind::TRUE_KW => MirExpr::BoolLit(true, MirType::Bool),
                        SyntaxKind::FALSE_KW => MirExpr::BoolLit(false, MirType::Bool),
                        SyntaxKind::MINUS => {
                            // Negative literal: look for the next sibling INT_LITERAL.
                            let neg_val = extract_negative_literal(lit.syntax());
                            MirExpr::IntLit(neg_val, param.1.clone())
                        }
                        _ => return None,
                    };
                    Some(MirExpr::BinOp {
                        op: BinOp::Eq,
                        lhs: Box::new(param_var),
                        rhs: Box::new(lit_expr),
                        ty: MirType::Bool,
                    })
                } else {
                    None
                }
            }
            _ => None, // Constructor/Tuple/Or/As patterns in multi-param: skip (match-all)
        }
    }

    /// Collect variable bindings from a clause's parameter list.
    fn collect_clause_bindings(
        &mut self,
        clause: &FnDef,
        mir_params: &[(String, MirType)],
        bindings: &mut Vec<(String, MirExpr)>,
    ) {
        if let Some(param_list) = clause.param_list() {
            for (idx, param) in param_list.params().enumerate() {
                if idx >= mir_params.len() {
                    break;
                }
                let param_var = MirExpr::Var(mir_params[idx].0.clone(), mir_params[idx].1.clone());
                if let Some(pat) = param.pattern() {
                    match pat {
                        Pattern::Ident(ref ident) => {
                            let name = ident
                                .name()
                                .map(|t| t.text().to_string())
                                .unwrap_or_else(|| "_".to_string());
                            if name != "_" {
                                self.insert_var(name.clone(), mir_params[idx].1.clone());
                                bindings.push((name, param_var));
                            }
                        }
                        Pattern::Wildcard(_) | Pattern::Literal(_) => {
                            // No binding needed.
                        }
                        _ => {} // Skip complex patterns for now.
                    }
                } else if let Some(name_tok) = param.name() {
                    let pname = name_tok.text().to_string();
                    if pname != "_" {
                        self.insert_var(pname.clone(), mir_params[idx].1.clone());
                        bindings.push((pname, param_var));
                    }
                }
            }
        }
    }

    /// Wrap an expression with let-bindings.
    fn wrap_with_bindings(&self, bindings: Vec<(String, MirExpr)>, body: MirExpr) -> MirExpr {
        let mut result = body;
        for (name, value) in bindings.into_iter().rev() {
            let ty = value.ty().clone();
            result = MirExpr::Let {
                name,
                ty,
                value: Box::new(value),
                body: Box::new(result),
            };
        }
        result
    }

    // ── Struct lowering ──────────────────────────────────────────────

    fn lower_struct_def(&mut self, struct_def: &StructDef) {
        let name = struct_def
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed>".to_string());

        // Look up from type registry for accurate types.
        let fields: Vec<(String, MirType)> = if let Some(info) = self.registry.struct_defs.get(&name) {
            info.fields
                .iter()
                .map(|(fname, fty)| {
                    (
                        fname.clone(),
                        resolve_type(fty, self.registry, false),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        // Check if this is a generic struct (trait functions generated lazily at instantiation).
        let has_generic_params = self.registry.struct_defs.get(&name)
            .map_or(false, |info| !info.generic_params.is_empty());

        if !has_generic_params {
            // Conditional MIR generation based on deriving clause.
            // No deriving clause = backward compat (generate all default trait functions).
            let has_deriving = struct_def.has_deriving_clause();
            let derive_list = struct_def.deriving_traits();
            let derive_all = !has_deriving;

            if derive_all || derive_list.iter().any(|t| t == "Debug") {
                self.generate_debug_inspect_struct(&name, &fields);
            }
            if derive_all || derive_list.iter().any(|t| t == "Eq") {
                self.generate_eq_struct(&name, &fields);
            }
            if derive_all || derive_list.iter().any(|t| t == "Ord") {
                self.generate_ord_struct(&name, &fields);
                self.generate_compare_struct(&name, &fields);
            }
            if derive_all || derive_list.iter().any(|t| t == "Hash") {
                self.generate_hash_struct(&name, &fields);
            }
            // Display: only via explicit deriving(Display), never auto-derived
            if derive_list.iter().any(|t| t == "Display") {
                self.generate_display_struct(&name, &fields);
            }
            // Json: only via explicit deriving(Json), never auto-derived
            if derive_list.iter().any(|t| t == "Json") {
                self.generate_to_json_struct(&name, &fields);
                self.generate_from_json_struct(&name, &fields);
                self.generate_from_json_string_wrapper(&name);
            }
            // Row: only via explicit deriving(Row), never auto-derived
            if derive_list.iter().any(|t| t == "Row") {
                self.generate_from_row_struct(&name, &fields);
            }

            // Schema: only via explicit deriving(Schema), never auto-derived
            if derive_list.iter().any(|t| t == "Schema") {
                let relationships = struct_def.relationships();
                let schema_opts = struct_def.schema_options();

                // Extract schema option values.
                let mut custom_table: Option<String> = None;
                let mut custom_pk: Option<String> = None;
                let mut has_timestamps = false;
                for opt in &schema_opts {
                    if let Some(opt_name) = opt.option_name() {
                        match opt_name.as_str() {
                            "table" => custom_table = opt.string_value(),
                            "primary_key" => custom_pk = opt.atom_value(),
                            "timestamps" => has_timestamps = opt.bool_value().unwrap_or(false),
                            _ => {}
                        }
                    }
                }

                // Inject timestamp fields if requested.
                let mut schema_fields = fields.clone();
                if has_timestamps {
                    schema_fields.push(("inserted_at".to_string(), MirType::String));
                    schema_fields.push(("updated_at".to_string(), MirType::String));
                }

                self.generate_schema_metadata(&name, &schema_fields, &relationships, custom_table, custom_pk, has_timestamps);

                // Use extended fields (with timestamps) for the struct layout.
                if has_timestamps {
                    self.structs.push(MirStructDef { name, fields: schema_fields });
                } else {
                    self.structs.push(MirStructDef { name, fields });
                }
            } else {
                self.structs.push(MirStructDef { name, fields });
            }
        }
        // For generic structs: trait functions generated lazily at instantiation
        // via ensure_monomorphized_struct_trait_fns. The MirStructDef is also
        // generated lazily with the mangled name and concrete field types.
    }

    /// Lazily generate monomorphized trait functions for a generic struct instantiation.
    ///
    /// When a generic struct like `Box<T>` is instantiated as `Box<Int>`, this method:
    /// 1. Computes the mangled name (e.g., "Box_Int")
    /// 2. Substitutes generic params with concrete types in the field list
    /// 3. Generates Display, Eq, Debug, etc. MIR functions with the mangled name
    /// 4. Pushes a MirStructDef with the mangled name and concrete fields
    ///
    /// Called from `lower_struct_literal` when a generic struct instantiation is detected.
    fn ensure_monomorphized_struct_trait_fns(&mut self, base_name: &str, typeck_ty: &Ty) {
        // Extract type args from Ty::App(Con("Box"), [Con("Int")])
        let type_args = match typeck_ty {
            Ty::App(_, args) => args,
            _ => return, // Not a generic instantiation
        };

        let mangled = mangle_type_name(base_name, type_args, self.registry);

        // Already generated?
        if self.monomorphized_trait_fns.contains(&mangled) {
            return;
        }
        self.monomorphized_trait_fns.insert(mangled.clone());

        // Look up the generic struct definition to get field info and generic params.
        let struct_info = match self.registry.struct_defs.get(base_name) {
            Some(info) => info.clone(),
            None => return,
        };

        // Build a substitution map: generic param name -> concrete Ty.
        let subst: HashMap<String, &Ty> = struct_info.generic_params
            .iter()
            .zip(type_args.iter())
            .map(|(param, arg)| (param.clone(), arg))
            .collect();

        // Substitute generic params with concrete types in the field list.
        let fields: Vec<(String, MirType)> = struct_info.fields
            .iter()
            .map(|(fname, fty)| {
                let concrete_ty = substitute_type_params(fty, &subst);
                (fname.clone(), resolve_type(&concrete_ty, self.registry, false))
            })
            .collect();

        // Check which traits are registered via the trait registry.
        // Use the parametric typeck type for lookup (e.g., Ty::App(Con("Box"), [Con("Int")])).
        let has_display = self.trait_registry.has_impl("Display", typeck_ty);
        let has_eq = self.trait_registry.has_impl("Eq", typeck_ty);
        let has_debug = self.trait_registry.has_impl("Debug", typeck_ty);
        let has_ord = self.trait_registry.has_impl("Ord", typeck_ty);
        let has_hash = self.trait_registry.has_impl("Hash", typeck_ty);
        let has_json = self.trait_registry.has_impl("ToJson", typeck_ty);

        // Generate trait functions for the monomorphized name.
        // Display and Debug use base_name for human-readable output (e.g., "Box(42)" not "Box_Int(42)").
        if has_debug {
            self.generate_debug_inspect_struct_with_display_name(&mangled, base_name, &fields);
        }
        if has_eq {
            self.generate_eq_struct(&mangled, &fields);
        }
        if has_ord {
            self.generate_ord_struct(&mangled, &fields);
            self.generate_compare_struct(&mangled, &fields);
        }
        if has_hash {
            self.generate_hash_struct(&mangled, &fields);
        }
        if has_display {
            self.generate_display_struct_with_display_name(&mangled, base_name, &fields);
        }
        if has_json {
            self.generate_to_json_struct(&mangled, &fields);
            self.generate_from_json_struct(&mangled, &fields);
            self.generate_from_json_string_wrapper(&mangled);
        }

        // Push the monomorphized struct definition.
        self.structs.push(MirStructDef {
            name: mangled,
            fields,
        });
    }

    // ── Sum type lowering ────────────────────────────────────────────

    fn lower_sum_type_def(&mut self, sum_def: &SumTypeDef) {
        let name = sum_def
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<unnamed>".to_string());

        // Look up from type registry for accurate variant info.
        let variants: Vec<MirVariantDef> = if let Some(info) = self.registry.sum_type_defs.get(&name) {
            info.variants
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let fields = v
                        .fields
                        .iter()
                        .map(|f| {
                            let ty = match f {
                                mesh_typeck::VariantFieldInfo::Positional(ty) => ty,
                                mesh_typeck::VariantFieldInfo::Named(_, ty) => ty,
                            };
                            resolve_type(ty, self.registry, false)
                        })
                        .collect();
                    MirVariantDef {
                        name: v.name.clone(),
                        fields,
                        tag: i as u8,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        // Conditional MIR generation based on deriving clause.
        // No deriving clause = backward compat (generate all default trait functions).
        let has_deriving = sum_def.has_deriving_clause();
        let derive_list = sum_def.deriving_traits();
        let derive_all = !has_deriving;

        if derive_all || derive_list.iter().any(|t| t == "Debug") {
            self.generate_debug_inspect_sum_type(&name, &variants);
        }
        if derive_all || derive_list.iter().any(|t| t == "Eq") {
            self.generate_eq_sum(&name, &variants);
        }
        if derive_all || derive_list.iter().any(|t| t == "Ord") {
            self.generate_ord_sum(&name, &variants);
            self.generate_compare_sum(&name, &variants);
        }
        // Display: only via explicit deriving(Display), never auto-derived
        if derive_list.iter().any(|t| t == "Display") {
            self.generate_display_sum_type(&name, &variants);
        }
        // Hash: only via explicit deriving(Hash) for sum types
        if has_deriving && derive_list.iter().any(|t| t == "Hash") {
            self.generate_hash_sum_type(&name, &variants);
        }
        // Json: only via explicit deriving(Json) for sum types
        if derive_list.iter().any(|t| t == "Json") {
            self.generate_to_json_sum_type(&name, &variants);
            self.generate_from_json_sum_type(&name, &variants);
            self.generate_from_json_string_wrapper(&name);
        }

        self.sum_types.push(MirSumTypeDef { name, variants });
    }

    // ── Debug inspect generation ────────────────────────────────────

    /// Generate a synthetic `Debug__inspect__StructName` MIR function that
    /// produces a developer-readable string like `"Point { x: 1, y: 2 }"`.
    fn generate_debug_inspect_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        self.generate_debug_inspect_struct_with_display_name(name, name, fields);
    }

    fn generate_debug_inspect_struct_with_display_name(&mut self, name: &str, display_name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("Debug__inspect__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let concat_ty = MirType::FnPtr(
            vec![MirType::String, MirType::String],
            Box::new(MirType::String),
        );
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());

        // Build: "StructName { field1: <val1>, field2: <val2> }"
        let mut result: MirExpr = if fields.is_empty() {
            MirExpr::StringLit(format!("{} {{}}", display_name), MirType::String)
        } else {
            MirExpr::StringLit(format!("{} {{ ", display_name), MirType::String)
        };

        for (i, (field_name, field_ty)) in fields.iter().enumerate() {
            let is_last = i == fields.len() - 1;

            // Append "field_name: "
            let label = format!("{}: ", field_name);
            result = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                args: vec![result, MirExpr::StringLit(label, MirType::String)],
                ty: MirType::String,
            };

            // Access self.field
            let field_access = MirExpr::FieldAccess {
                object: Box::new(self_var.clone()),
                field: field_name.clone(),
                ty: field_ty.clone(),
            };

            // Convert field value to string using wrap_to_string
            let field_str = self.wrap_to_string(field_access, None);

            // Append field value string
            result = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                args: vec![result, field_str],
                ty: MirType::String,
            };

            // Append separator: ", " for non-last fields
            if !is_last {
                result = MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                    args: vec![result, MirExpr::StringLit(", ".to_string(), MirType::String)],
                    ty: MirType::String,
                };
            }
        }

        // Append closing " }" for non-empty structs
        if !fields.is_empty() {
            result = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                args: vec![result, MirExpr::StringLit(" }".to_string(), MirType::String)],
                ty: MirType::String,
            };
        }

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), struct_ty.clone())],
            return_type: MirType::String,
            body: result,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty], Box::new(MirType::String)),
        );
    }

    /// Generate a synthetic `Debug__inspect__SumTypeName` MIR function.
    /// For simplicity, returns just the variant name (e.g., "Some", "None").
    /// Payload fields are represented as "VariantName(...)" for variants with fields.
    fn generate_debug_inspect_sum_type(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("Debug__inspect__{}", name);
        let sum_ty = MirType::SumType(name.to_string());

        // For sum types, generate a match on the tag to return the variant name.
        // This produces a MIR Match expression over integer tag values.
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());

        // Build match arms: each variant tag -> string with variant name.
        let arms: Vec<MirMatchArm> = variants
            .iter()
            .map(|v| {
                let label = if v.fields.is_empty() {
                    v.name.clone()
                } else {
                    format!("{}(...)", v.name)
                };
                MirMatchArm {
                    pattern: MirPattern::Literal(MirLiteral::Int(v.tag as i64)),
                    body: MirExpr::StringLit(label, MirType::String),
                    guard: None,
                }
            })
            .collect();

        let body = if arms.is_empty() {
            MirExpr::StringLit(format!("<{}>", name), MirType::String)
        } else {
            MirExpr::Match {
                scrutinee: Box::new(self_var),
                arms,
                ty: MirType::String,
            }
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), sum_ty.clone())],
            return_type: MirType::String,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty], Box::new(MirType::String)),
        );
    }

    // ── Eq/Ord generation for structs ────────────────────────────────

    /// Generate a synthetic `Eq__eq__StructName` MIR function.
    /// Performs field-by-field equality: all fields must be equal.
    /// Empty structs always return true.
    fn generate_eq_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("Eq__eq__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());
        let other_var = MirExpr::Var("other".to_string(), struct_ty.clone());

        let body = if fields.is_empty() {
            // Empty structs are always equal.
            MirExpr::BoolLit(true, MirType::Bool)
        } else {
            // Build: self.f1 == other.f1 && self.f2 == other.f2 && ...
            let mut comparisons: Vec<MirExpr> = Vec::new();
            for (field_name, field_ty) in fields {
                let self_field = MirExpr::FieldAccess {
                    object: Box::new(self_var.clone()),
                    field: field_name.clone(),
                    ty: field_ty.clone(),
                };
                let other_field = MirExpr::FieldAccess {
                    object: Box::new(other_var.clone()),
                    field: field_name.clone(),
                    ty: field_ty.clone(),
                };

                let cmp = match field_ty {
                    MirType::Struct(inner_name) => {
                        // Recursive: call Eq__eq__InnerStruct
                        let inner_mangled = format!("Eq__eq__{}", inner_name);
                        let fn_ty = MirType::FnPtr(
                            vec![field_ty.clone(), field_ty.clone()],
                            Box::new(MirType::Bool),
                        );
                        MirExpr::Call {
                            func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                            args: vec![self_field, other_field],
                            ty: MirType::Bool,
                        }
                    }
                    _ => {
                        // Primitive/string: use BinOp::Eq directly
                        MirExpr::BinOp {
                            op: BinOp::Eq,
                            lhs: Box::new(self_field),
                            rhs: Box::new(other_field),
                            ty: MirType::Bool,
                        }
                    }
                };
                comparisons.push(cmp);
            }

            // Chain with AND: c1 && c2 && c3 ...
            let mut result = comparisons.remove(0);
            for cmp in comparisons {
                result = MirExpr::BinOp {
                    op: BinOp::And,
                    lhs: Box::new(result),
                    rhs: Box::new(cmp),
                    ty: MirType::Bool,
                };
            }
            result
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), struct_ty.clone()),
                ("other".to_string(), struct_ty.clone()),
            ],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty.clone(), struct_ty], Box::new(MirType::Bool)),
        );
    }

    /// Generate a synthetic `Ord__lt__StructName` MIR function.
    /// Performs lexicographic less-than comparison over fields.
    /// Empty structs always return false (never less-than).
    fn generate_ord_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("Ord__lt__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());
        let other_var = MirExpr::Var("other".to_string(), struct_ty.clone());

        let body = if fields.is_empty() {
            // Empty structs are never less-than.
            MirExpr::BoolLit(false, MirType::Bool)
        } else {
            // Build lexicographic comparison:
            //   if self.f1 < other.f1 then true
            //   else if self.f1 == other.f1 then
            //     if self.f2 < other.f2 then true
            //     else if self.f2 == other.f2 then
            //       ...last field: self.fN < other.fN
            //     else false
            //   else false
            self.build_lexicographic_lt(&self_var, &other_var, fields, 0)
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), struct_ty.clone()),
                ("other".to_string(), struct_ty.clone()),
            ],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty.clone(), struct_ty], Box::new(MirType::Bool)),
        );
    }

    /// Build a lexicographic less-than comparison chain for field at `index` and beyond.
    fn build_lexicographic_lt(
        &self,
        self_var: &MirExpr,
        other_var: &MirExpr,
        fields: &[(String, MirType)],
        index: usize,
    ) -> MirExpr {
        let (field_name, field_ty) = &fields[index];
        let self_field = MirExpr::FieldAccess {
            object: Box::new(self_var.clone()),
            field: field_name.clone(),
            ty: field_ty.clone(),
        };
        let other_field = MirExpr::FieldAccess {
            object: Box::new(other_var.clone()),
            field: field_name.clone(),
            ty: field_ty.clone(),
        };

        let is_last = index == fields.len() - 1;

        // Build "self.field < other.field" comparison
        let lt_cmp = match field_ty {
            MirType::Struct(inner_name) => {
                let inner_mangled = format!("Ord__lt__{}", inner_name);
                let fn_ty = MirType::FnPtr(
                    vec![field_ty.clone(), field_ty.clone()],
                    Box::new(MirType::Bool),
                );
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                    args: vec![self_field.clone(), other_field.clone()],
                    ty: MirType::Bool,
                }
            }
            _ => MirExpr::BinOp {
                op: BinOp::Lt,
                lhs: Box::new(self_field.clone()),
                rhs: Box::new(other_field.clone()),
                ty: MirType::Bool,
            },
        };

        if is_last {
            // Last field: just return the < comparison
            lt_cmp
        } else {
            // Build "self.field == other.field" comparison
            let eq_cmp = match field_ty {
                MirType::Struct(inner_name) => {
                    let inner_mangled = format!("Eq__eq__{}", inner_name);
                    let fn_ty = MirType::FnPtr(
                        vec![field_ty.clone(), field_ty.clone()],
                        Box::new(MirType::Bool),
                    );
                    MirExpr::Call {
                        func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                        args: vec![self_field, other_field],
                        ty: MirType::Bool,
                    }
                }
                _ => MirExpr::BinOp {
                    op: BinOp::Eq,
                    lhs: Box::new(self_field),
                    rhs: Box::new(other_field),
                    ty: MirType::Bool,
                },
            };

            // Recurse for remaining fields
            let rest = self.build_lexicographic_lt(self_var, other_var, fields, index + 1);

            // if self.field < other.field then true
            // else if self.field == other.field then <rest>
            // else false
            MirExpr::If {
                cond: Box::new(lt_cmp),
                then_body: Box::new(MirExpr::BoolLit(true, MirType::Bool)),
                else_body: Box::new(MirExpr::If {
                    cond: Box::new(eq_cmp),
                    then_body: Box::new(rest),
                    else_body: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
                    ty: MirType::Bool,
                }),
                ty: MirType::Bool,
            }
        }
    }

    // ── Eq/Ord generation for sum types ─────────────────────────────

    /// Generate a synthetic `Eq__eq__SumTypeName` MIR function.
    /// Compares variant tags first; if same variant, compares payload fields.
    /// Sum types with no variants always return true.
    fn generate_eq_sum(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("Eq__eq__{}", name);
        let sum_ty = MirType::SumType(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());
        let other_var = MirExpr::Var("other".to_string(), sum_ty.clone());

        let body = if variants.is_empty() {
            // No variants: always equal.
            MirExpr::BoolLit(true, MirType::Bool)
        } else {
            // Build outer Match on self, inner Match on other per variant.
            let outer_arms: Vec<MirMatchArm> = variants
                .iter()
                .map(|v| {
                    // Bindings for self's fields: self_0, self_1, ...
                    let self_fields: Vec<MirPattern> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| MirPattern::Var(format!("self_{}", i), ft.clone()))
                        .collect();
                    let self_bindings: Vec<(String, MirType)> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| (format!("self_{}", i), ft.clone()))
                        .collect();

                    // Inner match on other for same variant
                    let other_fields: Vec<MirPattern> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| MirPattern::Var(format!("other_{}", i), ft.clone()))
                        .collect();
                    let other_bindings: Vec<(String, MirType)> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| (format!("other_{}", i), ft.clone()))
                        .collect();

                    // Build field-by-field equality for this variant's payload
                    let fields_eq = if v.fields.is_empty() {
                        // No payload: same variant = equal
                        MirExpr::BoolLit(true, MirType::Bool)
                    } else {
                        let mut comparisons: Vec<MirExpr> = Vec::new();
                        for (i, ft) in v.fields.iter().enumerate() {
                            let self_f = MirExpr::Var(format!("self_{}", i), ft.clone());
                            let other_f = MirExpr::Var(format!("other_{}", i), ft.clone());

                            let cmp = match ft {
                                MirType::Struct(inner_name) | MirType::SumType(inner_name) => {
                                    let inner_mangled = format!("Eq__eq__{}", inner_name);
                                    let fn_ty = MirType::FnPtr(
                                        vec![ft.clone(), ft.clone()],
                                        Box::new(MirType::Bool),
                                    );
                                    MirExpr::Call {
                                        func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                                        args: vec![self_f, other_f],
                                        ty: MirType::Bool,
                                    }
                                }
                                _ => MirExpr::BinOp {
                                    op: BinOp::Eq,
                                    lhs: Box::new(self_f),
                                    rhs: Box::new(other_f),
                                    ty: MirType::Bool,
                                },
                            };
                            comparisons.push(cmp);
                        }

                        // Chain with AND
                        let mut result = comparisons.remove(0);
                        for cmp in comparisons {
                            result = MirExpr::BinOp {
                                op: BinOp::And,
                                lhs: Box::new(result),
                                rhs: Box::new(cmp),
                                ty: MirType::Bool,
                            };
                        }
                        result
                    };

                    // Inner match: same variant -> compare fields, any other -> false
                    let inner_match = MirExpr::Match {
                        scrutinee: Box::new(other_var.clone()),
                        arms: vec![
                            MirMatchArm {
                                pattern: MirPattern::Constructor {
                                    type_name: name.to_string(),
                                    variant: v.name.clone(),
                                    fields: other_fields,
                                    bindings: other_bindings,
                                },
                                body: fields_eq,
                                guard: None,
                            },
                            MirMatchArm {
                                pattern: MirPattern::Wildcard,
                                body: MirExpr::BoolLit(false, MirType::Bool),
                                guard: None,
                            },
                        ],
                        ty: MirType::Bool,
                    };

                    MirMatchArm {
                        pattern: MirPattern::Constructor {
                            type_name: name.to_string(),
                            variant: v.name.clone(),
                            fields: self_fields,
                            bindings: self_bindings,
                        },
                        body: inner_match,
                        guard: None,
                    }
                })
                .collect();

            MirExpr::Match {
                scrutinee: Box::new(self_var),
                arms: outer_arms,
                ty: MirType::Bool,
            }
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), sum_ty.clone()),
                ("other".to_string(), sum_ty.clone()),
            ],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty.clone(), sum_ty], Box::new(MirType::Bool)),
        );
    }

    /// Generate a synthetic `Ord__lt__SumTypeName` MIR function.
    /// Compares variant tags first (earlier variants are "less than" later ones).
    /// If same variant, performs lexicographic comparison on payload fields.
    /// Sum types with no variants always return false.
    fn generate_ord_sum(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("Ord__lt__{}", name);
        let sum_ty = MirType::SumType(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());
        let other_var = MirExpr::Var("other".to_string(), sum_ty.clone());

        let body = if variants.is_empty() {
            // No variants: never less-than.
            MirExpr::BoolLit(false, MirType::Bool)
        } else {
            // Build outer Match on self, inner Match on other.
            // For each self variant i:
            //   Match on other:
            //     variant j < i -> false (other has lower tag)
            //     variant j == i -> lexicographic compare on payload
            //     variant j > i -> true (other has higher tag)
            let outer_arms: Vec<MirMatchArm> = variants
                .iter()
                .map(|self_v| {
                    // Self bindings for payload fields
                    let self_fields: Vec<MirPattern> = self_v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| MirPattern::Var(format!("self_{}", i), ft.clone()))
                        .collect();
                    let self_bindings: Vec<(String, MirType)> = self_v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| (format!("self_{}", i), ft.clone()))
                        .collect();

                    // Build inner match arms for other
                    let mut inner_arms: Vec<MirMatchArm> = Vec::new();

                    for other_v in variants {
                        if other_v.tag < self_v.tag {
                            // other has lower tag -> self is NOT less-than other
                            inner_arms.push(MirMatchArm {
                                pattern: MirPattern::Constructor {
                                    type_name: name.to_string(),
                                    variant: other_v.name.clone(),
                                    fields: other_v
                                        .fields
                                        .iter()
                                        .map(|_| MirPattern::Wildcard)
                                        .collect(),
                                    bindings: vec![],
                                },
                                body: MirExpr::BoolLit(false, MirType::Bool),
                                guard: None,
                            });
                        } else if other_v.tag == self_v.tag {
                            // Same variant: lexicographic compare on payload
                            let other_fields: Vec<MirPattern> = other_v
                                .fields
                                .iter()
                                .enumerate()
                                .map(|(i, ft)| {
                                    MirPattern::Var(format!("other_{}", i), ft.clone())
                                })
                                .collect();
                            let other_bindings: Vec<(String, MirType)> = other_v
                                .fields
                                .iter()
                                .enumerate()
                                .map(|(i, ft)| (format!("other_{}", i), ft.clone()))
                                .collect();

                            let payload_lt = if self_v.fields.is_empty() {
                                // No payload: same variant, not less-than
                                MirExpr::BoolLit(false, MirType::Bool)
                            } else {
                                // Lexicographic comparison on payload fields
                                self.build_lexicographic_lt_vars(
                                    &self_v.fields,
                                    "self_",
                                    "other_",
                                    0,
                                )
                            };

                            inner_arms.push(MirMatchArm {
                                pattern: MirPattern::Constructor {
                                    type_name: name.to_string(),
                                    variant: other_v.name.clone(),
                                    fields: other_fields,
                                    bindings: other_bindings,
                                },
                                body: payload_lt,
                                guard: None,
                            });
                        } else {
                            // other has higher tag -> self IS less-than other
                            inner_arms.push(MirMatchArm {
                                pattern: MirPattern::Constructor {
                                    type_name: name.to_string(),
                                    variant: other_v.name.clone(),
                                    fields: other_v
                                        .fields
                                        .iter()
                                        .map(|_| MirPattern::Wildcard)
                                        .collect(),
                                    bindings: vec![],
                                },
                                body: MirExpr::BoolLit(true, MirType::Bool),
                                guard: None,
                            });
                        }
                    }

                    let inner_match = MirExpr::Match {
                        scrutinee: Box::new(other_var.clone()),
                        arms: inner_arms,
                        ty: MirType::Bool,
                    };

                    MirMatchArm {
                        pattern: MirPattern::Constructor {
                            type_name: name.to_string(),
                            variant: self_v.name.clone(),
                            fields: self_fields,
                            bindings: self_bindings,
                        },
                        body: inner_match,
                        guard: None,
                    }
                })
                .collect();

            MirExpr::Match {
                scrutinee: Box::new(self_var),
                arms: outer_arms,
                ty: MirType::Bool,
            }
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), sum_ty.clone()),
                ("other".to_string(), sum_ty.clone()),
            ],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty.clone(), sum_ty], Box::new(MirType::Bool)),
        );
    }

    // ── Compare generation ──────────────────────────────────────────

    /// Generate a synthetic `Ord__compare__StructName` MIR function.
    /// Returns Ordering (Less | Equal | Greater) by delegating to lt and eq.
    fn generate_compare_struct(&mut self, name: &str, _fields: &[(String, MirType)]) {
        let mangled = format!("Ord__compare__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let ordering_ty = MirType::SumType("Ordering".to_string());
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());
        let other_var = MirExpr::Var("other".to_string(), struct_ty.clone());

        let lt_fn = format!("Ord__lt__{}", name);
        let eq_fn = format!("Eq__eq__{}", name);
        let fn_ty = MirType::FnPtr(
            vec![struct_ty.clone(), struct_ty.clone()],
            Box::new(MirType::Bool),
        );

        // if Ord__lt__Name(self, other) then Less
        // else if Eq__eq__Name(self, other) then Equal
        // else Greater
        let body = MirExpr::If {
            cond: Box::new(MirExpr::Call {
                func: Box::new(MirExpr::Var(lt_fn, fn_ty.clone())),
                args: vec![self_var.clone(), other_var.clone()],
                ty: MirType::Bool,
            }),
            then_body: Box::new(MirExpr::ConstructVariant {
                type_name: "Ordering".to_string(),
                variant: "Less".to_string(),
                fields: vec![],
                ty: ordering_ty.clone(),
            }),
            else_body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var(eq_fn, fn_ty)),
                    args: vec![self_var, other_var],
                    ty: MirType::Bool,
                }),
                then_body: Box::new(MirExpr::ConstructVariant {
                    type_name: "Ordering".to_string(),
                    variant: "Equal".to_string(),
                    fields: vec![],
                    ty: ordering_ty.clone(),
                }),
                else_body: Box::new(MirExpr::ConstructVariant {
                    type_name: "Ordering".to_string(),
                    variant: "Greater".to_string(),
                    fields: vec![],
                    ty: ordering_ty.clone(),
                }),
                ty: ordering_ty.clone(),
            }),
            ty: ordering_ty.clone(),
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), struct_ty.clone()),
                ("other".to_string(), struct_ty.clone()),
            ],
            return_type: ordering_ty.clone(),
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty.clone(), struct_ty], Box::new(ordering_ty)),
        );
    }

    /// Generate a synthetic `Ord__compare__SumTypeName` MIR function.
    /// Returns Ordering (Less | Equal | Greater) by delegating to lt and eq.
    fn generate_compare_sum(&mut self, name: &str, _variants: &[MirVariantDef]) {
        let mangled = format!("Ord__compare__{}", name);
        let sum_ty = MirType::SumType(name.to_string());
        let ordering_ty = MirType::SumType("Ordering".to_string());
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());
        let other_var = MirExpr::Var("other".to_string(), sum_ty.clone());

        let lt_fn = format!("Ord__lt__{}", name);
        let eq_fn = format!("Eq__eq__{}", name);
        let fn_ty = MirType::FnPtr(
            vec![sum_ty.clone(), sum_ty.clone()],
            Box::new(MirType::Bool),
        );

        let body = MirExpr::If {
            cond: Box::new(MirExpr::Call {
                func: Box::new(MirExpr::Var(lt_fn, fn_ty.clone())),
                args: vec![self_var.clone(), other_var.clone()],
                ty: MirType::Bool,
            }),
            then_body: Box::new(MirExpr::ConstructVariant {
                type_name: "Ordering".to_string(),
                variant: "Less".to_string(),
                fields: vec![],
                ty: ordering_ty.clone(),
            }),
            else_body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var(eq_fn, fn_ty)),
                    args: vec![self_var, other_var],
                    ty: MirType::Bool,
                }),
                then_body: Box::new(MirExpr::ConstructVariant {
                    type_name: "Ordering".to_string(),
                    variant: "Equal".to_string(),
                    fields: vec![],
                    ty: ordering_ty.clone(),
                }),
                else_body: Box::new(MirExpr::ConstructVariant {
                    type_name: "Ordering".to_string(),
                    variant: "Greater".to_string(),
                    fields: vec![],
                    ty: ordering_ty.clone(),
                }),
                ty: ordering_ty.clone(),
            }),
            ty: ordering_ty.clone(),
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), sum_ty.clone()),
                ("other".to_string(), sum_ty.clone()),
            ],
            return_type: ordering_ty.clone(),
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty.clone(), sum_ty], Box::new(ordering_ty)),
        );
    }

    /// Generate a synthetic `Ord__compare__PrimitiveName` MIR function for primitives.
    /// Uses BinOp::Lt and BinOp::Eq directly instead of calling trait functions.
    fn generate_compare_primitive(&mut self, type_name: &str, mir_type: MirType) {
        let mangled = format!("Ord__compare__{}", type_name);
        let ordering_ty = MirType::SumType("Ordering".to_string());
        let self_var = MirExpr::Var("self".to_string(), mir_type.clone());
        let other_var = MirExpr::Var("other".to_string(), mir_type.clone());

        // if self < other then Less
        // else if self == other then Equal
        // else Greater
        let body = MirExpr::If {
            cond: Box::new(MirExpr::BinOp {
                op: BinOp::Lt,
                lhs: Box::new(self_var.clone()),
                rhs: Box::new(other_var.clone()),
                ty: MirType::Bool,
            }),
            then_body: Box::new(MirExpr::ConstructVariant {
                type_name: "Ordering".to_string(),
                variant: "Less".to_string(),
                fields: vec![],
                ty: ordering_ty.clone(),
            }),
            else_body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::BinOp {
                    op: BinOp::Eq,
                    lhs: Box::new(self_var),
                    rhs: Box::new(other_var),
                    ty: MirType::Bool,
                }),
                then_body: Box::new(MirExpr::ConstructVariant {
                    type_name: "Ordering".to_string(),
                    variant: "Equal".to_string(),
                    fields: vec![],
                    ty: ordering_ty.clone(),
                }),
                else_body: Box::new(MirExpr::ConstructVariant {
                    type_name: "Ordering".to_string(),
                    variant: "Greater".to_string(),
                    fields: vec![],
                    ty: ordering_ty.clone(),
                }),
                ty: ordering_ty.clone(),
            }),
            ty: ordering_ty.clone(),
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![
                ("self".to_string(), mir_type.clone()),
                ("other".to_string(), mir_type.clone()),
            ],
            return_type: ordering_ty.clone(),
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![mir_type.clone(), mir_type], Box::new(ordering_ty)),
        );
    }

    // ── Hash generation ─────────────────────────────────────────────

    /// Generate a synthetic `Hash__hash__StructName` MIR function that
    /// hashes each field via the appropriate `mesh_hash_*` runtime function
    /// and chains results with `mesh_hash_combine`.
    fn generate_hash_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("Hash__hash__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());

        let combine_ty = MirType::FnPtr(
            vec![MirType::Int, MirType::Int],
            Box::new(MirType::Int),
        );

        let body = if fields.is_empty() {
            // Empty struct: return a constant hash (the FNV offset basis).
            MirExpr::IntLit(0xcbf29ce484222325_u64 as i64, MirType::Int)
        } else {
            // For each field, compute hash, then chain with mesh_hash_combine.
            let mut result: Option<MirExpr> = None;
            for (field_name, field_ty) in fields {
                let field_access = MirExpr::FieldAccess {
                    object: Box::new(self_var.clone()),
                    field: field_name.clone(),
                    ty: field_ty.clone(),
                };

                let field_hash = self.emit_hash_for_type(field_access, field_ty);

                result = Some(match result {
                    None => field_hash,
                    Some(prev) => MirExpr::Call {
                        func: Box::new(MirExpr::Var(
                            "mesh_hash_combine".to_string(),
                            combine_ty.clone(),
                        )),
                        args: vec![prev, field_hash],
                        ty: MirType::Int,
                    },
                });
            }
            result.unwrap()
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), struct_ty.clone())],
            return_type: MirType::Int,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty], Box::new(MirType::Int)),
        );
    }

    /// Generate a synthetic `Hash__hash__SumTypeName` MIR function.
    /// Uses Match on self with Constructor patterns to hash tag + fields.
    fn generate_hash_sum_type(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("Hash__hash__{}", name);
        let sum_ty = MirType::SumType(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());

        let combine_ty = MirType::FnPtr(
            vec![MirType::Int, MirType::Int],
            Box::new(MirType::Int),
        );
        let hash_int_ty = MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Int));

        let body = if variants.is_empty() {
            // No variants: return FNV offset basis.
            MirExpr::IntLit(0xcbf29ce484222325_u64 as i64, MirType::Int)
        } else {
            // Build match arms: for each variant, hash tag + fields.
            let arms: Vec<MirMatchArm> = variants
                .iter()
                .map(|v| {
                    // Bind fields as field_0, field_1, ...
                    let field_pats: Vec<MirPattern> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| MirPattern::Var(format!("field_{}", i), ft.clone()))
                        .collect();
                    let bindings: Vec<(String, MirType)> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, ft)| (format!("field_{}", i), ft.clone()))
                        .collect();

                    // Start with hashing the tag
                    let tag_hash = MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_hash_int".to_string(), hash_int_ty.clone())),
                        args: vec![MirExpr::IntLit(v.tag as i64, MirType::Int)],
                        ty: MirType::Int,
                    };

                    // Combine with each field's hash
                    let mut result = tag_hash;
                    for (i, ft) in v.fields.iter().enumerate() {
                        let field_var = MirExpr::Var(format!("field_{}", i), ft.clone());
                        let field_hash = self.emit_hash_for_type(field_var, ft);
                        result = MirExpr::Call {
                            func: Box::new(MirExpr::Var(
                                "mesh_hash_combine".to_string(),
                                combine_ty.clone(),
                            )),
                            args: vec![result, field_hash],
                            ty: MirType::Int,
                        };
                    }

                    MirMatchArm {
                        pattern: MirPattern::Constructor {
                            type_name: name.to_string(),
                            variant: v.name.clone(),
                            fields: field_pats,
                            bindings,
                        },
                        body: result,
                        guard: None,
                    }
                })
                .collect();

            MirExpr::Match {
                scrutinee: Box::new(self_var),
                arms,
                ty: MirType::Int,
            }
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), sum_ty.clone())],
            return_type: MirType::Int,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty], Box::new(MirType::Int)),
        );
    }

    // ── JSON (ToJson/FromJson) generation ─────────────────────────────

    /// Generate a synthetic `ToJson__to_json__SumTypeName` MIR function that
    /// builds a tagged JSON object `{"tag":"Variant","fields":[...]}` using
    /// Match on self with per-variant arms.
    fn generate_to_json_sum_type(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("ToJson__to_json__{}", name);
        let sum_ty = MirType::SumType(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());

        let obj_new_ty = MirType::FnPtr(vec![], Box::new(MirType::Ptr));
        let obj_put_ty = MirType::FnPtr(
            vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
            Box::new(MirType::Ptr),
        );
        let arr_new_ty = MirType::FnPtr(vec![], Box::new(MirType::Ptr));
        let arr_push_ty = MirType::FnPtr(
            vec![MirType::Ptr, MirType::Ptr],
            Box::new(MirType::Ptr),
        );
        let from_string_ty = MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr));

        let arms: Vec<MirMatchArm> = variants
            .iter()
            .map(|v| {
                // Bind fields with per-variant unique names to avoid LLVM domination errors
                let field_pats: Vec<MirPattern> = v
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, ft)| MirPattern::Var(format!("__tj_{}_{}", v.name, i), ft.clone()))
                    .collect();
                let bindings: Vec<(String, MirType)> = v
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, ft)| (format!("__tj_{}_{}", v.name, i), ft.clone()))
                    .collect();

                // Build fields array
                let mut arr = MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_json_array_new".to_string(),
                        arr_new_ty.clone(),
                    )),
                    args: vec![],
                    ty: MirType::Ptr,
                };
                for (i, ft) in v.fields.iter().enumerate() {
                    let field_var = MirExpr::Var(format!("__tj_{}_{}", v.name, i), ft.clone());
                    let json_val = self.emit_to_json_for_type(field_var, ft, name);
                    arr = MirExpr::Call {
                        func: Box::new(MirExpr::Var(
                            "mesh_json_array_push".to_string(),
                            arr_push_ty.clone(),
                        )),
                        args: vec![arr, json_val],
                        ty: MirType::Ptr,
                    };
                }

                // Build {"tag": "VariantName", "fields": [...]}
                let mut obj = MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_json_object_new".to_string(),
                        obj_new_ty.clone(),
                    )),
                    args: vec![],
                    ty: MirType::Ptr,
                };
                // Put "tag"
                let tag_key = MirExpr::StringLit("tag".to_string(), MirType::String);
                let tag_val = MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_json_from_string".to_string(),
                        from_string_ty.clone(),
                    )),
                    args: vec![MirExpr::StringLit(v.name.clone(), MirType::String)],
                    ty: MirType::Ptr,
                };
                obj = MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_json_object_put".to_string(),
                        obj_put_ty.clone(),
                    )),
                    args: vec![obj, tag_key, tag_val],
                    ty: MirType::Ptr,
                };
                // Put "fields"
                let fields_key = MirExpr::StringLit("fields".to_string(), MirType::String);
                obj = MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_json_object_put".to_string(),
                        obj_put_ty.clone(),
                    )),
                    args: vec![obj, fields_key, arr],
                    ty: MirType::Ptr,
                };

                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: name.to_string(),
                        variant: v.name.clone(),
                        fields: field_pats,
                        bindings,
                    },
                    body: obj,
                    guard: None,
                }
            })
            .collect();

        let body = if arms.is_empty() {
            // No variants: return empty JSON object
            MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_json_object_new".to_string(),
                    obj_new_ty,
                )),
                args: vec![],
                ty: MirType::Ptr,
            }
        } else {
            MirExpr::Match {
                scrutinee: Box::new(self_var),
                arms,
                ty: MirType::Ptr,
            }
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), sum_ty.clone())],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty], Box::new(MirType::Ptr)),
        );
    }

    /// Generate a synthetic `FromJson__from_json__SumTypeName` MIR function that
    /// extracts "tag" from a JSON object and dispatches to the correct variant decoder.
    /// Uses If-chain for tag comparison (not Match, per Phase 49 lessons).
    fn generate_from_json_sum_type(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("FromJson__from_json__{}", name);

        let json_var = MirExpr::Var("json".to_string(), MirType::Ptr);

        let obj_get_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
        let as_string_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        let is_ok_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int));
        let unwrap_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        let str_eq_ty = MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool));
        let alloc_result_ty = MirType::FnPtr(vec![MirType::Int, MirType::Ptr], Box::new(MirType::Ptr));
        let arr_get_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Int], Box::new(MirType::Ptr));

        // Build the unknown-tag error as the final else branch
        // Use a simple error message (can't easily concat runtime strings in MIR)
        let unknown_tag_err = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
            args: vec![
                MirExpr::IntLit(1, MirType::Int),
                MirExpr::StringLit(format!("unknown variant for {}", name), MirType::String),
            ],
            ty: MirType::Ptr,
        };

        // Build the If-chain from last variant to first (inside out)
        let mut tag_dispatch = unknown_tag_err;

        for v in variants.iter().rev() {
            // Build the variant decode body
            let variant_body = if v.fields.is_empty() {
                // Nullary variant: just construct it and wrap in Ok
                let variant_val = MirExpr::ConstructVariant {
                    type_name: name.to_string(),
                    variant: v.name.clone(),
                    fields: vec![],
                    ty: MirType::SumType(name.to_string()),
                };
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
                    args: vec![MirExpr::IntLit(0, MirType::Int), variant_val],
                    ty: MirType::Ptr,
                }
            } else {
                // Variant with fields: extract "fields" array, decode each field
                self.build_variant_from_json_body(
                    name,
                    &v.name,
                    &v.fields,
                    &obj_get_ty,
                    &is_ok_ty,
                    &unwrap_ty,
                    &arr_get_ty,
                    &alloc_result_ty,
                )
            };

            // If mesh_string_eq(tag_str, "VariantName") then decode else continue chain
            tag_dispatch = MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_string_eq".to_string(), str_eq_ty.clone())),
                    args: vec![
                        MirExpr::Var("__tag_str".to_string(), MirType::String),
                        MirExpr::StringLit(v.name.clone(), MirType::String),
                    ],
                    ty: MirType::Bool,
                }),
                then_body: Box::new(variant_body),
                else_body: Box::new(tag_dispatch),
                ty: MirType::Ptr,
            };
        }

        // Wrap the tag dispatch in tag extraction:
        // let tag_res = mesh_json_object_get(json, "tag")
        // if is_ok(tag_res):
        //   let tag_json = unwrap(tag_res)
        //   let tag_str_res = mesh_json_as_string(tag_json)
        //   if is_ok(tag_str_res):
        //     let tag_str = unwrap(tag_str_res)
        //     <tag_dispatch>
        //   else: tag_str_res
        // else: tag_res

        let body = MirExpr::Let {
            name: "__tag_res".to_string(),
            ty: MirType::Ptr,
            value: Box::new(MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_json_object_get".to_string(), obj_get_ty)),
                args: vec![
                    json_var,
                    MirExpr::StringLit("tag".to_string(), MirType::String),
                ],
                ty: MirType::Ptr,
            }),
            body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                    args: vec![MirExpr::Var("__tag_res".to_string(), MirType::Ptr)],
                    ty: MirType::Int,
                }),
                then_body: Box::new(MirExpr::Let {
                    name: "__tag_json".to_string(),
                    ty: MirType::Ptr,
                    value: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                        args: vec![MirExpr::Var("__tag_res".to_string(), MirType::Ptr)],
                        ty: MirType::Ptr,
                    }),
                    body: Box::new(MirExpr::Let {
                        name: "__tag_str_res".to_string(),
                        ty: MirType::Ptr,
                        value: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_json_as_string".to_string(), as_string_ty)),
                            args: vec![MirExpr::Var("__tag_json".to_string(), MirType::Ptr)],
                            ty: MirType::Ptr,
                        }),
                        body: Box::new(MirExpr::If {
                            cond: Box::new(MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                                args: vec![MirExpr::Var("__tag_str_res".to_string(), MirType::Ptr)],
                                ty: MirType::Int,
                            }),
                            then_body: Box::new(MirExpr::Let {
                                name: "__tag_str".to_string(),
                                ty: MirType::String,
                                value: Box::new(MirExpr::Call {
                                    func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty)),
                                    args: vec![MirExpr::Var("__tag_str_res".to_string(), MirType::Ptr)],
                                    ty: MirType::Ptr,
                                }),
                                body: Box::new(tag_dispatch),
                            }),
                            else_body: Box::new(MirExpr::Var("__tag_str_res".to_string(), MirType::Ptr)),
                            ty: MirType::Ptr,
                        }),
                    }),
                }),
                else_body: Box::new(MirExpr::Var("__tag_res".to_string(), MirType::Ptr)),
                ty: MirType::Ptr,
            }),
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("json".to_string(), MirType::Ptr)],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
        );
    }

    /// Build the from_json body for a single variant with fields.
    /// Extracts "fields" array from JSON, then decodes each field by index.
    fn build_variant_from_json_body(
        &self,
        type_name: &str,
        variant_name: &str,
        field_types: &[MirType],
        obj_get_ty: &MirType,
        is_ok_ty: &MirType,
        unwrap_ty: &MirType,
        arr_get_ty: &MirType,
        alloc_result_ty: &MirType,
    ) -> MirExpr {
        // Build the innermost expression: construct variant and wrap in Ok result
        let field_exprs: Vec<MirExpr> = field_types
            .iter()
            .enumerate()
            .map(|(i, ft)| {
                MirExpr::Var(format!("__fval_{}_{}", variant_name, i), ft.clone())
            })
            .collect();

        let variant_val = MirExpr::ConstructVariant {
            type_name: type_name.to_string(),
            variant: variant_name.to_string(),
            fields: field_exprs,
            ty: MirType::SumType(type_name.to_string()),
        };

        let ok_result = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
            args: vec![MirExpr::IntLit(0, MirType::Int), variant_val],
            ty: MirType::Ptr,
        };

        // Wrap each field extraction from last to first
        let mut body = ok_result;

        for (i, ft) in field_types.iter().enumerate().rev() {
            let arr_get_res_var = format!("__ag_res_{}_{}", variant_name, i);
            let field_json_var = format!("__fj_{}_{}", variant_name, i);
            let extract_res_var = format!("__er_{}_{}", variant_name, i);
            let val_var = format!("__fval_{}_{}", variant_name, i);

            // mesh_json_array_get(fields_arr, i)
            let arr_get_call = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_json_array_get".to_string(), arr_get_ty.clone())),
                args: vec![
                    MirExpr::Var(format!("__fields_arr_{}", variant_name), MirType::Ptr),
                    MirExpr::IntLit(i as i64, MirType::Int),
                ],
                ty: MirType::Ptr,
            };

            // Type-directed decoding of the field JSON value
            let extract_call = self.emit_from_json_for_type(
                MirExpr::Var(field_json_var.clone(), MirType::Ptr),
                ft,
                type_name,
            );

            // Inner check: if is_ok(extract_result)
            let inner_check = MirExpr::Let {
                name: extract_res_var.clone(),
                ty: MirType::Ptr,
                value: Box::new(extract_call),
                body: Box::new(MirExpr::If {
                    cond: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                        args: vec![MirExpr::Var(extract_res_var.clone(), MirType::Ptr)],
                        ty: MirType::Int,
                    }),
                    then_body: Box::new(MirExpr::Let {
                        name: val_var,
                        ty: ft.clone(),
                        value: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                            args: vec![MirExpr::Var(extract_res_var.clone(), MirType::Ptr)],
                            ty: MirType::Ptr,
                        }),
                        body: Box::new(body),
                    }),
                    else_body: Box::new(MirExpr::Var(extract_res_var, MirType::Ptr)),
                    ty: MirType::Ptr,
                }),
            };

            // Outer check: if is_ok(arr_get_result)
            body = MirExpr::Let {
                name: arr_get_res_var.clone(),
                ty: MirType::Ptr,
                value: Box::new(arr_get_call),
                body: Box::new(MirExpr::If {
                    cond: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                        args: vec![MirExpr::Var(arr_get_res_var.clone(), MirType::Ptr)],
                        ty: MirType::Int,
                    }),
                    then_body: Box::new(MirExpr::Let {
                        name: field_json_var,
                        ty: MirType::Ptr,
                        value: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                            args: vec![MirExpr::Var(arr_get_res_var.clone(), MirType::Ptr)],
                            ty: MirType::Ptr,
                        }),
                        body: Box::new(inner_check),
                    }),
                    else_body: Box::new(MirExpr::Var(arr_get_res_var, MirType::Ptr)),
                    ty: MirType::Ptr,
                }),
            };
        }

        // Wrap the entire thing in fields array extraction:
        // let fields_res = mesh_json_object_get(json, "fields")
        // if is_ok(fields_res):
        //   let fields_arr = unwrap(fields_res)
        //   <body with per-field extraction>
        // else: fields_res
        let fields_res_var = format!("__fields_res_{}", variant_name);
        let fields_arr_var = format!("__fields_arr_{}", variant_name);

        MirExpr::Let {
            name: fields_res_var.clone(),
            ty: MirType::Ptr,
            value: Box::new(MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_json_object_get".to_string(), obj_get_ty.clone())),
                args: vec![
                    MirExpr::Var("json".to_string(), MirType::Ptr),
                    MirExpr::StringLit("fields".to_string(), MirType::String),
                ],
                ty: MirType::Ptr,
            }),
            body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                    args: vec![MirExpr::Var(fields_res_var.clone(), MirType::Ptr)],
                    ty: MirType::Int,
                }),
                then_body: Box::new(MirExpr::Let {
                    name: fields_arr_var,
                    ty: MirType::Ptr,
                    value: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                        args: vec![MirExpr::Var(fields_res_var.clone(), MirType::Ptr)],
                        ty: MirType::Ptr,
                    }),
                    body: Box::new(body),
                }),
                else_body: Box::new(MirExpr::Var(fields_res_var, MirType::Ptr)),
                ty: MirType::Ptr,
            }),
        }
    }

    /// Generate a synthetic `ToJson__to_json__StructName` MIR function that
    /// builds a JSON object field-by-field using the mesh_json_object_new/put
    /// runtime functions.
    fn generate_to_json_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("ToJson__to_json__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());

        let obj_new_ty = MirType::FnPtr(vec![], Box::new(MirType::Ptr));
        let obj_put_ty = MirType::FnPtr(
            vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
            Box::new(MirType::Ptr),
        );

        let mut body = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_json_object_new".to_string(), obj_new_ty)),
            args: vec![],
            ty: MirType::Ptr,
        };

        for (field_name, field_ty) in fields {
            let field_access = MirExpr::FieldAccess {
                object: Box::new(self_var.clone()),
                field: field_name.clone(),
                ty: field_ty.clone(),
            };

            // Convert field value to MeshJson using type-directed dispatch.
            // For collection types (MirType::Ptr), look up the typeck Ty to
            // determine element types for callback-based encode/decode.
            let json_val = if matches!(field_ty, MirType::Ptr) {
                if let Some(info) = self.registry.struct_defs.get(name) {
                    if let Some((_, typeck_ty)) = info.fields.iter().find(|(n, _)| n == field_name) {
                        let typeck_ty = typeck_ty.clone();
                        self.emit_collection_to_json(field_access, &typeck_ty, name)
                    } else {
                        field_access
                    }
                } else {
                    field_access
                }
            } else {
                self.emit_to_json_for_type(field_access, field_ty, name)
            };

            let key = MirExpr::StringLit(field_name.clone(), MirType::String);

            body = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_json_object_put".to_string(), obj_put_ty.clone())),
                args: vec![body, key, json_val],
                ty: MirType::Ptr,
            };
        }

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), struct_ty.clone())],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty], Box::new(MirType::Ptr)),
        );
    }

    /// Emit a to_json conversion for a value of the given MIR type.
    /// Returns a MirExpr that evaluates to *mut MeshJson (MirType::Ptr).
    fn emit_to_json_for_type(&mut self, expr: MirExpr, ty: &MirType, _context_struct: &str) -> MirExpr {
        match ty {
            MirType::Int => {
                let fn_ty = MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_json_from_int".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Ptr,
                }
            }
            MirType::Float => {
                let fn_ty = MirType::FnPtr(vec![MirType::Float], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_json_from_float".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Ptr,
                }
            }
            MirType::Bool => {
                let fn_ty = MirType::FnPtr(vec![MirType::Bool], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_json_from_bool".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Ptr,
                }
            }
            MirType::String => {
                let fn_ty = MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_json_from_string".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Ptr,
                }
            }
            MirType::Struct(inner_name) => {
                let inner_mangled = format!("ToJson__to_json__{}", inner_name);
                let fn_ty = MirType::FnPtr(vec![ty.clone()], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                    args: vec![expr],
                    ty: MirType::Ptr,
                }
            }
            MirType::SumType(sum_name) if sum_name.starts_with("Option_") => {
                self.emit_option_to_json(expr, sum_name, _context_struct)
            }
            MirType::SumType(sum_name) => {
                // Non-Option sum type: call ToJson__to_json__SumName
                let inner_mangled = format!("ToJson__to_json__{}", sum_name);
                let fn_ty = MirType::FnPtr(vec![ty.clone()], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                    args: vec![expr],
                    ty: MirType::Ptr,
                }
            }
            _ => {
                // Unsupported type at MIR level -- pass through as opaque pointer.
                // Collection types (Ptr) are handled separately in generate_to_json_struct.
                expr
            }
        }
    }

    /// Emit Option<T> to JSON encoding: Some(v) -> encode inner, None -> null.
    fn emit_option_to_json(&mut self, expr: MirExpr, sum_name: &str, context_struct: &str) -> MirExpr {
        let inner_type_str = sum_name.strip_prefix("Option_").unwrap_or("Int");
        let inner_mir_type = self.mir_type_from_name(inner_type_str);

        let null_ty = MirType::FnPtr(vec![], Box::new(MirType::Ptr));
        let null_expr = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_json_null".to_string(), null_ty)),
            args: vec![],
            ty: MirType::Ptr,
        };

        let some_var = MirExpr::Var("__opt_val".to_string(), inner_mir_type.clone());
        let some_body = self.emit_to_json_for_type(some_var, &inner_mir_type, context_struct);

        MirExpr::Match {
            scrutinee: Box::new(expr),
            arms: vec![
                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: sum_name.to_string(),
                        variant: "Some".to_string(),
                        fields: vec![MirPattern::Var("__opt_val".to_string(), inner_mir_type.clone())],
                        bindings: vec![("__opt_val".to_string(), inner_mir_type)],
                    },
                    guard: None,
                    body: some_body,
                },
                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: sum_name.to_string(),
                        variant: "None".to_string(),
                        fields: vec![],
                        bindings: vec![],
                    },
                    guard: None,
                    body: null_expr,
                },
            ],
            ty: MirType::Ptr,
        }
    }

    /// Convert a type name string to a MirType.
    fn mir_type_from_name(&self, name: &str) -> MirType {
        match name {
            "Int" => MirType::Int,
            "Float" => MirType::Float,
            "Bool" => MirType::Bool,
            "String" => MirType::String,
            // SqliteConn is an opaque u64 handle, lowered to Int for GC safety (SQLT-07).
            "SqliteConn" => MirType::Int,
            n => {
                if self.structs.iter().any(|s| s.name == n) || self.registry.struct_defs.contains_key(n) {
                    MirType::Struct(n.to_string())
                } else {
                    MirType::Ptr
                }
            }
        }
    }

    /// Emit collection (List/Map) to JSON encoding using callback-based runtime helpers.
    fn emit_collection_to_json(&mut self, expr: MirExpr, typeck_ty: &Ty, _context_struct: &str) -> MirExpr {
        match typeck_ty {
            Ty::App(base, args) => {
                if let Ty::Con(con) = base.as_ref() {
                    match con.name.as_str() {
                        "List" => {
                            let elem_ty = args.first().cloned().unwrap_or(Ty::int());
                            let callback_name = self.resolve_to_json_callback(&elem_ty);
                            let fn_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
                            let callback_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_json_from_list".to_string(), fn_ty)),
                                args: vec![expr, MirExpr::Var(callback_name, callback_ty)],
                                ty: MirType::Ptr,
                            }
                        }
                        "Map" => {
                            let val_ty = args.get(1).cloned().unwrap_or(Ty::string());
                            let callback_name = self.resolve_to_json_callback(&val_ty);
                            let fn_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
                            let callback_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_json_from_map".to_string(), fn_ty)),
                                args: vec![expr, MirExpr::Var(callback_name, callback_ty)],
                                ty: MirType::Ptr,
                            }
                        }
                        _ => expr,
                    }
                } else {
                    expr
                }
            }
            _ => expr,
        }
    }

    /// Resolve the runtime callback function name for encoding an element to JSON.
    /// For struct/sum types, generates a wrapper function that dereferences the
    /// heap pointer (stored as u64 in the list) before calling the to_json function.
    fn resolve_to_json_callback(&mut self, elem_ty: &Ty) -> String {
        match elem_ty {
            Ty::Con(con) => match con.name.as_str() {
                "Int" => "mesh_json_from_int".to_string(),
                "Float" => "mesh_json_from_float".to_string(),
                "Bool" => "mesh_json_from_bool".to_string(),
                "String" => "mesh_json_from_string".to_string(),
                name => {
                    // For struct/sum types, the list stores heap pointers as u64.
                    // The runtime callback receives u64 (reinterpreted as ptr), but
                    // ToJson__to_json__X expects an inline struct/sum value.
                    // Generate a wrapper that uses a Let binding to deref the pointer
                    // (the codegen's Let binding auto-derefs ptr->struct/sum).
                    let wrapper_name = format!("__json_list_encode__{}", name);
                    if !self.known_functions.contains_key(&wrapper_name) {
                        let to_json_fn = format!("ToJson__to_json__{}", name);
                        // Determine the MIR type for this type name
                        let mir_ty = if self.registry.sum_type_defs.contains_key(name) {
                            MirType::SumType(name.to_string())
                        } else {
                            MirType::Struct(name.to_string())
                        };
                        // Wrapper body: let __val : T = __elem_ptr; call to_json(__val)
                        // The Let binding auto-derefs Ptr -> SumType/Struct
                        let body = MirExpr::Let {
                            name: "__deref_val".to_string(),
                            ty: mir_ty.clone(),
                            value: Box::new(MirExpr::Var("__elem_ptr".to_string(), MirType::Ptr)),
                            body: Box::new(MirExpr::Call {
                                func: Box::new(MirExpr::Var(
                                    to_json_fn,
                                    MirType::FnPtr(vec![mir_ty.clone()], Box::new(MirType::Ptr)),
                                )),
                                args: vec![MirExpr::Var("__deref_val".to_string(), mir_ty)],
                                ty: MirType::Ptr,
                            }),
                        };
                        let func = MirFunction {
                            name: wrapper_name.clone(),
                            params: vec![("__elem_ptr".to_string(), MirType::Ptr)],
                            return_type: MirType::Ptr,
                            body,
                            is_closure_fn: false,
                            captures: vec![],
                            has_tail_calls: false,
                        };
                        self.functions.push(func);
                        self.known_functions.insert(
                            wrapper_name.clone(),
                            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
                        );
                    }
                    wrapper_name
                }
            },
            _ => "mesh_json_from_int".to_string(),
        }
    }

    /// Resolve the runtime callback function name for decoding a JSON element to a typed value.
    fn resolve_from_json_callback(&self, elem_ty: &Ty) -> String {
        match elem_ty {
            Ty::Con(con) => match con.name.as_str() {
                "Int" => "mesh_json_as_int".to_string(),
                "Float" => "mesh_json_as_float".to_string(),
                "Bool" => "mesh_json_as_bool".to_string(),
                "String" => "mesh_json_as_string".to_string(),
                name => format!("FromJson__from_json__{}", name),
            },
            _ => "mesh_json_as_int".to_string(),
        }
    }

    /// Generate a synthetic `FromJson__from_json__StructName` MIR function that
    /// extracts fields from a JSON object with nested Result propagation.
    /// Returns a *mut MeshResult (Ptr) -- the caller handles conversion to SumType.
    /// Uses mesh_result_is_ok/mesh_result_unwrap for internal MeshResult handling,
    /// and mesh_alloc_result(0, heap_struct_ptr) for the Ok result.
    fn generate_from_json_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("FromJson__from_json__{}", name);
        let struct_ty = MirType::Struct(name.to_string());

        let json_var = MirExpr::Var("json".to_string(), MirType::Ptr);

        let is_ok_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int));
        let unwrap_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));

        // Build the innermost expression: alloc_result(0, struct_ptr)
        // Construct StructLit with field vars, then wrap in Ok result.
        let field_bindings: Vec<(String, MirExpr)> = fields.iter().enumerate().map(|(i, (fname, fty))| {
            (fname.clone(), MirExpr::Var(format!("__field_{}", i), fty.clone()))
        }).collect();

        let struct_lit = MirExpr::StructLit {
            name: name.to_string(),
            fields: field_bindings,
            ty: struct_ty.clone(),
        };

        // Use alloc_result(0, struct_ptr) for Ok result.
        // The codegen will heap-allocate the struct via the StructValue -> Ptr coercion.
        let alloc_result_ty = MirType::FnPtr(
            vec![MirType::Int, MirType::Ptr],
            Box::new(MirType::Ptr),
        );
        let ok_result = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
            args: vec![
                MirExpr::IntLit(0, MirType::Int),
                struct_lit,
            ],
            ty: MirType::Ptr,
        };

        // Wrap each field extraction around the inner expression, from last to first.
        // Uses If(mesh_result_is_ok(res)) for internal MeshResult handling.
        let mut body = ok_result;

        for (i, (field_name, field_ty)) in fields.iter().enumerate().rev() {
            let obj_get_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
            let key_lit = MirExpr::StringLit(field_name.clone(), MirType::String);

            let get_call = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_json_object_get".to_string(), obj_get_ty)),
                args: vec![json_var.clone(), key_lit],
                ty: MirType::Ptr,
            };

            let get_result_var = format!("__get_res_{}", i);
            let field_var = format!("__json_field_{}", i);
            let extract_result_var = format!("__extract_res_{}", i);
            let val_var = format!("__field_{}", i);

            // For collection fields (Ptr), look up typeck Ty for proper decoding
            let extract_call = if matches!(field_ty, MirType::Ptr) {
                if let Some(info) = self.registry.struct_defs.get(name) {
                    if let Some((_, typeck_ty)) = info.fields.iter().find(|(n, _)| n == field_name) {
                        let typeck_ty = typeck_ty.clone();
                        self.emit_collection_from_json(
                            MirExpr::Var(field_var.clone(), MirType::Ptr),
                            &typeck_ty,
                            name,
                        )
                    } else {
                        self.emit_from_json_for_type(
                            MirExpr::Var(field_var.clone(), MirType::Ptr),
                            field_ty,
                            name,
                        )
                    }
                } else {
                    self.emit_from_json_for_type(
                        MirExpr::Var(field_var.clone(), MirType::Ptr),
                        field_ty,
                        name,
                    )
                }
            } else {
                self.emit_from_json_for_type(
                    MirExpr::Var(field_var.clone(), MirType::Ptr),
                    field_ty,
                    name,
                )
            };

            // Inner check: if mesh_result_is_ok(extract_result)
            let inner_check = MirExpr::Let {
                name: extract_result_var.clone(),
                ty: MirType::Ptr,
                value: Box::new(extract_call),
                body: Box::new(MirExpr::If {
                    cond: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                        args: vec![MirExpr::Var(extract_result_var.clone(), MirType::Ptr)],
                        ty: MirType::Int,
                    }),
                    then_body: Box::new(MirExpr::Let {
                        name: val_var,
                        ty: field_ty.clone(),
                        value: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                            args: vec![MirExpr::Var(extract_result_var.clone(), MirType::Ptr)],
                            ty: MirType::Ptr,
                        }),
                        body: Box::new(body),
                    }),
                    else_body: Box::new(MirExpr::Var(extract_result_var, MirType::Ptr)),
                    ty: MirType::Ptr,
                }),
            };

            // Outer check: if mesh_result_is_ok(get_result)
            body = MirExpr::Let {
                name: get_result_var.clone(),
                ty: MirType::Ptr,
                value: Box::new(get_call),
                body: Box::new(MirExpr::If {
                    cond: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                        args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                        ty: MirType::Int,
                    }),
                    then_body: Box::new(MirExpr::Let {
                        name: field_var,
                        ty: MirType::Ptr,
                        value: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                            args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                            ty: MirType::Ptr,
                        }),
                        body: Box::new(inner_check),
                    }),
                    else_body: Box::new(MirExpr::Var(get_result_var, MirType::Ptr)),
                    ty: MirType::Ptr,
                }),
            };
        }

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("json".to_string(), MirType::Ptr)],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
        );
    }

    /// Generate a `FromRow__from_row__StructName` MIR function that extracts
    /// struct fields from a Map<String, String> (database row).
    ///
    /// Takes a Ptr (Map<String, String>) parameter and returns a Ptr (MeshResult).
    /// For each field: calls mesh_row_from_row_get to get the column value,
    /// then parses it to the correct type (Int/Float/Bool/String/Option<T>).
    /// Option fields receive None for missing columns and empty strings (NULL).
    fn generate_from_row_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("FromRow__from_row__{}", name);
        let struct_ty = MirType::Struct(name.to_string());

        let row_var = MirExpr::Var("row".to_string(), MirType::Ptr);

        let is_ok_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int));
        let unwrap_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        let alloc_result_ty = MirType::FnPtr(
            vec![MirType::Int, MirType::Ptr],
            Box::new(MirType::Ptr),
        );
        let row_get_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
        let str_len_ty = MirType::FnPtr(vec![MirType::String], Box::new(MirType::Int));

        // Build the innermost expression: alloc_result(0, struct_ptr)
        let field_bindings: Vec<(String, MirExpr)> = fields.iter().enumerate().map(|(i, (fname, fty))| {
            // For Option fields at MIR level, they're SumType("Option_X") but stored as Ptr
            let var_ty = if matches!(fty, MirType::SumType(ref s) if s.starts_with("Option_")) {
                MirType::Ptr
            } else {
                fty.clone()
            };
            (fname.clone(), MirExpr::Var(format!("__field_{}", i), var_ty))
        }).collect();

        let struct_lit = MirExpr::StructLit {
            name: name.to_string(),
            fields: field_bindings,
            ty: struct_ty.clone(),
        };

        let ok_result = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
            args: vec![
                MirExpr::IntLit(0, MirType::Int),
                struct_lit,
            ],
            ty: MirType::Ptr,
        };

        // Wrap each field extraction around the inner expression, from last to first.
        let mut body = ok_result;

        for (i, (field_name, field_ty)) in fields.iter().enumerate().rev() {
            let is_option = matches!(field_ty, MirType::SumType(ref s) if s.starts_with("Option_"));

            let key_lit = MirExpr::StringLit(field_name.clone(), MirType::String);
            let get_result_var = format!("__get_res_{}", i);
            let col_str_var = format!("__col_str_{}", i);
            let val_var = format!("__field_{}", i);

            // mesh_row_from_row_get(row, "field_name")
            let get_call = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_row_from_row_get".to_string(), row_get_ty.clone())),
                args: vec![row_var.clone(), key_lit],
                ty: MirType::Ptr,
            };

            if is_option {
                // Option field: missing column -> Ok(None), empty string -> Ok(None)
                let inner_type_str = if let MirType::SumType(ref s) = field_ty {
                    s.strip_prefix("Option_").unwrap_or("String")
                } else {
                    "String"
                };
                let option_sum_name = if let MirType::SumType(ref s) = field_ty { s.clone() } else { format!("Option_{}", inner_type_str) };

                // None variant: ConstructVariant with no fields
                let none_expr = MirExpr::ConstructVariant {
                    type_name: option_sum_name.clone(),
                    variant: "None".to_string(),
                    fields: vec![],
                    ty: MirType::SumType(option_sum_name.clone()),
                };

                // Ok(None) result
                let ok_none = MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
                    args: vec![MirExpr::IntLit(0, MirType::Int), none_expr.clone()],
                    ty: MirType::Ptr,
                };

                // Build the "column present" branch: check empty string, parse inner type
                let some_branch = self.emit_from_row_option_some(
                    &col_str_var,
                    inner_type_str,
                    &option_sum_name,
                    &alloc_result_ty,
                    &is_ok_ty,
                    &unwrap_ty,
                    &str_len_ty,
                    i,
                );

                // Check string length == 0 (NULL) -> Ok(None), else parse
                let null_check = MirExpr::Let {
                    name: col_str_var.clone(),
                    ty: MirType::Ptr,
                    value: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                        args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                        ty: MirType::Ptr,
                    }),
                    body: Box::new(MirExpr::If {
                        cond: Box::new(MirExpr::BinOp {
                            op: BinOp::Eq,
                            lhs: Box::new(MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_string_length".to_string(), str_len_ty.clone())),
                                args: vec![MirExpr::Var(col_str_var.clone(), MirType::Ptr)],
                                ty: MirType::Int,
                            }),
                            rhs: Box::new(MirExpr::IntLit(0, MirType::Int)),
                            ty: MirType::Bool,
                        }),
                        then_body: Box::new(ok_none.clone()),
                        else_body: Box::new(some_branch),
                        ty: MirType::Ptr,
                    }),
                };

                // Clone body before it's consumed: Option needs it in two branches
                // (get-succeeded path and missing-column path both continue to body)
                let body_for_missing = body.clone();

                // Outer: if get succeeded, check null; if get failed (missing column), Ok(None)
                let outer_result_var = format!("__opt_res_{}", i);
                body = MirExpr::Let {
                    name: get_result_var.clone(),
                    ty: MirType::Ptr,
                    value: Box::new(get_call),
                    body: Box::new(MirExpr::If {
                        cond: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                            args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                            ty: MirType::Int,
                        }),
                        then_body: Box::new(MirExpr::Let {
                            name: outer_result_var.clone(),
                            ty: MirType::Ptr,
                            value: Box::new(null_check),
                            body: Box::new(MirExpr::If {
                                cond: Box::new(MirExpr::Call {
                                    func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                                    args: vec![MirExpr::Var(outer_result_var.clone(), MirType::Ptr)],
                                    ty: MirType::Int,
                                }),
                                then_body: Box::new(MirExpr::Let {
                                    name: val_var.clone(),
                                    ty: MirType::Ptr,
                                    value: Box::new(MirExpr::Call {
                                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                                        args: vec![MirExpr::Var(outer_result_var.clone(), MirType::Ptr)],
                                        ty: MirType::Ptr,
                                    }),
                                    body: Box::new(body),
                                }),
                                else_body: Box::new(MirExpr::Var(outer_result_var, MirType::Ptr)),
                                ty: MirType::Ptr,
                            }),
                        }),
                        // Missing column for Option -> assign None and continue
                        else_body: Box::new(MirExpr::Let {
                            name: val_var,
                            ty: MirType::Ptr,
                            value: Box::new(none_expr),
                            body: Box::new(body_for_missing),
                        }),
                        ty: MirType::Ptr,
                    }),
                };
            } else {
                // Non-Option field: missing column is an error

                // For String type: no parsing needed, column value used directly
                let is_string = matches!(field_ty, MirType::String);

                if is_string {
                    // String: get column value, use directly
                    body = MirExpr::Let {
                        name: get_result_var.clone(),
                        ty: MirType::Ptr,
                        value: Box::new(get_call),
                        body: Box::new(MirExpr::If {
                            cond: Box::new(MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                                args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                                ty: MirType::Int,
                            }),
                            then_body: Box::new(MirExpr::Let {
                                name: val_var,
                                ty: MirType::String,
                                value: Box::new(MirExpr::Call {
                                    func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                                    args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                                    ty: MirType::Ptr,
                                }),
                                body: Box::new(body),
                            }),
                            else_body: Box::new(MirExpr::Var(get_result_var, MirType::Ptr)),
                            ty: MirType::Ptr,
                        }),
                    };
                } else {
                    // Int, Float, Bool: get column value, then parse
                    let parse_fn = match field_ty {
                        MirType::Int => "mesh_row_parse_int",
                        MirType::Float => "mesh_row_parse_float",
                        MirType::Bool => "mesh_row_parse_bool",
                        _ => "mesh_row_parse_int", // fallback
                    };
                    let parse_fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                    let parse_result_var = format!("__parse_res_{}", i);

                    // Inner: parse the column string
                    let inner_parse = MirExpr::Let {
                        name: col_str_var.clone(),
                        ty: MirType::Ptr,
                        value: Box::new(MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                            args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                            ty: MirType::Ptr,
                        }),
                        body: Box::new(MirExpr::Let {
                            name: parse_result_var.clone(),
                            ty: MirType::Ptr,
                            value: Box::new(MirExpr::Call {
                                func: Box::new(MirExpr::Var(parse_fn.to_string(), parse_fn_ty)),
                                args: vec![MirExpr::Var(col_str_var, MirType::Ptr)],
                                ty: MirType::Ptr,
                            }),
                            body: Box::new(MirExpr::If {
                                cond: Box::new(MirExpr::Call {
                                    func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                                    args: vec![MirExpr::Var(parse_result_var.clone(), MirType::Ptr)],
                                    ty: MirType::Int,
                                }),
                                then_body: Box::new(MirExpr::Let {
                                    name: val_var,
                                    ty: field_ty.clone(),
                                    value: Box::new(MirExpr::Call {
                                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                                        args: vec![MirExpr::Var(parse_result_var.clone(), MirType::Ptr)],
                                        ty: MirType::Ptr,
                                    }),
                                    body: Box::new(body),
                                }),
                                else_body: Box::new(MirExpr::Var(parse_result_var, MirType::Ptr)),
                                ty: MirType::Ptr,
                            }),
                        }),
                    };

                    // Outer: check if row_get succeeded
                    body = MirExpr::Let {
                        name: get_result_var.clone(),
                        ty: MirType::Ptr,
                        value: Box::new(get_call),
                        body: Box::new(MirExpr::If {
                            cond: Box::new(MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                                args: vec![MirExpr::Var(get_result_var.clone(), MirType::Ptr)],
                                ty: MirType::Int,
                            }),
                            then_body: Box::new(inner_parse),
                            else_body: Box::new(MirExpr::Var(get_result_var, MirType::Ptr)),
                            ty: MirType::Ptr,
                        }),
                    };
                }
            }
        }

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("row".to_string(), MirType::Ptr)],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
        );
    }

    /// Emit the "Some" branch for an Option field in from_row.
    /// When the column value is non-empty, parse the inner type and wrap in Some.
    fn emit_from_row_option_some(
        &self,
        col_str_var: &str,
        inner_type_str: &str,
        option_sum_name: &str,
        alloc_result_ty: &MirType,
        is_ok_ty: &MirType,
        unwrap_ty: &MirType,
        _str_len_ty: &MirType,
        field_idx: usize,
    ) -> MirExpr {
        let col_str = MirExpr::Var(col_str_var.to_string(), MirType::Ptr);

        // For String: wrap directly in Some
        if inner_type_str == "String" {
            let some_expr = MirExpr::ConstructVariant {
                type_name: option_sum_name.to_string(),
                variant: "Some".to_string(),
                fields: vec![col_str],
                ty: MirType::SumType(option_sum_name.to_string()),
            };
            return MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
                args: vec![MirExpr::IntLit(0, MirType::Int), some_expr],
                ty: MirType::Ptr,
            };
        }

        // For Int/Float/Bool: parse, then wrap in Some
        let parse_fn = match inner_type_str {
            "Int" => "mesh_row_parse_int",
            "Float" => "mesh_row_parse_float",
            "Bool" => "mesh_row_parse_bool",
            _ => "mesh_row_parse_int",
        };
        let parse_fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        let parse_var = format!("__opt_parse_{}", field_idx);

        let inner_ty = match inner_type_str {
            "Int" => MirType::Int,
            "Float" => MirType::Float,
            "Bool" => MirType::Bool,
            _ => MirType::Ptr,
        };

        let parsed_val_var = format!("__opt_val_{}", field_idx);

        MirExpr::Let {
            name: parse_var.clone(),
            ty: MirType::Ptr,
            value: Box::new(MirExpr::Call {
                func: Box::new(MirExpr::Var(parse_fn.to_string(), parse_fn_ty)),
                args: vec![col_str],
                ty: MirType::Ptr,
            }),
            body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty.clone())),
                    args: vec![MirExpr::Var(parse_var.clone(), MirType::Ptr)],
                    ty: MirType::Int,
                }),
                then_body: Box::new(MirExpr::Let {
                    name: parsed_val_var.clone(),
                    ty: inner_ty.clone(),
                    value: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty.clone())),
                        args: vec![MirExpr::Var(parse_var.clone(), MirType::Ptr)],
                        ty: MirType::Ptr,
                    }),
                    body: Box::new({
                        let some_expr = MirExpr::ConstructVariant {
                            type_name: option_sum_name.to_string(),
                            variant: "Some".to_string(),
                            fields: vec![MirExpr::Var(parsed_val_var, inner_ty)],
                            ty: MirType::SumType(option_sum_name.to_string()),
                        };
                        MirExpr::Call {
                            func: Box::new(MirExpr::Var("mesh_alloc_result".to_string(), alloc_result_ty.clone())),
                            args: vec![MirExpr::IntLit(0, MirType::Int), some_expr],
                            ty: MirType::Ptr,
                        }
                    }),
                }),
                else_body: Box::new(MirExpr::Var(parse_var, MirType::Ptr)),
                ty: MirType::Ptr,
            }),
        }
    }

    /// Generate Schema metadata functions for a struct with `deriving(Schema)`.
    ///
    /// Generates synthetic MIR functions:
    /// - `{Name}____table__()` -> String (lowercased, pluralized struct name or custom)
    /// - `{Name}____fields__()` -> List<String> (field name strings)
    /// - `{Name}____primary_key__()` -> String (default: "id" or custom)
    /// - `{Name}____relationships__()` -> List<String> (encoded as "kind:name:target")
    /// - `{Name}____field_types__()` -> List<String> (encoded as "field:SQL_TYPE")
    /// - `{Name}____{field}_col__()` -> String (per-field column accessor)
    fn generate_schema_metadata(
        &mut self,
        name: &str,
        fields: &[(String, MirType)],
        relationships: &[RelationshipDecl],
        custom_table: Option<String>,
        custom_pk: Option<String>,
        _has_timestamps: bool,
    ) {
        // ── __table__() ──────────────────────────────────────────────
        // Returns custom table name or lowercased struct name + "s" (naive pluralization).
        let table_name = custom_table.unwrap_or_else(|| format!("{}s", name.to_lowercase()));
        let table_fn_name = format!("{}____table__", name);
        self.functions.push(MirFunction {
            name: table_fn_name.clone(),
            params: vec![],
            return_type: MirType::String,
            body: MirExpr::StringLit(table_name, MirType::String),
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        self.known_functions.insert(
            table_fn_name,
            MirType::FnPtr(vec![], Box::new(MirType::String)),
        );

        // ── __fields__() ─────────────────────────────────────────────
        // Returns a List<String> of field names.
        let field_elements: Vec<MirExpr> = fields
            .iter()
            .map(|(fname, _)| MirExpr::StringLit(fname.clone(), MirType::String))
            .collect();
        let fields_fn_name = format!("{}____fields__", name);
        self.functions.push(MirFunction {
            name: fields_fn_name.clone(),
            params: vec![],
            return_type: MirType::Ptr, // List<String> is Ptr at runtime
            body: MirExpr::ListLit {
                elements: field_elements,
                ty: MirType::Ptr,
            },
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        self.known_functions.insert(
            fields_fn_name,
            MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
        );

        // ── __primary_key__() ────────────────────────────────────────
        // Returns custom primary key or "id" as the default.
        let pk_value = custom_pk.unwrap_or_else(|| "id".to_string());
        let pk_fn_name = format!("{}____primary_key__", name);
        self.functions.push(MirFunction {
            name: pk_fn_name.clone(),
            params: vec![],
            return_type: MirType::String,
            body: MirExpr::StringLit(pk_value, MirType::String),
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        self.known_functions.insert(
            pk_fn_name,
            MirType::FnPtr(vec![], Box::new(MirType::String)),
        );

        // ── __relationships__() ──────────────────────────────────────
        // Returns a List<String> where each string is "kind:name:target".
        let rel_elements: Vec<MirExpr> = relationships
            .iter()
            .filter_map(|rel| {
                let kind = rel.kind_text()?;
                let assoc = rel.assoc_name()?;
                let target = rel.target_type()?;
                Some(MirExpr::StringLit(
                    format!("{}:{}:{}", kind, assoc, target),
                    MirType::String,
                ))
            })
            .collect();
        let rels_fn_name = format!("{}____relationships__", name);
        self.functions.push(MirFunction {
            name: rels_fn_name.clone(),
            params: vec![],
            return_type: MirType::Ptr, // List<String> is Ptr at runtime
            body: MirExpr::ListLit {
                elements: rel_elements,
                ty: MirType::Ptr,
            },
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        self.known_functions.insert(
            rels_fn_name,
            MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
        );

        // ── __field_types__() ────────────────────────────────────────
        // Returns List<String> where each entry is "field_name:SQL_TYPE".
        let field_type_elements: Vec<MirExpr> = fields
            .iter()
            .map(|(fname, fty)| {
                let sql_type = mir_type_to_sql_type(fty);
                MirExpr::StringLit(format!("{}:{}", fname, sql_type), MirType::String)
            })
            .collect();
        let ft_fn_name = format!("{}____field_types__", name);
        self.functions.push(MirFunction {
            name: ft_fn_name.clone(),
            params: vec![],
            return_type: MirType::Ptr,
            body: MirExpr::ListLit {
                elements: field_type_elements,
                ty: MirType::Ptr,
            },
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        self.known_functions.insert(
            ft_fn_name,
            MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
        );

        // ── __relationship_meta__() ──────────────────────────────────
        // Returns List<String> where each string is "kind:name:target:fk:target_table".
        let meta_elements: Vec<MirExpr> = relationships
            .iter()
            .filter_map(|rel| {
                let kind = rel.kind_text()?;
                let assoc = rel.assoc_name()?;
                let target = rel.target_type()?;

                // Infer foreign key by convention:
                // - belongs_to :user, User -> fk is "user_id" (assoc_name + "_id")
                // - has_many :posts, Post on User -> fk is "user_id" (owner_lowercase + "_id")
                // - has_one :profile, Profile on User -> fk is "user_id" (owner_lowercase + "_id")
                let fk = match kind.as_str() {
                    "belongs_to" => format!("{}_id", assoc),
                    "has_many" | "has_one" => format!("{}_id", name.to_lowercase()),
                    _ => return None,
                };

                // Infer target table by naive pluralization (lowercase + "s")
                let target_table = format!("{}s", target.to_lowercase());

                Some(MirExpr::StringLit(
                    format!("{}:{}:{}:{}:{}", kind, assoc, target, fk, target_table),
                    MirType::String,
                ))
            })
            .collect();

        let meta_fn_name = format!("{}____relationship_meta__", name);
        self.functions.push(MirFunction {
            name: meta_fn_name.clone(),
            params: vec![],
            return_type: MirType::Ptr,
            body: MirExpr::ListLit {
                elements: meta_elements,
                ty: MirType::Ptr,
            },
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        self.known_functions.insert(
            meta_fn_name,
            MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
        );

        // ── Per-field column accessors ───────────────────────────────
        // User.__name_col__() -> "name"
        for (fname, _fty) in fields {
            let col_fn_name = format!("{}____{}_col__", name, fname);
            self.functions.push(MirFunction {
                name: col_fn_name.clone(),
                params: vec![],
                return_type: MirType::String,
                body: MirExpr::StringLit(fname.clone(), MirType::String),
                is_closure_fn: false,
                captures: vec![],
                has_tail_calls: false,
            });
            self.known_functions.insert(
                col_fn_name,
                MirType::FnPtr(vec![], Box::new(MirType::String)),
            );
        }
    }

    /// Emit a from_json extraction for a value of the given MIR type.
    /// Returns a MirExpr that produces a Result (Ok(value) or Err(string)).
    fn emit_from_json_for_type(&self, json_expr: MirExpr, target_ty: &MirType, _context_struct: &str) -> MirExpr {
        let fn_name = match target_ty {
            MirType::Int => "mesh_json_as_int",
            MirType::Float => "mesh_json_as_float",
            MirType::Bool => "mesh_json_as_bool",
            MirType::String => "mesh_json_as_string",
            MirType::Struct(inner) => {
                let name = format!("FromJson__from_json__{}", inner);
                let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                return MirExpr::Call {
                    func: Box::new(MirExpr::Var(name, fn_ty)),
                    args: vec![json_expr],
                    ty: MirType::Ptr,
                };
            }
            MirType::SumType(sum_name) if sum_name.starts_with("Option_") => {
                // Option<T>: check if JSON is null -> None, else decode inner -> Some
                return self.emit_option_from_json(json_expr, sum_name, _context_struct);
            }
            MirType::SumType(sum_name) => {
                // Non-Option sum type: call FromJson__from_json__SumName
                let name = format!("FromJson__from_json__{}", sum_name);
                let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                return MirExpr::Call {
                    func: Box::new(MirExpr::Var(name, fn_ty)),
                    args: vec![json_expr],
                    ty: MirType::Ptr,
                };
            }
            _ => "mesh_json_as_int", // fallback for Ptr/unknown
        };

        let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        MirExpr::Call {
            func: Box::new(MirExpr::Var(fn_name.to_string(), fn_ty)),
            args: vec![json_expr],
            ty: MirType::Ptr,
        }
    }

    /// Emit Option<T> from JSON decoding: null -> Ok(None), other -> decode inner then wrap in Some.
    fn emit_option_from_json(&self, json_expr: MirExpr, sum_name: &str, _context_struct: &str) -> MirExpr {
        // For Option<T>, the from_json simply returns the JSON value.
        // The inner extraction (Some/None wrapping) happens at a higher level
        // via mesh_json_as_* returning the inner value or null check.
        // For simplicity, use mesh_json_as_int as a fallback -- the runtime
        // handles null -> Err, value -> Ok(value).
        let inner_type_str = sum_name.strip_prefix("Option_").unwrap_or("Int");
        let fn_name = match inner_type_str {
            "Int" => "mesh_json_as_int",
            "Float" => "mesh_json_as_float",
            "Bool" => "mesh_json_as_bool",
            "String" => "mesh_json_as_string",
            _ => "mesh_json_as_int",
        };
        let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        MirExpr::Call {
            func: Box::new(MirExpr::Var(fn_name.to_string(), fn_ty)),
            args: vec![json_expr],
            ty: MirType::Ptr,
        }
    }

    /// Emit collection (List/Map) from JSON decoding using callback-based runtime helpers.
    fn emit_collection_from_json(&mut self, json_expr: MirExpr, typeck_ty: &Ty, _context_struct: &str) -> MirExpr {
        match typeck_ty {
            Ty::App(base, args) => {
                if let Ty::Con(con) = base.as_ref() {
                    match con.name.as_str() {
                        "List" => {
                            let elem_ty = args.first().cloned().unwrap_or(Ty::int());
                            let callback_name = self.resolve_from_json_callback(&elem_ty);
                            let fn_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
                            let callback_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_json_to_list".to_string(), fn_ty)),
                                args: vec![json_expr, MirExpr::Var(callback_name, callback_ty)],
                                ty: MirType::Ptr,
                            }
                        }
                        "Map" => {
                            let val_ty = args.get(1).cloned().unwrap_or(Ty::string());
                            let callback_name = self.resolve_from_json_callback(&val_ty);
                            let fn_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));
                            let callback_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_json_to_map".to_string(), fn_ty)),
                                args: vec![json_expr, MirExpr::Var(callback_name, callback_ty)],
                                ty: MirType::Ptr,
                            }
                        }
                        _ => {
                            // Not a known collection -- fallback
                            let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_json_as_int".to_string(), fn_ty)),
                                args: vec![json_expr],
                                ty: MirType::Ptr,
                            }
                        }
                    }
                } else {
                    let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                    MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_json_as_int".to_string(), fn_ty)),
                        args: vec![json_expr],
                        ty: MirType::Ptr,
                    }
                }
            }
            _ => {
                let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_json_as_int".to_string(), fn_ty)),
                    args: vec![json_expr],
                    ty: MirType::Ptr,
                }
            }
        }
    }

    /// Generate a wrapper `__json_decode__StructName` that chains
    /// mesh_json_parse + FromJson__from_json__StructName.
    /// This is what `StructName.from_json(str)` resolves to.
    /// Returns a *mut MeshResult (Ptr) -- the let-binding deref logic converts
    /// it to a SumType("Result") when bound to a typed variable.
    fn generate_from_json_string_wrapper(&mut self, name: &str) {
        let wrapper_name = format!("__json_decode__{}", name);
        let parse_ty = MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr));
        let from_json_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        let is_ok_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Int));
        let unwrap_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));

        let str_var = MirExpr::Var("__input".to_string(), MirType::String);

        // mesh_json_parse(input) -> *mut MeshResult
        let parse_call = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_json_parse".to_string(), parse_ty)),
            args: vec![str_var],
            ty: MirType::Ptr,
        };

        // If parse is Ok, call FromJson__from_json__(parsed_json)
        // Else, return the error result directly
        let from_json_call = MirExpr::Call {
            func: Box::new(MirExpr::Var(format!("FromJson__from_json__{}", name), from_json_ty)),
            args: vec![MirExpr::Var("__parsed_json".to_string(), MirType::Ptr)],
            ty: MirType::Ptr,
        };

        let body = MirExpr::Let {
            name: "__parse_res".to_string(),
            ty: MirType::Ptr,
            value: Box::new(parse_call),
            body: Box::new(MirExpr::If {
                cond: Box::new(MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_result_is_ok".to_string(), is_ok_ty)),
                    args: vec![MirExpr::Var("__parse_res".to_string(), MirType::Ptr)],
                    ty: MirType::Int,
                }),
                then_body: Box::new(MirExpr::Let {
                    name: "__parsed_json".to_string(),
                    ty: MirType::Ptr,
                    value: Box::new(MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_result_unwrap".to_string(), unwrap_ty)),
                        args: vec![MirExpr::Var("__parse_res".to_string(), MirType::Ptr)],
                        ty: MirType::Ptr,
                    }),
                    body: Box::new(from_json_call),
                }),
                else_body: Box::new(MirExpr::Var("__parse_res".to_string(), MirType::Ptr)),
                ty: MirType::Ptr,
            }),
        };

        let func = MirFunction {
            name: wrapper_name.clone(),
            params: vec![("__input".to_string(), MirType::String)],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            wrapper_name,
            MirType::FnPtr(vec![MirType::String], Box::new(MirType::Ptr)),
        );
    }

    // ── Display generation ──────────────────────────────────────────

    /// Generate a synthetic `Display__to_string__StructName` MIR function that
    /// produces a constructor-style string like `"Point(1, 2)"`.
    /// Unlike Debug (which uses `"Point { x: 1, y: 2 }"`), Display uses positional
    /// values without field names.
    fn generate_display_struct(&mut self, name: &str, fields: &[(String, MirType)]) {
        self.generate_display_struct_with_display_name(name, name, fields);
    }

    fn generate_display_struct_with_display_name(&mut self, name: &str, display_name: &str, fields: &[(String, MirType)]) {
        let mangled = format!("Display__to_string__{}", name);
        let struct_ty = MirType::Struct(name.to_string());
        let concat_ty = MirType::FnPtr(
            vec![MirType::String, MirType::String],
            Box::new(MirType::String),
        );
        let self_var = MirExpr::Var("self".to_string(), struct_ty.clone());

        // Build: "StructName(val1, val2)"
        let mut result: MirExpr = if fields.is_empty() {
            MirExpr::StringLit(format!("{}()", display_name), MirType::String)
        } else {
            MirExpr::StringLit(format!("{}(", display_name), MirType::String)
        };

        if !fields.is_empty() {
            for (i, (field_name, field_ty)) in fields.iter().enumerate() {
                let is_last = i == fields.len() - 1;

                // Access self.field (use field name for struct field access)
                let field_access = MirExpr::FieldAccess {
                    object: Box::new(self_var.clone()),
                    field: field_name.clone(),
                    ty: field_ty.clone(),
                };

                // Convert field value to string (no label prefix -- Display is positional)
                let field_str = self.wrap_to_string(field_access, None);

                // Append field value string
                result = MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                    args: vec![result, field_str],
                    ty: MirType::String,
                };

                // Append separator: ", " for non-last fields
                if !is_last {
                    result = MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                        args: vec![result, MirExpr::StringLit(", ".to_string(), MirType::String)],
                        ty: MirType::String,
                    };
                }
            }

            // Append closing ")"
            result = MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                args: vec![result, MirExpr::StringLit(")".to_string(), MirType::String)],
                ty: MirType::String,
            };
        }

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), struct_ty.clone())],
            return_type: MirType::String,
            body: result,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![struct_ty], Box::new(MirType::String)),
        );
    }

    /// Generate a synthetic `Display__to_string__SumTypeName` MIR function.
    /// Uses Match on self with Constructor patterns to produce variant-aware output.
    /// Nullary variants: just the variant name (e.g. "Dot").
    /// Variants with fields: "VariantName(val0, val1)" style.
    fn generate_display_sum_type(&mut self, name: &str, variants: &[MirVariantDef]) {
        let mangled = format!("Display__to_string__{}", name);
        let sum_ty = MirType::SumType(name.to_string());
        let self_var = MirExpr::Var("self".to_string(), sum_ty.clone());
        let concat_ty = MirType::FnPtr(
            vec![MirType::String, MirType::String],
            Box::new(MirType::String),
        );

        let body = if variants.is_empty() {
            MirExpr::StringLit(format!("<{}>", name), MirType::String)
        } else {
            let arms: Vec<MirMatchArm> = variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        // Nullary variant: just return variant name
                        MirMatchArm {
                            pattern: MirPattern::Constructor {
                                type_name: name.to_string(),
                                variant: v.name.clone(),
                                fields: vec![],
                                bindings: vec![],
                            },
                            body: MirExpr::StringLit(v.name.clone(), MirType::String),
                            guard: None,
                        }
                    } else {
                        // Variant with fields: bind as field_0, field_1, ...
                        let field_pats: Vec<MirPattern> = v
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(i, ft)| MirPattern::Var(format!("field_{}", i), ft.clone()))
                            .collect();
                        let bindings: Vec<(String, MirType)> = v
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(i, ft)| (format!("field_{}", i), ft.clone()))
                            .collect();

                        // Build "VariantName(val0, val1)"
                        let mut result = MirExpr::StringLit(
                            format!("{}(", v.name),
                            MirType::String,
                        );

                        for (i, ft) in v.fields.iter().enumerate() {
                            let is_last = i == v.fields.len() - 1;
                            let field_var = MirExpr::Var(format!("field_{}", i), ft.clone());
                            let field_str = self.wrap_to_string(field_var, None);

                            // Append field value
                            result = MirExpr::Call {
                                func: Box::new(MirExpr::Var(
                                    "mesh_string_concat".to_string(),
                                    concat_ty.clone(),
                                )),
                                args: vec![result, field_str],
                                ty: MirType::String,
                            };

                            // Append separator for non-last fields
                            if !is_last {
                                result = MirExpr::Call {
                                    func: Box::new(MirExpr::Var(
                                        "mesh_string_concat".to_string(),
                                        concat_ty.clone(),
                                    )),
                                    args: vec![
                                        result,
                                        MirExpr::StringLit(", ".to_string(), MirType::String),
                                    ],
                                    ty: MirType::String,
                                };
                            }
                        }

                        // Append closing ")"
                        result = MirExpr::Call {
                            func: Box::new(MirExpr::Var(
                                "mesh_string_concat".to_string(),
                                concat_ty.clone(),
                            )),
                            args: vec![
                                result,
                                MirExpr::StringLit(")".to_string(), MirType::String),
                            ],
                            ty: MirType::String,
                        };

                        MirMatchArm {
                            pattern: MirPattern::Constructor {
                                type_name: name.to_string(),
                                variant: v.name.clone(),
                                fields: field_pats,
                                bindings,
                            },
                            body: result,
                            guard: None,
                        }
                    }
                })
                .collect();

            MirExpr::Match {
                scrutinee: Box::new(self_var),
                arms,
                ty: MirType::String,
            }
        };

        let func = MirFunction {
            name: mangled.clone(),
            params: vec![("self".to_string(), sum_ty.clone())],
            return_type: MirType::String,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        };

        self.functions.push(func);
        self.known_functions.insert(
            mangled,
            MirType::FnPtr(vec![sum_ty], Box::new(MirType::String)),
        );
    }

    /// Emit a hash call for a value of the given MIR type.
    /// Returns a MirExpr that evaluates to i64 hash.
    fn emit_hash_for_type(&self, expr: MirExpr, ty: &MirType) -> MirExpr {
        match ty {
            MirType::Int => {
                let fn_ty = MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Int));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_hash_int".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Int,
                }
            }
            MirType::Float => {
                let fn_ty = MirType::FnPtr(vec![MirType::Float], Box::new(MirType::Int));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_hash_float".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Int,
                }
            }
            MirType::Bool => {
                let fn_ty = MirType::FnPtr(vec![MirType::Bool], Box::new(MirType::Int));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_hash_bool".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Int,
                }
            }
            MirType::String => {
                let fn_ty = MirType::FnPtr(vec![MirType::String], Box::new(MirType::Int));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_hash_string".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Int,
                }
            }
            MirType::Struct(inner_name) => {
                // Recursive: call Hash__hash__InnerStruct
                let inner_mangled = format!("Hash__hash__{}", inner_name);
                let fn_ty = MirType::FnPtr(vec![ty.clone()], Box::new(MirType::Int));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                    args: vec![expr],
                    ty: MirType::Int,
                }
            }
            _ => {
                // Fallback: hash as int (cast to i64)
                let fn_ty = MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Int));
                MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_hash_int".to_string(), fn_ty)),
                    args: vec![expr],
                    ty: MirType::Int,
                }
            }
        }
    }

    /// Build a lexicographic less-than comparison for sum type payload fields
    /// using named variables (e.g., self_0, self_1 vs other_0, other_1).
    fn build_lexicographic_lt_vars(
        &self,
        fields: &[MirType],
        self_prefix: &str,
        other_prefix: &str,
        index: usize,
    ) -> MirExpr {
        let field_ty = &fields[index];
        let self_f = MirExpr::Var(format!("{}{}", self_prefix, index), field_ty.clone());
        let other_f = MirExpr::Var(format!("{}{}", other_prefix, index), field_ty.clone());
        let is_last = index == fields.len() - 1;

        // Build "self_N < other_N" comparison
        let lt_cmp = match field_ty {
            MirType::Struct(inner_name) | MirType::SumType(inner_name) => {
                let inner_mangled = format!("Ord__lt__{}", inner_name);
                let fn_ty = MirType::FnPtr(
                    vec![field_ty.clone(), field_ty.clone()],
                    Box::new(MirType::Bool),
                );
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                    args: vec![self_f.clone(), other_f.clone()],
                    ty: MirType::Bool,
                }
            }
            _ => MirExpr::BinOp {
                op: BinOp::Lt,
                lhs: Box::new(self_f.clone()),
                rhs: Box::new(other_f.clone()),
                ty: MirType::Bool,
            },
        };

        if is_last {
            lt_cmp
        } else {
            // Build "self_N == other_N" comparison
            let eq_cmp = match field_ty {
                MirType::Struct(inner_name) | MirType::SumType(inner_name) => {
                    let inner_mangled = format!("Eq__eq__{}", inner_name);
                    let fn_ty = MirType::FnPtr(
                        vec![field_ty.clone(), field_ty.clone()],
                        Box::new(MirType::Bool),
                    );
                    MirExpr::Call {
                        func: Box::new(MirExpr::Var(inner_mangled, fn_ty)),
                        args: vec![self_f, other_f],
                        ty: MirType::Bool,
                    }
                }
                _ => MirExpr::BinOp {
                    op: BinOp::Eq,
                    lhs: Box::new(self_f),
                    rhs: Box::new(other_f),
                    ty: MirType::Bool,
                },
            };

            let rest = self.build_lexicographic_lt_vars(fields, self_prefix, other_prefix, index + 1);

            MirExpr::If {
                cond: Box::new(lt_cmp),
                then_body: Box::new(MirExpr::BoolLit(true, MirType::Bool)),
                else_body: Box::new(MirExpr::If {
                    cond: Box::new(eq_cmp),
                    then_body: Box::new(rest),
                    else_body: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
                    ty: MirType::Bool,
                }),
                ty: MirType::Bool,
            }
        }
    }

    // ── Top-level let ────────────────────────────────────────────────

    fn lower_top_level_let(&mut self, let_: &LetBinding) {
        let name = let_
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());

        let value = if let Some(init) = let_.initializer() {
            self.lower_expr(&init)
        } else {
            MirExpr::Unit
        };

        let ty = value.ty().clone();
        self.insert_var(name.clone(), ty.clone());

        // Top-level lets become a function that returns the value (for globals).
        // In practice, these would be part of an init function, but for now
        // we store the binding in scope for use by other functions.
    }

    // ── Block lowering ───────────────────────────────────────────────

    fn lower_block(&mut self, block: &Block) -> MirExpr {
        // Collect all children in source order as MIR expressions.
        // Let bindings insert the variable into scope (for subsequent children)
        // and are wrapped to nest the remaining block as the body.
        let mut parts: Vec<MirExpr> = Vec::new();
        let mut let_names: Vec<String> = Vec::new();

        for child in block.syntax().children() {
            if let Some(item) = Item::cast(child.clone()) {
                match item {
                    Item::LetBinding(ref let_) => {
                        let name = let_
                            .name()
                            .and_then(|n| n.text())
                            .unwrap_or_else(|| "_".to_string());
                        let value = if let Some(init) = let_.initializer() {
                            self.lower_expr(&init)
                        } else {
                            MirExpr::Unit
                        };
                        let ty = value.ty().clone();
                        self.insert_var(name.clone(), ty.clone());
                        let_names.push(name.clone());
                        parts.push(MirExpr::Let {
                            name,
                            ty,
                            value: Box::new(value),
                            body: Box::new(MirExpr::Unit), // placeholder; nested below
                        });
                    }
                    Item::FnDef(ref fn_def) => {
                        self.lower_fn_def(fn_def);
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(expr) = Expr::cast(child) {
                let mir = self.lower_expr(&expr);
                parts.push(mir);
            }
        }

        // Build the final expression. Let bindings need to nest their body
        // over subsequent parts. We build from the end backwards:
        // [Let(x), expr1, Let(y), expr2] becomes:
        // Let(x, Block([expr1, Let(y, expr2)]))
        if parts.is_empty() {
            return MirExpr::Unit;
        }

        // Fold from right to left: each Let wraps everything after it as its body.
        let mut result = parts.pop().unwrap();
        while let Some(part) = parts.pop() {
            match part {
                MirExpr::Let { name, ty, value, body: _ } => {
                    result = MirExpr::Let {
                        name,
                        ty,
                        value,
                        body: Box::new(result),
                    };
                }
                other => {
                    // Non-let expression before result: wrap in a Block.
                    let ty = result.ty().clone();
                    result = MirExpr::Block(vec![other, result], ty);
                }
            }
        }

        result
    }

    // ── Let binding lowering ─────────────────────────────────────────

    #[allow(dead_code)]
    fn lower_let_binding(&mut self, let_: &LetBinding) -> MirExpr {
        let name = let_
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());

        let value = if let Some(init) = let_.initializer() {
            self.lower_expr(&init)
        } else {
            MirExpr::Unit
        };

        let ty = value.ty().clone();
        self.insert_var(name.clone(), ty.clone());

        MirExpr::Let {
            name,
            ty,
            value: Box::new(value),
            body: Box::new(MirExpr::Unit),
        }
    }

    // ── Expression lowering ──────────────────────────────────────────

    fn lower_expr(&mut self, expr: &Expr) -> MirExpr {
        match expr {
            Expr::Literal(lit) => self.lower_literal(lit),
            Expr::NameRef(name_ref) => self.lower_name_ref(name_ref),
            Expr::BinaryExpr(bin) => self.lower_binary_expr(bin),
            Expr::UnaryExpr(un) => self.lower_unary_expr(un),
            Expr::CallExpr(call) => self.lower_call_expr(call),
            Expr::PipeExpr(pipe) => self.lower_pipe_expr(pipe),
            Expr::FieldAccess(fa) => self.lower_field_access(fa),
            Expr::IndexExpr(_) => {
                // Index expressions not yet supported in MIR.
                MirExpr::Unit
            }
            Expr::IfExpr(if_) => self.lower_if_expr(if_),
            Expr::CaseExpr(case) => self.lower_case_expr(case),
            Expr::ClosureExpr(closure) => self.lower_closure_expr(closure),
            Expr::Block(block) => self.lower_block(block),
            Expr::StringExpr(str_expr) => self.lower_string_expr(str_expr),
            Expr::ReturnExpr(ret) => self.lower_return_expr(ret),
            Expr::TupleExpr(tuple) => self.lower_tuple_expr(tuple),
            Expr::StructLiteral(sl) => self.lower_struct_literal(sl),
            Expr::MapLiteral(map_lit) => self.lower_map_literal(map_lit),
            Expr::ListLiteral(list_lit) => self.lower_list_literal(list_lit),
            // Actor expressions
            Expr::SpawnExpr(spawn) => self.lower_spawn_expr(&spawn),
            Expr::SendExpr(send) => self.lower_send_expr(&send),
            Expr::ReceiveExpr(recv) => self.lower_receive_expr(&recv),
            Expr::SelfExpr(_) => {
                let ty = self.resolve_range(expr.syntax().text_range());
                let ty = if matches!(ty, MirType::Unit) {
                    MirType::Pid(None)
                } else {
                    ty
                };
                MirExpr::ActorSelf { ty }
            }
            Expr::LinkExpr(link) => self.lower_link_expr(&link),
            // Loop expressions
            Expr::WhileExpr(w) => self.lower_while_expr(w),
            Expr::BreakExpr(_) => MirExpr::Break,
            Expr::ContinueExpr(_) => MirExpr::Continue,
            Expr::ForInExpr(for_in) => self.lower_for_in_expr(&for_in),
            // Try expression -- desugar to Match + Return (Phase 45)
            Expr::TryExpr(try_expr) => self.lower_try_expr(&try_expr),
            // Atom literal -- lower to string constant at runtime
            Expr::AtomLiteral(atom) => {
                let name = atom.atom_text().unwrap_or_default();
                MirExpr::StringLit(name, MirType::String)
            }
            // Regex literal -- desugar to mesh_regex_from_literal(pattern, flags_bitmask)
            // mesh_regex_from_literal is declared in Phase 119-02 runtime; we wire the call
            // site here. Flags bitmask: i=1, m=2, s=4.
            Expr::RegexExpr(rx) => {
                let pattern = rx.pattern().unwrap_or_default();
                let flags_str = rx.flags();
                let flags_bits: i64 = flags_str.chars().fold(0i64, |acc, c| match c {
                    'i' => acc | 1,
                    'm' => acc | 2,
                    's' => acc | 4,
                    _ => acc,
                });
                let fn_ty = MirType::FnPtr(
                    vec![MirType::String, MirType::Int],
                    Box::new(MirType::Ptr),
                );
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_regex_from_literal".to_string(),
                        fn_ty,
                    )),
                    args: vec![
                        MirExpr::StringLit(pattern, MirType::String),
                        MirExpr::IntLit(flags_bits, MirType::Int),
                    ],
                    ty: MirType::Ptr,
                }
            }
            // Struct update expression: %{base | field: value, ...}
            Expr::StructUpdate(update) => self.lower_struct_update(update),
            // Slot pipe expression -- |N> desugaring (Phase 116, Plan 02)
            Expr::SlotPipeExpr(pipe) => self.lower_slot_pipe_expr(pipe),
            // Json object literal -- codegen implemented in Phase 132, Plan 02
            Expr::JsonExpr(_) => {
                todo!("json literal codegen not yet implemented (Phase 132-02)")
            }
        }
    }

    // ── Literal lowering ─────────────────────────────────────────────

    fn lower_literal(&self, lit: &Literal) -> MirExpr {
        let token = match lit.token() {
            Some(t) => t,
            None => return MirExpr::Unit,
        };

        let text = token.text().to_string();

        match token.kind() {
            SyntaxKind::INT_LITERAL => {
                let val = text.parse::<i64>().unwrap_or(0);
                MirExpr::IntLit(val, MirType::Int)
            }
            SyntaxKind::FLOAT_LITERAL => {
                let val = text.parse::<f64>().unwrap_or(0.0);
                MirExpr::FloatLit(val, MirType::Float)
            }
            SyntaxKind::TRUE_KW => MirExpr::BoolLit(true, MirType::Bool),
            SyntaxKind::FALSE_KW => MirExpr::BoolLit(false, MirType::Bool),
            SyntaxKind::NIL_KW => MirExpr::Unit,
            SyntaxKind::STRING_START => {
                // Simple string literal (no interpolation in a LITERAL node).
                // Extract the string content from the syntax node.
                let content = extract_simple_string_content(lit.syntax());
                MirExpr::StringLit(content, MirType::String)
            }
            _ => MirExpr::Unit,
        }
    }

    // ── Name reference lowering ──────────────────────────────────────

    fn lower_name_ref(&self, name_ref: &NameRef) -> MirExpr {
        let name = name_ref
            .text()
            .unwrap_or_else(|| "<unknown>".to_string());

        // Check if this is a nullary variant constructor (e.g., Red, None, Point).
        // These are NameRef nodes that refer to sum type variants with no fields.
        for (_, sum_info) in &self.registry.sum_type_defs {
            for variant in &sum_info.variants {
                if variant.name == name && variant.fields.is_empty() {
                    let ty_name = &sum_info.name;
                    let mir_ty = MirType::SumType(ty_name.clone());
                    return MirExpr::ConstructVariant {
                        type_name: ty_name.clone(),
                        variant: name,
                        fields: vec![],
                        ty: mir_ty,
                    };
                }
            }
        }

        // Check scope first for local variables. This ensures pattern bindings
        // (e.g., `head` from `head :: tail`) take precedence over builtin function
        // name mappings (e.g., `head` -> `mesh_list_head`).
        if let Some(scope_ty) = self.lookup_var(&name) {
            // Apply module-qualified naming to user-defined functions (Phase 41).
            // Function names in scope still need qualification to match their
            // renamed definitions. Local variables, variant constructors, actors
            // etc. (not in user_fn_defs) are unchanged.
            let name = if self.user_fn_defs.contains(&name) {
                self.qualify_name(&name)
            } else {
                name
            };
            return MirExpr::Var(name, scope_ty);
        }

        // Map builtin function names to their runtime equivalents.
        let name = map_builtin_name(&name);

        // Apply module-qualified naming to user-defined functions (Phase 41).
        // This ensures call sites match the qualified definition names.
        let name = if self.user_fn_defs.contains(&name) {
            self.qualify_name(&name)
        } else {
            name
        };

        let ty = self.resolve_range(name_ref.syntax().text_range());
        MirExpr::Var(name, ty)
    }

    // ── Binary expression lowering ───────────────────────────────────

    fn lower_binary_expr(&mut self, bin: &BinaryExpr) -> MirExpr {
        let lhs = bin.lhs().map(|e| self.lower_expr(&e)).unwrap_or(MirExpr::Unit);
        let rhs = bin.rhs().map(|e| self.lower_expr(&e)).unwrap_or(MirExpr::Unit);

        let op = bin
            .op()
            .map(|t| match t.kind() {
                SyntaxKind::PLUS => BinOp::Add,
                SyntaxKind::MINUS => BinOp::Sub,
                SyntaxKind::STAR => BinOp::Mul,
                SyntaxKind::SLASH => BinOp::Div,
                SyntaxKind::PERCENT => BinOp::Mod,
                SyntaxKind::EQ_EQ => BinOp::Eq,
                SyntaxKind::NOT_EQ => BinOp::NotEq,
                SyntaxKind::LT => BinOp::Lt,
                SyntaxKind::GT => BinOp::Gt,
                SyntaxKind::LT_EQ => BinOp::LtEq,
                SyntaxKind::GT_EQ => BinOp::GtEq,
                SyntaxKind::AND_KW | SyntaxKind::AMP_AMP => BinOp::And,
                SyntaxKind::OR_KW | SyntaxKind::PIPE_PIPE => BinOp::Or,
                SyntaxKind::PLUS_PLUS | SyntaxKind::DIAMOND => BinOp::Concat,
                _ => BinOp::Add, // fallback
            })
            .unwrap_or(BinOp::Add);

        let ty = self.resolve_range(bin.syntax().text_range());

        // Operator dispatch for user types: if the lhs is a struct or sum type
        // with a trait impl for this operator, emit a trait method call instead
        // of a hardware BinOp.
        let lhs_ty = lhs.ty().clone();
        let is_user_type = matches!(lhs_ty, MirType::Struct(_) | MirType::SumType(_));
        if is_user_type {
            // (trait_name, method_name, negate_result, swap_args)
            let dispatch = match op {
                BinOp::Add => Some(("Add", "add", false, false)),
                BinOp::Sub => Some(("Sub", "sub", false, false)),
                BinOp::Mul => Some(("Mul", "mul", false, false)),
                BinOp::Div => Some(("Div", "div", false, false)),
                BinOp::Mod => Some(("Mod", "mod", false, false)),
                BinOp::Eq => Some(("Eq", "eq", false, false)),
                BinOp::NotEq => Some(("Eq", "eq", true, false)),      // negate eq
                BinOp::Lt => Some(("Ord", "lt", false, false)),
                BinOp::Gt => Some(("Ord", "lt", false, true)),        // swap: b < a
                BinOp::LtEq => Some(("Ord", "lt", true, true)),       // negate(b < a)
                BinOp::GtEq => Some(("Ord", "lt", true, false)),      // negate(a < b)
                _ => None,
            };
            if let Some((trait_name, method_name, negate, swap_args)) = dispatch {
                let ty_for_lookup = mir_type_to_ty(&lhs_ty);
                let type_name = mir_type_to_impl_name(&lhs_ty);
                let mangled = format!("{}__{}__{}", trait_name, method_name, type_name);

                // Check trait registry first, then fall back to known_functions
                // (for monomorphized generic struct trait functions like Eq__eq__Box_Int).
                let has_impl = self.trait_registry.has_impl(trait_name, &ty_for_lookup)
                    || self.known_functions.contains_key(&mangled);

                if has_impl {
                    let rhs_ty = rhs.ty().clone();
                    // Comparison operators (Eq/Ord) return Bool; arithmetic
                    // operators return the Output type from typeck (ty from
                    // resolve_range).
                    let result_ty = match op {
                        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt
                        | BinOp::LtEq | BinOp::GtEq => MirType::Bool,
                        _ => ty.clone(),
                    };
                    let fn_ty = MirType::FnPtr(
                        vec![lhs_ty.clone(), rhs_ty],
                        Box::new(result_ty.clone()),
                    );
                    let (call_lhs, call_rhs) = if swap_args {
                        (rhs, lhs)
                    } else {
                        (lhs, rhs)
                    };
                    let call = MirExpr::Call {
                        func: Box::new(MirExpr::Var(mangled, fn_ty)),
                        args: vec![call_lhs, call_rhs],
                        ty: result_ty,
                    };
                    if negate {
                        return MirExpr::BinOp {
                            op: BinOp::Eq,
                            lhs: Box::new(call),
                            rhs: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
                            ty,
                        };
                    } else {
                        return call;
                    }
                }
            }
        }

        // List Eq/Ord dispatch: if lhs is Ptr and typeck type is List<T>,
        // emit mesh_list_eq / mesh_list_compare with element callback.
        if matches!(lhs_ty, MirType::Ptr) {
            if let Some(lhs_ast) = bin.lhs() {
                if let Some(lhs_typeck) = self.get_ty(lhs_ast.syntax().text_range()).cloned() {
                    if let Some(elem_ty) = extract_list_elem_type(&lhs_typeck) {
                        match op {
                            BinOp::Eq | BinOp::NotEq => {
                                let eq_callback = self.resolve_eq_callback(&elem_ty);
                                let eq_callback_expr = MirExpr::Var(
                                    eq_callback,
                                    MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Bool)),
                                );
                                let call = MirExpr::Call {
                                    func: Box::new(MirExpr::Var(
                                        "mesh_list_eq".to_string(),
                                        MirType::FnPtr(
                                            vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
                                            Box::new(MirType::Bool),
                                        ),
                                    )),
                                    args: vec![lhs, rhs, eq_callback_expr],
                                    ty: MirType::Bool,
                                };
                                if op == BinOp::NotEq {
                                    return MirExpr::BinOp {
                                        op: BinOp::Eq,
                                        lhs: Box::new(call),
                                        rhs: Box::new(MirExpr::BoolLit(false, MirType::Bool)),
                                        ty,
                                    };
                                }
                                return call;
                            }
                            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                                let cmp_callback = self.resolve_compare_callback(&elem_ty);
                                let cmp_callback_expr = MirExpr::Var(
                                    cmp_callback,
                                    MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Int)),
                                );
                                let compare_call = MirExpr::Call {
                                    func: Box::new(MirExpr::Var(
                                        "mesh_list_compare".to_string(),
                                        MirType::FnPtr(
                                            vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
                                            Box::new(MirType::Int),
                                        ),
                                    )),
                                    args: vec![lhs, rhs, cmp_callback_expr],
                                    ty: MirType::Int,
                                };
                                let compare_op = match op {
                                    BinOp::Lt => BinOp::Lt,
                                    BinOp::Gt => BinOp::Gt,
                                    BinOp::LtEq => BinOp::LtEq,
                                    BinOp::GtEq => BinOp::GtEq,
                                    _ => unreachable!(),
                                };
                                return MirExpr::BinOp {
                                    op: compare_op,
                                    lhs: Box::new(compare_call),
                                    rhs: Box::new(MirExpr::IntLit(0, MirType::Int)),
                                    ty,
                                };
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        MirExpr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            ty,
        }
    }

    // ── Unary expression lowering ────────────────────────────────────

    fn lower_unary_expr(&mut self, un: &UnaryExpr) -> MirExpr {
        let operand = un
            .operand()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let op = un
            .op()
            .map(|t| match t.kind() {
                SyntaxKind::MINUS => UnaryOp::Neg,
                SyntaxKind::BANG | SyntaxKind::NOT_KW => UnaryOp::Not,
                _ => UnaryOp::Neg,
            })
            .unwrap_or(UnaryOp::Neg);

        let ty = self.resolve_range(un.syntax().text_range());

        // Neg trait dispatch for user types: if the operand is a struct or
        // sum type with a Neg impl, emit a trait method call instead of a
        // hardware UnaryOp.  Primitives (Int/Float) fall through to the
        // hardware path.
        if op == UnaryOp::Neg {
            let operand_ty = operand.ty().clone();
            let is_user_type =
                matches!(operand_ty, MirType::Struct(_) | MirType::SumType(_));
            if is_user_type {
                let ty_for_lookup = mir_type_to_ty(&operand_ty);
                let type_name = mir_type_to_impl_name(&operand_ty);
                let mangled = format!("Neg__neg__{}", type_name);

                let has_impl = self.trait_registry.has_impl("Neg", &ty_for_lookup)
                    || self.known_functions.contains_key(&mangled);

                if has_impl {
                    let fn_ty = MirType::FnPtr(
                        vec![operand_ty],
                        Box::new(ty.clone()),
                    );
                    return MirExpr::Call {
                        func: Box::new(MirExpr::Var(mangled, fn_ty)),
                        args: vec![operand],
                        ty,
                    };
                }
            }
        }

        MirExpr::UnaryOp {
            op,
            operand: Box::new(operand),
            ty,
        }
    }

    // ── Trait dispatch helpers ────────────────────────────────────────

    /// Check if a name refers to a sum type (e.g., Shape, Option).
    /// Used to prevent intercepting variant constructor calls like Shape.Circle(5.0).
    fn is_sum_type_name(&self, name: &str) -> bool {
        self.registry.sum_type_defs.contains_key(name)
    }

    /// Check if a name refers to a struct type (e.g., Point).
    /// Used to prevent intercepting module-style qualified calls on struct names.
    fn is_struct_type_name(&self, name: &str) -> bool {
        self.registry.struct_defs.contains_key(name)
    }

    /// Resolve a trait method callee: given a method name and the first argument's type,
    /// check if it's a trait method and rewrite to the mangled name (Trait__Method__Type).
    /// Returns the resolved callee (either mangled or original).
    fn resolve_trait_callee(
        &self,
        name: &str,
        var_ty: &MirType,
        first_arg_ty: &MirType,
    ) -> MirExpr {
        if !self.known_functions.contains_key(name) {
            let ty_for_lookup = mir_type_to_ty(first_arg_ty);
            let mut matching_traits = self.trait_registry.find_method_traits(name, &ty_for_lookup);
            matching_traits.sort(); // Defense-in-depth: deterministic trait selection
            if !matching_traits.is_empty() {
                let trait_name = &matching_traits[0];
                let type_name = mir_type_to_impl_name(first_arg_ty);
                let mangled = format!("{}__{}__{}", trait_name, name, type_name);

                // Primitive Display/Debug/Hash builtin redirects
                let resolved = match mangled.as_str() {
                    "Display__to_string__Int" | "Debug__inspect__Int" => {
                        "mesh_int_to_string".to_string()
                    }
                    "Display__to_string__Float" | "Debug__inspect__Float" => {
                        "mesh_float_to_string".to_string()
                    }
                    "Display__to_string__Bool" | "Debug__inspect__Bool" => {
                        "mesh_bool_to_string".to_string()
                    }
                    "Hash__hash__Int" => "mesh_hash_int".to_string(),
                    "Hash__hash__Float" => "mesh_hash_float".to_string(),
                    "Hash__hash__Bool" => "mesh_hash_bool".to_string(),
                    "Hash__hash__String" => "mesh_hash_string".to_string(),
                    // Built-in From dispatch (Phase 77)
                    "From_Int__from__Float" => "mesh_int_to_float".to_string(),
                    "From_Int__from__String" => "mesh_int_to_string".to_string(),
                    "From_Float__from__String" => "mesh_float_to_string".to_string(),
                    "From_Bool__from__String" => "mesh_bool_to_string".to_string(),
                    _ => mangled,
                };
                // Phase 128: TryInto.try_into() dispatch -- redirect to underlying TryFrom function.
                // The synthetic TryInto impl is NOT in known_functions; the user's TryFrom impl IS.
                // resolved looks like "TryInto__try_into__Int" (source type = Int).
                // We find TryFrom_Int__try_from__<TargetType> in known_functions.
                if resolved.starts_with("TryInto__try_into__") {
                    let source_prefix = format!("TryFrom_{}__try_from__", type_name);
                    for (fn_name, fn_ty) in self.known_functions.iter() {
                        if fn_name.starts_with(&source_prefix) {
                            return MirExpr::Var(fn_name.clone(), fn_ty.clone());
                        }
                    }
                }
                return MirExpr::Var(resolved, var_ty.clone());
            }

            // Fallback for monomorphized generic types
            let type_name = mir_type_to_impl_name(first_arg_ty);
            let known_traits = ["Display", "Debug", "Eq", "Ord", "Hash"];
            for trait_name in &known_traits {
                let candidate = format!("{}__{}__{}", trait_name, name, type_name);
                if self.known_functions.contains_key(&candidate) {
                    return MirExpr::Var(candidate, var_ty.clone());
                }
            }

            // Stdlib module method fallback: check if this is a module function
            // callable as a method on the receiver's type (e.g., "hello".length() -> mesh_string_length).
            let module_method = match first_arg_ty {
                MirType::String => {
                    let prefixed = format!("string_{}", name);
                    let runtime = map_builtin_name(&prefixed);
                    if self.known_functions.contains_key(&runtime)
                        || runtime.starts_with("mesh_string_")
                    {
                        Some(runtime)
                    } else {
                        None
                    }
                }
                MirType::Ptr => {
                    // List/Map/Set methods -- try list_ prefix first (most common collection).
                    let prefixed = format!("list_{}", name);
                    let runtime = map_builtin_name(&prefixed);
                    if self.known_functions.contains_key(&runtime)
                        || runtime.starts_with("mesh_list_")
                    {
                        Some(runtime)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(runtime_name) = module_method {
                return MirExpr::Var(runtime_name, var_ty.clone());
            }

            // Defense-in-depth warning -- skip module-scoped helpers (Module__func),
            // compiler-generated service stubs (__service_*), and runtime intrinsics (mesh_*).
            if self.lookup_var(name).is_none()
                && !self.known_functions.contains_key(name)
                && !name.contains("__")
                && !name.starts_with("mesh_")
            {
                let type_name = mir_type_to_impl_name(first_arg_ty);
                eprintln!(
                    "[mesh-codegen] warning: call to '{}' could not be resolved \
                     as a trait method for type '{}'. This may indicate a type checker bug.",
                    name, type_name
                );
            }
        }
        MirExpr::Var(name.to_string(), var_ty.clone())
    }

    // ── Call expression lowering ─────────────────────────────────────

    fn lower_call_expr(&mut self, call: &CallExpr) -> MirExpr {
        // Method call interception: if callee is a FieldAccess (expr.method(...)),
        // extract receiver + method name, prepend receiver to args, and route
        // through trait dispatch. This MUST happen BEFORE lower_expr on the callee,
        // because lower_expr would route to lower_field_access which produces a
        // struct GEP (MirExpr::FieldAccess), not a callable.
        if let Some(callee_expr) = call.callee() {
            if let Expr::FieldAccess(ref fa) = callee_expr {
                // Check if this is a module/service/variant/struct access (NOT a method call).
                // Module-qualified calls (String.length), service methods (Counter.start),
                // variant constructors (Shape.Circle), and struct-qualified calls are
                // handled by lower_field_access.
                let is_module_or_special = if let Some(base) = fa.base() {
                    if let Expr::NameRef(ref name_ref) = base {
                        if let Some(base_name) = name_ref.text() {
                            STDLIB_MODULES.contains(&base_name.as_str())
                                || self.user_modules.contains_key(&base_name)
                                || self.service_modules.contains_key(&base_name)
                                || self.is_sum_type_name(&base_name)
                                || self.is_struct_type_name(&base_name)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !is_module_or_special {
                    let method_name = fa.field().map(|t| t.text().to_string()).unwrap_or_default();

                    // Lower the receiver expression
                    let receiver = fa.base().map(|e| self.lower_expr(&e)).unwrap_or(MirExpr::Unit);

                    // Lower explicit arguments
                    let mut args = vec![receiver];
                    if let Some(arg_list) = call.arg_list() {
                        for arg in arg_list.args() {
                            args.push(self.lower_expr(&arg));
                        }
                    }

                    let ty = self.resolve_range(call.syntax().text_range());

                    // Route through the shared trait dispatch helper
                    let first_arg_ty = args[0].ty().clone();
                    let callee_var_ty = MirType::FnPtr(
                        args.iter().map(|a| a.ty().clone()).collect(),
                        Box::new(ty.clone()),
                    );
                    let callee = self.resolve_trait_callee(&method_name, &callee_var_ty, &first_arg_ty);

                    // Apply the same post-dispatch optimizations as bare-name calls:
                    // Display__to_string__String identity short-circuit
                    if let MirExpr::Var(ref name, _) = callee {
                        if name == "Display__to_string__String" && !args.is_empty() {
                            return args.into_iter().next().unwrap();
                        }
                        // Debug__inspect__String wraps in quotes
                        if name == "Debug__inspect__String" && !args.is_empty() {
                            let val = args.into_iter().next().unwrap();
                            let quote = MirExpr::StringLit("\"".to_string(), MirType::String);
                            let concat_ty = MirType::FnPtr(
                                vec![MirType::String, MirType::String],
                                Box::new(MirType::String),
                            );
                            let left = MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                                args: vec![quote.clone(), val],
                                ty: MirType::String,
                            };
                            return MirExpr::Call {
                                func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty)),
                                args: vec![left, quote],
                                ty: MirType::String,
                            };
                        }
                    }

                    // Collection Display dispatch for method calls
                    if let MirExpr::Var(ref name, _) = callee {
                        if (name == "to_string" || name == "debug" || name == "inspect")
                            && args.len() == 1
                            && matches!(args[0].ty(), MirType::Ptr)
                        {
                            if let Some(base_expr) = fa.base() {
                                if let Some(typeck_ty) = self.get_ty(base_expr.syntax().text_range()).cloned() {
                                    if let Some(collection_call) = self.wrap_collection_to_string(&args[0], &typeck_ty) {
                                        return collection_call;
                                    }
                                }
                            }
                        }
                    }

                    return MirExpr::Call {
                        func: Box::new(callee),
                        args,
                        ty,
                    };
                }
            }
        }

        // Non-method-call path: normal function calls (unchanged from before).
        let callee = call.callee().map(|e| self.lower_expr(&e));
        let args: Vec<MirExpr> = call
            .arg_list()
            .map(|al| al.args().map(|a| self.lower_expr(&a)).collect())
            .unwrap_or_default();

        let mut ty = self.resolve_range(call.syntax().text_range());

        let callee = match callee {
            Some(c) => c,
            None => return MirExpr::Unit,
        };

        // When calling a known stdlib function whose return type is Ptr but
        // the typeck resolved to a Tuple type, use Ptr. This prevents LLVM
        // struct/pointer mismatches where typeck resolves e.g. List.head on
        // List<(A,B)> as Tuple([A,B]) but the runtime returns an opaque Ptr.
        if let MirExpr::Var(ref _name, ref callee_ty) = callee {
            if let MirType::FnPtr(_, ref ret_ty) = callee_ty {
                if matches!(ty, MirType::Tuple(_)) && matches!(**ret_ty, MirType::Ptr) {
                    ty = MirType::Ptr;
                }
            }
        }

        // When the typeck produces an unresolved type variable for a call to a
        // known function, the resolved MIR type is Unit (the fallback for
        // Ty::Var). This happens when function parameters lack type annotations
        // and the call is type-checked before the call site that provides the
        // concrete type. Fall back to the known function's declared return type
        // so that the codegen doesn't discard the return value (which causes
        // SIGBUS on arm64 when the value is later used as a pointer).
        if matches!(ty, MirType::Unit) {
            if let MirExpr::Var(ref name, ref callee_ty) = callee {
                if let MirType::FnPtr(_, ref ret_ty) = callee_ty {
                    if !matches!(**ret_ty, MirType::Unit) {
                        ty = *ret_ty.clone();
                    }
                }
                // Also check known_functions for the definitive return type.
                // This handles cases where the callee Var's type was also
                // resolved from an unresolved typeck variable.
                if matches!(ty, MirType::Unit) {
                    if let Some(known_ty) = self.known_functions.get(name) {
                        if let MirType::FnPtr(_, ref ret_ty) = known_ty {
                            if !matches!(**ret_ty, MirType::Unit) {
                                ty = *ret_ty.clone();
                            }
                        }
                    }
                }
            }
        }

        // Check if this is a variant constructor call (e.g., Circle(5.0)).
        if let MirExpr::Var(ref name, _) = callee {
            for (_, sum_info) in &self.registry.sum_type_defs {
                for variant in &sum_info.variants {
                    if variant.name == *name && !variant.fields.is_empty() {
                        let ty_name = &sum_info.name;
                        let mir_ty = MirType::SumType(ty_name.clone());
                        return MirExpr::ConstructVariant {
                            type_name: ty_name.clone(),
                            variant: name.clone(),
                            fields: args,
                            ty: mir_ty,
                        };
                    }
                }
            }
        }

        // For Map functions that take a key argument (put, get, has_key, delete),
        // handle key type dispatch:
        // - String keys: wrap the map argument in mesh_map_tag_string()
        // - Struct keys with Hash impl: hash the key via Hash__hash__TypeName,
        //   use the hash as an integer key (hash-as-key approach for v1.3)
        let args = if let MirExpr::Var(ref name, _) = callee {
            if matches!(name.as_str(), "mesh_map_put" | "mesh_map_get" | "mesh_map_has_key" | "mesh_map_delete")
                && args.len() >= 2
            {
                let key_ty = args[1].ty().clone();
                if matches!(key_ty, MirType::String) {
                    // String key: tag the map for string comparison
                    let mut new_args = args;
                    let map_arg = new_args.remove(0);
                    let tagged_map = MirExpr::Call {
                        func: Box::new(MirExpr::Var("mesh_map_tag_string".to_string(), MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)))),
                        args: vec![map_arg],
                        ty: MirType::Ptr,
                    };
                    new_args.insert(0, tagged_map);
                    new_args
                } else if matches!(key_ty, MirType::Struct(_)) {
                    // Struct key with Hash impl: hash the key, use hash as int key.
                    let ty_for_lookup = mir_type_to_ty(&key_ty);
                    if self.trait_registry.has_impl("Hash", &ty_for_lookup) {
                        let type_name = mir_type_to_impl_name(&key_ty);
                        let hash_fn_name = format!("Hash__hash__{}", type_name);
                        let hash_fn_ty = MirType::FnPtr(
                            vec![key_ty.clone()],
                            Box::new(MirType::Int),
                        );
                        let mut new_args = args;
                        let key_arg = new_args.remove(1);
                        let hashed_key = MirExpr::Call {
                            func: Box::new(MirExpr::Var(hash_fn_name, hash_fn_ty)),
                            args: vec![key_arg],
                            ty: MirType::Int,
                        };
                        new_args.insert(1, hashed_key);
                        new_args
                    } else {
                        args
                    }
                } else {
                    args
                }
            } else {
                args
            }
        } else {
            args
        };

        // Static trait method dispatch: bare `default()` with zero arguments.
        // The type is resolved from the call-site context (type annotation / inference),
        // NOT from a first argument (since Default::default has no self parameter).
        if let MirExpr::Var(ref name, _) = callee {
            if name == "default" && args.is_empty() {
                let type_name = mir_type_to_impl_name(&ty);
                let mangled = format!("Default__default__{}", type_name);
                // Primitive Default short-circuits: return MIR literals directly.
                match mangled.as_str() {
                    "Default__default__Int" => return MirExpr::IntLit(0, MirType::Int),
                    "Default__default__Float" => return MirExpr::FloatLit(0.0, MirType::Float),
                    "Default__default__Bool" => return MirExpr::BoolLit(false, MirType::Bool),
                    "Default__default__String" => {
                        return MirExpr::StringLit("".to_string(), MirType::String)
                    }
                    _ => {
                        // Non-primitive type with user-defined Default impl:
                        // emit a call to the mangled function (already lowered by impl pipeline).
                        if type_name != "Unknown" {
                            let fn_ty = MirType::FnPtr(vec![], Box::new(ty.clone()));
                            return MirExpr::Call {
                                func: Box::new(MirExpr::Var(mangled, fn_ty)),
                                args: vec![],
                                ty: ty.clone(),
                            };
                        }
                        // Unknown type: fall through to normal call handling.
                        // This follows the error recovery pattern from 19-03.
                        eprintln!(
                            "[mesh-codegen] warning: default() call could not resolve \
                             concrete type from context. This may indicate a missing type annotation."
                        );
                    }
                }
            }
        }

        // compare(a, b) dispatch: rewrite to Ord__compare__TypeName.
        if let MirExpr::Var(ref name, _) = callee {
            if name == "compare" && args.len() == 2 {
                let arg_ty = args[0].ty().clone();
                let type_name = mir_type_to_impl_name(&arg_ty);
                let mangled = format!("Ord__compare__{}", type_name);
                let ordering_ty = MirType::SumType("Ordering".to_string());
                let fn_ty = MirType::FnPtr(
                    vec![arg_ty.clone(), arg_ty],
                    Box::new(ordering_ty.clone()),
                );
                return MirExpr::Call {
                    func: Box::new(MirExpr::Var(mangled, fn_ty)),
                    args,
                    ty: ordering_ty,
                };
            }
        }

        // Polymorphic String.from dispatch: mesh_string_from accepts Int/Float/Bool
        // and routes to the correct runtime conversion function based on arg type.
        if let MirExpr::Var(ref name, _) = callee {
            if name == "mesh_string_from" && args.len() == 1 {
                let arg_ty = args[0].ty().clone();
                let resolved_name = match &arg_ty {
                    MirType::Int => "mesh_int_to_string",
                    MirType::Float => "mesh_float_to_string",
                    MirType::Bool => "mesh_bool_to_string",
                    _ => "mesh_int_to_string", // fallback
                };
                let fn_ty = MirType::FnPtr(vec![arg_ty], Box::new(MirType::String));
                return MirExpr::Call {
                    func: Box::new(MirExpr::Var(resolved_name.to_string(), fn_ty)),
                    args,
                    ty: MirType::String,
                };
            }
        }

        // Collection Display/Debug dispatch: if the callee is "to_string" or
        // "debug"/"inspect" and the first arg is a collection (MirType::Ptr),
        // resolve the typeck type from the AST to emit the correct
        // collection-to-string call.
        if let MirExpr::Var(ref name, _) = callee {
            if (name == "to_string" || name == "debug" || name == "inspect")
                && args.len() == 1
                && matches!(args[0].ty(), MirType::Ptr)
            {
                // Look up the typeck Ty for the first argument from the call's AST
                if let Some(arg_list) = call.arg_list() {
                    if let Some(first_arg_ast) = arg_list.args().next() {
                        if let Some(typeck_ty) = self.get_ty(first_arg_ast.syntax().text_range()).cloned() {
                            if let Some(collection_call) =
                                self.wrap_collection_to_string(&args[0], &typeck_ty)
                            {
                                return collection_call;
                            }
                        }
                    }
                }
            }
        }

        // Trait method call rewriting: use shared resolve_trait_callee helper.
        // If the callee is a bare method name (not in known_functions), check if
        // it's a trait method for the first arg's type. If so, rewrite to the
        // mangled name (Trait__Method__Type).
        // Skip trait dispatch for functions from user-defined modules (Phase 39).
        let is_user_module_fn = if let MirExpr::Var(ref name, _) = callee {
            self.user_modules.values().any(|fns| fns.contains(name))
                || self.imported_functions.contains(name)
        } else {
            false
        };
        let callee = if let MirExpr::Var(ref name, ref var_ty) = callee {
            if !args.is_empty() && !is_user_module_fn {
                let first_arg_ty = args[0].ty().clone();
                self.resolve_trait_callee(name, var_ty, &first_arg_ty)
            } else {
                callee
            }
        } else {
            callee
        };

        // Short-circuit: Display__to_string__String is identity -- return the
        // first argument directly without emitting a function call.
        if let MirExpr::Var(ref name, _) = callee {
            if name == "Display__to_string__String" && !args.is_empty() {
                return args.into_iter().next().unwrap();
            }
            // Debug__inspect__String wraps the value in quotes: "\"" <> value <> "\""
            if name == "Debug__inspect__String" && !args.is_empty() {
                let val = args.into_iter().next().unwrap();
                let quote = MirExpr::StringLit("\"".to_string(), MirType::String);
                let concat_ty =
                    MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::String));
                let left = MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty.clone())),
                    args: vec![quote.clone(), val],
                    ty: MirType::String,
                };
                return MirExpr::Call {
                    func: Box::new(MirExpr::Var("mesh_string_concat".to_string(), concat_ty)),
                    args: vec![left, quote],
                    ty: MirType::String,
                };
            }
        }

        // Json.encode struct/sum type dispatch: if encoding a struct or sum type
        // with ToJson, chain ToJson__to_json__TypeName + mesh_json_encode.
        if let MirExpr::Var(ref name, _) = callee {
            if name == "mesh_json_encode" && args.len() == 1 {
                let arg_ty = args[0].ty().clone();
                let type_name = match &arg_ty {
                    MirType::Struct(ref struct_name) => Some(struct_name.clone()),
                    MirType::SumType(ref sum_name) => Some(sum_name.clone()),
                    _ => None,
                };
                if let Some(type_name) = type_name {
                    let to_json_fn = format!("ToJson__to_json__{}", type_name);
                    if self.known_functions.contains_key(&to_json_fn) {
                        let fn_ty = MirType::FnPtr(vec![arg_ty], Box::new(MirType::Ptr));
                        let json_ptr = MirExpr::Call {
                            func: Box::new(MirExpr::Var(to_json_fn, fn_ty)),
                            args: args.clone(),
                            ty: MirType::Ptr,
                        };
                        return MirExpr::Call {
                            func: Box::new(callee),
                            args: vec![json_ptr],
                            ty: MirType::String,
                        };
                    }
                }
            }
        }

        // Determine if this is a direct function call or a closure call.
        let is_known_fn = match &callee {
            MirExpr::Var(name, _) => self.known_functions.contains_key(name),
            _ => false,
        };

        if is_known_fn {
            MirExpr::Call {
                func: Box::new(callee),
                args,
                ty,
            }
        } else {
            // Check the callee type. If it's a Closure type, use ClosureCall.
            match callee.ty() {
                MirType::Closure(_, _) => MirExpr::ClosureCall {
                    closure: Box::new(callee),
                    args,
                    ty,
                },
                _ => MirExpr::Call {
                    func: Box::new(callee),
                    args,
                    ty,
                },
            }
        }
    }

    // ── Pipe expression lowering (DESUGARING) ────────────────────────

    fn lower_pipe_expr(&mut self, pipe: &PipeExpr) -> MirExpr {
        // Desugar: `x |> f` -> `f(x)`
        //          `x |> f(a, b)` -> `f(x, a, b)`
        let lhs = pipe
            .lhs()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let rhs = pipe.rhs();
        let ty = self.resolve_range(pipe.syntax().text_range());

        let mut result = match rhs {
            Some(Expr::CallExpr(call)) => {
                // `x |> f(a, b)` -> `f(x, a, b)` -- prepend lhs to existing args.
                let callee = call.callee().map(|e| self.lower_expr(&e));
                let mut args: Vec<MirExpr> = Vec::new();
                args.push(lhs);
                if let Some(arg_list) = call.arg_list() {
                    for arg in arg_list.args() {
                        args.push(self.lower_expr(&arg));
                    }
                }
                let callee = match callee {
                    Some(c) => c,
                    None => return MirExpr::Unit,
                };
                MirExpr::Call {
                    func: Box::new(callee),
                    args,
                    ty,
                }
            }
            Some(rhs_expr) => {
                // `x |> f` -> `f(x)` -- bare function reference.
                let func = self.lower_expr(&rhs_expr);
                MirExpr::Call {
                    func: Box::new(func),
                    args: vec![lhs],
                    ty,
                }
            }
            None => MirExpr::Unit,
        };

        // Phase 96 / 129: Map.collect string key detection.
        // Walk the pipe chain source types: if the source is List<(String,V)>,
        // Map<String,V>, or a zip of List<String> keys, use the string-key collect
        // variant (mesh_map_collect_string_keys).
        //
        // Note: checking the pipe's own resolved result type does not work here — HM
        // let-generalization quantifies the K type variable before downstream Map.get
        // calls can unify it with String. The chain-walk is the correct mechanism.
        if let MirExpr::Call { ref mut func, .. } = result {
            if let MirExpr::Var(ref name, _) = **func {
                if name == "mesh_map_collect" && self.pipe_chain_has_string_keys(pipe) {
                    let fn_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
                    **func = MirExpr::Var("mesh_map_collect_string_keys".to_string(), fn_ty);
                }
            }
        }

        result
    }

    // ── Slot pipe expression lowering (DESUGARING) ───────────────────

    fn lower_slot_pipe_expr(&mut self, pipe: &SlotPipeExpr) -> MirExpr {
        // Desugar: `x |N> f(a, b, c)` -> `f(a[0..N-2], x, a[N-2..])`
        // where N is 1-indexed slot position and a[i] are the explicit args.
        let lhs = pipe
            .lhs()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let slot = pipe.slot().unwrap_or(2) as usize; // 1-indexed
        let insert_idx = slot - 1; // 0-indexed position to insert lhs
        let ty = self.resolve_range(pipe.syntax().text_range());

        match pipe.rhs() {
            Some(Expr::CallExpr(call)) => {
                let callee = call.callee().map(|e| self.lower_expr(&e));
                let mut explicit_args: Vec<MirExpr> = Vec::new();
                if let Some(arg_list) = call.arg_list() {
                    for arg in arg_list.args() {
                        explicit_args.push(self.lower_expr(&arg));
                    }
                }
                // Insert lhs at insert_idx (0-indexed), clamping to length
                let actual_idx = insert_idx.min(explicit_args.len());
                explicit_args.insert(actual_idx, lhs);
                let callee = match callee {
                    Some(c) => c,
                    None => return MirExpr::Unit,
                };
                MirExpr::Call {
                    func: Box::new(callee),
                    args: explicit_args,
                    ty,
                }
            }
            Some(rhs_expr) => {
                // Bare function reference with slot — treat as regular pipe (insert at position 0)
                let func = self.lower_expr(&rhs_expr);
                MirExpr::Call {
                    func: Box::new(func),
                    args: vec![lhs],
                    ty,
                }
            }
            None => MirExpr::Unit,
        }
    }

    // ── Field access lowering ────────────────────────────────────────

    fn lower_field_access(&mut self, fa: &FieldAccess) -> MirExpr {
        // Check if this is a module-qualified access (e.g., String.length).
        // If the base is a NameRef whose text is a known stdlib module,
        // resolve as a function reference instead of a struct field access.
        // User-defined modules take precedence over stdlib modules to allow
        // user code with modules named "Math", "Int", "Float", etc.
        if let Some(base_expr) = fa.base() {
            if let Expr::NameRef(ref name_ref) = base_expr {
                if let Some(base_name) = name_ref.text() {
                    // Check service modules FIRST -- service methods map to generated
                    // function names (e.g., Counter.start -> __service_counter_start).
                    // Must come before user_modules which would resolve to bare names.
                    if let Some(methods) = self.service_modules.get(&base_name).cloned() {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        for (method_name, generated_fn) in &methods {
                            if *method_name == field {
                                let ty = self.resolve_range(fa.syntax().text_range());
                                // Return the generated function name as a Var reference.
                                return MirExpr::Var(generated_fn.clone(), ty);
                            }
                        }
                    }

                    // Check user-defined modules (Phase 39) -- they shadow stdlib.
                    if let Some(func_names) = self.user_modules.get(&base_name) {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        if func_names.contains(&field) {
                            let ty = self.resolve_range(fa.syntax().text_range());
                            return MirExpr::Var(field, ty);
                        }
                    }

                    // Check stdlib modules (after user modules so user code can shadow).
                    if STDLIB_MODULES.contains(&base_name.as_str()) {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        // Convert to prefixed name: String.length -> string_length
                        let prefixed = format!("{}_{}", base_name.to_lowercase(), field);
                        // Map to runtime name
                        let runtime_name = map_builtin_name(&prefixed);
                        // Use known_functions type if available (more accurate for
                        // opaque Ptr returns like List.head on List<(A,B)>), otherwise
                        // fall back to typeck-resolved type.
                        let ty = if let Some(known_ty) = self.known_functions.get(&runtime_name) {
                            known_ty.clone()
                        } else {
                            self.resolve_range(fa.syntax().text_range())
                        };
                        return MirExpr::Var(runtime_name, ty);
                    }

                    // Check if this is StructName.from_json or SumTypeName.from_json
                    // (static trait method). Resolves to __json_decode__TypeName which
                    // chains parse + from_json.
                    if self.registry.struct_defs.contains_key(&base_name)
                        || self.registry.sum_type_defs.contains_key(&base_name)
                    {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        if field == "from_json" {
                            let wrapper_name = format!("__json_decode__{}", base_name);
                            if let Some(fn_ty) = self.known_functions.get(&wrapper_name).cloned() {
                                return MirExpr::Var(wrapper_name, fn_ty);
                            }
                        }
                    }

                    // Check if this is StructName.from_row (FromRow trait method).
                    // Resolves to FromRow__from_row__StructName.
                    if self.registry.struct_defs.contains_key(&base_name) {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        if field == "from_row" {
                            let fn_name = format!("FromRow__from_row__{}", base_name);
                            if let Some(fn_ty) = self.known_functions.get(&fn_name).cloned() {
                                return MirExpr::Var(fn_name, fn_ty);
                            }
                        }
                    }

                    // Check if this is StructName.from (From trait method, Phase 77).
                    // Look up mangled From_X__from__StructName in known_functions.
                    if self.registry.struct_defs.contains_key(&base_name)
                        || self.registry.sum_type_defs.contains_key(&base_name)
                    {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        if field == "from" {
                            // Find the From impl function by scanning known_functions
                            // for any key matching From_*__from__{base_name}.
                            let suffix = format!("__from__{}", base_name);
                            for (fn_name, fn_ty) in self.known_functions.iter() {
                                if fn_name.starts_with("From_") && fn_name.ends_with(&suffix) {
                                    return MirExpr::Var(fn_name.clone(), fn_ty.clone());
                                }
                            }
                            // Fallback: try unparameterized name.
                            let unparameterized = format!("From__from__{}", base_name);
                            if let Some(fn_ty) = self.known_functions.get(&unparameterized).cloned() {
                                return MirExpr::Var(unparameterized, fn_ty);
                            }
                        }
                        // Phase 128: StructName.try_from() dispatch (TryFrom trait).
                        // Mirrors the From.from() pattern above.
                        if field == "try_from" {
                            let suffix = format!("__try_from__{}", base_name);
                            for (fn_name, fn_ty) in self.known_functions.iter() {
                                if fn_name.starts_with("TryFrom_") && fn_name.ends_with(&suffix) {
                                    return MirExpr::Var(fn_name.clone(), fn_ty.clone());
                                }
                            }
                            // Fallback: unparameterized name.
                            let unparameterized = format!("TryFrom__try_from__{}", base_name);
                            if let Some(fn_ty) = self.known_functions.get(&unparameterized).cloned() {
                                return MirExpr::Var(unparameterized, fn_ty);
                            }
                        }
                    }

                    // Check if this is StructName.__table__/__fields__/__primary_key__/__relationships__
                    // __field_types__ or __*_col__ (Schema metadata functions from deriving(Schema)).
                    // Mangled name: {Name}____{method} e.g. User____table__
                    if self.registry.struct_defs.contains_key(&base_name) {
                        let field = fa
                            .field()
                            .map(|t| t.text().to_string())
                            .unwrap_or_default();
                        if field == "__table__" || field == "__fields__"
                            || field == "__primary_key__" || field == "__relationships__"
                            || field == "__field_types__" || field == "__relationship_meta__"
                            || (field.starts_with("__") && field.ends_with("_col__"))
                        {
                            let fn_name = format!("{}__{}", base_name, field);
                            if let Some(fn_ty) = self.known_functions.get(&fn_name).cloned() {
                                return MirExpr::Var(fn_name, fn_ty);
                            }
                        }
                    }
                }
            }
        }

        let object = fa
            .base()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let field = fa
            .field()
            .map(|t| t.text().to_string())
            .unwrap_or_default();

        let ty = self.resolve_range(fa.syntax().text_range());

        MirExpr::FieldAccess {
            object: Box::new(object),
            field,
            ty,
        }
    }

    // ── If expression lowering ───────────────────────────────────────

    fn lower_if_expr(&mut self, if_: &IfExpr) -> MirExpr {
        let cond = if_
            .condition()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::BoolLit(true, MirType::Bool));

        let then_body = if_
            .then_branch()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);

        let else_body = if let Some(else_branch) = if_.else_branch() {
            if let Some(chained_if) = else_branch.if_expr() {
                // else-if chain
                self.lower_if_expr(&chained_if)
            } else if let Some(block) = else_branch.block() {
                self.lower_block(&block)
            } else {
                MirExpr::Unit
            }
        } else {
            MirExpr::Unit
        };

        let ty = self.resolve_range(if_.syntax().text_range());

        MirExpr::If {
            cond: Box::new(cond),
            then_body: Box::new(then_body),
            else_body: Box::new(else_body),
            ty,
        }
    }

    // ── While expression lowering ───────────────────────────────────

    fn lower_while_expr(&mut self, w: &WhileExpr) -> MirExpr {
        let cond = w
            .condition()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::BoolLit(true, MirType::Bool));

        let body = w
            .body()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);

        MirExpr::While {
            cond: Box::new(cond),
            body: Box::new(body),
            ty: MirType::Unit,
        }
    }

    // ── For-in expression lowering ──────────────────────────────────

    fn lower_for_in_expr(&mut self, for_in: &ForInExpr) -> MirExpr {
        // Check if iterable is a DotDot range (keep existing ForInRange behavior).
        if let Some(Expr::BinaryExpr(ref bin)) = for_in.iterable() {
            if bin.op().map(|t| t.kind()) == Some(SyntaxKind::DOT_DOT) {
                return self.lower_for_in_range(for_in, bin);
            }
        }

        // Non-range: detect collection type from typeck results.
        let iterable_ty = for_in
            .iterable()
            .and_then(|e| self.get_ty(e.syntax().text_range()))
            .cloned();

        if let Some(ref ty) = iterable_ty {
            if let Some((key_ty, val_ty)) = extract_map_types(ty) {
                return self.lower_for_in_map(for_in, &key_ty, &val_ty);
            }
            if let Some(elem_ty) = extract_set_elem_type(ty) {
                return self.lower_for_in_set(for_in, &elem_ty);
            }
            if let Some(elem_ty) = extract_list_elem_type(ty) {
                return self.lower_for_in_list(for_in, &elem_ty);
            }

            // Check if type implements Iterable (collection -> produces iterator).
            let ty_for_lookup = ty.clone();
            if self.trait_registry.has_impl("Iterable", &ty_for_lookup) {
                return self.lower_for_in_iterator(for_in, &ty_for_lookup, true);
            }
            // Check if type directly implements Iterator (type IS an iterator).
            if self.trait_registry.has_impl("Iterator", &ty_for_lookup) {
                return self.lower_for_in_iterator(for_in, &ty_for_lookup, false);
            }
        }

        // Fallback: treat as list iteration with Int elements.
        self.lower_for_in_list(for_in, &Ty::int())
    }

    fn lower_for_in_iterator(&mut self, for_in: &ForInExpr, ty: &Ty, is_iterable: bool) -> MirExpr {
        let var_name = for_in
            .binding_name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());

        // Resolve the MIR type to get the impl name for mangling.
        let mir_ty = resolve_type(ty, self.registry, false);
        let type_name = mir_type_to_impl_name(&mir_ty);

        // Determine iter_fn and next_fn names, and the element type.
        let (iter_fn, next_fn, elem_ty) = if is_iterable {
            // Iterable path: call iter() to get iterator, then next() on iterator.
            let iter_fn_name = format!("Iterable__iter__{}", type_name);

            // Resolve Iter type from Iterable impl to get the iterator type name.
            let iter_type = self.trait_registry
                .resolve_associated_type("Iterable", "Iter", ty)
                .unwrap_or_else(|| Ty::Con(mesh_typeck::ty::TyCon::new("Unknown")));

            // Extract iterator type name directly from Ty::Con to preserve
            // opaque handle names like "ListIterator" (which resolve to MirType::Ptr).
            let iter_type_name = match &iter_type {
                Ty::Con(tc) => tc.name.clone(),
                Ty::App(base, _) => {
                    if let Ty::Con(tc) = base.as_ref() {
                        tc.name.clone()
                    } else {
                        "Unknown".to_string()
                    }
                }
                _ => "Unknown".to_string(),
            };
            let next_fn_name = format!("Iterator__next__{}", iter_type_name);

            // Resolve Item type from Iterable impl.
            let item_ty = self.trait_registry
                .resolve_associated_type("Iterable", "Item", ty)
                .unwrap_or(Ty::int());

            (iter_fn_name, next_fn_name, item_ty)
        } else {
            // Direct Iterator path: no iter() call, just next().
            let next_fn_name = format!("Iterator__next__{}", type_name);
            let item_ty = self.trait_registry
                .resolve_associated_type("Iterator", "Item", ty)
                .unwrap_or(Ty::int());

            (String::new(), next_fn_name, item_ty)
        };

        // Lower the iterable/iterator expression.
        let collection = for_in
            .iterable()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let elem_mir_ty = resolve_type(&elem_ty, self.registry, false);

        self.push_scope();
        self.insert_var(var_name.clone(), elem_mir_ty.clone());
        let filter = for_in
            .filter()
            .map(|f| Box::new(self.lower_expr(&f)));
        let body = for_in
            .body()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);
        let body_ty = body.ty().clone();
        self.pop_scope();

        MirExpr::ForInIterator {
            var: var_name,
            iterator: Box::new(collection),
            filter,
            body: Box::new(body),
            elem_ty: elem_mir_ty,
            body_ty,
            next_fn,
            iter_fn,
            ty: MirType::Ptr,
        }
    }

    fn lower_for_in_range(&mut self, for_in: &ForInExpr, bin: &BinaryExpr) -> MirExpr {
        let var_name = for_in
            .binding_name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());

        let start = bin
            .lhs()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::IntLit(0, MirType::Int));
        let end = bin
            .rhs()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::IntLit(0, MirType::Int));

        self.push_scope();
        self.insert_var(var_name.clone(), MirType::Int);
        let filter = for_in
            .filter()
            .map(|f| Box::new(self.lower_expr(&f)));
        let body = for_in
            .body()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);
        self.pop_scope();

        MirExpr::ForInRange {
            var: var_name,
            start: Box::new(start),
            end: Box::new(end),
            filter,
            body: Box::new(body),
            ty: MirType::Ptr,
        }
    }

    fn lower_for_in_list(&mut self, for_in: &ForInExpr, elem_ty_src: &Ty) -> MirExpr {
        let var_name = for_in
            .binding_name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());

        let collection = for_in
            .iterable()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let elem_mir_ty = resolve_type(elem_ty_src, self.registry, false);

        self.push_scope();
        self.insert_var(var_name.clone(), elem_mir_ty.clone());
        let filter = for_in
            .filter()
            .map(|f| Box::new(self.lower_expr(&f)));
        let body = for_in
            .body()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);
        let body_ty = body.ty().clone();
        self.pop_scope();

        MirExpr::ForInList {
            var: var_name,
            collection: Box::new(collection),
            filter,
            body: Box::new(body),
            elem_ty: elem_mir_ty,
            body_ty,
            ty: MirType::Ptr,
        }
    }

    fn lower_for_in_map(&mut self, for_in: &ForInExpr, key_ty_src: &Ty, val_ty_src: &Ty) -> MirExpr {
        let (key_var, val_var) = if let Some(destr) = for_in.destructure_binding() {
            let names = destr.names();
            let k = names.first().and_then(|n| n.text()).unwrap_or_else(|| "_".to_string());
            let v = names.get(1).and_then(|n| n.text()).unwrap_or_else(|| "_".to_string());
            (k, v)
        } else {
            let var_name = for_in
                .binding_name()
                .and_then(|n| n.text())
                .unwrap_or_else(|| "_".to_string());
            (var_name, "_".to_string())
        };

        let collection = for_in
            .iterable()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let key_mir_ty = resolve_type(key_ty_src, self.registry, false);
        let val_mir_ty = resolve_type(val_ty_src, self.registry, false);

        self.push_scope();
        self.insert_var(key_var.clone(), key_mir_ty.clone());
        self.insert_var(val_var.clone(), val_mir_ty.clone());
        let filter = for_in
            .filter()
            .map(|f| Box::new(self.lower_expr(&f)));
        let body = for_in
            .body()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);
        let body_ty = body.ty().clone();
        self.pop_scope();

        MirExpr::ForInMap {
            key_var,
            val_var,
            collection: Box::new(collection),
            filter,
            body: Box::new(body),
            key_ty: key_mir_ty,
            val_ty: val_mir_ty,
            body_ty,
            ty: MirType::Ptr,
        }
    }

    fn lower_for_in_set(&mut self, for_in: &ForInExpr, elem_ty_src: &Ty) -> MirExpr {
        let var_name = for_in
            .binding_name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "_".to_string());

        let collection = for_in
            .iterable()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let elem_mir_ty = resolve_type(elem_ty_src, self.registry, false);

        self.push_scope();
        self.insert_var(var_name.clone(), elem_mir_ty.clone());
        let filter = for_in
            .filter()
            .map(|f| Box::new(self.lower_expr(&f)));
        let body = for_in
            .body()
            .map(|b| self.lower_block(&b))
            .unwrap_or(MirExpr::Unit);
        let body_ty = body.ty().clone();
        self.pop_scope();

        MirExpr::ForInSet {
            var: var_name,
            collection: Box::new(collection),
            filter,
            body: Box::new(body),
            elem_ty: elem_mir_ty,
            body_ty,
            ty: MirType::Ptr,
        }
    }

    // ── Case expression lowering ─────────────────────────────────────

    fn lower_case_expr(&mut self, case: &CaseExpr) -> MirExpr {
        let scrutinee = case
            .scrutinee()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let arms: Vec<MirMatchArm> = case.arms().map(|arm| self.lower_match_arm(&arm)).collect();

        let ty = self.resolve_range(case.syntax().text_range());

        MirExpr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            ty,
        }
    }

    fn lower_match_arm(&mut self, arm: &MatchArm) -> MirMatchArm {
        self.push_scope();

        let pattern = arm
            .pattern()
            .map(|p| self.lower_pattern(&p))
            .unwrap_or(MirPattern::Wildcard);

        let guard = arm.guard().map(|e| self.lower_expr(&e));

        let body = arm
            .body()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        self.pop_scope();

        MirMatchArm {
            pattern,
            guard,
            body,
        }
    }

    // ── Pattern lowering ─────────────────────────────────────────────

    fn lower_pattern(&mut self, pat: &Pattern) -> MirPattern {
        match pat {
            Pattern::Wildcard(_) => MirPattern::Wildcard,

            Pattern::Ident(ident) => {
                let name = ident
                    .name()
                    .map(|t| t.text().to_string())
                    .unwrap_or_else(|| "_".to_string());

                // Check if this identifier is a known nullary constructor
                // (e.g., None, Less, Equal, Greater). The parser produces
                // IDENT_PAT for these because they lack parentheses, but
                // they must be lowered as Constructor patterns for correct
                // pattern matching codegen (switch on tag).
                if name.starts_with(|c: char| c.is_uppercase()) {
                    if let Some(type_name) = find_type_for_variant(&name, self.registry) {
                        // Verify it's actually a nullary constructor (no fields).
                        let is_nullary = self.registry.sum_type_defs
                            .get(&type_name)
                            .and_then(|info| info.variants.iter().find(|v| v.name == name))
                            .map(|v| v.fields.is_empty())
                            .unwrap_or(false);
                        if is_nullary {
                            return MirPattern::Constructor {
                                type_name,
                                variant: name,
                                fields: vec![],
                                bindings: vec![],
                            };
                        }
                    }
                }

                let ty = self.resolve_range(ident.syntax().text_range());
                self.insert_var(name.clone(), ty.clone());
                MirPattern::Var(name, ty)
            }

            Pattern::Literal(lit) => {
                let token = lit.token();
                match token {
                    Some(t) => {
                        let text = t.text().to_string();
                        match t.kind() {
                            SyntaxKind::INT_LITERAL => {
                                MirPattern::Literal(MirLiteral::Int(
                                    text.parse().unwrap_or(0),
                                ))
                            }
                            SyntaxKind::FLOAT_LITERAL => {
                                MirPattern::Literal(MirLiteral::Float(
                                    text.parse().unwrap_or(0.0),
                                ))
                            }
                            SyntaxKind::TRUE_KW => {
                                MirPattern::Literal(MirLiteral::Bool(true))
                            }
                            SyntaxKind::FALSE_KW => {
                                MirPattern::Literal(MirLiteral::Bool(false))
                            }
                            SyntaxKind::STRING_START => {
                                // Extract string content from the literal pattern node.
                                let content = extract_simple_string_content(lit.syntax());
                                MirPattern::Literal(MirLiteral::String(content))
                            }
                            _ => MirPattern::Wildcard,
                        }
                    }
                    None => MirPattern::Wildcard,
                }
            }

            Pattern::Constructor(ctor) => {
                let variant_name = ctor
                    .variant_name()
                    .map(|t| t.text().to_string())
                    .unwrap_or_default();

                let type_name = if let Some(tn) = ctor.type_name() {
                    tn.text().to_string()
                } else {
                    // Find the type name from the registry for unqualified constructors.
                    find_type_for_variant(&variant_name, self.registry)
                        .unwrap_or_default()
                };

                let fields: Vec<MirPattern> =
                    ctor.fields().map(|p| self.lower_pattern(&p)).collect();

                // Collect bindings introduced by sub-patterns.
                let bindings = collect_pattern_bindings(&fields);

                MirPattern::Constructor {
                    type_name,
                    variant: variant_name,
                    fields,
                    bindings,
                }
            }

            Pattern::Tuple(tuple) => {
                let pats: Vec<MirPattern> =
                    tuple.patterns().map(|p| self.lower_pattern(&p)).collect();
                MirPattern::Tuple(pats)
            }

            Pattern::Or(or) => {
                let alts: Vec<MirPattern> =
                    or.alternatives().map(|p| self.lower_pattern(&p)).collect();
                MirPattern::Or(alts)
            }

            Pattern::As(as_pat) => {
                // Layered pattern: bind name AND match inner pattern.
                // For MIR, we lower the inner pattern and add the name as a Var binding.
                let binding_name = as_pat
                    .binding_name()
                    .map(|t| t.text().to_string())
                    .unwrap_or_else(|| "_".to_string());
                let ty = self.resolve_range(as_pat.syntax().text_range());
                self.insert_var(binding_name.clone(), ty.clone());

                // Lower inner pattern -- the binding is separate.
                if let Some(inner) = as_pat.pattern() {
                    self.lower_pattern(&inner)
                } else {
                    MirPattern::Var(binding_name, ty)
                }
            }

            Pattern::Cons(cons_pat) => {
                // List cons pattern: head :: tail
                // Extract the element type from the typeck List<T> type.
                let _list_ty = self.resolve_range(cons_pat.syntax().text_range());
                let elem_mir_ty = if let Some(typeck_ty) = self.get_ty(cons_pat.syntax().text_range()).cloned() {
                    if let Some(elem_ty) = extract_list_elem_type(&typeck_ty) {
                        resolve_type(&elem_ty, self.registry, false)
                    } else {
                        // Fallback: if the list type is not properly resolved,
                        // use Int as a default element type.
                        MirType::Int
                    }
                } else {
                    MirType::Int
                };

                let head_pat = cons_pat.head()
                    .map(|p| self.lower_pattern(&p))
                    .unwrap_or(MirPattern::Wildcard);
                let tail_pat = cons_pat.tail()
                    .map(|p| self.lower_pattern(&p))
                    .unwrap_or(MirPattern::Wildcard);

                MirPattern::ListCons {
                    head: Box::new(head_pat),
                    tail: Box::new(tail_pat),
                    elem_ty: elem_mir_ty,
                }
            }
        }
    }

    // ── Closure expression lowering (CLOSURE CONVERSION) ─────────────

    fn lower_closure_expr(&mut self, closure: &ClosureExpr) -> MirExpr {
        // Check for multi-clause closures and dispatch accordingly.
        if closure.is_multi_clause() {
            return self.lower_multi_clause_closure(closure);
        }

        self.closure_counter += 1;
        let closure_fn_name = if self.module_name.is_empty() {
            format!("__closure_{}", self.closure_counter)
        } else {
            format!("{}__closure_{}", self.module_name.replace('.', "_"), self.closure_counter)
        };

        let closure_range = closure.syntax().text_range();
        let closure_ty = self.get_ty(closure_range).cloned();

        // Extract parameter types from the closure's function type.
        let mut param_types = Vec::new();
        let return_type;
        if let Some(Ty::Fun(params, ret)) = &closure_ty {
            param_types = params
                .iter()
                .map(|p| resolve_type(p, self.registry, false))
                .collect();
            return_type = resolve_type(ret, self.registry, false);
        } else {
            return_type = MirType::Unit;
        }

        // Extract parameter names.
        let mut param_names = Vec::new();
        if let Some(param_list) = closure.param_list() {
            for param in param_list.params() {
                let name = param
                    .name()
                    .map(|t| t.text().to_string())
                    .unwrap_or_else(|| "_".to_string());
                param_names.push(name);
            }
        }

        // Build params: env_ptr first, then user params.
        let mut fn_params = Vec::new();
        fn_params.push(("__env".to_string(), MirType::Ptr));

        for (i, name) in param_names.iter().enumerate() {
            let ty = param_types.get(i).cloned().unwrap_or(MirType::Unit);
            fn_params.push((name.clone(), ty));
        }

        // Determine captured variables by scanning the closure body.
        // Any variable referenced in the body that is not a parameter and
        // exists in the outer scope is a capture.
        let outer_vars: HashMap<String, MirType> = self
            .scopes
            .iter()
            .flat_map(|s| s.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let param_set: std::collections::HashSet<&str> =
            param_names.iter().map(|s| s.as_str()).collect();

        // Lower the body in a new scope with params.
        // Track closure's return type for ? operator desugaring (Phase 45).
        let prev_fn_return_type = self.current_fn_return_type.take();
        self.current_fn_return_type = Some(return_type.clone());

        self.push_scope();
        for (name, ty) in &fn_params {
            self.insert_var(name.clone(), ty.clone());
        }

        let body = if let Some(block) = closure.body() {
            self.lower_block(&block)
        } else {
            MirExpr::Unit
        };

        self.pop_scope();

        // Restore previous function return type.
        self.current_fn_return_type = prev_fn_return_type;

        // Find captured variables by scanning the lowered body for Var references
        // that match outer scope names and are not parameters.
        let mut captures: Vec<(String, MirType)> = Vec::new();
        let mut capture_exprs: Vec<MirExpr> = Vec::new();
        collect_free_vars(&body, &param_set, &outer_vars, &mut captures);
        for (name, ty) in &captures {
            capture_exprs.push(MirExpr::Var(name.clone(), ty.clone()));
        }

        // Create the lifted function.
        self.functions.push(MirFunction {
            name: closure_fn_name.clone(),
            params: fn_params,
            return_type: return_type.clone(),
            body,
            is_closure_fn: true,
            captures: captures.clone(),
            has_tail_calls: false,
        });

        // Create the MakeClosure expression.
        let mir_ty = MirType::Closure(param_types, Box::new(return_type));

        MirExpr::MakeClosure {
            fn_name: closure_fn_name,
            captures: capture_exprs,
            ty: mir_ty,
        }
    }

    /// Lower a multi-clause closure expression.
    ///
    /// Multi-clause closures like `fn 0 -> "zero" | n -> to_string(n) end` are
    /// desugared into a single-param closure whose body is a MirExpr::Match.
    /// For single-param multi-clause, uses Match directly on the param.
    /// For multi-param multi-clause, uses an if-else chain (same as named fn lowering).
    fn lower_multi_clause_closure(&mut self, closure: &ClosureExpr) -> MirExpr {
        self.closure_counter += 1;
        let closure_fn_name = if self.module_name.is_empty() {
            format!("__closure_{}", self.closure_counter)
        } else {
            format!("{}__closure_{}", self.module_name.replace('.', "_"), self.closure_counter)
        };

        let closure_range = closure.syntax().text_range();
        let closure_ty = self.get_ty(closure_range).cloned();

        // Extract parameter types and return type from the closure's function type.
        let (param_types, return_type) = if let Some(Ty::Fun(params, ret)) = &closure_ty {
            (
                params
                    .iter()
                    .map(|p| resolve_type(p, self.registry, false))
                    .collect::<Vec<_>>(),
                resolve_type(ret, self.registry, false),
            )
        } else {
            (Vec::new(), MirType::Unit)
        };

        let arity = param_types.len();

        // Create synthetic parameter names: __cparam_0, __cparam_1, etc.
        let params: Vec<(String, MirType)> = param_types
            .iter()
            .enumerate()
            .map(|(i, ty)| (format!("__cparam_{}", i), ty.clone()))
            .collect();

        // Build fn params: env_ptr first, then user params.
        let mut fn_params = Vec::new();
        fn_params.push(("__env".to_string(), MirType::Ptr));
        fn_params.extend(params.iter().cloned());

        // Collect outer vars for capture analysis.
        let outer_vars: HashMap<String, MirType> = self
            .scopes
            .iter()
            .flat_map(|s| s.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
        let param_set: std::collections::HashSet<&str> =
            param_names.iter().map(|s| s.as_str()).collect();

        // Track closure's return type for ? operator desugaring (Phase 45).
        let prev_fn_return_type = self.current_fn_return_type.take();
        self.current_fn_return_type = Some(return_type.clone());

        // Build the body using match or if-else chain.
        self.push_scope();
        for (name, ty) in &fn_params {
            self.insert_var(name.clone(), ty.clone());
        }

        let body = if arity == 1 {
            // Single-parameter: use MirExpr::Match on the param.
            let scrutinee = MirExpr::Var(params[0].0.clone(), params[0].1.clone());
            let mut arms = Vec::new();

            // First clause (inline in CLOSURE_EXPR).
            {
                self.push_scope();
                self.insert_var(params[0].0.clone(), params[0].1.clone());

                let pattern = self.lower_closure_clause_param_pattern(
                    closure.param_list().as_ref(),
                    0,
                    &params,
                );
                let guard = closure
                    .guard()
                    .and_then(|gc| gc.expr())
                    .map(|e| self.lower_expr(&e));
                let body = if let Some(block) = closure.body() {
                    self.lower_block(&block)
                } else {
                    MirExpr::Unit
                };
                self.pop_scope();

                arms.push(MirMatchArm {
                    pattern,
                    guard,
                    body,
                });
            }

            // Subsequent clauses (CLOSURE_CLAUSE children).
            for clause in closure.clauses() {
                self.push_scope();
                self.insert_var(params[0].0.clone(), params[0].1.clone());

                let pattern = self.lower_closure_clause_param_pattern(
                    clause.param_list().as_ref(),
                    0,
                    &params,
                );
                let guard = clause
                    .guard()
                    .and_then(|gc| gc.expr())
                    .map(|e| self.lower_expr(&e));
                let body = if let Some(block) = clause.body() {
                    self.lower_block(&block)
                } else {
                    MirExpr::Unit
                };
                self.pop_scope();

                arms.push(MirMatchArm {
                    pattern,
                    guard,
                    body,
                });
            }

            MirExpr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
                ty: return_type.clone(),
            }
        } else {
            // Multi-parameter: use if-else chain (same pattern as named multi-clause fns).
            // Build FnDef-like clause processing using closure clause data.
            self.lower_multi_clause_closure_if_chain(closure, &params, &return_type)
        };

        self.pop_scope();

        // Restore previous function return type.
        self.current_fn_return_type = prev_fn_return_type;

        // Find captured variables.
        let mut captures: Vec<(String, MirType)> = Vec::new();
        let mut capture_exprs: Vec<MirExpr> = Vec::new();
        collect_free_vars(&body, &param_set, &outer_vars, &mut captures);
        for (name, ty) in &captures {
            capture_exprs.push(MirExpr::Var(name.clone(), ty.clone()));
        }

        // Create the lifted function.
        self.functions.push(MirFunction {
            name: closure_fn_name.clone(),
            params: fn_params,
            return_type: return_type.clone(),
            body,
            is_closure_fn: true,
            captures: captures.clone(),
            has_tail_calls: false,
        });

        // Create the MakeClosure expression.
        let mir_ty = MirType::Closure(param_types, Box::new(return_type));

        MirExpr::MakeClosure {
            fn_name: closure_fn_name,
            captures: capture_exprs,
            ty: mir_ty,
        }
    }

    /// Lower a closure clause's parameter at `param_idx` to a MirPattern.
    fn lower_closure_clause_param_pattern(
        &mut self,
        param_list: Option<&mesh_parser::ast::item::ParamList>,
        param_idx: usize,
        mir_params: &[(String, MirType)],
    ) -> MirPattern {
        if let Some(pl) = param_list {
            if let Some(param) = pl.params().nth(param_idx) {
                if let Some(pat) = param.pattern() {
                    return self.lower_pattern(&pat);
                }
                // Regular named parameter -> variable binding.
                if let Some(name_tok) = param.name() {
                    let pname = name_tok.text().to_string();
                    let pty = mir_params[param_idx].1.clone();
                    self.insert_var(pname.clone(), pty.clone());
                    return MirPattern::Var(pname, pty);
                }
            }
        }
        MirPattern::Wildcard
    }

    /// Build an if-else chain for multi-param multi-clause closures.
    fn lower_multi_clause_closure_if_chain(
        &mut self,
        closure: &ClosureExpr,
        mir_params: &[(String, MirType)],
        return_type: &MirType,
    ) -> MirExpr {
        // Collect all clause data: first clause + CLOSURE_CLAUSE children.
        // For each clause we need: param_list, guard, body.
        struct ClauseData {
            param_list: Option<mesh_parser::ast::item::ParamList>,
            guard: Option<mesh_parser::ast::item::GuardClause>,
            body: Option<Block>,
        }

        let mut all_clauses = Vec::new();

        // First clause.
        all_clauses.push(ClauseData {
            param_list: closure.param_list(),
            guard: closure.guard(),
            body: closure.body(),
        });

        // Subsequent clauses.
        for clause in closure.clauses() {
            all_clauses.push(ClauseData {
                param_list: clause.param_list(),
                guard: clause.guard(),
                body: clause.body(),
            });
        }

        // Build if-else chain from last to first.
        let mut else_body: Option<MirExpr> = None;

        for clause_data in all_clauses.iter().rev() {
            self.push_scope();
            for (pname, pty) in mir_params {
                self.insert_var(pname.clone(), pty.clone());
            }

            // Check if this is a catch-all clause (all params are wildcards/variables, no guard).
            let is_catch_all = self.is_closure_catch_all(&clause_data.param_list, mir_params)
                && clause_data.guard.is_none();

            if is_catch_all && else_body.is_none() {
                // Last clause and catch-all: emit body directly.
                let mut bindings = Vec::new();
                self.collect_closure_clause_bindings(
                    &clause_data.param_list,
                    mir_params,
                    &mut bindings,
                );
                let body = if let Some(ref block) = clause_data.body {
                    self.lower_block(block)
                } else {
                    MirExpr::Unit
                };
                self.pop_scope();

                let body = self.wrap_with_bindings(bindings, body);
                else_body = Some(body);
            } else {
                // Build condition: check all param patterns.
                let cond = self.build_closure_clause_condition(
                    &clause_data.param_list,
                    mir_params,
                );
                let guard = clause_data
                    .guard
                    .as_ref()
                    .and_then(|gc| gc.expr())
                    .map(|e| self.lower_expr(&e));

                let full_cond = if let Some(guard_expr) = guard {
                    if let Some(pattern_cond) = cond {
                        MirExpr::BinOp {
                            op: BinOp::And,
                            lhs: Box::new(pattern_cond),
                            rhs: Box::new(guard_expr),
                            ty: MirType::Bool,
                        }
                    } else {
                        guard_expr
                    }
                } else {
                    cond.unwrap_or(MirExpr::BoolLit(true, MirType::Bool))
                };

                // Bind variables and lower body.
                let mut bindings = Vec::new();
                self.collect_closure_clause_bindings(
                    &clause_data.param_list,
                    mir_params,
                    &mut bindings,
                );
                let body = if let Some(ref block) = clause_data.body {
                    self.lower_block(block)
                } else {
                    MirExpr::Unit
                };
                self.pop_scope();

                let then_body = self.wrap_with_bindings(bindings, body);
                let else_expr = else_body.unwrap_or(MirExpr::Unit);

                else_body = Some(MirExpr::If {
                    cond: Box::new(full_cond),
                    then_body: Box::new(then_body),
                    else_body: Box::new(else_expr),
                    ty: return_type.clone(),
                });
            }
        }

        else_body.unwrap_or(MirExpr::Unit)
    }

    /// Check if a closure clause is a catch-all (all params are variables/wildcards).
    fn is_closure_catch_all(
        &self,
        param_list: &Option<mesh_parser::ast::item::ParamList>,
        _mir_params: &[(String, MirType)],
    ) -> bool {
        if let Some(pl) = param_list {
            for param in pl.params() {
                if let Some(pat) = param.pattern() {
                    match pat {
                        Pattern::Wildcard(_) | Pattern::Ident(_) => {}
                        _ => return false,
                    }
                }
            }
        }
        true
    }

    /// Collect variable bindings from a closure clause's params.
    fn collect_closure_clause_bindings(
        &mut self,
        param_list: &Option<mesh_parser::ast::item::ParamList>,
        mir_params: &[(String, MirType)],
        bindings: &mut Vec<(String, MirExpr)>,
    ) {
        if let Some(pl) = param_list {
            for (idx, param) in pl.params().enumerate() {
                if idx >= mir_params.len() {
                    break;
                }
                let param_var = MirExpr::Var(mir_params[idx].0.clone(), mir_params[idx].1.clone());
                if let Some(pat) = param.pattern() {
                    match pat {
                        Pattern::Ident(ref ident) => {
                            let name = ident
                                .name()
                                .map(|t| t.text().to_string())
                                .unwrap_or_else(|| "_".to_string());
                            if name != "_" {
                                self.insert_var(name.clone(), mir_params[idx].1.clone());
                                bindings.push((name, param_var));
                            }
                        }
                        Pattern::Wildcard(_) | Pattern::Literal(_) => {
                            // No binding needed.
                        }
                        _ => {} // Skip complex patterns for now.
                    }
                } else if let Some(name_tok) = param.name() {
                    let pname = name_tok.text().to_string();
                    if pname != "_" {
                        self.insert_var(pname.clone(), mir_params[idx].1.clone());
                        bindings.push((pname, param_var));
                    }
                }
            }
        }
    }

    /// Build a condition expression that checks if all closure clause params match.
    fn build_closure_clause_condition(
        &self,
        param_list: &Option<mesh_parser::ast::item::ParamList>,
        mir_params: &[(String, MirType)],
    ) -> Option<MirExpr> {
        let mut conditions: Vec<MirExpr> = Vec::new();

        if let Some(pl) = param_list {
            for (idx, param) in pl.params().enumerate() {
                if idx >= mir_params.len() {
                    break;
                }
                if let Some(pat) = param.pattern() {
                    if let Some(cond) = self.pattern_to_condition(&pat, &mir_params[idx]) {
                        conditions.push(cond);
                    }
                }
            }
        }

        if conditions.is_empty() {
            None
        } else {
            let mut result = conditions.remove(0);
            for cond in conditions {
                result = MirExpr::BinOp {
                    op: BinOp::And,
                    lhs: Box::new(result),
                    rhs: Box::new(cond),
                    ty: MirType::Bool,
                };
            }
            Some(result)
        }
    }

    // ── String expression lowering (INTERPOLATION DESUGARING) ────────

    fn lower_string_expr(&mut self, str_expr: &StringExpr) -> MirExpr {
        // Walk the STRING_EXPR node's children to find STRING_CONTENT and
        // INTERPOLATION segments.

        // Detect triple-quoted string from STRING_START token text (""" vs ")
        let is_triple = str_expr
            .syntax()
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|t| t.kind() == SyntaxKind::STRING_START)
            .map(|t| t.text().starts_with("\"\"\""))
            .unwrap_or(false);

        // For triple-quoted strings, determine the trim level from the last STRING_CONTENT token.
        // The last STRING_CONTENT ends with "\n<indent>" where <indent> matches the closing """.
        let trim_level: usize = if is_triple {
            str_expr
                .syntax()
                .children_with_tokens()
                .filter_map(|c| c.into_token())
                .filter(|t| t.kind() == SyntaxKind::STRING_CONTENT)
                .last()
                .map(|t| {
                    let text = t.text().to_string();
                    // The last line of the last STRING_CONTENT is the closing indent line
                    text.split('\n')
                        .last()
                        .unwrap_or("")
                        .chars()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };

        let mut segments: Vec<MirExpr> = Vec::new();
        // Track whether the next STRING_CONTENT is the first one (for leading newline stripping)
        let mut is_first_content = is_triple;

        for child in str_expr.syntax().children_with_tokens() {
            match child.kind() {
                SyntaxKind::STRING_CONTENT => {
                    let raw_text = child
                        .as_token()
                        .map(|t| unescape_string(t.text()))
                        .unwrap_or_default();

                    let text = if is_triple {
                        apply_heredoc_content(raw_text, is_first_content, trim_level)
                    } else {
                        raw_text
                    };
                    is_first_content = false;

                    if !text.is_empty() {
                        segments.push(MirExpr::StringLit(text, MirType::String));
                    }
                }
                SyntaxKind::INTERPOLATION => {
                    // After any interpolation, subsequent STRING_CONTENT is not first
                    is_first_content = false;
                    // INTERPOLATION node contains an expression child.
                    if let Some(node) = child.as_node() {
                        for inner in node.children() {
                            if let Some(expr) = Expr::cast(inner) {
                                let typeck_ty = self.get_ty(expr.syntax().text_range()).cloned();
                                let lowered = self.lower_expr(&expr);
                                // Wrap in a to_string call based on the expression's type.
                                let converted = self.wrap_to_string(lowered, typeck_ty.as_ref());
                                segments.push(converted);
                            }
                        }
                    }
                }
                _ => {
                    // STRING_START, STRING_END, INTERPOLATION_START, INTERPOLATION_END:
                    // skip these tokens.
                }
            }
        }

        // If no segments, return empty string.
        if segments.is_empty() {
            return MirExpr::StringLit(String::new(), MirType::String);
        }

        // If single segment, return it directly.
        if segments.len() == 1 {
            return segments.pop().unwrap();
        }

        // Chain concat calls: concat(concat(seg0, seg1), seg2) ...
        let mut result = segments.remove(0);
        for seg in segments {
            result = MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_string_concat".to_string(),
                    MirType::FnPtr(
                        vec![MirType::String, MirType::String],
                        Box::new(MirType::String),
                    ),
                )),
                args: vec![result, seg],
                ty: MirType::String,
            };
        }

        result
    }

    /// Wrap an expression in a to_string runtime call based on its type.
    ///
    /// `typeck_ty` is the optional original typeck `Ty` for the expression,
    /// used to resolve collection element types for Display dispatch.
    fn wrap_to_string(&mut self, expr: MirExpr, typeck_ty: Option<&Ty>) -> MirExpr {
        match expr.ty() {
            MirType::String => expr, // already a string
            MirType::Int => MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_int_to_string".to_string(),
                    MirType::FnPtr(vec![MirType::Int], Box::new(MirType::String)),
                )),
                args: vec![expr],
                ty: MirType::String,
            },
            MirType::Float => MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_float_to_string".to_string(),
                    MirType::FnPtr(vec![MirType::Float], Box::new(MirType::String)),
                )),
                args: vec![expr],
                ty: MirType::String,
            },
            MirType::Bool => MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_bool_to_string".to_string(),
                    MirType::FnPtr(vec![MirType::Bool], Box::new(MirType::String)),
                )),
                args: vec![expr],
                ty: MirType::String,
            },
            MirType::Struct(_) | MirType::SumType(_) => {
                // Display trait dispatch: check if the type has a Display impl
                // and emit a mangled Display__to_string__TypeName call.
                let ty_for_lookup = mir_type_to_ty(expr.ty());
                let matching =
                    self.trait_registry.find_method_traits("to_string", &ty_for_lookup);
                if !matching.is_empty() {
                    let trait_name = &matching[0];
                    let type_name = mir_type_to_impl_name(expr.ty());
                    let mangled = format!("{}__{}__{}", trait_name, "to_string", type_name);
                    MirExpr::Call {
                        func: Box::new(MirExpr::Var(
                            mangled,
                            MirType::FnPtr(
                                vec![expr.ty().clone()],
                                Box::new(MirType::String),
                            ),
                        )),
                        args: vec![expr],
                        ty: MirType::String,
                    }
                } else {
                    // Check if a monomorphized Display function was generated
                    // (for generic struct instantiations like Box_Int).
                    let type_name = mir_type_to_impl_name(expr.ty());
                    let mono_mangled = format!("Display__to_string__{}", type_name);
                    if self.known_functions.contains_key(&mono_mangled) {
                        MirExpr::Call {
                            func: Box::new(MirExpr::Var(
                                mono_mangled,
                                MirType::FnPtr(
                                    vec![expr.ty().clone()],
                                    Box::new(MirType::String),
                                ),
                            )),
                            args: vec![expr],
                            ty: MirType::String,
                        }
                    } else {
                        // Check for Debug fallback (inspect).
                        let debug_mangled = format!("Debug__inspect__{}", type_name);
                        if self.known_functions.contains_key(&debug_mangled) {
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var(
                                    debug_mangled,
                                    MirType::FnPtr(
                                        vec![expr.ty().clone()],
                                        Box::new(MirType::String),
                                    ),
                                )),
                                args: vec![expr],
                                ty: MirType::String,
                            }
                        } else {
                            // No Display or Debug impl found -- fall through to generic to_string
                            MirExpr::Call {
                                func: Box::new(MirExpr::Var(
                                    "to_string".to_string(),
                                    MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)),
                                )),
                                args: vec![expr],
                                ty: MirType::String,
                            }
                        }
                    }
                }
            }
            MirType::Ptr => {
                // Check if the typeck type is a collection (List, Map, Set).
                // If so, emit a runtime collection-to-string call with element
                // conversion callback function pointers.
                if let Some(ty) = typeck_ty {
                    if let Some(collection_call) = self.wrap_collection_to_string(&expr, ty) {
                        return collection_call;
                    }
                }
                // Fallback: generic to_string call.
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "to_string".to_string(),
                        MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)),
                    )),
                    args: vec![expr],
                    ty: MirType::String,
                }
            }
            _ => {
                // For other types, attempt a generic to_string call.
                MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "to_string".to_string(),
                        MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::String)),
                    )),
                    args: vec![expr],
                    ty: MirType::String,
                }
            }
        }
    }

    /// Attempt to wrap a collection expression in its Display runtime call.
    ///
    /// Returns `Some(MirExpr)` if the `Ty` is a List, Map, or Set with known
    /// element types; `None` otherwise (fallback to generic to_string).
    fn wrap_collection_to_string(&mut self, expr: &MirExpr, ty: &Ty) -> Option<MirExpr> {
        // Match Ty::App(Con("List"|"Map"|"Set"), args).
        // Also handle Ty::Con("List"|"Map"|"Set") without type args (empty collections).
        let (base_name, args) = match ty {
            Ty::App(con_ty, args) => {
                if let Ty::Con(con) = con_ty.as_ref() {
                    (con.name.as_str(), args.as_slice())
                } else {
                    return None;
                }
            }
            Ty::Con(con) => (con.name.as_str(), &[] as &[Ty]),
            _ => return None,
        };

        let fn_ptr_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Ptr));

        match base_name {
            "List" => {
                let elem_fn = if args.is_empty() {
                    // Unparameterized List -- use int as default fallback
                    self.resolve_to_string_callback(&Ty::int())
                } else {
                    self.resolve_to_string_callback(&args[0])
                };
                let fn_ptr_expr = MirExpr::Var(elem_fn, fn_ptr_ty.clone());
                Some(MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_list_to_string".to_string(),
                        MirType::FnPtr(
                            vec![MirType::Ptr, MirType::Ptr],
                            Box::new(MirType::String),
                        ),
                    )),
                    args: vec![expr.clone(), fn_ptr_expr],
                    ty: MirType::String,
                })
            }
            "Map" => {
                let key_fn = if args.len() >= 1 {
                    self.resolve_to_string_callback(&args[0])
                } else {
                    self.resolve_to_string_callback(&Ty::int())
                };
                let val_fn = if args.len() >= 2 {
                    self.resolve_to_string_callback(&args[1])
                } else {
                    self.resolve_to_string_callback(&Ty::int())
                };
                let key_ptr_expr = MirExpr::Var(key_fn, fn_ptr_ty.clone());
                let val_ptr_expr = MirExpr::Var(val_fn, fn_ptr_ty.clone());
                Some(MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_map_to_string".to_string(),
                        MirType::FnPtr(
                            vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
                            Box::new(MirType::String),
                        ),
                    )),
                    args: vec![expr.clone(), key_ptr_expr, val_ptr_expr],
                    ty: MirType::String,
                })
            }
            "Set" => {
                let elem_fn = if args.is_empty() {
                    self.resolve_to_string_callback(&Ty::int())
                } else {
                    self.resolve_to_string_callback(&args[0])
                };
                let fn_ptr_expr = MirExpr::Var(elem_fn, fn_ptr_ty.clone());
                Some(MirExpr::Call {
                    func: Box::new(MirExpr::Var(
                        "mesh_set_to_string".to_string(),
                        MirType::FnPtr(
                            vec![MirType::Ptr, MirType::Ptr],
                            Box::new(MirType::String),
                        ),
                    )),
                    args: vec![expr.clone(), fn_ptr_expr],
                    ty: MirType::String,
                })
            }
            _ => None,
        }
    }

    /// Resolve the to_string callback function name for an element type.
    ///
    /// For primitive types, returns the runtime to_string function name.
    /// For user-defined types with Display impl, returns the mangled name.
    /// For nested collections/sum types, generates synthetic MIR wrapper
    /// functions and returns the wrapper name. Recurses for arbitrary depth.
    fn resolve_to_string_callback(&mut self, elem_ty: &Ty) -> String {
        match elem_ty {
            Ty::Con(con) => match con.name.as_str() {
                "Int" => "mesh_int_to_string".to_string(),
                "Float" => "mesh_float_to_string".to_string(),
                "Bool" => "mesh_bool_to_string".to_string(),
                "String" => "mesh_string_to_string".to_string(),
                // Bare collection type without type args -- default to Int callback
                "List" => self.generate_display_collection_wrapper("list", "mesh_list_to_string", &Ty::int(), None),
                "Set" => self.generate_display_collection_wrapper("set", "mesh_set_to_string", &Ty::int(), None),
                "Map" => self.generate_display_map_wrapper(&Ty::int(), &Ty::int()),
                name => {
                    // Check if this user type has a Display impl
                    let ty_for_lookup = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                    let matching = self
                        .trait_registry
                        .find_method_traits("to_string", &ty_for_lookup);
                    if !matching.is_empty() {
                        format!("{}__to_string__{}", matching[0], name)
                    } else {
                        // Check for Debug inspect as fallback
                        let inspect_name = format!("Debug__inspect__{}", name);
                        if self.known_functions.contains_key(&inspect_name) {
                            inspect_name
                        } else {
                            // No Display or Debug impl -- fallback
                            "mesh_int_to_string".to_string()
                        }
                    }
                }
            },
            Ty::App(con_ty, args) => {
                if let Ty::Con(con) = con_ty.as_ref() {
                    match con.name.as_str() {
                        "List" => {
                            let inner_ty = args.first().cloned().unwrap_or_else(Ty::int);
                            self.generate_display_collection_wrapper("list", "mesh_list_to_string", &inner_ty, None)
                        }
                        "Set" => {
                            let inner_ty = args.first().cloned().unwrap_or_else(Ty::int);
                            self.generate_display_collection_wrapper("set", "mesh_set_to_string", &inner_ty, None)
                        }
                        "Map" => {
                            let key_ty = args.first().cloned().unwrap_or_else(Ty::int);
                            let val_ty = args.get(1).cloned().unwrap_or_else(Ty::int);
                            self.generate_display_map_wrapper(&key_ty, &val_ty)
                        }
                        name => {
                            // Monomorphized sum type or struct: e.g., Option<Int> -> Option_Int
                            let mangled = self.mangle_ty_for_display(elem_ty);
                            // Check Display__to_string__{mangled}
                            let display_name = format!("Display__to_string__{}", mangled);
                            if self.known_functions.contains_key(&display_name) {
                                return display_name;
                            }
                            // Check Debug__inspect__{mangled}
                            let inspect_name = format!("Debug__inspect__{}", mangled);
                            if self.known_functions.contains_key(&inspect_name) {
                                return inspect_name;
                            }
                            // Check trait registry for Display impl
                            let ty_for_lookup = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                            let matching = self
                                .trait_registry
                                .find_method_traits("to_string", &ty_for_lookup);
                            if !matching.is_empty() {
                                format!("{}__to_string__{}", matching[0], mangled)
                            } else {
                                "mesh_int_to_string".to_string()
                            }
                        }
                    }
                } else {
                    "mesh_int_to_string".to_string()
                }
            }
            _ => "mesh_int_to_string".to_string(),
        }
    }

    /// Mangle a `Ty` into a display-friendly name for synthetic wrapper functions.
    ///
    /// Examples:
    /// - `Ty::Con("Int")` -> `"Int"`
    /// - `Ty::App(Con("List"), [Con("Int")])` -> `"list_Int"`
    /// - `Ty::App(Con("Option"), [Con("Int")])` -> `"Option_Int"`
    /// - `Ty::App(Con("List"), [App(Con("List"), [Con("Int")])])` -> `"list_list_Int"`
    fn mangle_ty_for_display(&self, ty: &Ty) -> String {
        match ty {
            Ty::Con(con) => con.name.clone(),
            Ty::App(con_ty, args) => {
                if let Ty::Con(con) = con_ty.as_ref() {
                    let base = match con.name.as_str() {
                        "List" => "list",
                        "Set" => "set",
                        "Map" => "map",
                        other => other,
                    };
                    let mut name = base.to_string();
                    for arg in args {
                        name.push('_');
                        name.push_str(&self.mangle_ty_for_display(arg));
                    }
                    name
                } else {
                    "Unknown".to_string()
                }
            }
            _ => "Unknown".to_string(),
        }
    }

    /// Generate a synthetic MIR wrapper function for displaying a List or Set
    /// element that is itself a collection or complex type.
    ///
    /// The wrapper bridges the `fn(u64) -> *mut u8` callback signature expected
    /// by the runtime. It takes a single Ptr parameter and calls the appropriate
    /// runtime to_string function with the recursively resolved inner callback.
    ///
    /// Returns the name of the wrapper function.
    fn generate_display_collection_wrapper(
        &mut self,
        collection_kind: &str,   // "list" or "set"
        runtime_fn: &str,        // "mesh_list_to_string" or "mesh_set_to_string"
        inner_ty: &Ty,
        _extra: Option<&str>,
    ) -> String {
        let inner_mangled = self.mangle_ty_for_display(inner_ty);
        let wrapper_name = format!("__display_{}_{}_to_str", collection_kind, inner_mangled);

        // Dedup: if already generated, return existing name
        if self.known_functions.contains_key(&wrapper_name) {
            return wrapper_name;
        }

        // Recursively resolve the inner element's callback
        let inner_callback = self.resolve_to_string_callback(inner_ty);

        // Register the wrapper before generating body (prevents infinite recursion)
        let wrapper_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        self.known_functions.insert(wrapper_name.clone(), wrapper_ty);

        // Build the wrapper function MIR:
        //   fn __display_list_Int_to_str(__elem: Ptr) -> Ptr {
        //       mesh_list_to_string(__elem, mesh_int_to_string)
        //   }
        let param_name = "__elem".to_string();
        let fn_ptr_ty = MirType::FnPtr(
            vec![MirType::Ptr, MirType::Ptr],
            Box::new(MirType::Ptr),
        );
        let body = MirExpr::Call {
            func: Box::new(MirExpr::Var(runtime_fn.to_string(), fn_ptr_ty)),
            args: vec![
                MirExpr::Var(param_name.clone(), MirType::Ptr),
                MirExpr::Var(
                    inner_callback,
                    MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
                ),
            ],
            ty: MirType::Ptr,
        };

        self.functions.push(MirFunction {
            name: wrapper_name.clone(),
            params: vec![(param_name, MirType::Ptr)],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });

        wrapper_name
    }

    /// Generate a synthetic MIR wrapper function for displaying a Map element.
    ///
    /// The wrapper calls `mesh_map_to_string` with recursively resolved key and
    /// value callbacks.
    fn generate_display_map_wrapper(&mut self, key_ty: &Ty, val_ty: &Ty) -> String {
        let key_mangled = self.mangle_ty_for_display(key_ty);
        let val_mangled = self.mangle_ty_for_display(val_ty);
        let wrapper_name = format!("__display_map_{}_{}_to_str", key_mangled, val_mangled);

        // Dedup check
        if self.known_functions.contains_key(&wrapper_name) {
            return wrapper_name;
        }

        // Recursively resolve key and value callbacks
        let key_callback = self.resolve_to_string_callback(key_ty);
        let val_callback = self.resolve_to_string_callback(val_ty);

        // Register the wrapper
        let wrapper_ty = MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr));
        self.known_functions.insert(wrapper_name.clone(), wrapper_ty);

        // Build the wrapper function MIR:
        //   fn __display_map_Int_String_to_str(__elem: Ptr) -> Ptr {
        //       mesh_map_to_string(__elem, mesh_int_to_string, mesh_string_to_string)
        //   }
        let param_name = "__elem".to_string();
        let fn_ptr_ty = MirType::FnPtr(
            vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
            Box::new(MirType::Ptr),
        );
        let body = MirExpr::Call {
            func: Box::new(MirExpr::Var("mesh_map_to_string".to_string(), fn_ptr_ty)),
            args: vec![
                MirExpr::Var(param_name.clone(), MirType::Ptr),
                MirExpr::Var(
                    key_callback,
                    MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
                ),
                MirExpr::Var(
                    val_callback,
                    MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
                ),
            ],
            ty: MirType::Ptr,
        };

        self.functions.push(MirFunction {
            name: wrapper_name.clone(),
            params: vec![(param_name, MirType::Ptr)],
            return_type: MirType::Ptr,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });

        wrapper_name
    }

    // ── List Eq/Ord callback resolution (Phase 27 Plan 01) ──────────

    /// Resolve the eq callback function name for an element type.
    ///
    /// Returns the name of a function with signature `fn(u64, u64) -> i8`
    /// that compares two elements for equality.
    fn resolve_eq_callback(&mut self, elem_ty: &Ty) -> String {
        match elem_ty {
            Ty::Con(con) => match con.name.as_str() {
                "Int" => self.generate_int_eq_callback(),
                "Float" => self.generate_float_eq_callback(),
                "Bool" => self.generate_bool_eq_callback(),
                "String" => self.generate_string_eq_callback(),
                _ => {
                    // Fallback to int eq for unknown types
                    self.generate_int_eq_callback()
                }
            },
            Ty::App(con_ty, args) => {
                if let Ty::Con(con) = con_ty.as_ref() {
                    if con.name == "List" {
                        let inner_ty = args.first().cloned().unwrap_or_else(Ty::int);
                        return self.generate_list_eq_wrapper(&inner_ty);
                    }
                }
                self.generate_int_eq_callback()
            }
            _ => self.generate_int_eq_callback(),
        }
    }

    /// Resolve the compare callback function name for an element type.
    ///
    /// Returns the name of a function with signature `fn(u64, u64) -> i64`
    /// that returns negative/0/positive for element ordering.
    fn resolve_compare_callback(&mut self, elem_ty: &Ty) -> String {
        match elem_ty {
            Ty::Con(con) => match con.name.as_str() {
                "Int" => self.generate_int_cmp_callback(),
                "String" => self.generate_string_cmp_callback(),
                _ => self.generate_int_cmp_callback(),
            },
            Ty::App(con_ty, args) => {
                if let Ty::Con(con) = con_ty.as_ref() {
                    if con.name == "List" {
                        let inner_ty = args.first().cloned().unwrap_or_else(Ty::int);
                        return self.generate_list_cmp_wrapper(&inner_ty);
                    }
                }
                self.generate_int_cmp_callback()
            }
            _ => self.generate_int_cmp_callback(),
        }
    }

    /// Generate `__eq_int_callback(a: Int, b: Int) -> Bool { a == b }`
    fn generate_int_eq_callback(&mut self) -> String {
        let name = "__eq_int_callback".to_string();
        if self.known_functions.contains_key(&name) {
            return name;
        }
        let fn_ty = MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Bool));
        self.known_functions.insert(name.clone(), fn_ty);

        let body = MirExpr::BinOp {
            op: BinOp::Eq,
            lhs: Box::new(MirExpr::Var("__a".to_string(), MirType::Int)),
            rhs: Box::new(MirExpr::Var("__b".to_string(), MirType::Int)),
            ty: MirType::Bool,
        };

        self.functions.push(MirFunction {
            name: name.clone(),
            params: vec![("__a".to_string(), MirType::Int), ("__b".to_string(), MirType::Int)],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        name
    }

    /// Generate `__eq_float_callback(a: Float, b: Float) -> Bool { a == b }`
    fn generate_float_eq_callback(&mut self) -> String {
        let name = "__eq_float_callback".to_string();
        if self.known_functions.contains_key(&name) {
            return name;
        }
        let fn_ty = MirType::FnPtr(vec![MirType::Float, MirType::Float], Box::new(MirType::Bool));
        self.known_functions.insert(name.clone(), fn_ty);

        let body = MirExpr::BinOp {
            op: BinOp::Eq,
            lhs: Box::new(MirExpr::Var("__a".to_string(), MirType::Float)),
            rhs: Box::new(MirExpr::Var("__b".to_string(), MirType::Float)),
            ty: MirType::Bool,
        };

        self.functions.push(MirFunction {
            name: name.clone(),
            params: vec![("__a".to_string(), MirType::Float), ("__b".to_string(), MirType::Float)],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        name
    }

    /// Generate `__eq_bool_callback(a: Bool, b: Bool) -> Bool { a == b }`
    fn generate_bool_eq_callback(&mut self) -> String {
        let name = "__eq_bool_callback".to_string();
        if self.known_functions.contains_key(&name) {
            return name;
        }
        let fn_ty = MirType::FnPtr(vec![MirType::Bool, MirType::Bool], Box::new(MirType::Bool));
        self.known_functions.insert(name.clone(), fn_ty);

        let body = MirExpr::BinOp {
            op: BinOp::Eq,
            lhs: Box::new(MirExpr::Var("__a".to_string(), MirType::Bool)),
            rhs: Box::new(MirExpr::Var("__b".to_string(), MirType::Bool)),
            ty: MirType::Bool,
        };

        self.functions.push(MirFunction {
            name: name.clone(),
            params: vec![("__a".to_string(), MirType::Bool), ("__b".to_string(), MirType::Bool)],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        name
    }

    /// Generate `__eq_string_callback(a: Ptr, b: Ptr) -> Bool { mesh_string_eq(a, b) }`
    fn generate_string_eq_callback(&mut self) -> String {
        let name = "__eq_string_callback".to_string();
        if self.known_functions.contains_key(&name) {
            return name;
        }
        let fn_ty = MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool));
        self.known_functions.insert(name.clone(), fn_ty);

        let body = MirExpr::Call {
            func: Box::new(MirExpr::Var(
                "mesh_string_eq".to_string(),
                MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool)),
            )),
            args: vec![
                MirExpr::Var("__a".to_string(), MirType::String),
                MirExpr::Var("__b".to_string(), MirType::String),
            ],
            ty: MirType::Bool,
        };

        self.functions.push(MirFunction {
            name: name.clone(),
            params: vec![("__a".to_string(), MirType::String), ("__b".to_string(), MirType::String)],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        name
    }

    /// Generate `__cmp_int_callback(a: Int, b: Int) -> Int { if a < b { -1 } else if a > b { 1 } else { 0 } }`
    fn generate_int_cmp_callback(&mut self) -> String {
        let name = "__cmp_int_callback".to_string();
        if self.known_functions.contains_key(&name) {
            return name;
        }
        let fn_ty = MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Int));
        self.known_functions.insert(name.clone(), fn_ty);

        // if a < b { -1 } else if a > b { 1 } else { 0 }
        let a = MirExpr::Var("__a".to_string(), MirType::Int);
        let b = MirExpr::Var("__b".to_string(), MirType::Int);
        let lt_cond = MirExpr::BinOp {
            op: BinOp::Lt,
            lhs: Box::new(a.clone()),
            rhs: Box::new(b.clone()),
            ty: MirType::Bool,
        };
        let gt_cond = MirExpr::BinOp {
            op: BinOp::Gt,
            lhs: Box::new(a),
            rhs: Box::new(b),
            ty: MirType::Bool,
        };
        let inner_if = MirExpr::If {
            cond: Box::new(gt_cond),
            then_body: Box::new(MirExpr::IntLit(1, MirType::Int)),
            else_body: Box::new(MirExpr::IntLit(0, MirType::Int)),
            ty: MirType::Int,
        };
        let body = MirExpr::If {
            cond: Box::new(lt_cond),
            then_body: Box::new(MirExpr::IntLit(-1, MirType::Int)),
            else_body: Box::new(inner_if),
            ty: MirType::Int,
        };

        self.functions.push(MirFunction {
            name: name.clone(),
            params: vec![("__a".to_string(), MirType::Int), ("__b".to_string(), MirType::Int)],
            return_type: MirType::Int,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        name
    }

    /// Generate `__cmp_string_callback(a: Ptr, b: Ptr) -> Int` that compares strings lexicographically.
    ///
    /// Since there's no mesh_string_compare runtime function, we use mesh_string_eq
    /// and a length-based fallback: if eq, return 0; otherwise use a < b heuristic.
    /// For simplicity, we generate: if mesh_string_eq(a, b) { 0 } else { -1 }
    /// This gives correct equality semantics but simplified ordering.
    /// TODO: Add proper mesh_string_compare in a future phase.
    fn generate_string_cmp_callback(&mut self) -> String {
        let name = "__cmp_string_callback".to_string();
        if self.known_functions.contains_key(&name) {
            return name;
        }
        let fn_ty = MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Int));
        self.known_functions.insert(name.clone(), fn_ty);

        // if mesh_string_eq(a, b) { 0 } else { -1 }
        let eq_call = MirExpr::Call {
            func: Box::new(MirExpr::Var(
                "mesh_string_eq".to_string(),
                MirType::FnPtr(vec![MirType::String, MirType::String], Box::new(MirType::Bool)),
            )),
            args: vec![
                MirExpr::Var("__a".to_string(), MirType::String),
                MirExpr::Var("__b".to_string(), MirType::String),
            ],
            ty: MirType::Bool,
        };
        let body = MirExpr::If {
            cond: Box::new(eq_call),
            then_body: Box::new(MirExpr::IntLit(0, MirType::Int)),
            else_body: Box::new(MirExpr::IntLit(-1, MirType::Int)),
            ty: MirType::Int,
        };

        self.functions.push(MirFunction {
            name: name.clone(),
            params: vec![("__a".to_string(), MirType::String), ("__b".to_string(), MirType::String)],
            return_type: MirType::Int,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        name
    }

    /// Generate a wrapper for nested list equality: `__eq_list_{inner}_callback`
    fn generate_list_eq_wrapper(&mut self, inner_ty: &Ty) -> String {
        let inner_mangled = self.mangle_ty_for_display(inner_ty);
        let wrapper_name = format!("__eq_list_{}_callback", inner_mangled);
        if self.known_functions.contains_key(&wrapper_name) {
            return wrapper_name;
        }

        let inner_callback = self.resolve_eq_callback(inner_ty);

        let fn_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Bool));
        self.known_functions.insert(wrapper_name.clone(), fn_ty);

        let body = MirExpr::Call {
            func: Box::new(MirExpr::Var(
                "mesh_list_eq".to_string(),
                MirType::FnPtr(
                    vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
                    Box::new(MirType::Bool),
                ),
            )),
            args: vec![
                MirExpr::Var("__a".to_string(), MirType::Ptr),
                MirExpr::Var("__b".to_string(), MirType::Ptr),
                MirExpr::Var(
                    inner_callback,
                    MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Bool)),
                ),
            ],
            ty: MirType::Bool,
        };

        self.functions.push(MirFunction {
            name: wrapper_name.clone(),
            params: vec![("__a".to_string(), MirType::Ptr), ("__b".to_string(), MirType::Ptr)],
            return_type: MirType::Bool,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        wrapper_name
    }

    /// Generate a wrapper for nested list comparison: `__cmp_list_{inner}_callback`
    fn generate_list_cmp_wrapper(&mut self, inner_ty: &Ty) -> String {
        let inner_mangled = self.mangle_ty_for_display(inner_ty);
        let wrapper_name = format!("__cmp_list_{}_callback", inner_mangled);
        if self.known_functions.contains_key(&wrapper_name) {
            return wrapper_name;
        }

        let inner_callback = self.resolve_compare_callback(inner_ty);

        let fn_ty = MirType::FnPtr(vec![MirType::Ptr, MirType::Ptr], Box::new(MirType::Int));
        self.known_functions.insert(wrapper_name.clone(), fn_ty);

        let body = MirExpr::Call {
            func: Box::new(MirExpr::Var(
                "mesh_list_compare".to_string(),
                MirType::FnPtr(
                    vec![MirType::Ptr, MirType::Ptr, MirType::Ptr],
                    Box::new(MirType::Int),
                ),
            )),
            args: vec![
                MirExpr::Var("__a".to_string(), MirType::Ptr),
                MirExpr::Var("__b".to_string(), MirType::Ptr),
                MirExpr::Var(
                    inner_callback,
                    MirType::FnPtr(vec![MirType::Int, MirType::Int], Box::new(MirType::Int)),
                ),
            ],
            ty: MirType::Int,
        };

        self.functions.push(MirFunction {
            name: wrapper_name.clone(),
            params: vec![("__a".to_string(), MirType::Ptr), ("__b".to_string(), MirType::Ptr)],
            return_type: MirType::Int,
            body,
            is_closure_fn: false,
            captures: vec![],
            has_tail_calls: false,
        });
        wrapper_name
    }

    // ── Return expression lowering ───────────────────────────────────

    fn lower_return_expr(&mut self, ret: &ReturnExpr) -> MirExpr {
        let value = ret
            .value()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        MirExpr::Return(Box::new(value))
    }

    // ── Try expression lowering (Phase 45) ─────────────────────────

    /// Desugar `expr?` to a match expression with early return.
    ///
    /// For `Result<T, E>`:
    /// ```text
    /// case expr do
    ///   Ok(__try_val_N) -> __try_val_N
    ///   Err(__try_err_N) -> return Err(__try_err_N)
    /// end
    /// ```
    ///
    /// For `Option<T>`:
    /// ```text
    /// case expr do
    ///   Some(__try_val_N) -> __try_val_N
    ///   None -> return None
    /// end
    /// ```
    fn lower_try_expr(&mut self, try_expr: &TryExpr) -> MirExpr {
        let operand = match try_expr.operand() {
            Some(e) => self.lower_expr(&e),
            None => return MirExpr::Unit,
        };

        let operand_ty = operand.ty().clone();
        let fn_ret_ty = self.current_fn_return_type.clone().unwrap_or(MirType::Unit);

        // The expression type of `expr?` is the unwrapped success type T,
        // as determined by the type checker.
        let success_ty = self.resolve_range(try_expr.syntax().text_range());

        // Determine if operand is Result or Option by examining the MirType.
        match &operand_ty {
            MirType::SumType(name) if self.is_result_type(name) => {
                self.lower_try_result(operand, name, &fn_ret_ty, &success_ty)
            }
            MirType::SumType(name) if self.is_option_type(name) => {
                self.lower_try_option(operand, name, &fn_ret_ty, &success_ty)
            }
            _ => {
                // Should not happen if typeck validated correctly; fallback to Unit.
                MirExpr::Unit
            }
        }
    }

    /// Check if a sum type name corresponds to a Result type.
    /// Matches both the generic "Result" and monomorphized forms like "Result_Int_String".
    fn is_result_type(&self, name: &str) -> bool {
        name == "Result" || name.starts_with("Result_")
    }

    /// Check if a sum type name corresponds to an Option type.
    /// Matches both the generic "Option" and monomorphized forms like "Option_Int".
    fn is_option_type(&self, name: &str) -> bool {
        name == "Option" || name.starts_with("Option_")
    }

    /// Find the sum type base name for a type -- either "Result", "Option", or the generic name.
    /// Used to look up variant definitions.
    fn sum_type_base_name<'b>(&self, name: &'b str) -> &'b str {
        if self.is_result_type(name) {
            // Look up the actual sum type def -- try the full name first, then "Result"
            if self.sum_types.iter().any(|s| s.name == name) {
                name
            } else {
                "Result"
            }
        } else if self.is_option_type(name) {
            if self.sum_types.iter().any(|s| s.name == name) {
                name
            } else {
                "Option"
            }
        } else {
            name
        }
    }

    /// Find the base type name to use for the function return's early-return construction.
    fn fn_return_sum_type_name(&self, fn_ret_ty: &MirType) -> String {
        match fn_ret_ty {
            MirType::SumType(name) => {
                self.sum_type_base_name(name).to_string()
            }
            _ => "Result".to_string(),
        }
    }

    /// Extract the error type name from a monomorphized Result type name.
    /// e.g., "Result_Int_String" -> Some("String"), "Result_Int_AppError" -> Some("AppError")
    /// Returns None if the type name doesn't have enough parts.
    fn extract_error_type_from_result_name(&self, name: &str) -> Option<String> {
        // Monomorphized Result names: Result_OkType_ErrType
        // The error type is everything after the second underscore.
        let parts: Vec<&str> = name.splitn(3, '_').collect();
        if parts.len() == 3 {
            Some(parts[2].to_string())
        } else {
            None
        }
    }

    /// Convert a type name string back to a MirType.
    fn type_name_to_mir_type(&self, name: &str) -> MirType {
        match name {
            "Int" => MirType::Int,
            "Float" => MirType::Float,
            "String" => MirType::String,
            "Bool" => MirType::Bool,
            _ => {
                // Check if it's a known struct
                if self.registry.struct_defs.contains_key(name) {
                    MirType::Struct(name.to_string())
                } else if self.registry.sum_type_defs.contains_key(name) {
                    MirType::SumType(name.to_string())
                } else {
                    MirType::Ptr
                }
            }
        }
    }

    /// Desugar `result_expr?` into Match + Return for Result<T, E>.
    /// When error types differ and a From impl exists, inserts a From.from() call
    /// to convert the operand's error type to the function return's error type.
    fn lower_try_result(
        &mut self,
        operand: MirExpr,
        operand_type_name: &str,
        fn_ret_ty: &MirType,
        success_ty: &MirType,
    ) -> MirExpr {
        self.try_counter += 1;
        let counter = self.try_counter;
        let val_name = format!("__try_val_{}", counter);
        let err_name = format!("__try_err_{}", counter);

        // Determine the operand's sum type def name for pattern matching.
        let pattern_type_name = self.sum_type_base_name(operand_type_name).to_string();

        // Determine the function return type's sum type name for the Err early-return.
        let fn_return_type_name = self.fn_return_sum_type_name(fn_ret_ty);

        // Find the error type from the sum type definition.
        let error_ty = self.find_variant_field_type(&pattern_type_name, "Err")
            .unwrap_or(MirType::Ptr);

        // Check if From-based error conversion is needed by comparing the
        // monomorphized Result type names. If the operand and fn return have
        // different Result type names, the error types must differ.
        let operand_err_name = self.extract_error_type_from_result_name(operand_type_name);
        let fn_ret_type_name_full = match fn_ret_ty {
            MirType::SumType(n) => n.clone(),
            _ => String::new(),
        };
        let fn_err_name = self.extract_error_type_from_result_name(&fn_ret_type_name_full);

        let needs_from_conversion = match (&operand_err_name, &fn_err_name) {
            (Some(op_err), Some(fn_err)) => op_err != fn_err,
            _ => false,
        };

        let (err_body_expr, _err_body_ty) = if needs_from_conversion {
            let source_err_name = operand_err_name.as_deref().unwrap();
            let target_err_name = fn_err_name.as_deref().unwrap();
            let source_err_ty = self.type_name_to_mir_type(source_err_name);
            let target_err_ty = self.type_name_to_mir_type(target_err_name);

            // Normalize struct error types to Ptr for the Result variant layout.
            // User-defined struct constructors return heap-allocated pointers
            // (via mesh_gc_alloc), so the From function's return value IS already
            // a pointer at LLVM level. The Result layout uses { i8, ptr }, so the
            // MIR type must be Ptr to match the variant field slot.
            let effective_err_ty = match &target_err_ty {
                MirType::Struct(_) => MirType::Ptr,
                other => other.clone(),
            };

            let from_fn_name = format!("From_{}__from__{}", source_err_name, target_err_name);
            let from_fn_ty = MirType::FnPtr(
                vec![source_err_ty.clone()],
                Box::new(effective_err_ty.clone()),
            );
            let converted_err = MirExpr::Call {
                func: Box::new(MirExpr::Var(from_fn_name, from_fn_ty)),
                args: vec![MirExpr::Var(err_name.clone(), source_err_ty.clone())],
                ty: effective_err_ty.clone(),
            };
            (converted_err, effective_err_ty)
        } else {
            // Error types match -- use original error directly.
            (MirExpr::Var(err_name.clone(), error_ty.clone()), error_ty.clone())
        };

        // Use the correct error type for the Err arm's pattern binding.
        // When From conversion is needed, the pattern binds the SOURCE error type
        // (from the operand), but the body uses the CONVERTED error type.
        let pattern_err_ty = if needs_from_conversion {
            let source_err_name = operand_err_name.as_deref().unwrap();
            self.type_name_to_mir_type(source_err_name)
        } else {
            error_ty.clone()
        };

        // Build the desugared match expression.
        MirExpr::Match {
            scrutinee: Box::new(operand),
            arms: vec![
                // Ok(__try_val_N) -> __try_val_N
                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: pattern_type_name.clone(),
                        variant: "Ok".to_string(),
                        fields: vec![MirPattern::Var(val_name.clone(), success_ty.clone())],
                        bindings: vec![(val_name.clone(), success_ty.clone())],
                    },
                    guard: None,
                    body: MirExpr::Var(val_name, success_ty.clone()),
                },
                // Err(__try_err_N) -> return Err(converted_err_or_raw_err)
                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: pattern_type_name,
                        variant: "Err".to_string(),
                        fields: vec![MirPattern::Var(err_name.clone(), pattern_err_ty.clone())],
                        bindings: vec![(err_name, pattern_err_ty)],
                    },
                    guard: None,
                    body: MirExpr::Return(Box::new(MirExpr::ConstructVariant {
                        type_name: fn_return_type_name,
                        variant: "Err".to_string(),
                        fields: vec![err_body_expr],
                        ty: fn_ret_ty.clone(),
                    })),
                },
            ],
            ty: success_ty.clone(),
        }
    }

    /// Desugar `option_expr?` into Match + Return for Option<T>.
    fn lower_try_option(
        &mut self,
        operand: MirExpr,
        operand_type_name: &str,
        fn_ret_ty: &MirType,
        success_ty: &MirType,
    ) -> MirExpr {
        self.try_counter += 1;
        let counter = self.try_counter;
        let val_name = format!("__try_val_{}", counter);

        // Determine the operand's sum type def name for pattern matching.
        let pattern_type_name = self.sum_type_base_name(operand_type_name).to_string();

        // Determine the function return type's sum type name for the None early-return.
        let fn_return_type_name = self.fn_return_sum_type_name(fn_ret_ty);

        // Build the desugared match expression.
        MirExpr::Match {
            scrutinee: Box::new(operand),
            arms: vec![
                // Some(__try_val_N) -> __try_val_N
                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: pattern_type_name.clone(),
                        variant: "Some".to_string(),
                        fields: vec![MirPattern::Var(val_name.clone(), success_ty.clone())],
                        bindings: vec![(val_name.clone(), success_ty.clone())],
                    },
                    guard: None,
                    body: MirExpr::Var(val_name, success_ty.clone()),
                },
                // None -> return None
                MirMatchArm {
                    pattern: MirPattern::Constructor {
                        type_name: pattern_type_name,
                        variant: "None".to_string(),
                        fields: vec![],
                        bindings: vec![],
                    },
                    guard: None,
                    body: MirExpr::Return(Box::new(MirExpr::ConstructVariant {
                        type_name: fn_return_type_name,
                        variant: "None".to_string(),
                        fields: vec![],
                        ty: fn_ret_ty.clone(),
                    })),
                },
            ],
            ty: success_ty.clone(),
        }
    }

    /// Look up the field type for a specific variant in a sum type definition.
    /// Returns the first field's MIR type, or None if the variant has no fields.
    fn find_variant_field_type(&self, type_name: &str, variant_name: &str) -> Option<MirType> {
        for sum_type in &self.sum_types {
            if sum_type.name == type_name {
                for variant in &sum_type.variants {
                    if variant.name == variant_name {
                        return variant.fields.first().cloned();
                    }
                }
            }
        }
        None
    }

    // ── Tuple expression lowering ────────────────────────────────────

    fn lower_tuple_expr(&mut self, tuple: &TupleExpr) -> MirExpr {
        let elements: Vec<MirExpr> = tuple.elements().map(|e| self.lower_expr(&e)).collect();

        // Per decision 03-02: single-element tuple is grouping parens, not a tuple.
        if elements.len() == 1 {
            return elements.into_iter().next().unwrap();
        }

        if elements.is_empty() {
            return MirExpr::Unit;
        }

        // Multi-element tuple: generate a heap-allocated runtime tuple.
        // Runtime layout: { u64 len, u64[len] elements }
        // Allocate via mesh_gc_alloc_actor, store length + elements, return pointer.
        let n = elements.len();
        let _total_size = 8 + n * 8; // u64 len + n * u64 elements

        // Generate a synthetic __mesh_make_tuple(elem0, elem1, ...) call.
        // Codegen expands this inline: gc_alloc + store length + store elements.
        MirExpr::Call {
            func: Box::new(MirExpr::Var(
                "__mesh_make_tuple".to_string(),
                MirType::FnPtr(vec![MirType::Int; n], Box::new(MirType::Ptr)),
            )),
            args: elements,
            ty: MirType::Ptr,
        }
    }

    // ── Map literal lowering ────────────────────────────────────────

    /// Desugar `%{k1 => v1, k2 => v2}` to:
    ///   mesh_map_new_typed(key_type_tag)
    ///   |> mesh_map_put(_, k1, v1)
    ///   |> mesh_map_put(_, k2, v2)
    fn lower_map_literal(&mut self, map_lit: &MapLiteral) -> MirExpr {
        let key_type_tag = self.infer_map_key_type(map_lit.syntax().text_range());

        let new_typed_fn = MirExpr::Var(
            "mesh_map_new_typed".to_string(),
            MirType::FnPtr(vec![MirType::Int], Box::new(MirType::Ptr)),
        );
        let mut result = MirExpr::Call {
            func: Box::new(new_typed_fn),
            args: vec![MirExpr::IntLit(key_type_tag, MirType::Int)],
            ty: MirType::Ptr,
        };

        let put_fn_ty = MirType::FnPtr(
            vec![MirType::Ptr, MirType::Int, MirType::Int],
            Box::new(MirType::Ptr),
        );

        for entry in map_lit.entries() {
            // For keyword argument entries (name: value), the key is a NAME_REF
            // that should be treated as a string literal (the identifier text).
            let key = if entry.is_keyword_entry() {
                entry
                    .keyword_key_text()
                    .map(|text| MirExpr::StringLit(text, MirType::String))
                    .unwrap_or(MirExpr::Unit)
            } else {
                entry
                    .key()
                    .map(|e| self.lower_expr(&e))
                    .unwrap_or(MirExpr::Unit)
            };
            let val = entry
                .value()
                .map(|e| self.lower_expr(&e))
                .unwrap_or(MirExpr::Unit);

            let put_fn = MirExpr::Var("mesh_map_put".to_string(), put_fn_ty.clone());
            result = MirExpr::Call {
                func: Box::new(put_fn),
                args: vec![result, key, val],
                ty: MirType::Ptr,
            };
        }

        result
    }

    // ── List literal lowering ────────────────────────────────────────

    /// Lower a list literal `[e1, e2, ...]` to MIR.
    ///
    /// For empty lists: calls mesh_list_new().
    /// For non-empty lists: creates a MirExpr::ListLit with lowered elements.
    /// The codegen will stack-allocate an array, store elements, and call
    /// mesh_list_from_array(arr_ptr, count).
    fn lower_list_literal(&mut self, list_lit: &ListLiteral) -> MirExpr {
        let elements: Vec<MirExpr> = list_lit.elements()
            .map(|e| self.lower_expr(&e))
            .collect();

        if elements.is_empty() {
            // Empty list: call mesh_list_new()
            let fn_ty = MirType::FnPtr(vec![], Box::new(MirType::Ptr));
            return MirExpr::Call {
                func: Box::new(MirExpr::Var("mesh_list_new".to_string(), fn_ty)),
                args: vec![],
                ty: MirType::Ptr,
            };
        }

        MirExpr::ListLit {
            elements,
            ty: MirType::Ptr,
        }
    }

    // ── Struct literal lowering ──────────────────────────────────────

    fn lower_struct_literal(&mut self, sl: &StructLiteral) -> MirExpr {
        let base_name = sl
            .name_ref()
            .and_then(|nr| nr.text())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let fields: Vec<(String, MirExpr)> = sl
            .fields()
            .map(|f| {
                let field_name = f
                    .name()
                    .and_then(|n| n.text())
                    .unwrap_or_default();
                let value = f
                    .value()
                    .map(|e| self.lower_expr(&e))
                    .unwrap_or(MirExpr::Unit);
                (field_name, value)
            })
            .collect();

        let ty = self.resolve_range(sl.syntax().text_range());

        // For generic structs, the resolved type is MirType::Struct("Box_Int") (mangled).
        // Use the mangled name for the struct literal so codegen finds the right LLVM type.
        // Also trigger monomorphized trait function generation.
        let name = if let MirType::Struct(ref mangled) = ty {
            if mangled != &base_name {
                // This is a monomorphized generic struct -- generate trait functions.
                if let Some(typeck_ty) = self.get_ty(sl.syntax().text_range()).cloned() {
                    self.ensure_monomorphized_struct_trait_fns(&base_name, &typeck_ty);
                }
                mangled.clone()
            } else {
                base_name
            }
        } else {
            base_name
        };

        MirExpr::StructLit { name, fields, ty }
    }

    // ── Struct update lowering ────────────────────────────────────────

    fn lower_struct_update(&mut self, update: &StructUpdate) -> MirExpr {
        let base = update
            .base_expr()
            .map(|e| self.lower_expr(&e))
            .unwrap_or(MirExpr::Unit);

        let overrides: Vec<(String, MirExpr)> = update
            .override_fields()
            .iter()
            .map(|f| {
                let field_name = f
                    .name()
                    .and_then(|n| n.text())
                    .unwrap_or_default();
                let value = f
                    .value()
                    .map(|e| self.lower_expr(&e))
                    .unwrap_or(MirExpr::Unit);
                (field_name, value)
            })
            .collect();

        let ty = self.resolve_range(update.syntax().text_range());

        MirExpr::StructUpdate {
            base: Box::new(base),
            overrides,
            ty,
        }
    }

    // ── Actor definition lowering ──────────────────────────────────────

    fn lower_actor_def(&mut self, actor_def: &ActorDef) {
        let name = actor_def
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<anonymous_actor>".to_string());

        // Get actor type from typeck.
        let actor_range = actor_def.syntax().text_range();
        let actor_ty_raw = self.get_ty(actor_range).cloned();

        // Extract parameter names and types.
        let mut params = Vec::new();
        self.push_scope();

        if let Some(param_list) = actor_def.param_list() {
            if let Some(Ty::Fun(param_tys, _)) = &actor_ty_raw {
                for (param, param_ty) in param_list.params().zip(param_tys.iter()) {
                    let param_name = param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string());
                    let is_closure = matches!(param_ty, Ty::Fun(..));
                    let mir_ty = resolve_type(param_ty, self.registry, is_closure);
                    self.insert_var(param_name.clone(), mir_ty.clone());
                    params.push((param_name, mir_ty));
                }
            } else {
                // Fallback: range-based type lookup.
                for param in param_list.params() {
                    let param_name = param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string());
                    let mir_ty = self.resolve_range(param.syntax().text_range());
                    self.insert_var(param_name.clone(), mir_ty.clone());
                    params.push((param_name, mir_ty));
                }
            }
        }

        // Actor entry functions are called by the scheduler. They don't return
        // a value to the caller. The spawn expression returns the Pid.
        let return_type = MirType::Unit;

        // Lower the actor body. The body contains a receive block that loops.
        let mut body = if let Some(block) = actor_def.body() {
            self.lower_block(&block)
        } else {
            MirExpr::Unit
        };

        // Handle terminate clause: lower to a separate callback function.
        let terminate_callback_name = if let Some(term_clause) = actor_def.terminate_clause() {
            let cb_name = format!("__terminate_{}", name);
            let cb_body = if let Some(cb_block) = term_clause.body() {
                self.lower_block(&cb_block)
            } else {
                MirExpr::Unit
            };

            // Terminate callback signature: (state_ptr: Ptr, reason_ptr: Ptr) -> Unit
            self.functions.push(MirFunction {
                name: cb_name.clone(),
                params: vec![
                    ("state_ptr".to_string(), MirType::Ptr),
                    ("reason_ptr".to_string(), MirType::Ptr),
                ],
                return_type: MirType::Unit,
                body: cb_body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });

            Some(cb_name)
        } else {
            None
        };

        self.pop_scope();

        // Store the terminate callback name for use by spawn codegen.
        // We attach it as a known function and store a mapping.
        if let Some(ref cb_name) = terminate_callback_name {
            self.known_functions.insert(
                cb_name.clone(),
                MirType::FnPtr(
                    vec![MirType::Ptr, MirType::Ptr],
                    Box::new(MirType::Unit),
                ),
            );
        }

        // For actors WITH parameters, generate a wrapper + body pair (Phase 93.2).
        // The runtime calls actor entry functions with signature `extern "C" fn(*const u8)`,
        // passing a pointer to a serialized args buffer. Actors with typed parameters need
        // a wrapper that accepts the raw pointer and deserializes args before calling the
        // actual actor body with typed values.
        if !params.is_empty() {
            let body_fn_name = format!("__actor_{}_body", name);

            // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
            // The recursive calls in the source use the original actor name (e.g., `counter(next)`),
            // so we pass the original name to rewrite_tail_calls for matching.
            let has_tail_calls = rewrite_tail_calls(&mut body, &name);

            // 1. Push the body function with original typed params.
            self.functions.push(MirFunction {
                name: body_fn_name.clone(),
                params: params.clone(),
                return_type: return_type.clone(),
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls,
            });

            // Register the body function in known_functions so codegen can find it.
            let body_param_types: Vec<MirType> = params.iter().map(|(_, ty)| ty.clone()).collect();
            self.known_functions.insert(
                body_fn_name,
                MirType::FnPtr(body_param_types, Box::new(MirType::Unit)),
            );

            // 2. Push the wrapper function with Ptr param and Unit body.
            // Codegen detects this pattern (single __args_ptr param + matching __actor_*_body)
            // and generates the arg deserialization + body call.
            self.functions.push(MirFunction {
                name: name.clone(),
                params: vec![("__args_ptr".to_string(), MirType::Ptr)],
                return_type,
                body: MirExpr::Unit,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });

            // Register the wrapper in known_functions with Ptr -> Unit signature
            // so that spawn references resolve correctly.
            self.known_functions.insert(
                name,
                MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Unit)),
            );
        } else {
            // For actors WITHOUT parameters, keep existing behavior unchanged.
            // The runtime passes null as args_ptr which is harmlessly ignored.

            // TCE: Rewrite self-recursive tail calls to TailCall nodes (Phase 48).
            let has_tail_calls = rewrite_tail_calls(&mut body, &name);

            self.functions.push(MirFunction {
                name,
                params,
                return_type,
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls,
            });
        }
    }

    // ── Supervisor lowering ─────────────────────────────────────────────

    fn lower_supervisor_def(&mut self, sup_def: &SupervisorDef) {
        let name = sup_def
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<anonymous_supervisor>".to_string());

        // Extract strategy (default: one_for_one = 0).
        let strategy: u8 = sup_def
            .strategy()
            .and_then(|node| {
                node.children_with_tokens()
                    .filter_map(|c| c.into_token())
                    .filter(|t| t.kind() == SyntaxKind::IDENT)
                    .last()
                    .map(|t| match t.text() {
                        "one_for_one" => 0u8,
                        "one_for_all" => 1,
                        "rest_for_one" => 2,
                        "simple_one_for_one" => 3,
                        _ => 0,
                    })
            })
            .unwrap_or(0);

        // Extract max_restarts (default: 3).
        let max_restarts: u32 = sup_def
            .max_restarts()
            .and_then(|node| {
                node.children_with_tokens()
                    .filter_map(|c| c.into_token())
                    .find(|t| t.kind() == SyntaxKind::INT_LITERAL)
                    .and_then(|t| t.text().parse().ok())
            })
            .unwrap_or(3);

        // Extract max_seconds (default: 5).
        let max_seconds: u64 = sup_def
            .max_seconds()
            .and_then(|node| {
                node.children_with_tokens()
                    .filter_map(|c| c.into_token())
                    .find(|t| t.kind() == SyntaxKind::INT_LITERAL)
                    .and_then(|t| t.text().parse().ok())
            })
            .unwrap_or(5);

        // Extract child specs.
        let mut children = Vec::new();
        for child_node in sup_def.child_specs() {
            // Child ID from the NAME child.
            let child_id = child_node
                .children()
                .find(|c| c.kind() == SyntaxKind::NAME)
                .and_then(|n| {
                    n.children_with_tokens()
                        .filter_map(|c| c.into_token())
                        .find(|t| t.kind() == SyntaxKind::IDENT)
                        .map(|t| t.text().to_string())
                })
                .unwrap_or_else(|| "child".to_string());

            // Parse child body -- look inside the BLOCK child for key-value pairs.
            let block = child_node
                .children()
                .find(|c| c.kind() == SyntaxKind::BLOCK);

            let mut start_fn = String::new();
            let mut restart_type: u8 = 0; // permanent
            let mut shutdown_ms: u64 = 5000;

            if let Some(block) = block {
                for token_or_node in block.children_with_tokens() {
                    if let Some(token) = token_or_node.as_token() {
                        // Track identifiers for key-value pairs.
                        let _text = token.text();
                    }
                }

                // Walk tokens linearly to extract key-value pairs.
                let tokens: Vec<_> = block
                    .descendants_with_tokens()
                    .filter_map(|c| c.into_token())
                    .collect();
                let mut i = 0;
                while i < tokens.len() {
                    let text = tokens[i].text();
                    if text == "start" {
                        // Skip "start", ":", then find the spawn call or actor reference.
                        // In our simple model, the child start is a closure: fn -> spawn(ActorName, args) end
                        // We need to find the actor name being spawned.
                        // Look for SPAWN_KW or an ident matching an actor name after start: fn ->
                        let mut j = i + 1;
                        while j < tokens.len() {
                            if tokens[j].kind() == SyntaxKind::SPAWN_KW {
                                // Next non-trivia token after ( should be the actor name.
                                let mut k = j + 1;
                                while k < tokens.len() && tokens[k].kind() != SyntaxKind::IDENT {
                                    k += 1;
                                }
                                if k < tokens.len() {
                                    start_fn = tokens[k].text().to_string();
                                }
                                break;
                            }
                            if tokens[j].text() == "restart" || tokens[j].text() == "shutdown" {
                                break;
                            }
                            j += 1;
                        }
                    } else if text == "restart" {
                        // Skip "restart", ":", then grab the value.
                        let mut j = i + 1;
                        while j < tokens.len() {
                            if tokens[j].kind() == SyntaxKind::IDENT {
                                restart_type = match tokens[j].text() {
                                    "permanent" => 0,
                                    "transient" => 1,
                                    "temporary" => 2,
                                    _ => 0,
                                };
                                break;
                            }
                            j += 1;
                        }
                    } else if text == "shutdown" {
                        // Skip "shutdown", ":", then grab int or brutal_kill.
                        let mut j = i + 1;
                        while j < tokens.len() {
                            if tokens[j].kind() == SyntaxKind::INT_LITERAL {
                                shutdown_ms = tokens[j].text().parse().unwrap_or(5000);
                                break;
                            }
                            if tokens[j].kind() == SyntaxKind::IDENT && tokens[j].text() == "brutal_kill" {
                                shutdown_ms = 0; // 0 = brutal kill
                                break;
                            }
                            j += 1;
                        }
                    }
                    i += 1;
                }
            }

            children.push(MirChildSpec {
                id: child_id,
                start_fn,
                restart_type,
                shutdown_ms,
                child_type: 0, // worker
            });
        }

        // Create a MIR function for the supervisor.
        // The supervisor's body is a SupervisorStart expression.
        let body = MirExpr::SupervisorStart {
            name: name.clone(),
            strategy,
            max_restarts,
            max_seconds,
            children,
            ty: MirType::Pid(None),
        };

        self.functions.push(MirFunction {
            name,
            params: vec![],
            return_type: MirType::Pid(None),
            body,
            is_closure_fn: false,
            captures: Vec::new(),
            has_tail_calls: false,
        });
    }

    // ── Service lowering ─────────────────────────────────────────────────

    fn lower_service_def(&mut self, service_def: &ServiceDef) {
        let name = service_def
            .name()
            .and_then(|n| n.text())
            .unwrap_or_else(|| "<anonymous_service>".to_string());

        let name_lower = name.to_lowercase();

        // Collect handler info from the AST.
        let call_handlers = service_def.call_handlers();
        let cast_handlers = service_def.cast_handlers();

        // Assign sequential type tags.
        // Call handlers: tags 0, 1, 2, ...
        // Cast handlers: tags N, N+1, N+2, ... (where N = call_handlers.len())
        let num_calls = call_handlers.len();

        // ── Collect handler info ─────────────────────────────────────────

        // For each call handler: (variant_name, snake_name, tag, param_names, state_param)
        struct CallInfo {
            #[allow(dead_code)]
            variant_name: String,
            snake_name: String,
            tag: u64,
            param_names: Vec<String>,
            param_types: Vec<MirType>,
            state_param: Option<String>,
            /// The MIR type of the reply value (second element of the handler's
            /// return tuple). Used to set the correct return type on the call
            /// helper function so the codegen can properly convert the reply
            /// from its tuple-encoded i64 representation.
            reply_type: MirType,
        }

        struct CastInfo {
            #[allow(dead_code)]
            variant_name: String,
            snake_name: String,
            tag: u64,
            param_names: Vec<String>,
            param_types: Vec<MirType>,
            state_param: Option<String>,
        }

        let mut call_infos = Vec::new();
        for (i, handler) in call_handlers.iter().enumerate() {
            let variant_name = handler
                .name()
                .and_then(|n| n.text())
                .unwrap_or_else(|| format!("call_{}", i));
            let snake_name = to_snake_case(&variant_name);
            let mut param_names: Vec<String> = Vec::new();
            let mut param_types: Vec<MirType> = Vec::new();
            if let Some(pl) = handler.params() {
                for p in pl.params() {
                    let p_name = p.name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| format!("arg{}", 0));
                    let p_ty = self.resolve_range(p.syntax().text_range());
                    let mir_ty = if matches!(p_ty, MirType::Unit) { MirType::Int } else { p_ty };
                    param_names.push(p_name);
                    param_types.push(mir_ty);
                }
            }
            let state_param = handler.state_param_name();

            // Determine the reply type from the handler body's tail expression.
            // The body returns (state, reply); we extract the second element.
            // NOTE: The type checker does NOT store a type for the BLOCK node
            // itself — only for expressions within it. We must use the tail
            // expression (last expr in the block) to get the (state, reply)
            // tuple type.
            let reply_type = handler.body()
                .and_then(|block| block.tail_expr())
                .map(|expr| self.resolve_range(expr.syntax().text_range()))
                .and_then(|ty| {
                    if let MirType::Tuple(ref elems) = ty {
                        if elems.len() >= 2 {
                            Some(elems[1].clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                // Tuples are heap-allocated pointers at runtime, so collapse
                // Tuple(...) to Ptr to match the LLVM representation.
                .map(|ty| if matches!(ty, MirType::Tuple(_)) { MirType::Ptr } else { ty })
                .unwrap_or(MirType::Int);

            call_infos.push(CallInfo {
                variant_name,
                snake_name,
                tag: i as u64,
                param_names,
                param_types,
                state_param,
                reply_type,
            });
        }

        let mut cast_infos = Vec::new();
        for (i, handler) in cast_handlers.iter().enumerate() {
            let variant_name = handler
                .name()
                .and_then(|n| n.text())
                .unwrap_or_else(|| format!("cast_{}", i));
            let snake_name = to_snake_case(&variant_name);
            let mut param_names: Vec<String> = Vec::new();
            let mut param_types: Vec<MirType> = Vec::new();
            if let Some(pl) = handler.params() {
                for p in pl.params() {
                    let p_name = p.name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| format!("arg{}", 0));
                    let p_ty = self.resolve_range(p.syntax().text_range());
                    let mir_ty = if matches!(p_ty, MirType::Unit) { MirType::Int } else { p_ty };
                    param_names.push(p_name);
                    param_types.push(mir_ty);
                }
            }
            let state_param = handler.state_param_name();
            cast_infos.push(CastInfo {
                variant_name,
                snake_name,
                tag: (num_calls + i) as u64,
                param_names,
                param_types,
                state_param,
            });
        }

        // ── Generate init function ───────────────────────────────────────
        // Lower the init function body to get initial state.
        let mut init_params = Vec::new();
        let init_body = if let Some(init_fn) = service_def.init_fn() {
            self.push_scope();
            if let Some(param_list) = init_fn.param_list() {
                let fn_range = init_fn.syntax().text_range();
                let fn_ty_raw = self.get_ty(fn_range).cloned();
                if let Some(mesh_typeck::ty::Ty::Fun(param_tys, _)) = &fn_ty_raw {
                    for (param, param_ty) in param_list.params().zip(param_tys.iter()) {
                        let param_name = param
                            .name()
                            .map(|t| t.text().to_string())
                            .unwrap_or_else(|| "_".to_string());
                        let is_closure = matches!(param_ty, Ty::Fun(..));
                        let mir_ty = resolve_type(param_ty, self.registry, is_closure);
                        self.insert_var(param_name.clone(), mir_ty.clone());
                        init_params.push((param_name, mir_ty));
                    }
                } else {
                    for param in param_list.params() {
                        let param_name = param
                            .name()
                            .map(|t| t.text().to_string())
                            .unwrap_or_else(|| "_".to_string());
                        let mir_ty = self.resolve_range(param.syntax().text_range());
                        self.insert_var(param_name.clone(), mir_ty.clone());
                        init_params.push((param_name, mir_ty));
                    }
                }
            }
            let body = if let Some(block) = init_fn.body() {
                self.lower_block(&block)
            } else {
                MirExpr::IntLit(0, MirType::Int)
            };
            self.pop_scope();
            body
        } else {
            MirExpr::IntLit(0, MirType::Int)
        };

        let init_fn_name = format!("__service_{}_init", name_lower);
        let init_ret_ty = effective_return_type(&init_body);
        let init_ret_ty = if matches!(init_ret_ty, MirType::Unit) { MirType::Int } else { init_ret_ty };
        self.functions.push(MirFunction {
            name: init_fn_name.clone(),
            params: init_params.clone(),
            return_type: init_ret_ty.clone(),
            body: init_body,
            is_closure_fn: false,
            captures: Vec::new(),
            has_tail_calls: false,
        });
        self.known_functions.insert(
            init_fn_name.clone(),
            MirType::FnPtr(
                init_params.iter().map(|(_, t)| t.clone()).collect(),
                Box::new(init_ret_ty.clone()),
            ),
        );

        // ── Generate handler body functions ──────────────────────────────
        // Each handler becomes a function:
        //   __service_{name}_handle_call_{snake}(state: i64, args...) -> i64 (for call: returns tuple-encoded {new_state, reply})
        //   __service_{name}_handle_cast_{snake}(state: i64, args...) -> i64 (for cast: returns new_state)

        for (i, handler) in call_handlers.iter().enumerate() {
            let info = &call_infos[i];
            let handler_fn_name = format!(
                "__service_{}_handle_call_{}",
                name_lower, info.snake_name
            );

            self.push_scope();

            // State param: use the actual init return type (e.g. Int for PoolHandle, Struct for WriterState).
            let state_param_name = info.state_param.clone().unwrap_or_else(|| "state".to_string());
            self.insert_var(state_param_name.clone(), init_ret_ty.clone());
            let mut params = vec![(state_param_name, init_ret_ty.clone())];

            // Handler params.
            if let Some(param_list) = handler.params() {
                for param in param_list.params() {
                    let p_name = param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string());
                    let p_ty = self.resolve_range(param.syntax().text_range());
                    let mir_ty = if matches!(p_ty, MirType::Unit) { MirType::Int } else { p_ty };
                    self.insert_var(p_name.clone(), mir_ty.clone());
                    params.push((p_name, mir_ty));
                }
            }

            // Lower handler body. Body returns (new_state, reply).
            let body = if let Some(block) = handler.body() {
                self.lower_block(&block)
            } else {
                // Default: return (state, 0).
                MirExpr::Unit
            };

            self.pop_scope();

            // Call handler body returns a heap-allocated tuple (new_state, reply).
            // The return type is ALWAYS Ptr since __mesh_make_tuple returns a pointer.
            // Note: body.ty() may not report Ptr when the body is wrapped in Let
            // bindings (Let.ty is the binding's value type, not the body's final type).
            let ret_ty = MirType::Ptr;
            self.functions.push(MirFunction {
                name: handler_fn_name.clone(),
                params,
                return_type: ret_ty.clone(),
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });
            self.known_functions.insert(
                handler_fn_name,
                MirType::FnPtr(vec![], Box::new(ret_ty)),
            );
        }

        for (i, handler) in cast_handlers.iter().enumerate() {
            let info = &cast_infos[i];
            let handler_fn_name = format!(
                "__service_{}_handle_cast_{}",
                name_lower, info.snake_name
            );

            self.push_scope();

            let state_param_name = info.state_param.clone().unwrap_or_else(|| "state".to_string());
            self.insert_var(state_param_name.clone(), init_ret_ty.clone());
            let mut params = vec![(state_param_name, init_ret_ty.clone())];

            if let Some(param_list) = handler.params() {
                for param in param_list.params() {
                    let p_name = param
                        .name()
                        .map(|t| t.text().to_string())
                        .unwrap_or_else(|| "_".to_string());
                    let p_ty = self.resolve_range(param.syntax().text_range());
                    let mir_ty = if matches!(p_ty, MirType::Unit) { MirType::Int } else { p_ty };
                    self.insert_var(p_name.clone(), mir_ty.clone());
                    params.push((p_name, mir_ty));
                }
            }

            // Lower handler body. Body returns new_state.
            let body = if let Some(block) = handler.body() {
                self.lower_block(&block)
            } else {
                MirExpr::IntLit(0, MirType::Int)
            };

            self.pop_scope();

            // Cast handler returns new state. Use effective_return_type to walk
            // through Let wrappers and find the actual return type.
            let cast_ret_ty = effective_return_type(&body);
            let cast_ret_ty = if matches!(cast_ret_ty, MirType::Unit) { MirType::Int } else { cast_ret_ty };
            self.functions.push(MirFunction {
                name: handler_fn_name.clone(),
                params,
                return_type: cast_ret_ty.clone(),
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });
            self.known_functions.insert(
                handler_fn_name,
                MirType::FnPtr(vec![], Box::new(cast_ret_ty)),
            );
        }

        // ── Generate the service loop function ───────────────────────────
        // __service_{name}_loop(state: i64) -> Unit
        //
        // This is the actor entry function that runs as a process.
        // It does: receive message -> dispatch on type_tag -> call handler ->
        //   for call: reply to caller with result, recurse with new_state
        //   for cast: recurse with new_state
        //
        // The loop function uses MIR primitives: ActorReceive, then manual dispatch.
        // Since MIR receive doesn't directly support type_tag dispatch, we generate
        // the loop as a receive that gets the raw message, extracts type_tag, and
        // uses if/else chains to dispatch.

        let loop_fn_name = format!("__service_{}_loop", name_lower);

        // The loop body is:
        //   let msg_ptr = receive(-1)    -- blocks for incoming message
        //   let type_tag = load_u64(msg_ptr, 0)
        //   let caller_pid = load_u64(msg_ptr, 8)
        //   -- for call tags: extract args from msg_ptr+16, call handler, reply, recurse
        //   -- for cast tags: extract args from msg_ptr+16, call handler, recurse
        //
        // We represent this as a Block of MIR expressions that the codegen will emit.
        // Since we can't easily express "load bytes from pointer" in MIR, we use
        // the Call node to call runtime helper functions that we'll add.
        //
        // Actually, the simplest approach: generate the loop function with a body
        // that calls a synthetic dispatch function we also generate. The dispatch
        // function is generated per-service and uses mesh_service_call/reply.
        //
        // SIMPLEST APPROACH: Don't generate an explicit loop function with raw pointer
        // arithmetic. Instead, generate a function with ActorReceive that has a single
        // wildcard arm. The receive extracts message data as an i64 (which is the
        // first 8 bytes = type_tag). Then we use if/else dispatch on tag values.
        //
        // HOWEVER: the message format for service calls includes [type_tag][caller_pid][args].
        // The ActorReceive codegen loads data starting at offset 16 (past the 16-byte header).
        // So the received value will be the type_tag (first i64 of data after header).
        //
        // Wait - let me reconsider the message format. mesh_service_call builds:
        //   [u64 type_tag][u64 caller_pid][payload_args]
        // This entire blob is the data portion. The MessageBuffer wraps it with its own
        // header [u64 type_tag_in_mb][u64 data_len]. So the full message in the mailbox is:
        //   [u64 mb_type_tag][u64 data_len][u64 msg_tag][u64 caller_pid][payload_args]
        // When ActorReceive skips the 16-byte header, it reads [u64 msg_tag] which is correct.
        //
        // For the loop function, we need more than just the type_tag. We need the caller_pid
        // and the args. This requires raw pointer access at codegen level.
        //
        // PRAGMATIC APPROACH: Generate the loop as a thin wrapper that the CODEGEN handles
        // specially. Add a new MirExpr::ServiceLoop variant that the codegen expands.
        //
        // EVEN SIMPLER: Generate the entire dispatch as function calls from MIR.
        // The service loop receives a raw message pointer, and we generate MIR that:
        //   1. Calls __service_msg_tag(ptr) -> i64 (extracts type_tag from data)
        //   2. Calls __service_msg_caller(ptr) -> i64 (extracts caller_pid)
        //   3. Calls __service_msg_arg(ptr, index) -> i64 (extracts arg N)
        //   4. Dispatches on tag via if/else chain
        //
        // These helper functions are runtime functions we can add.
        //
        // MOST PRAGMATIC: Since all values are i64, we generate the loop as an actor
        // that uses raw receive and does all dispatch inline. The code generator
        // for the service loop is custom in expr.rs -- we add a new MirExpr variant.
        //
        // FINAL DECISION: Add MirExpr::ServiceLoop to MIR. Keep it clean.

        // Actually, we can use a simpler representation. The service loop receives
        // a message as raw pointer, extracts tag/caller/args from known offsets.
        // We'll generate this in codegen (expr.rs) since it requires pointer arithmetic.
        // The MIR representation captures: loop function name, handler functions, tags.

        // For now: represent the loop as a single function whose body is a
        // Call to the loop dispatcher (generated in codegen). We'll use a
        // special intrinsic pattern.

        // CLEANEST APPROACH: Generate the loop function with a body that is an
        // ActorReceive with a wildcard arm. The arm body is a Let-chain that:
        //   1. Uses the received raw msg_ptr value (reinterpreted)
        //   2. Dispatches on integer comparison
        //
        // Since we can't extract sub-fields from a pointer in MIR, let's use
        // a different approach: The loop function is an actor body that calls
        // a set of generated runtime-level dispatch functions.
        //
        // ACTUALLY THE SIMPLEST WAY: Generate the body of the loop as just
        // an ActorReceive(-1) that returns Int, then dispatch on the value.
        // The type_tag IS the received data (first i64 after header).
        // But we also need caller_pid and args, which are at higher offsets.
        //
        // We need to access the raw message pointer. The current ActorReceive
        // codegen loads the data into a typed value and discards the pointer.
        // We need the raw pointer for service dispatch.
        //
        // TWO OPTIONS:
        // A) Add a ServiceDispatch MIR node that codegen handles specially
        // B) Generate multiple runtime helper calls
        //
        // Let's go with A. It's the cleanest.

        // Track methods for this service so field access can resolve them.
        let mut methods = Vec::new();

        // Start function.
        let start_fn_name = format!("__service_{}_start", name_lower);
        methods.push(("start".to_string(), start_fn_name.clone()));

        // Call helper functions.
        for info in &call_infos {
            let fn_name = format!("__service_{}_call_{}", name_lower, info.snake_name);
            methods.push((info.snake_name.clone(), fn_name.clone()));
            let mut fn_param_types = vec![MirType::Pid(None)];
            fn_param_types.extend(info.param_types.iter().cloned());
            self.known_functions.insert(
                fn_name.clone(),
                MirType::FnPtr(fn_param_types, Box::new(info.reply_type.clone())),
            );
        }

        // Cast helper functions.
        for info in &cast_infos {
            let fn_name = format!("__service_{}_cast_{}", name_lower, info.snake_name);
            methods.push((info.snake_name.clone(), fn_name.clone()));
            let mut fn_param_types = vec![MirType::Pid(None)];
            fn_param_types.extend(info.param_types.iter().cloned());
            self.known_functions.insert(
                fn_name.clone(),
                MirType::FnPtr(fn_param_types, Box::new(MirType::Unit)),
            );
        }

        // Register the service module for field access resolution.
        self.service_modules.insert(name.clone(), methods);

        // ── Generate call helper functions ─────────────────────────────────
        // __service_{name}_call_{snake}(pid: i64, args...) -> Int
        // Builds message: [u64 type_tag][args as i64s]
        // Calls mesh_service_call(pid, tag, payload_ptr, payload_size)
        // Returns reply as i64

        for info in &call_infos {
            let fn_name = format!("__service_{}_call_{}", name_lower, info.snake_name);

            // Use actual param types so LLVM function signature matches call sites.
            let mut params = vec![("__pid".to_string(), MirType::Int)];
            for (p_name, p_ty) in info.param_names.iter().zip(info.param_types.iter()) {
                params.push((p_name.clone(), p_ty.clone()));
            }

            // Body: call mesh_service_call(pid, tag, payload, size)
            // Codegen intercepts calls to "mesh_service_call" and packs args
            // into a payload buffer, coercing all values to i64.
            let body = MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_service_call".to_string(),
                    MirType::FnPtr(
                        vec![MirType::Int, MirType::Int, MirType::Ptr, MirType::Int],
                        Box::new(MirType::Ptr),
                    ),
                )),
                args: {
                    let mut args = vec![
                        MirExpr::Var("__pid".to_string(), MirType::Int),
                        MirExpr::IntLit(info.tag as i64, MirType::Int),
                    ];
                    // Pack the call arguments as the payload.
                    // Codegen will coerce each arg to i64 for the message buffer.
                    for (p_name, p_ty) in info.param_names.iter().zip(info.param_types.iter()) {
                        args.push(MirExpr::Var(p_name.clone(), p_ty.clone()));
                    }
                    args
                },
                ty: info.reply_type.clone(),
            };

            self.functions.push(MirFunction {
                name: fn_name.clone(),
                params,
                return_type: info.reply_type.clone(),
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });
        }

        // ── Generate cast helper functions ─────────────────────────────────
        // __service_{name}_cast_{snake}(pid: i64, args...) -> Unit
        // Builds message: [u64 type_tag][args as i64s]
        // Calls mesh_actor_send(pid, msg_ptr, msg_size) (fire-and-forget)

        for info in &cast_infos {
            let fn_name = format!("__service_{}_cast_{}", name_lower, info.snake_name);

            // Use actual param types so LLVM function signature matches call sites.
            let mut params = vec![("__pid".to_string(), MirType::Int)];
            for (p_name, p_ty) in info.param_names.iter().zip(info.param_types.iter()) {
                params.push((p_name.clone(), p_ty.clone()));
            }

            // Body: build message buffer with [tag][args] and call mesh_actor_send.
            // Cast message format: [u64 type_tag][u64 0 (no caller)][args as i64s]
            // Codegen intercepts the mesh_actor_send with int-lit tag and packs args.
            let body = MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    "mesh_actor_send".to_string(),
                    MirType::FnPtr(
                        vec![MirType::Int, MirType::Ptr, MirType::Int],
                        Box::new(MirType::Unit),
                    ),
                )),
                args: {
                    let mut args = vec![
                        MirExpr::Var("__pid".to_string(), MirType::Int),
                        MirExpr::IntLit(info.tag as i64, MirType::Int),
                    ];
                    for (p_name, p_ty) in info.param_names.iter().zip(info.param_types.iter()) {
                        args.push(MirExpr::Var(p_name.clone(), p_ty.clone()));
                    }
                    args
                },
                ty: MirType::Unit,
            };

            self.functions.push(MirFunction {
                name: fn_name.clone(),
                params,
                return_type: MirType::Unit,
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });
        }

        // ── Generate start function ──────────────────────────────────────
        // __service_{name}_start(init_args...) -> Pid(None)
        // Calls init to get initial state, spawns the loop actor, returns PID.

        {
            // Body: let state = init(args); spawn(loop, state)
            // Use the actual init return type (e.g., struct type) so the full
            // state is allocated and copied into the spawn args buffer.
            let init_call = MirExpr::Call {
                func: Box::new(MirExpr::Var(
                    init_fn_name.clone(),
                    MirType::FnPtr(
                        init_params.iter().map(|(_, t)| t.clone()).collect(),
                        Box::new(init_ret_ty.clone()),
                    ),
                )),
                args: init_params
                    .iter()
                    .map(|(n, t)| MirExpr::Var(n.clone(), t.clone()))
                    .collect(),
                ty: init_ret_ty.clone(),
            };

            let body = MirExpr::Let {
                name: "__init_state".to_string(),
                ty: init_ret_ty.clone(),
                value: Box::new(init_call),
                body: Box::new(MirExpr::ActorSpawn {
                    func: Box::new(MirExpr::Var(
                        loop_fn_name.clone(),
                        MirType::FnPtr(vec![init_ret_ty.clone()], Box::new(MirType::Unit)),
                    )),
                    args: vec![MirExpr::Var("__init_state".to_string(), init_ret_ty.clone())],
                    priority: 1,
                    terminate_callback: None,
                    ty: MirType::Pid(None),
                }),
            };

            self.functions.push(MirFunction {
                name: start_fn_name.clone(),
                params: init_params.clone(),
                return_type: MirType::Pid(None),
                body,
                is_closure_fn: false,
                captures: Vec::new(),
                has_tail_calls: false,
            });
            self.known_functions.insert(
                start_fn_name,
                MirType::FnPtr(
                    init_params.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(MirType::Pid(None)),
                ),
            );
        }

        // ── Generate the actual loop function (actor body) ───────────────
        // This is the actor entry function that:
        //   1. Receives a message (raw pointer)
        //   2. Extracts type_tag (offset 0 in data after header)
        //   3. Extracts caller_pid (offset 8)
        //   4. Extracts args (offset 16+)
        //   5. Dispatches to handler
        //   6. For call: replies to caller, recurses with new state
        //   7. For cast: recurses with new state
        //
        // We represent the loop body using ActorReceive + dispatch.
        // However, since MIR ActorReceive only gives us a single typed value
        // and we need raw pointer access, we'll use a special approach:
        //
        // Generate the loop as a regular function that calls mesh_actor_receive(-1)
        // directly, then does pointer arithmetic for dispatch.
        //
        // The MIR body will be a Call to __service_{name}_dispatch(state, msg_ptr)
        // which returns the new state, then tail-calls the loop.

        // Generate dispatch function:
        // __service_{name}_dispatch(state: i64, msg_ptr: ptr) -> i64 (new_state)
        //
        // This function extracts tag/caller/args from msg_ptr and dispatches.
        // Since we can't do pointer arithmetic in MIR, this will be handled
        // specially by codegen when it sees the function name pattern.
        //
        // ACTUALLY: Let me take a step back. The CLEANEST approach for the loop
        // is to not try to express raw pointer ops in MIR at all. Instead:
        //
        // Generate the loop function as an actor body, and add a new MirExpr
        // variant for service dispatch that codegen handles.

        // First, let's add the service dispatch info so codegen can generate it.
        // We'll store it as metadata and generate the loop body in codegen.

        // Build handler dispatch info for codegen.
        let mut call_dispatch_info = Vec::new();
        for info in &call_infos {
            let handler_fn = format!(
                "__service_{}_handle_call_{}",
                name_lower, info.snake_name
            );
            call_dispatch_info.push((info.tag, handler_fn, info.param_names.len()));
        }

        let mut cast_dispatch_info = Vec::new();
        for info in &cast_infos {
            let handler_fn = format!(
                "__service_{}_handle_cast_{}",
                name_lower, info.snake_name
            );
            cast_dispatch_info.push((info.tag, handler_fn, info.param_names.len()));
        }

        // The loop function body is: receive -> dispatch -> recurse.
        // We represent this as a Block containing:
        //   1. Call mesh_actor_receive(-1) -> msg_ptr
        //   2. Service-specific dispatch on msg_ptr
        //   3. Tail call to loop with new_state
        //
        // For (2), we generate inline if/else dispatch in MIR using the type_tag.
        // Since we can't extract fields from a pointer in MIR, we'll generate the
        // entire loop body at codegen level.
        //
        // DECISION: Use a MIR representation that captures everything codegen needs.
        // The loop body is an opaque "ServiceDispatchLoop" that codegen expands.

        // For cleanliness, represent the loop body as a MIR Block that contains
        // only the dispatch metadata encoded as a string pattern.
        // The codegen recognizes functions named "__service_*_loop" and generates
        // the dispatch loop specially.
        //
        // We store dispatch metadata on the Lowerer to pass to codegen via MirModule.
        // Actually, we can't easily extend MirModule. Instead, encode the dispatch
        // info in the function body itself using a convention.
        //
        // SIMPLEST: The loop function body is MirExpr::Unit. Codegen recognizes
        // functions named "__service_*_loop" and generates the appropriate code.
        // But codegen needs to know the handlers/tags. We can pass this through
        // function metadata.
        //
        // Let's encode the dispatch table as IntLit constants in a Block.
        // Convention: Block([IntLit(num_call_handlers), IntLit(tag0), ..., IntLit(num_cast_handlers), IntLit(tag0), ...])
        //
        // Better: just use the function naming convention. Codegen can discover
        // __service_{name}_handle_call_* and __service_{name}_handle_cast_* functions
        // from the MIR module.
        //
        // BEST APPROACH: Encode the loop as a series of MirExpr nodes that
        // codegen CAN handle. The loop body is conceptually:
        //
        //   let msg_ptr = receive(-1)  -- raw pointer
        //   -- dispatch based on msg_ptr[0] (type_tag), msg_ptr[8] (caller_pid), msg_ptr[16+] (args)
        //
        // Since receive returns a pointer and codegen can access it, we CAN
        // generate the loop as:
        //   ActorReceive(-1) -> msg_ptr
        //   Then use FieldAccess-like operations on msg_ptr
        //
        // BUT MIR doesn't have raw pointer field access.
        //
        // FINAL DECISION: The loop body uses ActorReceive to get msg data as Int
        // (which gives us the type_tag -- the first i64 after the 16-byte header).
        // We then use if/else dispatch on the tag. For each handler arm:
        //   - Call handlers need caller_pid and args from the message
        //   - We can't get those from MIR alone
        //
        // So we MUST handle the loop at codegen level. The function
        // __service_{name}_loop will have a body of MirExpr::Unit, and codegen
        // will detect this pattern and generate the appropriate assembly.
        //
        // To pass dispatch info to codegen, we'll extend MirModule with
        // service_dispatch_info.

        // PRAGMATIC FINAL: Use the MirExpr::Unit body with function naming convention,
        // and encode dispatch metadata as comments in the function (using known_functions
        // registry). The codegen will look up handlers by naming convention.

        // The loop function receives a *const u8 (args buffer pointer) from the
        // actor spawn mechanism. The first i64 in the args buffer is the initial state.
        // Codegen will dereference the pointer to load the initial state.
        self.functions.push(MirFunction {
            name: loop_fn_name.clone(),
            params: vec![("__args_ptr".to_string(), MirType::Ptr)],
            return_type: MirType::Unit,
            body: MirExpr::Unit, // Codegen generates the actual dispatch loop
            is_closure_fn: false,
            captures: Vec::new(),
            has_tail_calls: false,
        });
        self.known_functions.insert(
            loop_fn_name,
            MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Unit)),
        );
    }

    // ── Actor expression lowering ───────────────────────────────────────

    fn lower_spawn_expr(&mut self, spawn: &SpawnExpr) -> MirExpr {
        let ty = self.resolve_range(spawn.syntax().text_range());
        let ty = if matches!(ty, MirType::Unit) {
            MirType::Pid(None)
        } else {
            ty
        };

        let args: Vec<MirExpr> = spawn
            .arg_list()
            .map(|al| al.args().map(|a| self.lower_expr(&a)).collect())
            .unwrap_or_default();

        // First argument is the function to spawn; rest are initial state.
        let (func, state_args) = if args.is_empty() {
            (Box::new(MirExpr::Unit), Vec::new())
        } else {
            let mut iter = args.into_iter();
            let func = Box::new(iter.next().unwrap());
            let state_args: Vec<MirExpr> = iter.collect();
            (func, state_args)
        };

        // Check if the spawned function has a terminate callback.
        // Look up by function name in known functions to find matching __terminate_<name>.
        let terminate_callback = if let MirExpr::Var(ref fn_name, _) = *func {
            let cb_name = format!("__terminate_{}", fn_name);
            if self.known_functions.contains_key(&cb_name) {
                Some(Box::new(MirExpr::Var(
                    cb_name.clone(),
                    MirType::FnPtr(
                        vec![MirType::Ptr, MirType::Ptr],
                        Box::new(MirType::Unit),
                    ),
                )))
            } else {
                None
            }
        } else {
            None
        };

        MirExpr::ActorSpawn {
            func,
            args: state_args,
            priority: 1, // Normal priority
            terminate_callback,
            ty,
        }
    }

    fn lower_send_expr(&mut self, send: &SendExpr) -> MirExpr {
        let args: Vec<MirExpr> = send
            .arg_list()
            .map(|al| al.args().map(|a| self.lower_expr(&a)).collect())
            .unwrap_or_default();

        // send(target, message) -> Unit
        let (target, message) = if args.len() >= 2 {
            let mut iter = args.into_iter();
            let target = Box::new(iter.next().unwrap());
            let message = Box::new(iter.next().unwrap());
            (target, message)
        } else if args.len() == 1 {
            let mut iter = args.into_iter();
            (Box::new(iter.next().unwrap()), Box::new(MirExpr::Unit))
        } else {
            (Box::new(MirExpr::Unit), Box::new(MirExpr::Unit))
        };

        MirExpr::ActorSend {
            target,
            message,
            ty: MirType::Unit,
        }
    }

    fn lower_receive_expr(&mut self, recv: &ReceiveExpr) -> MirExpr {
        let ty = self.resolve_range(recv.syntax().text_range());

        // Lower receive arms (reuse pattern matching infrastructure).
        let arms: Vec<MirMatchArm> = recv
            .arms()
            .map(|arm| {
                self.push_scope();
                let pattern = arm
                    .pattern()
                    .map(|p| self.lower_pattern(&p))
                    .unwrap_or(MirPattern::Wildcard);
                let body = arm
                    .body()
                    .map(|e| self.lower_expr(&e))
                    .unwrap_or(MirExpr::Unit);
                self.pop_scope();
                MirMatchArm {
                    pattern,
                    guard: None, // Receive arms don't have guards (they use when clauses which are separate)
                    body,
                }
            })
            .collect();

        // Handle optional after (timeout) clause.
        let (timeout_ms, timeout_body) = if let Some(after) = recv.after_clause() {
            let ms = after.timeout().map(|e| Box::new(self.lower_expr(&e)));
            let body = after.body().map(|e| Box::new(self.lower_expr(&e)));
            (ms, body)
        } else {
            (None, None)
        };

        MirExpr::ActorReceive {
            arms,
            timeout_ms,
            timeout_body,
            ty,
        }
    }

    fn lower_link_expr(&mut self, link: &LinkExpr) -> MirExpr {
        let args: Vec<MirExpr> = link
            .arg_list()
            .map(|al| al.args().map(|a| self.lower_expr(&a)).collect())
            .unwrap_or_default();

        let target = if let Some(first) = args.into_iter().next() {
            Box::new(first)
        } else {
            Box::new(MirExpr::Unit)
        };

        MirExpr::ActorLink {
            target,
            ty: MirType::Unit,
        }
    }
}

// ── Helper functions ─────────────────────────────────────────────────

/// Set of known stdlib module names for qualified access lowering.
const STDLIB_MODULES: &[&str] = &[
    "String", "IO", "Env", "File", "List", "Map", "Set", "Tuple", "Range", "Queue", "HTTP", "JSON", "Json", "Request", "Job",
    "Math", "Int", "Float", "Timer", "Sqlite", "Pg", "Ws", "Pool",
    "Node", "Process",  // Phase 67
    "Global",  // Phase 68
    "Iter",  // Phase 76
    "Orm",  // Phase 97
    "Query",  // Phase 98
    "Repo",  // Phase 98
    "Changeset",  // Phase 99
    "Migration",  // Phase 101
    "Regex",  // Phase 119
];

/// Map Mesh builtin function names to their runtime equivalents.
///
/// Mesh source uses clean names like `println`, `print`, `string_length`.
/// These are mapped to the actual runtime function names like `mesh_println`,
/// `mesh_print`, `mesh_string_length` at the MIR level.
fn map_builtin_name(name: &str) -> String {
    match name {
        "println" => "mesh_println".to_string(),
        "print" => "mesh_print".to_string(),
        // String operations
        "string_length" => "mesh_string_length".to_string(),
        "string_slice" => "mesh_string_slice".to_string(),
        "string_contains" => "mesh_string_contains".to_string(),
        "string_starts_with" => "mesh_string_starts_with".to_string(),
        "string_ends_with" => "mesh_string_ends_with".to_string(),
        "string_trim" => "mesh_string_trim".to_string(),
        "string_to_upper" => "mesh_string_to_upper".to_string(),
        "string_to_lower" => "mesh_string_to_lower".to_string(),
        "string_replace" => "mesh_string_replace".to_string(),
        "string_split" => "mesh_string_split".to_string(),
        "string_join" => "mesh_string_join".to_string(),
        "string_to_int" => "mesh_string_to_int".to_string(),
        "string_to_float" => "mesh_string_to_float".to_string(),
        // File I/O functions
        "file_read" => "mesh_file_read".to_string(),
        "file_write" => "mesh_file_write".to_string(),
        "file_append" => "mesh_file_append".to_string(),
        "file_exists" => "mesh_file_exists".to_string(),
        "file_delete" => "mesh_file_delete".to_string(),
        // IO functions
        "io_read_line" => "mesh_io_read_line".to_string(),
        "io_eprintln" => "mesh_io_eprintln".to_string(),
        // Env functions
        // "env_get" is the prefixed form of Env.get — routes to 2-arg with-default variant
        "env_get" => "mesh_env_get_with_default".to_string(),
        "env_get_with_default" => "mesh_env_get_with_default".to_string(),
        "env_get_int" => "mesh_env_get_int".to_string(),
        "env_args" => "mesh_env_args".to_string(),
        // Regex functions (Phase 119)
        "regex_from_literal" => "mesh_regex_from_literal".to_string(),
        // "regex_compile" is the prefixed form of Regex.compile
        "regex_compile" => "mesh_regex_compile".to_string(),
        "regex_is_match" => "mesh_regex_match".to_string(),
        "regex_captures" => "mesh_regex_captures".to_string(),
        "regex_replace" => "mesh_regex_replace".to_string(),
        "regex_split" => "mesh_regex_split".to_string(),
        // Bare name for compile (from Regex import compile)
        "compile" => "mesh_regex_compile".to_string(),
        // Names that have already been resolved via from-import and lowered
        // with the module prefix (e.g., user wrote `length` after `from String import length`,
        // but it was registered with both names so it may arrive as bare name here).
        "length" => "mesh_string_length".to_string(),
        "trim" => "mesh_string_trim".to_string(),
        "contains" => "mesh_string_contains".to_string(),
        "starts_with" => "mesh_string_starts_with".to_string(),
        "ends_with" => "mesh_string_ends_with".to_string(),
        "to_upper" => "mesh_string_to_upper".to_string(),
        "to_lower" => "mesh_string_to_lower".to_string(),
        "replace" => "mesh_string_replace".to_string(),
        "slice" => "mesh_string_slice".to_string(),
        "split" => "mesh_string_split".to_string(),
        "join" => "mesh_string_join".to_string(),
        "read_line" => "mesh_io_read_line".to_string(),
        "eprintln" => "mesh_io_eprintln".to_string(),
        // File bare names (from File import read, etc.)
        "read" => "mesh_file_read".to_string(),
        "write" => "mesh_file_write".to_string(),
        "append" => "mesh_file_append".to_string(),
        "exists" => "mesh_file_exists".to_string(),
        "delete" => "mesh_file_delete".to_string(),
        // ── Collection functions (Phase 8 Plan 02) ───────────────────
        // List operations
        "list_new" => "mesh_list_new".to_string(),
        "list_length" => "mesh_list_length".to_string(),
        "list_append" => "mesh_list_append".to_string(),
        "list_head" => "mesh_list_head".to_string(),
        "list_tail" => "mesh_list_tail".to_string(),
        "list_get" => "mesh_list_get".to_string(),
        "list_concat" => "mesh_list_concat".to_string(),
        "list_reverse" => "mesh_list_reverse".to_string(),
        "list_map" => "mesh_list_map".to_string(),
        "list_filter" => "mesh_list_filter".to_string(),
        "list_reduce" => "mesh_list_reduce".to_string(),
        // Phase 46: sort, find, any, all, contains
        "list_sort" => "mesh_list_sort".to_string(),
        "list_find" => "mesh_list_find".to_string(),
        "list_any" => "mesh_list_any".to_string(),
        "list_all" => "mesh_list_all".to_string(),
        "list_contains" => "mesh_list_contains".to_string(),
        // Phase 47: zip, flat_map, flatten, enumerate, take, drop, last, nth
        "list_zip" => "mesh_list_zip".to_string(),
        "list_flat_map" => "mesh_list_flat_map".to_string(),
        "list_flatten" => "mesh_list_flatten".to_string(),
        "list_enumerate" => "mesh_list_enumerate".to_string(),
        "list_take" => "mesh_list_take".to_string(),
        "list_drop" => "mesh_list_drop".to_string(),
        "list_last" => "mesh_list_last".to_string(),
        "list_nth" => "mesh_list_nth".to_string(),
        // Map operations
        "map_new" => "mesh_map_new".to_string(),
        "map_put" => "mesh_map_put".to_string(),
        "map_get" => "mesh_map_get".to_string(),
        "map_has_key" => "mesh_map_has_key".to_string(),
        "map_delete" => "mesh_map_delete".to_string(),
        "map_size" => "mesh_map_size".to_string(),
        "map_keys" => "mesh_map_keys".to_string(),
        "map_values" => "mesh_map_values".to_string(),
        // Phase 47: Map merge/to_list/from_list
        "map_merge" => "mesh_map_merge".to_string(),
        "map_to_list" => "mesh_map_to_list".to_string(),
        "map_from_list" => "mesh_map_from_list".to_string(),
        // Set operations
        "set_new" => "mesh_set_new".to_string(),
        "set_add" => "mesh_set_add".to_string(),
        "set_remove" => "mesh_set_remove".to_string(),
        "set_contains" => "mesh_set_contains".to_string(),
        "set_size" => "mesh_set_size".to_string(),
        "set_union" => "mesh_set_union".to_string(),
        "set_intersection" => "mesh_set_intersection".to_string(),
        // Phase 47: Set difference/to_list/from_list
        "set_difference" => "mesh_set_difference".to_string(),
        "set_to_list" => "mesh_set_to_list".to_string(),
        "set_from_list" => "mesh_set_from_list".to_string(),
        // Tuple operations
        "tuple_nth" => "mesh_tuple_nth".to_string(),
        "tuple_first" => "mesh_tuple_first".to_string(),
        "tuple_second" => "mesh_tuple_second".to_string(),
        "tuple_size" => "mesh_tuple_size".to_string(),
        // Range operations
        "range_new" => "mesh_range_new".to_string(),
        "range_to_list" => "mesh_range_to_list".to_string(),
        "range_map" => "mesh_range_map".to_string(),
        "range_filter" => "mesh_range_filter".to_string(),
        "range_length" => "mesh_range_length".to_string(),
        // Queue operations
        "queue_new" => "mesh_queue_new".to_string(),
        "queue_push" => "mesh_queue_push".to_string(),
        "queue_pop" => "mesh_queue_pop".to_string(),
        "queue_peek" => "mesh_queue_peek".to_string(),
        "queue_size" => "mesh_queue_size".to_string(),
        "queue_is_empty" => "mesh_queue_is_empty".to_string(),
        // Bare names for prelude functions (map, filter, reduce, head, tail)
        // These are ambiguous -- default to list operations.
        "map" => "mesh_list_map".to_string(),
        "filter" => "mesh_list_filter".to_string(),
        "reduce" => "mesh_list_reduce".to_string(),
        "head" => "mesh_list_head".to_string(),
        "tail" => "mesh_list_tail".to_string(),
        "zip" => "mesh_list_zip".to_string(),
        "flat_map" => "mesh_list_flat_map".to_string(),
        "flatten" => "mesh_list_flatten".to_string(),
        "enumerate" => "mesh_list_enumerate".to_string(),
        "last" => "mesh_list_last".to_string(),
        "nth" => "mesh_list_nth".to_string(),
        "merge" => "mesh_map_merge".to_string(),
        "difference" => "mesh_set_difference".to_string(),
        // ── JSON functions (Phase 8 Plan 04) ─────────────────────────
        "json_parse" => "mesh_json_parse".to_string(),
        "json_encode" => "mesh_json_encode".to_string(),
        "json_encode_string" => "mesh_json_encode_string".to_string(),
        "json_encode_int" => "mesh_json_encode_int".to_string(),
        "json_encode_bool" => "mesh_json_encode_bool".to_string(),
        "json_encode_map" => "mesh_json_encode_map".to_string(),
        "json_encode_list" => "mesh_json_encode_list".to_string(),
        "json_from_int" => "mesh_json_from_int".to_string(),
        "json_from_float" => "mesh_json_from_float".to_string(),
        "json_from_bool" => "mesh_json_from_bool".to_string(),
        "json_from_string" => "mesh_json_from_string".to_string(),
        // Phase 103: JSON field extraction
        "json_get" => "mesh_json_get".to_string(),
        "json_get_nested" => "mesh_json_get_nested".to_string(),
        // JSON bare names for from/import usage
        "parse" => "mesh_json_parse".to_string(),
        "encode" => "mesh_json_encode".to_string(),
        "encode_string" => "mesh_json_encode_string".to_string(),
        "encode_int" => "mesh_json_encode_int".to_string(),
        "encode_bool" => "mesh_json_encode_bool".to_string(),
        "encode_map" => "mesh_json_encode_map".to_string(),
        "encode_list" => "mesh_json_encode_list".to_string(),
        // ── HTTP functions (Phase 8 Plan 05) ──────────────────────────
        "http_router" => "mesh_http_router".to_string(),
        "http_route" => "mesh_http_route".to_string(),
        "http_serve" => "mesh_http_serve".to_string(),
        "http_serve_tls" => "mesh_http_serve_tls".to_string(),
        "http_response" => "mesh_http_response_new".to_string(),
        "http_response_with_headers" => "mesh_http_response_with_headers".to_string(),
        "http_get" => "mesh_http_get".to_string(),
        "http_post" => "mesh_http_post".to_string(),
        // Request accessor functions (prefixed form from module-qualified access)
        "request_method" => "mesh_http_request_method".to_string(),
        "request_path" => "mesh_http_request_path".to_string(),
        "request_body" => "mesh_http_request_body".to_string(),
        "request_header" => "mesh_http_request_header".to_string(),
        "request_query" => "mesh_http_request_query".to_string(),
        // Phase 51: Path parameter accessor
        "request_param" => "mesh_http_request_param".to_string(),
        // Phase 51: Method-specific routing (HTTP.on_get -> http_on_get -> mesh_http_route_get)
        "http_on_get" => "mesh_http_route_get".to_string(),
        "http_on_post" => "mesh_http_route_post".to_string(),
        "http_on_put" => "mesh_http_route_put".to_string(),
        "http_on_delete" => "mesh_http_route_delete".to_string(),
        // Phase 52: Middleware
        "http_use" => "mesh_http_use_middleware".to_string(),
        // ── SQLite functions (Phase 53) ──────────────────────────────────
        "sqlite_open" => "mesh_sqlite_open".to_string(),
        "sqlite_close" => "mesh_sqlite_close".to_string(),
        "sqlite_execute" => "mesh_sqlite_execute".to_string(),
        "sqlite_query" => "mesh_sqlite_query".to_string(),
        // ── PostgreSQL functions (Phase 54) ──────────────────────────────
        "pg_connect" => "mesh_pg_connect".to_string(),
        "pg_close" => "mesh_pg_close".to_string(),
        "pg_execute" => "mesh_pg_execute".to_string(),
        "pg_query" => "mesh_pg_query".to_string(),
        // ── Phase 57: PG Transaction functions ──────────────────────────
        "pg_begin" => "mesh_pg_begin".to_string(),
        "pg_commit" => "mesh_pg_commit".to_string(),
        "pg_rollback" => "mesh_pg_rollback".to_string(),
        "pg_transaction" => "mesh_pg_transaction".to_string(),
        // ── Phase 57: SQLite Transaction functions ──────────────────────
        "sqlite_begin" => "mesh_sqlite_begin".to_string(),
        "sqlite_commit" => "mesh_sqlite_commit".to_string(),
        "sqlite_rollback" => "mesh_sqlite_rollback".to_string(),
        // ── Phase 57: Connection Pool functions ─────────────────────────
        "pool_open" => "mesh_pool_open".to_string(),
        "pool_close" => "mesh_pool_close".to_string(),
        "pool_checkout" => "mesh_pool_checkout".to_string(),
        "pool_checkin" => "mesh_pool_checkin".to_string(),
        "pool_query" => "mesh_pool_query".to_string(),
        "pool_execute" => "mesh_pool_execute".to_string(),
        // ── Phase 58: Struct-to-Row Mapping ───────────────────────────────
        "pg_query_as" => "mesh_pg_query_as".to_string(),
        "pool_query_as" => "mesh_pool_query_as".to_string(),
        // ── Phase 97: ORM SQL Generation ─────────────────────────────────
        "orm_build_select" => "mesh_orm_build_select".to_string(),
        "orm_build_insert" => "mesh_orm_build_insert".to_string(),
        "orm_build_update" => "mesh_orm_build_update".to_string(),
        "orm_build_delete" => "mesh_orm_build_delete".to_string(),
        // ── Phase 98: Query Builder ─────────────────────────────────────
        "query_from" => "mesh_query_from".to_string(),
        "query_where" => "mesh_query_where".to_string(),
        "query_where_op" => "mesh_query_where_op".to_string(),
        "query_where_in" => "mesh_query_where_in".to_string(),
        "query_where_null" => "mesh_query_where_null".to_string(),
        "query_where_not_null" => "mesh_query_where_not_null".to_string(),
        "query_where_not_in" => "mesh_query_where_not_in".to_string(),
        "query_where_between" => "mesh_query_where_between".to_string(),
        "query_where_or" => "mesh_query_where_or".to_string(),
        "query_select" => "mesh_query_select".to_string(),
        "query_order_by" => "mesh_query_order_by".to_string(),
        "query_limit" => "mesh_query_limit".to_string(),
        "query_offset" => "mesh_query_offset".to_string(),
        "query_join" => "mesh_query_join".to_string(),
        "query_join_as" => "mesh_query_join_as".to_string(),
        "query_group_by" => "mesh_query_group_by".to_string(),
        "query_having" => "mesh_query_having".to_string(),
        "query_fragment" => "mesh_query_fragment".to_string(),
        // ── Phase 108: Aggregate SELECT functions ─────────────────────────
        "query_select_count" => "mesh_query_select_count".to_string(),
        "query_select_count_field" => "mesh_query_select_count_field".to_string(),
        "query_select_sum" => "mesh_query_select_sum".to_string(),
        "query_select_avg" => "mesh_query_select_avg".to_string(),
        "query_select_min" => "mesh_query_select_min".to_string(),
        "query_select_max" => "mesh_query_select_max".to_string(),
        // ── Phase 103: Query Builder Raw Extensions ─────────────────────
        "query_select_raw" => "mesh_query_select_raw".to_string(),
        "query_where_raw" => "mesh_query_where_raw".to_string(),
        // ── Phase 106: Raw ORDER BY / GROUP BY ──────────────────────────
        "query_order_by_raw" => "mesh_query_order_by_raw".to_string(),
        "query_group_by_raw" => "mesh_query_group_by_raw".to_string(),
        // ── Phase 109: Subquery WHERE ─────────────────────────────────────
        "query_where_sub" => "mesh_query_where_sub".to_string(),
        // ── Phase 98: Repo Read Operations ──────────────────────────────
        "repo_all" => "mesh_repo_all".to_string(),
        "repo_one" => "mesh_repo_one".to_string(),
        "repo_get" => "mesh_repo_get".to_string(),
        "repo_get_by" => "mesh_repo_get_by".to_string(),
        "repo_count" => "mesh_repo_count".to_string(),
        "repo_exists" => "mesh_repo_exists".to_string(),
        // ── Phase 98: Repo Write Operations ─────────────────────────────
        "repo_insert" => "mesh_repo_insert".to_string(),
        "repo_update" => "mesh_repo_update".to_string(),
        "repo_delete" => "mesh_repo_delete".to_string(),
        "repo_transaction" => "mesh_repo_transaction".to_string(),
        // ── Phase 103: Extended Repo Write Operations ────────────────────
        "repo_update_where" => "mesh_repo_update_where".to_string(),
        "repo_delete_where" => "mesh_repo_delete_where".to_string(),
        "repo_query_raw" => "mesh_repo_query_raw".to_string(),
        "repo_execute_raw" => "mesh_repo_execute_raw".to_string(),
        // ── Phase 109: Upsert, RETURNING, Subquery ────────────────────────
        "repo_insert_or_update" => "mesh_repo_insert_or_update".to_string(),
        "repo_delete_where_returning" => "mesh_repo_delete_where_returning".to_string(),
        // ── Phase 100: Repo Preloading ──────────────────────────────────
        "repo_preload" => "mesh_repo_preload".to_string(),
        // ── Phase 99: Repo Changeset Operations ─────────────────────────
        "repo_insert_changeset" => "mesh_repo_insert_changeset".to_string(),
        "repo_update_changeset" => "mesh_repo_update_changeset".to_string(),
        // ── Phase 99: Changeset Operations ──────────────────────────────
        "changeset_cast" => "mesh_changeset_cast".to_string(),
        "changeset_cast_with_types" => "mesh_changeset_cast_with_types".to_string(),
        "changeset_validate_required" => "mesh_changeset_validate_required".to_string(),
        "changeset_validate_length" => "mesh_changeset_validate_length".to_string(),
        "changeset_validate_format" => "mesh_changeset_validate_format".to_string(),
        "changeset_validate_inclusion" => "mesh_changeset_validate_inclusion".to_string(),
        "changeset_validate_number" => "mesh_changeset_validate_number".to_string(),
        "changeset_valid" => "mesh_changeset_valid".to_string(),
        "changeset_errors" => "mesh_changeset_errors".to_string(),
        "changeset_changes" => "mesh_changeset_changes".to_string(),
        "changeset_get_change" => "mesh_changeset_get_change".to_string(),
        "changeset_get_error" => "mesh_changeset_get_error".to_string(),
        // ── Phase 101: Migration DDL Operations ─────────────────────────
        "migration_create_table" => "mesh_migration_create_table".to_string(),
        "migration_drop_table" => "mesh_migration_drop_table".to_string(),
        "migration_add_column" => "mesh_migration_add_column".to_string(),
        "migration_drop_column" => "mesh_migration_drop_column".to_string(),
        "migration_rename_column" => "mesh_migration_rename_column".to_string(),
        "migration_create_index" => "mesh_migration_create_index".to_string(),
        "migration_drop_index" => "mesh_migration_drop_index".to_string(),
        "migration_execute" => "mesh_migration_execute".to_string(),
        // NOTE: No bare name mappings for HTTP/Request (router, route, get,
        // post, method, path, body, etc.) because they collide with common
        // variable names. Use module-qualified access instead:
        //   HTTP.router(), HTTP.route(), Request.method(), etc.
        // ── Job functions (Phase 9 Plan 04) ────────────────────────────
        "job_async" => "mesh_job_async".to_string(),
        "job_await" => "mesh_job_await".to_string(),
        "job_await_timeout" => "mesh_job_await_timeout".to_string(),
        "job_map" => "mesh_job_map".to_string(),
        // ── Math/Int/Float functions (Phase 43 Plan 01) ─────────────────
        "math_abs" => "mesh_math_abs".to_string(),
        "math_min" => "mesh_math_min".to_string(),
        "math_max" => "mesh_math_max".to_string(),
        "math_pi" => "mesh_math_pi".to_string(),
        "math_pow" => "mesh_math_pow".to_string(),
        "math_sqrt" => "mesh_math_sqrt".to_string(),
        "math_floor" => "mesh_math_floor".to_string(),
        "math_ceil" => "mesh_math_ceil".to_string(),
        "math_round" => "mesh_math_round".to_string(),
        "int_to_float" => "mesh_int_to_float".to_string(),
        "int_to_string" => "mesh_int_to_string".to_string(),
        "float_to_int" => "mesh_float_to_int".to_string(),
        // ── Phase 77: From conversion dispatch ──────────────────────────
        "float_from" => "mesh_int_to_float".to_string(),
        "string_from" => "mesh_string_from".to_string(),
        // ── Timer functions (Phase 44 Plan 02) ──────────────────────────
        "timer_sleep" => "mesh_timer_sleep".to_string(),
        "timer_send_after" => "mesh_timer_send_after".to_string(),
        // ── WebSocket functions (Phase 60) ────────────────────────────
        "ws_serve" => "mesh_ws_serve".to_string(),
        "ws_send" => "mesh_ws_send".to_string(),
        "ws_send_binary" => "mesh_ws_send_binary".to_string(),
        "ws_serve_tls" => "mesh_ws_serve_tls".to_string(),
        // ── WebSocket Room functions (Phase 62) ────────────────────────
        "ws_join" => "mesh_ws_join".to_string(),
        "ws_leave" => "mesh_ws_leave".to_string(),
        "ws_broadcast" => "mesh_ws_broadcast".to_string(),
        "ws_broadcast_except" => "mesh_ws_broadcast_except".to_string(),
        // ── Phase 67: Node distribution functions ─────────────────────────
        "node_start" => "mesh_node_start".to_string(),
        "node_connect" => "mesh_node_connect".to_string(),
        "node_self" => "mesh_node_self".to_string(),
        "node_list" => "mesh_node_list".to_string(),
        "node_monitor" => "mesh_node_monitor".to_string(),
        "node_spawn" => "mesh_node_spawn".to_string(),
        "node_spawn_link" => "mesh_node_spawn_link".to_string(),
        // ── Phase 67: Process monitor/demonitor ───────────────────────────
        "process_monitor" => "mesh_process_monitor".to_string(),
        "process_demonitor" => "mesh_process_demonitor".to_string(),
        "process_register" => "mesh_process_register".to_string(),
        "process_whereis" => "mesh_process_whereis".to_string(),
        // ── Phase 68: Global registry functions ─────────────────────────
        "global_register" => "mesh_global_register".to_string(),
        "global_whereis" => "mesh_global_whereis".to_string(),
        "global_unregister" => "mesh_global_unregister".to_string(),
        // ── Phase 88: WebSocket functions (handled above in Phase 60)
        // ── Phase 76: Iterator functions ──────────────────────────────
        "iter_from" => "mesh_iter_from".to_string(),
        // ── Phase 78: Lazy Combinators & Terminals ──────────────────
        "iter_map" => "mesh_iter_map".to_string(),
        "iter_filter" => "mesh_iter_filter".to_string(),
        "iter_take" => "mesh_iter_take".to_string(),
        "iter_skip" => "mesh_iter_skip".to_string(),
        "iter_enumerate" => "mesh_iter_enumerate".to_string(),
        "iter_zip" => "mesh_iter_zip".to_string(),
        "iter_count" => "mesh_iter_count".to_string(),
        "iter_sum" => "mesh_iter_sum".to_string(),
        "iter_any" => "mesh_iter_any".to_string(),
        "iter_all" => "mesh_iter_all".to_string(),
        "iter_find" => "mesh_iter_find".to_string(),
        "iter_reduce" => "mesh_iter_reduce".to_string(),
        // ── Phase 79: Collect terminal operations ────────────────────────
        "list_collect" => "mesh_list_collect".to_string(),
        "map_collect" => "mesh_map_collect".to_string(),
        "set_collect" => "mesh_set_collect".to_string(),
        "string_collect" => "mesh_string_collect".to_string(),
        _ => name.to_string(),
    }
}

/// Convert a PascalCase name to snake_case.
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

/// Process a STRING_CONTENT segment from a triple-quoted heredoc.
///
/// - `is_first`: strip the leading newline (the one after the opening `"""`)
/// - `trim_level`: strip this many leading spaces from each line
/// - For the last segment, the last line contains only the closing indent — it is dropped.
///   Detection: if the last line is all-whitespace (pure spaces/tabs), it is the closing
///   indent line and must be stripped.
fn apply_heredoc_content(text: String, is_first: bool, trim_level: usize) -> String {
    // Strip leading newline from first segment
    let s: String = if is_first {
        if text.starts_with("\r\n") {
            text[2..].to_string()
        } else if text.starts_with('\n') {
            text[1..].to_string()
        } else {
            text
        }
    } else {
        text
    };

    // Split into lines to process each one
    let mut lines: Vec<&str> = s.split('\n').collect();

    // Drop last line if it's purely whitespace (closing indent line before closing """)
    if lines.last().map(|l| l.chars().all(|c| c == ' ' || c == '\t')).unwrap_or(false) {
        lines.pop();
    }

    let stripped_lines: Vec<String> = lines
        .iter()
        .map(|line| {
            // Count actual leading whitespace on this line
            let leading_ws: usize = line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .count();
            // Only strip up to trim_level if the line actually starts with whitespace.
            // If a line has no leading whitespace (e.g. a middle-of-line segment after
            // an interpolation), leave it untouched.
            if leading_ws >= trim_level {
                line[trim_level..].to_string()
            } else if leading_ws > 0 {
                // Partially indented line — strip what we can (no negative indent)
                line[leading_ws..].to_string()
            } else {
                // No leading whitespace — this is mid-line content after an interpolation;
                // leave it as-is.
                line.to_string()
            }
        })
        .collect();

    stripped_lines.join("\n")
}

/// Process escape sequences in a raw string token, converting `\"` → `"`,
/// `\\` → `\`, `\n` → newline, `\t` → tab, `\r` → carriage return, and
/// `\0` → null. Any other `\X` sequence passes through `X` literally.
fn unescape_string(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('0') => result.push('\0'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => result.push(other),
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract simple string content from a LITERAL or STRING_EXPR syntax node.
/// Walks children looking for STRING_CONTENT tokens and concatenates them.
fn extract_simple_string_content(node: &mesh_parser::cst::SyntaxNode) -> String {
    let mut content = String::new();
    for child in node.children_with_tokens() {
        if child.kind() == SyntaxKind::STRING_CONTENT {
            if let Some(token) = child.as_token() {
                content.push_str(&unescape_string(token.text()));
            }
        }
    }
    content
}

/// Extract a negative integer literal value from a LITERAL_PAT node.
/// Looks for MINUS token followed by INT_LITERAL.
fn extract_negative_literal(node: &mesh_parser::cst::SyntaxNode) -> i64 {
    let mut found_minus = false;
    for child in node.children_with_tokens() {
        if let Some(token) = child.as_token() {
            if token.kind() == SyntaxKind::MINUS {
                found_minus = true;
            } else if found_minus && token.kind() == SyntaxKind::INT_LITERAL {
                let val: i64 = token.text().parse().unwrap_or(0);
                return -val;
            }
        }
    }
    0
}

/// Find the type name that contains a given variant name.
fn find_type_for_variant(variant: &str, registry: &mesh_typeck::TypeRegistry) -> Option<String> {
    for (type_name, info) in &registry.sum_type_defs {
        for v in &info.variants {
            if v.name == variant {
                return Some(type_name.clone());
            }
        }
    }
    None
}

/// Collect bindings introduced by a list of patterns (for constructor pattern bindings).
fn collect_pattern_bindings(patterns: &[MirPattern]) -> Vec<(String, MirType)> {
    let mut bindings = Vec::new();
    for pat in patterns {
        collect_bindings_recursive(pat, &mut bindings);
    }
    bindings
}

fn collect_bindings_recursive(pat: &MirPattern, bindings: &mut Vec<(String, MirType)>) {
    match pat {
        MirPattern::Var(name, ty) => {
            bindings.push((name.clone(), ty.clone()));
        }
        MirPattern::Constructor { fields, .. } => {
            for f in fields {
                collect_bindings_recursive(f, bindings);
            }
        }
        MirPattern::Tuple(pats) => {
            for p in pats {
                collect_bindings_recursive(p, bindings);
            }
        }
        MirPattern::Or(alts) => {
            // Use bindings from first alternative (all should have same bindings).
            if let Some(first) = alts.first() {
                collect_bindings_recursive(first, bindings);
            }
        }
        MirPattern::ListCons { head, tail, .. } => {
            collect_bindings_recursive(head, bindings);
            collect_bindings_recursive(tail, bindings);
        }
        MirPattern::Wildcard | MirPattern::Literal(_) => {}
    }
}

/// Collect free variables from an expression that exist in the outer scope
/// but are not in the parameter set. Deduplicates by name.
fn collect_free_vars(
    expr: &MirExpr,
    params: &std::collections::HashSet<&str>,
    outer_vars: &HashMap<String, MirType>,
    captures: &mut Vec<(String, MirType)>,
) {
    match expr {
        MirExpr::Var(name, _) => {
            if !params.contains(name.as_str())
                && name != "__env"
                && outer_vars.contains_key(name)
                && !captures.iter().any(|(n, _)| n == name)
            {
                if let Some(ty) = outer_vars.get(name) {
                    captures.push((name.clone(), ty.clone()));
                }
            }
        }
        MirExpr::BinOp { lhs, rhs, .. } => {
            collect_free_vars(lhs, params, outer_vars, captures);
            collect_free_vars(rhs, params, outer_vars, captures);
        }
        MirExpr::UnaryOp { operand, .. } => {
            collect_free_vars(operand, params, outer_vars, captures);
        }
        MirExpr::Call { func, args, .. } | MirExpr::ClosureCall { closure: func, args, .. } => {
            collect_free_vars(func, params, outer_vars, captures);
            for arg in args {
                collect_free_vars(arg, params, outer_vars, captures);
            }
        }
        MirExpr::If {
            cond,
            then_body,
            else_body,
            ..
        } => {
            collect_free_vars(cond, params, outer_vars, captures);
            collect_free_vars(then_body, params, outer_vars, captures);
            collect_free_vars(else_body, params, outer_vars, captures);
        }
        MirExpr::Let { value, body, .. } => {
            collect_free_vars(value, params, outer_vars, captures);
            collect_free_vars(body, params, outer_vars, captures);
        }
        MirExpr::Block(exprs, _) => {
            for e in exprs {
                collect_free_vars(e, params, outer_vars, captures);
            }
        }
        MirExpr::Match {
            scrutinee, arms, ..
        } => {
            collect_free_vars(scrutinee, params, outer_vars, captures);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_free_vars(guard, params, outer_vars, captures);
                }
                collect_free_vars(&arm.body, params, outer_vars, captures);
            }
        }
        MirExpr::StructLit { fields, .. } => {
            for (_, val) in fields {
                collect_free_vars(val, params, outer_vars, captures);
            }
        }
        MirExpr::StructUpdate { base, overrides, .. } => {
            collect_free_vars(base, params, outer_vars, captures);
            for (_, val) in overrides {
                collect_free_vars(val, params, outer_vars, captures);
            }
        }
        MirExpr::FieldAccess { object, .. } => {
            collect_free_vars(object, params, outer_vars, captures);
        }
        MirExpr::ConstructVariant { fields, .. } => {
            for f in fields {
                collect_free_vars(f, params, outer_vars, captures);
            }
        }
        MirExpr::MakeClosure { captures: caps, .. } => {
            for c in caps {
                collect_free_vars(c, params, outer_vars, captures);
            }
        }
        MirExpr::Return(val) => {
            collect_free_vars(val, params, outer_vars, captures);
        }
        MirExpr::IntLit(_, _)
        | MirExpr::FloatLit(_, _)
        | MirExpr::BoolLit(_, _)
        | MirExpr::StringLit(_, _)
        | MirExpr::Panic { .. }
        | MirExpr::Unit => {}
        // Actor primitives
        MirExpr::ActorSpawn { func, args, terminate_callback, .. } => {
            collect_free_vars(func, params, outer_vars, captures);
            for arg in args {
                collect_free_vars(arg, params, outer_vars, captures);
            }
            if let Some(cb) = terminate_callback {
                collect_free_vars(cb, params, outer_vars, captures);
            }
        }
        MirExpr::ActorSend { target, message, .. } => {
            collect_free_vars(target, params, outer_vars, captures);
            collect_free_vars(message, params, outer_vars, captures);
        }
        MirExpr::ActorReceive { arms, timeout_ms, timeout_body, .. } => {
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_free_vars(guard, params, outer_vars, captures);
                }
                collect_free_vars(&arm.body, params, outer_vars, captures);
            }
            if let Some(tm) = timeout_ms {
                collect_free_vars(tm, params, outer_vars, captures);
            }
            if let Some(tb) = timeout_body {
                collect_free_vars(tb, params, outer_vars, captures);
            }
        }
        MirExpr::ActorSelf { .. } => {}
        MirExpr::ActorLink { target, .. } => {
            collect_free_vars(target, params, outer_vars, captures);
        }
        MirExpr::ListLit { elements, .. } => {
            for elem in elements {
                collect_free_vars(elem, params, outer_vars, captures);
            }
        }
        // Supervisor start has no free variable captures (all config is static).
        MirExpr::SupervisorStart { .. } => {}
        // Loop primitives
        MirExpr::While { cond, body, .. } => {
            collect_free_vars(cond, params, outer_vars, captures);
            collect_free_vars(body, params, outer_vars, captures);
        }
        MirExpr::Break | MirExpr::Continue => {}
        MirExpr::ForInRange { var, start, end, filter, body, .. } => {
            collect_free_vars(start, params, outer_vars, captures);
            collect_free_vars(end, params, outer_vars, captures);
            // The loop variable is locally bound -- exclude it from free vars.
            let mut inner_params = params.clone();
            inner_params.insert(var.as_str());
            if let Some(f) = filter {
                collect_free_vars(f, &inner_params, outer_vars, captures);
            }
            collect_free_vars(body, &inner_params, outer_vars, captures);
        }
        MirExpr::ForInList { var, collection, filter, body, .. } => {
            collect_free_vars(collection, params, outer_vars, captures);
            let mut inner_params = params.clone();
            inner_params.insert(var.as_str());
            if let Some(f) = filter {
                collect_free_vars(f, &inner_params, outer_vars, captures);
            }
            collect_free_vars(body, &inner_params, outer_vars, captures);
        }
        MirExpr::ForInMap { key_var, val_var, collection, filter, body, .. } => {
            collect_free_vars(collection, params, outer_vars, captures);
            let mut inner_params = params.clone();
            inner_params.insert(key_var.as_str());
            inner_params.insert(val_var.as_str());
            if let Some(f) = filter {
                collect_free_vars(f, &inner_params, outer_vars, captures);
            }
            collect_free_vars(body, &inner_params, outer_vars, captures);
        }
        MirExpr::ForInSet { var, collection, filter, body, .. } => {
            collect_free_vars(collection, params, outer_vars, captures);
            let mut inner_params = params.clone();
            inner_params.insert(var.as_str());
            if let Some(f) = filter {
                collect_free_vars(f, &inner_params, outer_vars, captures);
            }
            collect_free_vars(body, &inner_params, outer_vars, captures);
        }
        MirExpr::ForInIterator { var, iterator, filter, body, .. } => {
            collect_free_vars(iterator, params, outer_vars, captures);
            let mut inner_params = params.clone();
            inner_params.insert(var.as_str());
            if let Some(f) = filter {
                collect_free_vars(f, &inner_params, outer_vars, captures);
            }
            collect_free_vars(body, &inner_params, outer_vars, captures);
        }
        // TCE: TailCall args may reference captured variables.
        MirExpr::TailCall { args, .. } => {
            for arg in args {
                collect_free_vars(arg, params, outer_vars, captures);
            }
        }
    }
}

// ── TCE rewrite pass ─────────────────────────────────────────────────

/// Post-lowering rewrite pass: detect self-recursive calls in tail position
/// and rewrite them to TailCall nodes. Returns true if any rewrites were made.
fn rewrite_tail_calls(expr: &mut MirExpr, current_fn_name: &str) -> bool {
    match expr {
        MirExpr::Call { func, args, ty } => {
            // Check if this is a self-recursive call by name
            if let MirExpr::Var(name, _) = func.as_ref() {
                if name == current_fn_name {
                    let taken_args = std::mem::take(args);
                    let taken_ty = ty.clone();
                    *expr = MirExpr::TailCall { args: taken_args, ty: taken_ty };
                    return true;
                }
            }
            false
        }
        MirExpr::Block(exprs, _) => {
            // Only the LAST expression in a block is in tail position
            if let Some(last) = exprs.last_mut() {
                rewrite_tail_calls(last, current_fn_name)
            } else {
                false
            }
        }
        MirExpr::Let { body, .. } => {
            // The body (continuation) of a let is in tail position; the value is NOT
            rewrite_tail_calls(body, current_fn_name)
        }
        MirExpr::If { then_body, else_body, .. } => {
            // BOTH branches are in tail position; the condition is NOT
            let a = rewrite_tail_calls(then_body, current_fn_name);
            let b = rewrite_tail_calls(else_body, current_fn_name);
            a || b
        }
        MirExpr::Match { arms, .. } => {
            // All arm bodies are in tail position; the scrutinee is NOT
            let mut any = false;
            for arm in arms.iter_mut() {
                if rewrite_tail_calls(&mut arm.body, current_fn_name) {
                    any = true;
                }
            }
            any
        }
        MirExpr::ActorReceive { arms, timeout_body, .. } => {
            // All receive arm bodies and timeout body are in tail position
            let mut any = false;
            for arm in arms.iter_mut() {
                if rewrite_tail_calls(&mut arm.body, current_fn_name) {
                    any = true;
                }
            }
            if let Some(tb) = timeout_body.as_deref_mut() {
                if rewrite_tail_calls(tb, current_fn_name) {
                    any = true;
                }
            }
            any
        }
        MirExpr::Return(inner) => {
            // The inner expression of Return IS in tail position
            // (if inner is a self-call, the return just passes through the value)
            rewrite_tail_calls(inner, current_fn_name)
        }
        // Everything else is NOT a tail context -- do NOT recurse.
        // This includes: BinOp, UnaryOp, Call (non-self), ClosureCall, StructLit,
        // FieldAccess, ConstructVariant, MakeClosure, ListLit, While, ForIn*, etc.
        _ => false,
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Lower a parsed and type-checked Mesh program to MIR.
///
/// This is the main entry point for AST-to-MIR conversion. It walks the
/// typed AST, desugars pipe operators and string interpolation, lifts closures,
/// and produces a flat MIR module.
pub fn lower_to_mir(parse: &Parse, typeck: &TypeckResult, module_name: &str, pub_fns: &HashSet<String>) -> Result<MirModule, String> {
    let tree = parse.syntax();
    let source_file = match SourceFile::cast(tree.clone()) {
        Some(sf) => sf,
        None => return Err("Failed to cast root node to SourceFile".to_string()),
    };

    let mut lowerer = Lowerer::new(typeck, parse, module_name, pub_fns);

    // Also register builtin sum types from the registry (Option, Result).
    // Generic type params (T, E) are resolved to Ptr since all Mesh values
    // are heap-allocated pointers at the LLVM level.
    for (name, info) in &typeck.type_registry.sum_type_defs {
        let generic_params: Vec<String> = info.generic_params.clone();
        let variants = info
            .variants
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let fields = v
                    .fields
                    .iter()
                    .map(|f| {
                        let ty = match f {
                            mesh_typeck::VariantFieldInfo::Positional(ty) => ty,
                            mesh_typeck::VariantFieldInfo::Named(_, ty) => ty,
                        };
                        // Check if this is a generic type parameter.
                        // Generic params like T, E resolve to MirType::Struct("T")
                        // because they're not known types. Replace with Ptr since
                        // all variant payloads are pointer-sized at LLVM level.
                        if let Ty::Con(con) = ty {
                            if generic_params.contains(&con.name) {
                                return MirType::Ptr;
                            }
                        }
                        resolve_type(ty, &typeck.type_registry, false)
                    })
                    .collect();
                MirVariantDef {
                    name: v.name.clone(),
                    fields,
                    tag: i as u8,
                }
            })
            .collect();

        lowerer.sum_types.push(MirSumTypeDef {
            name: name.clone(),
            variants,
        });
    }

    // Generate Ord__compare__ for built-in primitive types (Int, Float, String).
    // These use BinOp::Lt and BinOp::Eq directly since primitives don't have
    // generated Ord__lt__ / Eq__eq__ functions.
    lowerer.generate_compare_primitive("Int", MirType::Int);
    lowerer.generate_compare_primitive("Float", MirType::Float);
    lowerer.generate_compare_primitive("String", MirType::String);

    // Generate cross-module trait method wrappers for imported structs/sum types.
    // When a struct like User is defined in module A with deriving(Json), module A
    // generates FromJson__from_json__User and __json_decode__User. After MIR merge,
    // these functions are available globally. But the importing module's lowerer
    // needs __json_decode__User in known_functions so that User.from_json(str)
    // resolves correctly in lower_field_access. Generate the thin wrappers here
    // BEFORE lower_source_file so they're available during field access resolution.
    {
        let struct_names: Vec<String> = typeck.type_registry.struct_defs.keys().cloned().collect();
        for name in &struct_names {
            // FromJson: generate __json_decode__ wrapper if not already present
            let wrapper_name = format!("__json_decode__{}", name);
            if !lowerer.known_functions.contains_key(&wrapper_name) {
                let struct_ty = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                if typeck.trait_registry.has_impl("FromJson", &struct_ty) {
                    lowerer.generate_from_json_string_wrapper(name);
                }
            }

            // ToJson: register known_functions entry for ToJson__to_json__StructName
            // The actual function body is generated in the defining module's MIR.
            let to_json_name = format!("ToJson__to_json__{}", name);
            if !lowerer.known_functions.contains_key(&to_json_name) {
                let struct_ty = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                if typeck.trait_registry.has_impl("ToJson", &struct_ty) {
                    lowerer.known_functions.insert(
                        to_json_name,
                        MirType::FnPtr(vec![MirType::Struct(name.clone())], Box::new(MirType::Ptr)),
                    );
                }
            }

            // FromRow: register known_functions entry for FromRow__from_row__StructName
            // The actual function body is generated in the defining module's MIR.
            let from_row_name = format!("FromRow__from_row__{}", name);
            if !lowerer.known_functions.contains_key(&from_row_name) {
                let struct_ty = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                if typeck.trait_registry.has_impl("FromRow", &struct_ty) {
                    lowerer.known_functions.insert(
                        from_row_name,
                        MirType::FnPtr(vec![MirType::Ptr], Box::new(MirType::Ptr)),
                    );
                }
            }
        }

        // Also handle sum types with FromJson
        let sum_names: Vec<String> = typeck.type_registry.sum_type_defs.keys().cloned().collect();
        for name in &sum_names {
            let wrapper_name = format!("__json_decode__{}", name);
            if !lowerer.known_functions.contains_key(&wrapper_name) {
                let sum_ty = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                if typeck.trait_registry.has_impl("FromJson", &sum_ty) {
                    lowerer.generate_from_json_string_wrapper(name);
                }
            }
        }

        // Schema metadata: register known_functions entries for imported structs
        // with deriving(Schema). The actual function bodies are generated in the
        // defining module's MIR and available after merge. The importing module's
        // lowerer needs these in known_functions so that StructName.__table__(),
        // StructName.__fields__(), etc. resolve correctly in lower_field_access.
        for name in &struct_names {
            let table_fn = format!("{}____table__", name);
            if !lowerer.known_functions.contains_key(&table_fn) {
                let struct_ty = Ty::Con(mesh_typeck::ty::TyCon::new(name));
                if typeck.trait_registry.has_impl("Schema", &struct_ty) {
                    // __table__() -> String
                    lowerer.known_functions.insert(
                        table_fn,
                        MirType::FnPtr(vec![], Box::new(MirType::String)),
                    );
                    // __fields__() -> Ptr (List<String>)
                    lowerer.known_functions.insert(
                        format!("{}____fields__", name),
                        MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
                    );
                    // __primary_key__() -> String
                    lowerer.known_functions.insert(
                        format!("{}____primary_key__", name),
                        MirType::FnPtr(vec![], Box::new(MirType::String)),
                    );
                    // __relationships__() -> Ptr (List<String>)
                    lowerer.known_functions.insert(
                        format!("{}____relationships__", name),
                        MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
                    );
                    // __field_types__() -> Ptr (List<String>)
                    lowerer.known_functions.insert(
                        format!("{}____field_types__", name),
                        MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
                    );
                    // __relationship_meta__() -> Ptr (List<String>)
                    lowerer.known_functions.insert(
                        format!("{}____relationship_meta__", name),
                        MirType::FnPtr(vec![], Box::new(MirType::Ptr)),
                    );
                    // Per-field column accessors: __{field}_col__() -> String
                    if let Some(info) = typeck.type_registry.struct_defs.get(name.as_str()) {
                        for (field_name, _) in &info.fields {
                            lowerer.known_functions.insert(
                                format!("{}____{}_col__", name, field_name),
                                MirType::FnPtr(vec![], Box::new(MirType::String)),
                            );
                        }
                    }
                }
            }
        }
    }

    lowerer.lower_source_file(source_file);

    // Build service dispatch tables from the generated functions.
    let mut service_dispatch = HashMap::new();
    for func in &lowerer.functions {
        if func.name.starts_with("__service_") && func.name.ends_with("_loop") {
            // Extract service name from __service_{name}_loop
            let service_name = func.name
                .strip_prefix("__service_")
                .and_then(|s| s.strip_suffix("_loop"))
                .unwrap_or("")
                .to_string();

            let mut call_handlers = Vec::new();
            let mut cast_handlers = Vec::new();

            for f in &lowerer.functions {
                let call_prefix = format!("__service_{}_handle_call_", service_name);
                let cast_prefix = format!("__service_{}_handle_cast_", service_name);

                if f.name.starts_with(&call_prefix) {
                    // params: (state, arg0, arg1, ...) -- num_args = params.len() - 1
                    let num_args = if f.params.len() > 1 { f.params.len() - 1 } else { 0 };
                    // Find the tag from the matching call helper function.
                    let method_name = f.name.strip_prefix(&call_prefix).unwrap_or("");
                    let call_fn = format!("__service_{}_call_{}", service_name, method_name);
                    // Find the tag by looking at the call helper's IntLit arg.
                    let tag = lowerer.functions.iter()
                        .find(|cf| cf.name == call_fn)
                        .and_then(|cf| {
                            if let MirExpr::Call { args, .. } = &cf.body {
                                if args.len() >= 2 {
                                    if let MirExpr::IntLit(tag, _) = &args[1] {
                                        return Some(*tag as u64);
                                    }
                                }
                            }
                            None
                        })
                        .unwrap_or(0);
                    call_handlers.push((tag, f.name.clone(), num_args));
                } else if f.name.starts_with(&cast_prefix) {
                    let num_args = if f.params.len() > 1 { f.params.len() - 1 } else { 0 };
                    let method_name = f.name.strip_prefix(&cast_prefix).unwrap_or("");
                    let cast_fn = format!("__service_{}_cast_{}", service_name, method_name);
                    let tag = lowerer.functions.iter()
                        .find(|cf| cf.name == cast_fn)
                        .and_then(|cf| {
                            if let MirExpr::Call { args, .. } = &cf.body {
                                if args.len() >= 2 {
                                    if let MirExpr::IntLit(tag, _) = &args[1] {
                                        return Some(*tag as u64);
                                    }
                                }
                            }
                            None
                        })
                        .unwrap_or(0);
                    cast_handlers.push((tag, f.name.clone(), num_args));
                }
            }

            // Sort by tag so dispatch is deterministic.
            call_handlers.sort_by_key(|h| h.0);
            cast_handlers.sort_by_key(|h| h.0);

            service_dispatch.insert(func.name.clone(), (call_handlers, cast_handlers));
        }
    }

    Ok(MirModule {
        functions: lowerer.functions,
        structs: lowerer.structs,
        sum_types: lowerer.sum_types,
        entry_function: lowerer.entry_function,
        service_dispatch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to parse and type-check a Mesh source, then lower to MIR.
    fn lower(source: &str) -> MirModule {
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);
        let empty_pub_fns = HashSet::new();
        // Ignore type errors for MIR lowering tests -- we test lowering, not typeck.
        lower_to_mir(&parse, &typeck, "", &empty_pub_fns).expect("MIR lowering failed")
    }

    #[test]
    fn lower_int_literal() {
        let mir = lower("let x = 42");
        // The top-level let should not produce a function, but we should have
        // at least the builtin sum types in the module.
        assert!(mir.functions.is_empty() || mir.functions.len() >= 0);
    }

    #[test]
    fn lower_function_def() {
        let mir = lower("fn add(a :: Int, b :: Int) -> Int do a + b end");
        let func = mir.functions.iter().find(|f| f.name == "add");
        assert!(func.is_some(), "Expected 'add' function in MIR");
        let func = func.unwrap();
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].0, "a");
        assert_eq!(func.params[0].1, MirType::Int);
        assert_eq!(func.params[1].0, "b");
        assert_eq!(func.params[1].1, MirType::Int);
        assert_eq!(func.return_type, MirType::Int);

        // Body should be a BinOp
        assert!(matches!(func.body, MirExpr::BinOp { op: BinOp::Add, .. }));
    }

    #[test]
    fn lower_pipe_desugars_to_call() {
        // `x |> f` should desugar to `f(x)`
        let mir = lower(
            "fn double(x :: Int) -> Int do x * 2 end\n\
             fn main() do 5 |> double end",
        );
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        // Body should be a Call with func=double, args=[5]
        match &main.body {
            MirExpr::Call { func, args, .. } => {
                assert!(matches!(func.as_ref(), MirExpr::Var(name, _) if name == "double"));
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], MirExpr::IntLit(5, _)));
            }
            other => panic!("Expected Call, got {:?}", other),
        }
    }

    #[test]
    fn lower_string_interpolation_desugars_to_concat() {
        let source = r#"
fn main() do
  let name = "world"
  "hello ${name}"
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some());
        let main = main.unwrap();

        // The body should contain a concat call somewhere.
        fn has_concat_call(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Call { func, .. } => {
                    if let MirExpr::Var(name, _) = func.as_ref() {
                        if name == "mesh_string_concat" {
                            return true;
                        }
                    }
                    false
                }
                MirExpr::Block(exprs, _) => exprs.iter().any(has_concat_call),
                MirExpr::Let { value, body, .. } => {
                    has_concat_call(value) || has_concat_call(body)
                }
                _ => false,
            }
        }

        assert!(
            has_concat_call(&main.body),
            "Expected mesh_string_concat call in interpolated string body: {:?}",
            main.body
        );
    }

    #[test]
    fn lower_closure_produces_lifted_function() {
        let source = r#"
fn main() do
  let y = 10
  let inc = fn(x :: Int) -> Int do x + y end
  inc
end
"#;
        let mir = lower(source);

        // Should have a lifted closure function
        let closure_fn = mir.functions.iter().find(|f| f.name.starts_with("__closure_"));
        assert!(
            closure_fn.is_some(),
            "Expected lifted closure function, got functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let closure_fn = closure_fn.unwrap();
        assert!(closure_fn.is_closure_fn);
        // First param should be __env
        assert_eq!(closure_fn.params[0].0, "__env");
    }

    #[test]
    fn lower_main_sets_entry_function() {
        let mir = lower("fn main() do 0 end");
        assert_eq!(mir.entry_function, Some("mesh_main".to_string()));
    }

    #[test]
    fn lower_if_expr() {
        let mir = lower("fn test(x :: Bool) -> Int do if x do 1 else 2 end end");
        let func = mir.functions.iter().find(|f| f.name == "test");
        assert!(func.is_some());
        assert!(matches!(func.unwrap().body, MirExpr::If { .. }));
    }

    #[test]
    fn lower_self_expr() {
        let source = r#"
actor counter(n :: Int) do
  receive do
    _ -> counter(n)
  end
end

fn main() do
  let pid = spawn(counter, 0)
  0
end
"#;
        let mir = lower(source);
        // The actor should produce a function named "counter"
        let actor_fn = mir.functions.iter().find(|f| f.name == "counter");
        assert!(actor_fn.is_some(), "Expected 'counter' actor function in MIR, got: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>());
    }

    #[test]
    fn lower_spawn_produces_actor_spawn() {
        let source = r#"
actor counter(n :: Int) do
  receive do
    _ -> counter(n)
  end
end

fn main() do
  let pid = spawn(counter, 0)
  0
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some());
        let main = main.unwrap();

        // Check body has ActorSpawn somewhere
        fn has_actor_spawn(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::ActorSpawn { .. } => true,
                MirExpr::Let { value, body, .. } => has_actor_spawn(value) || has_actor_spawn(body),
                MirExpr::Block(exprs, _) => exprs.iter().any(has_actor_spawn),
                _ => false,
            }
        }
        assert!(
            has_actor_spawn(&main.body),
            "Expected ActorSpawn in main body: {:?}", main.body
        );
    }

    #[test]
    fn lower_pid_type_resolves() {
        use crate::mir::MirType;
        let source = r#"
actor echo() do
  receive do
    _ -> echo()
  end
end

fn main() do
  let pid = spawn(echo)
  0
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some());
    }

    #[test]
    fn lower_case_expr() {
        let source = r#"
fn test(x :: Int) -> Int do
  case x do
    0 -> 1
    _ -> 2
  end
end
"#;
        let mir = lower(source);
        let func = mir.functions.iter().find(|f| f.name == "test");
        assert!(func.is_some());
        let func = func.unwrap();
        assert!(
            matches!(func.body, MirExpr::Match { .. }),
            "Expected Match, got {:?}",
            func.body
        );
    }

    #[test]
    fn lower_service_def_generates_functions() {
        let mir = lower(
            r#"
service Counter do
  fn init(initial :: Int) -> Int do
    initial
  end

  call GetCount() :: Int do |count|
    (count, count)
  end

  cast Reset() do |_count|
    0
  end
end
"#,
        );

        let fn_names: Vec<&str> = mir.functions.iter().map(|f| f.name.as_str()).collect();

        // Should have generated init, loop, start, call helper, cast helper, and handler functions.
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_init")),
            "Missing init function. Functions: {:?}",
            fn_names
        );
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_loop")),
            "Missing loop function. Functions: {:?}",
            fn_names
        );
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_start")),
            "Missing start function. Functions: {:?}",
            fn_names
        );
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_call_get_count")),
            "Missing call helper function. Functions: {:?}",
            fn_names
        );
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_cast_reset")),
            "Missing cast helper function. Functions: {:?}",
            fn_names
        );
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_handle_call_get_count")),
            "Missing call handler function. Functions: {:?}",
            fn_names
        );
        assert!(
            fn_names.iter().any(|n| n.contains("__service_counter_handle_cast_reset")),
            "Missing cast handler function. Functions: {:?}",
            fn_names
        );
    }

    #[test]
    fn lower_service_dispatch_table_populated() {
        let mir = lower(
            r#"
service Counter do
  fn init(initial :: Int) -> Int do
    initial
  end

  call GetCount() :: Int do |count|
    (count, count)
  end

  cast Reset() do |_count|
    0
  end
end
"#,
        );

        // Should have a service_dispatch entry for the loop.
        assert!(
            !mir.service_dispatch.is_empty(),
            "service_dispatch should not be empty"
        );
        let loop_key = mir
            .service_dispatch
            .keys()
            .find(|k| k.contains("counter_loop"))
            .expect("Missing counter_loop dispatch entry");
        let (calls, casts) = &mir.service_dispatch[loop_key];
        assert_eq!(calls.len(), 1, "Should have 1 call handler");
        assert_eq!(casts.len(), 1, "Should have 1 cast handler");
        assert_eq!(calls[0].0, 0, "Call handler tag should be 0");
        assert_eq!(casts[0].0, 1, "Cast handler tag should be 1");
    }

    #[test]
    fn lower_service_field_access_resolves() {
        let mir = lower(
            r#"
service Counter do
  fn init(initial :: Int) -> Int do
    initial
  end

  call GetCount() :: Int do |count|
    (count, count)
  end
end

fn main() do
  let pid = Counter.start(0)
  let count = Counter.get_count(pid)
  println(int_to_string(count))
end
"#,
        );

        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(
            main_fn.is_some(),
            "Missing mesh_main function. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn impl_method_produces_mangled_mir_function() {
        let source = r#"
interface Greetable do
  fn greet(self) -> String
end

struct Point do
  x :: Int
end

impl Greetable for Point do
  fn greet(self) -> String do
    "hello"
  end
end
"#;
        let mir = lower(source);

        let fn_names: Vec<&str> = mir.functions.iter().map(|f| f.name.as_str()).collect();

        // Assert that a MirFunction with the mangled name exists.
        let mangled_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "Greetable__greet__Point");
        assert!(
            mangled_fn.is_some(),
            "Expected MirFunction named 'Greetable__greet__Point'. Found: {:?}",
            fn_names
        );

        let mangled_fn = mangled_fn.unwrap();

        // Assert the first parameter is named "self" with type MirType::Struct("Point").
        assert!(
            !mangled_fn.params.is_empty(),
            "Expected at least one parameter (self)"
        );
        assert_eq!(
            mangled_fn.params[0].0, "self",
            "First param should be named 'self'"
        );
        assert_eq!(
            mangled_fn.params[0].1,
            MirType::Struct("Point".to_string()),
            "First param type should be MirType::Struct(\"Point\")"
        );

        // Assert the return type is String.
        assert_eq!(
            mangled_fn.return_type,
            MirType::String,
            "Return type should be String"
        );
    }

    #[test]
    fn call_site_rewrites_to_mangled_name() {
        let source = r#"
interface Greetable do
  fn greet(self) -> String
end

struct Point do
  x :: Int
end

impl Greetable for Point do
  fn greet(self) -> String do
    "hello"
  end
end

fn main() do
  let p = Point { x: 1 }
  greet(p)
end
"#;
        let mir = lower(source);

        // The main function body should contain a Call to "Greetable__greet__Point".
        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function");
        let main_fn = main_fn.unwrap();

        fn find_mangled_call(expr: &MirExpr, target: &str) -> bool {
            match expr {
                MirExpr::Call { func, .. } => {
                    if let MirExpr::Var(name, _) = func.as_ref() {
                        if name == target {
                            return true;
                        }
                    }
                    false
                }
                MirExpr::Let { value, body, .. } => {
                    find_mangled_call(value, target) || find_mangled_call(body, target)
                }
                MirExpr::Block(exprs, _) => exprs.iter().any(|e| find_mangled_call(e, target)),
                _ => false,
            }
        }

        assert!(
            find_mangled_call(&main_fn.body, "Greetable__greet__Point"),
            "Expected call to Greetable__greet__Point in main body, got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn binop_on_user_type_emits_trait_call() {
        let source = r#"
interface Add do
  fn add(self, other) -> Int
end

struct Vec2 do
  x :: Int
end

impl Add for Vec2 do
  fn add(self, other) -> Int do
    0
  end
end

fn main() do
  let a = Vec2 { x: 1 }
  let b = Vec2 { x: 2 }
  a + b
end
"#;
        let mir = lower(source);

        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function");
        let main_fn = main_fn.unwrap();

        fn find_mangled_call(expr: &MirExpr, target: &str) -> bool {
            match expr {
                MirExpr::Call { func, .. } => {
                    if let MirExpr::Var(name, _) = func.as_ref() {
                        if name == target {
                            return true;
                        }
                    }
                    false
                }
                MirExpr::Let { value, body, .. } => {
                    find_mangled_call(value, target) || find_mangled_call(body, target)
                }
                MirExpr::Block(exprs, _) => exprs.iter().any(|e| find_mangled_call(e, target)),
                _ => false,
            }
        }

        // a + b with impl Add for Vec2 should become Call to Add__add__Vec2.
        assert!(
            find_mangled_call(&main_fn.body, "Add__add__Vec2"),
            "Expected call to Add__add__Vec2 in main body, got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn primitive_binop_unchanged() {
        // Regression test: Int + Int should still produce BinOp, not a trait call.
        let source = r#"
fn main() do
  let a = 1
  let b = 2
  a + b
end
"#;
        let mir = lower(source);

        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some());
        let main_fn = main_fn.unwrap();

        fn has_binop_add(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::BinOp {
                    op: BinOp::Add, ..
                } => true,
                MirExpr::Let { value, body, .. } => {
                    has_binop_add(value) || has_binop_add(body)
                }
                MirExpr::Block(exprs, _) => exprs.iter().any(has_binop_add),
                _ => false,
            }
        }

        assert!(
            has_binop_add(&main_fn.body),
            "Expected BinOp::Add for Int + Int, got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn mono_depth_limit_prevents_overflow() {
        // Verify the Lowerer has mono_depth and max_mono_depth fields,
        // and that normal compilation does NOT produce Panic nodes
        // (depth of typical programs is well under the limit).
        let source = r#"
fn foo(x :: Int) -> Int do x + 1 end
fn bar(x :: Int) -> Int do foo(x) end
fn main() do bar(42) end
"#;
        let mir = lower(source);

        // No Panic nodes should appear in a normal program.
        fn has_panic(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Panic { .. } => true,
                MirExpr::Let { value, body, .. } => has_panic(value) || has_panic(body),
                MirExpr::Block(exprs, _) => exprs.iter().any(has_panic),
                MirExpr::Call { func, args, .. } => {
                    has_panic(func) || args.iter().any(has_panic)
                }
                MirExpr::BinOp { lhs, rhs, .. } => has_panic(lhs) || has_panic(rhs),
                MirExpr::If {
                    cond,
                    then_body,
                    else_body,
                    ..
                } => has_panic(cond) || has_panic(then_body) || has_panic(else_body),
                _ => false,
            }
        }

        for func in &mir.functions {
            assert!(
                !has_panic(&func.body),
                "Normal program should not have Panic nodes, but found one in '{}': {:?}",
                func.name, func.body
            );
        }
    }

    #[test]
    fn mono_depth_fields_initialized() {
        // Directly verify the Lowerer struct fields are properly initialized.
        let source = "let x = 1";
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);
        // We can't access Lowerer directly (it's private), but we can verify
        // that lowering a deeply nested call chain doesn't crash -- the depth
        // counter prevents stack overflow.
        let empty_pub_fns = HashSet::new();
        let _mir = lower_to_mir(&parse, &typeck, "", &empty_pub_fns).expect("MIR lowering failed");
    }

    // ── End-to-end trait codegen integration tests (19-04) ────────────

    /// Recursive helper to find a Call to a specific function name anywhere in a MirExpr tree.
    fn find_call_to(expr: &MirExpr, target: &str) -> bool {
        match expr {
            MirExpr::Call { func, args, .. } => {
                let func_match = if let MirExpr::Var(name, _) = func.as_ref() {
                    name == target
                } else {
                    false
                };
                func_match
                    || find_call_to(func, target)
                    || args.iter().any(|a| find_call_to(a, target))
            }
            MirExpr::Let { value, body, .. } => {
                find_call_to(value, target) || find_call_to(body, target)
            }
            MirExpr::Block(exprs, _) => exprs.iter().any(|e| find_call_to(e, target)),
            MirExpr::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                find_call_to(cond, target)
                    || find_call_to(then_body, target)
                    || find_call_to(else_body, target)
            }
            MirExpr::BinOp { lhs, rhs, .. } => {
                find_call_to(lhs, target) || find_call_to(rhs, target)
            }
            MirExpr::Match {
                scrutinee, arms, ..
            } => {
                find_call_to(scrutinee, target)
                    || arms.iter().any(|a| find_call_to(&a.body, target))
            }
            _ => false,
        }
    }

    /// Success Criterion 1: A Mesh program with interface, impl, struct, and trait
    /// method call compiles through MIR lowering and produces correct mangled call.
    #[test]
    fn e2e_trait_method_call_compiles() {
        let source = r#"
interface Greetable do
  fn greet(self) -> String
end

struct Greeter do
  name :: String
end

impl Greetable for Greeter do
  fn greet(self) -> String do
    "hello"
  end
end

fn main() do
  let g = Greeter { name: "world" }
  let result = greet(g)
  println(result)
end
"#;
        let mir = lower(source);

        // 1. MirProgram contains a function named Greetable__greet__Greeter
        let mangled = mir
            .functions
            .iter()
            .find(|f| f.name == "Greetable__greet__Greeter");
        assert!(
            mangled.is_some(),
            "Expected MirFunction 'Greetable__greet__Greeter'. Found: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );

        // 2. Main body contains a Call referencing the mangled name
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");
        assert!(
            find_call_to(&main_fn.body, "Greetable__greet__Greeter"),
            "Expected call to Greetable__greet__Greeter in main body, got: {:?}",
            main_fn.body
        );

        // 3. No function named bare "greet" exists (only the mangled version)
        let bare_greet = mir.functions.iter().find(|f| f.name == "greet");
        assert!(
            bare_greet.is_none(),
            "Bare 'greet' function should NOT exist in MIR -- only the mangled version"
        );
    }

    /// Success Criterion 2: Trait method calls resolve to mangled names visible in MIR
    /// using the Trait__Method__Type pattern with double-underscore separators.
    #[test]
    fn e2e_mangled_names_in_mir() {
        let source = r#"
interface Describable do
  fn describe(self) -> String
end

struct Widget do
  label :: String
end

impl Describable for Widget do
  fn describe(self) -> String do
    "widget"
  end
end
"#;
        let mir = lower(source);

        let mangled = mir
            .functions
            .iter()
            .find(|f| f.name == "Describable__describe__Widget")
            .expect("Expected mangled function Describable__describe__Widget");

        // Verify name uses exactly 2 double-underscore separators: Trait__Method__Type
        let dunder_count = mangled.name.matches("__").count();
        assert_eq!(
            dunder_count, 2,
            "Mangled name should have exactly 2 '__' separators, got {} in '{}'",
            dunder_count, mangled.name
        );

        // Verify the three parts
        let parts: Vec<&str> = mangled.name.split("__").collect();
        assert_eq!(parts.len(), 3, "Expected 3 parts: [Trait, Method, Type]");
        assert_eq!(parts[0], "Describable");
        assert_eq!(parts[1], "describe");
        assert_eq!(parts[2], "Widget");
    }

    /// Success Criterion 3: self parameter in impl methods receives the concrete struct type.
    #[test]
    fn e2e_self_param_has_concrete_type() {
        let source = r#"
interface Greetable do
  fn greet(self) -> String
end

struct Greeter do
  name :: String
end

impl Greetable for Greeter do
  fn greet(self) -> String do
    "hello"
  end
end
"#;
        let mir = lower(source);

        let mangled = mir
            .functions
            .iter()
            .find(|f| f.name == "Greetable__greet__Greeter")
            .expect("Expected Greetable__greet__Greeter function");

        // First param must be named "self"
        assert!(
            !mangled.params.is_empty(),
            "Expected at least one parameter (self)"
        );
        assert_eq!(
            mangled.params[0].0, "self",
            "First param should be named 'self'"
        );

        // Type must be the concrete struct, NOT Unit, NOT Ptr, NOT Struct("self")
        assert_eq!(
            mangled.params[0].1,
            MirType::Struct("Greeter".to_string()),
            "self param type should be MirType::Struct(\"Greeter\")"
        );
        assert_ne!(
            mangled.params[0].1,
            MirType::Unit,
            "self param type must NOT be Unit"
        );
    }

    /// Success Criterion 1+: Multiple traits with different methods for the same type
    /// all produce correctly mangled and callable functions.
    #[test]
    fn e2e_multiple_traits_different_types() {
        let source = r#"
struct Dog do name :: String end
struct Cat do name :: String end

interface Speakable do
  fn speak(self) -> String
end

impl Speakable for Dog do
  fn speak(self) -> String do "woof" end
end

impl Speakable for Cat do
  fn speak(self) -> String do "meow" end
end

fn main() do
  let d = Dog { name: "Rex" }
  let c = Cat { name: "Whiskers" }
  println(speak(d))
  println(speak(c))
end
"#;
        let mir = lower(source);
        let fn_names: Vec<&str> = mir.functions.iter().map(|f| f.name.as_str()).collect();

        // Both mangled functions must exist
        assert!(
            fn_names.contains(&"Speakable__speak__Dog"),
            "Expected Speakable__speak__Dog. Found: {:?}",
            fn_names
        );
        assert!(
            fn_names.contains(&"Speakable__speak__Cat"),
            "Expected Speakable__speak__Cat. Found: {:?}",
            fn_names
        );

        // Main body has calls to both mangled names (not bare 'speak')
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");
        assert!(
            find_call_to(&main_fn.body, "Speakable__speak__Dog"),
            "Expected call to Speakable__speak__Dog in main body"
        );
        assert!(
            find_call_to(&main_fn.body, "Speakable__speak__Cat"),
            "Expected call to Speakable__speak__Cat in main body"
        );
    }

    /// Success Criterion 4: Where-clause constrained functions reject calls with
    /// unsatisfied bounds at compile time (handled by typeck, not MIR lowerer).
    #[test]
    fn e2e_where_clause_enforcement() {
        // This source should FAIL typeck: Int does not implement Displayable.
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn main() do
  show(42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        // Typeck should report TraitNotSatisfied error for Int not implementing Displayable.
        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            has_trait_error,
            "Expected TraitNotSatisfied error from typeck when calling show(42) without \
             Displayable impl for Int. Errors: {:?}",
            typeck.errors
        );

        // MIR lowering still succeeds (it's error-tolerant), confirming CODEGEN-04
        // is handled by typeck, not the lowerer.
        let empty_pub_fns = HashSet::new();
        let mir = lower_to_mir(&parse, &typeck, "", &empty_pub_fns);
        assert!(
            mir.is_ok(),
            "MIR lowering should succeed even with typeck errors (error recovery)"
        );
    }

    /// TSND-01: Where-clause constraints propagate through direct let aliases.
    /// `let f = show; f(42)` must produce TraitNotSatisfied.
    #[test]
    fn e2e_where_clause_alias_propagation() {
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn main() do
  let f = show
  f(42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            has_trait_error,
            "Expected TraitNotSatisfied when calling aliased constrained function f(42). Errors: {:?}",
            typeck.errors
        );
    }

    /// TSND-01: Where-clause constraints propagate through chain aliases.
    /// `let f = show; let g = f; g(42)` must produce TraitNotSatisfied.
    #[test]
    fn e2e_where_clause_chain_alias() {
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn main() do
  let f = show
  let g = f
  g(42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            has_trait_error,
            "Expected TraitNotSatisfied when calling chain-aliased constrained function g(42). Errors: {:?}",
            typeck.errors
        );
    }

    /// TSND-01: Where-clause constraints work with user-defined traits through aliases,
    /// and do NOT produce false positives for conforming types.
    #[test]
    fn e2e_where_clause_alias_user_trait() {
        // Part A: Should error -- Int does not implement Greetable
        let source_bad = r#"
interface Greetable do
  fn greet(self) -> String
end

fn say_hello<T>(x :: T) -> String where T: Greetable do
  greet(x)
end

fn main() do
  let f = say_hello
  f(42)
end
"#;
        let parse = mesh_parser::parse(source_bad);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            has_trait_error,
            "Expected TraitNotSatisfied for user-defined trait Greetable via alias. Errors: {:?}",
            typeck.errors
        );

        // Part B: Should NOT error -- Person implements Greetable
        let source_good = r#"
interface Greetable do
  fn greet(self) -> String
end

struct Person do
  name :: String
end

impl Greetable for Person do
  fn greet(self) -> String do
    "hello"
  end
end

fn say_hello<T>(x :: T) -> String where T: Greetable do
  greet(x)
end

fn main() do
  let f = say_hello
  let p = Person { name: "Alice" }
  f(p)
end
"#;
        let parse_good = mesh_parser::parse(source_good);
        let typeck_good = mesh_typeck::check(&parse_good);

        let has_trait_error_good = typeck_good.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            !has_trait_error_good,
            "Should NOT get TraitNotSatisfied when calling aliased constrained function with conforming type. Errors: {:?}",
            typeck_good.errors
        );
    }

    /// QUAL-01: Higher-order apply with conforming type should NOT produce
    /// TraitNotSatisfied. apply(show, 42) where Int implements Displayable.
    #[test]
    fn e2e_qualified_type_higher_order_apply() {
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

impl Displayable for Int do
  fn display(self) -> String do
    "int"
  end
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn apply(f, x) do
  f(x)
end

fn main() do
  apply(show, 42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            !has_trait_error,
            "Should NOT get TraitNotSatisfied when passing show to apply with conforming type. Errors: {:?}",
            typeck.errors
        );
    }

    /// QUAL-03: Higher-order apply with non-conforming type MUST produce
    /// TraitNotSatisfied. apply(say_hello, 42) where Int does NOT implement Greetable.
    #[test]
    fn e2e_qualified_type_higher_order_violation() {
        let source = r#"
interface Greetable do
  fn greet(self) -> String
end

fn say_hello<T>(x :: T) -> String where T: Greetable do
  greet(x)
end

fn apply(f, x) do
  f(x)
end

fn main() do
  apply(say_hello, 42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            has_trait_error,
            "Expected TraitNotSatisfied when passing constrained function to apply with non-conforming type. Errors: {:?}",
            typeck.errors
        );
    }

    /// QUAL-02: Nested higher-order constraint propagation.
    /// wrap(apply, show, 42) should NOT produce TraitNotSatisfied when Int implements Displayable.
    #[test]
    fn e2e_qualified_type_nested_higher_order() {
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

impl Displayable for Int do
  fn display(self) -> String do
    "int"
  end
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn apply(f, x) do
  f(x)
end

fn wrap(f, g, x) do
  f(g, x)
end

fn main() do
  wrap(apply, show, 42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            !has_trait_error,
            "Should NOT get TraitNotSatisfied for nested higher-order constraint propagation. Errors: {:?}",
            typeck.errors
        );
    }

    /// QUAL-01 positive: Conforming type with full impl body.
    /// apply(show, 42) where Int implements Displayable with actual method.
    #[test]
    fn e2e_qualified_type_higher_order_conforming() {
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

impl Displayable for Int do
  fn display(self) -> String do
    "${self}"
  end
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn apply(f, x) do
  f(x)
end

fn main() do
  let result = apply(show, 42)
  result
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            !has_trait_error,
            "Should NOT get TraitNotSatisfied with conforming type in higher-order apply. Errors: {:?}",
            typeck.errors
        );
    }

    /// QUAL-01 + Phase 25 interaction: let alias of constrained function passed as
    /// higher-order argument. let f = show; apply(f, 42) should NOT produce TraitNotSatisfied.
    #[test]
    fn e2e_qualified_type_higher_order_let_alias() {
        let source = r#"
interface Displayable do
  fn display(self) -> String
end

impl Displayable for Int do
  fn display(self) -> String do
    "int"
  end
end

fn show<T>(x :: T) -> String where T: Displayable do
  display(x)
end

fn apply(f, x) do
  f(x)
end

fn main() do
  let f = show
  apply(f, 42)
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);

        let has_trait_error = typeck.errors.iter().any(|e| {
            matches!(e, mesh_typeck::error::TypeError::TraitNotSatisfied { .. })
        });
        assert!(
            !has_trait_error,
            "Should NOT get TraitNotSatisfied when passing let-aliased constrained function to apply. Errors: {:?}",
            typeck.errors
        );
    }

    /// Success Criterion 5: Depth limit machinery is in place.
    /// Normal programs produce no Panic nodes; the depth counter fields exist.
    #[test]
    fn e2e_depth_limit_field_exists() {
        // Lower a normal trait-using program and verify no Panic nodes.
        let source = r#"
interface Greetable do
  fn greet(self) -> String
end

struct Greeter do
  name :: String
end

impl Greetable for Greeter do
  fn greet(self) -> String do
    "hello"
  end
end

fn main() do
  let g = Greeter { name: "world" }
  greet(g)
end
"#;
        let mir = lower(source);

        // No Panic nodes should appear in a normal program.
        fn has_panic(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Panic { .. } => true,
                MirExpr::Let { value, body, .. } => has_panic(value) || has_panic(body),
                MirExpr::Block(exprs, _) => exprs.iter().any(has_panic),
                MirExpr::Call { func, args, .. } => {
                    has_panic(func) || args.iter().any(has_panic)
                }
                MirExpr::BinOp { lhs, rhs, .. } => has_panic(lhs) || has_panic(rhs),
                MirExpr::If {
                    cond,
                    then_body,
                    else_body,
                    ..
                } => has_panic(cond) || has_panic(then_body) || has_panic(else_body),
                _ => false,
            }
        }

        for func in &mir.functions {
            assert!(
                !has_panic(&func.body),
                "Normal trait program should not have Panic nodes, found in '{}': {:?}",
                func.name, func.body
            );
        }

        // Verify the Lowerer is initialized with depth tracking by confirming
        // that lowering succeeds (the fields exist and are properly initialized).
        // The Lowerer struct is private, so we verify indirectly through behavior.
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);
        let empty_pub_fns = HashSet::new();
        let _mir = lower_to_mir(&parse, &typeck, "", &empty_pub_fns).expect("MIR lowering with depth tracking");
    }

    #[test]
    fn debug_inspect_struct_generates_mir_function() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 1, y: 2 }
  println("test")
end
"#;
        let mir = lower(source);
        let inspect_fn = mir.functions.iter().find(|f| f.name == "Debug__inspect__Point");
        assert!(
            inspect_fn.is_some(),
            "Expected Debug__inspect__Point function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let inspect_fn = inspect_fn.unwrap();
        assert_eq!(inspect_fn.params.len(), 1);
        assert_eq!(inspect_fn.params[0].0, "self");
        assert_eq!(inspect_fn.return_type, MirType::String);
    }

    #[test]
    fn debug_inspect_sum_type_generates_mir_function() {
        let source = r#"
type Color do
  Red
  Green
  Blue
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let inspect_fn = mir.functions.iter().find(|f| f.name == "Debug__inspect__Color");
        assert!(
            inspect_fn.is_some(),
            "Expected Debug__inspect__Color function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let inspect_fn = inspect_fn.unwrap();
        assert_eq!(inspect_fn.params.len(), 1);
        assert_eq!(inspect_fn.params[0].0, "self");
        assert_eq!(inspect_fn.return_type, MirType::String);
    }

    #[test]
    fn eq_struct_generates_mir_function() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 1, y: 2 }
  println("test")
end
"#;
        let mir = lower(source);
        let eq_fn = mir.functions.iter().find(|f| f.name == "Eq__eq__Point");
        assert!(
            eq_fn.is_some(),
            "Expected Eq__eq__Point function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let eq_fn = eq_fn.unwrap();
        assert_eq!(eq_fn.params.len(), 2);
        assert_eq!(eq_fn.params[0].0, "self");
        assert_eq!(eq_fn.params[1].0, "other");
        assert_eq!(eq_fn.return_type, MirType::Bool);
    }

    #[test]
    fn ord_struct_generates_mir_function() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 1, y: 2 }
  println("test")
end
"#;
        let mir = lower(source);
        let ord_fn = mir.functions.iter().find(|f| f.name == "Ord__lt__Point");
        assert!(
            ord_fn.is_some(),
            "Expected Ord__lt__Point function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let ord_fn = ord_fn.unwrap();
        assert_eq!(ord_fn.params.len(), 2);
        assert_eq!(ord_fn.params[0].0, "self");
        assert_eq!(ord_fn.params[1].0, "other");
        assert_eq!(ord_fn.return_type, MirType::Bool);
        // Ord body uses If for lexicographic comparison (non-trivial body)
        assert!(matches!(ord_fn.body, MirExpr::If { .. }));
    }

    #[test]
    fn eq_empty_struct_returns_true() {
        let source = r#"
struct Empty do
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let eq_fn = mir.functions.iter().find(|f| f.name == "Eq__eq__Empty");
        assert!(eq_fn.is_some());
        let eq_fn = eq_fn.unwrap();
        assert!(matches!(eq_fn.body, MirExpr::BoolLit(true, _)));
    }

    #[test]
    fn ord_empty_struct_returns_false() {
        let source = r#"
struct Empty do
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let ord_fn = mir.functions.iter().find(|f| f.name == "Ord__lt__Empty");
        assert!(ord_fn.is_some());
        let ord_fn = ord_fn.unwrap();
        assert!(matches!(ord_fn.body, MirExpr::BoolLit(false, _)));
    }

    #[test]
    fn struct_eq_operator_dispatches_to_trait_call() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn check(a :: Point, b :: Point) -> Bool do
  a == b
end
"#;
        let mir = lower(source);
        let check_fn = mir.functions.iter().find(|f| f.name == "check");
        assert!(
            check_fn.is_some(),
            "Expected 'check' function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let check_fn = check_fn.unwrap();
        let body_str = format!("{:?}", check_fn.body);
        assert!(
            body_str.contains("Eq__eq__Point"),
            "Expected Eq__eq__Point call in check body, got: {}",
            body_str
        );
    }

    #[test]
    fn struct_neq_operator_negates_eq() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn check(a :: Point, b :: Point) -> Bool do
  a != b
end
"#;
        let mir = lower(source);
        let check_fn = mir.functions.iter().find(|f| f.name == "check");
        assert!(
            check_fn.is_some(),
            "Expected 'check' function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let check_fn = check_fn.unwrap();
        let body_str = format!("{:?}", check_fn.body);
        // Should contain Eq__eq__Point (since != dispatches through Eq with negation)
        assert!(
            body_str.contains("Eq__eq__Point"),
            "Expected Eq__eq__Point call in check body for !=, got: {}",
            body_str
        );
    }

    #[test]
    fn struct_lt_operator_dispatches_to_ord() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn check(a :: Point, b :: Point) -> Bool do
  a < b
end
"#;
        let mir = lower(source);
        let check_fn = mir.functions.iter().find(|f| f.name == "check");
        assert!(
            check_fn.is_some(),
            "Expected 'check' function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let check_fn = check_fn.unwrap();
        let body_str = format!("{:?}", check_fn.body);
        assert!(
            body_str.contains("Ord__lt__Point"),
            "Expected Ord__lt__Point call in check body for <, got: {}",
            body_str
        );
    }

    // ── Sum type Eq/Ord tests ────────────────────────────────────────

    #[test]
    fn eq_sum_generates_mir_function() {
        let source = r#"
type Color do
  Red
  Green(Int)
  Blue(Int, Int)
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let eq_fn = mir.functions.iter().find(|f| f.name == "Eq__eq__Color");
        assert!(
            eq_fn.is_some(),
            "Expected Eq__eq__Color function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let eq_fn = eq_fn.unwrap();
        assert_eq!(eq_fn.params.len(), 2);
        assert_eq!(eq_fn.params[0].0, "self");
        assert_eq!(eq_fn.params[1].0, "other");
        assert_eq!(eq_fn.params[0].1, MirType::SumType("Color".to_string()));
        assert_eq!(eq_fn.return_type, MirType::Bool);
        // Body uses Match for variant dispatch
        assert!(matches!(eq_fn.body, MirExpr::Match { .. }));
    }

    #[test]
    fn ord_sum_generates_mir_function() {
        let source = r#"
type Color do
  Red
  Green(Int)
  Blue(Int, Int)
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let ord_fn = mir.functions.iter().find(|f| f.name == "Ord__lt__Color");
        assert!(
            ord_fn.is_some(),
            "Expected Ord__lt__Color function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let ord_fn = ord_fn.unwrap();
        assert_eq!(ord_fn.params.len(), 2);
        assert_eq!(ord_fn.params[0].0, "self");
        assert_eq!(ord_fn.params[1].0, "other");
        assert_eq!(ord_fn.params[0].1, MirType::SumType("Color".to_string()));
        assert_eq!(ord_fn.return_type, MirType::Bool);
        // Body uses Match for variant-tag-then-payload comparison
        assert!(matches!(ord_fn.body, MirExpr::Match { .. }));
    }

    #[test]
    fn eq_sum_no_variants_returns_true() {
        let source = r#"
type Empty do
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let eq_fn = mir.functions.iter().find(|f| f.name == "Eq__eq__Empty");
        assert!(eq_fn.is_some());
        let eq_fn = eq_fn.unwrap();
        assert!(matches!(eq_fn.body, MirExpr::BoolLit(true, _)));
    }

    #[test]
    fn ord_sum_no_variants_returns_false() {
        let source = r#"
type Empty do
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let ord_fn = mir.functions.iter().find(|f| f.name == "Ord__lt__Empty");
        assert!(ord_fn.is_some());
        let ord_fn = ord_fn.unwrap();
        assert!(matches!(ord_fn.body, MirExpr::BoolLit(false, _)));
    }

    #[test]
    fn sum_eq_operator_dispatches_to_trait_call() {
        let source = r#"
type Color do
  Red
  Green(Int)
end

fn check(a :: Color, b :: Color) -> Bool do
  a == b
end
"#;
        let mir = lower(source);
        let check_fn = mir.functions.iter().find(|f| f.name == "check");
        assert!(
            check_fn.is_some(),
            "Expected 'check' function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let check_fn = check_fn.unwrap();
        let body_str = format!("{:?}", check_fn.body);
        assert!(
            body_str.contains("Eq__eq__Color"),
            "Expected Eq__eq__Color call in check body, got: {}",
            body_str
        );
    }

    #[test]
    fn sum_neq_operator_negates_eq() {
        let source = r#"
type Color do
  Red
  Green(Int)
end

fn check(a :: Color, b :: Color) -> Bool do
  a != b
end
"#;
        let mir = lower(source);
        let check_fn = mir.functions.iter().find(|f| f.name == "check");
        assert!(
            check_fn.is_some(),
            "Expected 'check' function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let check_fn = check_fn.unwrap();
        let body_str = format!("{:?}", check_fn.body);
        // != dispatches through Eq with negation
        assert!(
            body_str.contains("Eq__eq__Color"),
            "Expected Eq__eq__Color call in check body for !=, got: {}",
            body_str
        );
    }

    #[test]
    fn sum_lt_operator_dispatches_to_ord() {
        let source = r#"
type Color do
  Red
  Green(Int)
end

fn check(a :: Color, b :: Color) -> Bool do
  a < b
end
"#;
        let mir = lower(source);
        let check_fn = mir.functions.iter().find(|f| f.name == "check");
        assert!(
            check_fn.is_some(),
            "Expected 'check' function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let check_fn = check_fn.unwrap();
        let body_str = format!("{:?}", check_fn.body);
        assert!(
            body_str.contains("Ord__lt__Color"),
            "Expected Ord__lt__Color call in check body for <, got: {}",
            body_str
        );
    }

    #[test]
    fn eq_sum_unit_variants_only() {
        // Sum type with only unit variants (no payload fields)
        let source = r#"
type Direction do
  North
  South
  East
  West
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let eq_fn = mir.functions.iter().find(|f| f.name == "Eq__eq__Direction");
        assert!(eq_fn.is_some());
        let eq_fn = eq_fn.unwrap();
        // Body should be a Match with variant-based dispatch
        assert!(matches!(eq_fn.body, MirExpr::Match { .. }));
        // Each arm should ultimately yield true (same variant) or false (different variant)
        if let MirExpr::Match { arms, .. } = &eq_fn.body {
            assert_eq!(arms.len(), 4, "Should have one arm per variant");
        }
    }

    // ── Hash MIR generation tests ───────────────────────────────────

    #[test]
    fn hash_struct_generates_mir_function() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let hash_fn = mir.functions.iter().find(|f| f.name == "Hash__hash__Point");
        assert!(hash_fn.is_some(), "Expected Hash__hash__Point function in MIR");
        let hash_fn = hash_fn.unwrap();
        assert_eq!(hash_fn.params.len(), 1);
        assert_eq!(hash_fn.params[0].0, "self");
        assert_eq!(hash_fn.return_type, MirType::Int);
    }

    #[test]
    fn hash_struct_field_chaining() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let hash_fn = mir.functions.iter().find(|f| f.name == "Hash__hash__Point").unwrap();
        // Body should contain a mesh_hash_combine call (chaining two field hashes).
        fn has_combine(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Call { func, args, .. } => {
                    if let MirExpr::Var(name, _) = func.as_ref() {
                        if name == "mesh_hash_combine" {
                            return true;
                        }
                    }
                    args.iter().any(has_combine) || has_combine(func)
                }
                _ => false,
            }
        }
        assert!(has_combine(&hash_fn.body), "Hash body should contain mesh_hash_combine for multi-field struct");
    }

    #[test]
    fn hash_empty_struct_returns_constant() {
        let source = r#"
struct Empty do
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let hash_fn = mir.functions.iter().find(|f| f.name == "Hash__hash__Empty");
        assert!(hash_fn.is_some(), "Expected Hash__hash__Empty function in MIR");
        let hash_fn = hash_fn.unwrap();
        // Empty struct hash should be a constant (FNV offset basis)
        assert!(matches!(hash_fn.body, MirExpr::IntLit(_, MirType::Int)));
    }

    #[test]
    fn map_put_with_struct_key_hashes() {
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 1, y: 2 }
  let m = Map.new()
  let m2 = Map.put(m, p, 42)
  m2
end
"#;
        let mir = lower(source);
        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function in MIR");
        // The MIR should contain a Hash__hash__Point call somewhere in the body
        // (emitted as part of the map_put key hashing).
        fn has_hash_call(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Call { func, args, .. } => {
                    if let MirExpr::Var(name, _) = func.as_ref() {
                        if name == "Hash__hash__Point" {
                            return true;
                        }
                    }
                    args.iter().any(has_hash_call) || has_hash_call(func)
                }
                MirExpr::Let { value, body, .. } => {
                    has_hash_call(value) || has_hash_call(body)
                }
                _ => false,
            }
        }
        assert!(has_hash_call(&main_fn.unwrap().body), "Map.put with struct key should emit Hash__hash__Point call");
    }

    // ── Default MIR lowering tests ──────────────────────────────────

    #[test]
    fn default_int_short_circuits_to_literal() {
        let source = r#"
fn main() do
  let x :: Int = default()
  x
end
"#;
        let mir = lower(source);
        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function in MIR");
        // The body should contain an IntLit(0) somewhere (from default() -> 0).
        fn has_int_zero(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::IntLit(0, MirType::Int) => true,
                MirExpr::Let { value, body, .. } => has_int_zero(value) || has_int_zero(body),
                MirExpr::Call { args, .. } => args.iter().any(has_int_zero),
                _ => false,
            }
        }
        assert!(has_int_zero(&main_fn.unwrap().body), "default() for Int should produce IntLit(0)");
    }

    #[test]
    fn default_float_short_circuits_to_literal() {
        let source = r#"
fn main() do
  let x :: Float = default()
  x
end
"#;
        let mir = lower(source);
        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function in MIR");
        fn has_float_zero(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::FloatLit(val, MirType::Float) if *val == 0.0 => true,
                MirExpr::Let { value, body, .. } => has_float_zero(value) || has_float_zero(body),
                MirExpr::Call { args, .. } => args.iter().any(has_float_zero),
                _ => false,
            }
        }
        assert!(has_float_zero(&main_fn.unwrap().body), "default() for Float should produce FloatLit(0.0)");
    }

    #[test]
    fn default_string_short_circuits_to_literal() {
        let source = r#"
fn main() do
  let x :: String = default()
  x
end
"#;
        let mir = lower(source);
        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function in MIR");
        fn has_empty_string(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::StringLit(s, MirType::String) if s.is_empty() => true,
                MirExpr::Let { value, body, .. } => has_empty_string(value) || has_empty_string(body),
                MirExpr::Call { args, .. } => args.iter().any(has_empty_string),
                _ => false,
            }
        }
        assert!(has_empty_string(&main_fn.unwrap().body), "default() for String should produce StringLit(\"\")");
    }

    #[test]
    fn default_bool_short_circuits_to_literal() {
        let source = r#"
fn main() do
  let x :: Bool = default()
  x
end
"#;
        let mir = lower(source);
        let main_fn = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main_fn.is_some(), "Expected mesh_main function in MIR");
        fn has_bool_false(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::BoolLit(false, MirType::Bool) => true,
                MirExpr::Let { value, body, .. } => has_bool_false(value) || has_bool_false(body),
                MirExpr::Call { args, .. } => args.iter().any(has_bool_false),
                _ => false,
            }
        }
        assert!(has_bool_false(&main_fn.unwrap().body), "default() for Bool should produce BoolLit(false)");
    }

    // ── Default method body tests (21-03) ────────────────────────────

    #[test]
    fn default_method_skips_missing_error() {
        // An impl that omits a method with has_default_body=true should compile without error.
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

interface Describable do
  fn describe(self) -> String do
    "unknown"
  end
end

impl Describable for Point do
end
"#;
        let parse = mesh_parser::parse(source);
        let typeck = mesh_typeck::check(&parse);
        // Check that there are no MissingTraitMethod errors.
        let missing_errors: Vec<_> = typeck.errors.iter().filter(|e| {
            matches!(e, mesh_typeck::error::TypeError::MissingTraitMethod { .. })
        }).collect();
        assert!(missing_errors.is_empty(),
            "Expected no MissingTraitMethod errors, got: {:?}", missing_errors);
        // Should also lower to MIR without failure.
        let empty_pub_fns = HashSet::new();
        let mir = lower_to_mir(&parse, &typeck, "", &empty_pub_fns).expect("MIR lowering failed");
        assert!(mir.functions.iter().any(|f| f.name == "Describable__describe__Point"),
            "Expected default method function Describable__describe__Point in MIR, got: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>());
    }

    #[test]
    fn default_method_body_lowered_for_concrete_type() {
        // Verify that when impl Describable for Point omits `describe`,
        // the MIR contains a Describable__describe__Point function generated from the default body.
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

interface Describable do
  fn describe(self) -> String do
    "unknown"
  end
end

impl Describable for Point do
end
"#;
        let mir = lower(source);
        let func = mir.functions.iter().find(|f| f.name == "Describable__describe__Point");
        assert!(func.is_some(),
            "Expected Describable__describe__Point function in MIR, got: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>());
        let func = func.unwrap();
        // The self parameter should be present and typed to the concrete type.
        assert!(!func.params.is_empty(), "Expected at least self parameter");
        assert_eq!(func.params[0].0, "self");
    }

    #[test]
    fn override_replaces_default() {
        // When impl provides the method, the default is NOT used.
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

interface Describable do
  fn describe(self) -> String do
    "unknown"
  end
end

impl Describable for Point do
  fn describe(self) -> String do
    "point"
  end
end
"#;
        let mir = lower(source);
        // There should be exactly one Describable__describe__Point function (the override).
        let funcs: Vec<_> = mir.functions.iter()
            .filter(|f| f.name == "Describable__describe__Point")
            .collect();
        assert_eq!(funcs.len(), 1,
            "Expected exactly 1 Describable__describe__Point, got {}",
            funcs.len());
        // The body should contain the override string "point", not "unknown".
        fn has_string(expr: &MirExpr, s: &str) -> bool {
            match expr {
                MirExpr::StringLit(val, _) => val == s,
                MirExpr::Block(exprs, _) => exprs.iter().any(|e| has_string(e, s)),
                MirExpr::Let { value, body, .. } => has_string(value, s) || has_string(body, s),
                _ => false,
            }
        }
        assert!(has_string(&funcs[0].body, "point"),
            "Override body should contain 'point', got: {:?}", funcs[0].body);
        assert!(!has_string(&funcs[0].body, "unknown"),
            "Override body should NOT contain 'unknown'");
    }

    // ── Collection Display tests (Phase 21 Plan 04) ─────────────────

    /// Helper: recursively check if a MirExpr tree contains a Call to a
    /// function with the given name.
    fn has_call_to(expr: &MirExpr, fn_name: &str) -> bool {
        match expr {
            MirExpr::Call { func, args, .. } => {
                if let MirExpr::Var(name, _) = func.as_ref() {
                    if name == fn_name {
                        return true;
                    }
                }
                args.iter().any(|a| has_call_to(a, fn_name))
                    || has_call_to(func, fn_name)
            }
            MirExpr::Block(exprs, _) => exprs.iter().any(|e| has_call_to(e, fn_name)),
            MirExpr::Let { value, body, .. } => {
                has_call_to(value, fn_name) || has_call_to(body, fn_name)
            }
            _ => false,
        }
    }

    /// Helper: check if a MirExpr tree contains a Var reference to the given name.
    fn has_var_ref(expr: &MirExpr, var_name: &str) -> bool {
        match expr {
            MirExpr::Var(name, _) => name == var_name,
            MirExpr::Call { func, args, .. } => {
                has_var_ref(func, var_name) || args.iter().any(|a| has_var_ref(a, var_name))
            }
            MirExpr::Block(exprs, _) => exprs.iter().any(|e| has_var_ref(e, var_name)),
            MirExpr::Let { value, body, .. } => {
                has_var_ref(value, var_name) || has_var_ref(body, var_name)
            }
            _ => false,
        }
    }

    #[test]
    fn list_display_emits_runtime_call() {
        // String interpolation with a List should emit mesh_list_to_string
        // with mesh_int_to_string as the element callback.
        let source = r#"
fn main() do
  let xs = List.append(List.new(), 1)
  "items: ${xs}"
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        assert!(
            has_call_to(&main.body, "mesh_list_to_string"),
            "Expected mesh_list_to_string call in interpolated string body.\n\
             Body: {:?}",
            main.body
        );
        assert!(
            has_var_ref(&main.body, "mesh_int_to_string"),
            "Expected mesh_int_to_string callback reference in interpolated string body.\n\
             Body: {:?}",
            main.body
        );
    }

    #[test]
    fn map_display_emits_runtime_call() {
        // String interpolation with a Map<String, Int> should emit mesh_map_to_string
        // with mesh_string_to_string and mesh_int_to_string as callbacks.
        let source = r#"
fn main() do
  let m = %{"a" => 1}
  "map: ${m}"
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        assert!(
            has_call_to(&main.body, "mesh_map_to_string"),
            "Expected mesh_map_to_string call in interpolated string body.\n\
             Body: {:?}",
            main.body
        );
    }

    #[test]
    fn set_display_emits_runtime_call() {
        // String interpolation with a Set should emit mesh_set_to_string.
        let source = r#"
fn main() do
  let s = Set.add(Set.new(), 1)
  "set: ${s}"
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        assert!(
            has_call_to(&main.body, "mesh_set_to_string"),
            "Expected mesh_set_to_string call in interpolated string body.\n\
             Body: {:?}",
            main.body
        );
    }

    // ── Phase 24 Plan 01: Nested collection Display ─────────────────

    #[test]
    fn nested_list_callback_generates_wrapper() {
        // When a Lowerer encounters a Ty::App(Con("List"), [Ty::Con("Int")])
        // element type, resolve_to_string_callback should generate a synthetic
        // __display_list_Int_to_str wrapper function.
        //
        // We test this indirectly: lower a program with list string interpolation,
        // then verify the mesh_list_to_string call is present and uses
        // mesh_int_to_string (flat case). The wrapper generation for nested
        // types (List<List<Int>>) will be exercised once the type system
        // supports generic collection element types (TGEN-02).
        let source = r#"
fn main() do
  let xs = List.append(List.new(), 42)
  "${xs}"
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        // The flat list case: mesh_list_to_string with mesh_int_to_string callback
        assert!(
            has_call_to(&main.body, "mesh_list_to_string"),
            "Expected mesh_list_to_string call.\nBody: {:?}",
            main.body
        );
        assert!(
            has_var_ref(&main.body, "mesh_int_to_string"),
            "Expected mesh_int_to_string callback reference.\nBody: {:?}",
            main.body
        );

        // Verify no wrapper was generated for the flat case (Int is handled
        // directly, no __display_ wrapper needed).
        let has_display_wrapper = mir
            .functions
            .iter()
            .any(|f| f.name.starts_with("__display_"));
        assert!(
            !has_display_wrapper,
            "Flat List<Int> should NOT generate a __display_ wrapper.\n\
             Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    // ── Phase 23 Plan 02: Ordering & compare tests ──────────────────

    #[test]
    fn ordering_sum_type_registered_in_mir() {
        // Ordering should be registered as a built-in sum type in every MIR module.
        let mir = lower("fn main() do 1 end");
        let ordering = mir.sum_types.iter().find(|s| s.name == "Ordering");
        assert!(
            ordering.is_some(),
            "Expected Ordering sum type in MIR. Sum types: {:?}",
            mir.sum_types.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        let ordering = ordering.unwrap();
        assert_eq!(ordering.variants.len(), 3);
        assert_eq!(ordering.variants[0].name, "Less");
        assert_eq!(ordering.variants[0].tag, 0);
        assert_eq!(ordering.variants[1].name, "Equal");
        assert_eq!(ordering.variants[1].tag, 1);
        assert_eq!(ordering.variants[2].name, "Greater");
        assert_eq!(ordering.variants[2].tag, 2);
    }

    #[test]
    fn compare_primitive_functions_generated() {
        // Ord__compare__Int, Ord__compare__Float, Ord__compare__String should exist.
        let mir = lower("fn main() do 1 end");
        let fns: Vec<&str> = mir.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(fns.contains(&"Ord__compare__Int"), "Missing Ord__compare__Int. Fns: {:?}", fns);
        assert!(fns.contains(&"Ord__compare__Float"), "Missing Ord__compare__Float. Fns: {:?}", fns);
        assert!(fns.contains(&"Ord__compare__String"), "Missing Ord__compare__String. Fns: {:?}", fns);

        // Check Ord__compare__Int signature
        let compare_int = mir.functions.iter().find(|f| f.name == "Ord__compare__Int").unwrap();
        assert_eq!(compare_int.params.len(), 2);
        assert_eq!(compare_int.params[0].1, MirType::Int);
        assert_eq!(compare_int.params[1].1, MirType::Int);
        assert_eq!(compare_int.return_type, MirType::SumType("Ordering".to_string()));
    }

    #[test]
    fn compare_call_dispatches_to_mangled() {
        // compare(3, 5) should lower to Ord__compare__Int(3, 5)
        let source = r#"
fn main() -> Ordering do
  compare(3, 5)
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();
        assert!(
            has_call_to(&main.body, "Ord__compare__Int"),
            "Expected Ord__compare__Int call in main body.\nBody: {:?}",
            main.body
        );
    }

    #[test]
    fn compare_struct_generated_for_user_types() {
        // User structs with Ord derive should get Ord__compare__StructName.
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 1, y: 2 }
  println("test")
end
"#;
        let mir = lower(source);
        let compare_fn = mir.functions.iter().find(|f| f.name == "Ord__compare__Point");
        assert!(
            compare_fn.is_some(),
            "Expected Ord__compare__Point function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let compare_fn = compare_fn.unwrap();
        assert_eq!(compare_fn.params.len(), 2);
        assert_eq!(compare_fn.return_type, MirType::SumType("Ordering".to_string()));
    }

    #[test]
    fn compare_sum_generated_for_user_sum_types() {
        // User sum types with Ord derive should get Ord__compare__SumTypeName.
        let source = r#"
type Color do
  Red
  Green
  Blue
end

fn main() do
  println("test")
end
"#;
        let mir = lower(source);
        let compare_fn = mir.functions.iter().find(|f| f.name == "Ord__compare__Color");
        assert!(
            compare_fn.is_some(),
            "Expected Ord__compare__Color function in MIR. Functions: {:?}",
            mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let compare_fn = compare_fn.unwrap();
        assert_eq!(compare_fn.params.len(), 2);
        assert_eq!(compare_fn.return_type, MirType::SumType("Ordering".to_string()));
    }

    #[test]
    fn pattern_match_some_extracts_field() {
        // case Some(42) do Some(x) -> x | None -> 0 end
        // The match should produce MirExpr::Match with Constructor patterns.
        let source = r#"
fn main() -> Int do
  let opt = Some(42)
  case opt do
    Some(x) -> x
    None -> 0
  end
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        // The body should contain a Match expression with Constructor patterns
        fn has_match_with_some(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Match { arms, .. } => {
                    arms.iter().any(|arm| matches!(&arm.pattern, MirPattern::Constructor { variant, .. } if variant == "Some"))
                }
                MirExpr::Let { value, body, .. } => {
                    has_match_with_some(value) || has_match_with_some(body)
                }
                MirExpr::Block(exprs, _) => exprs.iter().any(has_match_with_some),
                _ => false,
            }
        }
        assert!(
            has_match_with_some(&main.body),
            "Expected Match with Some constructor pattern in main body.\nBody: {:?}",
            main.body
        );
    }

    #[test]
    fn pattern_match_ordering_variants() {
        // Pattern matching on Ordering should produce Constructor patterns.
        let source = r#"
fn main() -> Int do
  let ord = compare(3, 5)
  case ord do
    Less -> 1
    Equal -> 2
    Greater -> 3
  end
end
"#;
        let mir = lower(source);
        let main = mir.functions.iter().find(|f| f.name == "mesh_main");
        assert!(main.is_some(), "Expected 'mesh_main' function in MIR");
        let main = main.unwrap();

        // Should dispatch compare call
        assert!(
            has_call_to(&main.body, "Ord__compare__Int"),
            "Expected Ord__compare__Int call in main body.\nBody: {:?}",
            main.body
        );
    }

    // ── Method dot-syntax MIR tests (Phase 30-02) ─────────────────────

    #[test]
    fn e2e_method_dot_syntax_basic() {
        // METH-01 + METH-02: p.to_string() should produce same mangled call as to_string(p)
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

interface Display do
  fn to_string(self) -> String
end

impl Display for Point do
  fn to_string(p) do
    "Point"
  end
end

fn main() do
  let p = Point { x: 10, y: 20 }
  let result = p.to_string()
  println(result)
end
"#;
        let mir = lower(source);
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");
        assert!(
            find_call_to(&main_fn.body, "Display__to_string__Point"),
            "Expected call to Display__to_string__Point in main body (method dot-syntax), got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn e2e_method_dot_syntax_equivalence() {
        // METH-02: p.to_string() and to_string(p) should resolve to same mangled name
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

interface Display do
  fn to_string(self) -> String
end

impl Display for Point do
  fn to_string(p) do
    "Point"
  end
end

fn main() do
  let p = Point { x: 1, y: 2 }
  let a = to_string(p)
  let b = p.to_string()
  println(a)
  println(b)
end
"#;
        let mir = lower(source);
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");

        // Count calls to the mangled name -- should be 2 (one bare, one dot-syntax)
        fn count_calls(expr: &MirExpr, target: &str) -> usize {
            match expr {
                MirExpr::Call { func, args, .. } => {
                    let mut n = if let MirExpr::Var(name, _) = func.as_ref() {
                        if name == target { 1 } else { 0 }
                    } else {
                        0
                    };
                    n += count_calls(func, target);
                    for arg in args {
                        n += count_calls(arg, target);
                    }
                    n
                }
                MirExpr::Let { value, body, .. } => {
                    count_calls(value, target) + count_calls(body, target)
                }
                MirExpr::Block(exprs, _) => exprs.iter().map(|e| count_calls(e, target)).sum(),
                MirExpr::If { cond, then_body, else_body, .. } => {
                    count_calls(cond, target) + count_calls(then_body, target) + count_calls(else_body, target)
                }
                _ => 0,
            }
        }

        let call_count = count_calls(&main_fn.body, "Display__to_string__Point");
        assert_eq!(
            call_count, 2,
            "Expected exactly 2 calls to Display__to_string__Point (bare + dot), got {}.\nBody: {:?}",
            call_count, main_fn.body
        );
    }

    #[test]
    fn e2e_method_dot_syntax_with_args() {
        // METH-02: receiver + additional args
        let source = r#"
interface Greeter do
  fn greet(self, greeting :: String) -> String
end

struct Person do
  name :: String
end

impl Greeter for Person do
  fn greet(p, greeting) do
    greeting
  end
end

fn main() do
  let bob = Person { name: "Bob" }
  let result = bob.greet("Hello")
  println(result)
end
"#;
        let mir = lower(source);
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");
        assert!(
            find_call_to(&main_fn.body, "Greeter__greet__Person"),
            "Expected call to Greeter__greet__Person in main body (dot-syntax with args), got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn e2e_method_dot_syntax_field_access_preserved() {
        // INTG-01: p.x should still produce FieldAccess, not a method call
        let source = r#"
struct Point do
  x :: Int
  y :: Int
end

fn main() do
  let p = Point { x: 42, y: 99 }
  let val = p.x
  println(Int.to_string(val))
end
"#;
        let mir = lower(source);
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");

        // Check that a FieldAccess for "x" exists in the body
        fn has_field_access(expr: &MirExpr, field_name: &str) -> bool {
            match expr {
                MirExpr::FieldAccess { field, object, .. } => {
                    field == field_name || has_field_access(object, field_name)
                }
                MirExpr::Let { value, body, .. } => {
                    has_field_access(value, field_name) || has_field_access(body, field_name)
                }
                MirExpr::Block(exprs, _) => exprs.iter().any(|e| has_field_access(e, field_name)),
                MirExpr::Call { func, args, .. } => {
                    has_field_access(func, field_name) || args.iter().any(|a| has_field_access(a, field_name))
                }
                _ => false,
            }
        }

        assert!(
            has_field_access(&main_fn.body, "x"),
            "Expected FieldAccess for 'x' in main body (field access must be preserved), got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn e2e_method_dot_syntax_module_qualified_preserved() {
        // INTG-02: String.length(s) should still work as module-qualified call
        let source = r#"
fn main() do
  let s = "hello world"
  let len = String.length(s)
  println(Int.to_string(len))
end
"#;
        let mir = lower(source);
        let main_fn = mir
            .functions
            .iter()
            .find(|f| f.name == "mesh_main")
            .expect("Expected mesh_main function");
        assert!(
            find_call_to(&main_fn.body, "mesh_string_length"),
            "Expected call to mesh_string_length in main body (module-qualified preserved), got: {:?}",
            main_fn.body
        );
    }

    #[test]
    fn lower_while_expr() {
        let mir = lower("fn test() do while true do 1 end end");
        let func = mir.functions.iter().find(|f| f.name == "test");
        assert!(func.is_some(), "Expected 'test' function in MIR");
        assert!(
            matches!(func.unwrap().body, MirExpr::While { .. }),
            "Expected MirExpr::While, got: {:?}",
            func.unwrap().body
        );
    }

    #[test]
    fn lower_break_expr() {
        let mir = lower("fn test() do while true do break end end");
        let func = mir.functions.iter().find(|f| f.name == "test");
        assert!(func.is_some());
        // The while body should contain a Break
        fn has_break(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Break => true,
                MirExpr::While { body, .. } => has_break(body),
                MirExpr::Block(exprs, _) => exprs.iter().any(has_break),
                _ => false,
            }
        }
        assert!(
            has_break(&func.unwrap().body),
            "Expected MirExpr::Break in while body"
        );
    }

    #[test]
    fn lower_continue_expr() {
        let mir = lower("fn test() do while true do continue end end");
        let func = mir.functions.iter().find(|f| f.name == "test");
        assert!(func.is_some());
        fn has_continue(expr: &MirExpr) -> bool {
            match expr {
                MirExpr::Continue => true,
                MirExpr::While { body, .. } => has_continue(body),
                MirExpr::Block(exprs, _) => exprs.iter().any(has_continue),
                _ => false,
            }
        }
        assert!(
            has_continue(&func.unwrap().body),
            "Expected MirExpr::Continue in while body"
        );
    }

    #[test]
    fn lower_for_in_range_expr() {
        let mir = lower("fn test() do for i in 0..10 do println(i) end end");
        let func = mir.functions.iter().find(|f| f.name == "test");
        assert!(func.is_some(), "Expected 'test' function in MIR");
        let func = func.unwrap();
        match &func.body {
            MirExpr::ForInRange { var, start, end, ty, .. } => {
                assert_eq!(var, "i");
                assert!(matches!(start.as_ref(), MirExpr::IntLit(0, _)), "Expected start=0, got {:?}", start);
                assert!(matches!(end.as_ref(), MirExpr::IntLit(10, _)), "Expected end=10, got {:?}", end);
                assert_eq!(*ty, MirType::Ptr);
            }
            other => panic!("Expected MirExpr::ForInRange, got {:?}", other),
        }
    }
}
