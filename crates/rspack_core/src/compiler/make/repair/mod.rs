mod add;
mod build;
mod factorize;
mod process_dependencies;

use std::{hash::BuildHasherDefault, path::PathBuf, sync::Arc};

use indexmap::IndexSet;
use rspack_error::{Diagnostic, Result};
use rspack_identifier::{IdentifierMap, IdentifierSet};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet, FxHasher};

use super::MakeArtifact;
use crate::{
  cache::Cache,
  module_graph::{ModuleGraph, ModuleGraphPartial},
  tree_shaking::visitor::OptimizeAnalyzeResult,
  utils::task_loop::{run_task_loop, Task},
  BuildDependency, CacheCount, CacheOptions, Compilation, CompilationLogger, CompilerOptions,
  DependencyType, Logger, Module, ModuleFactory, ModuleIdentifier, ModuleProfile,
  NormalModuleSource, ResolverFactory, SharedPluginDriver,
};

struct MakeTaskContext {
  // compilation info
  plugin_driver: SharedPluginDriver,
  compiler_options: Arc<CompilerOptions>,
  resolver_factory: Arc<ResolverFactory>,
  loader_resolver_factory: Arc<ResolverFactory>,
  cache: Arc<Cache>,
  dependency_factories: HashMap<DependencyType, Arc<dyn ModuleFactory>>,

  // TODO move outof context
  logger: CompilationLogger,
  build_cache_counter: Option<CacheCount>,
  factorize_cache_counter: Option<CacheCount>,
  //  add_timer: StartTimeAggregate,
  //  process_deps_timer: StartTimeAggregate,
  //  factorize_timer: StartTimeAggregate,
  //  build_timer: StartTimeAggregate,
  /// Collecting all module that need to skip in tree-shaking ast modification phase
  //  bailout_module_identifiers: IdentifierMap<BailoutFlag>,
  // TODO change to artifact
  module_graph_partial: ModuleGraphPartial,
  make_failed_dependencies: HashSet<BuildDependency>,
  make_failed_module: HashSet<ModuleIdentifier>,

  entry_module_identifiers: IdentifierSet,
  diagnostics: Vec<Diagnostic>,
  optimize_analyze_result_map: IdentifierMap<OptimizeAnalyzeResult>,
  file_dependencies: IndexSet<PathBuf, BuildHasherDefault<FxHasher>>,
  context_dependencies: IndexSet<PathBuf, BuildHasherDefault<FxHasher>>,
  missing_dependencies: IndexSet<PathBuf, BuildHasherDefault<FxHasher>>,
  build_dependencies: IndexSet<PathBuf, BuildHasherDefault<FxHasher>>,
  has_module_graph_change: bool,
}

impl MakeTaskContext {
  fn new(compilation: &Compilation, artifact: MakeArtifact) -> Self {
    let logger = compilation.get_logger("rspack.Compilation");
    let mut build_cache_counter = None;
    let mut factorize_cache_counter = None;
    if !(matches!(compilation.options.cache, CacheOptions::Disabled)) {
      build_cache_counter = Some(logger.cache("module build cache"));
      factorize_cache_counter = Some(logger.cache("module factorize cache"));
    }

    Self {
      plugin_driver: compilation.plugin_driver.clone(),
      compiler_options: compilation.options.clone(),
      resolver_factory: compilation.resolver_factory.clone(),
      loader_resolver_factory: compilation.loader_resolver_factory.clone(),
      cache: compilation.cache.clone(),
      dependency_factories: compilation.dependency_factories.clone(),

      // TODO use timer in tasks
      logger,
      build_cache_counter,
      factorize_cache_counter,
      //      add_timer: logger.time_aggregate("module add task"),
      //      process_deps_timer: logger.time_aggregate("module process dependencies task"),
      //      factorize_timer: logger.time_aggregate("module factorize task"),
      //      build_timer: logger.time_aggregate("module build task"),
      module_graph_partial: artifact.module_graph_partial,
      // ignore make_failed_xxx and diagnostics
      make_failed_dependencies: Default::default(),
      make_failed_module: Default::default(),
      diagnostics: Default::default(),

      entry_module_identifiers: artifact.entry_module_identifiers,
      optimize_analyze_result_map: artifact.optimize_analyze_result_map,
      file_dependencies: artifact.file_dependencies,
      context_dependencies: artifact.context_dependencies,
      missing_dependencies: artifact.missing_dependencies,
      build_dependencies: artifact.build_dependencies,
      has_module_graph_change: artifact.has_module_graph_change,
    }
  }

