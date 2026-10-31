// Layout by name. Every layout takes the drawing model and returns
// { kind: "boxes" | "dots", pos, routes, sections, labels, parent? }.

import { blueprint } from "./blueprint.js";
import { force } from "./force.js";
import { radial } from "./radial.js";

const LAYOUTS = { blueprint, radial, force };

export const layout = (name, model) => (LAYOUTS[name] ?? blueprint)(model);
