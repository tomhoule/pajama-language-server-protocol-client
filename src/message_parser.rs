//! This module is responsible for consuming the message headers and returning json values out of
//! the language server stdout.
use chomp::ascii::{is_horizontal_space, is_whitespace, skip_whitespace};
use chomp::prelude::*;
use std::io::Read;
use std::iter::FromIterator;
use std::collections::HashMap;
use std::str;
use serde_json as json;
use serde::Deserialize;

fn header<'a, I: U8Input<Buffer=&'a [u8], Token=u8>>(i: I) -> SimpleResult<I, HeaderType> {
    parse!{i;
                   skip_while(is_whitespace);
        let name = take_while(|c| c != b':');
                   token(b':');
                   skip_while(is_horizontal_space);
        let value = take_while(|token| token != b'\r');
                    string(b"\r\n");


        ret {
            if let (Ok(name_str), Ok(value_str)) = (str::from_utf8(name), str::from_utf8(value)) {
                HeaderType::from_raw(name_str, value_str)
            } else {
                HeaderType::Invalid
            }
        }
    }
}

fn consume_whitespace<'a, I:U8Input<Buffer=&'a[u8], Token=u8>, T>(i: I, t: T) -> SimpleResult<I, T> {
    parse!{i;
            skip_whitespace();
        ret { t }
    }
}

fn headers<'a, I: U8Input<Buffer=&'a [u8], Token=u8>>(i: I) -> SimpleResult<I, Headers> {
    many1(i, header)
        .bind(|i, headers| consume_whitespace(i, headers))
        .bind(|i, headers: Vec<HeaderType>| {
            i.ret(headers.into_iter().collect::<Headers>())
        })
}

fn content<'a, I: U8Input<Token=u8, Buffer=&'a [u8]>>(i: I, size: usize) -> SimpleResult<I, &'a[u8]> {
    take(i, size).bind(|i, bytes| i.ret(bytes))
}

struct Message {
    headers: Headers,
    content: json::Value,
}

pub fn message<'a, I: U8Input<Buffer=&'a [u8], Token=u8>>(i: I) -> SimpleResult<I, Result<json::Value, json::Error>> {
    headers(i).bind(|i, headers| {
        content(i, headers.content_length)
    }).bind(|i, buffer| {
        i.ret(json::from_slice(buffer))
    })
}

type ContentLength = usize;
type ContentType = String;

#[derive(Clone, Debug, PartialEq)]
enum HeaderType {
    ContentLengthHeader(ContentLength),
    ContentTypeHeader(ContentType),
    Other,
    Invalid,
}

impl HeaderType {
    fn from_raw(name: &str, value: &str) -> HeaderType {
        match name {
            "Content-Type" => HeaderType::ContentTypeHeader(value.to_string()),
            "Content-Length" => {
                if let Ok(length) = value.parse::<usize>() {
                    HeaderType::ContentLengthHeader(length)
                } else {
                    HeaderType::Invalid
                }
            }
            _ => HeaderType::Other
        }
    }
}

struct Headers {
    content_length: ContentLength,
}

impl FromIterator<HeaderType> for Headers {
    fn from_iter<I: IntoIterator<Item=HeaderType>>(iter: I) -> Self {
        for header in iter {
            if let HeaderType::ContentLengthHeader(length) = header {
                return Headers {
                    content_length: length
                }
            }
        }
        unreachable!()
    }
}

#[cfg(test)]
mod test {
    use serde_json::builder::ObjectBuilder;
    use chomp::prelude::*;
    use super::{Message, Headers, HeaderType, header, headers, content, message};
    use std::io::Read;
use std::str;

    #[test]
    fn header_parser_works() {
        let expected = HeaderType::ContentLengthHeader(9000);
        let actual = parse_only(header, b"Content-Length: 9000\r\n").unwrap();
        assert_eq!(expected, actual);
    }

    fn valid_headers() -> &'static [u8] {
        "
Content-Length: 1\r
Content-Type: application/vscode-jsonrpc; charset=utf8\r
\r\n
".as_bytes()
    }

    #[test]
    fn headers_parse_works() {
        let headers = parse_only(headers, valid_headers()).unwrap();
        assert_eq!(headers.content_length, 1);
    }

    fn valid_message() -> &'static [u8] {
        "
Content-Length: 13\r
Content-Type: application/vscode-jsonrpc; charset=utf8\r
\r
{\"foo\": true}
".as_bytes()
    }

    #[test]
    fn content_parser_works() {
        let data = b"It's over 9000!!!";
        let expected = "It's over";
        let result = parse_only(|i| content(i, 9), data).unwrap();
        assert_eq!(result, expected.as_bytes());
    }

    #[test]
    fn content_can_read_to_exhaustion() {
        let data = b"It's over 9000!!!";
        let result = parse_only(|i| content(i, 17), data).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn content_works_with_multibyte_characters() {
        let data = "ワンパタン";
        let result = parse_only(|i| content(i, 15), data.as_bytes()).unwrap();
        assert_eq!(result, data.as_bytes());
    }

    #[test]
    fn message_can_parse_a_whole_message() {
        let result = parse_only(message, valid_message()).unwrap().unwrap();
        assert_eq!(result, ObjectBuilder::new().insert("foo", true).build());
    }
}
