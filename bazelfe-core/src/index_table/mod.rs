use nom::bytes::complete::take_while1;
use nom::character::complete::digit1;
use nom::character::complete::line_ending;
use nom::error::ParseError;
use nom::multi::{many0, many1};
use nom::{bytes::complete::tag, combinator::map, combinator::opt, sequence::tuple, IResult};
use tokio::sync::RwLock;
use std::{borrow::Cow, collections::HashMap, collections::HashSet, error::Error, sync::Arc};
mod index_table_value;
mod expand_target_to_guesses;
pub use index_table_value::*;

pub struct GuardedGet<'a, 'b>(
    Cow<'b, str>,
    tokio::sync::RwLockReadGuard<'a, HashMap<String,IndexTableValue>>);

impl<'a, 'b> GuardedGet<'a, 'b> {
    pub fn get(&self) -> Option<&IndexTableValue> {
        self.1.get(self.0.as_ref())
    }
}

#[derive(Clone, Debug)]
pub struct IndexTable {
    tbl_map: Arc<RwLock<HashMap<String, IndexTableValue>>>,
}
impl Default for IndexTable {
    fn default() -> Self {
        Self {
            tbl_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
impl IndexTable {
    pub fn new() -> Self {
        Self {
            tbl_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn from_hashmap(m: HashMap<String, Vec<(u16, String)>>) -> Self {
                Self { tbl_map: Arc::new(RwLock::new(m.into_iter().map(|(k,v)| {
                    
                    (k, IndexTableValue::from_vec(v))
                }).collect())) }
    }


    pub async fn insert<'b, S>(&self, key: S, value: (u16, String)) -> ()
    where
        S: Into<Cow<'b, str>>,
    {
        let mut guard = self.tbl_map.write().await;
        let k : Cow<'b, str> = key.into();
        
        match guard.get_mut(k.as_ref()) {
            Some(vec) => {
                vec.update_or_add_entry(value.1, value.0).await;
            }
            None => {
                let updated_v = IndexTableValueEntry {
                    target: value.1,
                    priority: Priority(value.0)
                };

                guard.insert(k.into_owned(), IndexTableValue::with_value(updated_v));
            }
        }
    }

    pub async fn get_or_guess<'b, S>(&self, key: S) -> IndexTableValue where
    S: Into<Cow<'b, str>>,
{
    let cow_k = key.into();

    match self.get(cow_k.clone()).await {
        Some(v) => v,
        None => {
            let guesses = expand_target_to_guesses::get_guesses_for_class_name(&cow_k);
            IndexTableValue::from_vec(guesses)
        }
    }
}

    pub async fn get<'b, S>(&self, key: S) -> Option<IndexTableValue>
    where
        S: Into<Cow<'b, str>>,
    {

        let v = self.tbl_map.read().await;
        v.get(&*key.into()).map(|e| e.clone())
    }

    pub async fn get_from_suffix<S>(&self, key: S) -> IndexTableValue
    where
        S: Into<String>,
    {
        let passed_k = key.into();
        let mut result: HashSet<IndexTableValueEntry> = HashSet::default();
        let tbl_map = self.tbl_map.read().await;
        for (k, v) in tbl_map.iter() {
            if k.ends_with(&passed_k) {
                for e in &v.read_iter().await {
                    result.insert(e.clone());
                }
            }
        }
        let mut vec_result: Vec<IndexTableValueEntry> = result.into_iter().collect();
        vec_result.sort();
        IndexTableValue::new(vec_result)
    }
}
fn element_extractor<'a, E>() -> impl Fn(&'a str) -> IResult<&str, (u16, &str), E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((
            digit1,
            tag(":"),
            take_while1(|chr| chr != ',' && chr != '\r' && chr != '\n'),
            opt(tag(",")),
        )),
        |(freq, _, target, _)| {
            let f: &str = freq;
            (f.parse::<u16>().unwrap(), target)
        },
    )
}

fn parse_index_line<'a, E>() -> impl Fn(&'a str) -> IResult<&str, (String, Vec<(u16, String)>), E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((
            map(take_while1(|chr| chr != '\t'), |e: &str| e.to_string()),
            tag("\t"),
            many1(map(element_extractor(), |(freq, v)| (freq, v.to_string()))),
        )),
        |tup| (tup.0, tup.2),
    )
}

