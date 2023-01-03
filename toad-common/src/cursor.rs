/// A cursor over a byte array (std- and alloc-less port of [`std::io::Cursor`])
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor<T> {
  t: T,
  cursor: usize,
  len: usize,
}

impl<T: AsRef<[u8]>> Cursor<T> {
  fn is_exhausted_(cursor: usize, len: usize) -> bool {
    Self::remaining_(cursor, len) <= 0
  }

  fn remaining_(cursor: usize, len: usize) -> isize {
    len as isize - cursor as isize
  }

  fn peek_(len: usize, cursor: usize, t: &T, n: usize) -> Option<&[u8]> {
    if n as isize > Self::remaining_(cursor, len) {
      None
    } else {
      Some(&t.as_ref()[cursor..cursor + n])
    }
  }

  fn skip_(cursor: &mut usize, len: usize, n: usize) -> usize {
    if Self::is_exhausted_(*cursor, len) {
      0
    } else if *cursor + n > len {
      let left = len - *cursor;
      *cursor += left;
      left
    } else {
      *cursor += n;
      n
    }
  }

  fn peek_until_end_(cursor: usize, len: usize, t: &T) -> &[u8] {
    if Self::is_exhausted_(cursor, len) {
      &[]
    } else {
      &t.as_ref()[cursor..]
    }
  }

  fn seek_to_end_(cursor: &mut usize, len: usize) {
    *cursor = len;
  }

  fn take_until_end_<'a, 'b>(cursor: &'a mut usize, len: usize, t: &'b T) -> &'b [u8] {
    let out = Self::peek_until_end_(*cursor, len, t);
    Self::seek_to_end_(cursor, len);

