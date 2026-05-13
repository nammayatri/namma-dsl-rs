{
  description = "namma-dsl-rs: YAML specs -> Rust + Diesel ORM storage layer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { system, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.stable.latest.default;

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          namma-dsl-rs = rustPlatform.buildRustPackage {
            pname = "namma-dsl-rs";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkgs.makeWrapper ];

            # The binary shells out to `git` (src/utils.rs) and `cargo fmt`
            # (src/main.rs) at runtime. Bundle them on PATH so the wrapped
            # binary works regardless of the caller's environment.
            postInstall = ''
              wrapProgram $out/bin/namma-dsl-rs \
                --prefix PATH : ${pkgs.lib.makeBinPath [
                  pkgs.git
                  rustToolchain
                ]}
            '';

            meta = with pkgs.lib; {
              description = "Code generator: YAML specs -> Rust + Diesel ORM storage layer";
              license = licenses.agpl3Plus;
              mainProgram = "namma-dsl-rs";
            };
          };
        in
        {
          packages.default = namma-dsl-rs;
          packages.namma-dsl-rs = namma-dsl-rs;

          apps.default = {
            type = "app";
            program = "${namma-dsl-rs}/bin/namma-dsl-rs";
          };

          devShells.default = pkgs.mkShell {
            name = "namma-dsl-rs-shell";
            packages = [
              rustToolchain
              pkgs.git
            ];
          };
        };
    };
}
