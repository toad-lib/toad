use toad_common::Array;
use toad_msg::Message;

use super::{exec_inner_step, Step, StepOutput};
use crate::net::Addrd;
use crate::platform::{self, Effect, Platform, Snapshot};
use crate::req::Req;
use crate::resp::Resp;
use crate::{time, todo};

/// The message parsing CoAP lifecycle step
///
/// Parameterized by the step that came before it,
/// most likely this is the [`Empty`](crate::step::Empty) step.
///
/// See the [module documentation](crate::step::parse) for more.
#[derive(Default, Debug, Clone, Copy)]
pub struct Parse<S>(S);

impl<S> Parse<S> {
  /// Create a new Parse step
  pub fn new(s: S) -> Self {
    Self(s)
  }
}

/// Errors that can occur during this step
#[derive(Clone, PartialEq)]
pub enum Error<E> {
  /// Datagram failed to parse as a CoAP message
  Parsing(toad_msg::MessageParseError),
  /// The inner step failed.
  ///
  /// This variant's Debug representation is completely
  /// replaced by the inner type E's debug representation
  ///
  /// ```
  /// use toad::step::parse::Error;
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
      | Self::Parsing(e) => f.debug_tuple("Parsing").field(e).finish(),
      | Self::Inner(e) => e.fmt(f),
    }
  }
}

impl<E: super::Error> super::Error for Error<E> {}

macro_rules! common {
  ($dgram:expr) => {{
    use toad_msg::TryFromBytes;
    $dgram.fold(|dgram, addr| Message::try_from_bytes(dgram).map(|dgram| Addrd(dgram, addr)))
          .map_err(Error::Parsing)
          .map_err(nb::Error::Other)
  }};
}

impl<Dgram,
      Inner,
      E,
      Effects,
      MessagePayload,
      MessageOptionValue,
      MessageOptions,
      NumberedOptions,
      Clock>
  Step<Dgram, Effects, MessagePayload, MessageOptionValue, MessageOptions, NumberedOptions, Clock>
  for Parse<Inner>
  where Dgram: crate::net::Dgram,
        E: super::Error,
        Inner: Step<Dgram,
                    Effects,
                    MessagePayload,
                    MessageOptionValue,
                    MessageOptions,
                    NumberedOptions,
                    Clock,
                    Error = E>,
        Effects: Array<Item = Effect<MessagePayload, MessageOptions>>,
        MessagePayload: todo::MessagePayload,
        MessageOptionValue: todo::MessageOptionValue,
        MessageOptions: todo::MessageOptions<MessageOptionValue>,
        NumberedOptions: todo::NumberedOptions<MessageOptionValue>,
        Clock: time::Clock
{
  type PollReq = Addrd<Req<MessagePayload, MessageOptionValue, MessageOptions, NumberedOptions>>;
  type PollResp = Addrd<Resp<MessagePayload, MessageOptionValue, MessageOptions, NumberedOptions>>;
  type Error = Error<Inner::Error>;
  type Inner = Inner;

  fn inner(&mut self) -> &mut Self::Inner {
    &mut self.0
  }

  fn poll_req(&mut self,
              snap: &Snapshot<Dgram, Clock>,
              effects: &mut Effects)
              -> StepOutput<Self::PollReq, Self::Error> {
    exec_inner_step!(self.0.poll_req(snap, effects), Error::Inner);
    Some(common!(snap.recvd_dgram.as_ref()).map(|addrd| addrd.map(Req::from)))
  }

  fn poll_resp(&mut self,
               snap: &Snapshot<Dgram, Clock>,
               effects: &mut Effects,
               token: toad_msg::Token,
               addr: no_std_net::SocketAddr)
               -> StepOutput<Self::PollResp, Self::Error> {
    exec_inner_step!(self.0.poll_resp(snap, effects, token, addr), Error::Inner);
    Some(common!(snap.recvd_dgram.as_ref()).map(|addrd| addrd.map(Resp::from)))
  }
}

