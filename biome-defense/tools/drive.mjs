#!/usr/bin/env node
// Drives the game in a throwaway headless Chrome at phone dimensions, for
// checking builds without a phone in hand. No dependencies: it speaks the
// DevTools protocol directly over Node's built-in WebSocket.
//
//   node tools/drive.mjs <url> <outdir> <step> [<step> ...]
//
// Steps:
//   wait:MS            shot:NAME          tap:X,Y        swipe:X1,Y1,X2,Y2[,MS]
//   eval:JS            offline:on|off     reload         size:W,H[,DPR]
//   drag:X1,Y1,X2,Y2   (mouse instead of touch)
//   expect:TEXT           fail unless the console has printed TEXT
//   waitfor:TEXT[|TEXT][,MS]  wait for the console to print one of them
//   tapbtn:NAME           touch a button the game reports (play, defend, call, ...)
//   tapspot:N             a quick touch on good building ground N of the stage
//                         being played (builds, if a structure has been chosen)
//   dragto:NAME,N[,MS[,hold]]  press on button NAME, slide to building ground
//                         N (or "low", the lowest on screen), let go: how a
//                         structure is dragged out of the
//                         tray. With "hold" the finger stays down (for a
//                         screenshot of the structure being placed) until
//   release               lifts it
//   grab:NAME             like shot, but read straight out of WebGL: works
//                         with DRIVE_GPU, where a screenshot shows no canvas
//   taptower:N            touch structure number N
//   taps:N[,X,Y]          N quick taps at a point (default: the story's text box)
// The game reports its buttons, good building ground and structures on the
// console when the page has ?debug=1 in its address.
// Coordinates are CSS pixels. Console output and page errors are printed.

import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [url, outdir, ...steps] = process.argv.slice(2);
if (!url || !outdir) {
  console.error("usage: drive.mjs <url> <outdir> <step>...");
  process.exit(2);
}
mkdirSync(outdir, { recursive: true });

