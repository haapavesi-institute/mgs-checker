// The page: reads the quiver, draws it, and sends work to the WebAssembly worker.

const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const seq = (s) => `(${s.join(", ")})`;
const arrows = (t) => esc(t).replaceAll("-&gt;", "→");

const EXAMPLES = [
  ["1 ⇉ 2 → 3", "1->2*2, 2->3"],
  ["Kronecker", "1->2*2"],
  ["Ã₂", "1->2, 2->3, 1->3"],
  ["Ã₂ with a tail", "1->2, 2->3, 1->3, 3->4"],
  ["Square", "1->2, 2->3, 1->4, 4->3"],
  ["D̃₄", "1->5, 2->5, 3->5, 4->5"],
  ["1→2→3⇉4 (BDP 5.5)", "1->2, 2->3, 3->4*2"],
  ["E₆", "1->2, 2->3, 3->4, 4->5, 3->6"],
];

// One worker per run, so Cancel can simply terminate it.
let worker = null;
let nextId = 1;
const pending = new Map();
function freshWorker() {
  worker?.terminate();
  for (const p of pending.values()) p.reject(new Error("cancelled"));
  pending.clear();
  worker = new Worker("worker.js", { type: "module" });
  worker.onmessage = ({ data }) => {
    const p = pending.get(data.id);
    if (!p) return;
    if (data.error) {
      pending.delete(data.id);
      p.reject(new Error(data.error));
      freshWorker(); // a panic can leave the WebAssembly instance unusable
    } else if (data.row || (data.progress && data.result === undefined)) p.onUpdate?.(data);
    else {
      pending.delete(data.id);
      p.resolve(data.result);
    }
  };
}
function call(message, onUpdate) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject, onUpdate });
    worker.postMessage({ id, ...message });
  });
}
freshWorker();

// ---- the quiver: parse, describe, draw ----------------------------------
let current = null;
async function refresh() {
  try {
    current = await call({ kind: "describe", input: $("quiver").value });
    $("parsed").innerHTML = `${current.vertices} vertices: <code>${arrows(current.arrows)}</code> · ${esc(current.facts.kind_name)}` + (current.rejected ? `<br><span class="error">Refused: ${esc(current.rejected)}.</span>` : "");
    draw(current);
  } catch (e) {
    current = null;
    if (String(e.message) !== "cancelled") $("parsed").innerHTML = `<span class="error">${esc(e.message)}</span>`;
    $("drawing").innerHTML = "";
  }
}

function draw(d) {
  const n = d.vertices;
  const R = n === 1 ? 0 : 42;
  const at = (i) => [R * Math.cos((2 * Math.PI * i) / n - Math.PI / 2), R * Math.sin((2 * Math.PI * i) / n - Math.PI / 2)];
  let svg = `<defs><marker id="head" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="currentColor"/></marker></defs><g fill="none" stroke="currentColor" color="var(--ink)">`;
  for (const [i, j, m] of d.list) {
    const [x1, y1] = at(i), [x2, y2] = at(j);
    const len = Math.hypot(x2 - x1, y2 - y1), ux = (x2 - x1) / len, uy = (y2 - y1) / len, r = 8;
    svg += `<line x1="${x1 + ux * r}" y1="${y1 + uy * r}" x2="${x2 - ux * r}" y2="${y2 - uy * r}" stroke-width="${m > 1 ? 1.6 : 0.9}" marker-end="url(#head)"/>`;
    if (m > 1) svg += `<text x="${(x1 + x2) / 2 - uy * 5}" y="${(y1 + y2) / 2 + ux * 5 + 2}" font-size="6" text-anchor="middle" fill="currentColor" stroke="none">×${m}</text>`;
  }
  svg += "</g>";
  for (let i = 0; i < n; i++) {
    const [x, y] = at(i);
    svg += `<circle cx="${x}" cy="${y}" r="7" fill="var(--accent)"/><text x="${x}" y="${y + 2.2}" font-size="6.5" font-weight="700" text-anchor="middle" fill="var(--accent-ink)">${i + 1}</text>`;
  }
  $("drawing").innerHTML = svg;
  $("drawing").setAttribute("aria-label", `Drawing of the quiver: ${d.arrows}`);
}

