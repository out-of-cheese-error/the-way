# Contributing to The Way

`the-way` welcomes contribution from everyone. Here are the guidelines if you are
thinking of helping out:


## Contributions

Contributions to `the-way` should be made in the form of GitHub
pull requests. Each pull request will be reviewed by me ([@Ninjani](https://github.com/Ninjani)) and either landed in the main tree or
given feedback for changes that would be required. 

Should you wish to work on an issue, please claim it first by commenting on
the GitHub issue that you want to work on it. Also feel free to ask for mentoring and I'll do the best I can!


## Testing

Tests need to be run single-threaded as the same environment variable is changed separately for each test.
Run tests locally with `cargo test -- --test-threads=1`. 
Tests are automatically run on [Travis CI](https://travis-ci.org/out-of-cheese-error/the-way) when a pull request is made.
When working on an issue, add tests relevant to the fixed bug or new feature.

## Conduct

All contributors are expected to follow the [Rust Code of Conduct](http://www.rust-lang.org/conduct.html).

All code in this repository is under the [MIT](http://opensource.org/licenses/MIT) license.

---
adapted from [Servo's CONTRIBUTING.md](https://github.com/servo/servo/blob/master/CONTRIBUTING.md)
