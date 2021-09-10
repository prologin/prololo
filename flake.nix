{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nix-community/naersk";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, naersk, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        myRust = pkgs.rust-bin.stable.latest.default;
        naersk-lib = naersk.lib."${system}".override {
          cargo = myRust;
          rustc = myRust;
        };
      in
        {
          defaultPackage = naersk-lib.buildPackage {
            pname = "prololo-reborn";
            version = "0.1.0";

            src = self;

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
              nixpkgs-fmt
              rust-analyzer
              myRust

              cmake
              openssl
              pkg-config
            ];

            RUST_SRC_PATH = pkgs.rust-bin.stable.latest.rust-std;
          };
        });
}
