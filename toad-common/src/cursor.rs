/// A cursor over a byte array (std- and alloc-less port of [`std::io::Cursor`])
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor<T> {
  t: T,
  cursor: usize,
  len: usize,
}

impl<T: AsRef<[u8]>> Cursor<T> {
  /// Creates a new cursor
  pub fn new(t: T) -> Cursor<T> {
    let len = t.as_ref().len();
    Cursor { t, cursor: 0, len }
  }

  /// Unwraps the cursor, discarding its internal position
  pub fn into_inner(self) -> T {
    self.t
  }

  fn peek_(len: usize, cursor: usize, t: &T, n: usize) -> Option<&[u8]> {
    if n > len - cursor {
      None
    } else {
      Some(&t.as_ref()[cursor..cursor + n])
    }
  }

  /// Take the next byte in the cursor, returning None
  /// if the cursor is exhausted.
  ///
  /// Runs in O(1) time.
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
                                                    self.cursor += n;
                                                    a
                                                  })
                                                  .unwrap_or_else(|| self.until_end())
  }

  /// Take `n` bytes from the cursor, returning None if
  /// the end of the buffer is encountered.
  ///
  /// Runs in O(1) time.
  pub fn take_exact(&mut self, n: usize) -> Option<&[u8]> {
    Self::peek_(self.len, self.cursor, &self.t, n).map(|a| {
                                                    self.cursor += n;
                                                    a
                                                  })
  }

  /// Without advancing the position, look at the next
  /// `n` bytes, or until the end if there are less than `n` bytes
  /// remaining.
  ///
  /// Runs in O(1) time.
  pub fn peek(&self, n: usize) -> &[u8] {
    Self::peek_(self.len, self.cursor, &self.t, n).unwrap_or(self.until_end())
  }

  /// Without advancing the position, look at the next
  /// `n` bytes, returning None if there are less than `n` bytes
  /// remaining.
  ///
  /// Runs in O(1) time.
  pub fn peek_exact(&self, n: usize) -> Option<&[u8]> {
    Self::peek_(self.len, self.cursor, &self.t, n)
  }

  /// Consume bytes until a predicate returns `false`.
  ///
  /// Runs in O(n) time.
  pub fn take_while(&mut self, mut f: impl FnMut(u8) -> bool) -> &[u8] {
    if self.is_exhausted() {
      return &[];
    }

    let mut i = 0;

    loop {
      if i + 1 >= self.len {
        break &self.t.as_ref()[self.cursor..];
      }

      i += 1;

      if !f(self.t.as_ref()[i]) {
        let out = &self.t.as_ref()[self.cursor..i];
        self.cursor += i;
        break out;
      }
    }
  }

  /// Whether the cursor has reached the end
  /// of the buffer.
  ///
  /// Runs in O(1) time.
  pub fn is_exhausted(&self) -> bool {
    self.cursor + 1 >= self.len
  }

  /// Get the bytes remaining in the buffer
  ///
  /// Runs in O(1) time.
  pub fn until_end(&self) -> &[u8] {
    if self.is_exhausted() {
      &[]
    } else {
      &self.t.as_ref()[self.cursor..]
    }
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
    let mut cur = Cursor::new(vec![1, 2, 3]);
    assert_eq!(cur.peek_exact(3), Some([1, 2, 3].as_ref()));
    assert_eq!(cur.peek_exact(1), Some([1].as_ref()));
    assert_eq!(cur.take_exact(4), None);
  }

  #[test]
  pub fn take_while() {
    let mut cur = Cursor::new(vec![2, 4, 6, 7]);
    assert_eq!(cur.take_while(|n| n % 2 == 0), &[2, 4, 6]);
    assert_eq!(cur.next(), Some(7));
    assert_eq!(cur.take_while(|_| true), &[]);
  }
}
