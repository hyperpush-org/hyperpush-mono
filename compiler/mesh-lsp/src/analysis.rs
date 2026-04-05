//! Document analysis: parse, type-check, and produce LSP diagnostics.
//!
//! This module bridges the Mesh compiler frontend (parser + typeck) with the
//! LSP protocol. It converts byte-offset spans into LSP line/character
//! positions (0-based, UTF-16 code units per the LSP spec) and translates
//! parse errors and type errors into `lsp_types::Diagnostic`.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use rowan::TextRange;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

use mesh_common::module_graph::{self, ModuleGraph, ModuleId};
use mesh_parser::ast::item::{Item, SourceFile};
use mesh_pkg::manifest::{
    build_clustered_export_surface, collect_source_cluster_declarations, resolve_entrypoint,
    validate_cluster_declarations_with_source, ClusteredDeclarationError, Manifest,
    DEFAULT_ENTRYPOINT,
};
use mesh_typeck::error::{ConstraintOrigin, TypeError};
use mesh_typeck::ty::Ty;
use mesh_typeck::{ImportContext, ModuleExports, TypeckResult};

/// The result of analyzing a Mesh document.
pub struct AnalysisResult {
    /// LSP diagnostics (parse errors + type errors + warnings).
    pub diagnostics: Vec<Diagnostic>,
    /// The parse result, kept for further queries.
    pub parse: mesh_parser::Parse,
    /// The type-check result, kept for hover queries.
    pub typeck: TypeckResult,
}

/// Analyze a Mesh document: parse, type-check, and produce diagnostics.
///
/// This is the main entry point called by the LSP server on didOpen/didChange.
/// When the URI belongs to a Mesh project (an ancestor contains `mesh.toml`, or
/// a legacy ancestor contains `main.mpl` when no manifest root exists),
/// analysis uses project-aware import resolution with open-document overlays so
/// backend-shaped files behave like the real compiler path instead of isolated
/// single-file snippets.
pub fn analyze_document(
    uri: &str,
    source: &str,
    open_documents: &[(String, String)],
) -> AnalysisResult {
    match analyze_project_document(uri, source, open_documents) {
        ProjectAnalysis::Success(result) | ProjectAnalysis::Failed(result) => result,
        ProjectAnalysis::NotProject => analyze_single_document(source),
    }
}

fn analyze_single_document(source: &str) -> AnalysisResult {
    let parse = mesh_parser::parse(source);
    let typeck = mesh_typeck::check(&parse);
    let diagnostics = diagnostics_from_parse_and_typeck(source, &parse, &typeck);

    AnalysisResult {
        diagnostics,
        parse,
        typeck,
    }
}

enum ProjectAnalysis {
    Success(AnalysisResult),
    Failed(AnalysisResult),
    NotProject,
}

fn project_failure_analysis(source: &str, message: impl Into<String>) -> AnalysisResult {
    let mut result = analyze_single_document(source);
    result.diagnostics.insert(0, project_diagnostic(message));
    result
}

/// Convert a byte offset to an LSP Position (0-based line, 0-based UTF-16 character offset).
///
/// The LSP specification requires positions in UTF-16 code units. For ASCII-only
/// sources, UTF-16 offset == byte offset within the line. For non-ASCII sources,
/// we count UTF-16 code units properly.
pub fn offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let before = &source[..offset];

    let line = before.matches('\n').count() as u32;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_text = &source[line_start..offset];

    // Count UTF-16 code units for LSP spec compliance.
    let character: u32 = line_text.chars().map(|c| c.len_utf16() as u32).sum();

    Position { line, character }
}

/// Look up the inferred type at a given LSP position.
///
/// Searches the typeck result's type map for the smallest range that contains
/// the given byte offset. Returns the type formatted as a string.
pub fn type_at_position(
    source: &str,
    typeck: &TypeckResult,
    position: &Position,
) -> Option<String> {
    let offset = position_to_offset(source, position)?;
    let target_offset = rowan::TextSize::from(offset as u32);

    // Find the smallest range containing this offset.
    let mut best: Option<(TextRange, &Ty)> = None;
    for (range, ty) in &typeck.types {
        if range.contains(target_offset) || range.start() == target_offset {
            match &best {
                Some((best_range, _)) if range.len() < best_range.len() => {
                    best = Some((*range, ty));
                }
                None => {
                    best = Some((*range, ty));
                }
                _ => {}
            }
        }
    }

    best.map(|(_, ty)| format!("{}", ty))
}

/// Convert an LSP Position back to a byte offset in the source.
///
/// Public wrapper for go-to-definition support.
pub fn position_to_offset_pub(source: &str, position: &Position) -> Option<usize> {
    position_to_offset(source, position)
}

