mod context_dependency_helper;
mod context_helper;
mod harmony_export_dependency_scanner;
mod harmony_import_dependency_scanner;
mod parser;
mod util;

use rspack_ast::javascript::Program;
use rspack_core::needs_refactor::WorkerSyntaxList;
use rspack_core::{
  AsyncDependenciesBlock, BoxDependency, BoxDependencyTemplate, BuildInfo, DependencyLocation,
};
use rspack_core::{BuildMeta, CompilerOptions, ModuleIdentifier, ModuleType, ResourceData};
use rspack_error::miette::Diagnostic;
use rustc_hash::{FxHashMap, FxHashSet};
use swc_core::common::comments::Comments;
use swc_core::common::{SourceFile, Span};
use swc_core::ecma::atoms::Atom;

pub use self::context_dependency_helper::create_context_dependency;
pub use self::context_helper::{scanner_context_module, ContextModuleScanResult};
use self::harmony_export_dependency_scanner::HarmonyExportDependencyScanner;
use self::harmony_import_dependency_scanner::HarmonyImportRefDependencyScanner;
pub use self::harmony_import_dependency_scanner::{ImportMap, ImporterReferenceInfo};
pub use self::parser::{CallExpressionInfo, CallHooksName, ExportedVariableInfo};
pub use self::parser::{JavascriptParser, MemberExpressionInfo, TagInfoData, TopLevelScope};
pub use self::util::*;

pub struct ScanDependenciesResult {
  pub dependencies: Vec<BoxDependency>,
  pub blocks: Vec<AsyncDependenciesBlock>,
  pub presentational_dependencies: Vec<BoxDependencyTemplate>,
  pub usage_span_record: FxHashMap<Span, ExtraSpanInfo>,
  pub import_map: ImportMap,
  pub warning_diagnostics: Vec<Box<dyn Diagnostic + Send + Sync>>,
}

#[derive(Debug, Clone, Default)]
pub enum ExtraSpanInfo {
  #[default]
  ReWriteUsedByExports,
  // (symbol, usage)
  // (local, exported) refer https://github.com/webpack/webpack/blob/ac7e531436b0d47cd88451f497cdfd0dad41535d/lib/javascript/JavascriptParser.js#L2347-L2352
  AddVariableUsage(Vec<(Atom, Atom)>),
}

#[allow(clippy::too_many_arguments)]
pub fn scan_dependencies(
  source_file: &SourceFile,
  program: &Program,
  worker_syntax_list: &mut WorkerSyntaxList,
  resource_data: &ResourceData,
  compiler_options: &CompilerOptions,
  module_type: &ModuleType,
  build_info: &mut BuildInfo,
  build_meta: &mut BuildMeta,
  module_identifier: ModuleIdentifier,
) -> Result<ScanDependenciesResult, Vec<Box<dyn Diagnostic + Send + Sync>>> {
  let mut warning_diagnostics: Vec<Box<dyn Diagnostic + Send + Sync>> = Vec::with_capacity(32);
  let mut errors = Vec::with_capacity(32);
  let mut dependencies = Vec::with_capacity(256);
  let mut blocks = Vec::with_capacity(256);
  let mut presentational_dependencies = Vec::with_capacity(256);
  let comments = program.comments.clone();
  let mut parser_exports_state = None;
  let mut ignored: FxHashSet<DependencyLocation> = FxHashSet::default();
  let mut import_map = FxHashMap::default();
  let mut rewrite_usage_span = FxHashMap::default();

  let mut parser = JavascriptParser::new(
    source_file,
    compiler_options,
    &mut dependencies,
    &mut presentational_dependencies,
    &mut blocks,
    &mut ignored,
    &mut import_map,
    &mut rewrite_usage_span,
    comments.as_ref().map(|c| c as &dyn Comments),
    &module_identifier,
    module_type,
    worker_syntax_list,
    resource_data,
    &mut parser_exports_state,
    build_meta,
    build_info,
    &mut errors,
    &mut warning_diagnostics,
  );

  parser.walk_program(program.get_inner_program());

  if module_type.is_js_auto() || module_type.is_js_esm() {
    program.visit_with(&mut HarmonyImportRefDependencyScanner::new(
      &import_map,
      &mut dependencies,
      &mut rewrite_usage_span,
      &mut ignored,
    ));
    let comments = program.comments.as_ref();
    program.visit_with(&mut HarmonyExportDependencyScanner::new(
      &mut dependencies,
      &mut presentational_dependencies,
      &import_map,
      build_info,
      &mut rewrite_usage_span,
      comments,
      &mut ignored,
    ));
  }

  if errors.is_empty() {
    Ok(ScanDependenciesResult {
      dependencies,
      blocks,
      presentational_dependencies,
      usage_span_record: rewrite_usage_span,
      import_map,
      warning_diagnostics,
    })
  } else {
    Err(errors)
  }
}
