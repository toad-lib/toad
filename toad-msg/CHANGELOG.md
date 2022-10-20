# Changelog

All notable changes to this project will be documented in this file. See [standard-version](https://github.com/conventional-changelog/standard-version) for commit guidelines.

## 0.8.0 (2022-10-20)


### ⚠ BREAKING CHANGES

* **toad-msg:** parsing messages should use slices instead of iterators (#142)

### Features

* **toad-msg:** parsing messages should use slices instead of iterators ([#142](https://github.com/clov-coffee/toad/issues/142)) ([03b3a5b](https://github.com/clov-coffee/toad/commit/03b3a5b0155dd8104ced35825be3cebd051d81c9))

## 0.7.0 (2022-10-08)


### ⚠ BREAKING CHANGES

* rename kwap_msg -> toad_msg (#136)

### Features

* rename kwap_msg -> toad_msg ([#136](https://github.com/clov-coffee/toad/issues/136)) ([1035350](https://github.com/clov-coffee/toad/commit/1035350f453c1c0d5433a13b287f5fc9d5c556e9))

### 0.6.1 (2022-06-18)


### Features

* **msg:** add critical, nocachekey, unsafe flags to optnumber ([#132](https://github.com/clov-coffee/toad/issues/132)) ([3db6794](https://github.com/clov-coffee/toad/commit/3db6794af1ea9ceb514a7c9692331c86d25b436d))

## 0.6.0 (2022-05-25)


### ⚠ BREAKING CHANGES

* **toad:** support ipv4 and ipv6 (#126)

### Features

* **toad:** support ipv4 and ipv6 ([#126](https://github.com/clov-coffee/toad/issues/126)) ([9150ca1](https://github.com/clov-coffee/toad/commit/9150ca13950db5c8f17f0963f3ae111f8362ba79))

### 0.5.1 (2022-05-20)


### Features

* **toad-msg:** add Token::opaque, Message::ack, Code.kind  ([#118](https://github.com/clov-coffee/toad/issues/118)) ([3119233](https://github.com/clov-coffee/toad/commit/31192330fa712d26147b61ef184f17ba3c534554))

## 0.5.0 (2022-05-20)


### ⚠ BREAKING CHANGES

* **toad:** Choose CON / NON responses to NON requests (#117)

### Features

* **toad:** Choose CON / NON responses to NON requests ([#117](https://github.com/clov-coffee/toad/issues/117)) ([5d39603](https://github.com/clov-coffee/toad/commit/5d3960314ffef7cac4f896d92c056d6e9100f10e))

### 0.4.7 (2022-05-10)

### 0.4.6 (2022-04-27)


### Features

* **msg:** update common ([#71](https://github.com/clov-coffee/toad/issues/71)) ([6cf4927](https://github.com/clov-coffee/toad/commit/6cf49272c096eb15694325596e368946249cd992))

### 0.4.5 (2022-01-29)

### 0.4.4 (2022-01-15)


### Bug Fixes

* **msg:** Code should impl Eq + Ord ([#57](https://github.com/clov-coffee/toad/issues/57)) ([96bdd8d](https://github.com/clov-coffee/toad/commit/96bdd8da8ab71c6c6a828c95d6d38ff90a3d1dd4))

### 0.4.3 (2022-01-11)

### 0.4.2 (2022-01-08)


### Bug Fixes

* **msg:** OptNumber should impl Default ([#47](https://github.com/clov-coffee/toad/issues/47)) ([e3b8687](https://github.com/clov-coffee/toad/commit/e3b8687b1b30de909a0c8b6203c355c1ee1c7aaa))

### 0.4.1 (2022-01-07)


### Bug Fixes

* **msg:** all where clauses should be gone ([#44](https://github.com/clov-coffee/toad/issues/44)) ([f067a87](https://github.com/clov-coffee/toad/commit/f067a87b705b0800b1729893dc34942aebb27917))

## 0.4.0 (2022-01-04)


### ⚠ BREAKING CHANGES

* **msg:** toad_msg should use latest version of toad_common (#42)

### Features

* **msg:** toad_msg should use latest version of toad_common ([#42](https://github.com/clov-coffee/toad/issues/42)) ([2f035ba](https://github.com/clov-coffee/toad/commit/2f035ba1373126968e7c31c0a4ea327fc9113e50))

## 0.3.0 (2022-01-01)


### ⚠ BREAKING CHANGES

* **msg:** make message type an enum (#38)

### Features

* **msg:** make message type an enum ([#38](https://github.com/clov-coffee/toad/issues/38)) ([91c77b6](https://github.com/clov-coffee/toad/commit/91c77b659a8066ca6c34dbe9e5b3df7abfe2d028))

### 0.2.7 (2022-01-01)


### Bug Fixes

* **msg:** update toad_common ([#37](https://github.com/clov-coffee/toad/issues/37)) ([73baf95](https://github.com/clov-coffee/toad/commit/73baf95ae05faded1751aa4e7e849c62df0ade6b))

### 0.2.6 (2021-12-31)

### 0.2.5 (2021-12-30)


### Bug Fixes

* **msg:** make Code::new a const fn ([#29](https://github.com/clov-coffee/toad/issues/29)) ([e0f22cf](https://github.com/clov-coffee/toad/commit/e0f22cf0663b9ba7a4617f3fa4efbaf82856d155))

### 0.2.4 (2021-12-30)


### Features

* **msg:** add Code::new ([#28](https://github.com/clov-coffee/toad/issues/28)) ([47c1ce7](https://github.com/clov-coffee/toad/commit/47c1ce7c0867052aec9ddba041335a2b763202c4))

### 0.2.3 (2021-12-30)

### 0.2.2 (2021-12-30)

### 0.2.1 (2021-12-30)

## 0.2.0 (2021-12-29)


### ⚠ BREAKING CHANGES

* combine no_alloc and alloc into one Message generic over collection (#17)

### Features

* combine no_alloc and alloc into one Message generic over collection ([#17](https://github.com/clov-coffee/toad/issues/17)) ([10939a2](https://github.com/clov-coffee/toad/commit/10939a230751183e49b60adb45d0bfa84d28a202))

### 0.1.6 (2021-12-27)

### 0.1.5 (2021-12-27)


### Bug Fixes

* bump amend ([c6e34cb](https://github.com/clov-coffee/toad/commit/c6e34cbe57c52bf6c888ef856a4c9ff43d4ccd23))

### 0.1.4 (2021-12-26)


### Bug Fixes

* update token to store token bytes verbatim  ([#8](https://github.com/clov-coffee/toad/issues/8)) ([9dc6724](https://github.com/clov-coffee/toad/commit/9dc6724d9bbc1ef6f7c81156f879ffec88cba20f))

### 0.1.3 (2021-12-26)

### 0.1.2 (2021-12-26)

### 0.1.1 (2021-12-26)
