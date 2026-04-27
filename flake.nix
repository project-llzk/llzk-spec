{
  inputs = {
    llzk-pkgs.url = "github:project-llzk/llzk-nix-pkgs";
    nixpkgs.follows = "llzk-pkgs/nixpkgs";
    flake-utils.follows = "llzk-pkgs/flake-utils";
    llzk-rs-pkgs = {
      url = "git+https://github.com/project-llzk/llzk-rs";
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

  outputs = {
      self,
      nixpkgs,
      flake-utils,
      release-helpers,
      llzk-pkgs,
      llzk-lib,
      llzk-rs-pkgs,
      rust-overlay,
    }:
    {
      overlays.default = final: prev:
        let
          mlirVersion = final.llzk-llvmPackages.mlir.version;
          _ = assert final.llzk-llvmPackages.libllvm.version == mlirVersion; null;
          mlir-with-llvm = final.symlinkJoin {
            name = "mlir-with-llvm-${mlirVersion}";
            paths = [
              final.llzk-llvmPackages.libllvm.dev
              final.llzk-llvmPackages.libllvm.lib
              final.llzk-llvmPackages.mlir.dev
              final.llzk-llvmPackages.mlir.lib
            ];
            nativeBuildInputs = final.lib.optionals final.stdenv.isDarwin [ final.rcodesign ];
            postBuild = ''
              out="${placeholder "out"}"
              llvm_config="$out/bin/llvm-config"
              llvm_config_original="$out/bin/llvm-config-native"
              cp -L "$llvm_config" "$llvm_config_original"
              rm "$llvm_config"
              ${final.lib.optionalString final.stdenv.isDarwin ''
                chmod +w "$llvm_config_original"
                rcodesign sign "$llvm_config_original"
              ''}
              substitute ${./nix/llvm-config.sh.in} "$llvm_config" \
                --subst-var-by out "$out" \
                --subst-var-by originalTool "$llvm_config_original"
              chmod +x "$llvm_config"
              rm -f "$out/lib/libMLIR.${if final.stdenv.isDarwin then "dylib" else "so"}"
              ${final.stdenv.cc}/bin/ar -r "$out/lib/libMLIR.a"
            '';
          };
          createFileCheckSymlink = ''
            mkdir -p $PWD/build-tools
            ln -sf "${final.llzk-llvmPackages.llvm}/bin/FileCheck" $PWD/build-tools/FileCheck
            export PATH="$PWD/build-tools:$PATH"
          '';
          llzkSharedEnvironment = {
            inherit createFileCheckSymlink;
            nativeBuildInputs = with final; [ cmake llzk-llvmPackages.clang ];
            buildInputs = with final; [ libxml2 zlib zstd z3.lib llzk-llvmPackages.libclang.dev ];
            devBuildInputs = with final; [ git ] ++ llzkSharedEnvironment.buildInputs;
            env = {
              CC = "clang";
              CXX = "clang++";
              MLIR_SYS_200_PREFIX = "${mlir-with-llvm}";
              TABLEGEN_200_PREFIX = "${mlir-with-llvm}";
              LLZK_SYS_10_PREFIX = "${final.llzk}";
              LIBCLANG_PATH = "${final.llzk-llvmPackages.libclang.lib}/lib";
              RUST_BACKTRACE = "1";
            };
            pkgSettings = {
              RUSTFLAGS = "-lLLVM -L ${mlir-with-llvm}/lib";
              CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL = "2";
              LLZK_SYS_ENABLE_WHOLE_ARCHIVE = "1";
              CARGO_INCREMENTAL = "1";
            };
            devSettings = {
              RUSTFLAGS = "-lLLVM -L ${mlir-with-llvm}/lib";
              RUST_SRC_PATH = final.rustPlatform.rustLibSrc;
              NIX_CFLAGS_COMPILE = " -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0";
              LLZK_SYS_ENABLE_WHOLE_ARCHIVE = "1";
            };
          };
        in {
          inherit mlir-with-llvm llzkSharedEnvironment;
          llzk-spec = final.rustPlatform.buildRustPackage ({
            pname = "llzk-spec";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = final.llzkSharedEnvironment.nativeBuildInputs;
            buildInputs = final.llzkSharedEnvironment.devBuildInputs;
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
            cargoBuildFlags = [ "--package" "llzk-spec" ];
            cargoTestFlags = [ "--package" "llzk-spec" ];
            dontUsePytestCheck = true;
            preBuild = createFileCheckSymlink;
          } // final.llzkSharedEnvironment.env // final.llzkSharedEnvironment.pkgSettings);
        };
    } // flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
            self.overlays.default
            llzk-pkgs.overlays.default
            llzk-lib.overlays.default
            release-helpers.overlays.default
          ];
        };
      in {
        packages = flake-utils.lib.flattenTree {
          inherit (pkgs) llzk llzk-debug;
          inherit (pkgs) mlir mlir-debug;
          inherit (pkgs) changelogCreator;
          inherit (pkgs) rust-bin;
          inherit (pkgs.llzk-llvmPackages) libllvm llvm;
          inherit (pkgs) mlir-with-llvm llzk-spec;
          default = pkgs.llzk-spec;
        };

        devShells = flake-utils.lib.flattenTree {
          default = pkgs.mkShell ({
            nativeBuildInputs = pkgs.llzkSharedEnvironment.nativeBuildInputs;
            buildInputs = pkgs.llzkSharedEnvironment.devBuildInputs ++ [
              pkgs.rust-bin.stable.latest.default
              pkgs.pre-commit
            ];
            shellHook = ''
              set -uo pipefail
              ${pkgs.llzkSharedEnvironment.createFileCheckSymlink}
              # set up pre-commit
              pre-commit install

              echo "Welcome to the llzk-spec devshell!"
              echo "To commit without pre-commit hooks, use \`git commit --no-verify\`"
            '';
          } // pkgs.llzkSharedEnvironment.env // pkgs.llzkSharedEnvironment.devSettings);
        };
      }
    );
}
