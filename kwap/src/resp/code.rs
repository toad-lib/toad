use kwap_macros::rfc_7252_doc;
pub use kwap_msg::Code;
use crate::code;

// 2.xx
code!(rfc7252("5.9.1.1") CREATED = 2 . 01);
code!(rfc7252("5.9.1.2") DELETED = 2 . 02);
code!(rfc7252("5.9.1.3") VALID   = 2 . 03);
code!(rfc7252("5.9.1.4") CHANGED = 2 . 04);
code!(rfc7252("5.9.1.5") CONTENT = 2 . 05);

// 4.xx
code!(rfc7252("5.9.2.1")  BAD_REQUEST                = 4 . 00);
code!(rfc7252("5.9.2.2")  UNAUTHORIZED               = 4 . 01);
code!(rfc7252("5.9.2.3")  BAD_OPTION                 = 4 . 02);
code!(rfc7252("5.9.2.4")  FORBIDDEN                  = 4 . 03);
code!(rfc7252("5.9.2.5")  NOT_FOUND                  = 4 . 04);
code!(rfc7252("5.9.2.6")  METHOD_NOT_ALLOWED         = 4 . 05);
code!(rfc7252("5.9.2.7")  NOT_ACCEPTABLE             = 4 . 06);
code!(rfc7252("5.9.2.8")  PRECONDITION_FAILED        = 4 . 12);
code!(rfc7252("5.9.2.9")  REQUEST_ENTITY_TOO_LARGE   = 4 . 13);
code!(rfc7252("5.9.2.10") UNSUPPORTED_CONTENT_FORMAT = 4 . 15);

// 5.xx
code!(rfc7252("5.9.3.1") INTERNAL_SERVER_ERROR  =  5 . 00);
code!(rfc7252("5.9.3.2") NOT_IMPLEMENTED        =  5 . 01);
code!(rfc7252("5.9.3.3") BAD_GATEWAY            =  5 . 02);
code!(rfc7252("5.9.3.4") SERVICE_UNAVAILABLE    =  5 . 03);
code!(rfc7252("5.9.3.5") GATEWAY_TIMEOUT        =  5 . 04);
code!(rfc7252("5.9.3.6") PROXYING_NOT_SUPPORTED =  5 . 05);
