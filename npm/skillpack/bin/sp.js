#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");

const ext = process.platform === "win32" ? ".exe" : "";
const binPath = path.join(__dirname, `sp${ext}`);

if (!fs.existsSync(binPath)) {
  console.error("skillpack: binary missing, reinstall package");
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 1));
