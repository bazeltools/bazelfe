use std::{sync::Arc, borrow::Cow};

use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub struct Priority(pub u16);

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0)
    }
}


#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexTableValueEntry {
    ///Todo impl the ordering's manually so the order of fields here doesn't matter
    pub priority: Priority,
    pub target: String,
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
    #[cfg(test)]
    pub (in crate) async fn as_vec(self) -> Vec<IndexTableValueEntry>  {
        let w = self.0.read().await;
        w.clone()
    }

    pub async fn read_iter<'a>(&'a self) -> IterGuard<'a> {
        let guard = self.0.read().await;
        return IterGuard { guard };
    }

    pub fn from_vec(data: Vec<(u16, String)>) -> Self {
        let transformed: Vec<IndexTableValueEntry> = data.into_iter().map(|e| {
            IndexTableValueEntry {
                target: e.1,
                priority: Priority(e.0)
            }
        }).collect();

        Self::new(transformed)
    }

    pub fn new(mut data: Vec<IndexTableValueEntry>) -> Self {
        data.sort();

        Self {
            0: Arc::new(RwLock::new(data))
        }
    }

    pub fn with_value(entry: IndexTableValueEntry) -> Self {
        Self::new(vec![entry])
    }

    pub async fn lookup_by_value<'b, S>(&self, target_v: S) -> Option<(usize, u16)>
    where S : Into<Cow<'b, str>> {

        let k = target_v.into();


        let mut idx = 0;
        
        for element in &self.read_iter().await {
            if element.target == k {
                return Some((idx, element.priority.0));
            }
            idx += 1;
        }
        None
    }
            
    pub async fn update_or_add_entry<'b, S>(&self, target_v: S, priority: u16) -> ()
    where S : Into<Cow<'b, str>> {
        let cow: Cow<'b, str> = target_v.into();
        let as_borrow = match &cow {
            Cow::Borrowed(b) => {
                Cow::Borrowed(*b)
            }
            Cow::Owned(own) => {
                Cow::Borrowed(*&own.as_str())
            }
        };
        match self.lookup_by_value(as_borrow).await {
            Some((position, old_priority)) => {
                let mut write_vec = self.0.write().await;
                if old_priority == priority {
                    return;
                }
                let insert_position = match write_vec.binary_search_by_key(&Priority(priority), |e| e.priority) {
                    Ok(current_position) => current_position,
                    Err(should_be_at_position) => should_be_at_position
                };
                if insert_position == position {
                    write_vec[position].priority = Priority(priority);
                    return;
                }
                write_vec.remove(position);
                let insert_position = if insert_position < position {
                    insert_position
                } else {
                    insert_position - 1
                };
                write_vec.insert(insert_position, IndexTableValueEntry{target: cow.into_owned(), priority: Priority(priority)});
            }
            None => {
                let mut write_vec = self.0.write().await;

                let insert_position = match write_vec.binary_search_by_key(&Priority(priority), |e| e.priority) {
                    Ok(current_position) => current_position,
                    Err(should_be_at_position) => should_be_at_position
                };
                write_vec.insert(insert_position, IndexTableValueEntry{target: cow.into_owned(), priority: Priority(priority)});

            }
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blank_entry() {
        let index = IndexTableValue::default().as_vec().await;
        let expected: Vec<IndexTableValueEntry> = Vec::default();
        assert_eq!(
            index,
            expected
        );
    }


    #[tokio::test]
    async  fn test_build_from_unsorted_list() {
        let index = IndexTableValue::from_vec(vec![
            (55, String::from("foo")),
            (22, String::from("foo2")),
            (99, String::from("foo3")),
        ]);
        let expected: Vec<IndexTableValueEntry> = vec![
    
      
            IndexTableValueEntry{
                target: String::from("foo3"),
                priority: Priority(99)
            },
            IndexTableValueEntry{
                target: String::from("foo"),
                priority: Priority(55)
            },
            IndexTableValueEntry{
                target: String::from("foo2"),
                priority: Priority(22)
            },
        ];
        assert_eq!(
            index.as_vec().await,
            expected
        );
    }

    #[tokio::test]
    async  fn test_lookup_by_value() {
        let index = IndexTableValue::from_vec(vec![
            (55, String::from("foo")),
            (22, String::from("foo2")),
            (99, String::from("foo3")),
        ]);
       
        
        assert_eq!(
            index.lookup_by_value("foo2").await,
            Some((2, 22))
        );

        assert_eq!(
            index.lookup_by_value("foo3").await,
            Some((0, 99))
        );
    }
    
    #[tokio::test]
    async fn test_update_or_add_entry() {
        let index = IndexTableValue::from_vec(vec![
            (55, String::from("foo")),
            (22, String::from("foo2")),
            (99, String::from("foo3")),
        ]);
       
        
        assert_eq!(
            index.lookup_by_value("foo2").await,
            Some((2, 22))
        );

        index.update_or_add_entry("foo2", 1200).await;
        assert_eq!(
            index.lookup_by_value("foo2").await,
            Some((0, 1200))
        );

        let expected: Vec<IndexTableValueEntry> = vec![
            IndexTableValueEntry{
                target: String::from("foo2"),
                priority: Priority(1200)
            },
         
            IndexTableValueEntry{
                target: String::from("foo3"),
                priority: Priority(99)
            },
            IndexTableValueEntry{
                target: String::from("foo"),
                priority: Priority(55)
            },

        ];
        assert_eq!(
            index.as_vec().await,
            expected
        );
    }
}
