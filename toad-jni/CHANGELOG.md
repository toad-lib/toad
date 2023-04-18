# Changelog

## [0.14.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.13.0...toad-jni-v0.14.0) (2023-04-18)


### Features

* should have easy route from Result&lt;Object, Throwable&gt; -> jobject ([#318](https://github.com/toad-lib/toad/issues/318)) ([2af8532](https://github.com/toad-lib/toad/commit/2af85325da9f2b8f67f6d98c6370495559c38355))

## [0.13.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.12.0...toad-jni-v0.13.0) (2023-04-17)


### Features

* java.util.logging.Handler ([#316](https://github.com/toad-lib/toad/issues/316)) ([10c62ed](https://github.com/toad-lib/toad/commit/10c62edef8ce1a30d7e38e02cbbeb13c7a8f66dd))

## [0.12.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.11.0...toad-jni-v0.12.0) (2023-04-17)


### Features

* java.io.logger.Logger ([#314](https://github.com/toad-lib/toad/issues/314)) ([c696d6f](https://github.com/toad-lib/toad/commit/c696d6ffaecd25a802912a3058da7cd2d969060b))

## [0.11.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.10.1...toad-jni-v0.11.0) (2023-04-16)


### Features

* System.out, System.err ([#310](https://github.com/toad-lib/toad/issues/310)) ([6cbcc87](https://github.com/toad-lib/toad/commit/6cbcc8787fa17ce201efd6b2d121e912a0c6410d))

## [0.10.1](https://github.com/toad-lib/toad/compare/toad-jni-v0.10.0...toad-jni-v0.10.1) (2023-04-16)


### Bug Fixes

* peekable channel should impl object, class and poll should work ([#308](https://github.com/toad-lib/toad/issues/308)) ([2f1f3b0](https://github.com/toad-lib/toad/commit/2f1f3b0c9d85385bdbb7f912ae9d6f62ba8fd858))

## [0.10.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.9.1...toad-jni-v0.10.0) (2023-04-15)


### Features

* Throwable's Debug should include error stack, java.io.IOException ([#306](https://github.com/toad-lib/toad/issues/306)) ([6f47fcb](https://github.com/toad-lib/toad/commit/6f47fcbccee7fe815b577a72490895e805d598d6))

## [0.9.1](https://github.com/toad-lib/toad/compare/toad-jni-v0.9.0...toad-jni-v0.9.1) (2023-04-13)


### Bug Fixes

* PeekableDatagramChannel unreachable  ([#304](https://github.com/toad-lib/toad/issues/304)) ([97988fb](https://github.com/toad-lib/toad/commit/97988fb76c516dd309944f8d1fc0e0a7cb98cda0))

## [0.9.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.8.0...toad-jni-v0.9.0) (2023-04-13)


### Features

* add DatagramChannel + impl toad::net::Socket ([#302](https://github.com/toad-lib/toad/issues/302)) ([55230ea](https://github.com/toad-lib/toad/commit/55230eae2e8b9ee8466cec143c3b17e1148a0097))

## [0.8.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.7.0...toad-jni-v0.8.0) (2023-04-11)


### Features

* java.lang.Throwable ([#300](https://github.com/toad-lib/toad/issues/300)) ([e6dede1](https://github.com/toad-lib/toad/commit/e6dede1a7a85ed6ebd409dc5ddbea056e9e67337))

## [0.7.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.6.0...toad-jni-v0.7.0) (2023-04-10)


### Features

* java.net InetSocketAddress, Inet4Address, Inet6Address, InetAddress ([#299](https://github.com/toad-lib/toad/issues/299)) ([7861a17](https://github.com/toad-lib/toad/commit/7861a17fdf63c707bd17a47ccbf710331fb02986))
* yield_to_java + unwrap_java ([#297](https://github.com/toad-lib/toad/issues/297)) ([0f2ae1c](https://github.com/toad-lib/toad/commit/0f2ae1c6f582e6ead1218faf6d91496a29e7e7b5))

## [0.6.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.5.1...toad-jni-v0.6.0) (2023-04-09)


### Features

* java.lang.System, java.io.Console ([#295](https://github.com/toad-lib/toad/issues/295)) ([30ceab3](https://github.com/toad-lib/toad/commit/30ceab3aca138b3a436ce59ada517211b98cbca8))

## [0.5.1](https://github.com/toad-lib/toad/compare/toad-jni-v0.5.0...toad-jni-v0.5.1) (2023-04-07)


### Bug Fixes

* field deadlock + panic ([#291](https://github.com/toad-lib/toad/issues/291)) ([3325fe1](https://github.com/toad-lib/toad/commit/3325fe123ffb58353479753187acba67fa8200a8))

## [0.5.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.4.1...toad-jni-v0.5.0) (2023-04-07)


### ⚠ BREAKING CHANGES

* rework toad_jni type system ([#289](https://github.com/toad-lib/toad/issues/289))

### Features

* rework toad_jni type system ([#289](https://github.com/toad-lib/toad/issues/289)) ([9b20ce7](https://github.com/toad-lib/toad/commit/9b20ce7b441e195e03768dbb0621f20e75ae7353))

## [0.4.1](https://github.com/toad-lib/toad/compare/toad-jni-v0.4.0...toad-jni-v0.4.1) (2023-04-05)


### Bug Fixes

* BigInteger should support narrowing if there are lots of leading zeroes ([#287](https://github.com/toad-lib/toad/issues/287)) ([c72dbdf](https://github.com/toad-lib/toad/commit/c72dbdfb0cd486fded8b33e0ca6f73ad7136f0fc))

## [0.4.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.3.0...toad-jni-v0.4.0) (2023-04-05)


### Features

* java.math.BigInteger ([#285](https://github.com/toad-lib/toad/issues/285)) ([6b6bd17](https://github.com/toad-lib/toad/commit/6b6bd1730aa8825dcc947eab0d3dc9996a485932))

## [0.3.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.2.0...toad-jni-v0.3.0) (2023-04-04)


### Features

* add java.util.Optional support ([#281](https://github.com/toad-lib/toad/issues/281)) ([f0727b1](https://github.com/toad-lib/toad/commit/f0727b1d552fbb320e64a7f483a6f3b2a1901b18))
* initial commit on toad-jni ([#279](https://github.com/toad-lib/toad/issues/279)) ([96cd758](https://github.com/toad-lib/toad/commit/96cd758621128d0085d9d22281b4b2d355e7bd64))
* java.time.Duration ([#283](https://github.com/toad-lib/toad/issues/283)) ([55fa83c](https://github.com/toad-lib/toad/commit/55fa83ce9aec93558e8cdefc0accabb783c87eaa))
* **kwap:** add support for coap runtime config ([c082f06](https://github.com/toad-lib/toad/commit/c082f0696a288d2a2db9b986c3e3eaf2e7a4e8f4))

## [0.2.0](https://github.com/toad-lib/toad/compare/toad-jni-v0.1.0...toad-jni-v0.2.0) (2023-04-04)


### Features

* add java.util.Optional support ([#281](https://github.com/toad-lib/toad/issues/281)) ([f0727b1](https://github.com/toad-lib/toad/commit/f0727b1d552fbb320e64a7f483a6f3b2a1901b18))
* java.time.Duration ([#283](https://github.com/toad-lib/toad/issues/283)) ([55fa83c](https://github.com/toad-lib/toad/commit/55fa83ce9aec93558e8cdefc0accabb783c87eaa))

## 0.1.0 (2023-04-02)


### Features

* initial commit on toad-jni ([#279](https://github.com/toad-lib/toad/issues/279)) ([96cd758](https://github.com/toad-lib/toad/commit/96cd758621128d0085d9d22281b4b2d355e7bd64))
* **kwap:** add support for coap runtime config ([c082f06](https://github.com/toad-lib/toad/commit/c082f0696a288d2a2db9b986c3e3eaf2e7a4e8f4))
