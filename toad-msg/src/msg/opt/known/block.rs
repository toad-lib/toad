/// Three items of information may need to be transferred in a
/// Block (Block1 or Block2) option:
/// * the size of the block ([`Block::size`])
/// * whether more blocks are following ([`Block::more`])
/// * the relative number of the block ([`Block::num`]) within a sequence of blocks with the given size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block(u32);

impl Block {
  #[allow(missing_docs)]
  pub fn new(size: u16, num: u32, more: bool) -> Self {
    let num = num << 4;
    let more = u32::from(more) << 3;
    let size = f32::from(size.max(16).min(1024)).log2() as u32 - 4;

    Self(num | more | size)
  }

  #[allow(missing_docs)]
  pub fn size(&self) -> u16 {
    let szx = (self.0 & 0b111).min(6);
    2u16.pow(szx + 4)
  }

  #[allow(missing_docs)]
  pub fn more(&self) -> bool {
    (self.0 & 0b1000) >> 3 == 1
  }

  #[allow(missing_docs)]
  pub fn num(&self) -> u32 {
    self.0 >> 4
  }
}

impl From<Block> for u32 {
  fn from(b: Block) -> Self {
    b.0
  }
}

impl From<u32> for Block {
  fn from(n: u32) -> Self {
    Block(n)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn block() {
    let b = Block(33);
    assert_eq!(b.size(), 32);
    assert_eq!(b.num(), 2);
    assert_eq!(b.more(), false);

    let b = Block(59);
    assert_eq!(b.size(), 128);
    assert_eq!(b.num(), 3);
    assert_eq!(b.more(), true);

    assert_eq!(Block::new(32, 2, false), Block(33));
    assert_eq!(Block::new(128, 3, true), Block(59));
  }

  #[test]
  fn size_rounds_down_to_nearest_power_of_two() {
    assert_eq!(Block::new(0, 1, false).size(), 16);
    assert_eq!(Block::new(10, 1, false).size(), 16);
    assert_eq!(Block::new(17, 1, false).size(), 16);
    assert_eq!(Block::new(31, 1, false).size(), 16);
    assert_eq!(Block::new(33, 1, false).size(), 32);
    assert_eq!(Block::new(64, 1, false).size(), 64);
    assert_eq!(Block::new(1024, 1, false).size(), 1024);
    assert_eq!(Block::new(2048, 1, false).size(), 1024);
  }
}
