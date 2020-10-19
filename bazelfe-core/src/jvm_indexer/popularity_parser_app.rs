

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ret = bazelfe_core::jvm_indexer::popularity_parser::build_popularity_map();
    for (k, v) in ret {
        println!("{} - {:#?}", v, k);
    }
    Ok(())
}
