use std::{
    borrow::Cow, collections::HashMap, collections::HashSet, io::Read, path::PathBuf,
    sync::atomic::Ordering, sync::Arc, time::SystemTime,
};
use tokio::sync::RwLock;
mod expand_target_to_guesses;
mod index_table_value;
pub use index_table_value::*;
use std::io::Write;
use std::sync::atomic::AtomicBool;

pub struct GuardedGet<'a, 'b>(
    Cow<'b, str>,
    tokio::sync::RwLockReadGuard<'a, HashMap<String, IndexTableValue>>,
);

impl<'a, 'b> GuardedGet<'a, 'b> {
    pub fn get(&self) -> Option<&IndexTableValue> {
        self.1.get(self.0.as_ref())
    }
}

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// Index table format should be

// map u64 -> Vec<u8>
// map String -> Vec((u16, u64))
#[derive(Clone, Debug)]
pub struct IndexTable {
    tbl_map: Arc<RwLock<HashMap<String, IndexTableValue>>>,
    id_to_ctime: Arc<RwLock<Vec<u64>>>,
    id_to_popularity: Arc<RwLock<Vec<u16>>>,
    id_to_replacement_id: Arc<RwLock<HashMap<usize, usize>>>,
    id_to_target_vec: Arc<RwLock<Vec<Arc<Vec<u8>>>>>,
    id_to_target_reverse_map: Arc<RwLock<HashMap<Arc<Vec<u8>>, usize>>>,
    mutated: Arc<AtomicBool>,
}
#[derive(Clone, Debug)]
pub struct DebugIndexTable {
    pub data_map: HashMap<String, String>,
}

