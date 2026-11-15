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
//   swipehint | taphint   play the game's own suggested move (needs ?debug=1)
//   expect:TEXT           fail unless the console has printed TEXT
//   waitfor:TEXT[,MS]     wait for the console to print TEXT
//   tapbtn:NAME           touch a button the game reports (play, go, next, map, ...)
//   playhints:N           swipe the suggested move N times
//   playuntil:TEXT[,MAX]  keep swiping suggested moves until the console prints TEXT
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
const profile = join(process.env.DRIVE_TMP || tmpdir(), `rufus-chrome-${process.pid}`);
const chrome = spawn(
  process.env.CHROME || "google-chrome",
  [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--use-gl=angle",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
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

let lastHint = null; // the move the game itself suggests (needs ?debug=1)
const seen = [];
const buttons = new Map(); // where the game says its buttons are (needs ?debug=1)
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
      console.log(`[console.${msg.params.type}] ${text}`);
      const hint = text.match(/RUFUS HINT (\d+) (\d+) (\d+) (\d+)/);
      if (hint) lastHint = hint.slice(1).map(Number);
      const btn = text.match(/RUFUS BTN (\w+) (-?\d+) (-?\d+)/);
      if (btn) buttons.set(btn[1], { x: Number(btn[2]), y: Number(btn[3]), at: Date.now() });
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

  let size = { w: 390, h: 844, dpr: 3 };
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
  // a short quick swipe from one cell toward the next, like a thumb does
  async function flick([x1, y1, x2, y2]) {
    const fx = x1 + (x2 - x1) * 0.6, fy = y1 + (y2 - y1) * 0.6;
    await touch("touchStart", [[x1, y1]]);
    for (let k = 1; k <= 5; k++) {
      await sleep(16);
      await touch("touchMove", [[x1 + ((fx - x1) * k) / 5, y1 + ((fy - y1) * k) / 5]]);
    }
    await touch("touchEnd", []);
    await sleep(150);
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
    } else if (cmd === "playuntil") {
      // playuntil:TEXT[|TEXT...][,MAX]: keep swiping the suggested move until the console says so
      const [text, max = "60"] = arg.split(",");
      const wants = text.split("|");
      let hit = null, moves = 0;
      while (!(hit = wants.find((w) => seen.slice(waitMark).some((t) => t.includes(w))))) {
        if (moves >= Number(max)) throw new Error(`${text} not reached in ${max} moves`);
        const until = Date.now() + 20000;
        while (!lastHint && !wants.some((w) => seen.slice(waitMark).some((t) => t.includes(w)))) {
          if (Date.now() > until) throw new Error(`stuck: no hint and no ${text}`);
          await sleep(40);
        }
        if (!lastHint) continue;
        await flick(lastHint);
        lastHint = null;
        moves++;
      }
      console.log(`[playuntil ok] ${hit} after ${moves} swipes`);
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
    } else if (cmd === "playhints") {
      // playhints:N: swipe the game's suggested move, N times over
      for (let n = 0; n < Number(arg); n++) {
        const until = Date.now() + 15000;
        while (!lastHint) {
          if (Date.now() > until) throw new Error(`no hint for move ${n + 1}`);
          await sleep(50);
        }
        const [x1, y1, x2, y2] = lastHint;
        lastHint = null;
        const fx = x1 + (x2 - x1) * 0.6, fy = y1 + (y2 - y1) * 0.6;
        await touch("touchStart", [[x1, y1]]);
        for (let k = 1; k <= 5; k++) {
          await sleep(16);
          await touch("touchMove", [[x1 + ((fx - x1) * k) / 5, y1 + ((fy - y1) * k) / 5]]);
        }
        await touch("touchEnd", []);
        await sleep(200);
      }
    } else if (cmd === "swipehint" || cmd === "taphint") {
      // play the hinted move the way a finger would
      if (!lastHint) throw new Error("no hint seen yet (is ?debug=1 set?)");
      const [x1, y1, x2, y2] = lastHint;
      lastHint = null;
      if (cmd === "swipehint") {
        // a short flick, not a full cell: phones rarely get more
        const fx = x1 + (x2 - x1) * 0.6, fy = y1 + (y2 - y1) * 0.6;
        await touch("touchStart", [[x1, y1]]);
        for (let k = 1; k <= 5; k++) {
          await sleep(16);
          await touch("touchMove", [[x1 + ((fx - x1) * k) / 5, y1 + ((fy - y1) * k) / 5]]);
        }
        await touch("touchEnd", []);
      } else {
        for (const [x, y] of [[x1, y1], [x2, y2]]) {
          await touch("touchStart", [[x, y]]);
          await sleep(50);
          await touch("touchEnd", []);
          await sleep(120);
        }
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
