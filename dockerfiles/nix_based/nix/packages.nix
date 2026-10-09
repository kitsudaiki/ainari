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

# Toolchains and runtime-environments of the docker-images. All packages come from the nixpkgs of
# flake.lock. Packages, which are not taken as they are from there, are defined in the first
# section together with the reason.
{ pkgs }:

let
  inherit (pkgs) lib;

  # ==========================================
  # packages, which differ from nixpkgs
  # ==========================================

  # toolchain of all rust-components except the torii
  rustStable = pkgs.rust-bin.stable."1.98.1".minimal;

  # The eBPF-programs of the torii are built with the nightly-toolchain, because core has to be
  # compiled for the bpf-target (build-std), which also needs the sources of the standard-library.
  # Without rustup aya-build compiles them with the toolchain of the torii itself, so the whole
  # torii is built with this toolchain. The LLVM of this nightly has to be the same major version
  # as the one of the bpf-linker below, because the bpf-linker reads the bitcode of rustc.
  rustNightly = pkgs.rust-bin.nightly."2026-09-28".minimal.override {
    extensions = [ "rust-src" ];
  };
  bpfLlvmPackages = pkgs.llvmPackages_23;

  # nixpkgs only has bpf-linker 0.9.15, which supports at most LLVM 21
  bpf-linker = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
    pname = "bpf-linker";
    version = "0.11.1";

    src = pkgs.fetchFromGitHub {
      owner = "aya-rs";
      repo = "bpf-linker";
      tag = "v${finalAttrs.version}";
      hash = "sha256-qo5cJBZQHsbKSL+6YA0vcmPCo76pg2soekWziysXTDM=";
    };

    cargoHash = "sha256-uiJQZ4U3lW2Vs1Dbb1v+eIgSNaLnd5MKpZi0O8KwGRE=";

    buildNoDefaultFeatures = true;
    buildFeatures = [ "llvm-${lib.versions.major bpfLlvmPackages.llvm.version}" ];

    nativeBuildInputs = [ bpfLlvmPackages.llvm ];
    buildInputs = [
      pkgs.zlib
      pkgs.libxml2
      (lib.getLib bpfLlvmPackages.llvm)
    ];

    # the tests need the clang and btfdump of the same LLVM-version
    doCheck = false;

    meta.mainProgram = "bpf-linker";
  });

  # The client-library 3.3 of MariaDB returns wrong results to diesel, but nixpkgs only has 3.3, so
  # 3.4 is built with the recipe of nixpkgs. The bindings, which are selected at build-time, and
  # the library at runtime are always the same, because the toolchain and the images both take it
  # from here.
  mariadb-connector-c =
    # The recipe is taken as path out of the nixpkgs, which is already in the store. A string like
    # "${nixpkgs}/pkgs/..." would copy the whole nixpkgs into the store a second time.
    (pkgs.callPackage (pkgs.path + "/pkgs/servers/sql/mariadb/connector-c") {
      version = "3.4.11";
      hash = "sha256-8H+r45drPNKVgmgbQHFfPestlR6Spsdpb7FERoiHTO4=";
    }).overrideAttrs
      (old: {
        # The recipe replaces the paths of mariadb_config by fixed strings, so the arguments of
        # their printf-calls are no longer used, which 3.4 treats as an error.
        env.NIX_CFLAGS_COMPILE = "-Wno-error=format-extra-args";
        # Since 3.4 the certificate of the server is verified by default. The services don't get
        # the CA of the mysql-server, which uses a self-signed certificate, so they can't verify
        # it and fail to connect. Like the library of Debian, the connection is encrypted, but the
        # certificate is not verified.
        cmakeFlags = old.cmakeFlags ++ [ "-DDEFAULT_SSL_VERIFY_SERVER_CERT=OFF" ];
      });

  # the hypervisor of sakura, nixpkgs has only 52.0
  cloud-hypervisor = pkgs.cloud-hypervisor.overrideAttrs (
    finalAttrs: old: {
      version = "53.0";
      src = pkgs.fetchFromGitHub {
        owner = "cloud-hypervisor";
        repo = "cloud-hypervisor";
        rev = "v${finalAttrs.version}";
        hash = "sha256-fPTGf8bAITDA8QwllWbbGXA7tJ6p/SxRDfcBQVRvCTI=";
      };
      cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
        inherit (finalAttrs) pname version src;
        hash = "sha256-+RbW/9ap/69MyODUk/bHBlH6ZuqYYIyKaarYSMQ2G7w=";
      };
      # since 53.0 openssl is also needed for the build and not only for the tests
      buildInputs = old.buildInputs ++ [ pkgs.openssl ];
      # the tests need /dev/kvm and /dev/net/tun
      doCheck = false;
    }
  );

  # firmware, which boots the virtual machines of cloud-hypervisor
  cloudHypervisorFirmware = "${pkgs.OVMF-cloud-hypervisor.fd}/FV/${
    if pkgs.stdenv.hostPlatform.isAarch64 then "CLOUDHV_EFI.fd" else "CLOUDHV.fd"
  }";

  # wg-quick sets 'net.ipv4.conf.all.src_valid_mark', which is not permitted within a container
  # and lets wg-quick fail. The setting is only needed for a default-route through the tunnel.
  wireguard-tools = pkgs.wireguard-tools.overrideAttrs (old: {
    postPatch = (old.postPatch or "") + ''
      grep -q src_valid_mark wg-quick/linux.bash
      sed -i '/src_valid_mark/d' wg-quick/linux.bash
    '';
  });

  # The images only have a rule without password for wg-quick, so sudo is built
  # without PAM, which would otherwise need its own configuration within the images.
  sudo =
    (pkgs.sudo.override {
      pam = null;
    }).overrideAttrs
      (old: {
        configureFlags = old.configureFlags ++ [ "--without-pam" ];
      });

  # ==========================================
  # runtime-environments
  # ==========================================

  # Creates the environment of an image. The executables of the environment are linked into /usr/bin
  # of the image and its configs into /etc by dockerfiles/nix_based/nix/make_rootfs.sh, which also
  # copies the whole closure of the environment into the image. Packages, which have nothing in /bin
  # or /etc, like the libraries, are listed in a file of the environment, so they are part of its
  # closure too.
  mkRuntime =
    name: paths:
    pkgs.buildEnv {
      name = "ainari-runtime-${name}";
      paths = basePackages ++ paths;
      pathsToLink = [
        "/bin"
        "/etc"
      ];
      postBuild = ''
        mkdir -p $out/share
        printf '%s\n' ${lib.escapeShellArgs (basePackages ++ paths)} > $out/share/ainari-packages
      '';
    };

  # Shell and basic tools, which every image has for the start-scripts and for debugging. These
  # are the tools, which the images of Debian and Ubuntu brought before, because the scripts of
  # the operator and the docker-compose-setup use them (like getent and awk in start_torii.sh of
  # deploy/operator/internal/render/templates/start_torii.sh).
  basePackages = with pkgs; [
    bashInteractive
    coreutils
    findutils
    gnugrep
    gnused
    gawk
    diffutils
    which
    gnutar
    gzip
    getent
    hostname-debian
    procps
    util-linuxMinimal
    cacert
    iana-etc
  ];

  # libraries, which the rust-components are linked against
  serviceLibraries = [
    pkgs.stdenv.cc.cc.lib
    pkgs.openssl.out
    pkgs.sqlite.out
  ];

  # tools, which the services with a wireguard-connection need
  wireguardPackages = [
    wireguard-tools
    pkgs.iptables
    pkgs.iproute2
    sudo
  ];

  # the dashboard is served by nginx with the same settings like in the official nginx-image
  dashboardNginxConfig = pkgs.writeTextDir "etc/nginx/nginx.conf" ''
    user nginx;
    worker_processes auto;

    error_log /dev/stderr notice;
    pid /run/nginx.pid;

    events {
        worker_connections 1024;
    }

    http {
        include ${pkgs.nginx}/conf/mime.types;
        default_type application/octet-stream;

        access_log /dev/stdout;
        sendfile on;
        keepalive_timeout 65;

        server {
            listen 80;
            listen [::]:80;
            server_name localhost;

            location / {
                root /usr/share/nginx/html;
                index index.html index.htm;
            }
        }
    }
  '';


