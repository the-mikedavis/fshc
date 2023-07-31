{
  description = "A language server for Erlang.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        inherit (pkgs) lib stdenv;
        craneLib = crane.mkLib pkgs;
        commonArgs =
          {
            src = ./.;
            doCheck = false;
            nativeBuildInputs = [] ++ (lib.optional stdenv.isDarwin pkgs.libiconv);
          };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in {
        packages = {
          fshc = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
          default = self.packages.${system}.fshc;
        };

        overlays.default = final: prev: {
          inherit (self.packages.${system}) fshc;
        };

        checks = {
          clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          fmt = craneLib.cargoFmt commonArgs;

          doc = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
            });

          test = craneLib.cargoTest (commonArgs
            // {
              inherit cargoArtifacts;
            });
        };

        devShells.default = pkgs.mkShell {
          RUST_BACKTRACE = "1";
          inputsFrom = builtins.attrValues self.checks.${system};
          nativeBuildInputs = with pkgs;
            [lld_13 cargo-flamegraph rust-analyzer]
              ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) pkgs.cargo-tarpaulin)
              ++ (lib.optional stdenv.isLinux pkgs.lldb)
              ++ (lib.optional stdenv.isDarwin pkgs.darwin.apple_sdk.frameworks.CoreFoundation);
        };
      }
    );
}
