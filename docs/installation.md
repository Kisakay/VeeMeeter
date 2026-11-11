# Installation

VeeMeeter runs on **Linux with PipeWire** (with `pipewire-pulse` if you want
pavucontrol and desktop volume keys to see its devices). It ships as a Nix
flake and as packages for Debian, Ubuntu, Fedora and Arch Linux; building
from source works on any distribution.

There are two programs in one binary:

| Command | What it is |
|---|---|
| `veemeeter daemon` | The audio engine. Owns the PipeWire graph; best run as a systemd **user** service. |
| `veemeeter` | The mixer window. Starts the daemon automatically if it is not running. |
| `veemeeter ctl …` | Command-line control of the running daemon (see [CLI reference](cli.md)). |

## Choose your setup

=== "NixOS module"

    Add the flake input and import the module:

    ```nix title="flake.nix"
    {
      inputs.veemeeter.url = "github:Kisakay/VeeMeeter";

      outputs = { nixpkgs, veemeeter, ... }: {
        nixosConfigurations.my-pc = nixpkgs.lib.nixosSystem {
          modules = [
            veemeeter.nixosModules.default
            ./configuration.nix
          ];
        };
      };
    }
    ```

    ```nix title="configuration.nix"
    {
      programs.veemeeter.enable = true;
    }
    ```

    This installs the package and the `veemeeter` systemd **user** service.
    After `nixos-rebuild switch`, log out and back in (or run
    `systemctl --user start veemeeter`).

=== "home-manager"

    ```nix title="flake.nix (home-manager)"
    {
      inputs.veemeeter.url = "github:Kisakay/VeeMeeter";
      # …
      homeConfigurations.me = home-manager.lib.homeManagerConfiguration {
        modules = [
          veemeeter.homeManagerModules.default
          { services.veemeeter.enable = true; }
        ];
      };
    }
    ```

    `services.veemeeter.enable` installs the package and runs the daemon as
    a user service for your account only.

=== "Debian / Ubuntu"

    Download the `.deb` for your release from the
    [Releases page](https://github.com/Kisakay/VeeMeeter/releases), then:

    ```sh
    sudo apt install ./veemeeter_*debian13_amd64.deb     # or *ubuntu24.04*, …
    systemctl --user enable --now veemeeter              # optional: engine at login
    ```

=== "Fedora"

    ```sh
    sudo dnf install ./veemeeter-*.fedora*.x86_64.rpm
    systemctl --user enable --now veemeeter
    ```

=== "Arch Linux"

    ```sh
    sudo tar -xzf veemeeter-*-arch-x86_64.tar.gz -C /
    systemctl --user enable --now veemeeter
    ```

    Needs `pipewire`, `wireplumber`, `libglvnd` and `libxkbcommon`.

=== "nix run"

    No installation at all, great for a first try:

    ```sh
    nix run github:Kisakay/VeeMeeter
    ```

    The window spawns the daemon. Closing the window does **not** stop the
    daemon; use `nix run github:Kisakay/VeeMeeter -- ctl quit` when you are done.

=== "From source"

    You need Rust, `libpipewire` (development headers) and clang (for
    `bindgen`). With Nix, the dev shell has everything:

    ```sh
    git clone https://github.com/Kisakay/VeeMeeter
    cd VeeMeeter
    nix develop
    cargo build --release
    ./target/release/veemeeter
    ```

    On other distributions, install the PipeWire development package
    (for example `libpipewire-0.3-dev` or `pipewire-devel`) plus `clang`, then
    run `cargo build --release`.

    To keep the daemon running, copy `assets/veemeeter.service` to
    `~/.config/systemd/user/`, fix the `ExecStart=` path if needed, and run:

    ```sh
    systemctl --user daemon-reload
    systemctl --user enable --now veemeeter
    ```

## Check that it works

```sh
systemctl --user status veemeeter   # if you use the service
veemeeter ctl status                # strips, routes, devices, apps
```

On the first start VeeMeeter writes a default setup to
`~/.config/veemeeter/config.toml`: a microphone, two virtual inputs (*Desktop*
and *Voice chat*), your speakers as **A1** and a virtual microphone as **B1**.
[Getting started](concepts.md) explains what each of them does.

!!! mimi-sleepy "Uninstalling"
    Stop the daemon (`veemeeter ctl quit` or `systemctl --user stop veemeeter`)
    and every VeeMeeter device disappears from PipeWire. Nothing is left
    behind except your `config.toml`.
