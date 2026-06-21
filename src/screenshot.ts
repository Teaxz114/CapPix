import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

let screenshotData = "";
let isSelecting = false;
let startX = 0;
let startY = 0;

const bgImg = document.getElementById("screenshot-bg") as HTMLImageElement;
const selection = document.getElementById("selection") as HTMLDivElement;
const selectionInfo = document.getElementById("selection-info") as HTMLDivElement;

// Listen for screenshot data
listen<string>("screenshot-ready", (event) => {
  screenshotData = event.payload;
  if (bgImg) {
    bgImg.src = `data:image/png;base64,${screenshotData}`;
    bgImg.style.display = "block";
  }
});

// Mouse events for region selection
document.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return;
  isSelecting = true;
  startX = e.clientX;
  startY = e.clientY;
  if (selection) {
    selection.style.display = "block";
    selection.style.left = `${startX}px`;
    selection.style.top = `${startY}px`;
    selection.style.width = "0px";
    selection.style.height = "0px";
  }
});

document.addEventListener("mousemove", (e) => {
  if (!isSelecting) return;
  const x = Math.min(e.clientX, startX);
  const y = Math.min(e.clientY, startY);
  const w = Math.abs(e.clientX - startX);
  const h = Math.abs(e.clientY - startY);
  if (selection) {
    selection.style.left = `${x}px`;
    selection.style.top = `${y}px`;
    selection.style.width = `${w}px`;
    selection.style.height = `${h}px`;
  }
  if (selectionInfo) {
    selectionInfo.style.display = "block";
    selectionInfo.style.left = `${x + w + 8}px`;
    selectionInfo.style.top = `${y}px`;
    selectionInfo.textContent = `${w} × ${h}`;
  }
});

document.addEventListener("mouseup", async (e) => {
  if (!isSelecting) return;
  isSelecting = false;

  const x = Math.min(e.clientX, startX);
  const y = Math.min(e.clientY, startY);
  const w = Math.abs(e.clientX - startX);
  const h = Math.abs(e.clientY - startY);

  if (w < 5 || h < 5) {
    if (selection) selection.style.display = "none";
    if (selectionInfo) selectionInfo.style.display = "none";
    return;
  }

  try {
    const result = await invoke<{ image_base64: string; width: number; height: number }>("capture_region", {
      x,
      y,
      width: w,
      height: h,
    });
    console.log("Captured region:", result.width, "x", result.height);
    await getCurrentWindow().close();
  } catch (err) {
    console.error("Failed to capture region:", err);
  }
});

// ESC to cancel
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    getCurrentWindow().close();
  }
});
