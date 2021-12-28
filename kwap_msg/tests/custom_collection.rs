use kwap_msg::*;
use std::fmt::Debug;

#[test]
fn msg_works() {
  type Msg = Message<List<u8>, List<u8>, List<Opt<List<u8>>>>;
  let msg = Msg {
    code: Code {class: 2, detail: 5},
    ty: Type(0),
    ver: Default::default(),
    id: Id(0),
    opts: List::Nil,
    payload: Payload(List::Nil),
    token: Token(Default::default())
    , __optc: Default::default()
  };
  let bytes = msg.try_into_bytes::<List<u8>>().unwrap();
  let _roundtrip = Msg::try_from_bytes(bytes).unwrap();
}

#[derive(Clone, Debug)]
enum List<T: Debug + Clone> {
  Cons(T, Box<List<T>>),
  Nil,
}

impl<T: Debug + Clone> Default for List<T> {
  fn default() -> Self {List::Nil}
}

impl<T: Debug + Clone> kwap_msg::Collection<T> for List<T> {}

impl<T: Debug + Clone> kwap_msg::GetSize for List<T> {
  fn get_size(&self) -> usize {
    self.into_iter().count()
  }
}

impl<T: Debug + Clone> kwap_msg::Capacity for List<T> {}

impl<T: Debug + Clone> List<T> {
  pub fn cons(self, t: T) -> Self {
    List::Cons(t, Box::from(self))
  }
}

struct ListIntoIter<T: Debug + Clone> {
  list: Box<Option<List<T>>>,
}

struct ListIter<'a, T: Debug + Clone> {
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
    ListIter {list: self}
  }
}

impl<T: Debug + Clone> IntoIterator for List<T> {
  type Item = T;
  type IntoIter = ListIntoIter<T>;

  fn into_iter(self) -> Self::IntoIter {
    ListIntoIter {list: Box::from(Some(self))}
  }
}

impl<T: Debug + Clone> Iterator for ListIntoIter<T> {
  type Item = T;
  fn next(&mut self) -> Option<T> {
    match self.list.take() {
      Some(List::Cons(t, next)) => {
        self.list = Box::from(Some(*next));
        Some(t)
      },
      None | Some(List::Nil) => None,
    }
  }
}

impl<'a, T: Debug + Clone> Iterator for ListIter<'a, T> {
  type Item = &'a T;
  fn next(&mut self) -> Option<&'a T> {
    match self.list {
      List::Cons(t, next) => {
        self.list = &next;
        Some(t)
      },
      List::Nil => None,
    }
  }
}

impl<T: Clone + Debug> Extend<T> for List<T> {
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    *self = self.clone().into_iter().chain(iter).collect();
  }
}
