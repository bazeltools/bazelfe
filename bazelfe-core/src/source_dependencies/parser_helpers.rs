use nom::error::ParseError;

use nom::{combinator::map, sequence::tuple, IResult};

pub(in crate::source_dependencies) fn parser_to_unit<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl Fn(&'a str) -> IResult<&'a str, (), E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    map(inner, |_| ())
}

pub(in crate::source_dependencies) fn not_end_of_line(chr: char) -> bool {
    chr != '\n' && chr != '\r'
}

pub(in crate::source_dependencies) fn eat_till_end_of_line(
    input: &str,
) -> IResult<&str, (&str, &str)> {
    map(
        tuple((
            nom::bytes::complete::take_while(not_end_of_line),
            nom::bytes::complete::take_while(|chr| chr == '\r'),
            nom::bytes::complete::take_while_m_n(0, 1, |chr| chr == '\n'),
        )),
        |r| (r.0, r.2),
    )(input)
}
