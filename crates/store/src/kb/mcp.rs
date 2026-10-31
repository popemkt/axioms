//! A session with `kb mcp`: kb's MCP server on stdio, the same action
//! registry its CLI and HTTP API serve. One process per load or save, so
//! every call in it sees one consistent store and nothing stays running.

use anyhow::{Context, bail};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    out: BufReader<ChildStdout>,
    next: u64,
}

/// A refusal kb answered: its domain error code (`not_found`,
/// `invalid_input`, …) and message.
#[derive(Debug)]
pub struct Refused {
    pub code: String,
    pub message: String,
}

impl Session {
    pub fn open(bin: &str, root: &Path, home: Option<&Path>) -> anyhow::Result<Self> {
        let mut cmd = Command::new(bin);
        if let Some(h) = home {
            cmd.env("KB_HOME", h);
        }
        let mut child = cmd
            .arg("--root")
            .arg(root)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("starting `{bin} mcp` (is kb installed? AXIOMS_KB_BIN)"))?;
        let stdin = child.stdin.take().context("kb mcp stdin")?;
        let out = BufReader::new(child.stdout.take().context("kb mcp stdout")?);
        let mut s = Session {
            child,
            stdin: Some(stdin),
            out,
            next: 0,
        };
        s.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "axioms", "version": env!("CARGO_PKG_VERSION") }
            }),
        )?;
        s.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
        Ok(s)
    }

    /// Invoke one registry action by its MCP tool name (`node_add`, …).
    pub fn call(&mut self, tool: &str, args: Value) -> anyhow::Result<Result<Value, Refused>> {
        let result = self.request("tools/call", json!({ "name": tool, "arguments": args }))?;
        let text = result["content"][0]["text"].as_str().unwrap_or("{}");
        let body: Value =
            serde_json::from_str(text).with_context(|| format!("kb {tool} answered: {text}"))?;
        if result["isError"].as_bool() == Some(true) {
            return Ok(Err(Refused {
                code: body["code"].as_str().unwrap_or("error").to_owned(),
                message: body["message"].as_str().unwrap_or(text).to_owned(),
            }));
        }
        Ok(Ok(body))
    }

    /// [`Session::call`], with a refusal as an error.
    pub fn must(&mut self, tool: &str, args: Value) -> anyhow::Result<Value> {
        match self.call(tool, args)? {
            Ok(v) => Ok(v),
            Err(r) => bail!("kb {tool}: {} ({})", r.message, r.code),
        }
    }

    fn send(&mut self, msg: &Value) -> anyhow::Result<()> {
        let stdin = self.stdin.as_mut().context("kb mcp closed")?;
        writeln!(stdin, "{msg}")?;
        stdin.flush().context("writing to kb mcp")
    }

    fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.next += 1;
        let id = self.next;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        let mut line = String::new();
        loop {
            line.clear();
            if self.out.read_line(&mut line)? == 0 {
                bail!("kb mcp exited during {method}");
            }
            let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if msg["id"].as_u64() != Some(id) {
                continue;
            }
            if let Some(e) = msg.get("error") {
                bail!("kb mcp {method}: {e}");
            }
            return Ok(msg["result"].clone());
        }
    }
}

impl Drop for Session {
    /// Close stdin so kb ends on its own; kill it only if it lingers.
    fn drop(&mut self) {
        drop(self.stdin.take());
        for _ in 0..50 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
