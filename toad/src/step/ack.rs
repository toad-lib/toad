use toad_common::Array;
use toad_msg::to_bytes::MessageToBytesError;
use toad_msg::{CodeKind, TryIntoBytes, Type};

use super::{exec_inner_step, Step, StepOutput};
use crate::net::Addrd;
use crate::platform::{Effect, Platform};
use crate::req::Req;
use crate::resp::Resp;

/// The message parsing CoAP lifecycle step
///
/// Parameterized by the step that came before it,
/// most likely this is the [`Empty`](crate::step::Empty) step.
///
/// See the [module documentation](crate::step::parse) for more.
#[derive(Debug, Clone, Copy)]
pub struct Ack<S>(S);

impl<S: Default> Default for Ack<S> {
  fn default() -> Self {
    Ack(Default::default())
  }
}

impl<S> Ack<S> {
  /// Create a new Ack step
  pub fn new(s: S) -> Self {
    Self(s)
  }
}

type InnerPollReq<P> = Addrd<Req<P>>;
type InnerPollResp<P> = Addrd<Resp<P>>;

/// Errors that can occur during this step
#[derive(Clone, PartialEq)]
pub enum Error<E> {
  /// Error serializing outbound ACK
  SerializingAck(MessageToBytesError),
  /// The inner step failed.
  ///
  /// This variant's Debug representation is completely
  /// replaced by the inner type E's debug representation
  ///
  /// ```
  /// use toad::step::ack::Error;
  ///
  /// #[derive(Debug)]
  /// struct Foo;
  ///
  /// let foo = Foo;
  /// let foo_error = Error::<Foo>::Inner(Foo);
  ///
  /// assert_eq!(format!("{foo:?}"), format!("{foo_error:?}"));
  /// ```
  Inner(E),
}

impl<E> From<E> for Error<E> {
  fn from(e: E) -> Self {
    Error::Inner(e)
  }
}

impl<E: core::fmt::Debug> core::fmt::Debug for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | Self::SerializingAck(e) => f.debug_tuple("SerializingAck").field(e).finish(),
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E: super::Error> super::Error for Error<E> {}

impl<Inner: Step<P, PollReq = InnerPollReq<P>, PollResp = InnerPollResp<P>>, P: Platform> Step<P>
  for Ack<Inner>
{
  type PollReq = Addrd<Req<P>>;
  type PollResp = Addrd<Resp<P>>;
  type Error = Error<Inner::Error>;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Inner {
    &mut self.0
  }

  fn poll_req(&mut self,
              snap: &crate::platform::Snapshot<P>,
              effects: &mut <P as Platform>::Effects)
              -> StepOutput<Self::PollReq, Error<Inner::Error>> {
    match exec_inner_step!(self.0.poll_req(snap, effects), Error::Inner) {
      | Some(req)
        if req.data().msg.ty == Type::Con && req.data().msg.code.kind() == CodeKind::Request =>
      {
        match Resp::ack(req.as_ref().data()).try_into_bytes() {
          | Ok(bytes) => {
            effects.push(Effect::SendDgram(Addrd(bytes, req.addr())));
            Some(Ok(req))
          },
          | Err(e) => Some(Err(nb::Error::Other(Error::SerializingAck(e)))),
        }
      },
      | Some(req) => Some(Ok(req)),
      | None => None,
    }
  }

  fn poll_resp(&mut self,
               snap: &crate::platform::Snapshot<P>,
               effects: &mut <P as Platform>::Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Error<Inner::Error>> {
    exec_inner_step!(self.0.poll_resp(snap, effects, token, addr), Error::Inner).map(Ok)
  }
}

#[cfg(test)]
mod test {
  use toad_msg::{Code, Type};

  use super::super::test;
  use super::{Ack, Effect, Error, Step, TryIntoBytes};
  use crate::net::Addrd;
  use crate::platform;
  use crate::req::Req;
  use crate::resp::Resp;

  type InnerPollReq = super::InnerPollReq<crate::test::Platform>;
  type InnerPollResp = super::InnerPollResp<crate::test::Platform>;

  fn test_msg(ty: Type,
              code: Code)
              -> (Addrd<Req<crate::test::Platform>>, Addrd<Resp<crate::test::Platform>>) {
    use toad_msg::*;

    type Msg = platform::Message<crate::test::Platform>;
    let msg = Msg { id: Id(1),
                    ty,
                    ver: Default::default(),
                    token: Token(Default::default()),
                    code,
                    opts: Default::default(),
                    payload: Payload(Default::default()) };

    let addr = crate::test::dummy_addr();

    (Addrd(Req::<_>::from(msg.clone()), addr), Addrd(Resp::<_>::from(msg), addr))
  }

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_errors [
        (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
        (inner.poll_resp => { Some(Err(nb::Error::Other(()))) })
      ]
      THEN this_should_error [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) }),
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_blocks [
        (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
        (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
      ]
      THEN this_should_block [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_non_request [
        (inner.poll_req => { Some(Ok(test_msg(Type::Non, Code::new(1, 01)).0)) })
      ]
      THEN poll_req_should_noop [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Non, Code::new(1, 01)).0))) }),
        (effects == { vec![] })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_response [
        (inner.poll_req => { Some(Ok(test_msg(Type::Ack, Code::new(0, 00)).0)) })
      ]
      THEN poll_req_should_noop [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg( Type::Ack, Code::new(0, 00)).0))) }),
        (effects == { vec![] })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_con_request [
        (inner.poll_req => { Some(Ok(test_msg(Type::Con, Code::new(1, 01)).0)) })
      ]
      THEN poll_req_should_ack [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Con, Code::new(1, 01)).0))) }),
        (effects == {
          vec![
            Effect::SendDgram(
              Addrd(
                Resp::ack(&test_msg(Type::Con, Code::new(1, 01)).0.0)
                  .try_into_bytes()
                  .unwrap(),
                crate::test::dummy_addr()
              )
            )
          ]
        })
      ]
  );

  test::test_step!(
      GIVEN Ack::<Dummy> where Dummy: {Step<PollReq = InnerPollReq, PollResp = InnerPollResp, Error = ()>};
      WHEN inner_yields_anything [
        (inner.poll_resp => { Some(Ok(test_msg(Type::Ack, Code::new(2, 04)).1)) })
      ]
      THEN poll_resp_should_noop [
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Ack, Code::new(2, 04)).1))) }),
        (effects == { vec![] })
      ]
  );
}
