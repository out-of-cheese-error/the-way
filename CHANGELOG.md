# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.17.1] - 2022-05-30
- Don't save environment variable `THE_WAY_GITHUB_TOKEN` to config file
- If `THE_WAY_GITHUB_TOKEN` not set and `github_access_token` not in config file, prompt for token and prompt for saving to config file

## [0.16.1] - 2022-02-19
### Added
- `--colorize` flag to force colorization even in non-tty environments.

## [0.16.0] - 2022-02-06
**BREAKING RELEASE - Make sure to back up your snippets**
**PLEASE RUN `the-way sync local` AFTER UPDATE (or delete and recreate the Gist)**

This is needed as the new Gist format has tags attached and running `the-way sync date` will get rid of all your tags.
Using `the-way sync local` or deleting and recreating the Gist will upload local snippets to synced gist with tags attached. 
Subsequent syncing can then use `the-way sync date`.

### Changed
* `the-way sync` is now split into three different commands (Issue [125](https://github.com/out-of-cheese-error/the-way/issues/125)): 
  * `the-way sync date` (old behavior, checks each snippet's updated date and uploads if newer than Gist updated date, downloads otherwise)
  * `the-way sync local` (uses local snippets as truth, needed after upgrading to this release in order to upload tags to Gist. Also useful if Gist gets messed up for some reason)
  * `the-way sync gist` (uses Gist snippets as truth, useful when syncing across computers)
* Don't use ANSI color codes when terminal is not in tty mode (Issue [123](https://github.com/out-of-cheese-error/the-way/issues/123))

### Added
* `-s` for `--stdout` in `copy` and `search` commands (Issue [122](https://github.com/out-of-cheese-error/the-way/issues/122))
* Option to import a `the-way`-style gist with `the-way import -w <gist-url>` (Issue [98](https://github.com/out-of-cheese-error/the-way/issues/98))
* Field `copy_cmd` in configuration file which allows user to change the default copy command. 
  (Issue [110](https://github.com/out-of-cheese-error/the-way/issues/122), Issue [76](https://github.com/out-of-cheese-error/the-way/issues/76))
  In case of empty field value (empty string) the default command is used.
  ([PR 118](https://github.com/out-of-cheese-error/the-way/pull/118))
* Enum error `NoDefaultCopyCommand` to represent the case where the OS copy
  command is not supported by default.

## [0.15.0] - 2022-01-07
### Added
* Search in code as well as description/tags (Issue [115](https://github.com/out-of-cheese-error/the-way/issues/115))
* `-e/--exact` option for search that toggles exact search

### Changed

* Prompt user to open snippet in editor when editing existing snippet. (Issue [#104](https://github.com/out-of-cheese-error/the-way/issues/104))
* Updated dependencies

## [0.14.4] - 2021-09-05
### Fixed
* .deb package extended description

## [0.14.3] - 2021-09-05

* Highlight parameters in shell snippets (Issue [#75](https://github.com/out-of-cheese-error/the-way/issues/75))
* Make .deb with CD
* Updated dependencies

## [0.14.2] - 2021-08-11

* Updated dependencies

## [0.14.1] - 2021-06-22

* Updated indicatif dependency
* Switched from Travis CI to Github Actions

## [0.14.0] - 2021-06-22

* Newline-delimited JSON as export format
* Updated dependencies
* Added `termux-clipboard-set` as copy command for Android (
  Issue [#93](https://github.com/out-of-cheese-error/the-way/issues/93))
* `--stdout` prints to stdout *without* copying

## [0.13.0] - 2021-01-10

### Added

`--stdout` flag to copy and search (Issue [#93](https://github.com/out-of-cheese-error/the-way/issues/93))

### Changed

* Updated dependencies (except directories-next)
* Updated ureq code to v2

## [0.12.1] - 2020-11-22

### Fixed

Regex for matching `<param>` and `<param=value>` failed when multiple `>` were present (
Issue [#91](https://github.com/out-of-cheese-error/the-way/issues/91))

## [0.12.0] - 2020-11-21

### Changed

* `the-way themes set` gives a list of available themes to select from if no theme is given as
  input ([PR 88](https://github.com/out-of-cheese-error/the-way/pull/88))
* Better clipboard errors ([PR 90](https://github.com/out-of-cheese-error/the-way/pull/90))
* Added `xclip`/`pbcopy` to requirements in README
* Updated dependencies

### Removed
`the-way themes list`

## [0.11.1] - 2020-10-24
### Changed
* Fixed keyboard shortcuts in search help and README
* Updated dependencies

## [0.11.0] - 2020-10-22
**Needs an export-import to fix shell snippet extensions!**

```bash
the-way export snippets.json
the-way clear
the-way import snippets.json
```

### Added
* Keyboard shortcuts in `search` mode for deleting and editing snippets interactively. ([PR 85](https://github.com/out-of-cheese-error/the-way/pull/85))
* Demo for `the-way cmd`

### Changed
* Single line code can be edited without external editor. ([PR 83](https://github.com/out-of-cheese-error/the-way/pull/83))
* Nicer errors: Backtrace information / warning no longer displayed. ([f1fbb68](https://github.com/out-of-cheese-error/the-way/commit/f1fbb68a0241840d8f952057e5aaaf7114b52e73))
* Print statements use chosen theme color ([da2cf91](https://github.com/out-of-cheese-error/the-way/commit/da2cf91abcbe30375d88c3e003e0b2a0b7634b93))

### Fixed
* Shell snippets saved with .sh extension instead of sh ([3d8d0c3](https://github.com/out-of-cheese-error/the-way/commit/3d8d0c39b0c998c5cd042c4820404dd1fccee6ca)). This needs an export + import.
* Better output when `the-way sync` is called with no snippets stored ([7e0991d](https://github.com/out-of-cheese-error/the-way/commit/7e0991d7ff1eec62434746cd772479b09e44cd41))

## [0.10.1] - 2020-10-18
### Added
Installation via `yay` ([PR 81](https://github.com/out-of-cheese-error/the-way/pull/81) and [82](https://github.com/out-of-cheese-error/the-way/pull/82) by [spikecodes](https://github.com/spikecodes))

### Changed
* Allow arrow key navigation while editing text in CLI ([Issue #73](https://github.com/out-of-cheese-error/the-way/issues/73))
* Spaces in `$EDITOR` work now ([Issue #80](https://github.com/out-of-cheese-error/the-way/issues/80))
* Updated dependencies

## [0.10.0] - 2020-10-14
### Added
* Added import from gist functionality ([PR 79](https://github.com/out-of-cheese-error/the-way/pull/79) by [@xiaochuanyu](https://github.com/xiaochuanyu)).
Run `the-way import -g <gist_url>`

### Changed
* Updated `languages.yml` for more GitHub language colors.

## [0.9.0] - 2020-10-08
### Added
* `the-way cmd` allows adding shell snippets without asking for the language. 
Also, it takes the code as an argument so can be used in a shell function to automatically add the last used command for instance.
* shell snippets can have user-controlled variables inside using `<param>` and `<param=value>`. 
These are queried interactively from the user whenever the snippet is selected (with `search` or `cp`)
* old snippet information is now editable in `the-way edit` (arrow key navigation still doesn't work, waiting on the next dialoguer release)

## [0.8.0] - 2020-10-06
### Added
* Filter using a regex pattern with `-p` or `--pattern` ([PR #68](https://github.com/out-of-cheese-error/the-way/pull/68) by [@meluskyc](https://github.com/meluskyc))
* Can install `the-way` with brew.

### Changed
* Updated dependencies

## [0.7.0] - 2020-09-03
**BREAKING RELEASE - needs a database migration**
* Before upgrade 
```bash
the-way export > snippets.json
the-way clear
```
* After upgrade
```bash
the-way import snippets.json
```
### Changed
* Switched from `reqwest` to `ureq`
* Updated dependencies


## [0.6.1] - 2020-07-23
Sort snippets numerically in `list` and `search`
Fixes [Issue #65](https://github.com/out-of-cheese-error/the-way/issues/65)

## [0.6.0] - 2020-07-15
**BREAKING RELEASE - needs a database migration**
* Before upgrade 
```bash
the-way export > snippets.json
the-way clear
```
* After upgrade
```bash
the-way import snippets.json
```
### Added
`the-way themes language <language.sublime-syntax>` - Add support for syntax highlighting non-default languages ([Issue #63](https://github.com/out-of-cheese-error/the-way/issues/63))
### Changed
* Removed `color_spantrace` dependency
* Bumped `sled` to v0.33.0

## [0.5.0] - 2020-07-14
**BREAKING RELEASE - needs a database migration:**
* Before upgrade 
```bash
the-way export > snippets.json
the-way clear
```
* After upgrade
```bash
the-way import snippets.json
```
### Added
**Sync to Gist functionality!** [Issue #60](https://github.com/out-of-cheese-error/the-way/issues/60)
### Fixed
* `export filename` and `config default filename` work without needing a `>` or an existing file
### Changed
* bumped all dependency versions to latest
* aliased `cp` to `copy`, `del` to `delete` and `config` to `configure`

## [0.4.0] - 2020-07-04
### Fixed
[Issue #58](https://github.com/out-of-cheese-error/the-way/issues/58) - changed search highlight and tag colors
### Changed
Bumped `eyre` and `color_eyre` versions. Holding off on bumping `sled` since it would need a database migration.

## [0.3.2] - 2020-06-03
### Fixed
[Issue #56](https://github.com/out-of-cheese-error/the-way/issues/56)

## [0.3.1] - 2020-05-26
### Changed
* Code highlighter defaults to .txt if syntax not found. This is a workaround b/c `syntect` uses some kind of default syntax set which is a subset of 
the GitHub syntax set. Need to figure this out though, Kotlin isn't highlighted!
* Switched to [`directories-next`](https://github.com/xdg-rs/dirs).

## [0.3.0] - 2020-05-14
I hope I'm following semver correctly, this is a minor version update and not a patch update because the CLI input prompt style changed
Also, no one should be using 0.2.4 b/c of the database bug.

### Added
Documentation for adding syntax highlighting themes to the README (Issue [#47](https://github.com/out-of-cheese-error/the-way/issues/47))

### Changed
Updated [dialoguer](https://github.com/mitsuhiko/dialoguer) to 0.6.2.
This makes `new` and `edit` look much nicer. Destructive commands (`clear` and `del`) now use dialoguer's Confirm prompt.

### Fixed
The code preview for `search` shows the correct number of lines now in the top right corner, previously it showed 3 extra because of newlines.
This fixes Issue [#46](https://github.com/out-of-cheese-error/the-way/issues/46)

## [0.2.5] - 2020-05-13
### Fixed
Fixed a pretty terrible bug - this is why tests matter. Snippet index is incremented after adding a snippet, also this is tested now 
(like it already should've been). Fixes Issue [#43](https://github.com/out-of-cheese-error/the-way/issues/43)

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

[0.16.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.16.0...v0.16.1
[0.16.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.14.4...v0.15.0
[0.14.4]: https://github.com/out-of-cheese-error/the-way/compare/v0.14.3...v0.14.4
[0.14.3]: https://github.com/out-of-cheese-error/the-way/compare/v0.14.2...v0.14.3
[0.14.2]: https://github.com/out-of-cheese-error/the-way/compare/v0.14.1...v0.14.2
[0.14.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.14.0...v0.14.1
[0.14.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.12.1...v0.13.0
[0.12.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.11.1...v0.12.0    
[0.11.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.11.0...v0.11.1
[0.11.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.10.1...v0.11.0
[0.10.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/out-of-cheese-error/the-way/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/out-of-cheese-error/the-way/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/out-of-cheese-error/the-way/compare/v0.2.5...v0.3.0
[0.2.5]: https://github.com/out-of-cheese-error/the-way/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/out-of-cheese-error/the-way/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/out-of-cheese-error/the-way/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/out-of-cheese-error/the-way/releases/tag/v0.2.2
[0.2.1]: https://github.com/out-of-cheese-error/the-way/releases/tag/v0.2.1-osx