impl<'a> Default for IndexTable {
    fn default() -> Self {
        Self::new()
    }
}
impl<'a> IndexTable {
    pub fn new() -> Self {
        Self {
            tbl_map: Arc::new(RwLock::new(HashMap::new())),
            id_to_ctime: Arc::new(RwLock::new(Vec::new())),
            id_to_popularity: Arc::new(RwLock::new(Vec::new())),
            id_to_replacement_id: Arc::new(RwLock::new(HashMap::new())),
            id_to_target_vec: Arc::new(RwLock::new(Vec::new())),
            id_to_target_reverse_map: Arc::new(RwLock::new(HashMap::new())),
            mutated: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn to_debug_table(&self) -> DebugIndexTable {
        let str_lut = self.id_to_target_vec.read().await;

        let tbl = self.tbl_map.clone();

        let mut id_to_str: Vec<String> = Vec::default();
        for e in str_lut.iter() {
            id_to_str.push(String::from_utf8_lossy(&*e).into_owned());
        }

        let mut res_map = HashMap::new();

        let tbl = tbl.read().await;
        for (k, v) in tbl.iter() {
            let data = v.clone().as_vec().await;
            let mut res_str = String::from("");
            for d in data.iter() {
                res_str = format!("{},{}:{}", res_str, d.priority.0, id_to_str[d.target]);
            }
            res_map.insert(k.clone(), res_str);
        }

        DebugIndexTable { data_map: res_map }
    }

    pub async fn add_transformation_mapping(&self, src_str: String, dest_str: String) {
        let src_id = self.maybe_insert_target_string(src_str).await;
        let dest_id = self.maybe_insert_target_string(dest_str).await;
        let mut lock = self.id_to_replacement_id.write().await;
        lock.insert(src_id, dest_id);
    }

    pub async fn maybe_update_id(&self, src_id: usize) -> usize {
        let lock = self.id_to_replacement_id.read().await;
        match lock.get(&src_id) {
            Some(dest) => *dest,
            None => src_id,
        }
    }
    pub async fn get_popularity(&self, label_id: usize) -> u16 {
        let lock = self.id_to_popularity.read().await;
        if label_id >= lock.len() {
            0
        } else {
            lock[label_id]
        }
    }

    pub async fn set_popularity(&self, label_id: usize, popularity: u16) -> () {
        let mut lock = self.id_to_popularity.write().await;
        if label_id >= lock.len() {
            lock.resize_with((label_id + 100) as usize, Default::default);
        }
        lock[label_id] = popularity;
    }

    pub async fn set_popularity_str(&self, label: String, popularity: u16) -> () {
        let id = self.maybe_insert_target_string(label).await;
        self.set_popularity(id, popularity).await
    }

    pub fn is_mutated(&self) -> bool {
        (*self.mutated).load(Ordering::Relaxed)
    }

    pub async fn write<W>(&self, file: &mut W) -> ()
    where
        W: Write,
    {
        let mut file = std::io::BufWriter::with_capacity(512 * 1024, file);
        let _ = {
            let id_vec = self.id_to_target_vec.read().await;
            file.write_u64::<LittleEndian>(id_vec.len() as u64).unwrap();

            for ele in id_vec.iter() {
                file.write_u16::<LittleEndian>(ele.len() as u16).unwrap();
                file.write_all(&ele).unwrap();
            }
        };

        let tbl_map = self.tbl_map.read().await;
        file.write_u64::<LittleEndian>(tbl_map.len() as u64)
            .unwrap();

        let mut vec_join_res = Vec::default();
        for (k, innerv) in tbl_map.iter() {
            let innerv = innerv.clone();
            let k = k.clone();
            vec_join_res.push(tokio::spawn(async move {
                let mut cur_buf = Vec::default();
                let bytes = k.as_bytes();
                cur_buf.write_u16::<LittleEndian>(k.len() as u16).unwrap();
                cur_buf.write_all(bytes).unwrap();

                innerv.write(&mut cur_buf).await;
                cur_buf
            }));
        }
        for e in vec_join_res.into_iter() {
            let data = e.await.unwrap();
            file.write_all(&data).unwrap();
        }

        let id_to_ctime = self.id_to_ctime.read().await;

        file.write_u64::<LittleEndian>(id_to_ctime.len() as u64)
            .unwrap();
        for e in id_to_ctime.iter() {
            file.write_u64::<LittleEndian>(*e as u64).unwrap();
        }

        let id_to_popularity = self.id_to_popularity.read().await;
        file.write_u64::<LittleEndian>(id_to_popularity.len() as u64)
            .unwrap();
        for e in id_to_popularity.iter() {
            file.write_u16::<LittleEndian>(*e).unwrap();
        }

        let id_to_replacement_id = self.id_to_replacement_id.read().await;
        file.write_u64::<LittleEndian>(id_to_replacement_id.len() as u64)
            .unwrap();
        for (k, v) in id_to_replacement_id.iter() {
            file.write_u64::<LittleEndian>(*k as u64).unwrap();
            file.write_u64::<LittleEndian>(*v as u64).unwrap();
        }

        file.flush().unwrap();
    }

    pub fn read<R>(rdr: &mut R) -> IndexTable
    where
        R: Read,
    {
        debug!("Starting to parse index");
        use std::io::BufReader;

        let mut rdr = BufReader::with_capacity(512 * 1024, rdr);
        let num_vec_entries = rdr.read_u64::<LittleEndian>().unwrap();
        let mut index_buf = Vec::default();
        let mut reverse_hashmap = HashMap::default();

        for _ in 0..num_vec_entries {
            let str_len = rdr.read_u16::<LittleEndian>().unwrap();
            let mut buf: Vec<u8> = Vec::default();
            buf.resize_with(str_len as usize, Default::default);

            rdr.read_exact(&mut buf).unwrap();

            let val_v = Arc::new(buf);
            let pos = index_buf.len();
            index_buf.push(Arc::clone(&val_v));
            reverse_hashmap.insert(Arc::clone(&val_v), pos);
        }

        debug!("Complete target string table");
        let mut tbl_map = HashMap::default();
        let map_siz = rdr.read_u64::<LittleEndian>().unwrap();

        for _ in 0..map_siz {
            let str_len = rdr.read_u16::<LittleEndian>().unwrap();
            let mut buf: Vec<u8> = Vec::default();
            buf.resize_with(str_len as usize, Default::default);
            rdr.read_exact(&mut buf).unwrap();

            let k = String::from_utf8_lossy(&buf).into_owned();

            let v = IndexTableValue::read(&mut rdr);
            tbl_map.insert(k, v);
        }

        debug!("Complete main map");
        let id_to_ctime_size = rdr.read_u64::<LittleEndian>().unwrap();
        let mut id_to_ctime = Vec::default();
        for _ in 0..id_to_ctime_size {
            let ctimestamp = rdr.read_u64::<LittleEndian>().unwrap();
            id_to_ctime.push(ctimestamp);
        }
        debug!("Complete id_to_ctime");

        let id_to_popularity_size = rdr.read_u64::<LittleEndian>().unwrap();
        let mut id_to_popularity = Vec::default();
        for _ in 0..id_to_popularity_size {
            let popularity = rdr.read_u16::<LittleEndian>().unwrap();
            id_to_popularity.push(popularity);
        }

        debug!("Complete id_to_popularity");
        let id_to_replacement_id_size = rdr.read_u64::<LittleEndian>().unwrap();
        let mut id_to_replacement_id = HashMap::default();
        for _ in 0..id_to_replacement_id_size {
            let k = rdr.read_u64::<LittleEndian>().unwrap();
            let v = rdr.read_u64::<LittleEndian>().unwrap();
            id_to_replacement_id.insert(k as usize, v as usize);
        }
        debug!("Complete id_to_replacement_id");

        debug!("Finished parsing..");

        Self {
            tbl_map: Arc::new(RwLock::new(tbl_map)),
            id_to_ctime: Arc::new(RwLock::new(id_to_ctime)),
            id_to_popularity: Arc::new(RwLock::new(id_to_popularity)),
            id_to_replacement_id: Arc::new(RwLock::new(id_to_replacement_id)),
            id_to_target_vec: Arc::new(RwLock::new(index_buf)),
            id_to_target_reverse_map: Arc::new(RwLock::new(reverse_hashmap)),
            mutated: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn index_jar(
        &self,
        target_kind: &Option<String>,
        target_name: String,
        paths: Vec<PathBuf>,
    ) -> u32 {
        let paths = match target_kind {
            Some(kind) => {
                // These are really sketchy transforms
                // but with aspects they can expose the full transitive set of dependencies
                // and we wind up with no way to tell what the output here should be.
                if kind == "java_proto_library" {
                    paths
                        .into_iter()
                        .filter_map(|e| {
                            let mut parent = e.parent().unwrap().to_path_buf();
                            let name = e.file_name().unwrap().to_str().unwrap();
                            if name.ends_with("-src.jar") {
                                // foo_barproto-speed-src.jar
                                // libfoo_barproto-speed.jar
                                let without_suffix = name.strip_suffix("-src.jar").unwrap();
                                let nme = format!("lib{}.jar", without_suffix);
                                parent.push(nme);
                                Some(parent)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    paths
                }
            }

            None => paths,
        };
        let current_time_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let newest_ctime = paths
            .iter()
            .map(|p| match p.metadata() {
                Ok(metadata) => metadata
                    .created()
                    .map(|e| e.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs())
                    .unwrap_or(current_time_since_epoch),
                Err(_) => current_time_since_epoch,
            })
            .max()
            .unwrap_or(current_time_since_epoch);

        let key_id = self.maybe_insert_target_string(target_name).await;

        let should_update = {
            let read_guard = self.id_to_ctime.read().await;
            match read_guard.get(key_id) {
                None => true,
                Some(prev) => *prev < newest_ctime,
            }
        };

        if should_update {
            let _ = {
                let mut w = self.id_to_ctime.write().await;
                if key_id > w.len() {
                    w.resize_with((key_id + 100) as usize, Default::default);
                }
                w.insert(key_id, newest_ctime);
            };

            let popularity = self.get_popularity(key_id).await;

            let mut found_classes = Vec::default();
            for p in paths.into_iter() {
                found_classes.extend(crate::zip_parse::extract_classes_from_zip(p));
            }

            let mut jvm_segments_indexed = 0;
            let key_id = self.maybe_update_id(key_id).await;

            for e in found_classes.into_iter() {
                jvm_segments_indexed += {
                    let ret = if e.starts_with("//") {
                        self.replace_with_id(&e, key_id, popularity).await
                    } else {
                        self.insert_with_id(&e, key_id, popularity).await
                    };
                    if ret {
                        1
                    } else {
                        0
                    }
                };
                for clazz in crate::label_utils::class_name_to_prefixes(e.as_str()) {
                    jvm_segments_indexed += if self.insert_with_id(clazz, key_id, popularity).await
                    {
                        1
                    } else {
                        0
                    };
                }
            }
            jvm_segments_indexed
        } else {
            0
        }
    }
    async fn maybe_insert_target_string(&self, str: String) -> usize {
        self.maybe_insert_target_bytes(str.as_bytes().to_vec())
            .await
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
        let mut id_to_target_reverse_map: HashMap<Arc<Vec<u8>>, usize> = HashMap::new();
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
            id_to_ctime: Arc::new(RwLock::new(Vec::default())),
            id_to_popularity: Arc::new(RwLock::new(Vec::default())),
            id_to_replacement_id: Arc::new(RwLock::new(HashMap::default())),
            id_to_target_vec: Arc::new(RwLock::new(id_to_target_vec)),
            id_to_target_reverse_map: Arc::new(RwLock::new(id_to_target_reverse_map)),
            mutated: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn from_hashmap(m: HashMap<String, Vec<(u16, String)>>) -> Self {
        IndexTable::from_vec(m.into_iter().collect())
    }

    pub async fn decode_string(&self, key: usize) -> Option<String> {
        let read_lock = self.id_to_target_vec.read().await;
        match read_lock.get(key) {
            Some(e) => unsafe { Some(std::str::from_utf8_unchecked(&e).to_string()) },
            None => None,
        }
    }

    pub async fn replace_with_id<'b, S>(&self, key: S, target_id: usize, priority: u16) -> bool
    where
        S: Into<Cow<'b, str>>,
    {
        let mut guard = self.tbl_map.write().await;
        let k: Cow<'b, str> = key.into();

        match guard.get(k.as_ref()) {
            Some(vec) => {
                let did_update = vec.replace_with_entry(target_id, priority, true).await;

                if did_update {
                    self.mutated.store(true, Ordering::Relaxed);
                };
                did_update
            }
            None => {
                self.mutated.store(true, Ordering::Relaxed);
                let updated_v = IndexTableValueEntry {
                    target: target_id,
                    priority: Priority(priority),
                };

                let index_v = IndexTableValue::with_value(updated_v);
                let k = k.into_owned();

                guard.insert(k, index_v);
                true
            }
        }
    }

    pub async fn insert_with_id<'b, S>(&self, key: S, target_id: usize, priority: u16) -> bool
    where
        S: Into<Cow<'b, str>>,
    {
        let mut guard = self.tbl_map.write().await;
        let k: Cow<'b, str> = key.into();

        match guard.get(k.as_ref()) {
            Some(vec) => {
                let did_update = vec.update_or_add_entry(target_id, priority, true).await;

                if did_update {
                    self.mutated.store(true, Ordering::Relaxed);
                };
                did_update
            }
            None => {
                self.mutated.store(true, Ordering::Relaxed);
                let updated_v = IndexTableValueEntry {
                    target: target_id,
                    priority: Priority(priority),
                };

                let index_v = IndexTableValue::with_value(updated_v);
                let k = k.into_owned();

                guard.insert(k, index_v);
                true
            }
        }
    }

    pub async fn insert<'b, S>(&self, key: S, value: (u16, String)) -> ()
    where
        S: Into<Cow<'b, str>>,
    {
        let (freq, target) = value;
        let key_id = { self.maybe_insert_target_string(target).await };

        self.insert_with_id(key, key_id, freq).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_round_trip_table() {
        let index_table = IndexTable::default();
        index_table
            .insert(
                "javax.annotation.Nullable",
                (
                    236,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305"),
                ),
            )
            .await;

        index_table
            .insert(
                "javax.annotation.Nullable",
                (
                    239,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:foop"),
                ),
            )
            .await;

        index_table
            .insert(
                "javax.annotation.Noof",
                (
                    1,
                    String::from("@doof//3rdparty/jvm/com/google/code/findbugs:foop"),
                ),
            )
            .await;

        let mut cursor = std::io::Cursor::new(Vec::default());
        index_table.write(&mut cursor).await;

        cursor.set_position(0);

        let table = IndexTable::read(&mut cursor);
        assert_eq!(
            table
                .get("javax.annotation.Noof")
                .await
                .unwrap()
                .as_vec()
                .await,
            index_table
                .get("javax.annotation.Noof")
                .await
                .unwrap()
                .as_vec()
                .await
        );
    }

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
                "javax.annotation.foo.boof.Nullable",
                (
                    236,
                    String::from("@third_party_jvm//3rdparty/jvm/com/google/code/findbugs:jsr305"),
                ),
            )
            .await;

        assert_eq!(
            index
                .get("javax.annotation.foo.boof.Nullable")
                .await
                .unwrap()
                .as_vec()
                .await,
            vec![IndexTableValueEntry {
                priority: Priority(236),
                target: 0
            }]
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
