use std::{io::Read, io::Write, sync::Arc};

use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub struct Priority(pub u16);

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexTableValueEntry {
    ///Todo impl the ordering's manually so the order of fields here doesn't matter
    pub priority: Priority,
    pub target: usize,
}

impl PartialOrd for IndexTableValueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexTableValueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Equal => self.target.cmp(&other.target),
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
        }
    }
}

pub struct IterGuard<'a> {
    guard: tokio::sync::RwLockReadGuard<'a, std::vec::Vec<IndexTableValueEntry>>,
}

impl<'a, 'b> IntoIterator for &'b IterGuard<'a> {
    type IntoIter = std::slice::Iter<'b, IndexTableValueEntry>;
    type Item = &'b IndexTableValueEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.guard.iter()
    }
}
impl<'a> IterGuard<'a> {
    pub fn iter(&self) -> std::slice::Iter<IndexTableValueEntry> {
        self.guard.iter()
    }
}

#[derive(Clone, Debug)]
pub struct IndexTableValue(Arc<RwLock<Vec<IndexTableValueEntry>>>);

impl Default for IndexTableValue {
    fn default() -> Self {
        IndexTableValue::new(Vec::default())
    }
}

impl IndexTableValue {
    /// Used in testing to convert into a simple vec for comparing.
    pub(in crate) async fn into_vec(self) -> Vec<IndexTableValueEntry> {
        let w = self.0.read().await;
        let r = w.clone();
        drop(w);
        r
    }

    pub fn read<T>(rdr: &mut T) -> Self
    where
        T: Read,
    {
        use byteorder::{LittleEndian, ReadBytesExt};
        let len = rdr.read_u16::<LittleEndian>().unwrap();

        let mut v = Vec::default();

        for _ in 0..len {
            let priority = rdr.read_u16::<LittleEndian>().unwrap();
            let target = rdr.read_u64::<LittleEndian>().unwrap();
            v.push(IndexTableValueEntry {
                priority: Priority(priority),
                target: target as usize,
            });
        }
        v.sort();

        Self(Arc::new(RwLock::new(v)))
    }

    pub async fn write<T>(&self, t: &mut T)
    where
        T: Write,
    {
        let guard = self.0.read().await;
        use byteorder::{LittleEndian, WriteBytesExt};
        t.write_u16::<LittleEndian>(guard.len() as u16).unwrap();

        for ele in guard.iter() {
            t.write_u16::<LittleEndian>(ele.priority.0).unwrap();
            t.write_u64::<LittleEndian>(ele.target as u64).unwrap();
        }
    }

    pub async fn read_iter(&'_ self) -> IterGuard<'_> {
        let guard = self.0.read().await;
        IterGuard { guard }
    }

    pub fn from_vec(data: Vec<(u16, usize)>) -> Self {
        let transformed: Vec<IndexTableValueEntry> = data
            .into_iter()
            .map(|e| IndexTableValueEntry {
                target: e.1,
                priority: Priority(e.0),
            })
            .collect();

        Self::new(transformed)
    }

    pub fn new(mut data: Vec<IndexTableValueEntry>) -> Self {
        data.sort();

        Self {
            0: Arc::new(RwLock::new(data)),
        }
    }

    pub fn with_value(entry: IndexTableValueEntry) -> Self {
        Self::new(vec![entry])
    }

    pub async fn lookup_by_value(&self, k: usize) -> Option<(usize, u16)> {
        for (idx, element) in (&self.read_iter().await).into_iter().enumerate() {
            if element.target == k {
                return Some((idx, element.priority.0));
            }
        }
        None
    }

    pub async fn replace_with_entry(&self, target_v: usize, priority: u16, use_max: bool) -> bool {
        match self.lookup_by_value(target_v).await {
            Some((_, old_priority)) => {
                let mut write_vec = self.0.write().await;
                if use_max && old_priority >= priority && write_vec.len() == 1 {
                    return false;
                }
                if old_priority == priority && write_vec.len() == 1 {
                    return false;
                }

                write_vec.clear();
                write_vec.push(IndexTableValueEntry {
                    target: target_v,
                    priority: Priority(priority),
                });
            }
            None => {
                let mut write_vec = self.0.write().await;
                write_vec.clear();
                write_vec.push(IndexTableValueEntry {
                    target: target_v,
                    priority: Priority(priority),
                });
            }
        }
        true
    }

    pub async fn update_or_add_entry(&self, target_v: usize, priority: u16, use_max: bool) -> bool {
        match self.lookup_by_value(target_v).await {
            Some((position, old_priority)) => {
                let mut write_vec = self.0.write().await;
                if use_max && old_priority >= priority {
                    return false;
                }
                if old_priority == priority {
                    return false;
                }

                write_vec.remove(position);

                write_vec.push(IndexTableValueEntry {
                    target: target_v,
                    priority: Priority(priority),
                });
                write_vec.sort();
                if write_vec.len() > 10 {
                    write_vec.truncate(10);
                }
            }
            None => {
                let mut write_vec = self.0.write().await;

                write_vec.push(IndexTableValueEntry {
                    target: target_v,
                    priority: Priority(priority),
                });
                write_vec.sort();
                if write_vec.len() > 10 {
                    write_vec.truncate(10);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blank_entry() {
        let index = IndexTableValue::default().into_vec().await;
        let expected: Vec<IndexTableValueEntry> = Vec::default();
        assert_eq!(index, expected);
    }

    #[tokio::test]
    async fn test_build_from_unsorted_list() {
        let index = IndexTableValue::from_vec(vec![(55, 1003), (22, 1002), (99, 1001)]);
        let expected: Vec<IndexTableValueEntry> = vec![
            IndexTableValueEntry {
                target: 1001,
                priority: Priority(99),
            },
            IndexTableValueEntry {
                target: 1003,
                priority: Priority(55),
            },
            IndexTableValueEntry {
                target: 1002,
                priority: Priority(22),
            },
        ];
        assert_eq!(index.into_vec().await, expected);
    }

    #[tokio::test]
    async fn test_lookup_by_value() {
        let index = IndexTableValue::from_vec(vec![(55, 1003), (22, 1002), (99, 1001)]);

        assert_eq!(index.lookup_by_value(1002).await, Some((2, 22)));

        assert_eq!(index.lookup_by_value(1001).await, Some((0, 99)));
    }

    #[tokio::test]
    async fn test_update_or_add_entry() {
        let index = IndexTableValue::from_vec(vec![(55, 1003), (22, 1002), (99, 1001)]);

        assert_eq!(index.lookup_by_value(1002).await, Some((2, 22)));

        index.update_or_add_entry(1002, 1200, false).await;
        assert_eq!(index.lookup_by_value(1002).await, Some((0, 1200)));

        let expected: Vec<IndexTableValueEntry> = vec![
            IndexTableValueEntry {
                target: 1002,
                priority: Priority(1200),
            },
            IndexTableValueEntry {
                target: 1001,
                priority: Priority(99),
            },
            IndexTableValueEntry {
                target: 1003,
                priority: Priority(55),
            },
        ];
        assert_eq!(index.into_vec().await, expected);
    }
}
