{
  lib,
  rustPlatform,
  installShellFiles,
}:
rustPlatform.buildRustPackage {
  pname = "aes";
  version = (lib.importTOML ./Cargo.toml).workspace.package.version;

  src = lib.cleanSource ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  nativeBuildInputs = [ installShellFiles ];

  doCheck = true;

  meta = {
    description = "Toolchain for the .aes fine-grained Aegis schema language";
    homepage = "https://github.com/aegis-run/aes";
    license = lib.licenses.mit;
    mainProgram = "aes";
  };
}