  fn transform_to_make_artifact(self) -> MakeArtifact {
    let Self {
      module_graph_partial,
      make_failed_dependencies,
      make_failed_module,
      diagnostics,
      entry_module_identifiers,
      optimize_analyze_result_map,
      file_dependencies,
      context_dependencies,
      missing_dependencies,
      build_dependencies,
      has_module_graph_change,
      build_cache_counter,
      factorize_cache_counter,
      logger,
      ..
    } = self;
    if let Some(counter) = build_cache_counter {
      logger.cache_end(counter);
    }
    if let Some(counter) = factorize_cache_counter {
      logger.cache_end(counter);
    }
    MakeArtifact {
      module_graph_partial,
      make_failed_dependencies,
      make_failed_module,
      diagnostics,
      entry_module_identifiers,
      optimize_analyze_result_map,
      file_dependencies,
      context_dependencies,
      missing_dependencies,
      build_dependencies,
      has_module_graph_change,
    }
  }

  // TODO use module graph with make artifact
  fn get_module_graph_mut(partial: &mut ModuleGraphPartial) -> ModuleGraph {
    ModuleGraph::new(vec![], Some(partial))
  }
}

pub fn repair(
  compilation: &Compilation,
  mut artifact: MakeArtifact,
  build_dependencies: HashSet<BuildDependency>,
) -> Result<MakeArtifact> {
  let module_graph = artifact.get_module_graph_mut();
  let init_tasks = build_dependencies
    .into_iter()
    .filter_map::<Box<dyn Task<MakeTaskContext>>, _>(|(id, parent_module_identifier)| {
      let dependency = module_graph
        .dependency_by_id(&id)
        .expect("dependency not found");
      // filter module_dependency and context_dependency
      if dependency.as_module_dependency().is_none() && dependency.as_context_dependency().is_none()
      {
        return None;
      }

      // filter parent module existed dependency
      let parent_module =
        parent_module_identifier.and_then(|id| module_graph.module_by_identifier(&id));
      if parent_module_identifier.is_some() && parent_module.is_none() {
        return None;
      }

      let current_profile = compilation
        .options
        .profile
        .then(Box::<ModuleProfile>::default);
      let module_graph = compilation.get_module_graph();
      let original_module_source = parent_module_identifier
        .and_then(|i| module_graph.module_by_identifier(&i))
        .and_then(|m| m.as_normal_module())
        .and_then(|m| {
          if let NormalModuleSource::BuiltSucceed(s) = m.source() {
            Some(s.clone())
          } else {
            None
          }
        });
      Some(Box::new(factorize::FactorizeTask {
        module_factory: compilation.get_dependency_factory(dependency),
        original_module_identifier: parent_module_identifier,
        original_module_source,
        issuer: parent_module
          .and_then(|m| m.as_normal_module())
          .and_then(|module| module.name_for_condition()),
        original_module_context: parent_module.and_then(|m| m.get_context()),
        dependency: dependency.clone(),
        dependencies: vec![id],
        is_entry: parent_module_identifier.is_none(),
        resolve_options: parent_module.and_then(|module| module.get_resolve_options()),
        resolver_factory: compilation.resolver_factory.clone(),
        loader_resolver_factory: compilation.loader_resolver_factory.clone(),
        options: compilation.options.clone(),
        plugin_driver: compilation.plugin_driver.clone(),
        cache: compilation.cache.clone(),
        current_profile,
      }))
    })
    .collect::<Vec<_>>();

  let mut ctx = MakeTaskContext::new(compilation, artifact);
  run_task_loop(&mut ctx, init_tasks)?;
  Ok(ctx.transform_to_make_artifact())
}
