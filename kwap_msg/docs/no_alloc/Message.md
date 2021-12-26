Low-level representation of a message
that has been parsed from a byte array

To convert an iterator of bytes into a Message, there is a provided trait [`crate::TryFromBytes`].

```
use kwap_msg::TryFromBytes;
use kwap_msg::alloc::*;
use kwap_msg::no_alloc;
# use tinyvec::ArrayVec;

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

// Heap allocated version from `kwap_msg::alloc`
let msg = Message::try_from_bytes(packet.clone()).unwrap();
# let opt = Opt {
#   delta: OptDelta(12),
#   value: OptValue(content_format.iter().map(|u| *u).collect()),
# };
let mut opts_expected = /* create expected options */
# Vec::new();
# opts_expected.push(opt);

let expected = Message {
  id: Id(1),
  ty: Type(0),
  ver: Version(1),
  token: Token(tinyvec::array_vec!([u8; 8] => 254)),
  opts: opts_expected,
  code: Code {class: 2, detail: 5},
  payload: Payload(b"hello, world!".to_vec()),
};

assert_eq!(msg, expected);

// Stack allocated version from `kwap_msg::no_alloc`
// respective capacities are:
// - the size of the message payload buffer (13)
// - number of options (1)
// - size of option value buffers (16)
let msg = no_alloc::Message::<13, 1, 16>::try_from_bytes(packet).unwrap();
# let opt = no_alloc::Opt::<16> {
#   delta: OptDelta(12),
#   value: no_alloc::OptValue(content_format.iter().copied().collect()),
# };
let mut opts_expected = /* create expected options */
# ArrayVec::new();
# opts_expected.push(opt);

let expected = no_alloc::Message::<13, 1, 16> {
  id: Id(1),
  ty: Type(0),
  ver: Version(1),
  token: Token(tinyvec::array_vec!([u8; 8] => 254)),
  opts: opts_expected,
  code: Code {class: 2, detail: 5},
  payload: no_alloc::Payload(b"hello, world!".into_iter().copied().collect()),
};

assert_eq!(msg, expected);
```

See [RFC7252 - Message Details](https://datatracker.ietf.org/doc/html/rfc7252#section-3) for context
