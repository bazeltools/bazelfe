pub(crate) fn get_guesses_for_class_name(class_name: &str) -> Vec<(u16, String)> {
    let mut sections: Vec<&str> = class_name.split(".").collect();

    // heuristic looking for a class name, to ignore separate from the package...

    let mut idx = 0;
    let mut found = false;
    while idx < sections.len() {
        let ele = &sections[idx];
        if ele.starts_with(|ch: char| ch.is_uppercase()) {
            found = true;
            break;
        }
        idx += 1;
    }

    if found {
        sections.truncate(idx);
    }

    if sections.len() < 3 {
        return vec![];
    }

    let suffix = format!("{}:{}", sections.join("/"), sections.last().unwrap());

    vec![
        (0, format!("//src/main/scala/{}", suffix).to_string()),
        (0, format!("//src/main/java/{}", suffix).to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_for_class_name() {
        assert_eq!(
            get_guesses_for_class_name("com.example.foo.bar.baz"),
            vec![
                (
                    0,
                    String::from("//src/main/scala/com/example/foo/bar/baz:baz")
                ),
                (
                    0,
                    String::from("//src/main/java/com/example/foo/bar/baz:baz")
                )
            ]
        );
    }

    #[test]
    fn test_guess_for_class_name_too_short() {
        assert_eq!(
            get_guesses_for_class_name("com.example"),
            Vec::<(u16, String)>::new()
        );
    }

    #[test]
    fn test_guess_for_class_name_strip_class_name() {
        assert_eq!(
            get_guesses_for_class_name("com.example.foo.bar.baz.MyObject.InnerObject"),
            vec![
                (
                    0,
                    String::from("//src/main/scala/com/example/foo/bar/baz:baz")
                ),
                (
                    0,
                    String::from("//src/main/java/com/example/foo/bar/baz:baz")
                )
            ]
        );
    }

    #[test]
    fn test_guess_for_class_name_too_short_post_strip() {
        assert_eq!(
            get_guesses_for_class_name("com.example.MyObject.MyObject.InnerObject"),
            Vec::<(u16, String)>::new()
        );
    }

    #[test]
    fn test_guess_for_class_start_with_class_name() {
        assert_eq!(
            get_guesses_for_class_name("MyObject.MyObject.InnerObject"),
            Vec::<(u16, String)>::new()
        );
    }
}
