use crate::source_dependencies::parser_helpers::*;
use crate::source_dependencies::{Import, ParsedFile, Result, SelectorType};

use nom::character::complete::{alphanumeric1, multispace0, space0, space1};
use nom::combinator::recognize;
use nom::multi::many1;
use nom::{
    bytes::complete::tag,
    combinator::{map, opt},
    sequence::tuple,
    IResult,
};

fn is_valid_import_segment_item(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn parse_import(line_number: u32, input: &str) -> IResult<&str, Import> {
    let (input, _) = tuple((multispace0, tag("import"), space1, multispace0))(input)?;

    let (input, (extracted, opt_wildcard)) = map(
        tuple((
            recognize(many1(tuple((
                opt(tag(".")),
                alphanumeric1,
                nom::bytes::complete::take_while(is_valid_import_segment_item),
            )))),
            opt(tag(".*")),
            space0,
            tag(";"),
        )),
        |r| (r.0, r.1),
    )(input)?;

    let selector = if opt_wildcard.is_none() {
        SelectorType::NoSelector
    } else {
        SelectorType::WildcardSelector
    };

    Ok((
        input,
        Import {
            line_number: line_number,
            prefix_section: extracted.to_string(),
            suffix: selector,
        },
    ))
}

// END UTILITIES FOR IMPORT PARSING

// START UTILITIES FOR PACAKGE PARSING
fn extract_package_from_line(ln: &str) -> Result<&str> {
    let (_, res) = map(
        nom::combinator::complete(tuple((
            space0,
            tag("package"),
            space1,
            nom::bytes::complete::take_while1(|chr: char| {
                chr.is_alphanumeric() || chr == '.' || chr == '_'
            }),
            space0,
            tag(";"),
        ))),
        |tup| tup.3,
    )(ln)?;
    Ok(res)
}

fn extract_package_from_file(file_lines: &str) -> Result<Option<&str>> {
    for ln in file_lines.lines() {
        if ln.contains("package") {
            match extract_package_from_line(ln) {
                Ok(pkg) => return Ok(Some(pkg)),
                Err(_) => (),
            }
        }
    }
    Ok(None)
}
// END UTILITIES FOR PACAKGE PARSING

// PUBLIC METHODS
pub fn parse_imports(input: &str) -> Result<Vec<Import>> {
    let mut results_vec = Vec::new();
    let mut line_number = 1;
    let mut remaining_input = input;
    while remaining_input.len() > 3 {
        match eat_till_end_of_line(remaining_input) {
            Ok((r, (current_line, end_of_line_eaten))) => {
                if current_line.len() > 0 && current_line.contains("import") {
                    match parse_import(line_number, remaining_input) {
                        Ok((_, found)) => results_vec.push(found),
                        Err(_) => (),
                    };
                }

                // if we never found an end of line, must be end of file.
                if end_of_line_eaten.len() > 0 {
                    remaining_input = r;
                } else {
                    remaining_input = "";
                }
            }
            Err(_) => {
                remaining_input = "";
            }
        }
        line_number = line_number + 1;
    }

    Ok(results_vec)
}

pub fn parse_file(input: &str) -> Result<ParsedFile> {
    let package = extract_package_from_file(input)?;

    let imports = parse_imports(input)?;

    Ok(ParsedFile {
        package_name: package.map(|e| e.to_string()),
        imports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_header_line() {
        assert_eq!(
            extract_package_from_file(
                "
            asdf
            asdf
            package foo.bar.baz;
            asdf
            asdf"
            )
            .unwrap(),
            Some("foo.bar.baz")
        );
    }

    #[test]
    fn parse_header_line_with_comments() {
        assert_eq!(
            extract_package_from_file(
                "
            asdf
            asdf
            package foo.bar.baz; // have end of line comments here
            asdf
            asdf"
            )
            .unwrap(),
            Some("foo.bar.baz")
        );

        assert_eq!(
            extract_package_from_file(
                "
            asdf
            asdf
            package foo.bar.baz i am totally invalid and not a package line
            asdf
            asdf"
            )
            .unwrap(),
            None
        );

        assert_eq!(
            extract_package_from_file(
                "
            asdf
            asdf
            package foo.bar.baz
            asdf;
            asdf"
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn parse_simple_input() {
        let sample_input = "import com.twitter.scalding.RichDate;";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twitter.scalding.RichDate".to_string(),
            suffix: SelectorType::NoSelector,
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn parse_multiple_lines_input() {
        let sample_input = "
        import com.twitter.scalding.RichDate;
        import com.twitter.scalding.RichDate;

        import com.twitter.scalding.RichDate;
        ";
        let expected_results = vec![
            Import {
                line_number: 2,
                prefix_section: "com.twitter.scalding.RichDate".to_string(),
                suffix: SelectorType::NoSelector,
            },
            Import {
                line_number: 3,
                prefix_section: "com.twitter.scalding.RichDate".to_string(),
                suffix: SelectorType::NoSelector,
            },
            Import {
                line_number: 5,
                prefix_section: "com.twitter.scalding.RichDate".to_string(),
                suffix: SelectorType::NoSelector,
            },
        ];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_wildcard() {
        let sample_input = "import com.twitter.scalding.*;";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::WildcardSelector,
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_underscores() {
        let sample_input = "import com.twit__ter.scalding.My_Richness;";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twit__ter.scalding.My_Richness".to_string(),
            suffix: SelectorType::NoSelector,
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }
}
