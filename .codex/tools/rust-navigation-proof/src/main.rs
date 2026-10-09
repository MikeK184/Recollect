//! Actual local MCP proof. Does not send a model/provider request.
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
struct Client {
    child: Child,
    input: Option<ChildStdin>,
    output: Receiver<String>,
    sequence: u64,
    receipts: Vec<Value>,
}
impl Client {
    fn start(root: &Path, name: &str, executable: &str, args: &[&str]) -> Result<Self> {
        let log = File::create(root.join(format!(".cache/rust-navigation/{name}-stderr.log")))?;
        let executable = if executable.contains('/') {
            root.join(executable)
        } else {
            executable.into()
        };
        let mut child = Command::new(executable)
            .args(args)
            .current_dir(root.join("crates/server"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log)
            .spawn()?;
        let input = child.stdin.take();
        let stream = child.stdout.take().ok_or("Missing server stdout")?;
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stream)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut client = Self {
            child,
            input,
            output,
            sequence: 0,
            receipts: vec![],
        };
        client.request("initialize", json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"recollect-local-proof","version":"0.0.0"}}))?;
        client.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
        Ok(client)
    }
    fn registered(root: &Path, name: &str) -> Result<Self> {
        let config = Command::new("codex")
            .args(["mcp", "get", name, "--json"])
            .current_dir(root.join("crates/server"))
            .output()?;
        if !config.status.success() {
            return Err("Project MCP configuration was not discovered".into());
        }
        let config: Value = serde_json::from_slice(&config.stdout)?;
        let transport = &config["transport"];
        let command = transport["command"]
            .as_str()
            .ok_or("Missing stdio command")?;
        let args: Vec<&str> = transport["args"]
            .as_array()
            .ok_or("Missing stdio args")?
            .iter()
            .map(|v| v.as_str().ok_or("Invalid stdio arg"))
            .collect::<std::result::Result<_, _>>()?;
        let mut client = Self::start(root, name, command, &args)?;
        client.receipts.insert(0, json!({"discovery":"codex mcp get","launch_cwd":"crates/server","configuration":config}));
        Ok(client)
    }
    fn send(&mut self, message: Value) -> Result<()> {
        let input = self.input.as_mut().ok_or("Server input closed")?;
        writeln!(input, "{message}")?;
        input.flush()?;
        Ok(())
    }
    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.sequence += 1;
        let id = self.sequence;
        let started = Instant::now();
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let line = self.output.recv_timeout(Duration::from_secs(55))?;
            let response: Value = serde_json::from_str(&line)?;
            if response["id"] != id {
                continue;
            }
            if response.get("error").is_some() {
                return Err(format!("MCP error: {response}").into());
            }
            let result = response["result"].clone();
            if result["isError"] == true {
                return Err(format!("MCP tool failed: {result}").into());
            }
            self.receipts.push(json!({"method":method,"params":params,"seconds":started.elapsed().as_secs_f64(),"result":result}));
            return Ok(result);
        }
    }
    fn call(&mut self, name: &str, arguments: Value) -> Result<Value> {
        self.request("tools/call", json!({"name":name,"arguments":arguments}))
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.input.take();
        for _ in 0..50 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()?;
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|v| v == "--isolation-child") {
        // Open for writing without writing any bytes: even a broken policy would
        // leave the existing home file unchanged. The effective policy must deny.
        let outside = OpenOptions::new().write(true).open(&args[2]).unwrap_err();
        assert_eq!(outside.kind(), std::io::ErrorKind::PermissionDenied);
        let network = TcpStream::connect(&args[3]).unwrap_err();
        assert_eq!(network.kind(), std::io::ErrorKind::PermissionDenied);
        let canary = root.join(".cache/rust-navigation/isolation-canary");
        fs::write(&canary, "owned checkout write allowed")?;
        fs::remove_file(canary)?;
        println!("Home write denied; network connect denied; repository write allowed");
        return Ok(());
    }
    let home_config = Path::new(&std::env::var("HOME")?).join(".codex/config.toml");
    let before = fs::read(&home_config)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let isolated = Command::new("/usr/bin/sandbox-exec")
        .args(["-D", &format!("RECOLLECT_ROOT={}", root.display()), "-f"])
        .arg(root.join(".codex/tools/rust-navigation.sb"))
        .arg(std::env::current_exe()?)
        .arg("--isolation-child")
        .arg(&home_config)
        .arg(listener.local_addr()?.to_string())
        .output()?;
    assert!(
        isolated.status.success(),
        "Isolation failed: {}",
        String::from_utf8_lossy(&isolated.stderr)
    );
    assert_eq!(before, fs::read(home_config)?);
    fs::write(
        root.join(".cache/rust-navigation/isolation-receipt.txt"),
        isolated.stdout,
    )?;
    println!("Runtime isolation: denied home write/network, allowed local write PASS");
    let mut rust = Client::registered(&root, "rust_analyzer")?;
    let tools = rust.request("tools/list", json!({}))?;
    assert!(tools.to_string().contains("rust_analyzer_definition"));
    let source = fs::read_to_string(root.join("crates/server/src/auth.rs"))?;
    let (line, text) = source
        .lines()
        .enumerate()
        .find(|(_, s)| s.contains("crate::db::preparation_tx(pool"))
        .ok_or("Cross-module control is missing")?;
    let byte = text
        .find("preparation_tx")
        .ok_or("Control position missing")?;
    let character: usize = text[..byte].chars().map(char::len_utf16).sum();
    let position =
        json!({"file_path":"crates/server/src/auth.rs","line":line,"character":character});
    let mut definition = Value::Null;
    let ready_deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < ready_deadline {
        definition = rust.call("rust_analyzer_definition", position.clone())?;
        if definition.to_string().contains("crates/server/src/db.rs") {
            break;
        }
        thread::sleep(Duration::from_secs(2));
    }
    assert!(
        definition.to_string().contains("crates/server/src/db.rs"),
        "No cross-module definition: {definition}"
    );
    let hover = rust.call("rust_analyzer_hover", position.clone())?;
    assert!(
        hover.to_string().contains("preparation_tx"),
        "No resolved signature: {hover}"
    );
    let references = rust.call("rust_analyzer_references", position)?;
    assert!(
        references.to_string().contains("crates/server/src/auth.rs"),
        "No source reference: {references}"
    );
    let symbols = rust.call(
        "rust_analyzer_symbols",
        json!({"file_path":"crates/server/src/db.rs"}),
    )?;
    assert!(symbols.to_string().contains("preparation_tx"));
    let mapping_file = "crates/server/src/knowledge_mapping/storage.rs";
    let mapping_source = fs::read_to_string(root.join(mapping_file))?;
    let (line, text) = mapping_source
        .lines()
        .enumerate()
        .find(|(_, s)| s.contains("let candidates = source_candidates("))
        .ok_or("Native mapping call is missing")?;
    let byte = text
        .find("source_candidates")
        .ok_or("Mapping position missing")?;
    let character: usize = text[..byte].chars().map(char::len_utf16).sum();
    let position = json!({"file_path":mapping_file,"line":line,"character":character});
    let definition = rust.call("rust_analyzer_definition", position.clone())?;
    assert!(
        definition
            .to_string()
            .contains("crates/server/src/knowledge_mapping.rs"),
        "Mapping definition unresolved: {definition}"
    );
    let hover = rust.call("rust_analyzer_hover", position)?;
    assert!(hover.to_string().contains("source_candidates"));
    let read_file = "crates/server/src/graph/read.rs";
    let read_source = fs::read_to_string(root.join(read_file))?;
    let (line, text) = read_source
        .lines()
        .enumerate()
        .find(|(_, s)| s.contains("let mut qualified = retrieval::graph::qualify("))
        .ok_or("Graph qualification call is missing")?;
    let byte = text.find("qualify").ok_or("Graph call position missing")?;
    let character: usize = text[..byte].chars().map(char::len_utf16).sum();
    let position = json!({"file_path":read_file,"line":line,"character":character});
    let definition = rust.call("rust_analyzer_definition", position.clone())?;
    assert!(
        definition
            .to_string()
            .contains("crates/server/src/retrieval/graph.rs")
    );
    let hover = rust.call("rust_analyzer_hover", position)?;
    assert!(hover.to_string().contains("qualify"));
    let queue_file = "crates/server/src/semantic/queue.rs";
    let queue_source = fs::read_to_string(root.join(queue_file))?;
    let (line, text) = queue_source
        .lines()
        .enumerate()
        .find(|(_, s)| s.contains("let input = representation(state"))
        .ok_or("Semantic qualification call is missing")?;
    let byte = text
        .find("representation")
        .ok_or("Semantic call position missing")?;
    let character: usize = text[..byte].chars().map(char::len_utf16).sum();
    let position = json!({"file_path":queue_file,"line":line,"character":character});
    let definition = rust.call("rust_analyzer_definition", position.clone())?;
    assert!(
        definition
            .to_string()
            .contains("crates/server/src/semantic.rs")
    );
    let hover = rust.call("rust_analyzer_hover", position)?;
    assert!(hover.to_string().contains("representation"));
    let discovery_file = "crates/server/src/autonomous.rs";
    let discovery_source = fs::read_to_string(root.join(discovery_file))?;
    let (line, text) = discovery_source
        .lines()
        .enumerate()
        .find(|(_, s)| s.contains("support_discovery::schedule(state"))
        .ok_or("Bounded support discovery call is missing")?;
    let byte = text.find("schedule").ok_or("Audit position missing")?;
    let character: usize = text[..byte].chars().map(char::len_utf16).sum();
    let position = json!({"file_path":discovery_file,"line":line,"character":character});
    let definition = rust.call("rust_analyzer_definition", position.clone())?;
    assert!(
        definition
            .to_string()
            .contains("crates/server/src/autonomous/support_discovery.rs")
    );
    let hover = rust.call("rust_analyzer_hover", position)?;
    assert!(hover.to_string().contains("schedule"));
    fs::write(
        root.join(".cache/rust-navigation/rust-mcp-receipt.json"),
        serde_json::to_vec_pretty(&rust.receipts)?,
    )?;
    println!("Rust MCP: initialize/list/definition/hover/references/symbols PASS");
    drop(rust);

    let mut graft = Client::registered(&root, "graft")?;
    let tools = graft.request("tools/list", json!({}))?;
    assert_eq!(
        tools["tools"]
            .as_array()
            .ok_or("Missing Graft tools")?
            .len(),
        6
    );
    let api = graft.call("graft_file_api", json!({"file":"crates/server/src/db.rs"}))?;
    assert!(
        api.to_string().contains("preparation_tx"),
        "Missing real Rust file API: {api}"
    );
    let callers = graft.call(
        "graft_trace_calls",
        json!({"symbol":"preparation_tx","in":"crates/server/src","depth":1}),
    )?;
    assert!(
        callers.to_string().contains("auth.rs"),
        "Missing structural caller: {callers}"
    );
    let context = graft.call("graft_find_code", json!({"query":"graph preparation snapshot publication","in":"crates/server/src","limit":3}))?;
    assert!(context.to_string().contains("crates/server/src"));
    let mapping_api = graft.call(
        "graft_file_api",
        json!({"file":"crates/server/src/knowledge_mapping/storage.rs"}),
    )?;
    assert!(
        mapping_api.to_string().contains("prepare_source")
            && mapping_api.to_string().contains("stage_source")
    );
    let queue_api = graft.call(
        "graft_file_api",
        json!({"file":"crates/server/src/semantic/queue.rs"}),
    )?;
    assert!(
        queue_api.to_string().contains("maintain_brain")
            && queue_api.to_string().contains("reindex")
    );
    let discovery_api = graft.call(
        "graft_file_api",
        json!({"file":"crates/server/src/autonomous/support_discovery.rs"}),
    )?;
    assert!(
        discovery_api.to_string().contains("prepare")
            && discovery_api.to_string().contains("publish")
    );
    let freshness = graft.call("graft_check_freshness", json!({}))?;
    assert!(
        freshness.to_string().contains("graph check: OK"),
        "Structural graph drift: {freshness}"
    );
    fs::write(
        root.join(".cache/rust-navigation/graft-mcp-receipt.json"),
        serde_json::to_vec_pretty(&graft.receipts)?,
    )?;
    println!("Graft MCP: initialize/list/file API/callers/context/freshness PASS");
    let checked = Command::new(root.join(".codex/tools/graft/graft"))
        .arg("check")
        .current_dir(root.join("crates/server"))
        .output()?;
    assert!(
        checked.status.success(),
        "Repository-local CLI index lookup failed"
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("graph check: OK"));
    println!("Graft CLI: nested working directory selects the owned index PASS");
    println!("No paid model/provider requests; receipts contain local code navigation.");
    Ok(())
}
