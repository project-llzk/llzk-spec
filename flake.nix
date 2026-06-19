{
  inputs = {
    llzk-pkgs.url = "github:project-llzk/llzk-nix-pkgs";
    nixpkgs.follows = "llzk-pkgs/nixpkgs";
    flake-utils.follows = "llzk-pkgs/flake-utils";
    llzk-rs-pkgs = {
      url = "github:project-llzk/llzk-rs?ref=dani/verif-and-quant-ops";
      inputs = {
        nixpkgs.follows = "llzk-pkgs/nixpkgs";
        flake-utils.follows = "llzk-pkgs/flake-utils";
        llzk-pkgs.follows = "llzk-pkgs";
      };
    };
    llzk-lib.follows = "llzk-rs-pkgs/llzk-lib";
    release-helpers.follows = "llzk-rs-pkgs/llzk-lib/release-helpers";
    rust-overlay.follows = "llzk-rs-pkgs/rust-overlay";
  };

  # Custom colored bash prompt
  nixConfig.bash-prompt = "\\[\\e[0;32m\\][llzk-spec]\\[\\e[m\\] \\[\\e[38;5;244m\\]\\w\\[\\e[m\\] % ";

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      release-helpers,
      llzk-pkgs,
      llzk-lib,
      llzk-rs-pkgs,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
            llzk-pkgs.overlays.default
            llzk-lib.overlays.default
            llzk-rs-pkgs.overlays.default
            release-helpers.overlays.default
          ];
        };

        # Lit tests need FileCheck but directly adding the LLVM `bin` dir to the path causes
        # linking problems in `llzk-sys`. Instead, create a symlink in a new directory for the path.
        createFileCheckSymlink = ''
          mkdir -p $PWD/build-tools
          ln -sf "${pkgs.llzk-llvmPackages.llvm}/bin/FileCheck" $PWD/build-tools/FileCheck
          export PATH="$PWD/build-tools:$PATH"
        '';

        llzkSpec = pkgs.rustPlatform.buildRustPackage (
          {
            pname = "llzk-spec";
            version = "0.1.0";
            src = ./.;

            nativeBuildInputs = pkgs.llzkSharedEnvironment.nativeBuildInputs;
            buildInputs = pkgs.llzkSharedEnvironment.devBuildInputs;

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            cargoBuildFlags = [
              "--package"
              "llzk-spec"
            ];
            cargoTestFlags = [
              "--package"
              "llzk-spec"
            ];
            dontUsePytestCheck = true;
            preBuild = createFileCheckSymlink;
          }
          // pkgs.llzkSharedEnvironment.env
          // pkgs.llzkSharedEnvironment.pkgSettings
        );
      in
      {
        packages = flake-utils.lib.flattenTree {
          llzk-spec = llzkSpec;
          default = llzkSpec;
        };

        devShells = flake-utils.lib.flattenTree {
          default = pkgs.mkShell (
            {
              nativeBuildInputs = pkgs.llzkSharedEnvironment.nativeBuildInputs;
              buildInputs = pkgs.llzkSharedEnvironment.devBuildInputs ++ [
                pkgs.changelogCreator
                pkgs.nixfmt-rfc-style
                pkgs.rust-bin.stable.latest.default
                pkgs.pre-commit
              ];

              shellHook = ''
                # Bail out of pipes where any command fails
                set -uo pipefail
                ${createFileCheckSymlink}
                # set up pre-commit
                pre-commit install

                echo "Welcome to the llzk-spec devshell!"
                echo "To commit without pre-commit hooks, use \`git commit --no-verify\`"
              '';
            }
            // pkgs.llzkSharedEnvironment.env
            // pkgs.llzkSharedEnvironment.devSettings
          );
        };
      }
    );
}