const port = 9400 + Math.floor(Math.random() * 400);
const profile = join(process.env.DRIVE_TMP || tmpdir(), `biome-chrome-${process.pid}`);
const chrome = spawn(
  process.env.CHROME || "google-chrome",
  [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    // Chrome's software renderer by default, so runs look the same on any
    // machine. DRIVE_GPU=vulkan (or gl-egl, ...) uses the real graphics card
    // through that ANGLE backend instead.
    ...(process.env.DRIVE_GPU
      ? ["--use-gl=angle", `--use-angle=${process.env.DRIVE_GPU}`, "--enable-gpu", "--enable-features=Vulkan"]
      : ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"]),
    "--ignore-gpu-blocklist",
    "--autoplay-policy=no-user-gesture-required",
    "--mute-audio",
    "--window-size=500,1000",
    "about:blank",
  ],
  { stdio: "ignore" }
);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function browserWs() {
  for (let i = 0; i < 100; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json/version`);
      return (await r.json()).webSocketDebuggerUrl;
    } catch {
      await sleep(100);
    }
  }
  throw new Error("chrome did not start");
}

const seen = [];
const buttons = new Map(); // where the game says its buttons are (needs ?debug=1)
const spots = new Map(); // good building ground
const towers = new Map(); // structures standing
let cell = 43; // pixels to a tile, as the game reports it
let nextId = 1;
const pending = new Map();
let ws;
let sessionId;
function send(method, params = {}, session = sessionId) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject, method });
    ws.send(JSON.stringify({ id, method, params, ...(session ? { sessionId: session } : {}) }));
  });
}

let failed = false;
try {
  ws = new WebSocket(await browserWs());
  await new Promise((r, j) => {
    ws.onopen = r;
    ws.onerror = j;
  });
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const p = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? p.reject(new Error(`${p.method}: ${msg.error.message}`)) : p.resolve(msg.result);
    } else if (msg.method === "Runtime.consoleAPICalled") {
      const text = msg.params.args.map((a) => a.value ?? a.description ?? "").join(" ");
      const btn = text.match(/BIOME BTN (\w+) (-?\d+) (-?\d+)/);
      if (btn) buttons.set(btn[1], { x: Number(btn[2]), y: Number(btn[3]), at: Date.now() });
      const spot = text.match(/BIOME SPOT (\d+) (-?\d+) (-?\d+)/);
      if (spot) spots.set(Number(spot[1]), { x: Number(spot[2]), y: Number(spot[3]), at: Date.now() });
      const tower = text.match(/BIOME TOWER (\d+) (-?\d+) (-?\d+)/);
      if (tower) towers.set(Number(tower[1]), { x: Number(tower[2]), y: Number(tower[3]), at: Date.now() });
      const grid = text.match(/BIOME GRID ([\d.]+)/);
      if (grid) cell = Number(grid[1]);
      // the chatter (where things are, how the stage stands) is for this tool, not the reader
      if (!/BIOME (BTN|SPOT|TOWER|GRID|STATE)/.test(text) || process.env.DRIVE_VERBOSE) console.log(`[console.${msg.params.type}] ${text}`);
      seen.push(text);
    } else if (msg.method === "Runtime.exceptionThrown") {
      const d = msg.params.exceptionDetails;
      console.log(`[page error] ${d.exception?.description || d.text}`);
    } else if (msg.method === "Network.loadingFailed") {
      console.log(`[net failed] ${msg.params.errorText}`);
    }
  };

  const { targetId } = await send("Target.createTarget", { url: "about:blank" }, null);
  sessionId = (await send("Target.attachToTarget", { targetId, flatten: true }, null)).sessionId;
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Network.enable");

  // DRIVE_SIZE=W,H,DPR starts at another size (the size: step changes it later)
  const [sw = 390, sh = 844, sd = 3] = (process.env.DRIVE_SIZE || "").split(",").filter(Boolean).map(Number);
  let size = { w: sw, h: sh, dpr: sd };
  async function applySize() {
    await send("Emulation.setDeviceMetricsOverride", {
      width: size.w,
      height: size.h,
      deviceScaleFactor: size.dpr,
      mobile: true,
    });
    await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
  }
  await applySize();
  // DRIVE_INIT=file.js runs a script in the page before anything else (for instrumenting)
  if (process.env.DRIVE_INIT) {
    const { readFileSync } = await import("node:fs");
    await send("Page.addScriptToEvaluateOnNewDocument", { source: readFileSync(process.env.DRIVE_INIT, "utf8") });
  }
  await send("Page.navigate", { url });

  const touch = (type, pts) =>
    send("Input.dispatchTouchEvent", {
      type,
      touchPoints: pts.map(([x, y]) => ({ x, y, id: 1, radiusX: 8, radiusY: 8, force: 1 })),
    });

  // console lines before this index have already been waited on
  let waitMark = 0;
  async function sawAny(wants, from, ms) {
    const until = Date.now() + ms;
    for (;;) {
      const hit = wants.find((w) => seen.slice(from).some((t) => t.includes(w)));
      if (hit) return hit;
      if (Date.now() > until) return null;
      await sleep(40);
    }
  }
  for (const step of steps) {
    const i = step.indexOf(":");
    const cmd = i < 0 ? step : step.slice(0, i);
    const arg = i < 0 ? "" : step.slice(i + 1);
    const nums = arg.split(",").map(Number);
    if (cmd === "wait") {
      await sleep(Number(arg));
    } else if (cmd === "shot") {
      const { data } = await send("Page.captureScreenshot", {
        format: "png",
        clip: { x: 0, y: 0, width: size.w, height: size.h, scale: Number(process.env.SHOT_SCALE || 1) },
      });
      const file = join(outdir, `${arg}.png`);
      writeFileSync(file, Buffer.from(data, "base64"));
      console.log(`[shot] ${file}`);
    } else if (cmd === "tap") {
      await touch("touchStart", [[nums[0], nums[1]]]);
      await sleep(60);
      await touch("touchEnd", []);
      await sleep(60);
    } else if (cmd === "swipe") {
      const [x1, y1, x2, y2, ms = 160] = nums;
      await touch("touchStart", [[x1, y1]]);
      const n = 8;
      for (let k = 1; k <= n; k++) {
        await sleep(ms / n);
        await touch("touchMove", [[x1 + ((x2 - x1) * k) / n, y1 + ((y2 - y1) * k) / n]]);
      }
      await sleep(30);
      await touch("touchEnd", []);
      await sleep(60);
    } else if (cmd === "waitfor") {
      // waitfor:TEXT[|TEXT...][,TIMEOUT_MS]: wait until the console prints one of them
      const [text, ms = "30000"] = arg.split(",");
      const hit = await sawAny(text.split("|"), waitMark, Number(ms));
      if (!hit) throw new Error(`timed out waiting for: ${text}`);
      console.log(`[waitfor ok] ${hit}`);
      waitMark = seen.length;
    } else if (cmd === "tapbtn") {
      // tapbtn:NAME: a real touch on a button the game is currently showing
      const until = Date.now() + 8000;
      let b;
      while (!((b = buttons.get(arg)) && Date.now() - b.at < 700)) {
        if (Date.now() > until) throw new Error(`button not on screen: ${arg}`);
        await sleep(50);
      }
      await touch("touchStart", [[b.x, b.y]]);
      await sleep(60);
      await touch("touchEnd", []);
      await sleep(120);
      buttons.delete(arg);
      console.log(`[tapbtn] ${arg} at ${b.x},${b.y}`);
    } else if (cmd === "tapspot" || cmd === "taptower") {
      // a real touch on building ground, or on a structure, of the stage being played
      const from = cmd === "tapspot" ? spots : towers;
      const until = Date.now() + 8000;
      let m;
      while (!((m = from.get(Number(arg))) && Date.now() - m.at < 1500)) {
        if (Date.now() > until) throw new Error(`${cmd}: not on screen: ${arg}`);
        await sleep(50);
      }
      await touch("touchStart", [[m.x, m.y]]);
      await sleep(40);
      await touch("touchEnd", []);
      await sleep(250);
      console.log(`[${cmd}] ${arg} at ${m.x},${m.y}`);
    } else if (cmd === "dragto") {
      // dragto:NAME,N[,MS]: press a tray button, slide onto the field, let go.
      // A structure being slid floats 1.15 tiles above the finger, so the
      // finger stops that far below where it should stand.
      const [name, n, ms = "600", hold] = arg.split(",");
      const until = Date.now() + 8000;
      let b, m;
      // N can be "low": whichever building ground is nearest the bottom of the field
      const pick = () => (n === "low" ? [...spots.values()].sort((p, q) => q.y - p.y)[0] : spots.get(Number(n)));
      while (!((b = buttons.get(name)) && Date.now() - b.at < 700 && (m = pick()) && Date.now() - m.at < 1500)) {
        if (Date.now() > until) throw new Error(`dragto: not on screen: ${arg}`);
        await sleep(50);
      }
      const [x2, y2] = [m.x, m.y + 1.15 * cell];
      await touch("touchStart", [[b.x, b.y]]);
      const steps = 12;
      for (let k = 1; k <= steps; k++) {
        await sleep(Number(ms) / steps);
        await touch("touchMove", [[b.x + ((x2 - b.x) * k) / steps, b.y + ((y2 - b.y) * k) / steps]]);
      }
      await sleep(120);
      if (!hold) {
        await touch("touchEnd", []);
        await sleep(250);
      }
      console.log(`[dragto] ${name} to spot ${n} at ${x2},${y2.toFixed(0)}`);
    } else if (cmd === "grab") {
      // grab:NAME: the frame as the game drew it, read straight back out of
      // WebGL at the end of its next frame. A screenshot does not include
      // the canvas when Chrome is drawing with the real graphics card.
      const { result, exceptionDetails } = await send("Runtime.evaluate", {
        awaitPromise: true,
        returnByValue: true,
        expression: `new Promise((resolve) => {
          const orig = window.requestAnimationFrame;
          window.requestAnimationFrame = function (cb) {
            return orig.call(window, function (t) {
              cb(t);
              window.requestAnimationFrame = orig;
              const c = document.querySelector("canvas");
              const gl = c.getContext("webgl") || c.getContext("webgl2") || c.getContext("experimental-webgl");
              const w = c.width, h = c.height, px = new Uint8Array(w * h * 4);
              gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, px);
              const o = document.createElement("canvas");
              o.width = w; o.height = h;
              const ctx = o.getContext("2d"), img = ctx.createImageData(w, h);
              for (let y = 0; y < h; y++) img.data.set(px.subarray((h - 1 - y) * w * 4, (h - y) * w * 4), y * w * 4);
              for (let i = 3; i < img.data.length; i += 4) img.data[i] = 255;
              ctx.putImageData(img, 0, 0);
              resolve(o.toDataURL("image/png") + "|" + gl.getError());
            });
          };
        })`,
      });
      if (exceptionDetails) throw new Error("grab: " + JSON.stringify(exceptionDetails.exception || exceptionDetails));
      const [dataUrl, glError] = result.value.split("|");
      const file = join(outdir, `${arg}.png`);
      writeFileSync(file, Buffer.from(dataUrl.split(",")[1], "base64"));
      console.log(`[grab] ${file} (GL error ${glError})`);
    } else if (cmd === "release") {
      await touch("touchEnd", []);
      await sleep(250);
    } else if (cmd === "taps") {
      // taps:N[,X,Y]: N quick taps, for paging through the story
      const [n, x = 195, y = 740] = nums;
      for (let k = 0; k < n; k++) {
        await touch("touchStart", [[x, y]]);
        await sleep(50);
        await touch("touchEnd", []);
        await sleep(130);
      }
    } else if (cmd === "expect") {
      // fail the run unless the console has shown this text
      if (!seen.some((t) => t.includes(arg))) throw new Error(`expected console text not seen: ${arg}`);
      console.log(`[expect ok] ${arg}`);
    } else if (cmd === "drag") {
      const [x1, y1, x2, y2] = nums;
      const m = (type, x, y) =>
        send("Input.dispatchMouseEvent", { type, x, y, button: "left", buttons: 1, clickCount: 1 });
      await m("mousePressed", x1, y1);
      for (let k = 1; k <= 6; k++) {
        await sleep(20);
        await m("mouseMoved", x1 + ((x2 - x1) * k) / 6, y1 + ((y2 - y1) * k) / 6);
      }
      await m("mouseReleased", x2, y2);
    } else if (cmd === "eval") {
      const r = await send("Runtime.evaluate", { expression: arg, awaitPromise: true, returnByValue: true });
      console.log(`[eval] ${JSON.stringify(r.result.value ?? r.result.description)}`);
    } else if (cmd === "offline") {
      await send("Network.emulateNetworkConditions", {
        offline: arg === "on",
        latency: 0,
        downloadThroughput: -1,
        uploadThroughput: -1,
      });
      console.log(`[offline] ${arg}`);
    } else if (cmd === "reload") {
      await send("Page.reload", {});
    } else if (cmd === "size") {
      size = { w: nums[0], h: nums[1], dpr: nums[2] || size.dpr };
      await applySize();
    } else {
      throw new Error(`unknown step: ${step}`);
    }
  }
} catch (e) {
  failed = true;
  console.error(`[drive] ${e.message}`);
} finally {
  try {
    ws?.close();
  } catch {}
  chrome.kill("SIGKILL");
  await sleep(150);
  rmSync(profile, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
