Low-level representation of a freshly parsed CoAP Option

Both requests and responses may include a list of one or more
options. For example, the URI in a request is transported in several
options, and metadata that would be carried in an HTTP header in HTTP
is supplied as options as well.

## Option Numbers
This struct just stores data parsed directly from the message on the wire,
and does not compute or store the Option Number.

To get Option [`OptNumber`]s, you can use the iterator extension [`EnumerateOptNumbers`] on a collection of [`Opt`]s.

## `alloc` / no-`alloc`
When crate feature `alloc` is enabled, you can use [`opt_alloc::Opt`] or just `opt::Opt`, which uses heap allocation
for data storage.

When `alloc` is disabled, you must use [`opt_fixed::Opt`] or just `opt::Opt`, which instead has a fixed capacity and
uses stack allocation for data storage.

# Related
- [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
- [RFC7252#section-5.4 Options](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4)
