use std::{collections::HashMap, sync::Arc};

use napi_derive::napi;
use rspack_core::CompilerOptionsBuilder;
use rspack_plugin_split_chunks::{CacheGroupOptions, ChunkType, SplitChunksOptions, TestFn};
use serde::Deserialize;

use crate::RawOption;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct RawSplitChunksOptions {
  pub cache_groups: Option<HashMap<String, RawCacheGroupOptions>>,
  /// What kind of chunks should be selected.
  pub chunks: Option<String>,
  //   pub automatic_name_delimiter: String,
  pub max_async_requests: Option<u32>,
  pub max_initial_requests: Option<u32>,
  //   pub default_size_types: Option<Vec<SizeType>>,
  pub min_chunks: Option<u32>,
  // hide_path_info: bool,
  pub min_size: Option<f64>,
  //   pub min_size_reduction: usize,
  pub enforce_size_threshold: Option<f64>,
  pub min_remaining_size: Option<f64>,
  // layer: String,
  //   pub max_size: usize,
  //   pub max_async_size: usize,
  //   pub max_initial_size: usize,
}

impl RawOption<SplitChunksOptions> for RawSplitChunksOptions {
  #[allow(clippy::field_reassign_with_default)]
  fn to_compiler_option(
    self,
    _options: &CompilerOptionsBuilder,
  ) -> anyhow::Result<SplitChunksOptions> {
    let mut defaults = SplitChunksOptions::default();
    defaults.max_async_requests = self.max_async_requests;
    defaults.max_initial_requests = self.max_initial_requests;
    defaults.min_chunks = self.min_chunks;
    defaults.min_size = self.min_size;
    defaults.enforce_size_threshold = self.enforce_size_threshold;
    defaults.min_remaining_size = self.min_remaining_size;
    defaults.chunks = self.chunks.map(|chunks| match chunks.as_str() {
      "initial" => ChunkType::Initial,
      "async" => ChunkType::Async,
      "all" => ChunkType::All,
      _ => panic!("Invalid chunk type: {chunks}"),
    });

    defaults
      .cache_groups
      .extend(
        self
          .cache_groups
          .unwrap_or_default()
          .into_iter()
          .map(|(k, v)| {
            (
              k,
              CacheGroupOptions {
                name: v.name,
                priority: v.priority,
                reuse_existing_chunk: v.reuse_existing_chunk,
                test: v.test.clone().map(|test| {
                  let f: TestFn = Arc::new(move |module| {
                    let re = rspack_regex::RspackRegex::new(&test)
                      .unwrap_or_else(|_| panic!("Invalid regex: {}", &test));
                    module
                      .name_for_condition()
                      .map_or(false, |name| re.test(&name))
                  });
                  f
                }),
                chunks: v.chunks.map(|chunks| match chunks.as_str() {
                  "initial" => ChunkType::Initial,
                  "async" => ChunkType::Async,
                  "all" => ChunkType::All,
                  _ => panic!("Invalid chunk type: {chunks}"),
                }),
                min_chunks: v.min_chunks,
                ..Default::default()
              },
            )
          }),
      );
    Ok(defaults)
  }

  fn fallback_value(_options: &CompilerOptionsBuilder) -> Self {
    RawSplitChunksOptions {
      cache_groups: None,
      chunks: None,
      max_async_requests: None,
      max_initial_requests: None,
      min_chunks: None,
      min_size: None,
      enforce_size_threshold: None,
      min_remaining_size: None,
    }
  }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct RawCacheGroupOptions {
  pub priority: Option<i32>,
  pub reuse_existing_chunk: Option<bool>,
  //   pub r#type: SizeType,
  pub test: Option<String>,
  //   pub filename: String,
  //   pub enforce: bool,
  //   pub id_hint: String,
  /// What kind of chunks should be selected.
  pub chunks: Option<String>,
  //   pub automatic_name_delimiter: String,
  //   pub max_async_requests: usize,
  //   pub max_initial_requests: usize,
  pub min_chunks: Option<u32>,
  // hide_path_info: bool,
  //   pub min_size: usize,
  //   pub min_size_reduction: usize,
  //   pub enforce_size_threshold: usize,
  //   pub min_remaining_size: usize,
  // layer: String,
  //   pub max_size: usize,
  //   pub max_async_size: usize,
  //   pub max_initial_size: usize,
  pub name: Option<String>,
  // used_exports: bool,
}
