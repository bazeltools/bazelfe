use nom::bytes::complete::take_while1;
use nom::character::complete::digit1;
use nom::character::complete::line_ending;
use nom::error::ParseError;
use nom::multi::{many0, many1};
use nom::{bytes::complete::tag, combinator::map, combinator::opt, sequence::tuple, IResult};
use std::{borrow::Cow, collections::HashMap, collections::HashSet, path::Path, sync::Arc, io::Read};
use tokio::sync::RwLock;
mod expand_target_to_guesses;
mod index_table_value;
pub use index_table_value::*;
use std::io::Write;

pub struct GuardedGet<'a, 'b>(
    Cow<'b, str>,
    tokio::sync::RwLockReadGuard<'a, HashMap<String, IndexTableValue>>,
);

impl<'a, 'b> GuardedGet<'a, 'b> {
    pub fn get(&self) -> Option<&IndexTableValue> {
        self.1.get(self.0.as_ref())
    }
}

use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

// Index table format should be

// map u64 -> Vec<u8>
// map String -> Vec((u16, u64))
#[derive(Clone, Debug)]
pub struct IndexTable {
    tbl_map: Arc<RwLock<HashMap<String, IndexTableValue>>>,
    id_to_target_vec: Arc<RwLock<Vec<Arc<Vec<u8>>>>>,
    id_to_target_reverse_map: Arc<RwLock<HashMap<Arc<Vec<u8>>, usize>>>
}

