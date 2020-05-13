# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.4] - 2020-05-13
### Fixed
"Failed to open configuration file" error when running `the-way config default` - the config command now runs without having a valid 
config file location (i.e. one that has read and write permissions) since you can use the command to make a valid file. 
Any other command that needs a valid config file and can't load it now throws a more helpful error telling you how to fix it.
Fixes Issue [#41](https://github.com/out-of-cheese-error/the-way/issues/41)

## [0.2.3] - 2020-05-08
### Added
Colorful errors with suggestions, courtesy of [color_eyre](https://github.com/yaahc/color-eyre)

### Fixed 
A bug in the change_snippet test that made its own release directory causing clashes between targets in Travis.
This uses the correct release directory now based on the TARGET environment variable.


## [0.2.2] - 2020-05-06
### Removed
- clipboard dependency:
Instead, I'm using xclip and pbcopy respectively on Linux and OSX. This fixes Travis 
compilation issues on Linux and the weird issue that clipboard is cleared 
when the-way exits in Linux, which would've been pretty sad.
clipboard is still a dev-dependency for MacOS, to test that copying works.
The copy test is not run for Linux.
- onig_sys dependency:
This was also causing issues on Linux, so now I'm using the 
[fancy_regex feature flag for syntect](https://github.com/trishume/syntect#pure-rust-fancy-regex-mode-without-onig) instead.

### Added
- Linux binaries
- Tests to Linux compilation with Travis
- A changelog:
I'll make sure to add changes to it from now, the previous two releases weren't perfectly documented.

### Changed
- The CLI:
    - copy -> cp
    - delete -> del
    - show -> view
    - change -> edit
    - themes current -> themes get
    - themes -> themes list

## [0.2.1] - 2020-05-02
### Added
- OSX binary
- Better demo
- Added Travis CI (only for OSX)


## 0.2.0 - 2020-05-02
### Added
- A first working version of the-way
- cargo install option
    
[0.2.4]: https://github.com/out-of-cheese-error/the-way/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/out-of-cheese-error/the-way/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/out-of-cheese-error/the-way/releases/tag/v0.2.2
[0.2.1]: https://github.com/out-of-cheese-error/the-way/releases/tag/v0.2.1-osx
