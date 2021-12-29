use kwap_msg::*;

// NOTE: duplicated in src/lib
pub fn test_msg() -> (VecMessage, Vec<u8>) {
  let header: [u8; 4] = 0b01_00_0001_01000101_0000000000000001u32.to_be_bytes();
  let token: [u8; 1] = [254u8];
  let content_format: &[u8] = b"application/json";
  let options: [&[u8]; 2] = [&[0b_1100_1101u8, 0b00000011u8], content_format];
  let payload: [&[u8]; 2] = [&[0b_11111111u8], b"hello, world!"];
  let bytes = [header.as_ref(),
               token.as_ref(),
               options.concat().as_ref(),
               payload.concat().as_ref()].concat();

  let mut opts = Vec::new();
  let opt = Opt { delta: OptDelta(12),
                  value: OptValue(content_format.iter().copied().collect()) };
  opts.push(opt);

  let msg = VecMessage { id: Id(1),
                         ty: Type(0),
                         ver: Version(1),
                         token: Token(tinyvec::array_vec!([u8; 8] => 254)),
                         opts,
                         code: Code { class: 2, detail: 5 },
                         payload: Payload(b"hello, world!".into_iter().copied().collect()),
                         __optc: Default::default() };
  (msg, bytes)
}
