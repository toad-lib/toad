# Using the Block1 Option
In a request with a request payload (e.g., PUT or POST), the Block1
Option refers to the payload in the request (descriptive usage).

In response to a request with a payload (e.g., a PUT or POST
transfer), the block size given in the Block1 Option indicates the
block size preference of the server for this resource (control
usage).  Obviously, at this point the first block has already been
transferred by the client without benefit of this knowledge.  Still,
the client SHOULD heed the preference indicated and, for all further
blocks, use the block size preferred by the server or a smaller one.
Note that any reduction in the block size may mean that the second
request starts with a block number larger than one, as the first
request already transferred multiple blocks as counted in the smaller
size.

To counter the effects of adaptation-layer fragmentation on packet-
delivery probability, a client may want to give up retransmitting a
request with a relatively large payload even before MAX_RETRANSMIT
has been reached, and try restating the request as a block-wise
transfer with a smaller payload.  Note that this new attempt is then
a new message-layer transaction and requires a new Message ID.
(Because of the uncertainty about whether the request or the
acknowledgement was lost, this strategy is useful mostly for
idempotent requests.)

In a block-wise transfer of a request payload (e.g., a PUT or POST)
that is intended to be implemented in an atomic fashion at the
server, the actual creation/replacement takes place at the time the
final block, i.e., a block with the M bit unset in the Block1 Option,
is received.  In this case, all success responses to non-final blocks
carry the response code 2.31 (Continue, Section 2.9.1).  If not all
previous blocks are available at the server at the time of processing
the final block, the transfer fails and error code 4.08 (Request
Entity Incomplete, Section 2.9.2) MUST be returned.  A server MAY
also return a 4.08 error code for any (final or non-final) Block1
transfer that is not in sequence; therefore, clients that do not have
specific mechanisms to handle this case SHOULD always start with
block zero and send the following blocks in order.

One reason that a client might encounter a 4.08 error code is that
the server has already timed out and discarded the partial request
body being assembled.  Clients SHOULD strive to send all blocks of a
request without undue delay.  Servers can fully expect to be free to
discard any partial request body when a period of EXCHANGE_LIFETIME
([RFC7252], Section 4.8.2) has elapsed after the most recent block
was transferred; however, there is no requirement on a server to
always keep the partial request body for as long.

The error code 4.13 (Request Entity Too Large) can be returned at any
time by a server that does not currently have the resources to store
blocks for a block-wise request payload transfer that it would intend
to implement in an atomic fashion.  (Note that a 4.13 response to a
request that does not employ Block1 is a hint for the client to try
sending Block1, and a 4.13 response with a smaller SZX in its Block1
Option than requested is a hint to try a smaller SZX.)

A block-wise transfer of a request payload that is implemented in a
stateless fashion at the server is likely to leave the resource being
operated on in an inconsistent state while the transfer is still
ongoing or when the client does not complete the transfer.  This
characteristic is closer to that of remote file systems than to that
of HTTP, where state is always kept on the server during a transfer.
Techniques well known from shared file access (e.g., client-specific
temporary resources) can be used to mitigate this difference from
HTTP.

The Block1 Option provides no way for a single endpoint to perform
multiple concurrently proceeding block-wise request payload transfer
(e.g., PUT or POST) operations to the same resource.  Starting a new
block-wise sequence of requests to the same resource (before an old
sequence from the same endpoint was finished) simply overwrites the
context the server may still be keeping.  (This is probably exactly
what one wants in this case -- the client may simply have restarted
and lost its knowledge of the previous sequence.)
