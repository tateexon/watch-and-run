{ pkgs}:
with pkgs;
let
  # libPath = with pkgs; lib.makeLibraryPath [
    # load external libraries that you need in your rust project here
  # ];
in
mkShell {
  nativeBuildInputs = [
    # basics
    bash
    git
    curl
    gnumake
    jq
    dasel
    github-cli

    # rust
    rustc
    cargo
    gcc
    rustfmt
    clippy
    rust-analyzer

    # linting tools
    typos
    pre-commit
    python3
    shfmt
    shellcheck
  ];

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  shellHook = ''
    # Uninstall pre-commit hooks in case they get messed up
    pre-commit uninstall > /dev/null || true
    pre-commit uninstall --hook-type pre-push > /dev/null || true

    # enable pre-commit hooks
    pre-commit install > /dev/null
    pre-commit install -f --hook-type pre-push > /dev/null
  '';
}