for (const [label, text] of EXAMPLES) {
  const b = document.createElement("button");
  b.className = "chip";
  b.textContent = label;
  b.onclick = () => {
    $("quiver").value = text;
    refresh();
  };
  $("examples").append(b);
}
$("file").onchange = async (e) => {
  const f = e.target.files?.[0];
  if (f) {
    $("quiver").value = await f.text();
    refresh();
  }
};
let timer = 0;
$("quiver").oninput = () => {
  clearTimeout(timer);
  timer = setTimeout(refresh, 250);
};

// ---- check ---------------------------------------------------------------
const num = (id) => Number($(id).value);
function options() {
  return { max_gap: num("max-gap"), max_length: num("max-length"), max_states: num("max-states"), prune: $("prune").checked, sample: num("sample"), banned_depth: num("banned-depth") };
}
const STATUS_CLASS = { exhausted: "", "no-gap-theorem": "", conditional: "conditional" };

function reasonText(r) {
  switch (r.kind) {
    case "bhit332": return `<span class="badge proved">proved</span>BHIT 3.3.2: vertex ${r.vertex} is the target of ${r.arrows} arrows from ${r.from}`;
    case "dead-end": return `<span class="badge proved">proved</span>every continuation reaches a BHIT 3.3.2 step within ${r.within} steps`;
    case "no-gap-theorem": return `<span class="badge proved">proved</span>no continuation within the bound, and the No Gap theorem applies`;
    case "tentative": return `<span class="badge tentative">tentative</span>empirically infinite (max gap ${r.max_gap})`;
    case "longer-mgs": return `<span class="badge bad">not banned</span>finishes as an MGS of length ${r.length}: the gap assumption fails`;
    default: return `<span class="badge bad">unresolved</span>entries outgrew 64 bits`;
  }
}

function renderReport(r, ms) {
  const cls = STATUS_CLASS[r.status.kind] ?? "incomplete";
  const max = Math.max(1, ...r.by_length.map((c) => Number(c.count)));
  let h = `<div class="status ${cls}"><strong>${esc(r.explanation)}</strong><br><span class="muted">${r.states.toLocaleString()} exchange-graph classes, ${(ms / 1000).toFixed(2)} s</span></div>`;
  if (r.by_length.length) {
    h += `<p><strong>${esc(r.total)}</strong> maximal green sequences; shortest ${r.shortest}, longest ${r.longest}.</p>`;
    h += `<div class="scroll" tabindex="0" role="region" aria-label="MGS by length"><table class="counts"><thead><tr><th scope="col" class="num">Length</th><th scope="col" class="num">MGS</th><th scope="col"><span class="sr-only">Share</span></th></tr></thead><tbody>`;
    for (const c of r.by_length) h += `<tr><td class="num">${c.length}</td><td class="num mono">${esc(c.count)}</td><td><div class="bar" style="width:${(100 * Number(c.count)) / max}%"></div></td></tr>`;
    h += `</tbody></table></div>`;
  } else h += `<p>No maximal green sequence found.</p>`;
  for (const a of r.agreements) h += `<p class="muted">${a.agrees ? "✓" : "✗ DISAGREES:"} ${a.what} MGS ${a.expected} by ${esc(a.source)}; found ${a.found ?? "none"}.</p>`;
  if (r.seeds) h += seedsHtml(r);
  if (r.cut) h += cutHtml(r);
  if (r.sample.length) h += `<details open><summary>${r.sample.length} of them</summary><ul class="seqs">${r.sample.map((s) => `<li>${seq(s)}</li>`).join("")}</ul></details>`;
  if (r.banned.length) {
    h += `<h3>Banned prefixes</h3><p class="muted">Each can never be finished; every shorter prefix can. Tails show a green continuation.</p><div class="scroll" tabindex="0" role="region" aria-label="Banned prefixes"><table><thead><tr><th scope="col">Prefix</th><th scope="col">Why</th><th scope="col">Then</th></tr></thead><tbody>`;
    for (const b of r.banned) {
      const tail = b.tail.length ? `${seq(b.tail)}${b.tail_end === "still-green" ? "… still green" : b.tail_end === "dead" ? ", then every green step breaks BHIT 3.3.2" : ""}` : "";
      h += `<tr><td class="mono">${seq(b.prefix)}</td><td>${reasonText(b.reason)}</td><td class="mono">${tail}</td></tr>`;
    }
    h += `</tbody></table></div>`;
  }
  $("result").innerHTML = h;
}

