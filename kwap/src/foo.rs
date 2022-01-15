use kwap::{std::Clock, retry::{Strategy, RetryTimer, Attempts}};

fn main() {
  let try_to_do_something_that_fails_once = || -> Result<(), ()> {
    // ...
  };

  let strategy = Strategy::Exponential(Milliseconds(10));
  let mut retry = RetryTimer::try_new(
                    Clock::new(),
                    strategy,
                    Attempts(2),
                  ).unwrap();

  let go = || {
    let attempt = try_to_do_something_that_fails_once();

    match attempt {
      Ok(val) => val,
      Err(_) => match retry.what_should_i_do() {
        Ok(YouShould::Retry) => go(),
        Ok(YouShould::Cry) => panic!("no more attempts! it failed more than once!!"),
        _ => unreachable!(),
	  }
    }
  };

  go();
}