in
{
  packages = {
    runtime-miko = mkRuntime "miko" (
      serviceLibraries
      ++ [
        pkgs.curl
        pkgs.openssl
        mariadb-connector-c
      ]
    );
    runtime-omamori = mkRuntime "omamori" (
      serviceLibraries
      ++ [
        pkgs.curl
        pkgs.openssl
        mariadb-connector-c
      ]
    );
    runtime-izakaya = mkRuntime "izakaya" (
      serviceLibraries
      ++ [
        pkgs.curl
        pkgs.openssl
        mariadb-connector-c
      ]
    );
    runtime-hanami = mkRuntime "hanami" (
      serviceLibraries
      ++ [
        pkgs.curl
        pkgs.openssl
        mariadb-connector-c
      ]
    );
    runtime-ryokan = mkRuntime "ryokan" (
      serviceLibraries
      ++ wireguardPackages
      ++ [
        pkgs.curl
        pkgs.openssl
        mariadb-connector-c
      ]
    );
    runtime-onsen = mkRuntime "onsen" (serviceLibraries ++ wireguardPackages ++ [ pkgs.openssl ]);
    runtime-sakura = mkRuntime "sakura" (
      serviceLibraries
      ++ wireguardPackages
      ++ [
        pkgs.curl
        pkgs.openssl
        # qemu-img converts a downloaded image into the boot-disk of a virtual machine
        pkgs.qemu-utils
        # cloud-localds builds the cloud-init seed-image
        pkgs.cloud-utils
        # setcap is only used while building the image
        pkgs.libcap
        cloud-hypervisor
        (pkgs.runCommand "cloud-hypervisor-firmware" { } ''
          mkdir -p $out/etc/cloud-hypervisor
          ln -s ${cloudHypervisorFirmware} $out/etc/cloud-hypervisor/CLOUDHV.fd
        '')
      ]
    );
    runtime-torii = mkRuntime "torii" (
      serviceLibraries
      ++ (with pkgs; [
        iproute2
        openssh
        ethtool
        iputils
      ])
    );
    runtime-dashboard = mkRuntime "dashboard" [
      pkgs.nginx
      dashboardNginxConfig
    ];
    runtime-docs = mkRuntime "docs" [ pkgs.python3 ];
    # The operator is a static go-binary, which needs no library. The go of nixpkgs reads the
    # time-zones and the mime-types out of the paths of tzdata and mailcap within the store, which
    # are compiled into the binary, so both are part of the image.
    runtime-operator = mkRuntime "operator" [
      pkgs.tzdata
      pkgs.mailcap
    ];
  };

  devShells = {
    # toolchain of all rust-components except the torii (dockerfiles/nix_based/Dockerfile_services)
    services = pkgs.mkShell {
      packages = [
        rustStable
        # removes the libraries, which are not needed, out of the RUNPATH of the binaries, see
        # dockerfiles/nix_based/Dockerfile_services
        pkgs.patchelf
        pkgs.protobuf
        pkgs.pkg-config
        pkgs.git
      ];
      buildInputs = [
        pkgs.openssl
        pkgs.sqlite
        mariadb-connector-c
      ];
    };

    # toolchain of the torii (dockerfiles/nix_based/Dockerfile_torii)
    torii = pkgs.mkShell {
      packages = [
        rustNightly
        bpf-linker
        # removes the paths of the toolchain out of the torii, see
        # dockerfiles/nix_based/Dockerfile_torii
        pkgs.removeReferencesTo
        pkgs.patchelf
        pkgs.protobuf
        pkgs.pkg-config
        pkgs.git
      ];
      buildInputs = [
        pkgs.openssl
        pkgs.sqlite
      ];
    };

    # builds the dashboard (dockerfiles/nix_based/Dockerfile_dashboard)
    dashboard = pkgs.mkShell {
      packages = [ pkgs.nodejs_24 ];
    };

    # toolchain of the kubernetes-operator (dockerfiles/nix_based/Dockerfile_operator)
    operator = pkgs.mkShell {
      packages = [ pkgs.go ];
    };

    # builds the documentation (dockerfiles/nix_based/Dockerfile_docs)
    docs = pkgs.mkShell {
      packages = [ pkgs.zensical ];
    };

    # generates the SBOMs of the runtime-environments (scripts/generate_sbom.sh)
    sbom = pkgs.mkShell {
      packages = [ pkgs.sbomnix ];
    };
  };
}
