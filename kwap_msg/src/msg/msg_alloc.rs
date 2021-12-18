use super::*;

#[doc = msg_docs!()]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Message {
  /// see [`Id`] for details
  pub id: Id,
  /// see [`Type`] for details
  pub ty: Type,
  /// see [`Version`] for details
  pub ver: Version,
  /// see [`TokenLength`] for details
  pub tkl: TokenLength,
  /// see [`Token`] for details
  pub token: Token,
  /// see [`Code`] for details
  pub code: Code,
  /// see [`opt::Opt`] for details
  pub opts: Vec<opt::Opt>,
  /// See [`Payload`]
  pub payload: Payload,
}

fn try_next<I>(iter: &mut impl Iterator<Item = I>) -> Result<I, MessageParseError> {
  iter.next().ok_or(MessageParseError::UnexpectedEndOfStream)
}

impl TryFromBytes for Message {
  type Error = MessageParseError;

  fn try_from_bytes<T: IntoIterator<Item = u8>>(bytes: T) -> Result<Self, Self::Error> {
    let mut bytes = bytes.into_iter();

    let Byte1 {tkl, ty, ver} = try_next(&mut bytes)?.into();
    let code: Code = try_next(&mut bytes)?.into();
    let id: Id = Id::try_consume_bytes(&mut bytes)?;
    let token = Token::try_consume_bytes(bytes.by_ref().take(tkl.0 as usize))?;
    let opts = Vec::<opt_alloc::Opt>::try_consume_bytes(&mut bytes).map_err(MessageParseError::OptParseError)?;
    let payload = Payload(bytes.collect());

    Ok(Message {tkl, id, ty, ver, code, token, opts, payload})
  }
}