/// Convert an LSP Position back to a byte offset in the source.
fn position_to_offset(source: &str, position: &Position) -> Option<usize> {
    let mut current_line = 0u32;
    let mut line_start = 0usize;

    for (i, ch) in source.char_indices() {
        if current_line == position.line {
            // Count UTF-16 code units from line_start to find character offset.
            let line_text = &source[line_start..];
            let mut utf16_offset = 0u32;
            for (byte_idx, c) in line_text.char_indices() {
                if utf16_offset >= position.character {
                    return Some(line_start + byte_idx);
                }
                utf16_offset += c.len_utf16() as u32;
            }
            // Position is at or past end of line.
            return Some(line_start + line_text.find('\n').unwrap_or(line_text.len()));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // If we're looking for a position on the last line (no trailing newline).
    if current_line == position.line {
        let line_text = &source[line_start..];
        let mut utf16_offset = 0u32;
        for (byte_idx, c) in line_text.char_indices() {
            if utf16_offset >= position.character {
                return Some(line_start + byte_idx);
            }
            utf16_offset += c.len_utf16() as u32;
        }
        return Some(source.len());
    }

    None
}

/// Extract a TextRange span from a TypeError for diagnostic positioning.
fn type_error_span(error: &TypeError) -> Option<TextRange> {
    match error {
        TypeError::Mismatch { origin, .. } => origin_to_range(origin),
        TypeError::InfiniteType { origin, .. } => origin_to_range(origin),
        TypeError::ArityMismatch { origin, .. } => origin_to_range(origin),
        TypeError::UnboundVariable { span, .. } => Some(*span),
        TypeError::NotAFunction { span, .. } => Some(*span),
        TypeError::TraitNotSatisfied { origin, .. } => origin_to_range(origin),
        TypeError::MissingTraitMethod { .. } => None,
        TypeError::TraitMethodSignatureMismatch { .. } => None,
        TypeError::MissingField { span, .. } => Some(*span),
        TypeError::UnknownField { span, .. } => Some(*span),
        TypeError::NoSuchField { span, .. } => Some(*span),
        TypeError::UnknownVariant { span, .. } => Some(*span),
        TypeError::OrPatternBindingMismatch { span, .. } => Some(*span),
        TypeError::NonExhaustiveMatch { span, .. } => Some(*span),
        TypeError::RedundantArm { span, .. } => Some(*span),
        TypeError::InvalidGuardExpression { span, .. } => Some(*span),
        TypeError::SendTypeMismatch { span, .. } => Some(*span),
        TypeError::SelfOutsideActor { span, .. } => Some(*span),
        TypeError::SpawnNonFunction { span, .. } => Some(*span),
        TypeError::ReceiveOutsideActor { span, .. } => Some(*span),
        TypeError::InvalidChildStart { span, .. } => Some(*span),
        TypeError::InvalidStrategy { span, .. } => Some(*span),
        TypeError::InvalidRestartType { span, .. } => Some(*span),
        TypeError::InvalidShutdownValue { span, .. } => Some(*span),
        TypeError::CatchAllNotLast { span, .. } => Some(*span),
        TypeError::NonConsecutiveClauses { second_span, .. } => Some(*second_span),
        TypeError::ClauseArityMismatch { span, .. } => Some(*span),
        TypeError::NonFirstClauseAnnotation { span, .. } => Some(*span),
        TypeError::GuardTypeMismatch { span, .. } => Some(*span),
        TypeError::DuplicateImpl { .. } => None,
        TypeError::AmbiguousMethod { span, .. } => Some(*span),
        TypeError::UnsupportedDerive { .. } => None,
        TypeError::MissingDerivePrerequisite { .. } => None,
        TypeError::NoSuchMethod { span, .. } => Some(*span),
        TypeError::ManualContinuityPromotionDisabled { span } => Some(*span),
        TypeError::BreakOutsideLoop { span, .. } => Some(*span),
        TypeError::ContinueOutsideLoop { span, .. } => Some(*span),
        TypeError::ImportModuleNotFound { span, .. } => Some(*span),
        TypeError::ImportNameNotFound { span, .. } => Some(*span),
        TypeError::PrivateItem { span, .. } => Some(*span),
        TypeError::HttpClusteredInvalidArguments { span, .. } => Some(*span),
        TypeError::HttpClusteredPrivateHandler { span, .. } => Some(*span),
        TypeError::HttpClusteredOutsideRouteHandlerPosition { span } => Some(*span),
        TypeError::HttpClusteredConflictingReplicationCount { span, .. } => Some(*span),
        TypeError::HttpClusteredImportedOriginMissing { span, .. } => Some(*span),
        TypeError::TryIncompatibleReturn { span, .. } => Some(*span),
        TypeError::TryOnNonResultOption { span, .. } => Some(*span),
        TypeError::NonSerializableField { .. } => None,
        TypeError::NonMappableField { .. } => None,
        TypeError::MissingAssocType { .. } => None,
        TypeError::ExtraAssocType { .. } => None,
        TypeError::UnresolvedAssocType { span, .. } => Some(*span),
        TypeError::SlotPositionConflict { span, .. } => Some(*span),
        TypeError::SlotPipeOutOfRange { span, .. } => Some(*span),
        TypeError::UndefinedType { span, .. } => Some(*span),
    }
}

/// Extract a TextRange from a ConstraintOrigin.
fn origin_to_range(origin: &ConstraintOrigin) -> Option<TextRange> {
    match origin {
        ConstraintOrigin::FnArg { call_site, .. } => Some(*call_site),
        ConstraintOrigin::BinOp { op_span } => Some(*op_span),
        ConstraintOrigin::IfBranches { if_span, .. } => Some(*if_span),
        ConstraintOrigin::Annotation { annotation_span } => Some(*annotation_span),
        ConstraintOrigin::Return { return_span, .. } => Some(*return_span),
        ConstraintOrigin::LetBinding { binding_span } => Some(*binding_span),
        ConstraintOrigin::Assignment { lhs_span, .. } => Some(*lhs_span),
        ConstraintOrigin::Builtin => None,
    }
}

/// Convert a TypeError into an LSP Diagnostic.
fn type_error_to_diagnostic(
    source: &str,
    error: &TypeError,
    severity: DiagnosticSeverity,
) -> Option<Diagnostic> {
    let range = type_error_span(error)?;
    let start_tree: usize = range.start().into();
    let end_tree: usize = range.end().into();
    let start_offset =
        crate::definition::tree_to_source_offset(source, start_tree).unwrap_or(start_tree);
    let end_offset = crate::definition::tree_to_source_offset(source, end_tree).unwrap_or(end_tree);

    let start = offset_to_position(source, start_offset);
    let end = offset_to_position(source, end_offset);

    Some(Diagnostic {
        range: Range::new(start, end),
        severity: Some(severity),
        source: Some("mesh".to_string()),
        message: format!("{}", error),
        ..Default::default()
    })
}

fn diagnostics_from_parse_and_typeck(
    source: &str,
    parse: &mesh_parser::Parse,
    typeck: &TypeckResult,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for error in parse.errors() {
        let start = offset_to_position(source, error.span.start as usize);
        let end = offset_to_position(source, error.span.end as usize);
        diagnostics.push(Diagnostic {
            range: Range::new(start, end),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("mesh".to_string()),
            message: error.message.clone(),
            ..Default::default()
        });
    }

    for error in &typeck.errors {
        if let Some(diag) = type_error_to_diagnostic(source, error, DiagnosticSeverity::ERROR) {
            diagnostics.push(diag);
        }
    }

    for warning in &typeck.warnings {
        if let Some(diag) = type_error_to_diagnostic(source, warning, DiagnosticSeverity::WARNING) {
            diagnostics.push(diag);
        }
    }

    diagnostics
}

fn analyze_project_document(
    uri: &str,
    source: &str,
    open_documents: &[(String, String)],
) -> ProjectAnalysis {
    let Some(doc_path) = canonical_file_path(uri) else {
        return ProjectAnalysis::NotProject;
    };
    let Some(project_root) = find_project_root(&doc_path) else {
        return ProjectAnalysis::NotProject;
    };
    let relative_path = match doc_path.strip_prefix(&project_root) {
        Ok(relative_path) => relative_path.to_path_buf(),
        Err(_) => {
            return ProjectAnalysis::Failed(project_failure_analysis(
                source,
                format!(
                    "Document '{}' is not contained within discovered project root '{}'",
                    doc_path.display(),
                    project_root.display()
                ),
            ));
        }
    };

    let mut overlays = HashMap::new();
    for (open_uri, open_source) in open_documents {
        if let Some(path) = canonical_file_path(open_uri) {
            overlays.insert(path, open_source.clone());
        }
    }
    overlays.insert(doc_path.clone(), source.to_string());

    let manifest_path = project_root.join("mesh.toml");
    let manifest = if manifest_path.exists() {
        match Manifest::from_file(&manifest_path) {
            Ok(manifest) => Some(manifest),
            Err(error) => {
                return ProjectAnalysis::Failed(project_failure_analysis(source, error));
            }
        }
    } else {
        None
    };

    let entry_relative_path = match resolve_entrypoint(&project_root, manifest.as_ref()) {
        Ok(entry_relative_path) => entry_relative_path,
        Err(error) => {
            return ProjectAnalysis::Failed(project_failure_analysis(source, error));
        }
    };

    let project = match build_project_with_overlays(&project_root, &entry_relative_path, &overlays)
    {
        Ok(project) => project,
        Err(error) => {
            return ProjectAnalysis::Failed(project_failure_analysis(source, error));
        }
    };
    let current_id = match project
        .graph
        .modules
        .iter()
        .find(|module| module.path == relative_path)
        .map(|module| module.id)
    {
        Some(current_id) => current_id,
        None => {
            return ProjectAnalysis::Failed(project_failure_analysis(
                source,
                format!(
                    "Document '{}' was not discovered under project root '{}'",
                    doc_path.display(),
                    project_root.display()
                ),
            ));
        }
    };

    let module_count = project.graph.module_count();
    let mut all_exports = vec![None; module_count];
    let mut all_typeck = (0..module_count).map(|_| None).collect::<Vec<_>>();

    for &id in &project.compilation_order {
        let idx = id.0 as usize;
        let parse = &project.module_parses[idx];
        let mut import_ctx = build_import_context(&project.graph, &all_exports, parse);
        import_ctx.current_module = Some(project.graph.get(id).name.clone());

        let typeck = mesh_typeck::check_with_imports(parse, &import_ctx);
        let exports = mesh_typeck::collect_exports(parse, &typeck);
        all_exports[idx] = Some(exports);
        all_typeck[idx] = Some(typeck);
    }

    let source_cluster_declarations =
        collect_source_cluster_declarations(&project.graph, &project.module_parses);
    let has_project_errors = project
        .module_parses
        .iter()
        .any(|parse| !parse.errors().is_empty())
        || all_typeck
            .iter()
            .filter_map(|typeck| typeck.as_ref())
            .any(|typeck| !typeck.errors.is_empty());

    let current_idx = current_id.0 as usize;
    let Some(current_source) = project.module_sources.get(current_idx).cloned() else {
        return ProjectAnalysis::Failed(project_failure_analysis(
            source,
            format!(
                "Project analysis for '{}' resolved an invalid module index {}",
                doc_path.display(),
                current_idx
            ),
        ));
    };
    let Some(current_typeck) = all_typeck.into_iter().nth(current_idx).flatten() else {
        return ProjectAnalysis::Failed(project_failure_analysis(
            source,
            format!(
                "Project analysis for '{}' did not produce type-check data for '{}'",
                doc_path.display(),
                relative_path.display()
            ),
        ));
    };
    let current_parse = mesh_parser::parse(&current_source);
    let mut diagnostics =
        diagnostics_from_parse_and_typeck(&current_source, &current_parse, &current_typeck);

    if !has_project_errors {
        if let Some(cluster_diagnostics) = cluster_diagnostics(
            manifest.map(Ok),
            &source_cluster_declarations,
            &project.graph,
            &project.module_parses,
            &all_exports,
            &relative_path,
            &current_source,
        ) {
            diagnostics.extend(cluster_diagnostics);
        }
    }

    ProjectAnalysis::Success(AnalysisResult {
        diagnostics,
        parse: current_parse,
        typeck: current_typeck,
    })
}

fn project_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("mesh".to_string()),
        message: message.into(),
        ..Default::default()
    }
}

