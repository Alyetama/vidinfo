(() => {
  const tc = document.getElementById("timecode");
  if (!tc) return;

  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function pad(n, w = 2) {
    return String(n).padStart(w, "0");
  }

  function tick() {
    const d = new Date();
    // Drop-frame-ish display: HH:MM:SS:FF at ~30fps feel
    const ff = Math.floor((d.getMilliseconds() / 1000) * 30);
    tc.textContent = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}:${pad(ff)}`;
  }

  tick();
  if (!reduced) {
    setInterval(tick, 33);
  }

  // Soft reveal on load
  if (!reduced) {
    document.body.classList.add("ready");
  }
})();
