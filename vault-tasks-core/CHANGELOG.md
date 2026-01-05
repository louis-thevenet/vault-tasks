# Changelog

## [1.0.0](https://github.com/louis-thevenet/vault-tasks/compare/core-v0.13.0...core-v1.0.0) (2026-01-05)


### ⚠ BREAKING CHANGES

* rework config options

### Features

* **core:** include relative date in stdout mode ([671408b](https://github.com/louis-thevenet/vault-tasks/commit/671408b2dc056969387667b0457f83f6fad83f33))
* remove trackers ([#174](https://github.com/louis-thevenet/vault-tasks/issues/174)) ([8de42d5](https://github.com/louis-thevenet/vault-tasks/commit/8de42d5976d12bb1f35b39bc7d6563fb737818b8))
* **tui:** add default drop file for tasks created with new-task CLI command ([a07ac5f](https://github.com/louis-thevenet/vault-tasks/commit/a07ac5f88c0776668dc487df42a1951119be4943))


### Bug Fixes

* **core:** create file if it doesn't exist when fixing task's attributes ([0ee8de9](https://github.com/louis-thevenet/vault-tasks/commit/0ee8de9b5962fe7bc834aec8df92751e42ab86b6))
* **core:** default task's line number is None ([629b1b0](https://github.com/louis-thevenet/vault-tasks/commit/629b1b0020fe4ea14d532ee4728944c452177cde))
* **core:** remove extra file wrapping causing extra header containing filename in addition to the usual file wrapping ([c5a5f2d](https://github.com/louis-thevenet/vault-tasks/commit/c5a5f2db42b83e0df5de6c57e45eac4fc7616c01))
* tui artifacts by removing carriage returns ([#145](https://github.com/louis-thevenet/vault-tasks/issues/145)) ([08b36e9](https://github.com/louis-thevenet/vault-tasks/commit/08b36e9eb41d90916a4de326cd9299486be13775))


### Code Refactoring

* rework config options ([24a217a](https://github.com/louis-thevenet/vault-tasks/commit/24a217a56874bcf989ca4d4bf6297e01ddeaa384))
