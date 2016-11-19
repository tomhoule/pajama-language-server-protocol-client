// use chomp::ascii::{is_horizontal_space, is_whitespace};
// use chomp::prelude::*;
// use std::io::Read;
// use std::iter::FromIterator;
// use std::collections::HashMap;
// use std::str;
// use serde_json;
// use serde::Deserialize;

// fn header<'a, I: U8Input<Buffer=&'a [u8]>>(i: I) -> SimpleResult<I, Option<HeaderType>> {
//     parse!{i;
//                    skip_while(is_whitespace);
//         let name = take_while(|c| c != b':');
//                    token(b':');
//                    skip_while(is_horizontal_space);
//         let value = take_while(|token| token != b'\r');
//                     string(b"\r\n");


//         ret {
//             if let (Ok(name_str), Ok(value_str)) = (str::from_utf8(name), str::from_utf8(value)) {
//                 HeaderType::from_raw(name_str, value_str)
//             } else {
//                 None
//             }
//         }
//     }
// }

// fn headers<'a, I: U8Input<Buffer=&'a [u8]>>(i: I) -> SimpleResult<I, Headers> {
//     many1(i, header)
//         .bind::<_, _, Error<u8>>(|i, headers: Vec<Option<HeaderType>>| {
//             i.ret(headers.iter().filter_map(|v| {
//                 match v {
//                     &Some(ref header_type) => Some((header_type.as_key(), Box::new(header_type.clone()))),
//                     &None => None
//                 }
//             }).collect::<Headers>())
//         })
// }

// fn read_content<R: Read>(reader: &mut R, size: usize) -> String {
//     let mut buf = Vec::with_capacity(size);
//     reader.read_exact(&mut buf).unwrap();
//     String::from_utf8(buf).unwrap()
// }

// fn content<I: U8Input<Token=u8>>(i: I, size: usize) -> SimpleResult<I, String> {
//     take(i, size).bind(|i, bytes| i.ret(String::from_utf8(bytes.to_vec()).unwrap()))
// }

// struct RawMessage {
//     headers: Headers,
//     content: String,
// }

// struct Message<C> {
//     headers: Headers,
//     content: C,
// }

// fn message<C: Deserialize>(i: &[u8]) -> Message<C> {
//     let raw_message = parse_only(raw_message, i).unwrap();
//     Message {
//         headers: raw_message.headers,
//         content: serde_json::from_str::<C>(&raw_message.content).unwrap()
//     }
// }
// /// Per the spec, calls can be batched and sent in one message as an array
// fn raw_message<I: U8Input<Token=u8>>(i: &[u8]) -> SimpleResult<I, RawMessage> {
//     headers(i).bind(|i, headers| {
//         if let Some(&box HeaderType::ContentLengthHeader(length)) = raw_message.headers.get("Content-Length") {
//             i.ret(RawMessage {
//                 headers: headers,
//                 content: content(i, length)
//             })
//         } else {
//             panic!("aw shite");
//         }
//     })
// }

// type ContentLength = usize;
// type ContentType = String;

// #[derive(Clone, Debug, PartialEq)]
// enum HeaderType {
//     ContentLengthHeader(ContentLength),
//     ContentTypeHeader(ContentType),
// }

// impl HeaderType {
//     fn from_raw(name: &str, value: &str) -> Option<HeaderType> {
//         match name {
//             "Content-Type" => Some(HeaderType::ContentTypeHeader(value.to_string())),
//             "Content-Length" => Some(HeaderType::ContentLengthHeader(value.parse::<usize>().unwrap())),
//             _ => None
//         }
//     }

//     fn as_key(&self) -> &'static str {
//         match *self {
//             HeaderType::ContentLengthHeader(_) => "Content-Length",
//             HeaderType::ContentTypeHeader(_) => "Content-Type",
//         }
//     }
// }

// type Headers = HashMap<&'static str, Box<HeaderType>>;

// struct Content;


// #[cfg(test)]
// mod test {
//     use serde_json;
//     use chomp::prelude::*;
//     use super::{Message, Headers, HeaderType, header, headers, content, message};
//     use std::io::Read;
// use std::str;

//     #[test]
//     fn header_parser_works() {
//         let expected = HeaderType::ContentLengthHeader(9000);
//         let actual = parse_only(header, b"Content-Length: 9000\r\n").unwrap();
//         assert_eq!(expected, actual.unwrap());
//     }

//     fn valid_headers() -> &'static [u8] {
//         "
// Content-Length: 1\r
// Content-Type: application/vscode-jsonrpc; charset=utf8\r
// \r\n
// ".as_bytes()
//     }

//     #[test]
//     fn headers_parse_works() {
//         let headers = parse_only(headers, valid_headers()).unwrap();
//         if let Some(&box HeaderType::ContentLengthHeader(length)) = headers.get("Content-Length") {
//             assert_eq!(length, 1);
//         } else {
//             panic!();
//         }
//     }

//     #[derive(Deserialize, PartialEq, Debug)]
//     struct FooTrue {
//         foo: bool,
//     }

//     fn valid_message() -> &'static [u8] {
//         "
// Content-Length: 11\r
// Content-Type: application/vscode-jsonrpc; charset=utf8\r
// \r
// {\"foo\": true}
// ".as_bytes()
//     }

//     #[test]
//     fn content_parser_works() {
//         let data = b"It's over 9000!!!";
//         let expected = "It's over".to_string();
//         let result = parse_only(|i| content(i, 9), data).unwrap();
//         assert_eq!(result, expected);
//     }

//     #[test]
//     fn content_can_read_to_exhaustion() {
//         let data = b"It's over 9000!!!";
//         let result = parse_only(|i| content(i, 17), data).unwrap();
//         assert_eq!(result, str::from_utf8(data).unwrap().to_string());
//     }

//     #[test]
//     fn content_works_with_multibyte_characters() {
//         let data = "ワンパタン";
//         let result = parse_only(|i| content(i, 15), data.as_bytes()).unwrap();
//         assert_eq!(result, data.to_string());
//     }

//     #[test]
//     fn message_can_parse_a_whole_message() {
//         let result: Message<FooTrue> = message(valid_message());
//         assert_eq!(result.content, FooTrue { foo: true });
//     }

//     #[test]
//     fn messages_can_parse_an_arbitrary_number_of_messages() {
//         panic!();
//     }
// }
