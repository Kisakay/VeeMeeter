{ lib
, rustPlatform
, pkg-config
, llvmPackages
, makeWrapper
, pipewire
, rnnoise-plugin
, pkgs
  # Suppression de bruit prête à l'emploi (plugin LADSPA RNNoise).
, withRnnoise ? true
}:

let
  runtimeLibs = import ./runtime-libs.nix pkgs;
in
rustPlatform.buildRustPackage {
  pname = "veemeeter";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = lib.cleanSourceWith {
    src = ../.;
    filter = path: type:
      let base = baseNameOf path; in
      !(lib.elem base [ "target" "result" ".git" ]);
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config rustPlatform.bindgenHook makeWrapper ];
  buildInputs = [ pipewire ];

  postInstall = ''
    install -Dm644 assets/veemeeter.desktop $out/share/applications/veemeeter.desktop
    install -Dm644 assets/veemeeter.svg $out/share/icons/hicolor/scalable/apps/veemeeter.svg
    install -Dm644 assets/veemeeter.service $out/lib/systemd/user/veemeeter.service
    substituteInPlace $out/lib/systemd/user/veemeeter.service \
      --replace-fail "/usr/bin/veemeeter" "$out/bin/veemeeter"
  '';

  postFixup = ''
    wrapProgram $out/bin/veemeeter \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs} ${lib.optionalString withRnnoise "--suffix LADSPA_PATH : ${rnnoise-plugin}/lib/ladspa"}
  '';

  meta = {
    description = "Native PipeWire audio mixer and router (Voicemeeter-like) for Linux";
    homepage = "https://github.com/Kisakay/VeeMeeter";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
    mainProgram = "veemeeter";
  };
}
