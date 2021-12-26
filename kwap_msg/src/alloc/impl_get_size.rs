use super::*;
use crate::get_size::*;

impl GetSize for Message {
  fn get_size(&self) -> usize {
    let header_size = 4;
    let payload_marker_size = 1;
    let payload_size = self.payload.0.len();
    let token_size = self.token.0.len();
    let opts_size: usize = self.opts.iter().map(|o| o.get_size()).sum();

    header_size + payload_marker_size + payload_size + token_size + opts_size
  }
}

impl GetSize for Opt {
  fn get_size(&self) -> usize {
    let header_size = 1;
    let delta_size = match self.delta.0 {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    let value_len_size = match self.value.0.len() {
      | n if n >= 269 => 2,
      | n if n >= 13 => 1,
      | _ => 0,
    };

    header_size + delta_size + value_len_size + self.value.0.len()
  }
}