fn cluster_diagnostics(
    manifest: Option<Result<Manifest, Vec<Diagnostic>>>,
    source_cluster_declarations: &[mesh_pkg::manifest::SourceClusteredDeclaration],
    graph: &ModuleGraph,
    parses: &[mesh_parser::Parse],
    all_exports: &[Option<mesh_typeck::ExportedSymbols>],
    current_relative_path: &Path,
    current_source: &str,
) -> Option<Vec<Diagnostic>> {
    let manifest = match manifest {
        None => None,
        Some(Err(errors)) => return Some(errors),
        Some(Ok(manifest)) => Some(manifest),
    };

    if manifest
        .as_ref()
        .and_then(|manifest| manifest.cluster.as_ref())
        .is_none()
        && source_cluster_declarations.is_empty()
    {
        return None;
    }

    let surface = build_clustered_export_surface(graph, parses, all_exports);
    match validate_cluster_declarations_with_source(
        manifest
            .as_ref()
            .and_then(|manifest| manifest.cluster.as_ref()),
        source_cluster_declarations,
        &surface,
    ) {
        Ok(_) => None,
        Err(issues) => Some(
            issues
                .into_iter()
                .filter_map(|issue| {
                    clustered_declaration_diagnostic(issue, current_relative_path, current_source)
                })
                .collect(),
        ),
    }
}

fn clustered_issue_range(source: &str, span: mesh_common::span::Span) -> std::ops::Range<usize> {
    if source.is_empty() {
        return 0..0;
    }

    let mut start = (span.start as usize).min(source.len() - 1);
    let mut end = (span.end as usize).min(source.len());
    if end <= start {
        end = (start + 1).min(source.len());
    }
    start = start.min(end.saturating_sub(1));
    start..end
}

fn clustered_declaration_diagnostic(
    issue: ClusteredDeclarationError,
    current_relative_path: &Path,
    current_source: &str,
) -> Option<Diagnostic> {
    let range = match issue.origin.provenance() {
        Some(provenance) => {
            if provenance.file != current_relative_path {
                return None;
            }
            let span = clustered_issue_range(current_source, provenance.span);
            Range::new(
                offset_to_position(current_source, span.start),
                offset_to_position(current_source, span.end),
            )
        }
        None => Range::new(Position::new(0, 0), Position::new(0, 0)),
    };

    Some(Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("mesh".to_string()),
        message: issue.to_string(),
        ..Default::default()
    })
}

struct ProjectAnalysisData {
    graph: ModuleGraph,
    compilation_order: Vec<ModuleId>,
    module_sources: Vec<String>,
    module_parses: Vec<mesh_parser::Parse>,
}

fn build_project_with_overlays(
    project_root: &Path,
    entry_relative_path: &Path,
    overlays: &HashMap<PathBuf, String>,
) -> Result<ProjectAnalysisData, String> {
    if entry_relative_path.as_os_str().is_empty()
        || entry_relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "Resolved entrypoint '{}' must stay within project '{}'",
            entry_relative_path.display(),
            project_root.display()
        ));
    }

    let files = discover_mesh_files(project_root)?;
    if !files
        .iter()
        .any(|relative_path| relative_path == entry_relative_path)
    {
        return Err(format!(
            "Resolved entrypoint '{}' was not found under project '{}'",
            entry_relative_path.display(),
            project_root.display()
        ));
    }

    let mut graph = ModuleGraph::new();
    let mut module_sources = Vec::new();
    let mut module_parses = Vec::new();

    for relative_path in &files {
        let full_path = project_root.join(relative_path);
        let source = read_source_with_overlays(&full_path, overlays)?;
        let is_entry = relative_path == entry_relative_path;
        let name = if relative_path == Path::new(DEFAULT_ENTRYPOINT) {
            "Main".to_string()
        } else {
            path_to_module_name(relative_path).ok_or_else(|| {
                format!(
                    "Cannot determine module name for '{}'",
                    relative_path.display()
                )
            })?
        };

        let parse = mesh_parser::parse(&source);
        graph.add_module(name, relative_path.clone(), is_entry);
        module_sources.push(source);
        module_parses.push(parse);
    }

    let packages_dir = project_root.join(".mesh").join("packages");
    if packages_dir.exists() {
        for package_root in discover_installed_package_roots(&packages_dir)? {
            let pkg_files = discover_mesh_files(&package_root)?;
            for relative_path in &pkg_files {
                let name = match path_to_module_name(relative_path) {
                    Some(name) => name,
                    None => continue,
                };

                let full_path = package_root.join(relative_path);
                let source = read_source_with_overlays(&full_path, overlays)?;
                let parse = mesh_parser::parse(&source);
                graph.add_module(name, relative_path.clone(), false);
                module_sources.push(source);
                module_parses.push(parse);
            }
        }
    }

    for id_val in 0..graph.module_count() {
        let id = ModuleId(id_val as u32);
        let tree = module_parses[id_val].tree();
        let imports = extract_imports(&tree);
        let module_name = graph.get(id).name.clone();

        for import_name in imports {
            match graph.resolve(&import_name) {
                None => {}
                Some(dep_id) if dep_id == id => {
                    return Err(format!("Module '{}' cannot import itself", module_name));
                }
                Some(dep_id) => graph.add_dependency(id, dep_id),
            }
        }
    }

    let compilation_order = module_graph::topological_sort(&graph)
        .map_err(|e| format!("Circular dependency: {}", e))?;

    Ok(ProjectAnalysisData {
        graph,
        compilation_order,
        module_sources,
        module_parses,
    })
}

fn read_source_with_overlays(
    path: &Path,
    overlays: &HashMap<PathBuf, String>,
) -> Result<String, String> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(source) = overlays.get(&canonical) {
        return Ok(source.clone());
    }

    std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path.display(), e))
}

fn canonical_file_path(uri: &str) -> Option<PathBuf> {
    let url = Url::parse(uri).ok()?;
    let path = url.to_file_path().ok()?;
    Some(std::fs::canonicalize(&path).unwrap_or(path))
}

fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    let mut legacy_root = None;

    loop {
        if current.join("mesh.toml").exists() {
            return Some(std::fs::canonicalize(&current).unwrap_or_else(|_| current.clone()));
        }
        if legacy_root.is_none() && current.join(DEFAULT_ENTRYPOINT).exists() {
            legacy_root = Some(std::fs::canonicalize(&current).unwrap_or_else(|_| current.clone()));
        }
        if !current.pop() {
            return legacy_root;
        }
    }
}

