Low-level representation of a message
that has been parsed from a byte array

To convert an iterator of bytes into a Message, there is a provided trait [`crate::TryFromBytes`].

```
use toad_msg::TryFromBytes;
use toad_msg::*;
# //                       version  token len  code (2.05 Content)
# //                       |        |          /
# //                       |  type  |         /  message ID
# //                       |  |     |        |   |
# //                       vv vv vvvv vvvvvvvv vvvvvvvvvvvvvvvv
# let header: [u8; 4] = 0b_01_00_0001_01000101_0000000000000001u32.to_be_bytes();
# let token: [u8; 1] = [254u8];
# let content_format: &[u8] = b"application/json";
# let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
# let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
let packet: Vec<u8> = /* bytes! */
# [header.as_ref(), token.as_ref(), options.concat().as_ref(), payload.concat().as_ref()].concat();

// `VecMessage` uses `Vec` as the backing structure for byte buffers
let msg = VecMessage::try_from_bytes(packet.clone()).unwrap();
# let opt = Opt {
#   delta: OptDelta(12),
#   value: OptValue(content_format.iter().map(|u| *u).collect()),
# };
let mut opts_expected = /* create expected options */
# Vec::new();
# opts_expected.push(opt);

let expected = VecMessage {
  id: Id(1),
  ty: Type(0),
  ver: Version(1),
  token: Token(tinyvec::array_vec!([u8; 8] => 254)),
  opts: opts_expected,
  code: Code {class: 2, detail: 5},
  payload: Payload(b"hello, world!".to_vec()),
  __optc: Default::default(),
};

assert_eq!(msg, expected);
```

See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
