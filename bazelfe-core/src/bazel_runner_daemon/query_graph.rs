use crate::jvm_indexer::bazel_query::BazelQuery;
use ::prost::Message;
use bazelfe_protos::*;

pub async fn graph_query<B: BazelQuery + ?Sized, Q: AsRef<str>>(
    bazel_query: &B,
    query: Q,
) -> Result<blaze_query::QueryResult, Box<dyn std::error::Error>> {
    let res = bazel_query
        .execute(&vec![
            String::from("query"),
            String::from("--keep_going"),
            String::from("--output"),
            String::from("proto"),
            String::from(query.as_ref()),
        ])
        .await;

    Ok(blaze_query::QueryResult::decode(&*res.stdout_raw)?)
}
