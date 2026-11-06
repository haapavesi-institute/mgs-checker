// Runs the WebAssembly checker off the main thread. The page cancels a run by
// terminating this worker and starting a new one.
import init, { check_quiver, conjecture7_plan, conjecture7_row, describe, explore_plan, explore_row, verify_sequence } from "./pkg/mgs_wasm.js";

const ready = init();

self.onmessage = async ({ data }) => {
  await ready;
  const { id, kind } = data;
  const reply = (payload) => self.postMessage({ id, ...payload });
  try {
    if (kind === "describe") reply({ result: JSON.parse(describe(data.input)) });
    else if (kind === "check") reply({ result: JSON.parse(check_quiver(data.input, JSON.stringify(data.options))) });
    else if (kind === "verify") reply({ result: JSON.parse(verify_sequence(data.input, data.sequence)) });
    else if (kind === "explore" || kind === "conjecture7") {
      const [plan, check] = kind === "explore" ? [explore_plan, explore_row] : [conjecture7_plan, conjecture7_row];
      const items = JSON.parse(plan(JSON.stringify(data.spec)));
      reply({ progress: { done: 0, total: items.length } });
      const options = JSON.stringify(data.options);
      for (let i = 0; i < items.length; i++) {
        const row = JSON.parse(check(JSON.stringify(items[i]), options));
        reply({ row, progress: { done: i + 1, total: items.length } });
      }
      reply({ result: "done" });
    }
  } catch (e) {
    reply({ error: String(e) });
  }
};
