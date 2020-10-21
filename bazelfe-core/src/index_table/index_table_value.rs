use std::{borrow::Cow};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexTableValueEntry {
    pub target: String,
    pub priority: u16
}
impl PartialOrd for IndexTableValueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl Ord for IndexTableValueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => {
                self.target.cmp(&other.target)
            }
            o => o
        }
    }
}
pub struct IndexTableValue(Vec<IndexTableValueEntry>);

impl Default for IndexTableValue {
    fn default() -> Self {
        IndexTableValue::new(Vec::default())
    }
}
impl IndexTableValue {

    pub fn new(data: Vec<(String, u16)>) -> Self {
        let mut transformed: Vec<IndexTableValueEntry> = data.into_iter().map(|e| {
            IndexTableValueEntry {
                target: e.0,
                priority: e.1
            }
        }).collect();
        transformed.sort();

        Self {
            0: transformed
        }
    }
    pub fn lookup_by_value<'b, S>(&self, target_v: S) -> Option<(usize, u16)>
    where S : Into<Cow<'b, str>> {

        let k = target_v.into();


        let mut idx = 0;
        for element in &self.0 {
            if element.target == k {
                return Some((idx, element.priority));
            }
            idx += 1;
        }
        None
    }
            
    pub fn update_or_add_entry<'b, S>(&mut self, target_v: S, priority: u16) -> ()
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
        match self.lookup_by_value(as_borrow) {
            Some((position, old_priority)) => {
                if old_priority == priority {
                    return;
                }
                let insert_position = match self.0.binary_search_by_key(&priority, |e| e.priority) {
                    Ok(current_position) => current_position,
                    Err(should_be_at_position) => should_be_at_position
                };
                if insert_position == position {
                    self.0[position].priority = priority;
                    return;
                }
                self.0.remove(position);
                let insert_position = if insert_position < position {
                    insert_position
                } else {
                    insert_position - 1
                };
                self.0.insert(insert_position, IndexTableValueEntry{target: cow.into_owned(), priority: priority});
            }
            None => {}
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blank_entry() {
        let index = IndexTableValue::default();
        let expected: Vec<IndexTableValueEntry> = Vec::default();
        assert_eq!(
            index.0,
            expected
        );
    }


    #[test]
    fn test_build_from_unsorted_list() {
        let index = IndexTableValue::new(vec![
            (String::from("foo"), 55),
            (String::from("foo2"), 22),
            (String::from("foo3"), 99),
        ]);
        let expected: Vec<IndexTableValueEntry> = vec![
            IndexTableValueEntry{
                target: String::from("foo2"),
                priority: 22
            },
            IndexTableValueEntry{
                target: String::from("foo"),
                priority: 55
            },
            IndexTableValueEntry{
                target: String::from("foo3"),
                priority: 99
            }

        ];
        assert_eq!(
            index.0,
            expected
        );
    }

    #[test]
    fn test_lookup_by_value() {
        let index = IndexTableValue::new(vec![
            (String::from("foo"), 55),
            (String::from("foo2"), 22),
            (String::from("foo3"), 99),
        ]);
       
        
        assert_eq!(
            index.lookup_by_value("foo2"),
            Some((0, 22))
        );

        assert_eq!(
            index.lookup_by_value("foo3"),
            Some((2, 99))
        );
    }
    
    #[test]
    fn test_update_or_add_entry() {
        let mut index = IndexTableValue::new(vec![
            (String::from("foo"), 55),
            (String::from("foo2"), 22),
            (String::from("foo3"), 99),
        ]);
       
        
        assert_eq!(
            index.lookup_by_value("foo2"),
            Some((0, 22))
        );

        index.update_or_add_entry("foo2", 1200);
        assert_eq!(
            index.lookup_by_value("foo2"),
            Some((2, 1200))
        );

        let expected: Vec<IndexTableValueEntry> = vec![
        
            IndexTableValueEntry{
                target: String::from("foo"),
                priority: 55
            },
            IndexTableValueEntry{
                target: String::from("foo3"),
                priority: 99
            },
            IndexTableValueEntry{
                target: String::from("foo2"),
                priority: 1200
            },

        ];
        assert_eq!(
            index.0,
            expected
        );
    }
}
