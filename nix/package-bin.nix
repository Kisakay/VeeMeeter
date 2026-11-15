# VeeMeeter déjà compilé par la CI (archive Linux de la dernière release,
# construite sur Debian), adapté à NixOS par patchelf : rien n'est compilé.
# `nix/bin.json` (version, url, hash) est écrit par la CI à chaque release.
{ lib
, stdenv
, fetchurl
, autoPatchelfHook
, makeWrapper
, pipewire
, libxkbcommon
, wayland
, fontconfig
, freetype
, expat
, rnnoise-plugin
, pkgs
, withRnnoise ? true
, bin ? lib.importJSON ./bin.json
}:

let
  runtimeLibs = import ./runtime-libs.nix pkgs;
in
stdenv.mkDerivation {
  pname = "veemeeter-bin";
  inherit (bin) version;

  src = fetchurl { inherit (bin) url hash; };
  sourceRoot = ".";

  nativeBuildInputs = [ autoPatchelfHook makeWrapper ];
  # Bibliothèques liées par le binaire, résolues par autoPatchelfHook.
  buildInputs = [ pipewire libxkbcommon wayland fontconfig freetype expat stdenv.cc.cc.lib ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r usr/* $out/
    substituteInPlace $out/lib/systemd/user/veemeeter.service \
      --replace-fail "/usr/bin/veemeeter" "$out/bin/veemeeter"
    runHook postInstall
  '';

  # Bibliothèques chargées à l'exécution (dlopen) : Vulkan, Wayland, X11…
  postFixup = ''
    wrapProgram $out/bin/veemeeter \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs} ${lib.optionalString withRnnoise "--suffix LADSPA_PATH : ${rnnoise-plugin}/lib/ladspa"}
  '';

  meta = {
    description = "Native PipeWire audio mixer and router (prebuilt by the project's CI)";
    homepage = "https://github.com/Kisakay/VeeMeeter";
    license = lib.licenses.gpl3Only;
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    mainProgram = "veemeeter";
  };
}
