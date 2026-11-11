# Contributing

Thanks for wanting to help (ﾉ◕ヮ◕)ﾉ*:･ﾟ✧ Bug reports, docs fixes and code are
all welcome.

## Report a bug

Open an [issue](https://github.com/Kisakay/VeeMeeter/issues) with:

- your distribution and PipeWire version (`veemeeter ctl status` prints it);
- the output of `veemeeter ctl status`;
- the daemon logs (`journalctl --user -u veemeeter -e`, or
  `RUST_LOG=debug veemeeter daemon` in a terminal);
- what you expected and what happened.

## Hack on the code

```sh
git clone https://github.com/Kisakay/VeeMeeter
cd VeeMeeter
nix develop          # Rust, libpipewire, clang
cargo build
cargo test
cargo run -- daemon  # terminal 1
cargo run            # terminal 2: the window
```

Keep pull requests focused, run `cargo fmt` and `cargo clippy`, and add a
test when you touch the config or routing logic.

## Edit the docs

The site is [MkDocs](https://www.mkdocs.org/) with
[Material for MkDocs](https://squidfunk.github.io/mkdocs-material/). Pages
live in `docs/`, navigation in `mkdocs.yml`.

=== "With Nix"

    ```sh
    nix shell --impure --expr 'with import (builtins.getFlake "nixpkgs") {}; python3.withPackages (p: [ p.mkdocs-material ])' \
      -c python3 -m mkdocs serve
    ```

=== "With pip"

    ```sh
    python3 -m venv .venv
    .venv/bin/pip install -r docs/requirements.txt
    .venv/bin/mkdocs serve
    ```

Open <http://127.0.0.1:8000/VeeMeeter/>. Before pushing, `mkdocs build --strict`
must pass: the GitHub Pages workflow uses the same command.

### Writing style

- English, short sentences, accurate first. Cute second, but always.
- Mascot boxes: `!!! mimi`, `!!! mimi-think`, `!!! mimi-oops`,
  `!!! mimi-sleepy`. One per page is plenty.
- See the [branding kit](branding.md) for colours and tone.

## Licence

VeeMeeter is **GPL-3.0-only**. By contributing you agree that your
contribution is licensed the same way.
