# Changelog

## 0.13.0

### Enhancements

* Added `user_attended_stderr` for checking if stderr is a terminal
* Removed `termios` dependency

### Bug Fixes

* Better handling of key recognition on unix

### Deprecated

* Deprecate `Term::is_term()` in favor of `Term::features().is_attended()`

### BREAKING

* Remove `Term::want_emoji()` in favor of `Term::features().wants_emoji()`
