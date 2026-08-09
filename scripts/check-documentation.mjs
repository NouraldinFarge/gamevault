import { readFile, readdir, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
const ignoredDirectories = new Set([
  ".git",
  "active-build",
  "cache",
  "coverage",
  "dist",
  "logs",
  "node_modules",
  "output",
  "portable-builds",
  "release",
  "target",
  "temp",
]);
const failures = [];

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(absolute)));
    else files.push(absolute);
  }
  return files;
}

function display(absolute) {
  return path.relative(root, absolute).replaceAll("\\", "/");
}

function localTarget(rawTarget) {
  const withoutTitle = rawTarget.trim().replace(/^<|>$/g, "");
  if (!withoutTitle || withoutTitle.startsWith("#") || /^[a-z][a-z0-9+.-]*:/i.test(withoutTitle)) {
    return null;
  }
  const pathname = withoutTitle.split(/[?#]/, 1)[0];
  try {
    return decodeURIComponent(pathname);
  } catch {
    return pathname;
  }
}

async function checkLocalTarget(file, rawTarget) {
  const target = localTarget(rawTarget);
  if (!target) return;
  const resolved = path.resolve(path.dirname(file), target);
  try {
    await stat(resolved);
  } catch {
    failures.push(`${display(file)}: missing local target ${rawTarget}`);
  }
}

async function checkMarkdown(file) {
  const source = await readFile(file, "utf8");
  const linkPattern = /(!?)\[([^\]]*)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g;
  for (const match of source.matchAll(linkPattern)) {
    const [, imageMarker, label, rawTarget] = match;
    if (imageMarker && !label.trim()) {
      failures.push(`${display(file)}: image ${rawTarget} has empty alternative text`);
    }
    await checkLocalTarget(file, rawTarget);
  }

  for (const match of source.matchAll(/<img\b[^>]*>/gi)) {
    const tag = match[0];
    const src = tag.match(/\bsrc=["']([^"']+)["']/i)?.[1];
    const alt = tag.match(/\balt=["']([^"']*)["']/i)?.[1];
    if (!alt?.trim()) failures.push(`${display(file)}: HTML image is missing alternative text`);
    if (src) await checkLocalTarget(file, src);
  }
}

function pngDimensions(buffer) {
  if (buffer.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a") return null;
  return { width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) };
}

function jpegDimensions(buffer) {
  if (buffer[0] !== 0xff || buffer[1] !== 0xd8) return null;
  const startOfFrameMarkers = new Set([
    0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf,
  ]);
  let offset = 2;
  while (offset + 8 < buffer.length) {
    if (buffer[offset] !== 0xff) {
      offset += 1;
      continue;
    }
    while (buffer[offset] === 0xff) offset += 1;
    const marker = buffer[offset];
    offset += 1;
    if (marker === 0xd9 || marker === 0xda) break;
    if (marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) continue;
    if (offset + 2 > buffer.length) break;
    const length = buffer.readUInt16BE(offset);
    if (length < 2 || offset + length > buffer.length) break;
    if (startOfFrameMarkers.has(marker)) {
      return { width: buffer.readUInt16BE(offset + 5), height: buffer.readUInt16BE(offset + 3) };
    }
    offset += length;
  }
  return null;
}

async function checkMedia() {
  const media = [
    [".github/social-preview.png", 1280, 640, true, "png"],
    ["docs/images/gamevault-home.jpg", 1200, 600, false, "jpeg"],
    ["docs/images/gamevault-library.jpg", 1200, 600, false, "jpeg"],
    ["docs/images/gamevault-local-files.jpg", 1200, 600, false, "jpeg"],
  ];

  for (const [relative, minimumWidth, minimumHeight, exact, type] of media) {
    let dimensions;
    try {
      const buffer = await readFile(path.join(root, relative));
      dimensions = type === "png" ? pngDimensions(buffer) : jpegDimensions(buffer);
    } catch {
      failures.push(`${relative}: required presentation image is missing`);
      continue;
    }
    if (!dimensions) {
      failures.push(`${relative}: expected a valid ${type.toUpperCase()} image`);
      continue;
    }
    const valid = exact
      ? dimensions.width === minimumWidth && dimensions.height === minimumHeight
      : dimensions.width >= minimumWidth && dimensions.height >= minimumHeight;
    if (!valid) {
      failures.push(
        `${relative}: ${dimensions.width}x${dimensions.height} does not meet ${
          exact ? "the required" : "the minimum"
        } ${minimumWidth}x${minimumHeight} size`,
      );
    }
  }
}

async function checkPresentation() {
  const manifest = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
  const version = manifest.version;
  const readme = await readFile(path.join(root, "README.md"), "utf8");

  if (!readme.includes(`Current release ${version}`)) {
    failures.push(`README.md: current release does not match package.json (${version})`);
  }
  if (!readme.includes(`GameVault-v${version}-windows-x64-portable.zip`)) {
    failures.push(`README.md: download filename does not match package.json (${version})`);
  }
  if (
    !readme.includes("$expected") ||
    !readme.includes("$actual") ||
    !readme.includes('throw "GameVault archive checksum mismatch"')
  ) {
    failures.push("README.md: checksum instructions must compare expected and actual hashes");
  }

  for (const relative of [
    "docs/images/gamevault-home.jpg",
    "docs/images/gamevault-library.jpg",
    "docs/images/gamevault-local-files.jpg",
  ]) {
    if (!readme.includes(`](${relative})](${relative})`)) {
      failures.push(`README.md: ${relative} must link to its full-size image`);
    }
  }

  for (const required of [
    "docs/ARCHITECTURE.md",
    "docs/RELEASE_CHECKLIST.md",
    "docs/images/README.md",
    "SECURITY.md",
    "AI tools supported",
  ]) {
    if (!readme.includes(required)) {
      failures.push(`README.md: missing required presentation reference ${required}`);
    }
  }
}

const files = await walk(root);
const markdown = files.filter((file) => file.endsWith(".md"));
await Promise.all(markdown.map(checkMarkdown));
await checkMedia();
await checkPresentation();

if (failures.length) {
  console.error("Documentation check failed:\n");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log(
    `Documentation check passed: ${markdown.length} Markdown files and 4 presentation images verified.`,
  );
}
