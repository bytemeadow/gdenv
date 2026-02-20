{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [ pkgs.git ];

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # Enable Rust via devenv
  languages.rust = {
    enable = true;
    channel = "stable";
  };

  # Scripts
  scripts = {
    check.exec = ''
      echo "Running cargo check..."
      cargo check --all-targets --all-features
    '';

    lint.exec = ''
      echo "Running cargo fmt and clippy..."
      cargo fmt --all
      cargo clippy --all-targets --all-features
    '';

    run.exec = ''
      echo "Running cargo run..."
      cargo run -- "''$@"
    '';
  };

  enterShell = ''
    echo "gdenv development environment"
    echo "Rust: $(rustc --version)"
    echo ""
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
