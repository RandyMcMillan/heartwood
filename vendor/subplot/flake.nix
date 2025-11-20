# This is a nix "flake" which is used both by `direnv` and to offer
# a quick way to acquire `subplot` in a nix/nixos environment.
# If you have `direnv` support, just allow this tree and you should
# be able to develop `subplot`.  If you use this as a package flake
# then the `subplot` package derivation is available in the usual way

{
  inputs = { flake-utils.url = "github:numtide/flake-utils"; };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        test-python-packages = python-packages:
          with python-packages; [
            requests
            flake8
          ];

      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            stdenv
            graphviz
            plantuml
            daemonize
            librsvg
            (python3.withPackages test-python-packages)
            black
            html-tidy
          ];
          SUBPLOT_DOT_PATH = "${pkgs.graphviz}/bin/dot";
          SUBPLOT_JAVA_PATH = "${pkgs.jre}/bin/java";
          SUBPLOT_PLANTUML_JAR_PATH = "${pkgs.plantuml}/lib/plantuml.jar";
        };
      });
}
