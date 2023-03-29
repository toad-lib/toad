pub use toad_msg::Code;

use crate::code;

// 2.xx
code!(rfc7252("5.9.1.1") CREATED = 2*01);
code!(rfc7252("5.9.1.2") DELETED = 2*02);
code!(rfc7252("5.9.1.3") VALID   = 2*03);
code!(rfc7252("5.9.1.4") CHANGED = 2*04);
code!(rfc7252("5.9.1.5") CONTENT = 2*05);
code!(
      #[doc = concat!(
    "## [2.31 Continue](https://www.rfc-editor.org/rfc/rfc7959#section-2.9.1)\n",
    "This success status code indicates that the transfer of this\n",
    "block of the request body was successful and that the server\n",
    "encourages sending further blocks, but that a final outcome of the\n",
    "whole block-wise request cannot yet be determined.  No payload is\n",
    "returned with this response code.",
  )]
      CONTINUE = 2 * 31
);

// 4.xx
code!(rfc7252("5.9.2.1")  BAD_REQUEST                = 4*00);
code!(rfc7252("5.9.2.2")  UNAUTHORIZED               = 4*01);
code!(rfc7252("5.9.2.3")  BAD_OPTION                 = 4*02);
code!(rfc7252("5.9.2.4")  FORBIDDEN                  = 4*03);
code!(rfc7252("5.9.2.5")  NOT_FOUND                  = 4*04);
code!(rfc7252("5.9.2.6")  METHOD_NOT_ALLOWED         = 4*05);
code!(rfc7252("5.9.2.7")  NOT_ACCEPTABLE             = 4*06);
code!(
      #[doc = concat!(
    "## [4.08 Request Entity Incomplete](https://www.rfc-editor.org/rfc/rfc7959#section-2.9.2)\n",
    "This client error status code indicates that the server has not\n",
    "received the blocks of the request body that it needs to proceed.\n",
    "The client has not sent all blocks, not sent them in the order\n",
    "required by the server, or has sent them long enough ago that the\n",
    "server has already discarded them.",
  )]
      REQUEST_ENTITY_INCOMPLETE = 4 * 08
);
code!(rfc7252("5.9.2.8")  PRECONDITION_FAILED        = 4*12);
code!(rfc7252("5.9.2.9")  REQUEST_ENTITY_TOO_LARGE   = 4*13);
code!(rfc7252("5.9.2.10") UNSUPPORTED_CONTENT_FORMAT = 4*15);

// 5.xx
code!(rfc7252("5.9.3.1") INTERNAL_SERVER_ERROR  =  5*00);
code!(rfc7252("5.9.3.2") NOT_IMPLEMENTED        =  5*01);
code!(rfc7252("5.9.3.3") BAD_GATEWAY            =  5*02);
code!(rfc7252("5.9.3.4") SERVICE_UNAVAILABLE    =  5*03);
code!(rfc7252("5.9.3.5") GATEWAY_TIMEOUT        =  5*04);
code!(rfc7252("5.9.3.6") PROXYING_NOT_SUPPORTED =  5*05);
