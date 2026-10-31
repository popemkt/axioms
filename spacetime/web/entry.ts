// The one module the web UI imports for live mode: the generated bindings
// (DbConnection typed to axioms' tables and reducers). Bundled with the SDK
// by scripts/web-bundle.sh, so the UI keeps no build step of its own.
export { DbConnection, reducers, tables } from "./bindings/index";