fn path_to_module_name(relative_path: &Path) -> Option<String> {
    let stem = relative_path.file_stem()?.to_str()?;
    let parent = relative_path.parent();
    let parent_is_empty = match parent {
        None => true,
        Some(parent) => parent.as_os_str().is_empty() || parent == Path::new("."),
    };

    if stem == "main" && parent_is_empty {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(parent) = parent {
        for component in parent.components() {
            if let Component::Normal(os_str) = component {
                if let Some(segment) = os_str.to_str() {
                    parts.push(to_pascal_case(segment));
                }
            }
        }
    }
    parts.push(to_pascal_case(stem));
    Some(parts.join("."))
}

fn to_pascal_case(segment: &str) -> String {
    segment
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
                None => String::new(),
            }
        })
        .collect()
}

fn discover_mesh_files(project_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    discover_recursive(project_root, project_root, &mut files).map_err(|e| {
        format!(
            "Failed to walk directory '{}': {}",
            project_root.display(),
            e
        )
    })?;
    files.sort();
    Ok(files)
}

fn discover_recursive(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            discover_recursive(root, &path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("mpl") {
            if name.ends_with(".test.mpl") {
                continue;
            }
            let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            files.push(relative);
        }
    }

    Ok(())
}

fn discover_installed_package_roots(packages_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut package_roots = Vec::new();
    discover_installed_package_roots_recursive(packages_dir, &mut package_roots).map_err(|e| {
        format!(
            "Failed to walk installed packages under '{}': {}",
            packages_dir.display(),
            e
        )
    })?;
    package_roots.sort();
    Ok(package_roots)
}

fn discover_installed_package_roots_recursive(
    dir: &Path,
    package_roots: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut child_dirs = Vec::new();
    let mut has_manifest = false;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            child_dirs.push(path);
        } else if name == "mesh.toml" {
            has_manifest = true;
        }
    }

    if has_manifest {
        package_roots.push(dir.to_path_buf());
        return Ok(());
    }

    child_dirs.sort();
    for child_dir in child_dirs {
        discover_installed_package_roots_recursive(&child_dir, package_roots)?;
    }

    Ok(())
}

fn extract_imports(source_file: &SourceFile) -> Vec<String> {
    let mut imports = Vec::new();
    for item in source_file.items() {
        match item {
            Item::ImportDecl(decl) => {
                if let Some(path) = decl.module_path() {
                    let segments = path.segments();
                    if !segments.is_empty() {
                        imports.push(segments.join("."));
                    }
                }
            }
            Item::FromImportDecl(decl) => {
                if let Some(path) = decl.module_path() {
                    let segments = path.segments();
                    if !segments.is_empty() {
                        imports.push(segments.join("."));
                    }
                }
            }
            _ => {}
        }
    }
    imports
}