fn parse_file_e(input: &str) -> IResult<&str, Vec<(String, Vec<(u16, String)>)>> {
    many0(map(tuple((parse_index_line(), opt(line_ending))), |e| e.0))(input)
}

pub fn parse_file(input: &str) -> Result<IndexTable, Box<dyn Error>> {
    debug!("Start parsing");

    let extracted_result: Vec<(String, Vec<(u16, String)>)> = parse_file_e(input).unwrap().1;
    debug!("Finished parsing..");

    let mut index_data = HashMap::new();
    index_data.reserve(extracted_result.len());

    for (k, v) in extracted_result.into_iter() {
        index_data.insert(k, v);
    }
    Ok(IndexTable::from_hashmap(index_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_parse_index_line(input: &str) -> IResult<&str, (String, Vec<(u16, String)>)> {
        parse_index_line()(input)
    }

    #[test]
    fn parse_sample_line() {
        assert_eq!(
            run_parse_index_line(
                "PantsWorkaroundCache\t0:@third_party_jvm//3rdparty/jvm/com/twitter:util_cache"
            )
            .unwrap()
            .1,
            (
                String::from("PantsWorkaroundCache"),
                vec![(
                    0,
                    String::from("@third_party_jvm//3rdparty/jvm/com/twitter:util_cache")
                )]
            )
        );
    }

    #[test]
    fn test_parse_troublesome_line() {
        assert_eq!(
            run_parse_index_line(
                "javax.annotation.Nullable\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"
            )
            .unwrap()
            .1,
            (
                String::from("javax.annotation.Nullable"),
                vec![(
                    236,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
                ),
                (
                    75,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"),
                ),
                ]
            )
        );
    }

    #[tokio::test]
    async fn parse_multiple_lines() {
        let parsed_file = parse_file(
            "scala.reflect.internal.SymbolPairs.Cursor.anon.1\t1:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_reflect
org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme\t0:@third_party_jvm//3rdparty/jvm/org/apache/parquet:parquet_thrift_jar_tests
org.apache.commons.lang3.concurrent.MultiBackgroundInitializer\t68:@third_party_jvm//3rdparty/jvm/org/apache/commons:commons_lang3
org.apache.hadoop.util.GcTimeMonitor\t38:@third_party_jvm//3rdparty/jvm/org/apache/hadoop:hadoop_common
org.apache.hadoop.fs.FSProtos.FileStatusProto.FileType\t38:@third_party_jvm//3rdparty/jvm/org/apache/hadoop:hadoop_common
com.twitter.chill.JavaIterableWrapperSerializer\t2:@third_party_jvm//3rdparty/jvm/com/twitter:chill
org.apache.commons.collections4.map.ListOrderedMap$EntrySetView\t0:@third_party_jvm//3rdparty/jvm/org/apache/commons:commons_collections4
scala.collection.convert.AsJavaConverters\t41:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library
org.ehcache.xml.XmlConfiguration.1\t0:@third_party_jvm//3rdparty/jvm/org/ehcache:ehcache
com.ibm.icu.text.CharsetRecog_sbcs$CharsetRecog_8859_1_de\t1:@third_party_jvm//3rdparty/jvm/com/ibm/icu:icu4j
scala.reflect.internal.Definitions$DefinitionsClass$VarArityClass\t1:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_reflect
org.apache.http.nio.pool.AbstractNIOConnPool.1\t0:@third_party_jvm//3rdparty/jvm/org/apache/httpcomponents:httpcore_nio
io.circe.generic.util.macros.DerivationMacros$$typecreator1$1 21:@third_party_jvm//3rdparty/jvm/io/circe:circe_generic
org.apache.zookeeper.server.NettyServerCnxn.DumpCommand\t0:@third_party_jvm//3rdparty/jvm/org/apache/zookeeper:zookeeper
org.apache.logging.log4j.core.appender.OutputStreamAppender$OutputStreamManagerFactory\t53:@third_party_jvm//3rdparty/jvm/org/apache/logging/log4j:log4j_core
com.twitter.finagle.http.service.RoutingService.anonfun\t2:@third_party_jvm//3rdparty/jvm/com/twitter:finagle_http
org.bouncycastle.util.CollectionStor\t10:@third_party_jvm//3rdparty/jvm/org/bouncycastle:bcprov_jdk15on
org.apache.avro.io.parsing.JsonGrammarGenerator$1\t0:@third_party_jvm//3rdparty/jvm/org/apache/avro:avro
org.terracotta.statistics.util\t0:@third_party_jvm//3rdparty/jvm/org/ehcache:ehcache
com.ibm.icu.impl.Normalizer2Impl$1\t1:@third_party_jvm//3rdparty/jvm/com/ibm/icu:icu4j
org.eclipse.jetty.io.ByteBufferPool.Bucket\t0:@third_party_jvm//3rdparty/jvm/org/eclipse
javax.annotation.Nonnull$Checker\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
javax.annotation.Nonnull.Checker\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
javax.annotation.Nullable\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
javax.annotation.OverridingMethodsMustInvokeSuper\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
javax.annotation.ParametersAreNonnullByDefault\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
javax.annotation.ParametersAreNullableByDefault\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"
        ).unwrap();

        assert_eq!(
            parsed_file.get("org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme").await.unwrap().as_vec().await,
            vec![IndexTableValueEntry{
                priority: Priority(0),
                target: String::from(
                    "@third_party_jvm//3rdparty/jvm/org/apache/parquet:parquet_thrift_jar_tests"
                )
            }]
        );

        assert_eq!(
            parsed_file.get("javax.annotation.Nullable").await.unwrap().as_vec().await,
            vec![
                IndexTableValueEntry{
                    priority: Priority(236),
                    target: String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
                },
                IndexTableValueEntry{
                    priority: Priority(75),
                    target: String::from(
                        "@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"
                    ),
                },
            ]
        );
    }

    #[tokio::test]
    async fn updating_index() {
        let index = IndexTable::default();

        assert_eq!(
            index.get("org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme").await.is_none(),
            true
        );


        // Insert new element
        index.insert("javax.annotation.Nullable", 
        (
            236,
            String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
        )
        ).await;

        assert_eq!(
            index.get("javax.annotation.Nullable").await.unwrap().as_vec().await,
            vec![
                IndexTableValueEntry{
                    priority: Priority(236),
                    target: String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
                },
            ]
        );


          // Update existing element
          index.insert("javax.annotation.Nullable", 
          (
              236,
              String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
          )
          ).await;


          index.insert("javax.annotation.Nullable", 
          (
              1,
              String::from("@third_party_jvm//3rdparty/jvm/com/google:guava")
          )
          ).await;
  
          assert_eq!(
              index.get("javax.annotation.Nullable").await.unwrap().as_vec().await,
              vec![
                IndexTableValueEntry{
                    priority: Priority(236),
                    target: String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
                },
                IndexTableValueEntry{
                    priority: Priority(1),
                    target: String::from("@third_party_jvm//3rdparty/jvm/com/google:guava")
                },
              ]
          );
    }


        // #[tokio::test]
    // async fn get_candidates_from_map() {
    //     let mut tbl_map = HashMap::new();
    //     tbl_map.insert(
    //         String::from("com.example.foo.bar.Baz"),
    //         vec![(13, String::from("//src/main/foop/blah:oop"))],
    //     );
    //     let index_table = index_table::IndexTable::from_hashmap(tbl_map);

    //     let error_info = ActionFailedErrorInfo {
    //         label: String::from("//src/main/foo/asd/we:wer"),
    //         output_files: vec![],
    //         target_kind: Some(String::from("scala_library")),
    //     };

    //     assert_eq!(
    //         get_candidates_for_class_name(&error_info, "com.example.bar.Baz", &index_table).await,
    //         vec![
    //             (0, String::from("//src/main/scala/com/example/bar:bar")),
    //             (0, String::from("//src/main/java/com/example/bar:bar")),
    //         ]
    //     );

    //     assert_eq!(
    //         get_candidates_for_class_name(&error_info, "com.example.foo.bar.Baz", &index_table).await,
    //         vec![
    //             (13, String::from("//src/main/foop/blah:oop")),
    //             (0, String::from("//src/main/scala/com/example/foo/bar:bar")),
    //             (0, String::from("//src/main/java/com/example/foo/bar:bar"))
    //         ]
    //     );

    //     assert_eq!(
    //         get_candidates_for_class_name(&error_info, "com.example.a.b.c.Baz", &index_table).await,
    //         vec![
    //             (0, String::from("//src/main/scala/com/example/a/b/c:c")),
    //             (0, String::from("//src/main/java/com/example/a/b/c:c"))
    //         ]
    //     );
    // }
}
