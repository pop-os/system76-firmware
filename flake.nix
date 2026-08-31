{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      fenix,
      nixpkgs,
      utils,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            fenix.packages.${system}.complete.toolchain
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.openssl
            pkgs.xz
            pkgs.dbus
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.openssl
            pkgs.xz
            pkgs.dbus
          ];
        };
      }
    );
}