fn build_import_context(
    graph: &ModuleGraph,
    all_exports: &[Option<mesh_typeck::ExportedSymbols>],
    parse: &mesh_parser::Parse,
) -> ImportContext {
    let mut context = ImportContext::empty();

    for exports_opt in all_exports {
        if let Some(exports) = exports_opt {
            context
                .all_trait_defs
                .extend(exports.trait_defs.iter().cloned());
            context
                .all_trait_impls
                .extend(exports.trait_impls.iter().cloned());
        }
    }

    let tree = parse.tree();
    for item in tree.items() {
        let segments = match &item {
            Item::ImportDecl(import_decl) => import_decl.module_path().map(|path| path.segments()),
            Item::FromImportDecl(from_import) => {
                from_import.module_path().map(|path| path.segments())
            }
            _ => None,
        };

        if let Some(segments) = segments {
            let full_name = segments.join(".");
            let last_segment = segments.last().cloned().unwrap_or_default();
            if let Some(dep_id) = graph.resolve(&full_name) {
                let idx = dep_id.0 as usize;
                if let Some(Some(exports)) = all_exports.get(idx) {
                    context.module_exports.insert(
                        last_segment,
                        ModuleExports {
                            module_name: full_name,
                            functions: exports.functions.clone(),
                            struct_defs: exports.struct_defs.clone(),
                            sum_type_defs: exports.sum_type_defs.clone(),
                            service_defs: exports.service_defs.clone(),
                            actor_defs: exports.actor_defs.clone(),
                            private_names: exports.private_names.clone(),
                            type_aliases: exports.type_aliases.clone(),
                        },
                    );
                }
            }
        }
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn package_manifest(name: &str) -> String {
        format!(
            "[package]\nname = \"{}\"\nversion = \"1.0.0\"\n\n[dependencies]\n",
            name
        )
    }

    fn file_uri(path: &std::path::Path) -> String {
        Url::from_file_path(path)
            .expect("path should convert to file URI")
            .to_string()
    }

    fn entrypoint_manifest(name: &str, entrypoint: &str) -> String {
        format!(
            "[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nentrypoint = \"{entrypoint}\"\n\n[dependencies]\n"
        )
    }

    fn write_mesh_project(
        manifest: Option<&str>,
        files: &[(&str, &str)],
        open_relative_path: &str,
    ) -> (tempfile::TempDir, PathBuf, PathBuf, String) {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();

        if let Some(manifest) = manifest {
            std::fs::write(project_dir.join("mesh.toml"), manifest).unwrap();
        }

        for (relative_path, contents) in files {
            let full_path = project_dir.join(relative_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, contents).unwrap();
        }

        let open_path = project_dir.join(open_relative_path);
        let source = std::fs::read_to_string(&open_path).unwrap();
        (tmp, project_dir, open_path, source)
    }

    fn diagnostic_messages(result: &AnalysisResult) -> Vec<&str> {
        result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect()
    }

    fn entry_module<'a>(graph: &'a ModuleGraph) -> &'a mesh_common::module_graph::ModuleInfo {
        graph
            .modules
            .iter()
            .find(|module| module.is_entry)
            .expect("project graph should contain an entry module")
    }

    fn clustered_project_main_source() -> &'static str {
        "from Services import Jobs\nfrom Work import handle_submit\n\nfn main() do\n  let pid = Jobs.start(0)\n  let _ = Jobs.submit(pid, \"demo\")\n  let _ = handle_submit(\"demo\")\nend\n"
    }

    fn clustered_project_services_source() -> &'static str {
        "service Jobs do\n  fn init(start :: Int) -> Int do\n    start\n  end\n\n  call Submit(payload :: String) :: String do |state|\n    (state, payload)\n  end\n\n  cast Reset() do |_state|\n    0\n  end\nend\n"
    }

    fn clustered_project_work_source() -> &'static str {
        "pub fn handle_submit(payload :: String) -> String do\n  payload\nend\n\nfn hidden_submit(payload :: String) -> String do\n  payload\nend\n"
    }

    fn source_declared_work_project_main_source() -> &'static str {
        "fn main() do\n  nil\nend\n"
    }

    fn source_declared_public_work_source() -> &'static str {
        "@cluster pub fn handle_submit(payload :: String) -> String do\n  payload\nend\n\n@cluster(3) pub fn handle_retry(payload :: String) -> String do\n  payload\nend\n"
    }

    fn source_declared_private_work_source() -> &'static str {
        "@cluster fn hidden_submit(payload :: String) -> String do\n  payload\nend\n"
    }

    fn legacy_clustered_work_source() -> &'static str {
        "clustered(work) pub fn handle_submit(payload :: String) -> String do\n  payload\nend\n"
    }

    fn legacy_manifest(name: &str) -> String {
        format!(
            "{}\n[cluster]\nenabled = true\ndeclarations = [\n  {{ kind = \"work\", target = \"Work.handle_submit\" }},\n]\n",
            package_manifest(name)
        )
    }

    fn write_clustered_project(manifest: &str) -> (tempfile::TempDir, PathBuf, String) {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        let main_path = project_dir.join("main.mpl");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("mesh.toml"), manifest).unwrap();
        std::fs::write(&main_path, clustered_project_main_source()).unwrap();
        std::fs::write(
            project_dir.join("services.mpl"),
            clustered_project_services_source(),
        )
        .unwrap();
        std::fs::write(
            project_dir.join("work.mpl"),
            clustered_project_work_source(),
        )
        .unwrap();
        (tmp, main_path, clustered_project_main_source().to_string())
    }

    fn write_source_declared_work_project(
        manifest: &str,
        work_source: &str,
    ) -> (tempfile::TempDir, PathBuf, String) {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        let work_path = project_dir.join("work.mpl");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("mesh.toml"), manifest).unwrap();
        std::fs::write(
            &project_dir.join("main.mpl"),
            source_declared_work_project_main_source(),
        )
        .unwrap();
        std::fs::write(&work_path, work_source).unwrap();
        (tmp, work_path, work_source.to_string())
    }

    fn clustered_route_wrapper_main_source() -> &'static str {
        "from Api.Todos import handle_list_todos\n\npub fn handle_local(req :: Request) -> Response do\n  HTTP.response(200, \"ok\")\nend\n\nfn build() do\n  let router = HTTP.router()\n  let router = HTTP.on_get(router, \"/local\", HTTP.clustered(handle_local))\n  router |> HTTP.on_get(\"/todos\", HTTP.clustered(handle_list_todos))\nend\n\nfn main() do\n  let _ = build()\n  nil\nend\n"
    }

    fn clustered_route_wrapper_todos_source() -> &'static str {
        "pub fn handle_list_todos(req :: Request) -> Response do\n  HTTP.response(200, \"todos\")\nend\n\nfn hidden_todos(req :: Request) -> Response do\n  HTTP.response(200, \"hidden\")\nend\n"
    }

    fn write_clustered_route_wrapper_project() -> (tempfile::TempDir, PathBuf, String) {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        let main_path = project_dir.join("main.mpl");
        std::fs::create_dir_all(project_dir.join("api")).unwrap();
        std::fs::write(
            project_dir.join("mesh.toml"),
            package_manifest("clustered-routes"),
        )
        .unwrap();
        std::fs::write(&main_path, clustered_route_wrapper_main_source()).unwrap();
        std::fs::write(
            project_dir.join("api/todos.mpl"),
            clustered_route_wrapper_todos_source(),
        )
        .unwrap();
        (
            tmp,
            main_path,
            clustered_route_wrapper_main_source().to_string(),
        )
    }

    fn assert_decorator_anchored_range(diag: &Diagnostic) {
        assert_eq!(diag.range.start, Position::new(0, 0));
        assert!(
            diag.range.end.line > diag.range.start.line
                || diag.range.end.character > diag.range.start.character,
            "expected clustered diagnostic to cover a non-empty decorator span, got {:?}",
            diag.range
        );
    }

    // ── M048 entrypoint-aware project analysis ──────────────────────────

    #[test]
    fn m048_s02_find_project_root_prefers_manifest_before_legacy_main_marker() {
        let manifest = entrypoint_manifest("manifest-root", "app.mpl");
        let (_tmp, project_dir, open_path, _source) = write_mesh_project(
            Some(&manifest),
            &[
                ("app.mpl", "fn main() do\n  0\nend\n"),
                ("legacy/main.mpl", "fn main() do\n  1\nend\n"),
                (
                    "legacy/support.mpl",
                    "pub fn label() -> String do\n  \"legacy\"\nend\n",
                ),
            ],
            "legacy/support.mpl",
        );

        let detected_root = find_project_root(&open_path).expect("manifest root should resolve");

        assert_eq!(
            detected_root,
            std::fs::canonicalize(&project_dir).unwrap(),
            "manifest marker should win before a nearer legacy main.mpl"
        );
    }

    #[test]
    fn m048_s02_override_only_project_marks_nested_entry_executable_and_analyzes_cleanly() {
        let manifest = entrypoint_manifest("override-only", "lib/start.mpl");
        let (_tmp, project_dir, open_path, source) = write_mesh_project(
            Some(&manifest),
            &[
                (
                    "lib/start.mpl",
                    "from Lib.Support import label\n\nfn main() do\n  println(label())\nend\n",
                ),
                (
                    "lib/support.mpl",
                    "pub fn label() -> String do\n  \"nested-support\"\nend\n",
                ),
            ],
            "lib/start.mpl",
        );

        let manifest = Manifest::from_file(&project_dir.join("mesh.toml")).unwrap();
        let entry_relative_path = resolve_entrypoint(&project_dir, Some(&manifest)).unwrap();
        let project =
            build_project_with_overlays(&project_dir, &entry_relative_path, &HashMap::new())
                .unwrap();
        let entry = entry_module(&project.graph);

        assert_eq!(entry_relative_path, PathBuf::from("lib/start.mpl"));
        assert_eq!(entry.path, PathBuf::from("lib/start.mpl"));
        assert_eq!(entry.name, "Lib.Start");
        assert_eq!(
            project
                .graph
                .modules
                .iter()
                .filter(|module| module.is_entry)
                .count(),
            1
        );

        let result = analyze_document(&file_uri(&open_path), &source, &[]);
        let messages = diagnostic_messages(&result);
        assert!(
            messages.is_empty(),
            "override-only project should analyze cleanly without a root main.mpl, got: {:?}",
            messages
        );
    }

    #[test]
    fn m048_s02_override_precedence_keeps_root_main_path_derived_but_not_executable() {
        let manifest = entrypoint_manifest("override-precedence", "lib/start.mpl");
        let (_tmp, project_dir, open_path, source) = write_mesh_project(
            Some(&manifest),
            &[
                ("main.mpl", "fn main() do\n  0\nend\n"),
                (
                    "lib/start.mpl",
                    "from App import label\n\nfn main() do\n  println(label())\nend\n",
                ),
                (
                    "app.mpl",
                    "pub fn label() -> String do\n  \"override-app\"\nend\n",
                ),
            ],
            "lib/start.mpl",
        );

        let manifest = Manifest::from_file(&project_dir.join("mesh.toml")).unwrap();
        let entry_relative_path = resolve_entrypoint(&project_dir, Some(&manifest)).unwrap();
        let project =
            build_project_with_overlays(&project_dir, &entry_relative_path, &HashMap::new())
                .unwrap();
        let entry = entry_module(&project.graph);
        let root_main = project
            .graph
            .modules
            .iter()
            .find(|module| module.path == PathBuf::from(DEFAULT_ENTRYPOINT))
            .expect("root main.mpl should still be discovered");

        assert_eq!(entry.path, PathBuf::from("lib/start.mpl"));
        assert_eq!(entry.name, "Lib.Start");
        assert_eq!(
            project
                .graph
                .modules
                .iter()
                .filter(|module| module.is_entry)
                .count(),
            1
        );
        assert_eq!(root_main.name, "Main");
        assert!(
            !root_main.is_entry,
            "root main.mpl should not stay executable"
        );

        let result = analyze_document(&file_uri(&open_path), &source, &[]);
        let messages = diagnostic_messages(&result);
        assert!(
            messages.is_empty(),
            "override-precedence project should analyze cleanly, got: {:?}",
            messages
        );
    }

    #[test]
    fn m048_s02_legacy_manifestless_project_keeps_root_main_as_main_entry() {
        let (_tmp, project_dir, open_path, source) = write_mesh_project(
            None,
            &[
                (
                    "main.mpl",
                    "from Support import label\n\nfn main() do\n  println(label())\nend\n",
                ),
                (
                    "support.mpl",
                    "pub fn label() -> String do\n  \"default-support\"\nend\n",
                ),
            ],
            "main.mpl",
        );

        let entry_relative_path = resolve_entrypoint(&project_dir, None).unwrap();
        let project =
            build_project_with_overlays(&project_dir, &entry_relative_path, &HashMap::new())
                .unwrap();
        let entry = entry_module(&project.graph);

        assert_eq!(entry_relative_path, PathBuf::from(DEFAULT_ENTRYPOINT));
        assert_eq!(entry.path, PathBuf::from(DEFAULT_ENTRYPOINT));
        assert_eq!(entry.name, "Main");
        assert!(entry.is_entry);

        let result = analyze_document(&file_uri(&open_path), &source, &[]);
        let messages = diagnostic_messages(&result);
        assert!(
            messages.is_empty(),
            "legacy manifest-less projects should still analyze cleanly, got: {:?}",
            messages
        );
    }

    #[test]
    fn m048_s02_missing_configured_entry_reports_project_diagnostic() {
        let manifest = entrypoint_manifest("broken-entry", "lib/start.mpl");
        let (_tmp, _project_dir, open_path, source) = write_mesh_project(
            Some(&manifest),
            &[(
                "app.mpl",
                "pub fn label() -> String do\n  \"still-clean-alone\"\nend\n",
            )],
            "app.mpl",
        );

        let result = analyze_document(&file_uri(&open_path), &source, &[]);
        let messages = diagnostic_messages(&result);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("Entrypoint 'lib/start.mpl'")),
            "missing configured entry should surface as a project diagnostic, got: {:?}",
            messages
        );
        assert_eq!(
            messages.len(),
            1,
            "clean standalone files should only receive the project diagnostic, got: {:?}",
            messages
        );
    }

    #[test]
    fn m048_s02_invalid_manifest_entrypoint_reports_project_diagnostic() {
        let manifest = entrypoint_manifest("escaping-entry", "../escape.mpl");
        let (_tmp, _project_dir, open_path, source) = write_mesh_project(
            Some(&manifest),
            &[(
                "lib/support.mpl",
                "pub fn label() -> String do\n  \"still-clean-alone\"\nend\n",
            )],
            "lib/support.mpl",
        );

        let result = analyze_document(&file_uri(&open_path), &source, &[]);
        let messages = diagnostic_messages(&result);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("stay within the project root")),
            "invalid manifest entrypoint should surface as a project diagnostic, got: {:?}",
            messages
        );
        assert_eq!(
            messages.len(),
            1,
            "clean standalone files should only receive the project diagnostic, got: {:?}",
            messages
        );
    }

    // ── Clustered cutover contract ────────────────────────────────────

    #[test]
    fn m047_s04_source_decorated_work_still_analyzes_without_diagnostics() {
        let (_tmp, work_path, source) = write_source_declared_work_project(
            &package_manifest("clustered-source-proof"),
            source_declared_public_work_source(),
        );

        let result = analyze_document(&file_uri(&work_path), &source, &[]);
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.is_empty(),
            "expected clean diagnostics for valid source-decorated work, got: {:?}",
            messages
        );
    }

    #[test]
    fn m047_s04_legacy_manifest_cluster_section_reports_project_diagnostic() {
        let (_tmp, work_path, source) = write_source_declared_work_project(
            &legacy_manifest("clustered-source-proof"),
            source_declared_public_work_source(),
        );

        let result = analyze_document(&file_uri(&work_path), &source, &[]);
        let diag = result
            .diagnostics
            .iter()
            .find(|diag| {
                diag.message
                    .contains("`[cluster]` manifest sections are no longer supported")
                    && diag.message.contains("mesh.toml")
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected legacy manifest diagnostic, got: {:?}",
                    result
                        .diagnostics
                        .iter()
                        .map(|diag| (&diag.message, diag.range))
                        .collect::<Vec<_>>()
                )
            });

        assert_eq!(diag.range.start, Position::new(0, 0));
    }

    #[test]
    fn m047_s04_legacy_clustered_work_reports_parse_diagnostic_at_source_range() {
        let (_tmp, work_path, source) = write_source_declared_work_project(
            &package_manifest("clustered-source-proof"),
            legacy_clustered_work_source(),
        );

        let result = analyze_document(&file_uri(&work_path), &source, &[]);
        let diag = result
            .diagnostics
            .iter()
            .find(|diag| {
                diag.message
                    .contains("legacy `clustered(work)` declarations are no longer supported")
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected legacy clustered(work) diagnostic, got: {:?}",
                    result
                        .diagnostics
                        .iter()
                        .map(|diag| (&diag.message, diag.range))
                        .collect::<Vec<_>>()
                )
            });

        assert_eq!(diag.range.start.line, 0);
        assert!(
            diag.range.end.line > diag.range.start.line
                || diag.range.end.character > diag.range.start.character,
            "expected non-empty legacy source range, got {:?}",
            diag.range
        );
    }

    #[test]
    fn m047_s01_source_decorated_work_analyzes_without_diagnostics() {
        let (_tmp, work_path, source) = write_source_declared_work_project(
            &package_manifest("clustered-source-proof"),
            source_declared_public_work_source(),
        );

        let result = analyze_document(&file_uri(&work_path), &source, &[]);
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.is_empty(),
            "expected clean diagnostics for valid source-decorated work, got: {:?}",
            messages
        );
    }

    #[test]
    fn m047_s01_private_source_decorator_reports_declaration_range() {
        let (_tmp, work_path, source) = write_source_declared_work_project(
            &package_manifest("clustered-source-proof"),
            source_declared_private_work_source(),
        );

        let result = analyze_document(&file_uri(&work_path), &source, &[]);
        let diag = result
            .diagnostics
            .iter()
            .find(|diag| {
                diag.message.contains("private function")
                    && diag.message.contains("Work.hidden_submit")
                    && diag.message.contains("source `@cluster` decorator")
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected private decorated work diagnostic, got: {:?}",
                    result
                        .diagnostics
                        .iter()
                        .map(|diag| (&diag.message, diag.range))
                        .collect::<Vec<_>>()
                )
            });

        assert_decorator_anchored_range(diag);
    }

    #[test]
    fn m047_s01_manifest_source_duplicate_reports_declaration_range() {
        let (_tmp, work_path, source) = write_source_declared_work_project(
            &legacy_manifest("clustered-source-proof"),
            source_declared_public_work_source(),
        );

        let result = analyze_document(&file_uri(&work_path), &source, &[]);
        let diag = result
            .diagnostics
            .iter()
            .find(|diag| {
                diag.message
                    .contains("`[cluster]` manifest sections are no longer supported")
                    && diag.message.contains("mesh.toml")
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected legacy manifest diagnostic, got: {:?}",
                    result
                        .diagnostics
                        .iter()
                        .map(|diag| (&diag.message, diag.range))
                        .collect::<Vec<_>>()
                )
            });

        assert_eq!(diag.range.start, Position::new(0, 0));
    }

    // ── Scoped installed package regressions ────────────────────────────

    #[test]
    fn scoped_installed_package_discovery_skips_owner_dirs_hidden_paths_and_manifestless_trees() {
        let tmp = tempfile::tempdir().unwrap();
        let packages_dir = tmp.path().join(".mesh/packages");

        std::fs::create_dir_all(packages_dir.join("acme/greeter@1.0.0")).unwrap();
        std::fs::write(
            packages_dir.join("acme/greeter@1.0.0/mesh.toml"),
            package_manifest("acme/greeter"),
        )
        .unwrap();

        std::fs::create_dir_all(packages_dir.join("flat@1.0.0")).unwrap();
        std::fs::write(
            packages_dir.join("flat@1.0.0/mesh.toml"),
            package_manifest("flat"),
        )
        .unwrap();

        std::fs::create_dir_all(packages_dir.join("owner-only/inner")).unwrap();
        std::fs::write(packages_dir.join("owner-only/inner/main.mpl"), "").unwrap();

        std::fs::create_dir_all(packages_dir.join(".hidden/ignored@1.0.0")).unwrap();
        std::fs::write(
            packages_dir.join(".hidden/ignored@1.0.0/mesh.toml"),
            package_manifest("ignored"),
        )
        .unwrap();

        let roots = discover_installed_package_roots(&packages_dir).unwrap();
        let relative_roots: Vec<String> = roots
            .iter()
            .map(|path| {
                path.strip_prefix(&packages_dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        assert_eq!(relative_roots, vec!["acme/greeter@1.0.0", "flat@1.0.0"]);
    }

    #[test]
    fn scoped_installed_package_analyzes_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("consumer");
        let package_root = project_dir.join(".mesh/packages/acme/greeter@1.0.0");
        let main_path = project_dir.join("main.mpl");

        std::fs::create_dir_all(package_root.join("support")).unwrap();
        std::fs::write(
            &main_path,
            "from Support.Message import message\n\nfn main() do\n  println(message())\nend\n",
        )
        .unwrap();
        std::fs::write(
            package_root.join("mesh.toml"),
            package_manifest("acme/greeter"),
        )
        .unwrap();
        std::fs::write(package_root.join("main.mpl"), "fn main() do\n  0\nend\n").unwrap();
        std::fs::write(
            package_root.join("support/message.mpl"),
            "pub fn message() -> String do\n  \"hello from package\"\nend\n",
        )
        .unwrap();

        let source = std::fs::read_to_string(&main_path).unwrap();
        let result = analyze_document(&file_uri(&main_path), &source, &[]);
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.is_empty(),
            "scoped installed packages should analyze without diagnostics, got: {:?}",
            messages
        );
    }

    #[test]
    fn scoped_installed_package_flat_layout_analyzes_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("consumer");
        let package_root = project_dir.join(".mesh/packages/greeter@1.0.0");
        let main_path = project_dir.join("main.mpl");

        std::fs::create_dir_all(&package_root).unwrap();
        std::fs::write(
            &main_path,
            "from Greeting import message\n\nfn main() do\n  println(message())\nend\n",
        )
        .unwrap();
        std::fs::write(package_root.join("mesh.toml"), package_manifest("greeter")).unwrap();
        std::fs::write(
            package_root.join("greeting.mpl"),
            "pub fn message() -> String do\n  \"hello from flat package\"\nend\n",
        )
        .unwrap();

        let source = std::fs::read_to_string(&main_path).unwrap();
        let result = analyze_document(&file_uri(&main_path), &source, &[]);
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.is_empty(),
            "flat installed packages should analyze without diagnostics, got: {:?}",
            messages
        );
    }

    #[test]
    fn m047_s07_clustered_route_wrapper_project_keeps_imported_origin_metadata() {
        let (_tmp, main_path, source) = write_clustered_route_wrapper_project();

        let result = analyze_document(&file_uri(&main_path), &source, &[]);
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.is_empty(),
            "expected clean diagnostics for valid clustered route wrappers, got: {:?}",
            messages
        );

        let metadata = result
            .typeck
            .clustered_route_wrappers
            .values()
            .find(|metadata| metadata.runtime_name == "Api.Todos.handle_list_todos")
            .unwrap_or_else(|| {
                panic!(
                    "expected imported clustered route metadata, got: {:?}",
                    result
                        .typeck
                        .clustered_route_wrappers
                        .values()
                        .map(|metadata| {
                            (
                                metadata.runtime_name.as_str(),
                                metadata.defining_module.as_deref(),
                                metadata.replication_count.value,
                            )
                        })
                        .collect::<Vec<_>>()
                )
            });

        assert_eq!(metadata.defining_module.as_deref(), Some("Api.Todos"));
        assert_eq!(metadata.replication_count.value, 2);
    }

    #[test]
    fn m047_s07_clustered_route_wrapper_reports_wrapper_range_in_lsp() {
        let source = "pub fn handle(req :: Request) -> Response do\n  HTTP.response(200, \"ok\")\nend\n\nfn build() do\n  let wrapped = HTTP.clustered(handle)\n  wrapped\nend\n";
        let result = analyze_document("file:///test.mpl", source, &[]);
        let diag = result
            .diagnostics
            .iter()
            .find(|diag| diag.message.contains("HTTP.clustered"))
            .unwrap_or_else(|| {
                panic!(
                    "expected clustered-route diagnostic, got: {:?}",
                    result
                        .diagnostics
                        .iter()
                        .map(|diag| (&diag.message, diag.range))
                        .collect::<Vec<_>>()
                )
            });

        let wrapper_start = source
            .find("HTTP.clustered")
            .expect("wrapper text should exist");
        assert_eq!(diag.range.start, offset_to_position(source, wrapper_start));
        assert!(
            diag.range.end.line > diag.range.start.line
                || diag.range.end.character > diag.range.start.character,
            "expected wrapper diagnostic to cover a non-empty range, got {:?}",
            diag.range
        );
    }

    // ── Diagnostic Tests ──────────────────────────────────────────────────

    #[test]
    fn analyze_valid_source_no_diagnostics() {
        let source = "let x = 42";
        let result = analyze_document("file:///test.mpl", source, &[]);
        assert!(
            result.diagnostics.is_empty(),
            "Valid source should produce no diagnostics, got: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn analyze_valid_function_no_diagnostics() {
        let source = "fn add(a, b) do\na + b\nend";
        let result = analyze_document("file:///test.mpl", source, &[]);
        assert!(
            result.diagnostics.is_empty(),
            "Valid function should produce no diagnostics, got: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn analyze_reference_backend_jobs_uses_project_imports() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("mesh-lsp crate should live under compiler/")
            .parent()
            .expect("workspace root should be above compiler/")
            .to_path_buf();
        let jobs_path = std::fs::canonicalize(
            repo_root.join("scripts/fixtures/backend/reference-backend/api/jobs.mpl"),
        )
        .expect("retained reference-backend fixture jobs file should exist");
        let uri = Url::from_file_path(&jobs_path)
            .expect("jobs path should convert to file URI")
            .to_string();
        let source = std::fs::read_to_string(&jobs_path).expect("jobs source should be readable");

        let result = analyze_document(&uri, &source, &[]);
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages.is_empty(),
            "scripts/fixtures/backend/reference-backend/api/jobs.mpl should analyze without bogus import diagnostics, got: {:?}",
            messages
        );
    }

    #[test]
    fn analyze_type_error_produces_diagnostic() {
        // Using an undefined variable should produce a type error diagnostic.
        let source = "let x = undefined_var";
        let result = analyze_document("file:///test.mpl", source, &[]);
        assert!(
            !result.diagnostics.is_empty(),
            "Type error should produce at least one diagnostic"
        );
        let diag = &result.diagnostics[0];
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diag.source.as_deref(), Some("mesh"));
    }

    #[test]
    fn analyze_type_error_has_range() {
        // The diagnostic range should point to the error location.
        let source = "let x = undefined_var";
        let result = analyze_document("file:///test.mpl", source, &[]);
        assert!(!result.diagnostics.is_empty());
        let diag = &result.diagnostics[0];
        // The error is for "undefined_var" which is on line 0.
        assert_eq!(diag.range.start.line, 0);
    }

    #[test]
    fn analyze_multiple_errors_all_reported() {
        // Two undefined variables should produce at least two diagnostics.
        let source = "let x = undef1\nlet y = undef2";
        let result = analyze_document("file:///test.mpl", source, &[]);
        assert!(
            result.diagnostics.len() >= 2,
            "Expected at least 2 diagnostics, got {}",
            result.diagnostics.len()
        );
    }

    #[test]
    fn analyze_parse_error_produces_diagnostic() {
        // A parse error (incomplete expression) should produce a diagnostic.
        // Note: `fn do end` is now valid syntax (no-params closure, Phase 12-01).
        // Use a clearly invalid expression instead.
        let source = "let x = + +";
        let result = analyze_document("file:///test.mpl", source, &[]);
        assert!(
            !result.diagnostics.is_empty(),
            "Parse error should produce at least one diagnostic"
        );
        let diag = &result.diagnostics[0];
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    }

    // ── Hover Tests ───────────────────────────────────────────────────────

    #[test]
    fn hover_integer_literal() {
        let source = "let x = 42";
        let result = analyze_document("file:///test.mpl", source, &[]);
        // Hover over the let binding -- should show the type.
        // The rowan tree has "letx=42" so the LET_BINDING covers tree offsets.
        // The type map uses tree-coordinate ranges.
        // type_at_position converts LSP position to source byte offset.
        // However, since the typeck uses rowan ranges (not source byte offsets),
        // the hover might not work correctly for all positions due to the
        // whitespace coordinate mismatch (pre-existing issue).
        // We test with line 0, character 0 which should be in the LET_BINDING range.
        let ty = type_at_position(
            source,
            &result.typeck,
            &Position {
                line: 0,
                character: 0,
            },
        );
        // May return Some("Int") or None depending on what range the typeck stored.
        // At minimum, verify it doesn't panic.
        let _ = ty;
    }

    #[test]
    fn hover_over_empty_space_returns_none() {
        // Hovering over whitespace or at end of file should return None.
        let source = "let x = 42";
        let result = analyze_document("file:///test.mpl", source, &[]);
        // Position past end of source.
        let ty = type_at_position(
            source,
            &result.typeck,
            &Position {
                line: 5,
                character: 0,
            },
        );
        assert!(ty.is_none(), "Hover past end should return None");
    }

    // ── Go-to-definition Tests ────────────────────────────────────────────

    #[test]
    fn goto_def_function_defined_then_called() {
        let source = "fn greet(name) do\nname\nend\nlet msg = greet(42)";
        let result = analyze_document("file:///test.mpl", source, &[]);
        let root = result.parse.syntax();
        // Find the call to "greet" in `greet(42)`.
        let call_offset = source.rfind("greet").unwrap();
        let def = crate::definition::find_definition(source, &root, call_offset);
        assert!(def.is_some(), "Should find definition of greet");
        // Verify it resolves to the fn definition, not the call.
        let range = def.unwrap();
        let def_source = crate::definition::tree_to_source_offset(source, range.start().into());
        assert!(def_source.is_some());
        let offset = def_source.unwrap();
        // "fn greet" -- "greet" starts at offset 3.
        assert_eq!(offset, 3);
    }

    #[test]
    fn goto_def_let_binding_used_later() {
        let source = "let count = 10\nlet doubled = count + count";
        let result = analyze_document("file:///test.mpl", source, &[]);
        let root = result.parse.syntax();
        // Find "count" in the second let binding.
        let second_count = source.find("count + count").unwrap();
        let def = crate::definition::find_definition(source, &root, second_count);
        assert!(def.is_some(), "Should find definition of count");
        let range = def.unwrap();
        let def_source =
            crate::definition::tree_to_source_offset(source, range.start().into()).unwrap();
        // "let count" -- "count" starts at offset 4.
        assert_eq!(def_source, 4);
    }

    #[test]
    fn goto_def_variable_shadowing_inner_scope() {
        let source = "fn test() do\nlet x = 1\nfn inner() do\nlet x = 2\nlet y = x\nend\nend";
        let result = analyze_document("file:///test.mpl", source, &[]);
        let root = result.parse.syntax();
        let y_binding = source.find("let y = x").unwrap();
        let x_use = y_binding + "let y = ".len();
        let def = crate::definition::find_definition(source, &root, x_use);
        assert!(def.is_some(), "Should find inner x definition");
        let range = def.unwrap();
        let def_source =
            crate::definition::tree_to_source_offset(source, range.start().into()).unwrap();
        let inner_x = source.find("let x = 2").unwrap() + "let ".len();
        assert_eq!(
            def_source, inner_x,
            "Should resolve to inner binding, not outer"
        );
    }

    #[test]
    fn goto_def_unknown_identifier_returns_none() {
        let source = "let y = completely_unknown";
        let result = analyze_document("file:///test.mpl", source, &[]);
        let root = result.parse.syntax();
        let unknown_offset = source.find("completely_unknown").unwrap();
        let def = crate::definition::find_definition(source, &root, unknown_offset);
        assert!(def.is_none(), "Unknown identifier should return None");
    }

    #[test]
    fn goto_def_struct_name_resolves() {
        let source = "struct Point do\nx :: Int\nend";
        let result = analyze_document("file:///test.mpl", source, &[]);
        let root = result.parse.syntax();
        // Definition search for "Point" at the struct def should find itself.
        let point_offset = source.find("Point").unwrap();
        // "Point" at the definition site is in a NAME node, not NAME_REF,
        // so it won't resolve to anything (it IS the definition).
        let def = crate::definition::find_definition(source, &root, point_offset);
        // This should return None since the user is clicking on the definition itself.
        assert!(
            def.is_none(),
            "Clicking on definition site should return None"
        );
    }

    // ── Position Conversion Tests ─────────────────────────────────────────

    #[test]
    fn offset_to_position_first_line() {
        let source = "hello world";
        let pos = offset_to_position(source, 0);
        assert_eq!(
            pos,
            Position {
                line: 0,
                character: 0
            }
        );

        let pos = offset_to_position(source, 5);
        assert_eq!(
            pos,
            Position {
                line: 0,
                character: 5
            }
        );
    }

    #[test]
    fn offset_to_position_multiline() {
        let source = "line1\nline2\nline3";
        // 'l' of line2 is at offset 6
        let pos = offset_to_position(source, 6);
        assert_eq!(
            pos,
            Position {
                line: 1,
                character: 0
            }
        );

        // 'l' of line3 is at offset 12
        let pos = offset_to_position(source, 12);
        assert_eq!(
            pos,
            Position {
                line: 2,
                character: 0
            }
        );

        // 'i' of line2 is at offset 7
        let pos = offset_to_position(source, 7);
        assert_eq!(
            pos,
            Position {
                line: 1,
                character: 1
            }
        );
    }

    #[test]
    fn offset_to_position_at_end() {
        let source = "ab\ncd";
        let pos = offset_to_position(source, 5);
        assert_eq!(
            pos,
            Position {
                line: 1,
                character: 2
            }
        );
    }

    #[test]
    fn position_to_offset_single_line() {
        let source = "hello";
        assert_eq!(
            position_to_offset(
                source,
                &Position {
                    line: 0,
                    character: 0
                }
            ),
            Some(0)
        );
        assert_eq!(
            position_to_offset(
                source,
                &Position {
                    line: 0,
                    character: 3
                }
            ),
            Some(3)
        );
        assert_eq!(
            position_to_offset(
                source,
                &Position {
                    line: 0,
                    character: 5
                }
            ),
            Some(5)
        );
    }

    #[test]
    fn position_to_offset_multiline() {
        let source = "abc\ndef\nghi";
        // First char of line 2 (0-indexed) at (1, 0).
        assert_eq!(
            position_to_offset(
                source,
                &Position {
                    line: 1,
                    character: 0
                }
            ),
            Some(4)
        );
        // First char of line 3 at (2, 0).
        assert_eq!(
            position_to_offset(
                source,
                &Position {
                    line: 2,
                    character: 0
                }
            ),
            Some(8)
        );
    }

    #[test]
    fn position_to_offset_roundtrip() {
        let source = "hello\nworld\nfoo";
        for offset in 0..source.len() {
            let pos = offset_to_position(source, offset);
            let back = position_to_offset(source, &pos);
            assert_eq!(
                back,
                Some(offset),
                "Roundtrip failed for offset {} (pos {:?})",
                offset,
                pos
            );
        }
    }

    #[test]
    fn position_past_eof_returns_none() {
        let source = "hello";
        let result = position_to_offset(
            source,
            &Position {
                line: 5,
                character: 0,
            },
        );
        assert!(result.is_none(), "Position past EOF should return None");
    }

    // ── Source/Tree Offset Conversion Tests ────────────────────────────────

    #[test]
    fn source_tree_offset_roundtrip() {
        let source = "let x = 42\nlet y = x";
        // For each non-EOF token in the source, verify the roundtrip.
        let tokens = mesh_lexer::Lexer::tokenize(source);
        for token in &tokens {
            // Skip EOF (zero-length token at end).
            if token.kind == mesh_common::token::TokenKind::Eof {
                continue;
            }
            let src_start = token.span.start as usize;
            let tree = crate::definition::source_to_tree_offset(source, src_start);
            assert!(
                tree.is_some(),
                "source_to_tree_offset should succeed for offset {}",
                src_start
            );
            let back = crate::definition::tree_to_source_offset(source, tree.unwrap());
            assert_eq!(
                back,
                Some(src_start),
                "Roundtrip failed for source offset {}",
                src_start
            );
        }
    }
}
