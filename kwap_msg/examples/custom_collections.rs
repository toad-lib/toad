use collection_heapless_vec::HeaplessVec;
use collection_list::List;
use kwap_msg::*;

fn main() {
  type ListMsg = Message<List<u8>, List<u8>, List<Opt<List<u8>>>>;
  let list_msg = ListMsg { code: Code { class: 2, detail: 5 },
                           ty: Type(0),
                           ver: Default::default(),
                           id: Id(0),
                           opts: List::Nil,
                           payload: Payload(List::Nil),
                           token: Token(Default::default()),
                           __optc: Default::default() };
  let bytes = list_msg.try_into_bytes::<List<u8>>().unwrap();
  let _roundtrip = ListMsg::try_from_bytes(bytes).unwrap();

  type StackMsg = Message<HeaplessVec<u8, 16>, HeaplessVec<u8, 32>, HeaplessVec<Opt<HeaplessVec<u8, 32>>, 1>>;
  let stack_msg = StackMsg { code: Code { class: 2, detail: 5 },
                             ty: Type(0),
                             ver: Default::default(),
                             id: Id(0),
                             opts: Default::default(),
                             payload: Payload(Default::default()),
                             token: Token(Default::default()),
                             __optc: Default::default() };
  let bytes = stack_msg.try_into_bytes::<List<u8>>().unwrap();
  let _roundtrip = StackMsg::try_from_bytes(bytes).unwrap();
}

pub(crate) mod collection_heapless_vec {
  use kwap_msg::*;
  #[derive(Default)]
  pub struct HeaplessVec<T: Default, const N: usize>(heapless::Vec<T, N>);
  impl<T: Default, const N: usize> kwap_msg::Collection<T> for HeaplessVec<T, N> {}
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

mod collection_list {
  use std::fmt::Debug;

  use kwap_msg::*;
  #[derive(Clone, Debug)]
  pub enum List<T: Debug + Clone> {
    Cons(T, Box<List<T>>),
    Nil,
  }

  impl<T: Debug + Clone> Default for List<T> {
    fn default() -> Self {
      List::Nil
    }
  }

  impl<T: Debug + Clone> kwap_msg::Collection<T> for List<T> {}

  impl<T: Debug + Clone> kwap_msg::GetSize for List<T> {
    fn get_size(&self) -> usize {
      self.into_iter().count()
    }

    fn max_size(&self) -> Option<usize> {
      None
    }
  }

  impl<T: Debug + Clone> kwap_msg::Reserve for List<T> {}

  impl<T: Debug + Clone> List<T> {
    pub fn cons(self, t: T) -> Self {
      List::Cons(t, Box::from(self))
    }
  }

  pub struct ListIntoIter<T: Debug + Clone> {
    list: Box<Option<List<T>>>,
  }

  pub struct ListIter<'a, T: Debug + Clone> {
    list: &'a List<T>,
  }

  impl<T: Debug + Clone> FromIterator<T> for List<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
      let mut items = iter.into_iter().collect::<Vec<_>>();
      items.reverse();

      items.into_iter().fold(List::<T>::Nil, |list, t| list.cons(t))
    }
  }

  impl<'a, T: Debug + Clone> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = ListIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
      ListIter { list: self }
    }
  }

  impl<T: Debug + Clone> IntoIterator for List<T> {
    type Item = T;
    type IntoIter = ListIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
      ListIntoIter { list: Box::from(Some(self)) }
    }
  }

  impl<T: Debug + Clone> Iterator for ListIntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
      match self.list.take() {
        | Some(List::Cons(t, next)) => {
          self.list = Box::from(Some(*next));
          Some(t)
        },
        | None | Some(List::Nil) => None,
      }
    }
  }

  impl<'a, T: Debug + Clone> Iterator for ListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
      match self.list {
        | List::Cons(t, next) => {
          self.list = &next;
          Some(t)
        },
        | List::Nil => None,
      }
    }
  }

  impl<T: Clone + Debug> Extend<T> for List<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
      *self = self.clone().into_iter().chain(iter).collect();
    }
  }
}
