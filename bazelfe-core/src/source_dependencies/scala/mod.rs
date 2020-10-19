use crate::source_dependencies::parser_helpers::*;
use crate::source_dependencies::{Error, Import, ParsedFile, Result, SelectorType};

use nom::branch::alt;
use nom::bytes::complete::{is_a, is_not};
use nom::bytes::complete::{take_while, take_while1};
use nom::character::complete::{alphanumeric1, multispace0, space0, space1};
use nom::combinator::recognize;
use nom::error::ParseError;
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

fn tuple_extractor<'a, E>() -> impl Fn(&'a str) -> IResult<&str, (&str, Option<&str>), E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((
            multispace0,
            alt((
                take_while1(|chr: char| chr.is_alphanumeric() || chr == '_'),
                map(
                    tuple((
                        tag("`"),
                        take_while(|chr| chr != '\n' && chr != '\r' && chr != '`'),
                        tag("`"),
                    )),
                    |e| e.1,
                ),
            )),
            multispace0,
            opt(tuple((
                tag("=>"),
                multispace0,
                take_while1(|chr: char| chr.is_alphanumeric() || chr == '_'),
            ))),
            multispace0,
            opt(tag(",")),
            multispace0,
        )),
        |e| (e.1, e.3.map(|r| r.2)),
    )
}
fn consume_selector<'a, E>() -> impl Fn(&'a str) -> IResult<&str, SelectorType, E>
where
    E: ParseError<&'a str>,
{
    alt((
        map(
            tuple((
                multispace0,
                tag("."),
                multispace0,
                is_a("{"),
                many1(map(tuple_extractor(), |s| {
                    (s.0.to_string(), s.1.map(|e| e.to_string()))
                })),
                is_a("}"),
                multispace0,
            )),
            |r| SelectorType::SelectorList(r.4),
        ),
        map(tuple((multispace0, tag("._"))), |_| {
            SelectorType::WildcardSelector
        }),
        map(tuple((space0, is_not("."))), |_| SelectorType::NoSelector),
    ))
}

pub fn parse_import(line_number: u32, input: &str) -> IResult<&str, Import> {
    let (input, _) = tuple((multispace0, tag("import"), space1, multispace0))(input)?;

    let (input, extracted) = map(
        tuple((
            opt(tag("_root_.")),
            recognize(many1(tuple((
                opt(tag(".")),
                alphanumeric1,
                nom::bytes::complete::take_while(is_valid_import_segment_item),
            )))),
        )),
        |r| r.1,
    )(input)?;

    if input.len() > 0 {
        let (input, selector) = consume_selector()(&input)?;

        Ok((
            input,
            Import {
                line_number: line_number,
                prefix_section: extracted.to_string(),
                suffix: selector,
            },
        ))
    } else {
        Ok((
            input,
            Import {
                line_number: line_number,
                prefix_section: extracted.to_string(),
                suffix: SelectorType::NoSelector,
            },
        ))
    }
}

// END UTILITIES FOR IMPORT PARSING

// START UTILITIES FOR PACAKGE PARSING
fn extract_package_from_line(ln: &str) -> Result<&str> {
    let (remaining, res) = map(
        nom::combinator::complete(tuple((
            space0,
            tag("package"),
            space1,
            nom::bytes::complete::take_while1(|chr: char| {
                chr.is_alphanumeric() || chr == '.' || chr == '_'
            }),
            opt(tag(";")),
            space0,
            opt(tuple((
                alt((
                    parser_to_unit(tag("//")),
                    parser_to_unit(tuple((tag("/"), space0, tag("*")))),
                )),
                take_while(not_end_of_line),
            ))),
        ))),
        |tup| tup.3,
    )(ln)?;
    if remaining.len() > 0 {
        return Err(Error::UnexpectedRemainingData(remaining.to_string()));
    }
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
            package foo.bar.baz // have end of line comments here
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
    }

    #[test]
    fn parse_simple_input() {
        let sample_input = "import com.twitter.scalding.RichDate";
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
        import com.twitter.scalding.RichDate
        import com.twitter.scalding.RichDate

        import com.twitter.scalding.RichDate
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
    fn sub_sections() {
        let sample_input = "import com.twitter.scalding.{RichDate, DateOps}";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::SelectorList(vec![
                ("RichDate".to_string(), None),
                ("DateOps".to_string(), None),
            ]),
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_wildcard() {
        let sample_input = "import com.twitter.scalding._";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::WildcardSelector,
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    fn tuple_extractor_parser(i: &str) -> IResult<&str, (&str, Option<&str>)> {
        tuple_extractor()(i)
    }

    #[test]
    fn test_alias() {
        let sample_input = "import com.twitter.scalding.{RichDate => MyRichDate}";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::SelectorList(vec![(
                "RichDate".to_string(),
                Some("MyRichDate".to_string()),
            )]),
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_backticks() {
        let sample_input = "import com.twitter.scalding.{`RichDate foo bar baz` => MyRichDate}";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::SelectorList(vec![(
                "RichDate foo bar baz".to_string(),
                Some("MyRichDate".to_string()),
            )]),
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_underscores() {
        let sample_input = "import _root_.com.twit__ter.scalding.{My_Richness => MyRichD_ate}";
        let expected_results = vec![Import {
            line_number: 1,
            prefix_section: "com.twit__ter.scalding".to_string(),
            suffix: SelectorType::SelectorList(vec![(
                "My_Richness".to_string(),
                Some("MyRichD_ate".to_string()),
            )]),
        }];

        let parsed_result = parse_imports(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_tuple_extractor() {
        let sample_input = "RichDate, DateOps, Src => DestTpe";
        let (remaining, parsed_result) = tuple_extractor_parser(sample_input).unwrap();
        assert_eq!(parsed_result.0, "RichDate");
        assert_eq!(remaining, "DateOps, Src => DestTpe");

        let (remaining, parsed_result) = tuple_extractor_parser(remaining).unwrap();
        assert_eq!(parsed_result.0, "DateOps");
        assert_eq!(remaining, "Src => DestTpe");

        let (remaining, parsed_result) = tuple_extractor_parser(remaining).unwrap();
        assert_eq!(parsed_result.0, "Src");
        assert_eq!(parsed_result.1, Some("DestTpe"));
        assert_eq!(remaining, "");
    }
}
