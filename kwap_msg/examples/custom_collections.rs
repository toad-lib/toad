use collection_heapless_vec::HeaplessVec;
use kwap_common::{Array, GetSize, Reserve};
use kwap_msg::*;

fn main() {
  type StackMsg = Message<HeaplessVec<u8, 1>, HeaplessVec<u8, 1>, HeaplessVec<Opt<HeaplessVec<u8, 1>>, 1>>;
  let stack_msg = StackMsg { code: Code { class: 2, detail: 5 },
                             ty: Type::Con,
                             ver: Default::default(),
                             id: Id(0),
                             opts: Default::default(),
                             payload: Payload(Default::default()),
                             token: Token(Default::default()) };
  println!("created {}b message using heapless::Vec", stack_msg.get_size());

  let bytes = stack_msg.clone().try_into_bytes::<HeaplessVec<u8, 5>>().unwrap();
  println!("message -> bytes success!");

  let roundtrip = StackMsg::try_from_bytes(bytes).unwrap();
  println!("bytes -> message success!");

  assert_eq!(roundtrip, stack_msg);
}

pub(crate) mod collection_heapless_vec {
  use std::ops::{Deref, DerefMut, Index, IndexMut};
  use std::ptr;

  use kwap_common::Insert;

  use super::*;
  #[derive(Debug, Default, PartialEq, Clone)]
  pub struct HeaplessVec<T: Default, const N: usize>(heapless::Vec<T, N>);
  impl<T: Default, const N: usize> Array for HeaplessVec<T, N> {
    type Item = T;
  }

  impl<T: Default, const N: usize> Deref for HeaplessVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
      &self.0
    }
  }
  impl<T: Default, const N: usize> DerefMut for HeaplessVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
      &mut self.0
    }
  }

  impl<T: Default, const N: usize> Insert<T> for HeaplessVec<T, N> {
    // we can use the default implementation of Insert::push because `insert` invokes push for us

    fn insert_at(&mut self, index: usize, value: T) {
      if index == self.0.len() {
        self.push(value);
        return;
      }

      // please do NOT use this code,
      // this is a terrible implementation of `insert_at` just for demonstration purposes.
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