function seedsHtml(r) {
  const s = r.seeds;
  const scope = { exhausted: "every MGS", "no-gap-theorem": "every MGS", conditional: "every MGS, if the gap assumption holds" }[r.status.kind] ?? `the MGS of length ≤ ${r.searched_to}`;
  let h = `<h3>C-vectors</h3><p>Over ${scope}: <strong>${s.step_c_vectors}</strong> distinct c-vectors at the steps, largest entry <strong>${s.largest_entry}</strong>. The MGS pass through ${s.classes.toLocaleString()} exchange-graph classes, and meet at most ${s.most_arrows} parallel arrows.</p>`;
  h += `<p class="muted">${s.maximal.length} maximal under the entrywise order (none is ≥ another in every entry), each with a shortest green prefix whose last step has that c-vector.</p>`;
  h += `<div class="scroll" tabindex="0" role="region" aria-label="Maximal c-vectors"><table><thead><tr><th scope="col">c-vector</th><th scope="col" class="num">Total</th><th scope="col">First at the last step of</th></tr></thead><tbody>`;
  for (const m of s.maximal) h += `<tr><td class="mono">(${m.vector.join(", ")})</td><td class="num">${m.vector.reduce((a, b) => a + b, 0)}</td><td class="mono">${seq(m.prefix)}</td></tr>`;
  return h + `</tbody></table></div>`;
}

const vec = (v) => `(${v.join(", ")})`;
function cutHtml(r) {
  const c = r.cut, v = c.verdict;
  const scope = { exhausted: "every MGS", "no-gap-theorem": "every MGS", conditional: "every MGS (if the gap assumption holds)" }[r.status.kind] ?? `every MGS of length ≤ ${r.searched_to}`;
  let text;
  if (v.verdict === "holds") text = `<span class="badge proved">holds</span>${scope} is an MGS of Q′ = <code>${arrows(c.q_prime)}</code>, with the same c-vector at every step.`;
  else if (v.verdict === "unfinished") text = `<span class="badge bad">unfinished</span>the walk ran out of its state budget.`;
  else {
    const f = v.failure;
    const what = f.kind === "not-maximal-in-prime" ? `it is an MGS of Q, but ${seq(f.still_green)} are still green in Q′` : f.kind === "red-in-prime" ? `its last step is red in Q′: c = ${vec(f.c_in_q)} in Q, ${vec(f.c_in_prime)} in Q′` : `its last step has c = ${vec(f.c_in_q)} in Q but ${vec(f.c_in_prime)} in Q′`;
    text = `<span class="badge bad">fails</span>on Q′ = <code>${arrows(c.q_prime)}</code>: ${seq(v.prefix)} — ${what}.`;
  }
  let h = `<h3>Conjecture #7</h3><p>Deleting the multiple arrows: ${text}</p>`;
  if (c.label_covers_multiple) h += `<p class="muted">Note: the last step of ${seq(c.label_covers_multiple[0])} has c = ${vec(c.label_covers_multiple[1])}, covering both ends of a multiple arrow.</p>`;
  return h;
}

async function runCheck() {
  $("run").disabled = true;
  $("cancel").disabled = false;
  $("busy").textContent = "Searching…";
  $("result").innerHTML = "";
  const t0 = performance.now();
  try {
    renderReport(await call({ kind: "check", input: $("quiver").value, options: options() }), performance.now() - t0);
  } catch (e) {
    $("result").innerHTML = `<p class="error">${esc(e.message)}</p>`;
  } finally {
    $("run").disabled = false;
    $("cancel").disabled = true;
    $("busy").textContent = "";
  }
}
$("run").onclick = runCheck;
$("cancel").onclick = () => {
  freshWorker();
  refresh();
};

