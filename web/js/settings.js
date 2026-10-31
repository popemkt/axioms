// Display settings: one schema, which the Display panel renders and every
// drawing step reads. Remembered per device (localStorage). Mirrors kb's
// graph lens options (layout, label density, link style, colour/size by)
// without sharing its code.

const KEY = "axioms.display.v1";

export const SCHEMA = [
  { key: "layout", label: "Layout", options: [["blueprint", "Blueprint"], ["radial", "Radial"], ["force", "Force"]] },
  { key: "labels", label: "Labels", options: [["all", "All"], ["important", "Important"], ["hover", "Hover"], ["none", "None"]] },
  { key: "colorBy", label: "Colour by", options: [["section", "Section"], ["state", "State"], ["depth", "Depth"]] },
  { key: "known", label: "Known concepts", options: [["dim", "Dim"], ["show", "Show"], ["hide", "Hide"]] },
  { key: "edges", label: "Edges", options: [["reduced", "Reduced"], ["all", "All"]] },
  { key: "linkStyle", label: "Link style", options: [["flow", "Flow"], ["curved", "Curved"], ["straight", "Straight"]] },
  { key: "sizeBy", label: "Size by", options: [["uniform", "Uniform"], ["dependants", "Dependants"], ["depth", "Depth"]] },
  { key: "labelSize", label: "Label size", options: [["s", "S"], ["m", "M"], ["l", "L"]] },
];

export const DEFAULTS = {
  layout: "blueprint",
  labels: "all",
  colorBy: "section",
  known: "dim",
  edges: "reduced",
  linkStyle: "flow",
  sizeBy: "uniform",
  labelSize: "m",
};

function valid(key, value) {
  return SCHEMA.find((s) => s.key === key)?.options.some(([v]) => v === value);
}

export function createSettings() {
  let stored = {};
  try {
    stored = JSON.parse(localStorage.getItem(KEY) || "{}");
  } catch {}
  const values = { ...DEFAULTS };
  for (const [k, v] of Object.entries(stored)) if (valid(k, v)) values[k] = v;
  const listeners = new Set();
  return {
    get: () => ({ ...values }),
    set(key, value) {
      if (!valid(key, value) || values[key] === value) return;
      values[key] = value;
      localStorage.setItem(KEY, JSON.stringify(values));
      listeners.forEach((f) => f({ ...values }, key));
    },
    reset() {
      Object.assign(values, DEFAULTS);
      localStorage.removeItem(KEY);
      listeners.forEach((f) => f({ ...values }, null));
    },
    onChange: (f) => listeners.add(f),
  };
}
