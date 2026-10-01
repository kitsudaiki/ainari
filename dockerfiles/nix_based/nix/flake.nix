# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License")
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Packages of all docker-images of dockerfiles/. The revisions of nixpkgs and rust-overlay are
# pinned in flake.lock, so every build of an image installs exactly the same packages. The
# versions only change, when flake.lock is updated with 'nix flake update' (see
# docs/developer/docker_images.md).
#
# Every image has a runtime-environment 'runtime-<image>', which contains all packages of the
# image, and the images, which compile something, have a development-shell with their toolchain.
{
  description = "Pinned packages of the docker-images of ainari";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      # the images are built for these platforms, see the matrix of .github/workflows/build_test.yml
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import ./packages.nix {
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
              };
            }
          )
        );
    in
    {
      packages = forAllSystems (set: set.packages);
      devShells = forAllSystems (set: set.devShells);
    };
}
