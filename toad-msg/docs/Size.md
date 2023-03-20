In many cases when transferring a large resource representation block
by block, it is advantageous to know the total size early in the
process. Some indication may be available from the maximum size
estimate attribute "sz" provided in a resource description [RFC6690].
However, the size may vary dynamically, so a more up-to-date
indication may be useful.

This specification defines two CoAP options, Size1 for indicating the
size of the representation transferred in requests, and Size2 for
indicating the size of the representation transferred in responses.
(Size1 has already been defined in [Section 5.10.9 of RFC7252] to
provide "size information about the resource representation in a
request"; however, that section only details the narrow case of
indicating in 4.13 responses the maximum size of request payload that
the server is able and willing to handle. The present specification
provides details about its use as a request option as well.)

The Size2 Option may be used for two purposes:
* In a request, to ask the server to provide a size estimate along with the usual response ("size request"). For this usage, the value MUST be set to 0.
* In a response carrying a Block2 Option, to indicate the current estimate the server has of the total size of the resource representation, measured in bytes ("size indication").

Similarly, the Size1 Option may be used for two purposes:
* In a request carrying a Block1 Option, to indicate the current estimate the client has of the total size of the resource representation, measured in bytes ("size indication").
* In a 4.13 response, to indicate the maximum size that would have been acceptable [RFC7252], measured in bytes.

Apart from conveying/asking for size information, the Size options
have no other effect on the processing of the request or response.
If the client wants to minimize the size of the payload in the
resulting response, it should add a Block2 Option to the request with
a small block size (e.g., setting SZX=0).

The Size options are "elective", i.e., a client MUST be prepared for
the server to ignore the size estimate request. Either Size option
MUST NOT occur more than once in a single message.

[RFC6690]: https://www.rfc-editor.org/rfc/rfc6690.html
[RFC7252]: https://www.rfc-editor.org/rfc/rfc7252.html
[Section 5.10.9 of RFC7252]: https://www.rfc-editor.org/rfc/rfc7252.html#section-5.10.9
