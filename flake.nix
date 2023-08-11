{
  description = "Memoradical: A no-frills local-only notecard app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

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
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          # Set the build targets supported by the toolchain,
          # wasm32-unknown-unknown is required for trunk
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # When filtering sources, we want to allow assets other than .rs files
        src = lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (lib.hasSuffix "\.html" path) ||
            (lib.hasInfix "/static/" path) ||
            (craneLib.filterCargoSources path type)
          ;
        };

        commonArgs = {
          inherit src;
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          RUSTFLAGS = "--cfg=web_sys_unstable_apis";
        };

        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          # You cannot run cargo test on a wasm build
          doCheck = false;
        });

        memoradical = craneLib.buildTrunkPackage (commonArgs // {
          inherit cargoArtifacts;
        });

        serve-memoradical = pkgs.writeShellScriptBin "serve-memoradical" ''
          ${pkgs.python3Minimal}/bin/python3 -m http.server --directory ${memoradical} 8000
        '';
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit memoradical;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          memoradical-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Check formatting
          memoradical-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        packages.deps = cargoArtifacts;
        packages.default = memoradical;

        apps.default = flake-utils.lib.mkApp {
          drv = serve-memoradical;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks;
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            trunk
          ];
        };
      });
}
