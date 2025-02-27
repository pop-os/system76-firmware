{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
       cargo
       dbus
       gnumake
       pkg-config
       openssl
       rustc
       xz
       ];
    shellHook = ''
       export RUST_BACKTRACE=1;
    '';
}
