The Option number identifies which Option is being set (e.g. Content-Format has a Number of 12)

Because Option Numbers are only able to be computed in the context of other options, in order to
get Option Numbers you must have a collection of [`Opt`]s.

Then you can use the provided [`EnumerateOptNumbers`] iterator extension to enumerate over options
with their numbers.

<details>
<summary>Click to see table of Option Numbers defined in the original CoAP RFC</summary>

```text
+--------+------------------+-----------+
| Number | Name             | Reference |
+--------+------------------+-----------+
|      0 | (Reserved)       | [RFC7252] |
|      1 | If-Match         | [RFC7252] |
|      3 | Uri-Host         | [RFC7252] |
|      4 | ETag             | [RFC7252] |
|      5 | If-None-Match    | [RFC7252] |
|      7 | Uri-Port         | [RFC7252] |
|      8 | Location-Path    | [RFC7252] |
|     11 | Uri-Path         | [RFC7252] |
|     12 | Content-Format   | [RFC7252] |
|     14 | Max-Age          | [RFC7252] |
|     15 | Uri-Query        | [RFC7252] |
|     17 | Accept           | [RFC7252] |
|     20 | Location-Query   | [RFC7252] |
|     35 | Proxy-Uri        | [RFC7252] |
|     39 | Proxy-Scheme     | [RFC7252] |
|     60 | Size1            | [RFC7252] |
|    128 | (Reserved)       | [RFC7252] |
|    132 | (Reserved)       | [RFC7252] |
|    136 | (Reserved)       | [RFC7252] |
|    140 | (Reserved)       | [RFC7252] |
+--------+------------------+-----------+
```
</details>

# Related
- [RFC7252#section-3.1 Option Format](https://datatracker.ietf.org/doc/html/rfc7252#section-3.1)
- [RFC7252#section-5.4.6 Option Numbers](https://datatracker.ietf.org/doc/html/rfc7252#section-5.4.6)
