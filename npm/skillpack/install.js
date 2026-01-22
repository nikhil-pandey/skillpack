const fs = require("fs");
const path = require("path");
const https = require("https");

const repo = "nikhil-pandey/skillpack";
const version = process.env.npm_package_version;

if (!version) {
  console.error("skillpack: missing npm_package_version");
  process.exit(1);
}

const target = (() => {
  if (process.platform === "darwin" && process.arch === "x64") {
    return "x86_64-apple-darwin";
  }
  if (process.platform === "darwin" && process.arch === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (process.platform === "linux" && process.arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }
  if (process.platform === "win32" && process.arch === "x64") {
    return "x86_64-pc-windows-msvc";
  }
  return null;
})();

if (!target) {
  console.error(`skillpack: unsupported ${process.platform}/${process.arch}`);
  console.error("skillpack: build from source with cargo install --path crates/skillpack");
  process.exit(1);
}

const ext = process.platform === "win32" ? ".exe" : "";
const asset = `sp-${target}${ext}`;
const url = `https://github.com/${repo}/releases/download/v${version}/${asset}`;
const binDir = path.join(__dirname, "bin");
const binPath = path.join(binDir, `sp${ext}`);
const wrapperPath = path.join(binDir, "sp.js");

fs.mkdirSync(binDir, { recursive: true });
if (process.platform !== "win32" && fs.existsSync(wrapperPath)) {
  fs.chmodSync(wrapperPath, 0o755);
}

function download(currentUrl, redirectsLeft) {
  return new Promise((resolve, reject) => {
    https
      .get(currentUrl, (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          if (redirectsLeft === 0) {
            reject(new Error("too many redirects"));
            return;
          }
          res.resume();
          resolve(download(res.headers.location, redirectsLeft - 1));
          return;
        }

        if (res.statusCode !== 200) {
          reject(new Error(`download failed: ${res.statusCode}`));
          res.resume();
          return;
        }

        const file = fs.createWriteStream(binPath, { mode: 0o755 });
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
        file.on("error", reject);
      })
      .on("error", reject);
  });
}

(async () => {
  try {
    await download(url, 3);
    if (process.platform !== "win32") {
      fs.chmodSync(binPath, 0o755);
    }
    console.log(`skillpack: installed ${binPath}`);
  } catch (err) {
    console.error(`skillpack: ${err.message}`);
    process.exit(1);
  }
})();