$("verify").onclick = async () => {
  try {
    const v = await call({ kind: "verify", input: $("quiver").value, sequence: $("sequence").value });
    const verdict = { maximal: "a maximal green sequence", "green-not-maximal": `green but not maximal; still green: ${seq(v.green_at_end)}`, "not-green": "not green: some step mutates a red vertex" }[v.verdict];
    const steps = v.steps.map((s, t) => `<li>step ${t + 1}: ${s.vertex} — ${s.green ? "green" : "<strong>red</strong>"}, c = (${s.c_vector.join(", ")})${s.bhit ? ` · breaks BHIT 3.3.2 (${s.bhit[1]} arrows from ${s.bhit[0]})` : ""}</li>`).join("");
    $("verified").innerHTML = `<p><strong>${verdict}</strong></p><ol class="muted">${steps}</ol>`;
  } catch (e) {
    $("verified").innerHTML = `<p class="error">${esc(e.message)}</p>`;
  }
};

// ---- explore ---------------------------------------------------------------
let rows = [];
function spec() {
  return {
    families: [...document.querySelectorAll('input[name="fam"]:checked')].map((c) => JSON.parse(c.value)),
    orientations: $("orientations").value,
    include_base: $("base").checked,
    mods: { multiplicity: $("mod-mult").checked, new_edge: $("mod-new").checked },
    steps: Number($("steps").value),
    up_to_opposite: $("opposite").checked,
  };
}
function addRow(r) {
  const tr = document.createElement("tr");
  const settled = r.settled ? (r.proved ? '<span class="badge proved">exact</span>' : '<span class="badge tentative">if no gap</span>') : '<span class="badge bad">incomplete</span>';
  tr.innerHTML = `<td>${r.id}</td><td>${arrows(r.base)}</td><td>${arrows(r.change || "—")}</td><td class="mono">${arrows(r.arrows)}</td><td>${esc(r.kind)}</td><td class="num mono">${esc(r.total)}</td><td class="mono">${esc(r.by_length)}</td><td class="num">${r.longest ?? ""}${r.predicted_longest != null ? ` (${r.predicted_longest})` : ""}</td><td class="mono">${esc(r.maximal_c)}</td><td>${esc(r.banned_first)}</td><td title="${esc(r.status)}">${settled}</td>`;
  document.querySelector("#explore-table tbody").append(tr);
}
$("explore-run").onclick = async () => {
  rows = [];
  document.querySelector("#explore-table tbody").innerHTML = "";
  $("explore-table").hidden = false;
  for (const id of ["explore-run", "explore-csv", "explore-json"]) $(id).disabled = true;
  $("explore-cancel").disabled = false;
  try {
    await call({ kind: "explore", spec: spec(), options: options() }, ({ row, progress }) => {
      if (row) {
        rows.push(row);
        addRow(row);
      }
      $("explore-busy").textContent = `${progress.done} of ${progress.total} quivers checked`;
    });
  } catch (e) {
    $("explore-busy").innerHTML = `<span class="error">${esc(e.message)}</span>`;
  } finally {
    $("explore-run").disabled = false;
    $("explore-cancel").disabled = true;
    $("explore-csv").disabled = $("explore-json").disabled = rows.length === 0;
  }
};
$("explore-cancel").onclick = () => {
  freshWorker();
  refresh();
};
function download(name, text, type) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
$("explore-json").onclick = () => download("mgs-explore.json", JSON.stringify(rows, null, 2), "application/json");
$("explore-csv").onclick = () => {
  const cols = ["id", "parent", "base", "change", "arrows", "kind", "settled", "proved", "total", "shortest", "longest", "predicted_longest", "by_length", "classes_on_mgs", "step_c_vectors", "maximal_c", "largest_c_entry", "most_arrows", "banned_first", "status"];
  const q = (v) => (v == null ? "" : /[",\n]/.test(String(v)) ? `"${String(v).replace(/"/g, '""')}"` : String(v));
  download("mgs-explore.csv", [cols.join(","), ...rows.map((r) => cols.map((c) => q(r[c])).join(","))].join("\n") + "\n", "text/csv");
};

// ---- conjecture #7 ----------------------------------------------------------
let c7rows = [];
let c7total = 0;
const c7verdict = (r) => (r.verdict === "holds" && !r.proved ? "holds if no gap" : r.verdict);
function c7summary() {
  const byN = new Map();
  for (const r of c7rows) {
    const m = byN.get(r.vertices) ?? new Map();
    m.set(c7verdict(r), (m.get(c7verdict(r)) ?? 0) + 1);
    byN.set(r.vertices, m);
  }
  const lines = [...byN].sort((a, b) => a[0] - b[0]).map(([n, m]) => `<li>${n} vertices: ${[...m].map(([v, c]) => `${c.toLocaleString()} ${esc(v)}`).join(", ")}</li>`);
  const fails = c7rows.filter((r) => r.verdict === "fails").length;
  $("c7-summary").innerHTML = `<ul>${lines.join("")}</ul><p><strong>${fails ? `${fails} counterexample${fails > 1 ? "s" : ""}` : "No counterexample"}</strong> among ${c7rows.length.toLocaleString()} quivers checked.</p>`;
}
function c7addRow(r) {
  if (c7total > 1000 && r.verdict === "holds") return;
  const tr = document.createElement("tr");
  const badge = r.verdict === "holds" ? (r.proved ? '<span class="badge proved">holds</span>' : '<span class="badge tentative">holds if no gap</span>') : r.verdict === "no MGS" ? "no MGS" : `<span class="badge bad">${esc(r.verdict)}</span>`;
  tr.innerHTML = `<td>${r.id}</td><td class="mono">${arrows(r.arrows)}</td><td>${esc(r.kind)}</td><td>${r.on_cycle ? "yes" : "no"}</td><td>${r.iz_cut ? `k = ${r.iz_cut}` : "none"}</td><td class="num mono">${esc(r.total)}</td><td title="${esc(r.status)}">${badge}</td><td>${esc(r.detail)}</td>`;
  document.querySelector("#c7-table tbody").append(tr);
}
$("c7-run").onclick = async () => {
  c7rows = [];
  c7total = 0;
  document.querySelector("#c7-table tbody").innerHTML = "";
  $("c7-summary").innerHTML = "";
  $("c7-table").hidden = false;
  for (const id of ["c7-run", "c7-csv", "c7-json"]) $(id).disabled = true;
  $("c7-cancel").disabled = false;
  const spec = { min_vertices: num("c7-min"), max_vertices: num("c7-max"), max_multiplicity: num("c7-m"), class: $("c7-class").value };
  $("c7-busy").textContent = "Listing the quivers…";
  try {
    await call({ kind: "conjecture7", spec, options: options() }, ({ row, progress }) => {
      c7total = progress.total;
      if (row) {
        c7rows.push(row);
        c7addRow(row);
        if (c7rows.length % 200 === 0) c7summary();
      }
      $("c7-busy").textContent = `${progress.done.toLocaleString()} of ${progress.total.toLocaleString()} quivers checked`;
    });
    c7summary();
  } catch (e) {
    $("c7-busy").innerHTML = `<span class="error">${esc(e.message)}</span>`;
  } finally {
    $("c7-run").disabled = false;
    $("c7-cancel").disabled = true;
    $("c7-csv").disabled = $("c7-json").disabled = c7rows.length === 0;
  }
};
$("c7-cancel").onclick = () => {
  freshWorker();
  refresh();
  if (c7rows.length) c7summary();
};
$("c7-json").onclick = () => download("mgs-conjecture7.json", JSON.stringify(c7rows, null, 2), "application/json");
$("c7-csv").onclick = () => {
  const cols = ["id", "vertices", "arrows", "kind", "on_cycle", "iz_cut", "total", "longest", "verdict", "proved", "detail", "labels_avoid_multiple", "q_prime", "status"];
  const q = (v) => (v == null ? "" : /[",\n]/.test(String(v)) ? `"${String(v).replace(/"/g, '""')}"` : String(v));
  download("mgs-conjecture7.csv", [cols.join(","), ...c7rows.map((r) => cols.map((c) => q(r[c])).join(","))].join("\n") + "\n", "text/csv");
};

refresh();
