{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
        {
          defaultPackage = pkgs.rustPlatform.buildRustPackage {
            pname = "prololo-reborn";
            version = "0.1.0";

            src = self;

            cargoSha256 = "sha256-dM3lNanFcm/9QggXgQ6MTjPxF57fxeKfR0lSXGv9bVY=";

            meta = with pkgs.lib; {
              homepage = "https://github.com/prologin/prololo-reborn";
              license = with licenses; [ mit asl20 ];
              platforms = platforms.unix;
            };

            nativeBuildInputs = with pkgs; [ cmake pkg-config ];
            buildInputs = with pkgs; [ openssl ];
          };

          defaultApp = flake-utils.lib.mkApp {
            drv = self.defaultPackage."${system}";
          };

          devShell = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo
              clippy
              nixpkgs-fmt
              rust-analyzer
              rustPackages.clippy
              rustc
              rustfmt

              cmake
              openssl
              pkg-config
            ];

            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          };
        });
}
