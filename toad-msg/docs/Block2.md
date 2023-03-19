# Using the Block2 Option
When a request is answered with a response carrying a Block2 Option
with the M bit set, the requester may retrieve additional blocks of
the resource representation by sending further requests with the same
options as the initial request and a Block2 Option giving the block
number and block size desired.  In a request, the client MUST set the
M bit of a Block2 Option to zero and the server MUST ignore it on
reception.

To influence the block size used in a response, the requester MAY
also use the Block2 Option on the initial request, giving the desired
size, a block number of zero and an M bit of zero.  A server MUST use
the block size indicated or a smaller size.  Any further block-wise
requests for blocks beyond the first one MUST indicate the same block
size that was used by the server in the response for the first
request that gave a desired size using a Block2 Option.

Once the Block2 Option is used by the requester and a first response
has been received with a possibly adjusted block size, all further
requests in a single block-wise transfer will ultimately converge on
using the same size, except that there may not be enough content to
fill the last block (the one returned with the M bit not set).  (Note
that the client may start using the Block2 Option in a second request
after a first request without a Block2 Option resulted in a Block2
Option in the response.)  The server uses the block size indicated in
the request option or a smaller size, but the requester MUST take
note of the actual block size used in the response it receives to its
initial request and proceed to use it in subsequent requests.  The
server behavior MUST ensure that this client behavior results in the
same block size for all responses in a sequence (except for the last
one with the M bit not set, and possibly the first one if the initial
request did not contain a Block2 Option).

Block-wise transfers can be used to GET resources whose
representations are entirely static (not changing over time at all,
such as in a schema describing a device), or for dynamically changing
resources.  In the latter case, the Block2 Option SHOULD be used in
conjunction with the ETag Option ([RFC7252], Section 5.10.6), to
ensure that the blocks being reassembled are from the same version of
the representation: The server SHOULD include an ETag Option in each
response.  If an ETag Option is available, the client, when
reassembling the representation from the blocks being exchanged, MUST
compare ETag Options.  If the ETag Options do not match in a GET
transfer, the requester has the option of attempting to retrieve
fresh values for the blocks it retrieved first.  To minimize the
resulting inefficiency, the server MAY cache the current value of a
representation for an ongoing sequence of requests.  (The server may
identify the sequence by the combination of the requesting endpoint
and the URI being the same in each block-wise request.)  Note well
that this specification makes no requirement for the server to
establish any state; however, servers that offer quickly changing
resources may thereby make it impossible for a client to ever
retrieve a consistent set of blocks.  Clients that want to retrieve
all blocks of a resource SHOULD strive to do so without undue delay.
Servers can fully expect to be free to discard any cached state after
a period of EXCHANGE_LIFETIME ([RFC7252], Section 4.8.2) after the
last access to the state, however, there is no requirement to always
keep the state for as long.

The Block2 Option provides no way for a single endpoint to perform
multiple concurrently proceeding block-wise response payload transfer
(e.g., GET) operations to the same resource.  This is rarely a
requirement, but as a workaround, a client may vary the cache key
(e.g., by using one of several URIs accessing resources with the same
semantics, or by varying a proxy-safe elective option).
