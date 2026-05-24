const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

// Paths
const projectRoot = path.resolve(__dirname, '..');
const packageJsonPath = path.join(projectRoot, 'package.json');
const tauriConfPath = path.join(projectRoot, 'src-tauri', 'tauri.conf.json');
const cargoTomlPath = path.join(projectRoot, 'src-tauri', 'Cargo.toml');

function getArgs() {
  const args = process.argv.slice(2);
  const options = {
    bump: true,
    version: null,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--no-bump' || args[i] === '--skip-bump' || args[i] === '--nobump') {
      options.bump = false;
    } else if (args[i] === '--version' && args[i + 1]) {
      options.version = args[i + 1];
      i++;
    }
  }
  return options;
}

function bumpVersion() {
  const options = getArgs();
  
  // 1. Read package.json
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const currentVersion = packageJson.version;
  let targetVersion = currentVersion;

  if (options.version) {
    targetVersion = options.version;
    console.log(`Setting explicit version: ${targetVersion}`);
  } else if (options.bump) {
    const match = currentVersion.match(/^(\d+)\.(\d+)\.(\d+)(.*)$/);
    if (!match) {
      console.error(`Error: version "${currentVersion}" is not in SemVer format.`);
      process.exit(1);
    }
    const major = parseInt(match[1], 10);
    const minor = parseInt(match[2], 10);
    const patch = parseInt(match[3], 10);
    const rest = match[4] || '';
    targetVersion = `${major}.${minor}.${patch + 1}${rest}`;
    console.log(`Bumping version from ${currentVersion} to ${targetVersion}...`);
  } else {
    console.log(`Building with current version: ${currentVersion} (no bump)`);
  }

  // 2. Update package.json
  if (packageJson.version !== targetVersion) {
    packageJson.version = targetVersion;
    fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n', 'utf8');
    console.log(`Updated package.json to version ${targetVersion}`);
  }

  // 3. Update tauri.conf.json
  if (fs.existsSync(tauriConfPath)) {
    const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
    if (tauriConf.version !== targetVersion) {
      tauriConf.version = targetVersion;
      fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n', 'utf8');
      console.log(`Updated src-tauri/tauri.conf.json to version ${targetVersion}`);
    }
  }

  // 4. Update Cargo.toml
  if (fs.existsSync(cargoTomlPath)) {
    let cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
    // Match version under [package]
    const updatedCargoToml = cargoToml.replace(/(\[package\][\s\S]*?^version\s*=\s*")([^"]+)(")/m, `$1${targetVersion}$3`);
    if (cargoToml !== updatedCargoToml) {
      fs.writeFileSync(cargoTomlPath, updatedCargoToml, 'utf8');
      console.log(`Updated src-tauri/Cargo.toml to version ${targetVersion}`);
    }
  }

  return targetVersion;
}

function build() {
  const version = bumpVersion();
  console.log(`Starting Tauri build for version ${version}...`);
  
  const buildResult = spawnSync('npx', ['tauri', 'build'], {
    stdio: 'inherit',
    shell: true,
    cwd: projectRoot
  });

  if (buildResult.status !== 0) {
    console.error(`Build failed with exit code ${buildResult.status}`);
    process.exit(buildResult.status || 1);
  }

  console.log(`Build completed successfully for version ${version}!`);
}

build();
