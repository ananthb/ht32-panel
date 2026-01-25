{ pkgs, lib, config, inputs, ... }:

{
  # Environment variables
  env.RUST_BACKTRACE = "1";
  env.RUST_LOG = "info";

  # System packages
  packages = with pkgs; [
    # Build tools
    pkg-config
    cmake

    # Hardware libraries
    hidapi
    udev
    systemd

    # D-Bus and GTK for applet
    dbus
    glib
    gtk3
    libappindicator-gtk3

    # Development tools
    just
    watchexec
    cargo-watch
    cargo-nextest
    cargo-audit
    cargo-outdated
  ];

  # Rust language support
  languages.rust = {
    enable = true;
    channel = "stable";
    version = "latest";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
  };

  # Development scripts/commands
  scripts = {
    # Build commands
    build.exec = ''
      echo "Building all crates..."
      cargo build --workspace
    '';

    build-release.exec = ''
      echo "Building release..."
      cargo build --workspace --release
    '';

    # Test commands
    test.exec = ''
      echo "Running tests..."
      cargo nextest run --workspace
    '';

    test-unit.exec = ''
      echo "Running unit tests..."
      cargo nextest run --workspace --lib
    '';

    # Lint commands
    lint.exec = ''
      echo "Running clippy..."
      cargo clippy --workspace --all-targets -- -D warnings
    '';

    fmt.exec = ''
      echo "Formatting code..."
      cargo fmt --all
    '';

    fmt-check.exec = ''
      echo "Checking formatting..."
      cargo fmt --all -- --check
    '';

    # Hardware commands (requires device)
    lcd-test.exec = ''
      echo "Testing LCD (requires hardware)..."
      cargo run -p ht32-panel-cli -- lcd info
    '';

    led-test.exec = ''
      echo "Testing LED (requires hardware)..."
      cargo run -p ht32-panel-cli -- led set rainbow --intensity 3 --speed 3
    '';

    # Daemon commands
    daemon.exec = ''
      echo "Starting daemon..."
      cargo run -p ht32-panel-daemon -- config/default.toml
    '';

    daemon-watch.exec = ''
      echo "Starting daemon with auto-reload..."
      cargo watch -x "run -p ht32-panel-daemon -- config/default.toml"
    '';

    # Security audit
    audit.exec = ''
      echo "Running security audit..."
      cargo audit
    '';

    # Check for outdated dependencies
    outdated.exec = ''
      echo "Checking for outdated dependencies..."
      cargo outdated -R
    '';

    # Clean build artifacts
    clean.exec = ''
      echo "Cleaning build artifacts..."
      cargo clean
    '';

    # CI simulation
    ci.exec = ''
      echo "Running CI checks locally..."
      fmt-check
      lint
      test
      build
    '';
  };

  # Shell hook
  enterShell = ''
    echo ""
    echo "🔧 HT32 Panel Development Environment"
    echo ""
    echo "Available commands:"
    echo "  build         - Build all crates"
    echo "  build-release - Build release binaries"
    echo "  test          - Run all tests"
    echo "  lint          - Run clippy linter"
    echo "  fmt           - Format code"
    echo "  daemon        - Start the daemon (includes HTMX web UI)"
    echo "  daemon-watch  - Start daemon with auto-reload"
    echo "  ci            - Run full CI checks locally"
    echo ""
    echo "Hardware commands (requires device):"
    echo "  lcd-test      - Test LCD connection"
    echo "  led-test      - Test LED strip"
    echo ""
  '';

  # Pre-commit hooks
  git-hooks.hooks = {
    # Rust formatting
    rustfmt = {
      enable = true;
      entry = "${config.languages.rust.toolchain.cargo}/bin/cargo fmt --all -- --check";
      pass_filenames = false;
    };

    # Rust linting
    clippy = {
      enable = true;
      entry = "${config.languages.rust.toolchain.cargo}/bin/cargo clippy --workspace --all-targets -- -D warnings";
      pass_filenames = false;
    };

    # Check for merge conflicts
    check-merge-conflicts.enable = true;

    # Check config file syntax
    check-toml.enable = true;
    check-yaml.enable = true;
    check-json.enable = true;

    # Detect secrets
    detect-private-keys.enable = true;

    # Whitespace
    trim-trailing-whitespace.enable = true;
    end-of-file-fixer.enable = true;
  };

  # Test configuration
  enterTest = ''
    echo "Running test suite..."
    cargo nextest run --workspace
  '';
}
