[![Crates.io](https://img.shields.io/crates/v/the-way.svg)](https://crates.io/crates/the-way)
[![Build Status](https://github.com/out-of-cheese-error/the-way/workflows/Continuous%20Integration/badge.svg)](https://github.com/out-of-cheese-error/the-way/actions)
[![GitHub release](https://img.shields.io/github/release/out-of-cheese-error/the-way.svg)](https://GitHub.com/out-of-cheese-error/the-way/releases/)
[![dependency status](https://deps.rs/repo/github/out-of-cheese-error/the-way/status.svg)](https://deps.rs/repo/github/out-of-cheese-error/the-way)
[![GitHub license](https://img.shields.io/github/license/out-of-cheese-error/the-way.svg)](https://github.com/out-of-cheese-error/the-way/blob/master/LICENSE)

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/ninjani)

# The Way

A code snippets manager for your terminal.

Record and retrieve snippets you use every day, or once in a blue moon,
without having to spin up a browser. Just call `the-way new` to add a snippet with a
description, a language, and some tags attached.

`the-way search` fuzzy searches your snippets library (with optional filters on language and tags) and
lets you

* edit a snippet with Shift-Right
* delete a snippet with Shift-Left
* copy a particular snippet to your clipboard (with Enter), so you can paste it into whatever editor or IDE you're
  working with.

See it in action with some self-referential examples (click to open in asciinema):

[![demo](https://asciinema.org/a/436289.png)](https://asciinema.org/a/436289)


Table of Contents
=================

* [Install](#install)
    * [Requirements](#requirements)
    * [Binaries](#binaries)
    * [With brew](#with-brew)
    * [With cargo](#with-cargo)
    * [With yay](#with-yay)
    * [On Android](#on-android)
    * [Upgrading](#upgrading)
* [Usage](#usage)
* [Features](#features)
    * [Main features](#main-features)
    * [Shell commands](#shell-commands)
        * [bash](#bash)
        * [zsh](#zsh)
        * [fish](#fish)
    * [Sync to Gist](#sync-to-gist)
    * [Shell completions](#shell-completions)
    * [Syntax highlighting](#syntax-highlighting)
    * [Configuration](#configuration)
        * [Copy command](#copy-command)
* [Why "The Way"?](#why-the-way)

# Install

## Requirements

`xclip` on Linux and `pbcopy` on OSX (see [here](https://github.com/out-of-cheese-error/the-way#copy-command) for more
options)

## Binaries

See the [releases](https://github.com/out-of-cheese-error/the-way/releases/latest)

* OSX - allow `the-way` via System Preferences (necessary in Catalina at least)
* Linux - `chmod +x the-way`
* Currently doesn't work on Windows (waiting on [this issue](https://github.com/lotabout/skim/issues/293))
* Can work on Windows Subsystem for Linux by changing the copy command as
  described [here](https://github.com/out-of-cheese-error/the-way/issues/134#issuecomment-1144645554)

## With brew

```bash
brew tap out-of-cheese-error/the-way && brew install the-way
```

## With cargo

```bash
cargo install the-way
```

## With yay

```bash
yay -S the-way-git
```

## On Android

Needs Termux, Termux:API and `pkg install termux-api`

Clone the repository, run `cargo build --release`, and use `target/release/the-way`

## Upgrading

**Some upgrades need a database migration** (mentioned in the release notes):

* Before upgrade

```bash
the-way export > snippets.json
the-way clear
```

* After upgrade

```bash
the-way import snippets.json
```

# Usage

```
A code snippets manager for your terminal

Usage: the-way [OPTIONS] <COMMAND>

Commands:
  new       Add a new code snippet
  cmd       Add a new shell snippet
  search    Fuzzy search to find a snippet and copy, edit or delete it
  sync      Sync snippets to a Gist
  list      Lists (optionally filtered) snippets
  import    Imports code snippets from JSON
  export    Saves (optionally filtered) snippets to JSON
  clear     Clears all data
  complete  Generate shell completions
  themes    Manage syntax highlighting themes
  config    Manage the-way data locations
  edit      Change snippet
  del       Delete snippet
  cp        Copy snippet to clipboard
  view      View snippet

Options:
  -c, --colorize  Force colorization even when not in TTY mode
  -p, --plain     Turn off colorization
  -h, --help      Print help information (use `--help` for more detail)
  -V, --version   Print version information
```

# Features

## Main features

* Add code and shell snippets
* Interactive fuzzy or exact search with edit, delete and copy to clipboard functionality
* Filter by tag, date, language and/or regex pattern
* Import / export via JSON
* Import from Gist (with `the-way import -g <gist_url>`)
* Sync to gist
* Syntax highlighting

## Shell commands

`the-way cmd` (inspired by [pet](https://github.com/knqyf263/pet)) makes it easier to save single-line bash/shell
snippets with variables that can be filled in whenever the snippet is needed.

Add the following function according to your shell of choice. Every time you spend ages hand-crafting the perfect
command: run it, close all the stackoverflow tabs, and run `cmdsave` to save it to `the-way`. You can then
use `cmdsearch` to search these shell snippets and have the selected one already pasted into the terminal, ready to run.

### bash

```shell script
function cmdsave() {
  PREV=$(echo `history | tail -n2 | head -n1` | sed 's/[0-9]* //')
  sh -c "the-way cmd `printf %q "$PREV"`"
}

function cmdsearch() {
  BUFFER=$(the-way search --stdout --languages="sh")
  bind '"\e[0n": "'"$BUFFER"'"'; printf '\e[5n'
}
```

### zsh

```shell script
function cmdsave() {
  PREV=$(fc -lrn | head -n 1)
  sh -c "the-way cmd `printf %q "$PREV"`"
}

function cmdsearch() {
  BUFFER=$(the-way search --stdout --languages="sh")
  print -z $BUFFER
}
```

### fish

```shell script
function cmdsave
  set line (echo $history[1])
  the-way cmd $line
end

function cmdsearch
  commandline (the-way search --languages=sh --stdout)
end
```

You'll usually want different parameters each time you need a shell command: save variables in a shell snippet
as `<param>` or `<param=default_value>` and every time you select it you can interactively fill them in (or keep the
defaults). Parameters can appear more than once, just use the same name and write in the default the first time it's
used.

Here's another self-referential example that saves a shell command to add new language syntaxes:

[![cmd_demo](https://asciinema.org/a/436293.png)](https://asciinema.org/a/436293)

(todo: Use cmdsearch instead of search)

## Sync to Gist

`the-way sync date` syncs snippets to a Gist, each named `snippet_<index>.<extension>`, with an `index.md` file linking
each snippet's description and tags.
Local updates and deletions are uploaded to the Gist and Gist updates are downloaded.

`the-way sync local` uploads all local changes, additions, and deletions to the Gist.
This is useful after upgrading to a new version of the-way if the Gist format has changed, or something gets messed up
in the Gist.

`the-way sync gist` downloads all Gist changes, additions, and deletions to the local database.
This is useful to sync snippets across computers, as it uses the Gist as the source of truth.

![gist](images/gist.png)

This functionality needs a [GitHub access token](https://github.com/settings/tokens/new) with the "gist" scope.
Either enter this token on running `sync` for the first time or set it to the environment
variable `$THE_WAY_GITHUB_TOKEN`.

You can also import snippets from a Gist created by the-way using `the-way import -w <gist_url>`.

## Shell completions

```bash
the-way complete zsh > .oh-my-zsh/completions/_the-way
exec zsh
```

## Syntax highlighting

The Way maps languages to their extensions and uses this to

1. Enable syntax highlighting in `$EDITOR` (if the editor supports it),
2. Upload snippets to Gist with the correct extension,
3. Add a small colored language indicator (GitHub-flavored)
4. Syntax highlight code in the terminal

The last point can be customized via `the-way themes`.

Use `the-way themes set` to see available themes and enable a theme.

Default themes:

```
Darcula
InspiredGitHub
Solarized (dark)
Solarized (light)
base16-eighties.dark
base16-mocha.dark
base16-ocean.dark
base16-ocean.light
base16-tomorrow.dark
base16-twilight.dark
```

Use `the-way themes add <theme.tmTheme>` to add a new theme to your themes folder.
Theme files need to be in Sublime's [.tmTheme](https://www.sublimetext.com/docs/3/color_schemes_tmtheme.html) format.
Searching GitHub for [.tmTheme](https://github.com/search?q=.tmTheme) pulls up some examples.

Use `the-way themes language <language.sublime-syntax>` to add highlight support for a new language
([many languages](https://github.com/sublimehq/Packages/) are supported by default).
Syntax files need to be in Sublime's sublime-syntax format.
[Zola](https://github.com/getzola/zola/tree/master/sublime/syntaxes) has a nice collection of such files.

Here's how it looks before and after adding `Kotlin.sublime-syntax`:

* Before:

![kotlin_plain](images/kotlin_plain.png)

* After:

![kotlin_highlight](images/kotlin_highlight.png)

To get syntax highlighting for code blocks in markdown files, download and add the patched `Markdown.sublime-syntax`
file in this repository,
taken from [bat](https://github.com/sharkdp/bat/blob/master/assets/patches/Markdown.sublime-syntax.patch)
(the default syntax file [doesn't do this anymore](https://github.com/sharkdp/bat/issues/963))

## Configuration

The default config TOML file is located in

* Linux: `/home/<username>/.config/the-way`
* Mac: `/Users/<username>/Library/Preferences/rs.the-way`

This file contains locations of data directories, which are automatically created and set according to XDG and Standard
Directories guidelines.
Change this by creating a config file with `the-way config default > config.toml` and then setting the environment
variable `$THE_WAY_CONFIG` to point to this file.

### Copy command

By default `xclip` is used on Linux, `pbcopy` on OSX and `termux-clipboard-set` on Android.
You can override the default command by setting the `copy_cmd` field in the configuration file.
For example to use `wl-copy` as a copy command on Wayland, set the `copy_cmd` field as follows:

```toml
copy_cmd = 'wl-copy --trim-newline'
```

# Why "The Way"?

The name is a reference to [the Way of Mrs.Cosmopilite](https://wiki.lspace.org/The_Way_of_Mrs._Cosmopilite), kōans for
every situation.
