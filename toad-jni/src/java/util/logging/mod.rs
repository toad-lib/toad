mod logger;
#[doc(inline)]
pub use logger::Logger;

mod level;
#[doc(inline)]
pub use level::Level;

mod handler;
#[doc(inline)]
pub use handler::{ConsoleHandler, Handler};