    out
  }

  /// Creates a new cursor
  pub fn new(t: T) -> Cursor<T> {
    let len = t.as_ref().len();
    Cursor { t, cursor: 0, len }
  }

  /// Unwraps the cursor, discarding its internal position
  pub fn into_inner(self) -> T {
    self.t
  }

  /// Take the next byte in the cursor, returning None
  /// if the cursor is exhausted.
  ///
  /// Runs in O(1) time.
  #[allow(clippy::should_implement_trait)]
  pub fn next(&mut self) -> Option<u8> {
    self.take_exact(1).and_then(|a| match a {
                        | &[a] => Some(a),
                        | _ => None,
                      })
  }

  /// Take `n` bytes from the cursor, stopping early if
  /// the end of the buffer is encountered.
  ///
  /// Runs in O(1) time.
  pub fn take(&mut self, n: usize) -> &[u8] {
    Self::peek_(self.len, self.cursor, &self.t, n).map(|a| {
                                                    Self::skip_(&mut self.cursor, self.len, n);
                                                    a
                                                  })
                                                  .unwrap_or_else(|| {
                                                    Self::take_until_end_(&mut self.cursor,
                                                                          self.len,
                                                                          &self.t)
                                                  })
  }

  /// Take `n` bytes from the cursor, returning None if
  /// the end of the buffer is encountered.
  ///
  /// Runs in O(1) time.
  pub fn take_exact(&mut self, n: usize) -> Option<&[u8]> {
    Self::peek_(self.len, self.cursor, &self.t, n).map(|a| {
                                                    Self::skip_(&mut self.cursor, self.len, n);
                                                    a
                                                  })
  }

  /// Without advancing the position, look at the next
  /// `n` bytes, or until the end if there are less than `n` bytes
  /// remaining.
  ///
  /// Runs in O(1) time.
  pub fn peek(&self, n: usize) -> &[u8] {
    Self::peek_(self.len, self.cursor, &self.t, n).unwrap_or_else(|| self.peek_until_end())
  }

  /// Without advancing the position, look at the next
  /// `n` bytes, returning None if there are less than `n` bytes
  /// remaining.
  ///
  /// Runs in O(1) time.
  pub fn peek_exact(&self, n: usize) -> Option<&[u8]> {
    Self::peek_(self.len, self.cursor, &self.t, n)
  }

  /// Advance the cursor by `n` bytes.
  ///
  /// Returns the actual number of bytes skipped:
  ///  - Equal to n if there are at least n more bytes in the buffer
  ///  - Less than n if n would seek past the end
  ///  - Zero if the cursor is already exhausted
  ///
  /// Runs in O(1) time.
  pub fn skip(&mut self, n: usize) -> usize {
    Self::skip_(&mut self.cursor, self.len, n)
  }

  /// Consume bytes until a predicate returns `false` or the end is reached.
  ///
  /// Runs in O(n) time.
  pub fn take_while(&mut self, mut f: impl FnMut(u8) -> bool) -> &[u8] {
    if self.is_exhausted() {
      return &[];
    }

    (self.cursor..self.len).into_iter()
                           .take_while(|ix| f(self.t.as_ref()[*ix]))
                           .last()
                           .map(|end_ix| {
                             let out = &self.t.as_ref()[self.cursor..=end_ix];
                             self.cursor = end_ix + 1;
                             out
                           })
                           .unwrap_or(&[])
  }

  /// Whether the cursor has reached the end
  /// of the buffer.
  ///
  /// Runs in O(1) time.
  pub fn is_exhausted(&self) -> bool {
    Self::is_exhausted_(self.cursor, self.len)
  }

  /// The number of elements not yet consumed
  ///
  /// Runs in O(1) time.
  pub fn remaining(&self) -> usize {
    Self::remaining_(self.cursor, self.len).max(0) as usize
  }

  /// Get the bytes remaining in the buffer without advancing the position.
  ///
  /// Runs in O(1) time.
  pub fn peek_until_end(&self) -> &[u8] {
    Self::peek_until_end_(self.cursor, self.len, &self.t)
  }

  /// Get the bytes remaining in the buffer, advancing
  /// the position to the end.
  ///
  /// Runs in O(1) time.
  pub fn take_until_end(&mut self) -> &[u8] {
    Self::take_until_end_(&mut self.cursor, self.len, &self.t)
  }

  /// Get the position the cursor points to within
  /// the buffer
  pub fn position(&self) -> usize {
    self.cursor
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn peek_until_end() {
    let cur = Cursor::new(vec![]);
    assert_eq!(cur.peek_until_end(), &[]);

    let cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.peek_until_end(), &[1, 2, 3]);

    let mut cur = Cursor::new(vec![1, 2, 3]);
    cur.skip(1);
    assert_eq!(cur.peek_until_end(), &[2, 3]);
  }

  #[test]
  pub fn take_until_end() {
    let mut cur = Cursor::new(vec![]);
    assert_eq!(cur.take_until_end(), &[]);
    assert_eq!(cur.take_until_end(), &[]);
    assert_eq!(cur.take_until_end(), &[]);

    let mut cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.take_until_end(), &[1, 2, 3]);

    let mut cur = Cursor::new(vec![1, 2, 3]);
    cur.skip(1);
    assert_eq!(cur.take_until_end(), &[2, 3]);
    assert_eq!(cur.peek_until_end(), &[]);
  }

  #[test]
  pub fn next() {
    let mut cur = Cursor::new(vec![1]);
    assert_eq!(cur.next(), Some(1));
    assert_eq!(cur.next(), None);
    assert_eq!(cur.next(), None);
  }

  #[test]
  pub fn take() {
    let mut cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.take(2), &[1, 2]);
    assert_eq!(cur.take(1), &[3]);
    assert_eq!(cur.take(1), &[]);
  }

  #[test]
  pub fn peek() {
    let mut cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.peek(2), &[1, 2]);
    assert_eq!(cur.peek(1), &[1]);
    assert_eq!(cur.peek(4), &[1, 2, 3]);
    cur.take(3);
    assert_eq!(cur.peek(1), &[]);
  }

  #[test]
  pub fn take_exact() {
    let mut cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.take_exact(2), Some([1, 2].as_ref()));
    assert_eq!(cur.take_exact(2), None);
    assert_eq!(cur.take_exact(1), Some([3].as_ref()));
  }

  #[test]
  pub fn peek_exact() {
    let cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.peek_exact(3), Some([1, 2, 3].as_ref()));
    assert_eq!(cur.peek_exact(1), Some([1].as_ref()));
    assert_eq!(cur.peek_exact(4), None);
  }

  #[test]
  pub fn take_while() {
    let mut cur = Cursor::new("abc/def");

    let til_slash = |c: &mut Cursor<&str>| {
      core::str::from_utf8(c.take_while(|b| (b as char) != '/')).unwrap()
                                                                .to_string()
    };

    assert_eq!(til_slash(&mut cur), "abc".to_string());
    cur.skip(1);
    assert_eq!(til_slash(&mut cur), "def".to_string());
    assert_eq!(til_slash(&mut cur), "".to_string());

    let mut cur = Cursor::new("a");
    assert_eq!(til_slash(&mut cur), "a");

    let mut cur = Cursor::new("");
    assert_eq!(til_slash(&mut cur), "");

    let mut cur = Cursor::new("ab");
    assert_eq!(til_slash(&mut cur), "ab");

    let mut cur = Cursor::new("/abcd");
    assert_eq!(til_slash(&mut cur), "");
  }

  #[test]
  pub fn seek() {
    let mut cur = Cursor::new(vec![1, 2, 3, 4]);
    assert_eq!(cur.skip(0), 0); // 0 <- cursor
    assert_eq!(cur.skip(1), 1); // 1
    assert_eq!(cur.skip(2), 2); // 3
    assert_eq!(cur.skip(1), 1); // 4
    assert_eq!(cur.skip(1), 0); // 4
  }
}
