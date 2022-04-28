use std::{collections::HashSet, sync::Arc};

use crate::jvm_indexer::bazel_query::BazelQuery;
use ::prost::Message;
use bazelfe_protos::*;
use tokio::sync::Mutex;

pub async fn graph_query<B: BazelQuery + ?Sized, Q: AsRef<str>>(
    bazel_query: &B,
    query: Q,
    extra_args: &[&str],
    print_cmd_line: bool,
) -> Result<blaze_query::QueryResult, Box<dyn std::error::Error>> {
    let mut query_v = vec![
        String::from("query"),
        String::from("--keep_going"),
        String::from("--output"),
        String::from("proto"),
    ];
    for arg in extra_args {
        query_v.push(arg.to_string());
    }

    query_v.push(String::from(query.as_ref()));

    if print_cmd_line {
        eprintln!("Running bazel query operation: {:#?}", query_v.join(" "));
    }
    // info!("{:#?}", query_v);
    let res = bazel_query.execute(&query_v).await;

    Ok(blaze_query::QueryResult::decode(&*res.stdout_raw)?)
}

pub async fn r_deps(
    bazel_query: Arc<Mutex<Box<dyn BazelQuery>>>,
    target: &str,
) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let bazel_query = bazel_query.lock().await;
    let dependencies_calculated = crate::bazel_query::graph_query(
        bazel_query.as_ref(),
        &format!("r_deps({})", target),
        &["--noimplicit_deps"],
        false,
    )
    .await?;

    let mut result = HashSet::default();

    for target in dependencies_calculated.target.iter() {
        if let Some(rule) = target.rule.as_ref() {
            result.insert(crate::label_utils::sanitize_label(rule.name.to_string()));
        }
    }
    Ok(result)
}

pub async fn in_repo_dependencies(
    bazel_query: Arc<Mutex<Box<dyn BazelQuery>>>,
    target: &str,
) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let bazel_query = bazel_query.lock().await;
    let dependencies_calculated = crate::bazel_query::graph_query(
        bazel_query.as_ref(),
        &format!("deps({}) intersect //...", target),
        &["--noimplicit_deps"],
        false,
    )
    .await?;

    let mut result = HashSet::default();

    for target in dependencies_calculated.target.iter() {
        if let Some(rule) = target.rule.as_ref() {
            result.insert(crate::label_utils::sanitize_label(rule.name.to_string()));
        }
    }
    Ok(result)
}

#[async_trait::async_trait]
pub trait BazelQueryEngine: Send + Sync + std::fmt::Debug {
    async fn dependency_link(
        self: &Self,
        edge_src: &str,
        edge_dest: &str,
    ) -> Result<bool, Box<dyn std::error::Error>>;

    async fn r_deps(
        self: &Self,
        target: &str,
    ) -> Result<HashSet<String>, Box<dyn std::error::Error>>;
}

#[derive(Debug)]
pub struct RealBazelQueryEngine {
    query: Arc<Mutex<Box<dyn crate::jvm_indexer::bazel_query::BazelQuery>>>,
}

impl RealBazelQueryEngine {
    pub fn new(
        query: Arc<Mutex<Box<dyn crate::jvm_indexer::bazel_query::BazelQuery>>>,
    ) -> RealBazelQueryEngine {
        RealBazelQueryEngine { query }
    }
}

#[async_trait::async_trait]
impl BazelQueryEngine for RealBazelQueryEngine {
    async fn r_deps(
        self: &Self,
        target: &str,
    ) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
        let res_set = r_deps(Arc::clone(&self.query), target).await?;

        Ok(res_set)
    }

    async fn dependency_link(
        self: &Self,
        edge_src: &str,
        edge_dest: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let res_set = r_deps(Arc::clone(&self.query), edge_dest).await?;

        // info!("When looking for edges from {} to {}, we found edges: {:#?}", edge_src, edge_dest, res_set);

        Ok(res_set.contains(edge_src))
    }
}