impl<'a> Default for IndexTable {
    fn default() -> Self {
        Self {
            tbl_map: Arc::new(RwLock::new(HashMap::new())),
            id_to_target_vec: Arc::new(RwLock::new(Vec::new())),
            id_to_target_reverse_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
impl<'a> IndexTable {
    pub fn new() -> Self {
        Self {
            tbl_map: Arc::new(RwLock::new(HashMap::new())),
            id_to_target_vec: Arc::new(RwLock::new(Vec::new())),
            id_to_target_reverse_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn serialize_to_file(&self, path: &Path) -> () {
        let mut file = std::fs::File::create(&path).unwrap();

        let _ = {
        let id_vec = self.id_to_target_vec.read().await;
        file.write_u64::<LittleEndian>(id_vec.len() as u64).unwrap();

        for ele in id_vec.iter() {
            file.write_u16::<LittleEndian>(ele.len() as u16).unwrap();
            file.write_all(&ele).unwrap();
        }
    };

        let tbl_map = self.tbl_map.read().await;
        file.write_u64::<LittleEndian>(tbl_map.len() as u64).unwrap();

        for (k, innerv) in tbl_map.iter() {
            let bytes = k.as_bytes();
            file.write_u16::<LittleEndian>(k.len() as u16).unwrap();
            file.write_all(bytes).unwrap();

                innerv.write(&mut file).await
        }
    }

    
pub fn parse_file<R>(rdr: & mut R) -> IndexTable where R: Read{
    debug!("Start parsing");


    let num_vec_entries = rdr.read_u64::<LittleEndian>().unwrap();
    let mut index_buf = Vec::default();
    let mut reverse_hashmap = HashMap::default();

    for _ in 0..num_vec_entries  {
        let str_len = rdr.read_u16::<LittleEndian>().unwrap();
        let mut buf = Vec::with_capacity(str_len as usize);
        rdr.read_exact(&mut buf).unwrap();

        let val_v = Arc::new(buf);
        let pos = index_buf.len();
        index_buf.push(Arc::clone(&val_v));
        reverse_hashmap.insert(Arc::clone(&val_v), pos);
    }

    let mut tbl_map = HashMap::default();
    let map_siz = rdr.read_u64::<LittleEndian>().unwrap();
    for _ in 0..map_siz {
        let str_len = rdr.read_u16::<LittleEndian>().unwrap();
        let mut buf = Vec::with_capacity(str_len as usize);
        rdr.read_exact(&mut buf).unwrap();

        let k = String::from_utf8(buf).unwrap();

        let v = IndexTableValue::read(rdr);
        tbl_map.insert(k, v);
    }
    

    debug!("Finished parsing..");

    Self {
        tbl_map: Arc::new(RwLock::new(tbl_map)),
        id_to_target_vec: Arc::new(RwLock::new(index_buf)),
        id_to_target_reverse_map: Arc::new(RwLock::new(reverse_hashmap)),
    }

}
    async fn maybe_insert_target_string(&self, str: String) -> usize {
        self.maybe_insert_target_bytes(str.as_bytes().to_vec()).await
    }

    async fn maybe_insert_target_bytes(&self, bytes: Vec<u8>) -> usize {
        let val = Arc::new(bytes);
        let read_lock = self.id_to_target_reverse_map.read().await;
        if let Some(id) = read_lock.get(&val) {
                return id.clone();
        }
        drop(read_lock);
        let mut id_to_target_vec = self.id_to_target_vec.write().await;
        let mut id_to_target_reverse_map = self.id_to_target_reverse_map.write().await;

        let id = id_to_target_vec.len();
        id_to_target_vec.push(val);
        id_to_target_reverse_map.insert(Arc::clone(&id_to_target_vec[id]), id);
        id
    }

    pub fn from_vec(m: Vec<(String, Vec<(u16, String)>)>) -> Self {
        let mut id_to_target_vec = Vec::new();
        let mut id_to_target_reverse_map:HashMap<Arc<Vec<u8>>, usize> = HashMap::new();
        let mut tbl_map = HashMap::new();
        for (k, v) in m.into_iter() {
            let mut nxt = Vec::default();
            for (freq, v) in v {
                let val = Arc::new(v.as_bytes().to_vec());
                match id_to_target_reverse_map.get(&val) {
                    Some(id) => {
                        nxt.push((freq, id.clone()));
                    }
                    None => {
                        let id = id_to_target_vec.len();
                        id_to_target_vec.push(Arc::new(v.as_bytes().to_vec()));
                        id_to_target_reverse_map.insert(Arc::clone(&id_to_target_vec[id]), id);
                        nxt.push((freq, id));
                    }
                }
            }
            tbl_map.insert(k, IndexTableValue::from_vec(nxt));
        }
        Self {
            tbl_map: Arc::new(RwLock::new(tbl_map)),
            id_to_target_vec: Arc::new(RwLock::new(id_to_target_vec)),
            id_to_target_reverse_map: Arc::new(RwLock::new(id_to_target_reverse_map)),
        }
    }
    pub fn from_hashmap(m: HashMap<String, Vec<(u16, String)>>) -> Self {
      IndexTable::from_vec(m.into_iter().collect())
    }

    pub async fn decode_string(&self, key: usize) -> Option<String> {
        let read_lock = self.id_to_target_vec.read().await;
        match read_lock.get(key) {
            Some(e) =>
                unsafe {
                    Some(std::str::from_utf8_unchecked(&e).to_string())
                },
            None => None
        }
    }

    pub async fn insert<'b, S>(&self, key: S, value: (u16, String)) -> ()
    where
        S: Into<Cow<'b, str>>,
    {
        let (freq, target) = value;
        let key_id = {
            self.maybe_insert_target_string(target).await
        };

        let mut guard = self.tbl_map.write().await;
        let k: Cow<'b, str> = key.into();

        match guard.get_mut(k.as_ref()) {
            Some(vec) => {
                vec.update_or_add_entry(key_id, freq).await;
            }
            None => {
                let updated_v = IndexTableValueEntry {
                    target: key_id,
                    priority: Priority(freq),
                };

                guard.insert(k.into_owned(), IndexTableValue::with_value(updated_v));
            }
        }
    }

    pub async fn get_or_guess<'b, S>(&self, key: S) -> IndexTableValue
    where
        S: Into<Cow<'b, str>>,
    {
        let cow_k = key.into();

        match self.get(cow_k.clone()).await {
            Some(v) => v,
            None => {
                let guesses = expand_target_to_guesses::get_guesses_for_class_name(&cow_k);

                let mut guesses2 = Vec::default();
                for (k, v) in guesses.into_iter() {
                    guesses2.push((k, self.maybe_insert_target_string(v).await));
                }

                IndexTableValue::from_vec(guesses2)
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
// fn element_extractor<'a, E>() -> impl Fn(&'a str) -> IResult<&str, (u16, &str), E>
// where
//     E: ParseError<&'a str>,
// {
//     map(
//         tuple((
//             digit1,
//             tag(":"),
//             take_while1(|chr| chr != ',' && chr != '\r' && chr != '\n'),
//             opt(tag(",")),
//         )),
//         |(freq, _, target, _)| {
//             let f: &str = freq;
//             (f.parse::<u16>().unwrap(), target)
//         },
//     )
// }

// fn parse_index_line<'a, E>() -> impl Fn(&'a str) -> IResult<&[u8], (String, Vec<(u16, String)>), E>
// where
//     E: ParseError<&'a str>,
// {
//     map(
//         tuple((
//             map(take_while1(|chr| chr != '\t'), |e: &str| e.to_string()),
//             tag("\t"),
//             many1(map(element_extractor(), |(freq, v)| (freq, v.to_string()))),
//         )),
//         |tup| (tup.0, tup.2),
//     )
// }

// fn parse_file_e(input: &str) -> IResult<&str, Vec<(String, Vec<(u16, String)>)>> {
//     many0(map(tuple((parse_index_line(), opt(line_ending))), |e| e.0))(input)
// }


#[cfg(test)]
mod tests {
    use super::*;
    // fn run_parse_index_line(input: &str) -> IResult<&str, (String, Vec<(u16, String)>)> {
    //     parse_index_line()(input)
    // }

//     #[test]
//     fn parse_sample_line() {
//         assert_eq!(
//             run_parse_index_line(
//                 "PantsWorkaroundCache\t0:@third_party_jvm//3rdparty/jvm/com/twitter:util_cache"
//             )
//             .unwrap()
//             .1,
//             (
//                 String::from("PantsWorkaroundCache"),
//                 vec![(
//                     0,
//                     String::from("@third_party_jvm//3rdparty/jvm/com/twitter:util_cache")
//                 )]
//             )
//         );
//     }

//     #[test]
//     fn test_parse_troublesome_line() {
//         assert_eq!(
//             run_parse_index_line(
//                 "javax.annotation.Nullable\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"
//             )
//             .unwrap()
//             .1,
//             (
//                 String::from("javax.annotation.Nullable"),
//                 vec![(
//                     236,
//                     String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305")
//                 ),
//                 (
//                     75,
//                     String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"),
//                 ),
//                 ]
//             )
//         );
//     }

//     #[tokio::test]
//     async fn parse_multiple_lines() {
//         let parsed_file = parse_file(
//             "scala.reflect.internal.SymbolPairs.Cursor.anon.1\t1:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_reflect
// org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme\t0:@third_party_jvm//3rdparty/jvm/org/apache/parquet:parquet_thrift_jar_tests
// org.apache.commons.lang3.concurrent.MultiBackgroundInitializer\t68:@third_party_jvm//3rdparty/jvm/org/apache/commons:commons_lang3
// org.apache.hadoop.util.GcTimeMonitor\t38:@third_party_jvm//3rdparty/jvm/org/apache/hadoop:hadoop_common
// org.apache.hadoop.fs.FSProtos.FileStatusProto.FileType\t38:@third_party_jvm//3rdparty/jvm/org/apache/hadoop:hadoop_common
// com.twitter.chill.JavaIterableWrapperSerializer\t2:@third_party_jvm//3rdparty/jvm/com/twitter:chill
// org.apache.commons.collections4.map.ListOrderedMap$EntrySetView\t0:@third_party_jvm//3rdparty/jvm/org/apache/commons:commons_collections4
// scala.collection.convert.AsJavaConverters\t41:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library
// org.ehcache.xml.XmlConfiguration.1\t0:@third_party_jvm//3rdparty/jvm/org/ehcache:ehcache
// com.ibm.icu.text.CharsetRecog_sbcs$CharsetRecog_8859_1_de\t1:@third_party_jvm//3rdparty/jvm/com/ibm/icu:icu4j
// scala.reflect.internal.Definitions$DefinitionsClass$VarArityClass\t1:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_reflect
// org.apache.http.nio.pool.AbstractNIOConnPool.1\t0:@third_party_jvm//3rdparty/jvm/org/apache/httpcomponents:httpcore_nio
// io.circe.generic.util.macros.DerivationMacros$$typecreator1$1 21:@third_party_jvm//3rdparty/jvm/io/circe:circe_generic
// org.apache.zookeeper.server.NettyServerCnxn.DumpCommand\t0:@third_party_jvm//3rdparty/jvm/org/apache/zookeeper:zookeeper
// org.apache.logging.log4j.core.appender.OutputStreamAppender$OutputStreamManagerFactory\t53:@third_party_jvm//3rdparty/jvm/org/apache/logging/log4j:log4j_core
// com.twitter.finagle.http.service.RoutingService.anonfun\t2:@third_party_jvm//3rdparty/jvm/com/twitter:finagle_http
// org.bouncycastle.util.CollectionStor\t10:@third_party_jvm//3rdparty/jvm/org/bouncycastle:bcprov_jdk15on
// org.apache.avro.io.parsing.JsonGrammarGenerator$1\t0:@third_party_jvm//3rdparty/jvm/org/apache/avro:avro
// org.terracotta.statistics.util\t0:@third_party_jvm//3rdparty/jvm/org/ehcache:ehcache
// com.ibm.icu.impl.Normalizer2Impl$1\t1:@third_party_jvm//3rdparty/jvm/com/ibm/icu:icu4j
// org.eclipse.jetty.io.ByteBufferPool.Bucket\t0:@third_party_jvm//3rdparty/jvm/org/eclipse
// javax.annotation.Nonnull$Checker\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
// javax.annotation.Nonnull.Checker\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
// javax.annotation.Nullable\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
// javax.annotation.OverridingMethodsMustInvokeSuper\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
// javax.annotation.ParametersAreNonnullByDefault\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations
// javax.annotation.ParametersAreNullableByDefault\t236:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305,75:@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:annotations"
//         ).unwrap();

//         assert_eq!(
//             parsed_file
//                 .get("org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme")
//                 .await
//                 .unwrap()
//                 .as_vec()
//                 .await,
//             vec![IndexTableValueEntry {
//                 priority: Priority(0),
//                 target: 1
//             }]
//         );

//         assert_eq!(
//             parsed_file
//                 .get("javax.annotation.Nullable")
//                 .await
//                 .unwrap()
//                 .as_vec()
//                 .await,
//             vec![
//                 IndexTableValueEntry {
//                     priority: Priority(236),
//                     target: 16
//                 },
//                 IndexTableValueEntry {
//                     priority: Priority(75),
//                     target: 17,
//                 },
//             ]
//         );
//     }

    #[tokio::test]
    async fn updating_index() {
        let index = IndexTable::default();

        assert_eq!(
            index
                .get("org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme")
                .await
                .is_none(),
            true
        );

        // Insert new element
        index
            .insert(
                "javax.annotation.Nullable",
                (
                    236,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305"),
                ),
            )
            .await;

        assert_eq!(
            index
                .get("javax.annotation.Nullable")
                .await
                .unwrap()
                .as_vec()
                .await,
            vec![IndexTableValueEntry {
                priority: Priority(236),
                target: 0
            },]
        );

        // Update existing element
        index
            .insert(
                "javax.annotation.Nullable",
                (
                    236,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305"),
                ),
            )
            .await;

        index
            .insert(
                "javax.annotation.Nullable",
                (
                    1,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google:guava"),
                ),
            )
            .await;

        assert_eq!(
            index
                .get("javax.annotation.Nullable")
                .await
                .unwrap()
                .as_vec()
                .await,
            vec![
                IndexTableValueEntry {
                    priority: Priority(236),
                    target: 0
                },
                IndexTableValueEntry {
                    priority: Priority(1),
                    target: 1
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