#[cfg(test)]
mod test {
  use embedded_time::Clock;
  use toad_msg::{Code, Type};

  use super::super::test;
  use super::{Error, Parse, Step};
  use crate::net::Addrd;
  use crate::platform;
  use crate::req::{Req, ReqForPlatform};
  use crate::resp::{Resp, RespForPlatform};

  fn test_msg(
    ty: Type,
    code: Code)
    -> (Addrd<Vec<u8>>,
        Addrd<ReqForPlatform<crate::test::Platform>>,
        Addrd<RespForPlatform<crate::test::Platform>>) {
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

    (Addrd(msg.clone().try_into_bytes().unwrap(), addr),
     Addrd(Req::<_, _, _, _>::from(msg.clone()), addr),
     Addrd(Resp::<_, _, _, _>::from(msg), addr))
  }

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN inner_errors [
          (inner.poll_req => { Some(Err(nb::Error::Other(()))) }),
          (inner.poll_resp => { Some(Err(nb::Error::Other(()))) })
        ]
      THEN this_should_error
        [
          (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(())))) )}),
          (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::Other(Error::Inner(()))))) })
        ]
  );

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN inner_would_block [
        (inner.poll_req => { Some(Err(nb::Error::WouldBlock)) }),
        (inner.poll_resp => { Some(Err(nb::Error::WouldBlock)) })
      ]
      THEN this_should_block [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) }),
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Err(nb::Error::WouldBlock))) })
      ]
  );

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN con_request_recvd [
        (inner.poll_req => {None}),
        (snapshot = {
          platform::Snapshot {
            time: crate::test::ClockMock::new().try_now().unwrap(),
            recvd_dgram: test_msg(Type::Con, Code::new(1, 01)).0,
            config: Default::default(),
          }
        })
      ]
      THEN poll_req_should_parse_it [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out,Some(Ok(test_msg(Type::Con, Code::new(1, 01)).1)))})
      ]
  );

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN empty_ack_recvd [
        (inner.poll_req => {None}),
        (snapshot = {
          platform::Snapshot {
            time: crate::test::ClockMock::new().try_now().unwrap(),
            recvd_dgram: test_msg(Type::Ack, Code::new(0, 0)).0,
            config: Default::default(),
          }
        })
      ]
      THEN poll_req_should_parse_it [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Ack, Code::new(0, 0)).1))) })
      ]
  );

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN piggy_ack_recvd [
        (inner.poll_req => {None}),
        (snapshot = {
          platform::Snapshot {
            time: crate::test::ClockMock::new().try_now().unwrap(),
            recvd_dgram: test_msg(Type::Ack, Code::new(2, 04)).0,
            config: Default::default(),
          }
        })
      ]
      THEN poll_req_should_parse_it [
        (poll_req(_, _) should satisfy { |out| assert_eq!(out,Some(Ok(test_msg(Type::Ack, Code::new(2, 04)).1))) })
      ]
  );

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN recvd_ack [
          (inner.poll_resp => {None}),
          (snapshot = {
            platform::Snapshot {
              time: crate::test::ClockMock::new().try_now().unwrap(),
              recvd_dgram: test_msg(Type::Ack, Code::new(2, 04)).0,
              config: Default::default(),
            }
          })
        ]
      THEN poll_resp_should_parse_it [
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Ack, Code::new(2, 04)).2))) })
      ]
  );

  test::test_step!(
      GIVEN Parse::<Dummy> where Dummy: {Step<PollReq = (), PollResp = (), Error = ()>};
      WHEN request_recvd [
        (inner.poll_resp => {None}),
        (snapshot = {
          platform::Snapshot {
           time: crate::test::ClockMock::new().try_now().unwrap(),
           recvd_dgram: test_msg(Type::Con, Code::new(1, 1)).0,
           config: Default::default(),
          }
        })
      ]
      THEN poll_resp_should_parse_it [
        (poll_resp(_, _, _, _) should satisfy { |out| assert_eq!(out, Some(Ok(test_msg(Type::Con, Code::new(1, 1)).2))) })
      ]
  );
}
