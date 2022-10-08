Interpret the full length or delta value of the first byte of an option (byte 0).

This does the heavy lifting of:
> if value < 13, yield it.
> if value == 13, interpret byte 1 as a u8 and yield it + 13.
> if value == 14, interpret bytes 1 & 2 as a u16 and yield it + 269.

We can invoke this shared logic on both length and delta under the following assumptions:
 - this function will be invoked for delta first
 - the same iterator will be used to interpret the length

If these both hold true, then any extended bytes for the delta will not be seen when we calculate the length.

e.g.

```ignore
/*
the first 4 bits are 13, indicating that the delta is (13 + byte 1); 14
the last  4 bits are 14, indicating that the length is (269 + u16::from_be_bytes(bytes 2 & 3)); 270

  0           1          2          3
| 1101 1101 | 00000001 | 00000000 | 00000001 |
*/

let byte0: u8 = 0b1101_1110;
let bytes: Vec<u8> = vec![0b00000001, 0b00000010, 0b00000001];
let bytes_iter = bytes.into_iter();

// first, we invoke this function for delta:
let del = opt_len_or_delta(13, &mut bytes_iter, err).unwrap();
assert_eq!(del, 14);

// if we were to collect the iterator here, we would be missing byte 1 since it was consumed in order to interpret delta.
assert_eq!(bytes_iter.clone().collect::<Vec<_>>(), vec![0b00000010])

// this means we can reuse the original iterator and treat it the same way for length:
let len = opt_len_or_delta(13, &mut bytes_iter, err).unwrap();
assert_eq!(len, 270);
```
