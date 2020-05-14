[![Crates.io](https://img.shields.io/crates/v/the-way.svg)](https://crates.io/crates/the-way)
[![Build Status](https://travis-ci.org/out-of-cheese-error/the-way.svg?branch=master)](https://travis-ci.org/out-of-cheese-error/the-way)

# The Way (`the-way`)
A code snippets manager for your terminal.

Record and retrieve snippets you use every day, or once in a blue moon,
without having to spin up a browser. Just call `the-way new` to add a snippet with a 
description, a language, and some tags attached. `the-way search` fuzzy 
searches your snippets library (with optional filters on language and tags) and 
lets you copy a particular snippet to your clipboard, so you can paste 
it into whatever editor or IDE you're working with.

See it in action (with some self-referential examples):

![demo](demo.gif)

> made with [Terminalizer](https://github.com/faressoft/terminalizer)

## Install

### Binaries
See the [releases](https://github.com/out-of-cheese-error/the-way/releases/latest)

* OSX - allow `the-way` via System Preferences (necessary in Catalina at least)
* Linux - `chmod +x the-way` 

### With cargo
```bash
cargo install the-way
```

Currently doesn't work on Windows (waiting on [this issue](https://github.com/lotabout/skim/issues/293))

### Make shell completions:
```bash
the-way complete zsh > .oh_my_zsh/completions/_the-way
exec zsh
```

### Syntax highlighting themes
Theme files need to be in Sublime's [.tmTheme](https://www.sublimetext.com/docs/3/color_schemes_tmtheme.html) format. 
Searching GitHub for [.tmTheme](https://github.com/search?q=.tmTheme) pulls up some examples. Use `the-way themes add <theme.tmTheme>`
to add a new theme to your themes folder, and then `the-way themes set <theme>` to make it your default. 
`the-way themes list` shows all available themes.

## Features
- [X] Recognizes language extension to enable syntax highlighting in $EDITOR (if the editor supports it)
- [X] Configurable syntax highlighting themes for display via `the-way themes`
- [X] Import/export to JSON
- [X] Shell completion
- [ ] Language and code statistics

## Why "The Way"?
The name is a reference to [the Way of Mrs.Cosmopilite](https://wiki.lspace.org/mediawiki/The_Way_of_Mrs._Cosmopilite), kōans for every situation.
