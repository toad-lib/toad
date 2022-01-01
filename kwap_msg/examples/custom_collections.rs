use collection_heapless_vec::HeaplessVec;
use kwap_common::{Array, GetSize, Reserve};
use kwap_msg::*;

fn main() {
  type StackMsg = Message<HeaplessVec<u8, 16>, HeaplessVec<u8, 32>, HeaplessVec<Opt<HeaplessVec<u8, 32>>, 1>>;
  let stack_msg = StackMsg { code: Code { class: 2, detail: 5 },
                             ty: Type(0),
                             ver: Default::default(),
                             id: Id(0),
                             opts: Default::default(),
                             payload: Payload(Default::default()),
                             token: Token(Default::default()),
                             __optc: Default::default() };
  let bytes = stack_msg.try_into_bytes::<HeaplessVec<u8, 128>>().unwrap();
  let _roundtrip = StackMsg::try_from_bytes(bytes).unwrap();
}

pub(crate) mod collection_heapless_vec {
  use std::{ops::{Index, IndexMut},
            ptr};

  use kwap_common::Insert;

  use super::*;
  #[derive(Default)]
  pub struct HeaplessVec<T: Default, const N: usize>(heapless::Vec<T, N>);
  impl<T: Default, const N: usize> Array<T> for HeaplessVec<T, N> {}

  impl<T: Default, const N: usize> Insert<T> for HeaplessVec<T, N> {
    fn insert_at(&mut self, index: usize, value: T) {
      if index == self.0.len() {
        self.push(value);
        return;
      }

      let me: Self = unsafe { ptr::read(self as *const Self) };

      let mut value_container = Some(value);

      let buffer = me.into_iter()
                     .enumerate()
                     .fold(heapless::Vec::<T, N>::new(), |mut buf, (ix, val)| {
                       if ix == index {
                         buf.push(value_container.take().unwrap()).ok();
                       } else {
                         buf.push(val).ok();
                       }

                       buf
                     });

      *self = Self(buffer);
    }
  }

  impl<T: Default, const N: usize> Index<usize> for HeaplessVec<T, N> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
      &self.0[index]
    }
  }

  impl<T: Default, const N: usize> IndexMut<usize> for HeaplessVec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
      &mut self.0[index]
    }
  }

  impl<T: Default, const N: usize> Reserve for HeaplessVec<T, N> {}
  impl<T: Default, const N: usize> GetSize for HeaplessVec<T, N> {
    fn get_size(&self) -> usize {
      self.0.len()
    }
    fn max_size(&self) -> Option<usize> {
      Some(N)
    }
  }
  impl<T: Default, const N: usize> Extend<T> for HeaplessVec<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
      self.0.extend(iter);
    }
  }

  impl<T: Default, const N: usize> FromIterator<T> for HeaplessVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
      Self(heapless::Vec::from_iter(iter))
    }
  }
  impl<T: Default, const N: usize> IntoIterator for HeaplessVec<T, N> {
    type Item = T;
    type IntoIter = <heapless::Vec<T, N> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
      self.0.into_iter()
    }
  }
  impl<'a, T: Default, const N: usize> IntoIterator for &'a HeaplessVec<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
      self.0.iter()
    }
  }
}
